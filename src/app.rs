//! Application state machine: menu → camp (map/merchant/forge/inventory) →
//! mission (HUD, pause, inventory) → end screens. Drives winit + gilrs + wgpu.

use crate::assets::Assets;
use crate::consts;
use crate::game::combat::PickupKind;
use crate::game::enemy;
use crate::game::items::{ArmorClass, Enchant, ItemClass};
use crate::game::inventory::Inventory;
use crate::game::player::PlayerState;
use crate::game::progress::Save;
use crate::game::shop::{forge_price, forge_upgrade, Camp};
use crate::game::{Game, RunState};
use crate::gfx::camera::Camera;
use crate::gfx::text::{GlyphCache, SIZE_SMALL};

/// Index de classe d'armure portée (0 = léger/cuir, 1 = maille/fer,
/// 2 = lourd/diamant) pour l'affichage du héros, None si rien d'équipé.
fn armor_class_idx(inv: &Inventory) -> Option<usize> {
    let a = inv.armor.as_ref()?;
    match a.class {
        ItemClass::A(ArmorClass::Light) => Some(0),
        ItemClass::A(ArmorClass::Medium) => Some(1),
        ItemClass::A(ArmorClass::Heavy) => Some(2),
        _ => None,
    }
}
use crate::gfx::{BoxInstance, BillboardInstance, CameraUniform, FrameData, Gfx, QuadInstance};
use crate::input::{Input, UINav};
use crate::models::SkinRects;
use crate::ui::{self, Ui};
use crate::ui_mcd;
use crate::world::missions::MISSIONS;
use glam::{Mat4, Vec2, Vec3, Vec4};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::KeyCode;
use winit::window::{Window, WindowId};

#[derive(Clone, Copy, PartialEq)]
pub enum Screen {
    MainMenu,
    Camp,
    Playing,
    End,
}

pub struct App {
    pub window: Option<Arc<Window>>,
    pub gfx: Option<Gfx>,
    pub assets: Assets,
    pub glyphs: GlyphCache,
    pub skins: Vec<SkinRects>,
    pub input: Input,
    pub gilrs: Option<gilrs::Gilrs>,
    pub audio: crate::game::audio::Audio,
    pub save: Save,
    pub camp: Camp,
    pub screen: Screen,
    pub sel: usize,
    pub tab: usize,
    pub show_controls: bool,
    /// hero editor (skin selection) opened from "RÉGLAGES HÉROS"
    pub hero_open: bool,
    /// hero editor carousel index (live preview on the 3D hero)
    pub hero_sel: usize,
    /// hero editor cursor: 0=prev 1=next 2=apply 3=back
    pub hero_cursor: usize,
    pub inv_open: bool,
    pub inv_player: usize,
    pub inv_sel: usize,
    /// last selected ITEM slot (<100) — action buttons (100+) operate on it
    pub inv_item_sel: usize,
    pub inv_enchant: Option<Vec<(Enchant, u8, u32)>>,
    pub paused: bool,
    pub end_victory: bool,
    pub selected_mission: Option<usize>,
    pub time: f32,
    /// timer d'affichage du toast « cheat F3+L » (déblocage)
    pub cheat_toast_t: f32,
    pub last_frame: std::time::Instant,
    pub game: Option<Game>,
    pub camera: Camera,
    /// caméra du J2 (écran scindé — moitié droite)
    pub camera2: Camera,
    pub xp_before: u32,
    pub mob_tp_done: bool,
    /// F3 debug overlay
    pub debug: bool,
    /// smoothed FPS for the debug overlay
    pub fps: f32,
    /// last frame draw stats (for the debug overlay)
    pub last_boxes: usize,
    pub last_billboards: usize,
    /// static 3D scene behind the menus (Creeper Woods camp, slow orbit)
    pub bg_game: Option<Game>,
}

impl App {
    pub fn new() -> App {
        let mut assets = Assets::load();
        let glyphs = GlyphCache::new(&assets.font);
        let save = Save::load();
        // restore the saved hero skin BEFORE building the skin sheet
        let saved = save.players.first().map(|p| p.skin).unwrap_or(0);
        let armor1 = save.players.first().and_then(|p| armor_class_idx(&p.inventory));
        let armor2 = save.players.get(1).and_then(|p| armor_class_idx(&p.inventory));
        assets.rebuild_skin(saved.min(assets.skin_catalog.len().saturating_sub(1)), armor1, armor2);
        // Combined 64x128 skin sheet: P1 (Steve/custom) at v=0, P2 (Alex) at v=64.
        let mut skins = Vec::new();
        skins.push(SkinRects::from_skin_at(assets.skin_slim, 0));
        skins.push(SkinRects::from_skin_at(assets.skin2_slim, 64));
        let mut app = App {
            window: None,
            gfx: None,
            assets,
            glyphs,
            skins,
            input: Input::new(),
            gilrs: gilrs::Gilrs::new().ok(),
            audio: crate::game::audio::Audio::new(),
            save,
            camp: Camp::new(),
            screen: Screen::MainMenu,
            sel: 0,
            tab: 0,
            show_controls: false,
            hero_open: false,
            hero_sel: 0,
            hero_cursor: 2,
            inv_open: false,
            inv_player: 0,
            inv_sel: 0,
            inv_item_sel: 0,
            inv_enchant: None,
            paused: false,
            end_victory: false,
            selected_mission: None,
            time: 0.0,
            cheat_toast_t: 0.0,
            last_frame: std::time::Instant::now(),
            game: None,
            camera: Camera::new(16.0 / 9.0),
            camera2: Camera::new(8.0 / 9.0),
            xp_before: 0,
            mob_tp_done: false,
            debug: std::env::var("MD_DEBUG").is_ok(),
            fps: 60.0,
            last_boxes: 0,
            last_billboards: 0,
            bg_game: None,
        };
        app.camp.restock(app.hero_power());
        app
    }

    pub fn hero_power(&self) -> u32 {
        self.save
            .players
            .first()
            .map(|p| p.inventory.power())
            .unwrap_or(1)
    }

    // ------------------------------------------------------------------
    // Hero editor (skin selection)
    // ------------------------------------------------------------------

    /// Rebâtit la feuille de skins avec les armures équipées de chaque joueur
    /// (appelé après équipement / ôtage / rebattre / achat boutique).
    fn refresh_skin_armor(&mut self) {
        let idx = self.save.players.first().map(|p| p.skin).unwrap_or(0);
        let armor1 = self.save.players.first().and_then(|p| armor_class_idx(&p.inventory));
        let armor2 = self.save.players.get(1).and_then(|p| armor_class_idx(&p.inventory));
        let idx = idx.min(self.assets.skin_catalog.len().saturating_sub(1));
        self.assets.rebuild_skin(idx, armor1, armor2);
        self.skins[0] = SkinRects::from_skin_at(self.assets.skin_slim, 0);
        if let Some(gfx) = self.gfx.as_mut() {
            if let Some(img) = self.assets.skin.as_ref() {
                gfx.update_skin(img);
            }
        }
    }

