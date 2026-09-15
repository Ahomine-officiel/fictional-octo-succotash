//! Game session: owns the level, entities (hecs), projectiles, particles,
//! pickups, damage numbers, and drives per-frame updates.

pub mod audio;
pub mod combat;
pub mod enemy;
pub mod items;
pub mod loot;
pub mod player;
pub mod progress;
pub mod shop;
pub mod inventory;

use crate::assets::Assets;
use crate::gfx::{self, BillboardInstance, BoxInstance};
use crate::models::{self, AnimState, SkinRects};
use crate::world::gen;
use crate::world::missions::MissionDef;
use glam::{Vec2, Vec3, Vec4};
use hecs::World;

pub use combat::{ParticleSystem, Pickup, PickupKind, Floater, Projectile, ProjKind, StatusKind};
pub use crate::input::PlayerInput;
pub use player::PlayerState;
use hecs::Entity;

pub struct Pos(pub Vec2);
pub struct Health {
    pub hp: f32,
    pub max: f32,
}
pub struct Anim(pub AnimState);
pub struct HitFlash(pub f32);

#[derive(Clone, Copy, PartialEq)]
pub enum RunState {
    Running,
    Victory,
    Failed,
}

pub struct Game {
    pub world: World,
    pub players: Vec<PlayerState>,
    pub projectiles: Vec<Projectile>,
    pub particles: ParticleSystem,
    pub pickups: Vec<Pickup>,
    pub floaters: Vec<Floater>,
    pub zones: Vec<combat::Zone>,
    pub level: crate::world::Level,
    pub mission_id: usize,
    pub tier: usize,
    pub time: f32,
    pub emeralds: u32,
    pub boss_entity: Option<hecs::Entity>,
    pub boss_phase: u32,
    pub state: RunState,
    pub captives_rescued: u32,
    pub rune_found: bool,
    pub seed: u64,
    pub static_bake: Vec<BoxInstance>,
    pub kills: u32,
    pub items_found: Vec<items::Item>,
    /// chest open animation set (index into level.chests already opened)
    pub opened: std::collections::HashSet<(i32, i32)>,
}

impl Game {
    /// Continuous ambient particles (menu backdrop + tests use this too).
    pub fn spawn_ambient(&mut self, center: Vec2, dt: f32) {
        self.particles.spawn_ambient(self.level.biome, center, dt);
    }

    pub fn new(mission: &MissionDef, tier: usize, seed: u64, players: Vec<PlayerState>) -> Game {
        let level = gen::generate(mission, seed);
        let mut world = World::new();

        // captive NPCs near cages
        for &pos in &level.captives {
            let _e = world.spawn((
                enemy::Enemy::captive(),
                Pos(pos + Vec2::new(0.0, 0.6)),
                Health { hp: 20.0, max: 20.0 },
                Anim(AnimState::default()),
                HitFlash(0.0),
            ));
        }

        let mut g = Game {
            world,
            players,
            projectiles: Vec::new(),
            particles: ParticleSystem::new(),
            pickups: Vec::new(),
            floaters: Vec::new(),
            zones: Vec::new(),
            level,
            mission_id: mission.id,
            tier,
            time: 0.0,
            emeralds: 0,
            boss_entity: None,
            boss_phase: 1,
            state: RunState::Running,
            captives_rescued: 0,
            rune_found: false,
            seed,
            static_bake: Vec::new(),
            kills: 0,
            items_found: Vec::new(),
            opened: std::collections::HashSet::new(),
        };

        // spawn boss if any (deferred activation handled by arena proximity)
        if let Some(bs) = g.level.boss_spawn {
            let kind = match mission.boss.unwrap() {
                crate::world::missions::BossKind::CorruptedCauldron => "boss_cauldron",
                crate::world::missions::BossKind::RedstoneMonstrosity => "boss_monstrosity",
                crate::world::missions::BossKind::NamelessOne => "boss_nameless",
                crate::world::missions::BossKind::ArchIllager => "boss_arch",
                crate::world::missions::BossKind::MiniGolemHorde => "redstone_golem",
                crate::world::missions::BossKind::JungleAbomination => "boss_jungle",
                crate::world::missions::BossKind::WretchedWraith => "boss_wretched",
                crate::world::missions::BossKind::TempestGolem => "boss_tempest",
                crate::world::missions::BossKind::AncientGuardian => "boss_ancient",
                crate::world::missions::BossKind::Wildfire => "boss_wildfire",
                crate::world::missions::BossKind::VengefulHeart => "boss_vengeful",
            };
            let e = enemy::spawn_enemy(&mut g.world, kind, bs, tier);
            if kind.starts_with("boss_") {
                g.boss_entity = Some(e);
            }
        }

        g
    }