    /// Rebuild assets + GPU texture + SkinRects so the menu hero wears `idx`
    /// (l'armure équipée reste portée pendant l'aperçu).
    fn preview_skin(&mut self, idx: usize) {
        let n = self.assets.skin_catalog.len().saturating_sub(1);
        self.hero_sel = if idx > n { n } else { idx };
        let armor1 = self.save.players.first().and_then(|p| armor_class_idx(&p.inventory));
        let armor2 = self.save.players.get(1).and_then(|p| armor_class_idx(&p.inventory));
        self.assets.rebuild_skin(self.hero_sel, armor1, armor2);
        self.skins[0] = SkinRects::from_skin_at(self.assets.skin_slim, 0);
        if let Some(gfx) = self.gfx.as_mut() {
            if let Some(img) = self.assets.skin.as_ref() {
                gfx.update_skin(img);
            }
        }
    }

    fn open_hero_editor(&mut self) {
        self.hero_open = true;
        self.hero_cursor = 2; // APPLIQUER
        let saved = self.save.players[0].skin.min(self.assets.skin_catalog.len().saturating_sub(1));
        self.preview_skin(saved);
    }

    /// Commit the previewed skin to the save file.
    fn apply_hero_skin(&mut self) {
        self.save.players[0].skin = self.hero_sel;
        self.save.store();
        log::info!("hero skin applied: {} ({})", self.hero_sel, self.assets.skin_catalog[self.hero_sel].name);
        self.hero_open = false;
    }

    /// Close without applying: restore the saved skin on the preview.
    fn close_hero_editor(&mut self) {
        let saved = self.save.players[0].skin.min(self.assets.skin_catalog.len().saturating_sub(1));
        if self.hero_sel != saved {
            self.preview_skin(saved);
        }
        self.hero_open = false;
    }

    fn cycle_skin(&mut self, dir: i32) {
        let n = self.assets.skin_catalog.len() as i32;
        let cur = self.hero_sel as i32;
        let next = ((cur + dir).rem_euclid(n)) as usize;
        self.preview_skin(next);
    }

    fn main_inventory(&self) -> &Inventory {
        &self.save.players[0].inventory
    }

    // ------------------------------------------------------------------
    // Mission lifecycle
    // ------------------------------------------------------------------
    fn start_mission(&mut self, mission_id: usize) {
        let m = &MISSIONS[mission_id];
        let tier = self.save.tier.min(2);
        let seed = self
            .save
            .seed
            .wrapping_add(mission_id as u64 * 7919)
            .wrapping_add(self.save.missions_played as u64 * 104729);
        // le P2 rejoint si une manette est branchée, si le mode co-op est
        // activé dans le camp (clavier J2) ou pour les captures (MD_COOP)
        let wants_p2 = self.input.gamepad_count > 0
            || self.save.coop_p2
            || std::env::var("MD_COOP").is_ok();
        let mut players = Vec::new();
        for (i, ps) in self.save.players.iter().enumerate() {
            if i >= 1 && !wants_p2 {
                break; // pas de P2
            }
            let mut p = PlayerState::new(i, Vec2::ZERO);
            p.inventory = ps.inventory.clone();
            p.level = ps.level;
            p.xp = ps.xp;
            p.hp = p.max_hp();
            players.push(p);
        }
        // BUG FIX co-op : la sauvegarde ne contient qu'UN héros — le P2 doit
        // être SYNTHÉTISÉ (inventaire de départ, Alex) sinon il ne rejoignait
        // jamais, même manette branchée.
        if wants_p2 && players.len() < 2 {
            let mut p = PlayerState::new(1, Vec2::ZERO);
            p.hp = p.max_hp();
            players.push(p);
        }
        let mut game = Game::new(m, tier, seed, players);
        // place players at spawns
        for (i, p) in game.players.iter_mut().enumerate() {
            p.pos = game
                .level
                .spawns
                .get(i)
                .copied()
                .unwrap_or(game.level.spawns[0]);
        }
        game.rebake_static(&self.assets);
        self.xp_before = game.players.first().map(|p| p.xp).unwrap_or(0);
        self.camera.smooth = Vec3::new(
            game.players[0].pos.x,
            0.0,
            game.players[0].pos.y,
        );
        // restore gameplay camera angles (menu backdrop may have left an orbit)
        self.camera.pitch_deg = consts::CAM_PITCH_DEG;
        self.camera.yaw_deg = consts::CAM_YAW_DEG;
        self.camera.dist = consts::CAM_DIST;
        // caméra du J2 : même réglage, posée sur son spawn
        if game.players.len() >= 2 {
            let p1 = game.players[1].pos;
            self.camera2.smooth = Vec3::new(p1.x, 0.0, p1.y);
            self.camera2.pitch_deg = consts::CAM_PITCH_DEG;
            self.camera2.yaw_deg = consts::CAM_YAW_DEG;
            self.camera2.dist = consts::CAM_DIST;
        }
        self.inv_open = false;
        self.inv_enchant = None;
        self.paused = false;
        self.game = Some(game);
        self.screen = Screen::Playing;
        self.audio.set_music(crate::game::audio::Music::Exploration);
    }

    fn finish_mission(&mut self, victory: bool) {
        let Some(game) = self.game.as_ref() else { return };
        // write back player progress
        while self.save.players.len() < game.players.len() {
            self.save.players.push(crate::game::progress::PlayerSave {
                inventory: Inventory::starting(),
                level: 1,
                xp: 0,
                skin: 1, // P2 = Alex
            });
        }
        for (i, p) in game.players.iter().enumerate() {
            self.save.players[i].inventory = p.inventory.clone();
            self.save.players[i].level = p.level;
            self.save.players[i].xp = p.xp;
        }
        if victory {
            self.save.emeralds += game.emeralds;
            if !self.save.completed.contains(&game.mission_id) {
                self.save.completed.push(game.mission_id);
            }
            if game.rune_found {
                if !self.save.secrets_found.contains(&game.mission_id) {
                    self.save.secrets_found.push(game.mission_id);
                }
            }
            // tier unlock on final mission
            if game.mission_id == 11 {
                let t = self.save.tier.min(2);
                self.save.tier_unlocked = self.save.tier_unlocked.max((t + 1).min(2));
            }
            self.camp.restock(self.hero_power());
        }
        self.save.missions_played += 1;
        self.save.store();
        self.end_victory = victory;
        self.screen = Screen::End;
        self.sel = 0;
        self.audio.set_music(if victory {
            crate::game::audio::Music::Victory
        } else {
            crate::game::audio::Music::Menu
        });
    }

    // ------------------------------------------------------------------
    // Inventory helpers
    // ------------------------------------------------------------------
    fn active_inventory(&mut self) -> &mut Inventory {
        let i = self.inv_player.min(self.save.players.len().saturating_sub(1));
        &mut self.save.players[i].inventory
    }

    fn inv_activate(&mut self) {
        let sel = self.inv_sel;
        if self.inv_enchant.is_some() {
            // enchant overlay selection
            let Some(list) = self.inv_enchant.clone() else { return };
            let idx = sel.checked_sub(200);
            if let Some(i) = idx {
                if let Some((en, tier, cost)) = list.get(i) {
                    let stash_idx = self.inv_item_sel.saturating_sub(6);
                    let cost = *cost;
                    let en = *en;
                    let tier = *tier;
                    let inv = self.active_inventory();
                    if inv.enchant_points >= cost && inv.enchant(stash_idx, en, tier) {
                        self.inv_enchant = None;
                    }
                }
            }
            return;
        }
        if sel >= 100 {
            // action buttons (100..): operate on the last selected ITEM slot
            let item_sel = self.inv_item_sel;
            match sel - 100 {
                0 => {
                    // Ôter (equip -> stash) or Équiper (stash -> equip)
                    let inv = self.active_inventory();
                    if item_sel < 6 {
                        let taken = match item_sel {
                            0 => inv.melee.take(),
                            1 => inv.ranged.take(),
                            2 => inv.armor.take(),
                            3 => inv.artifacts[0].take(),
                            4 => inv.artifacts[1].take(),
                            _ => inv.artifacts[2].take(),
                        };
                        if let Some(t) = taken {
                            inv.push_stash(t);
                        }
                    } else {
                        let stash_idx = item_sel - 6;
                        if stash_idx < inv.stash.len() {
                            let it = inv.stash.remove(stash_idx);
                            inv.equip(it);
                            self.inv_item_sel = item_sel.min(5);
                        }
                    }
                    self.refresh_skin_armor();
                    self.inv_sel = 100;
                }
                1 => {
                    // Enchanter: build option list (stash items only)
                    let stash_idx = item_sel.saturating_sub(6);
                    let inv = self.active_inventory();
                    if let Some(item) = inv.stash.get(stash_idx) {
                        let mut list = Vec::new();
                        for e in [
                            Enchant::Sharpness,
                            Enchant::CriticalHit,
                            Enchant::Swirling,
                            Enchant::Thundering,
                            Enchant::Radiance,
                            Enchant::FireAspect,
                            Enchant::PoisonCloud,
                            Enchant::Gravity,
                        ] {
                            if !e.applies_to(item.slot()) {
                                continue;
                            }
                            if item.enchants.iter().any(|(x, _)| *x == e) {
                                continue;
                            }
                            if item.enchants.len() >= item.rarity.slots() {
                                continue;
                            }
                            for tier in 1..=3u8 {
                                list.push((e, tier, Enchant::cost(tier)));
                            }
                        }
                        if !list.is_empty() {
                            self.inv_enchant = Some(list);
                            self.inv_sel = 200;
                        }
                    }
                }
                2 => {
                    // Rebattre (salvage a stash item)
                    let stash_idx = item_sel.saturating_sub(6);
                    let inv = self.active_inventory();
                    if stash_idx < inv.stash.len() {
                        if let Some((emeralds, _pts)) = inv.salvage(stash_idx) {
                            self.save.emeralds += emeralds;
                            self.save.store();
                            self.inv_item_sel = item_sel.saturating_sub(1).max(6);
                            self.refresh_skin_armor();
                        }
                    }
                    self.inv_sel = 102;
                }
                _ => {}
            }
            return;
        }
        // item slot activation: equipment -> unequip; stash -> equip
        self.inv_item_sel = sel;
        if sel < 6 {
            let inv = self.active_inventory();
            let taken = match sel {
                0 => inv.melee.take(),
                1 => inv.ranged.take(),
                2 => inv.armor.take(),
                3 => inv.artifacts[0].take(),
                4 => inv.artifacts[1].take(),
                _ => inv.artifacts[2].take(),
            };
            if let Some(t) = taken {
                inv.push_stash(t);
            }
        } else {
            let stash_idx = sel - 6;
            let inv = self.active_inventory();
            if stash_idx < inv.stash.len() {
                let it = inv.stash.remove(stash_idx);
                inv.equip(it);
            }
        }
        self.refresh_skin_armor();
        self.save.store();
    }

    /// Map combined sel index to stash index (6 + i = stash slot i).
    #[allow(dead_code)]
    fn inv_sel_stash(&self) -> usize {
        self.inv_sel.saturating_sub(6)
    }