    pub fn players_alive(&self) -> usize {
        self.players.iter().filter(|p| p.downed_t.is_none()).count()
    }

    pub fn nearest_player(&self, pos: Vec2) -> Option<(usize, f32)> {
        let mut best: Option<(usize, f32)> = None;
        for (i, p) in self.players.iter().enumerate() {
            if p.downed_t.is_some() { continue; }
            let d = p.pos.distance_squared(pos);
            if best.map(|(_, bd)| d < bd).unwrap_or(true) {
                best = Some((i, d));
            }
        }
        best.map(|(i, d)| (i, d.sqrt()))
    }

    pub fn total_threat(&self) -> u32 {
        crate::world::missions::MISSIONS[self.mission_id].threat
    }

    /// Activate spawn groups near any alive player.
    fn update_triggers(&mut self) {
        for g in self.level.groups.iter_mut() {
            if g.activated { continue; }
            for p in &self.players {
                if p.downed_t.is_some() { continue; }
                if p.pos.distance(g.pos) < g.radius {
                    g.activated = true;
                    let tier = self.tier;
                    for kind in &g.kinds {
                        // spawn in a ring around the trigger with a telegraph
                        let off = Vec2::new(
                            (self.time * 7.7 + kind.len() as f32).sin() * 2.0,
                            (self.time * 5.3 + kind.len() as f32).cos() * 2.0,
                        );
                        let pos = g.pos + off;
                        let e = enemy::spawn_enemy(&mut self.world, kind, pos, tier);
                        self.particles.spawn_burst(Vec3::new(pos.x, 0.5, pos.y), 10, Vec4::new(0.4, 0.3, 0.7, 0.8), 3.0);
                        let _ = e;
                    }
                    break;
                }
            }
        }
    }

    pub fn update(&mut self, dt: f32, inputs: &[PlayerInput]) {
        if self.state != RunState::Running {
            return;
        }
        self.time += dt;

        // players
        for (i, input) in inputs.iter().enumerate() {
            player::update_player(self, i, input, dt);
        }

        self.update_triggers();
        enemy::update_enemies(self, dt);
        combat::update_projectiles(self, dt);
        combat::update_zones(self, dt);
        combat::update_artifact_auras(self, dt);
        self.particles.update(dt);
        // ambient biome particles around the action (nappe MCD)
        let focus = self
            .players
            .first()
            .map(|p| p.pos)
            .unwrap_or(self.level.spawns[0]);
        self.particles.spawn_ambient(self.level.biome, focus, dt);
        loot::update_pickups(self, dt);

        // floaters
        for f in self.floaters.iter_mut() {
            f.life -= dt;
            f.pos.y += dt * 1.4;
        }
        self.floaters.retain(|f| f.life > 0.0);

        // hit flashes decay
        for hf in self.world.query::<&mut HitFlash>().iter() {
            hf.0 = (hf.0 - dt * 5.0).max(0.0);
        }

        // portal activation + victory check
        self.check_portal(dt);
    }

    fn check_portal(&mut self, _dt: f32) {
        // portal activates when boss dead (or, for boss-less missions, when all groups cleared)
        let portal_open = match self.boss_entity {
            Some(e) => self.world.get::<&Health>(e).map(|h| h.hp <= 0.0).unwrap_or(true),
            None => self.level.groups.iter().filter(|g| g.activated).count() >= 1
                && self.kills >= 3,
        };
        if let Some((_, active)) = self.level.portal.as_mut() {
            if !*active && portal_open {
                *active = true;
                if let Some(pos) = self.level.portal.map(|(p, _)| p) {
                    self.particles.spawn_burst(Vec3::new(pos.x, 1.5, pos.y), 40, Vec4::new(0.5, 1.0, 0.9, 1.0), 5.0);
                }
            }
        }
        // victory when a player reaches an active portal
        if let Some((pos, active)) = self.level.portal {
            if active {
                for p in &self.players {
                    if p.downed_t.is_none() && p.pos.distance(pos) < 1.6 {
                        self.state = RunState::Victory;
                    }
                }
            }
        }
    }