    // ------------------------------------------------------------------
    // Per-frame update
    // ------------------------------------------------------------------
    fn update(&mut self, dt: f32) {
        self.time += dt;
        self.audio.update();
        let nav: UINav = self.input.ui_nav();

        // F3 toggles the debug overlay on any screen
        if self.input.key_pressed(KeyCode::F3) {
            self.debug = !self.debug;
        }
        // F3 maintenu + L : cheat de déblocage — toutes les missions non
        // secrètes complétées, tous les secrets trouvés, palier Apocalypse
        if self.input.key(KeyCode::F3) && self.input.key_pressed(KeyCode::KeyL) {
            let n = MISSIONS.len();
            let done: Vec<usize> = (0..n).filter(|&i| !MISSIONS[i].is_secret).collect();
            let secrets: Vec<usize> = (0..n).filter(|&i| MISSIONS[i].is_secret).collect();
            log::info!(
                "cheat F3+L : {} missions + {} secrets débloqués, palier Apocalypse",
                done.len(),
                secrets.len()
            );
            self.save.completed = done;
            self.save.secrets_found = secrets;
            self.save.tier_unlocked = 2;
            self.save.store();
            self.cheat_toast_t = 6.0;
        }
        self.cheat_toast_t = (self.cheat_toast_t - dt).max(0.0);
        // smooth FPS for the debug overlay
        let inst = 1.0 / dt.max(0.0001);
        self.fps += (inst - self.fps) * 0.06;

        // lazily build the menu backdrop scene (Creeper Woods, no boss)
        if self.bg_game.is_none() && self.time > 0.3 {
            let m = &MISSIONS[0];
            let mut bg = Game::new(m, 0, 0xA11CE, Vec::new());
            // clairière autour du spawn : la caméra rapprochée de la page titre
            // doit voir le héros, le feu et la tente sans massif devant
            let c0 = bg.level.spawns[0];
            bg.level.carve_plaza(c0, 6.5);
            // les captifs (et leur cage disparue) n'ont pas leur place dans le décor
            bg.world.clear();
            bg.rebake_static(&self.assets);
            self.bg_game = Some(bg);
            let c = self.bg_game.as_ref().unwrap().level.spawns[0];
            self.camera.smooth = Vec3::new(c.x, 0.0, c.y);
            log::info!("menu backdrop scene ready ({} boîtes)", self.bg_game.as_ref().unwrap().static_bake.len());
        }
        // menus/end: cinématique page titre MCD + particules d'ambiance
        // (caméra rapprochée et basse sur le héros, dérive lente — vrai menu)
        if self.screen != Screen::Playing {
            if let Some(bg) = self.bg_game.as_mut() {
                let c0 = bg.level.spawns[0];
                self.camera.pitch_deg = 13.0;
                self.camera.dist = 5.8;
                self.camera.yaw_deg = 26.0 + 7.0 * (self.time * 0.09).sin();
                self.camera.update(Vec3::new(c0.x, 1.08, c0.y), dt);
                // braises du feu de camp (même offset que models::push_menu_camp)
                let yr = self.camera.yaw_deg.to_radians();
                let right = Vec2::new(yr.cos(), -yr.sin());
                let look = Vec2::new(-yr.sin(), -yr.cos());
                let fire = Vec2::new(c0.x, c0.y) - right * crate::models::MENU_FIRE_OFF
                    + look * crate::models::MENU_FIRE_LOOK;
                bg.particles.spawn_fire(fire, dt);
                bg.spawn_ambient(Vec2::new(c0.x, c0.y), dt);
                bg.particles.update(dt);
            }
        }

        // headless visual test: MD_SCREEN=camp jumps straight to the camp
        if std::env::var("MD_SCREEN").as_deref() == Ok("camp") && self.screen == Screen::MainMenu && self.time > 1.0 {
            self.screen = Screen::Camp;
            self.sel = 4;
            self.selected_mission = Some(self.save.next_mission());
        }
        // headless visual test: MD_SCREEN=hero opens the hero editor (captures)
        if std::env::var("MD_SCREEN").as_deref() == Ok("hero") && self.screen == Screen::MainMenu && self.time > 1.0 && !self.hero_open {
            self.open_hero_editor();
            // park the preview on a hero-pack skin so captures show a change
            self.cycle_skin(2);
        }
        // headless visual test: MD_UNLOCK=1 simule le cheat F3+L (débloque tout)
        if std::env::var("MD_UNLOCK").is_ok() && self.time > 1.5 && self.cheat_toast_t <= 0.0 && std::env::var("MD_UNLOCK_DONE").is_err() {
            std::env::set_var("MD_UNLOCK_DONE", "1");
            self.save.completed = (0..MISSIONS.len()).filter(|&i| !MISSIONS[i].is_secret).collect();
            self.save.secrets_found = (0..MISSIONS.len()).filter(|&i| MISSIONS[i].is_secret).collect();
            self.save.tier_unlocked = 2;
            self.save.store();
            self.cheat_toast_t = 6.0;
            log::info!("MD_UNLOCK : {} missions + {} secrets + palier Apocalypse", self.save.completed.len(), self.save.secrets_found.len());
        }
        // headless visual test: MD_AUTO[=<mission id>] launches that mission automatically
        if std::env::var("MD_AUTO").is_ok() && self.screen == Screen::MainMenu && self.time > 1.0 {
            let mid: usize = std::env::var("MD_AUTO").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
            self.start_mission(mid.min(crate::world::missions::MISSIONS.len() - 1));
        }
        // headless visual test: warp player to the first spawn group to meet enemies
        if std::env::var("MD_AUTO").is_ok() && self.screen == Screen::Playing && !self.mob_tp_done && self.time > 2.0 {
            self.mob_tp_done = true;
            if let Some(game) = self.game.as_mut() {
                if let Some(g0) = game.level.groups.first().map(|g| g.pos) {
                    log::info!("group kinds: {:?}", game.level.groups.first().map(|g| g.kinds.clone()).unwrap_or_default());
                    if let Some(p) = game.players.first_mut() {
                        p.pos = g0 + glam::Vec2::new(1.0, 0.0);
                    }
                    log::info!("mob test: player warped to first spawn group at {g0:?}");
                }
            }
        }
        match self.screen {
            Screen::MainMenu => {
                if self.hero_open {
                    // ---- hero editor: carousel + cursor ----
                    if nav.left { self.cycle_skin(-1); }
                    if nav.right { self.cycle_skin(1); }
                    if nav.up { self.hero_cursor = self.hero_cursor.saturating_sub(1); }
                    if nav.down { self.hero_cursor = (self.hero_cursor + 1).min(3); }
                    if nav.cancel {
                        self.close_hero_editor();
                    } else if nav.confirm {
                        match self.hero_cursor {
                            0 => self.cycle_skin(-1),
                            1 => self.cycle_skin(1),
                            2 => self.apply_hero_skin(),
                            _ => self.close_hero_editor(),
                        }
                    }
                } else {
                if nav.up { self.sel = self.sel.saturating_sub(1); }
                if nav.down { self.sel = (self.sel + 1).min(4); }
                if nav.left { self.sel = self.sel.saturating_sub(1); }
                if nav.right { self.sel = (self.sel + 1).min(4); }
                // raccourcis directs façon MCD : H = éditeur de héros, F1/F2 = options
                if !self.show_controls && self.input.key_pressed(KeyCode::KeyH) {
                    self.open_hero_editor();
                }
                if !self.show_controls
                    && (self.input.key_pressed(KeyCode::F1) || self.input.key_pressed(KeyCode::F2))
                {
                    self.show_controls = true;
                }
                if nav.confirm && !self.show_controls {
                    super::game::audio::sfx(super::game::audio::Sfx::UiClick);
                    match self.sel {
                        0 => {
                            self.screen = Screen::Camp;
                            self.sel = 0;
                            // preselect the next mission to run
                            self.selected_mission = Some(self.save.next_mission());
                            self.audio.set_music(crate::game::audio::Music::Menu);
                        }
                        1 => self.open_hero_editor(),
                        2 | 3 => self.show_controls = true,
                        _ => {
                            self.save.store();
                            if let Some(w) = self.window.clone() {
                                w.set_visible(false);
                            }
                            std::process::exit(0);
                        }
                    }
                }
                if nav.cancel {
                    if self.show_controls {
                        self.show_controls = false;
                    } else {
                        // nothing above the main menu
                    }
                }
                // while the controls overlay is open, Enter/click just closes it
                if self.show_controls && (nav.confirm || nav.cancel) {
                    self.show_controls = false;
                }
                }
            }
            Screen::Camp => {
                if self.inv_open {
                    self.update_inventory_nav(&nav);
                } else {
                    let n = self.camp_rect_count();
                    if nav.left { self.sel = self.sel.saturating_sub(1); }
                    if nav.right { self.sel = self.sel.saturating_add(1).min(n.saturating_sub(1)); }
                    if nav.up { self.sel = self.sel.saturating_sub(1); }
                    if nav.down { self.sel = self.sel.saturating_add(1).min(n.saturating_sub(1)); }
                    // track selected mission for the LAUNCH button
                    if self.tab == 0 && self.sel >= 4 && self.sel < 4 + MISSIONS.len() {
                        self.selected_mission = Some(self.sel - 4);
                    }
                    if nav.cancel {
                        if self.tab != 0 {
                            self.tab = 0;
                            self.sel = 0;
                        } else {
                            // back to the main menu
                            self.screen = Screen::MainMenu;
                            self.sel = 0;
                            self.audio.set_music(crate::game::audio::Music::Menu);
                        }
                    }
                    if nav.confirm {
                        self.camp_activate();
                    }
                }
            }
            Screen::Playing => {
                if self.inv_open {
                    self.update_inventory_nav(&nav);
                } else if self.paused {
                    if nav.up { self.sel = self.sel.saturating_sub(1); }
                    if nav.down { self.sel = (self.sel + 1).min(2); }
                    if nav.confirm {
                        match self.sel {
                            0 => self.paused = false,
                            1 => {
                                self.inv_open = true;
                                self.inv_player = 0;
                                self.inv_sel = 0;
                            }
                            _ => self.finish_mission(false),
                        }
                    }
                    if nav.cancel {
                        self.paused = false;
                    }
                } else {
                    if self.input.key_pressed(KeyCode::Escape) {
                        self.paused = true;
                        self.sel = 0;
                    }
                    if self.input.key_pressed(KeyCode::KeyI) || self.input.key_pressed(KeyCode::Tab) {
                        self.inv_open = true;
                        self.inv_player = 0;
                        self.inv_sel = 0;
                        super::game::audio::sfx(super::game::audio::Sfx::UiOpen);
                    }
                    // gamepad pause / inventory
                    if self.input.key_pressed(KeyCode::F1) {
                        self.inv_open = true;
                        super::game::audio::sfx(super::game::audio::Sfx::UiOpen);
                    }
                }
            }
            Screen::End => {
                if nav.confirm {
                    self.screen = Screen::Camp;
                    self.sel = 0;
                    self.game = None;
                    self.audio.set_music(crate::game::audio::Music::Menu);
                }
            }
        }

        // game simulation
        if self.screen == Screen::Playing && !self.paused && !self.inv_open {
            if let Some(game) = self.game.as_mut() {
                // écran scindé actif dès que 2 héros sont en mission
                let p2_active = game.players.len() >= 2;
                let mut inputs = vec![self.input.p1_input(p2_active)];
                if p2_active {
                    inputs.push(self.input.p2_input());
                }
                // MD_BOT: attract-mode — P1 walks and swings constantly
                // (headless visual tests of combat VFX). MD_BOT=south walks
                // toward the camera so the face/head can be inspected.
                if std::env::var("MD_BOT").is_ok() {
                    let south = std::env::var("MD_BOT").as_deref() == Ok("south");
                    inputs[0].mv = glam::Vec2::new(0.0, if south { 1.0 } else { -1.0 });
                    inputs[0].melee = !south;
                    inputs[0].ranged = !south && self.time % 2.0 < 1.0;
                }
                game.update(dt, &inputs);
                // headless visual test: MD_ROLL_AT=<t> déclenche une roulade
                // à t secondes de jeu (planche contact du salto)
                if let Ok(t) = std::env::var("MD_ROLL_AT") {
                    let at: f32 = t.parse().unwrap_or(3.5);
                    if game.time >= at && std::env::var("MD_ROLL_FIRED").is_err() {
                        std::env::set_var("MD_ROLL_FIRED", "1");
                        if let Some(p) = game.players.first_mut() {
                            if p.roll_t <= 0.0 {
                                p.roll_t = consts::ROLL_TIME;
                                p.roll_cd = consts::ROLL_COOLDOWN;
                                p.roll_dir = p.forward();
                                p.iframes = p.iframes.max(consts::ROLL_TIME);
                                log::info!("MD_ROLL_AT : roulade déclenchée à t={:.2}", game.time);
                            }
                        }
                    }
                }
                // headless visual test: MD_ROLL_FREEZE=<0..1> fige le salto à
                // cette progression (pose exacte pour captures)
                if let Ok(fp) = std::env::var("MD_ROLL_FREEZE") {
                    if let Ok(f) = fp.parse::<f32>() {
                        if let Some(p) = game.players.first_mut() {
                            p.roll_t = consts::ROLL_TIME;
                            p.anim.roll = f.clamp(0.0, 1.0);
                            // salto sur place (pas de dérive pendant la capture)
                            p.roll_dir = Vec2::ZERO;
                        }
                    }
                }
                // camera target
                if game.players.len() >= 2 {
                    // ÉCRAN SCINDÉ : chaque caméra suit SON héros (grossissement
                    // fixe — les moitiés 8:9 n'ont pas besoin du zoom-out shared)
                    let p0 = game.players[0].pos;
                    let p1 = game.players[1].pos;
                    self.camera.dist = consts::CAM_DIST;
                    self.camera.update(Vec3::new(p0.x, 0.0, p0.y), dt);
                    self.camera2.dist = consts::CAM_DIST;
                    self.camera2.update(Vec3::new(p1.x, 0.0, p1.y), dt);
                } else {
                    let alive: Vec<Vec2> = game
                        .players
                        .iter()
                        .filter(|p| p.downed_t.is_none())
                        .map(|p| p.pos)
                        .collect();
                    let target = if alive.is_empty() {
                        game.players.first().map(|p| p.pos).unwrap_or(Vec2::ZERO)
                    } else {
                        alive.iter().sum::<Vec2>() / alive.len() as f32
                    };
                    let dist = game
                        .players
                        .iter()
                        .filter(|p| p.downed_t.is_none())
                        .map(|p| p.pos.distance(target))
                        .fold(0.0f32, f32::max);
                    self.camera.dist = (consts::CAM_DIST + dist * 0.55).min(26.0);
                    self.camera
                        .update(Vec3::new(target.x, 0.0, target.y), dt);
                }
                // end conditions
                if game.state == RunState::Victory {
                    self.finish_mission(true);
                } else if game.state == RunState::Failed {
                    self.finish_mission(false);
                }
            }
        }
        // (menu/end screens: the camera is driven by the backdrop orbit above)

        self.input.end_frame();
    }

    fn update_inventory_nav(&mut self, nav: &UINav) {
        if let Some(list) = &self.inv_enchant {
            if nav.up { self.inv_sel = (self.inv_sel - 200).saturating_sub(1).min(list.len() - 1) + 200; }
            if nav.down {
                let i = (self.inv_sel - 200).saturating_add(1).min(list.len() - 1);
                self.inv_sel = 200 + i;
            }
            if nav.cancel {
                self.inv_enchant = None;
                self.inv_sel = 100;
            }
            if nav.confirm {
                self.inv_activate();
            }
            return;
        }
        // navigation domain: item slots 0..6+stash, action buttons 100..=102
        let stash = self.save.players[self.inv_player.min(self.save.players.len() - 1)].inventory.stash.len();
        let item_max = 6 + stash; // exclusive
        if nav.left {
            self.inv_sel = if self.inv_sel >= 100 {
                if self.inv_sel == 100 { item_max.saturating_sub(1) } else { self.inv_sel - 1 }
            } else {
                self.inv_sel.saturating_sub(1)
            };
        }
        if nav.right {
            self.inv_sel = if self.inv_sel >= 100 {
                (self.inv_sel + 1).min(102)
            } else if self.inv_sel + 1 >= item_max {
                100
            } else {
                self.inv_sel + 1
            };
        }
        if nav.up {
            if self.inv_sel >= 100 {
                self.inv_sel = item_max.saturating_sub(1);
            } else {
                self.inv_sel = self.inv_sel.saturating_sub(3);
            }
        }
        if nav.down {
            if self.inv_sel >= 100 {
                // already on the action bar
            } else {
                let n = self.inv_sel + 3;
                self.inv_sel = if n >= item_max { 100 } else { n };
            }
        }
        self.inv_sel = self.inv_sel.min(if self.inv_sel >= 100 { 102 } else { item_max.saturating_sub(1) });
        if self.inv_sel < 100 {
            self.inv_item_sel = self.inv_sel;
        }
        if self.input.key_pressed(KeyCode::Escape) || nav.cancel {
            if self.screen == Screen::Playing {
                self.inv_open = false;
            } else {
                self.inv_open = false;
                self.tab = 0;
            }
            self.inv_enchant = None;
        }
        if nav.confirm || self.input.key_pressed(KeyCode::KeyI) {
            self.inv_activate();
        }
        if self.input.key_pressed(KeyCode::KeyE) && self.screen == Screen::Playing {
            // quick equip toggle
            self.inv_activate();
        }
    }

    fn camp_rect_count(&self) -> usize {
        4 + match self.tab {
            0 => MISSIONS.len() + 3 + 2, // missions + tiers + co-op + launch
            1 => self.camp.stock.len(),
            2 => self.main_inventory().stash.len(),
            _ => 0,
        }
    }