    /// All enemies dead check for "horde finale" missions
    pub fn boss_name(&self) -> Option<&'static str> {
        let m = &crate::world::missions::MISSIONS[self.mission_id];
        m.boss.map(|b| match b {
            crate::world::missions::BossKind::CorruptedCauldron => "Chaudron Corrompu",
            crate::world::missions::BossKind::RedstoneMonstrosity => "Monstruosité Redstone",
            crate::world::missions::BossKind::NamelessOne => "le Sans-Nom",
            crate::world::missions::BossKind::ArchIllager => "Arch-Illageois",
            crate::world::missions::BossKind::MiniGolemHorde => "la Garde Golem",
            crate::world::missions::BossKind::JungleAbomination => "Abomination de la Jungle",
            crate::world::missions::BossKind::WretchedWraith => "Spectre Repoussant",
            crate::world::missions::BossKind::TempestGolem => "Golem Tempête",
            crate::world::missions::BossKind::AncientGuardian => "Gardien Antique",
            crate::world::missions::BossKind::Wildfire => "Feu Sauvage",
            crate::world::missions::BossKind::VengefulHeart => "Cœur d'Ender Vengeur",
        })
    }

    // ------------------------------------------------------------------
    // Rendering
    // ------------------------------------------------------------------
    pub fn render(
        &self,
        assets: &Assets,
        skins: &[SkinRects],
        boxes_atlas: &mut Vec<BoxInstance>,
        boxes_skin: &mut Vec<BoxInstance>,
        billboards: &mut Vec<BillboardInstance>,
    ) {
        // static level
        boxes_atlas.extend_from_slice(&self.static_bake);

        // enemies & captives
        for (e, en, pos, health, anim, flash) in self
            .world
            .query::<(Entity, &enemy::Enemy, &Pos, &Health, &Anim, &HitFlash)>()
            .iter()
        {
            let model = enemy::model_for(&en.kind);
            let mut tint = Vec4::new(1.0, 1.0, 1.0, 1.0);
            if flash.0 > 0.0 {
                tint = Vec4::new(1.0 + flash.0, 1.0 - flash.0 * 0.5, 1.0 - flash.0 * 0.5, 1.0);
            }
            for s in &en.status {
                match s.kind {
                    StatusKind::Poison => tint = Vec4::new(0.7, 1.0, 0.7, 1.0),
                    StatusKind::Burn => tint = Vec4::new(1.2, 0.8, 0.6, 1.0),
                    StatusKind::Slow => {}
                }
            }
            if en.captive {
                // cage baked statically; draw villager inside
            }
            let mut a = anim.0;
            if health.hp <= 0.0 && en.dying_t > 0.0 {
                a.dead = (1.0 - en.dying_t / enemy::DEATH_TIME).min(1.0);
            }
            models::push_model(boxes_atlas, model, Vec3::new(pos.0.x, 0.0, pos.0.y), en.yaw, &a, assets, tint);

            // shadow
            let sc = 0.55 * model.scale;
            billboards.push(BillboardInstance::new(
                Vec3::new(pos.0.x, 0.03, pos.0.y),
                [sc, sc],
                Vec4::new(0.0, 0.0, 0.0, 0.35),
                0.0,
            ));

            // small HP bar when damaged
            if health.hp < health.max && health.hp > 0.0 && !en.captive {
                let frac = (health.hp / health.max).clamp(0.0, 1.0);
                let y = 2.2 * model.scale + 0.3;
                billboards.push(BillboardInstance::new(
                    Vec3::new(pos.0.x, y, pos.0.y),
                    [1.1, 0.14],
                    Vec4::new(0.1, 0.1, 0.1, 0.8),
                    1.0,
                ));
                billboards.push(BillboardInstance::new(
                    Vec3::new(pos.0.x, y, pos.0.y),
                    [1.06 * frac, 0.1],
                    Vec4::new(0.9, 0.15, 0.15, 0.9),
                    1.0,
                ));
            }
            let _ = e;
        }

        // players
        for (i, p) in self.players.iter().enumerate() {
            let tint = if p.iframes > 0.0 && p.roll_t > 0.0 {
                Vec4::new(1.0, 1.0, 1.0, 0.55)
            } else if p.downed_t.is_some() {
                Vec4::new(1.0, 0.5, 0.5, 0.9)
            } else {
                Vec4::ONE
            };
            if let Some(skin) = skins.get(i) {
                models::push_player(boxes_skin, skin, Vec3::new(p.pos.x, 0.0, p.pos.y), p.yaw, &p.anim, tint);
                // held melee weapon in the right hand — the swing is now READABLE
                // (MCD heroes always carry their weapon; ref_46 swing arc)
                if p.downed_t.is_none() && p.roll_t <= 0.0 {
                    let swing_a = if p.anim.swing > 0.0 {
                        -2.1 * (1.0 - p.anim.swing) * p.anim.swing * 4.0
                    } else {
                        0.0
                    };
                    let arm_total = p.anim.walk_phase.sin() * 0.55 * p.anim.moving + swing_a;
                    let aw = if skin.slim { 0.1875 } else { 0.25 };
                    let pivot = Vec3::new(0.25 + aw / 2.0, 1.4, 0.0);
                    let q_yaw = glam::Quat::from_rotation_y(p.yaw);
                    let q_loc = glam::Quat::from_axis_angle(Vec3::X, arm_total);
                    let q = q_yaw * q_loc;
                    let hand = Vec3::new(p.pos.x, 0.0, p.pos.y) + q_yaw * pivot + q * Vec3::new(0.0, -0.78, 0.0);
                    // tilt the blade forward so it reads as a sword, not a stick
                    let q_sword = q * glam::Quat::from_axis_angle(Vec3::X, 1.2);
                    let rect = gfx::cell_norm(assets, "wpn_blade");
                    boxes_skin.push(BoxInstance::new(
                        hand,
                        q_sword,
                        Vec3::new(0.11, 0.9, 0.11),
                        [rect; 6],
                        Vec4::ONE,
                        0.12,
                    ));
                }
            }
            // MCD-style selection ring: gold for P1, crimson for P2
            let ring = if i == 0 {
                Vec4::new(1.0, 0.78, 0.16, 0.55)
            } else {
                Vec4::new(0.92, 0.16, 0.3, 0.55)
            };
            billboards.push(BillboardInstance::new(
                Vec3::new(p.pos.x, 0.02, p.pos.y),
                [0.95, 0.95],
                ring,
                0.0,
            ));
            // shadow
            billboards.push(BillboardInstance::new(
                Vec3::new(p.pos.x, 0.03, p.pos.y),
                [0.6, 0.6],
                Vec4::new(0.0, 0.0, 0.0, 0.4),
                0.0,
            ));
        }

        // projectiles
        for pr in &self.projectiles {
            // model "front" is -Z, same convention as characters:
            // R_y(yaw)*(0,0,-1) = (-sin, -cos) must equal vel dir.
            let yaw = (-pr.vel.x).atan2(-pr.vel.y);
            let key = match pr.kind {
                ProjKind::Arrow => "icon_arrow",
                ProjKind::Magic => "crystal",
                ProjKind::Fire => "lava",
                ProjKind::Slime => "slime_skin",
                ProjKind::Firework => "art_fireworks",
                ProjKind::Seeds => "art_seeds",
                ProjKind::Ice => "blue_ice",
            };
            let rect = gfx::cell_norm(assets, key);
            let scale = match pr.kind {
                ProjKind::Arrow => Vec3::new(0.1, 0.1, 0.55),
                ProjKind::Slime => Vec3::splat(0.35),
                _ => Vec3::splat(0.3),
            };
            boxes_atlas.push(BoxInstance::new(
                Vec3::new(pr.pos.x, 1.0, pr.pos.y),
                glam::Quat::from_rotation_y(yaw),
                scale,
                [rect; 6],
                Vec4::new(1.3, 1.3, 1.3, 1.0),
                match pr.kind {
                    ProjKind::Arrow => 0.0,
                    _ => 0.5,
                },
            ));
        }

        // pickups
        for pk in &self.pickups {
            let (key, e) = match &pk.kind {
                PickupKind::Emerald(_) => ("icon_emerald", 0.3),
                PickupKind::Arrows(_) => ("icon_arrow", 0.1),
                PickupKind::Heart => ("icon_heart", 0.2),
                PickupKind::Item(_) => ("chest_top", 0.1),
            };
            let rect = gfx::cell_norm(assets, key);
            let bob = (self.time * 3.0 + pk.pos.x).sin() * 0.1 + 0.35;
            boxes_atlas.push(BoxInstance::new(
                Vec3::new(pk.pos.x, bob, pk.pos.y),
                glam::Quat::from_rotation_y(self.time * 1.5),
                Vec3::splat(0.32),
                [rect; 6],
                Vec4::ONE,
                e,
            ));
            billboards.push(BillboardInstance::new(
                Vec3::new(pk.pos.x, 0.03, pk.pos.y),
                [0.3, 0.3],
                Vec4::new(0.0, 0.0, 0.0, 0.3),
                0.0,
            ));
        }

        // particles -> billboards
        self.particles.render(billboards);

        // interact markers (fountain, portal, lever glow, rune, chest hint)
        // handled through particles for simplicity in v1
        let _ = assets;
    }

    /// Rebuild the static bake with real assets (called once after level load).
    pub fn rebake_static(&mut self, assets: &Assets) {
        let mut out = Vec::with_capacity(self.static_bake.len().max(20000));
        self.level.bake_from(assets, &mut out);
        self.static_bake = out;
    }
}