    fn camp_activate(&mut self) {
        super::game::audio::sfx(super::game::audio::Sfx::UiClick);
        let sel = self.sel;
        if sel < 4 {
            if sel == 3 {
                self.inv_open = true;
                self.inv_player = 0;
                self.inv_sel = 0;
            } else {
                self.tab = sel;
                self.sel = 4;
            }
            return;
        }
        match self.tab {
            0 => {
                let i = sel - 4;
                if i < MISSIONS.len() {
                    // mission select
                    if self.save.is_unlocked(i) {
                        self.start_mission(i);
                    }
                } else if i < MISSIONS.len() + 3 {
                    let tier = i - MISSIONS.len();
                    if tier <= self.save.tier_unlocked {
                        self.save.tier = tier;
                        self.save.store();
                    }
                } else if i == MISSIONS.len() + 3 {
                    // bascule co-op écran scindé (clavier J2 / manette)
                    self.save.coop_p2 = !self.save.coop_p2;
                    self.save.store();
                } else {
                    // launch selected mission (last sel)
                    // find selected mission: track separately — launch uses last selected mission id
                    if let Some(mid) = self.selected_mission {
                        if self.save.is_unlocked(mid) {
                            self.start_mission(mid);
                        }
                    }
                }
            }
            1 => {
                let i = sel - 4;
                if let Some((item, price)) = self.camp.stock.get(i).cloned() {
                    if self.save.emeralds >= price {
                        self.save.emeralds -= price;
                        self.camp.stock.remove(i);
                        self.save.players[0].inventory.equip(item);
                        self.save.store();
                        self.refresh_skin_armor();
                    }
                }
            }
            2 => {
                let i = sel - 4;
                let inv = &mut self.save.players[0].inventory;
                if let Some(item) = inv.stash.get(i).cloned() {
                    let price = forge_price(&item);
                    if self.save.emeralds >= price {
                        self.save.emeralds -= price;
                        let inv = &mut self.save.players[0].inventory;
                        if let Some(it) = inv.stash.get_mut(i) {
                            forge_upgrade(it);
                        }
                        self.save.store();
                    }
                }
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------------
    // Render
    // ------------------------------------------------------------------
    fn render(&mut self) {
        let mut frame = FrameData::default();
        let (w, h) = self
            .gfx
            .as_ref()
            .map(|g| g.size)
            .unwrap_or((1280, 720));

        // camera uniform — gameplay fog, else menu backdrop fog, else default
        // (menu = nuit MCD : brume sombre pour faire ressortir les feux)
        let (fog_src, menu_mode) = match (self.game.as_ref(), self.bg_game.as_ref()) {
            (Some(g), _) => (Some(g), false),
            (None, Some(g)) => (Some(g), true),
            (None, None) => (None, false),
        };
        frame.cam = CameraUniform {
            vp: self.camera.view_proj.to_cols_array_2d(),
            campos: [self.camera.eye.x, self.camera.eye.y, self.camera.eye.z, 1.0],
            fogcolor: if menu_mode {
                [0.07, 0.075, 0.1, 1.0]
            } else {
                fog_src.map(|g| [g.level.fog.0.x, g.level.fog.0.y, g.level.fog.0.z, 1.0]).unwrap_or([0.08, 0.07, 0.1, 1.0])
            },
            fogrange: if menu_mode {
                [20.0, 62.0, 0.0, 0.0]
            } else {
                fog_src.map(|g| [g.level.fog.1, g.level.fog.2, 0.0, 0.0]).unwrap_or([30.0, 90.0, 0.0, 0.0])
            },
        };

        // écran scindé : caméra J2 + fog identique (même niveau)
        let split = self.screen == Screen::Playing
            && self.game.as_ref().map(|g| g.players.len() >= 2).unwrap_or(false);
        if split {
            let mut cam2 = CameraUniform {
                vp: self.camera2.view_proj.to_cols_array_2d(),
                campos: [self.camera2.eye.x, self.camera2.eye.y, self.camera2.eye.z, 1.0],
                fogcolor: frame.cam.fogcolor,
                fogrange: frame.cam.fogrange,
            };
            // screen shake : décalage aléatoire de la caméra J2 aussi
            if let Some(g) = self.game.as_ref() {
                if g.shake > 0.001 {
                    let (dx, dy) = shake_offset(g.shake);
                    apply_shake(&mut cam2, dx, dy);
                }
            }
            frame.cam2 = Some(cam2);
        }
        // screen shake J1 (une seule fois, après fog)
        if let Some(g) = self.game.as_ref() {
            if g.shake > 0.001 {
                let (dx, dy) = shake_offset(g.shake);
                apply_shake(&mut frame.cam, dx, dy);
            }
        }

        // world
        let mut world_drawn = false;
        if self.screen == Screen::Playing {
            if let Some(game) = &self.game {
                let mut boxes_atlas: Vec<BoxInstance> = Vec::with_capacity(game.static_bake.len() + 1024);
                let mut boxes_skin: Vec<BoxInstance> = Vec::new();
                let mut billboards: Vec<BillboardInstance> = Vec::new();
                game.render(&self.assets, &self.skins, &mut boxes_atlas, &mut boxes_skin, &mut billboards);
                if std::env::var("MD_NOTERR").is_ok() {
                    boxes_atlas.clear();
                }
                self.last_boxes = boxes_atlas.len() + boxes_skin.len();
                self.last_billboards = billboards.len();
                frame.boxes_atlas = boxes_atlas;
                frame.boxes_skin = boxes_skin;
                frame.billboards = billboards;
                world_drawn = true;
            }
        } else if matches!(self.screen, Screen::MainMenu | Screen::Camp | Screen::End) {
            // menu backdrop: static scenery + particles only (no simulation)
            if let Some(bg) = &self.bg_game {
                let mut boxes_atlas: Vec<BoxInstance> = Vec::with_capacity(bg.static_bake.len());
                let mut boxes_skin: Vec<BoxInstance> = Vec::new();
                let mut billboards: Vec<BillboardInstance> = Vec::new();
                bg.render(&self.assets, &self.skins, &mut boxes_atlas, &mut boxes_skin, &mut billboards);
                // ---- page titre MCD : camp (feu + tente + caisses) + héros ----
                let c0 = bg.level.spawns[0];
                let hero = Vec3::new(c0.x, 0.0, c0.y);
                crate::models::push_menu_camp(
                    &mut boxes_atlas,
                    &self.assets,
                    hero,
                    self.camera.yaw_deg.to_radians(),
                    self.time,
                );
                if let Some(skin) = self.skins.first() {
                    // héros statique face caméra, respiration subtile
                    let mut anim = crate::models::AnimState::default();
                    anim.moving = 0.05;
                    anim.walk_phase = self.time * 1.6;
                    let hero_yaw = self.camera.yaw_deg.to_radians() + std::f32::consts::PI;
                    crate::models::push_player(&mut boxes_skin, skin, hero, hero_yaw, &anim, Vec4::ONE);
                    // épée tenue en main droite, inclinée vers le bas pour rester
                    // lisible sous la main depuis la caméra de la page titre
                    let arm_total = anim.walk_phase.sin() * 0.55 * anim.moving;
                    let aw = if skin.slim { 0.1875 } else { 0.25 };
                    let pivot = Vec3::new(0.25 + aw / 2.0, 1.4, 0.0);
                    let q_yaw = glam::Quat::from_rotation_y(hero_yaw);
                    let q_loc = glam::Quat::from_axis_angle(Vec3::X, arm_total);
                    let q = q_yaw * q_loc;
                    let hand = hero + q_yaw * pivot + q * Vec3::new(0.0, -0.78, 0.0);
                    let q_sword = q
                        * glam::Quat::from_axis_angle(Vec3::X, 0.5)
                        * glam::Quat::from_axis_angle(Vec3::Z, -0.35)
                        * glam::Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2);
                    let rect = crate::gfx::cell_norm(&self.assets, "wpn_blade");
                    boxes_skin.push(BoxInstance::new(
                        hand,
                        q_sword,
                        Vec3::new(0.11, 0.9, 0.11),
                        [rect; 6],
                        Vec4::ONE,
                        0.12,
                    ));
                }
                // ombre du héros (pas d'anneau de sélection sur la page titre)
                billboards.push(BillboardInstance::new(
                    Vec3::new(hero.x, 0.03, hero.z),
                    [0.6, 0.6],
                    Vec4::new(0.0, 0.0, 0.0, 0.4),
                    0.0,
                ));
                self.last_boxes = boxes_atlas.len();
                self.last_billboards = billboards.len();
                frame.boxes_atlas = boxes_atlas;
                frame.boxes_skin = boxes_skin;
                frame.billboards = billboards;
                world_drawn = true;
            }
        }
        if !world_drawn {
            self.last_boxes = 0;
            self.last_billboards = 0;
        }

        // UI
        {
            let mut quads = std::mem::take(&mut frame.quads);
            let mut text = std::mem::take(&mut frame.text);
            self.draw_ui(&mut quads, &mut text, w as f32, h as f32);
            frame.quads = quads;
            frame.text = text;
        }

        if let Some(gfx) = self.gfx.as_mut() {
            gfx.render(&mut frame);
        }
    }

    fn draw_ui(&mut self, quads: &mut Vec<QuadInstance>, text: &mut Vec<QuadInstance>, w: f32, h: f32) {
        // ---- ÉCRAN SCINDÉ : HUD par moitié, dessiné en premier ----
        // (le reste de l'UI — pause/inventaire/fin — reste plein écran)
        let split = self.screen == Screen::Playing
            && self.game.as_ref().map(|g| g.players.len() >= 2).unwrap_or(false);
        if split {
            if let Some(game) = &self.game {
                let half = w / 2.0;
                let p2_pad = self.input.p2_on_pad();
                // moitié gauche : J1 (clavier/souris)
                {
                    let mut u1 = Ui {
                        quads: &mut *quads,
                        text: &mut *text,
                        glyphs: &self.glyphs,
                        assets: &self.assets,
                        w: half,
                        h,
                        mouse: self.input.mouse_pos,
                        shrink: 0.74,
                    };
                    let _ = ui_mcd::draw_hud(&mut u1, game, &self.camera, 0, &ui_mcd::SCHEME_P1);
                    ui_mcd::draw_player_tag(&mut u1, 0, self.time);
                }
                // moitié droite : J2 — dessinée à x=0 puis décalée sur CPU
                let mut q2: Vec<QuadInstance> = Vec::new();
                let mut t2: Vec<QuadInstance> = Vec::new();
                {
                    let mut u2 = Ui {
                        quads: &mut q2,
                        text: &mut t2,
                        glyphs: &self.glyphs,
                        assets: &self.assets,
                        w: half,
                        h,
                        mouse: self.input.mouse_pos,
                        shrink: 0.74,
                    };
                    let _ = ui_mcd::draw_hud(&mut u2, game, &self.camera2, 1, &ui_mcd::scheme_p2(p2_pad));
                    ui_mcd::draw_player_tag(&mut u2, 1, self.time);
                }
                for mut q in q2 {
                    q.pos[0] += half;
                    quads.push(q);
                }
                for mut t in t2 {
                    t.pos[0] += half;
                    text.push(t);
                }
                // séparateur vertical : ombre + liseré doré fin
                quads.push(QuadInstance {
                    pos: [half - 3.0, 0.0],
                    size: [6.0, h],
                    uv: [0.0; 4],
                    color: [0.02, 0.02, 0.03, 0.85],
                    flag: 0.0,
                    _pad: 0.0,
                });
                quads.push(QuadInstance {
                    pos: [half + 3.0, 0.0],
                    size: [1.5, h],
                    uv: [0.0; 4],
                    color: [0.55, 0.45, 0.22, 0.4],
                    flag: 0.0,
                    _pad: 0.0,
                });
            }
        }
        let mut ui = Ui {
            quads,
            text,
            glyphs: &self.glyphs,
            assets: &self.assets,
            w,
            h,
            mouse: self.input.mouse_pos,
            shrink: 1.0,
        };
        match self.screen {
            Screen::MainMenu => {
                // stats du héros pour le bloc central (niveau, puissance, émeraudes)
                let p1 = &self.save.players[0];
                let inv = &p1.inventory;
                let eq: Vec<u32> = [&inv.melee, &inv.ranged, &inv.armor]
                    .iter()
                    .filter_map(|s| s.as_ref().map(|it| it.power))
                    .collect();
                let power = if eq.is_empty() {
                    1
                } else {
                    (eq.iter().sum::<u32>() / eq.len() as u32).max(1)
                };
                let rects = ui_mcd::draw_main_menu(
                    &mut ui,
                    self.sel,
                    self.time,
                    p1.level,
                    power,
                    self.save.emeralds,
                    self.hero_open,
                );
                // mouse hover (menu hotspots disabled while the editor is open)
                if !self.hero_open {
                    for (i, r) in rects.iter().enumerate() {
                        if ui.hotspot(*r) {
                            self.sel = i;
                        }
                    }
                }
                // ---- hero editor overlay (skin selection, live on the 3D hero) ----
                if self.hero_open {
                    let names: Vec<&str> = self.assets.skin_catalog.iter().map(|e| e.name).collect();
                    let hrects = ui::draw_hero_editor(
                        &mut ui,
                        &names,
                        self.hero_sel,
                        self.hero_cursor,
                        self.time,
                    );
                    for (i, r) in hrects.iter().enumerate() {
                        if ui.hotspot(*r) {
                            self.hero_cursor = i;
                        }
                    }
                }
                if self.show_controls {
                    ui.dim_overlay(0.6);
                    let s = ui.scale();
                    let x = 60.0 * s;
                    let mut y = 100.0 * s;
                    for line in [
                        "COMMANDES — Joueur 1 (clavier / souris)",
                        "Déplacement : ZQSD / WASD     Attaque mêlée : clic gauche",
                        "Tir (visée auto) : clic droit    Roulade : Espace",
                        "Artefacts : 1 / 2 / 3    Potion : F    Interagir : E ou X",
                        "Inventaire : I ou Tab    Pause : Échap    Debug : F3",
                        "",
                        "COMMANDES — Joueur 2 (clavier, écran scindé)",
                        "Déplacement : flèches   Mêlée : U   Tir : O   Roulade : P",
                        "Artefacts : J / K / L   Potion : H   Interagir : Y",
                        "",
                        "COMMANDES — Joueur 2 (manette)",
                        "Déplacement : stick gauche   Mêlée : A/Croix   Tir : B/Cercle",
                        "Roulade : RB   Artefacts : X / Y / L2   Potion : LB   Interagir : haut",
                        "(co-op à activer dans le camp — ou branche une manette)",
                    ] {
                        ui.text(line, x, y, SIZE_SMALL, if line.starts_with("COMMANDES") { crate::ui::ACCENT } else { crate::ui::WHITE });
                        y += 26.0 * s;
                    }
                }
            }
            Screen::Camp => {
                if self.inv_open {
                    let inv = self.save.players[self.inv_player.min(self.save.players.len() - 1)].inventory.clone();
                    let rects = ui::draw_inventory(&mut ui, &inv, self.inv_player, self.inv_sel, self.inv_enchant.clone());
                    for (i, r) in rects.iter().enumerate() {
                        if ui.hotspot(*r) && self.inv_enchant.is_none() {
                            if i >= 100 {
                                self.inv_sel = 100 + i.saturating_sub(100).min(2);
                            } else {
                                self.inv_sel = i;
                                self.inv_item_sel = i;
                            }
                        }
                    }
                } else {
                    let inv = self.save.players[0].inventory.clone();
                    let rects = ui::draw_camp(&mut ui, &self.save, &inv, &self.camp, self.tab, self.sel);
                    for (i, r) in rects.iter().enumerate() {
                        if ui.hotspot(*r) {
                            self.sel = i;
                        }
                    }
                }
            }
            Screen::Playing => {
                if let Some(game) = &self.game {
                    // HUD plein écran uniquement hors split (le split l'a déjà dessiné)
                    let split = game.players.len() >= 2;
                    if !split {
                        let rects_hud = ui_mcd::draw_hud(&mut ui, game, &self.camera, 0, &ui_mcd::SCHEME_P1);
                        let _ = rects_hud;
                    }
                }
                if self.inv_open {
                    let inv = self.save.players[self.inv_player.min(self.save.players.len() - 1)].inventory.clone();
                    let rects = ui::draw_inventory(&mut ui, &inv, self.inv_player, self.inv_sel, self.inv_enchant.clone());
                    for (i, r) in rects.iter().enumerate() {
                        if ui.hotspot(*r) && self.inv_enchant.is_none() {
                            if i >= 100 {
                                self.inv_sel = 100 + i.saturating_sub(100).min(2);
                            } else {
                                self.inv_sel = i;
                                self.inv_item_sel = i;
                            }
                        }
                    }
                } else if self.paused {
                    let rects = ui_mcd::draw_pause(&mut ui, self.sel);
                    for (i, r) in rects.iter().enumerate() {
                        if ui.hotspot(*r) {
                            self.sel = i;
                        }
                    }
                }
            }
            Screen::End => {
                if let Some(game) = &self.game {
                    let rects = ui_mcd::draw_end(&mut ui, game, self.end_victory, self.sel, self.save.emeralds, self.xp_before);
                    for (i, r) in rects.iter().enumerate() {
                        if ui.hotspot(*r) {
                            self.sel = i;
                        }
                    }
                }
            }
        }
        // toast « cheat F3+L » : bandeau doré centré
        if self.cheat_toast_t > 0.0 {
            let a = (self.cheat_toast_t / 0.6).min(1.0) as f32;
            let msg = "TOUTES LES MISSIONS ET DIFFICULTÉS DÉBLOQUÉES (F3+L)";
            let tw = ui.measure(msg, 0);
            let cx = ui.w / 2.0;
            let ty = ui.h * 0.14;
            ui.rect(cx - tw / 2.0 - 18.0, ty - 8.0, tw + 36.0, 34.0, [0.32, 0.24, 0.02, 0.88 * a]);
            ui.rect(cx - tw / 2.0 - 15.0, ty - 5.0, tw + 30.0, 28.0, [0.85, 0.66, 0.14, 0.95 * a]);
            ui.rect(cx - tw / 2.0 - 12.0, ty - 2.0, tw + 24.0, 22.0, [0.14, 0.09, 0.01, 0.85 * a]);
            let px = ui.glyphs.px(0);
            ui.text(msg, cx - tw / 2.0, ty + 11.0 - px * 0.55, 0, [1.0, 0.87, 0.4, a]);
        }
        // F3 debug overlay (on top of everything)
        if self.debug {
            let info = ui::DebugInfo {
                fps: self.fps,
                screen: match self.screen {
                    Screen::MainMenu => "menu",
                    Screen::Camp => "camp",
                    Screen::Playing => "mission",
                    Screen::End => "fin",
                },
                boxes: self.last_boxes,
                billboards: self.last_billboards,
            };
            ui::draw_debug(&mut ui, self.game.as_ref(), &info);
        }
    }
}

pub fn run() {
    let event_loop: EventLoop<()> = EventLoop::new().expect("event loop");
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("run");
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = Window::default_attributes()
                .with_title(concat!("Minecraft Dungeons — remake non officiel v", env!("CARGO_PKG_VERSION"), " (usage privé)"))
                .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0));
            let window = Arc::new(event_loop.create_window(attrs).expect("window"));
            log::info!("window created inner_size={:?}", window.inner_size());
            let gfx = pollster::block_on(Gfx::new(window.clone(), &self.assets, &self.glyphs));
            let (w, h) = gfx.size;
            self.camera.aspect = w as f32 / h as f32;
            self.camera2.aspect = (w as f32 / 2.0) / h as f32;
            self.gfx = Some(gfx);
            self.window = Some(window);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.save.store();
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(gfx) = self.gfx.as_mut() {
                    gfx.resize(size.width, size.height);
                    self.camera.aspect = size.width as f32 / size.height.max(1) as f32;
                    // écran scindé : chaque moitié a son propre aspect 8:9 env.
                    self.camera2.aspect = (size.width as f32 / 2.0) / size.height.max(1) as f32;
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let winit::keyboard::PhysicalKey::Code(kc) = event.physical_key {
                    self.input.on_key(event.state, kc);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.input.on_mouse(state, button);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input.mouse_pos = (position.x as f32, position.y as f32);
            }
            WindowEvent::RedrawRequested => {
                let now = std::time::Instant::now();
                let dt = (now - self.last_frame).as_secs_f32().min(0.05);
                self.last_frame = now;
                self.input.poll_gamepads(&mut self.gilrs);
                self.update(dt);
                self.render();
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _loop: &ActiveEventLoop, _id: winit::event::DeviceId, _event: DeviceEvent) {}

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.save.store();
    }
}

// keep unused imports referenced
#[allow(unused)]
fn _t(_q: &QuadInstance, _p: &PickupKind, _e: &enemy::Enemy, _b: &BillboardInstance, _v: &Vec4) {}

// ----------------------------------------------------------------------
// Screen shake — translation de monde aléatoire appliquée AU VP (après
// coup), donc l'effet n'est pas adouci par le lissage de caméra.
// ----------------------------------------------------------------------

fn shake_offset(amp: f32) -> (f32, f32) {
    use rand::Rng;
    let mut r = rand::thread_rng();
    let a = amp.min(0.55);
    (r.gen_range(-a..a), r.gen_range(-a..a))
}

/// Décale la matrice view-proj d'un tremblement (dx, dz en espace monde).
fn apply_shake(cam: &mut CameraUniform, dx: f32, dy: f32) {
    let m = Mat4::from_cols_array_2d(&cam.vp) * Mat4::from_translation(Vec3::new(dx, 0.0, dy));
    cam.vp = m.to_cols_array_2d();
}
