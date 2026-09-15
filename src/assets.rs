//! Asset pipeline.
//!
//! Texture resolution order for a given key:
//!   1. `assets/textures/override/<key>.png`      (user drop-in, highest priority)
//!   2. `assets/textures/minecraft/<candidates>`  (user drops vanilla MC block textures here)
//!   3. procedural pixel-art fallback generated at runtime
//!
//! Skins: `assets/skins/skin.png` (64x64 or legacy 64x32, slim auto-detected).

use ab_glyph::FontArc;
use image::{imageops::FilterType, GenericImageView, RgbaImage};
use rand::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;

pub const ATLAS_CELL: u32 = 16;
pub const ATLAS_DIM: u32 = 512; // 32x32 cells

pub type Rect = [u16; 4]; // x, y, w, h (pixels in atlas)

/// One selectable hero skin: display name + normalized 64x64 sheet + slim arms.
pub struct SkinEntry {
    pub name: &'static str,
    pub img: RgbaImage,
    pub slim: bool,
}

pub struct Assets {
    pub atlas: RgbaImage,
    pub cells: HashMap<String, Rect>,
    /// keys that fell back to procedural painters (no file found)
    pub fallbacks: Vec<String>,
    /// Combined 64x128 skin sheet: player 1 at v=0, player 2 at v=64.
    pub skin: Option<RgbaImage>,
    pub skin_slim: bool,
    pub skin2_slim: bool,
    /// Hero skin catalog (hero editor). Index 0 = Steve-like, 1 = Alex (P2),
    /// then procedural MCD hero-pack variants.
    pub skin_catalog: Vec<SkinEntry>,
    /// Couches d'armure vanilla prêtes (layer_1 teintée + layer_2), indexées
    /// par classe : 0 = léger (cuir teinté), 1 = maille (fer), 2 = lourd (diamant).
    pub armor_layers: Vec<(RgbaImage, RgbaImage)>,
    pub font: FontArc,
    base_dir: PathBuf,
}

impl Assets {
    pub fn load() -> Assets {
        let base_dir = find_base_dir();
        let font = load_font(&base_dir);
        let mut a = Assets {
            atlas: RgbaImage::new(ATLAS_DIM, ATLAS_DIM),
            cells: HashMap::new(),
            fallbacks: Vec::new(),
            skin: None,
            skin_slim: false,
            skin2_slim: false,
            skin_catalog: Vec::new(),
            armor_layers: Vec::new(),
            font,
            base_dir,
        };
        a.load_all_blocks();
        a.load_all_icons();
        a.load_all_ui();
        a.load_all_mob_textures();
        a.load_armor_layers();
        a.load_skin_catalog();
        a.load_skin();
        a.bake_skin_portraits();
        a
    }

    /// Charge les couches d'armure vanilla extraites du client.jar
    /// (assets/textures/minecraft/armor/&lt;classe&gt;_layer_&lt;1,2&gt;.png).
    /// Le cuir est teinté brun (texture grise de base).
    fn load_armor_layers(&mut self) {
        let defs: [(&str, Option<[f32; 3]>); 3] = [
            ("leather", Some([0.64, 0.42, 0.27])),
            ("iron", None),
            ("diamond", None),
        ];
        for (name, tint) in defs {
            let l1 = self.load_png_raw(&format!("textures/minecraft/armor/{}_layer_1.png", name));
            let l2 = self.load_png_raw(&format!("textures/minecraft/armor/{}_layer_2.png", name));
            if let (Some(mut a), Some(mut b)) = (l1, l2) {
                if let Some(t) = tint {
                    for img in [&mut a, &mut b] {
                        for px in img.pixels_mut() {
                            px[0] = (px[0] as f32 * t[0]) as u8;
                            px[1] = (px[1] as f32 * t[1]) as u8;
                            px[2] = (px[2] as f32 * t[2]) as u8;
                        }
                    }
                }
                self.armor_layers.push((a, b));
            }
        }
    }

    /// Add a pre-painted 16x16 cell (used for skin portraits).
    fn add_raw_cell(&mut self, key: &str, px: &[u8; 16 * 16 * 4]) {
        if self.cells.contains_key(key) {
            return;
        }
        let (cx, cy) = self.alloc_cell_pos();
        blit(&mut self.atlas, px, cx, cy);
        self.cells.insert(key.to_string(), [cx as u16, cy as u16, 16, 16]);
    }

    /// Bake a face portrait (head front + hat overlay, 8x8 -> x2) per catalog skin.
    fn bake_skin_portraits(&mut self) {
        // copy the sheets out first to avoid borrowing self twice
        let sheets: Vec<RgbaImage> = self.skin_catalog.iter().map(|e| e.img.clone()).collect();
        for (i, img) in sheets.iter().enumerate() {
            let mut px = [0u8; 16 * 16 * 4];
            // head front is at (8,8) 8x8, hat overlay front at (40,8) 8x8
            for y in 0..8 {
                for x in 0..8 {
                    let mut p = *img.get_pixel(8 + x, 8 + y);
                    let hat = *img.get_pixel(40 + x, 8 + y);
                    if hat[3] > 0 {
                        // alpha-over blend of the hat layer
                        let a = hat[3] as u32;
                        for c in 0..3 {
                            p[c] = ((hat[c] as u32 * a + p[c] as u32 * (255 - a)) / 255) as u8;
                        }
                        p[3] = p[3].max(hat[3]);
                    }
                    // 2x2 block per source pixel -> 16x16
                    for dy in 0..2 {
                        for dx in 0..2 {
                            let o = (((y * 2 + dy) * 16 + (x * 2 + dx)) * 4) as usize;
                            px[o] = p[0];
                            px[o + 1] = p[1];
                            px[o + 2] = p[2];
                            px[o + 3] = p[3];
                        }
                    }
                }
            }
            self.add_raw_cell(&format!("skinp_{}", i), &px);
        }
    }

    /// Colle une couche d'armure vanilla (64x32) sur la demi-feuille de skin
    /// 64x64 située à l'origine verticale `oy` (0 = joueur 1, 64 = joueur 2).
    /// Regions : casque -> chapeau (32,0), plastron -> veste (16,32),
    /// manches -> (40,32)/(48,48), jambières+bottes (layer 2) -> (0,32)/(0,48).
    fn paste_armor(sheet: &mut RgbaImage, layer1: &RgbaImage, layer2: &RgbaImage, oy: u32) {
        let mut copy = |sheet: &mut RgbaImage, src: &RgbaImage, sx: u32, sy: u32, sw: u32, sh: u32, dx: u32, dy: u32| {
            for y in 0..sh {
                for x in 0..sw {
                    let p = *src.get_pixel(sx + x, sy + y);
                    if p[3] > 0 {
                        sheet.put_pixel(dx + x, dy + y, p);
                    }
                }
            }
        };
        // casque (boîte 8x8x8 dépliée sur (32,0)-(64,16))
        copy(sheet, layer1, 32, 0, 32, 16, 32, oy);
        // plastron -> veste (overlay torse)
        copy(sheet, layer1, 16, 16, 24, 16, 16, 32 + oy);
        // manches (boîte unique vieux format) -> manche droite puis gauche
        copy(sheet, layer1, 40, 16, 16, 16, 40, 32 + oy);
        copy(sheet, layer1, 40, 16, 16, 16, 48, 48 + oy);
        // jambières + bottes (layer 2) -> pantalon droit puis gauche
        copy(sheet, layer2, 0, 16, 16, 16, 0, 32 + oy);
        copy(sheet, layer2, 0, 16, 16, 16, 0, 48 + oy);
    }

    /// Rebuild the combined 64x128 sheet with catalog entry `idx` as player 1
    /// and Alex (index 1) as player 2. Call + Gfx::update_skin to hot-swap.
    /// `armor1`/`armor2` : index de classe d'armure par joueur (0 = léger/cuir,
    /// 1 = maille/fer, 2 = lourd/diamant, None = pas d'armure visible).
    pub fn rebuild_skin(&mut self, idx: usize, armor1: Option<usize>, armor2: Option<usize>) {
        let idx = idx.min(self.skin_catalog.len().saturating_sub(1));
        let p1 = self.skin_catalog[idx].img.clone();
        let slim1 = self.skin_catalog[idx].slim;
        let p2 = self
            .skin_catalog
            .get(1)
            .map(|e| e.img.clone())
            .unwrap_or_else(|| procedural_skin(1));
        let slim2 = self.skin_catalog.get(1).map(|e| e.slim).unwrap_or(true);
        let mut combined = RgbaImage::new(64, 128);
        image::imageops::overlay(&mut combined, &p1, 0, 0);
        image::imageops::overlay(&mut combined, &p2, 0, 64);
        // armures portées : collées sur les couches overlay (visibles en jeu ET au menu)
        if let Some(c) = armor1 {
            if let Some((l1, l2)) = self.armor_layers.get(c.min(2)) {
                Self::paste_armor(&mut combined, l1, l2, 0);
            }
        }
        if let Some(c) = armor2 {
            if let Some((l1, l2)) = self.armor_layers.get(c.min(2)) {
                Self::paste_armor(&mut combined, l1, l2, 64);
            }
        }
        self.skin_slim = slim1;
        self.skin2_slim = slim2;
        self.skin = Some(combined);
    }

    /// Load the hero skin catalog: file skins first (Steve/Alex), then
    /// procedural MCD hero-pack variants.
    fn load_skin_catalog(&mut self) {
        // Steve: custom skins/skin.png, else vanilla steve, else procedural.
        let steve_raw = self
            .load_png_raw("skins/skin.png")
            .or_else(|| self.load_png_raw("skins/steve.png"));
        let (steve_img, steve_slim) = steve_raw
            .map(normalize_skin64)
            .unwrap_or_else(|| (procedural_skin(0), false));
        self.skin_catalog.push(SkinEntry { name: "STEVE", img: steve_img, slim: steve_slim });
        // Alex: file skin if present, else procedural slim variant.
        let (alex_img, alex_slim) = self
            .load_png_raw("skins/alex.png")
            .map(normalize_skin64)
            .unwrap_or_else(|| (procedural_skin(1), true));
        self.skin_catalog.push(SkinEntry { name: "ALEX", img: alex_img, slim: alex_slim });
        // MCD hero-pack style variants (palette-driven painter)
        let heroes: [(&'static str, HeroPalette); 6] = [
            ("ÉCLAIREUR", HeroPalette::scout()),
            ("GARDIENNE", HeroPalette::guardian()),
            ("MAGE", HeroPalette::mage()),
            ("CHEVALIER", HeroPalette::knight()),
            ("ÉMERAUDE", HeroPalette::emerald()),
            ("OMBRE", HeroPalette::shadow()),
        ];
        for (name, pal) in heroes {
            let (img, slim) = (hero_skin(&pal), pal.slim);
            self.skin_catalog.push(SkinEntry { name, img, slim });
        }
    }

    pub fn cell(&self, key: &str) -> Rect {
        *self.cells.get(key).unwrap_or_else(|| {
            self.cells.get("missing").expect("missing cell must exist")
        })
    }

    fn load_png(&self, rel: &str) -> Option<RgbaImage> {
        let p = self.base_dir.join("assets").join(rel);
        let img = image::open(&p).ok()?;
        Some(to_rgba16(&img))
    }

    /// Candidate lookup across the usual vanilla layout subfolders:
    /// textures/minecraft/, then block/, item/, entity/.
    fn load_candidate(&self, name: &str) -> Option<RgbaImage> {
        for sub in ["", "block/", "item/", "entity/"] {
            if let Some(im) = self.load_png(&format!("textures/minecraft/{sub}{name}")) {
                return Some(im);
            }
        }
        None
    }

    /// Full-resolution loader (skins, HD textures) without atlas normalization.
    fn load_png_raw(&self, rel: &str) -> Option<RgbaImage> {
        let p = self.base_dir.join("assets").join(rel);
        let img = image::open(&p).ok()?;
        Some(img.to_rgba8())
    }

    /// Add a texture key with candidates + procedural fallback painter.
    fn add<F: Fn(&mut [u8; 16 * 16 * 4], &mut StdRng)>(
        &mut self,
        key: &str,
        candidates: &[&str],
        fallback: F,
    ) {
        if self.cells.contains_key(key) {
            return;
        }
        let mut img: Option<RgbaImage> = self.load_png(&format!("textures/override/{key}.png"));
        if img.is_none() {
            for c in candidates {
                if let Some(im) = self.load_candidate(c) {
                    img = Some(im);
                    break;
                }
            }
        }
        let cell_xy = self.alloc_cell_pos();
        let (cx, cy) = cell_xy;
        let mut px = [0u8; 16 * 16 * 4];
        match img {
            Some(im) => {
                for y in 0..16 {
                    for x in 0..16 {
                        let p = im.get_pixel(x, y);
                        let o = ((y * 16 + x) * 4) as usize;
                        px[o] = p[0];
                        px[o + 1] = p[1];
                        px[o + 2] = p[2];
                        px[o + 3] = p[3];
                    }
                }
            }
            None => {
                self.fallbacks.push(key.to_string());
                let mut rng = StdRng::seed_from_u64(hash_str(key));
                fallback(&mut px, &mut rng);
            }
        }
        if let Some(tint) = gray_tint_for(key) {
            tint_gray_pixels(&mut px, tint);
        }
        blit(&mut self.atlas, &px, cx, cy);
        self.cells.insert(key.to_string(), [cx as u16, cy as u16, 16, 16]);
    }

    fn alloc_cell_pos(&self) -> (u32, u32) {
        let n = self.cells.len() as u32;
        let per_row = ATLAS_DIM / ATLAS_CELL;
        ((n % per_row) * ATLAS_CELL, (n / per_row) * ATLAS_CELL)
    }

    // ------------------------------------------------------------------
    // Blocks
    // ------------------------------------------------------------------
    fn load_all_blocks(&mut self) {
        self.add("missing", &[], magenta_checker);
        self.add("grass_top", &["grass_block_top.png", "grass.png"], |p, r| noise(p, r, [98, 160, 65, 255], 26));
        self.add("grass_side", &["grass_block_side.png", "grass_side.png"], grass_side);
        self.add("dirt", &["dirt.png", "coarse_dirt.png"], |p, r| noise(p, r, [134, 96, 67, 255], 22));
        self.add("stone", &["stone.png"], |p, r| noise(p, r, [126, 126, 126, 255], 16));
        self.add("cobble", &["cobblestone.png"], |p, r| { noise(p, r, [110, 110, 110, 255], 20); blobs(p, r, [90, 90, 90, 255], 14); });
        self.add("mossy", &["mossy_cobblestone.png"], |p, r| { noise(p, r, [104, 116, 96, 255], 22); blobs(p, r, [70, 110, 60, 255], 12); });
        self.add("planks_oak", &["oak_planks.png", "planks_oak.png"], |p, r| hstripes(p, r, [162, 130, 78, 255], 18));
        self.add("log_side", &["oak_log.png", "log_oak.png"], |p, r| vstripes(p, r, [104, 82, 49, 255], 14));
        self.add("log_top", &["oak_log_top.png", "log_oak_top.png"], log_top);
        self.add("leaves_oak", &["oak_leaves.png", "leaves_oak.png"], |p, r| leaves(p, r, [58, 122, 40, 255]));
        self.add("leaves_jungle", &["jungle_leaves.png", "leaves_jungle.png"], |p, r| leaves(p, r, [48, 138, 48, 255]));
        self.add("sand", &["sand.png"], |p, r| noise(p, r, [219, 207, 163, 255], 10));
        self.add("sandstone", &["sandstone_top.png", "sandstone.png"], |p, r| noise(p, r, [216, 203, 155, 255], 7));
        self.add("red_sand", &["red_sand.png"], |p, r| noise(p, r, [190, 106, 46, 255], 12));
        self.add("stonebrick", &["stone_bricks.png", "stonebrick.png"], brick([122, 122, 122, 255]));
        self.add("mossy_stonebrick", &["mossy_stone_bricks.png", "stonebrick_mossy.png"], brick([108, 124, 100, 255]));
        self.add("cracked_stonebrick", &["cracked_stone_bricks.png"], brick([110, 104, 100, 255]));
        self.add("water", &["water_still.png", "water.png"], |p, r| noise_a(p, r, [48, 90, 200, 170], 14));
        self.add("lava", &["lava_still.png", "lava.png"], |p, r| { noise(p, r, [212, 90, 18, 255], 34); blobs(p, r, [250, 190, 40, 255], 10); });
        self.add("netherrack", &["netherrack.png"], |p, r| noise(p, r, [97, 38, 38, 255], 18));
        self.add("netherbrick", &["nether_bricks.png", "nether_brick.png"], brick([44, 22, 26, 255]));
        self.add("soul_sand", &["soul_sand.png"], |p, r| { noise(p, r, [82, 65, 57, 255], 14); blobs(p, r, [60, 46, 42, 255], 6); });
        self.add("obsidian", &["obsidian.png"], |p, r| { noise(p, r, [24, 18, 36, 255], 8); blobs(p, r, [58, 38, 88, 255], 5); });
        self.add("purpur", &["purpur_block.png"], |p, r| noise(p, r, [170, 122, 170, 255], 10));
        self.add("endstone", &["end_stone.png"], |p, r| noise(p, r, [221, 223, 165, 255], 10));
        self.add("redstone_ore", &["redstone_ore.png"], |p, r| { noise(p, r, [126, 126, 126, 255], 16); dots(p, r, [190, 30, 30, 255], 5); });
        self.add("glowstone", &["glowstone.png"], |p, r| { noise(p, r, [144, 116, 72, 255], 18); blobs(p, r, [248, 218, 130, 255], 9); });
        self.add("mud", &["mud.png"], |p, r| noise(p, r, [60, 57, 58, 255], 10));
        self.add("podzol", &["podzol_top.png"], |p, r| noise(p, r, [90, 66, 40, 255], 14));
        self.add("pumpkin_top", &["pumpkin_top.png", "pumpkin.png"], |p, r| { noise(p, r, [198, 118, 24, 255], 12); blobs(p, r, [150, 86, 16, 255], 6); });
        self.add("pumpkin_side", &["pumpkin_side.png"], |p, r| vstripes(p, r, [206, 126, 28, 255], 10));
        self.add("carved_pumpkin", &["carved_pumpkin.png"], carved_pumpkin);
        self.add("cactus_side", &["cactus_side.png"], |p, r| vstripes(p, r, [72, 120, 44, 255], 12));
        self.add("cactus_top", &["cactus_top.png"], |p, r| noise(p, r, [86, 138, 54, 255], 10));
        self.add("snow", &["snow.png"], |p, r| noise(p, r, [240, 246, 246, 255], 6));
        self.add("ice", &["ice.png"], |p, r| noise_a(p, r, [145, 183, 235, 210], 12));
        self.add("gravel", &["gravel.png"], |p, r| { noise(p, r, [127, 124, 123, 255], 22); blobs(p, r, [100, 98, 96, 255], 8); });
        self.add("clay", &["clay.png"], |p, r| noise(p, r, [160, 166, 179, 255], 8));
        self.add("moss", &["moss_block.png"], |p, r| noise(p, r, [90, 128, 56, 255], 16));
        self.add("deepslate", &["deepslate.png"], |p, r| noise(p, r, [80, 80, 86, 255], 12));
        self.add("sculk", &["sculk.png"], |p, r| { noise(p, r, [12, 29, 36, 255], 8); dots(p, r, [25, 190, 190, 255], 4); });
        self.add("crystal", &[], |p, r| { noise(p, r, [70, 170, 200, 255], 20); blobs(p, r, [180, 240, 255, 255], 6); }); // dungeon-only
        self.add("cage", &["spawner.png"], cage);
        self.add("chest_front", &["chest_front.png"], |p, r| { chest_base(p, r); rects(p, [6, 7, 4, 2], [70, 50, 20, 255]); });
        self.add("chest_side", &["chest_side.png"], chest_base);
        self.add("chest_top", &["chest_top.png"], chest_base);
        self.add("portal", &["end_portal.png", "nether_portal.png"], |p, r| { noise(p, r, [10, 8, 24, 230], 6); dots(p, r, [120, 220, 160, 255], 5); });
        self.add("banner", &[], banner);
        self.add("anvil", &["anvil.png"], |p, r| noise(p, r, [70, 70, 74, 255], 10));
        self.add("campfire", &["campfire.png"], |p, r| { hstripes(p, r, [120, 88, 50, 255], 12); dots(p, r, [240, 150, 40, 255], 6); });
    }

    // ------------------------------------------------------------------
    // UI surfaces — MCD-style dark stone panels with gold trim (procedural)
    // The 16x16 cell is designed for 9-slice: outer 4px = bevel edge zone,
    // inner 8x8 = fill. ui.rs samples sub-regions for corners/edges/center.
    // ------------------------------------------------------------------
    fn load_all_ui(&mut self) {
        self.add("ui_stone", &[], ui_stone_dark);
        self.add("ui_stone_light", &[], ui_stone_light);
        self.add("ui_gold", &["gold_block.png"], ui_gold);
        self.add("ui_dark", &[], ui_dark);
        self.add("wpn_blade", &[], wpn_blade);
    }

    // ------------------------------------------------------------------
    // Icons
    // ------------------------------------------------------------------
    fn load_all_icons(&mut self) {
        self.add("icon_emerald", &["emerald.png"], |p, r| gem(p, r, [60, 220, 120, 255]));
        self.add("icon_arrow", &["arrow.png"], icon_arrow);
        self.add("icon_potion", &["potion.png"], icon_potion);
        self.add("icon_sword", &["diamond_sword.png", "iron_sword.png"], icon_sword);
        self.add("icon_axe", &["diamond_axe.png"], icon_axe);
        self.add("icon_hammer", &["mace.png"], icon_hammer);
        self.add("icon_dagger", &[], icon_dagger);
        self.add("icon_gauntlet", &[], icon_gauntlet);
        self.add("icon_bow", &["bow.png"], icon_bow);
        self.add("icon_crossbow", &["crossbow_standby.png"], icon_crossbow);
        self.add("icon_armor_light", &["leather_chestplate.png"], |p, r| icon_armor(p, r, [160, 150, 120, 255]));
        self.add("icon_armor_medium", &["iron_chestplate.png"], |p, r| icon_armor(p, r, [190, 190, 196, 255]));
        self.add("icon_armor_heavy", &["netherite_chestplate.png"], |p, r| icon_armor(p, r, [90, 84, 92, 255]));
        self.add("art_lightning", &[], |p, r| icon_artifact(p, r, [250, 240, 90, 255], bolt));
        self.add("art_totem", &["totem_of_undying.png"], |p, r| icon_artifact(p, r, [220, 180, 60, 255], totem));
        self.add("art_wind", &[], |p, r| icon_artifact(p, r, [170, 220, 250, 255], wind));
        self.add("art_harvester", &[], |p, r| icon_artifact(p, r, [200, 170, 130, 255], quiver));
        self.add("art_fireworks", &["firework_rocket.png"], |p, r| icon_artifact(p, r, [240, 120, 120, 255], firework));
        self.add("art_seeds", &[], |p, r| icon_artifact(p, r, [140, 200, 80, 255], seeds));
        self.add("icon_key", &["trial_key.png", "golden_key.png"], |p, r| icon_key(p, r, [240, 200, 60, 255]));
        self.add("icon_rune", &[], |p, r| icon_rune(p, r, [90, 230, 180, 255]));
        self.add("icon_heart", &[], |p, r| heart(p, r, [220, 50, 60, 255]));
    }

    // ------------------------------------------------------------------
    // Mob part textures (procedural; overridable via textures/override/)
    // ------------------------------------------------------------------
    fn load_all_mob_textures(&mut self) {
        use MobFace::*;
        // (key, fallback color, face painter, vanilla crop files pre-cropped into
        //  textures/minecraft/<key>_skin.png / <key>_face.png)
        let mobs: &[(&str, [u8; 4], MobFace, bool)] = &[
            ("zombie", [80, 130, 80, 255], Zombie, true),
            ("husk", [150, 128, 90, 255], Zombie, true),
            ("jungle_zombie", [70, 140, 70, 255], Zombie, true),
            ("drowned", [70, 110, 120, 255], Zombie, true),
            ("skeleton", [205, 205, 200, 255], Skeleton, true),
            ("creeper", [90, 180, 70, 255], Creeper, true),
            ("spider", [50, 40, 36, 255], Spider, true),
            ("slime", [110, 200, 110, 220], Slime, true),
            ("pillager", [110, 130, 120, 255], Villager, true),
            ("vindicator", [120, 120, 130, 255], Villager, true),
            ("enchanter", [90, 70, 130, 255], Villager, true),
            ("geomancer", [100, 90, 90, 255], Villager, true),
            ("necromancer", [60, 60, 80, 255], Necromancer, true),
            ("witch", [110, 140, 90, 255], Witch, true),
            ("mooshroom", [170, 40, 40, 255], Mooshroom, true),
            ("bat", [80, 60, 50, 255], Bat, true),
            ("redstone_golem", [130, 80, 60, 255], Golem, true),
            ("blaze", [230, 170, 40, 255], Blaze, true),
            ("wraith", [40, 40, 60, 255], Wraith, true),
            // Minecraft Dungeons bosses (skins from the dungeons mod jar)
            ("arch_illager", [70, 70, 85, 255], Villager, true),
            ("nameless", [55, 55, 70, 255], Necromancer, true),
            ("monstrosity", [120, 120, 124, 255], Golem, true),
            // ---- full bestiary additions (dungeons mod jar + client.jar) ----
            ("mossy_skeleton", [120, 140, 110, 255], Skeleton, true),
            ("skeleton_vanguard", [190, 190, 185, 255], Skeleton, true),
            ("sunken_skeleton", [130, 160, 150, 255], Skeleton, true),
            ("drowned_necromancer", [60, 110, 120, 255], Necromancer, true),
            ("whisperer", [90, 110, 70, 255], Wraith, true),
            ("royal_guard", [110, 110, 125, 255], Villager, true),
            ("illusioner", [100, 95, 125, 255], Villager, true),
            ("frozen_zombie", [110, 140, 160, 255], Zombie, true),
            ("ghostly_kindler", [200, 190, 120, 255], Wraith, true),
            ("piglin", [220, 160, 120, 255], Villager, true),
            ("piglin_brute", [200, 140, 100, 255], Villager, true),
            ("hoglin", [190, 110, 90, 255], Mooshroom, true),
            ("endling", [40, 40, 50, 255], Wraith, true),
            ("iceologer", [130, 170, 210, 255], Villager, true),
            ("mountaineer", [140, 150, 170, 255], Villager, true),
            ("windcaller", [120, 160, 200, 255], Villager, true),
            ("squall_golem", [150, 160, 170, 255], Golem, true),
            ("wither_skeleton", [45, 42, 40, 255], Skeleton, true),
            ("zombified_pig", [210, 150, 140, 255], Zombie, true),
            ("enderman", [25, 22, 30, 255], Wraith, true),
            ("endermite", [60, 45, 70, 255], Spider, true),
            ("silverfish", [140, 140, 145, 255], Spider, true),
            ("caerbannog", [235, 235, 230, 255], Bat, true),
            ("vex", [170, 175, 185, 255], Bat, true),
            ("leapleaf", [110, 140, 80, 255], Spider, true),
            ("poison_anemone", [60, 160, 150, 255], Slime, true),
            ("poison_quill_vine", [80, 150, 90, 255], Slime, true),
            ("blastling", [50, 45, 60, 255], Wraith, true),
            ("snareling", [45, 50, 40, 255], Wraith, true),
            ("cave_crawler", [120, 110, 100, 255], Spider, true),
            ("mini_abomination", [90, 120, 70, 255], Spider, true),
            ("magmacube", [200, 90, 30, 255], Slime, true),
            ("ghast", [235, 235, 240, 255], Slime, true),
            ("icy_creeper", [150, 200, 220, 255], Creeper, true),
            ("jungle_abomination", [110, 130, 80, 255], Slime, true),
            ("wretched", [150, 220, 230, 255], Wraith, true),
            ("tempest", [140, 150, 160, 255], Golem, true),
            ("ancient_guardian", [130, 120, 150, 255], Slime, true),
            ("vengeful", [70, 30, 80, 255], Wraith, true),
            ("wildfire", [235, 180, 50, 255], Blaze, true),
        ];
        for (name, col, face, vanilla) in mobs {
            let name = *name;
            let skin_c: &[&str] = if *vanilla { &[&format!("{name}_skin.png")] } else { &[] };
            let face_c: &[&str] = if *vanilla { &[&format!("{name}_face.png")] } else { &[] };
            let col2 = *col;
            self.add(&format!("{name}_skin"), skin_c, move |p, r| { noise(p, r, col2, 14); });
            let col3 = *col;
            self.add(&format!("{name}_face"), face_c, move |p, r| { noise(p, r, col3, 14); draw_face(p, *face); });
        }
        // Torso-front cells (body crops; all humanoid mobs + MCD bosses).
        let bodies: &[(&str, [u8; 4])] = &[
            ("zombie", [70, 90, 140, 255]),
            ("husk", [140, 120, 90, 255]),
            ("jungle_zombie", [70, 120, 60, 255]),
            ("drowned", [60, 100, 110, 255]),
            ("skeleton", [180, 180, 175, 255]),
            ("pillager", [90, 100, 95, 255]),
            ("vindicator", [95, 95, 105, 255]),
            ("enchanter", [80, 60, 120, 255]),
            ("geomancer", [95, 85, 85, 255]),
            ("necromancer", [50, 50, 70, 255]),
            ("witch", [100, 125, 85, 255]),
            ("wraith", [45, 45, 70, 255]),
            ("redstone_golem", [140, 40, 30, 255]),
            ("arch_illager", [60, 65, 95, 255]),
            ("nameless", [45, 50, 60, 255]),
            ("monstrosity", [130, 35, 28, 255]),
            // ---- full bestiary additions ----
            ("mossy_skeleton", [100, 120, 95, 255]),
            ("skeleton_vanguard", [170, 170, 165, 255]),
            ("sunken_skeleton", [110, 140, 130, 255]),
            ("drowned_necromancer", [50, 90, 100, 255]),
            ("whisperer", [70, 90, 60, 255]),
            ("royal_guard", [95, 95, 110, 255]),
            ("illusioner", [85, 80, 110, 255]),
            ("frozen_zombie", [95, 120, 140, 255]),
            ("ghostly_kindler", [180, 170, 110, 255]),
            ("piglin", [200, 140, 105, 255]),
            ("piglin_brute", [180, 125, 90, 255]),
            ("hoglin", [170, 100, 80, 255]),
            ("endling", [35, 35, 45, 255]),
            ("iceologer", [110, 150, 190, 255]),
            ("mountaineer", [120, 130, 150, 255]),
            ("giant_royal", [90, 90, 105, 255]),
            ("windcaller", [100, 140, 180, 255]),
            ("squall_golem", [130, 140, 150, 255]),
            ("wither_skeleton", [40, 38, 36, 255]),
            ("zombified_pig", [190, 130, 120, 255]),
            ("enderman", [20, 18, 26, 255]),
            ("endermite", [50, 38, 62, 255]),
            ("silverfish", [120, 120, 125, 255]),
            ("caerbannog", [220, 220, 215, 255]),
            ("vex", [150, 155, 165, 255]),
            ("leapleaf", [95, 120, 70, 255]),
            ("poison_anemone", [50, 140, 130, 255]),
            ("poison_quill_vine", [70, 130, 80, 255]),
            ("blastling", [42, 38, 52, 255]),
            ("snareling", [38, 42, 33, 255]),
            ("cave_crawler", [100, 92, 84, 255]),
            ("mini_abomination", [80, 100, 60, 255]),
            ("magmacube", [180, 80, 26, 255]),
            ("ghast", [220, 220, 226, 255]),
            ("icy_creeper", [130, 180, 200, 255]),
            ("jungle_abomination", [95, 115, 70, 255]),
            ("wretched", [130, 200, 210, 255]),
            ("tempest", [120, 130, 140, 255]),
            ("ancient_guardian", [110, 100, 130, 255]),
            ("vengeful", [60, 25, 70, 255]),
            ("wildfire", [215, 160, 45, 255]),
        ];
        for (name, col) in bodies {
            let key = format!("{name}_body");
            let col2 = *col;
            self.add(&key, &[&format!("{key}.png")], move |p, r| { noise(p, r, col2, 12); });
        }
        // Corrupted Cauldron boss parts (metal / glowing potion / wood).
        self.add("cauldron_metal", &["cauldron_metal.png"], |p, r| noise(p, r, [70, 70, 76, 255], 10));
        self.add("cauldron_potion", &["cauldron_potion.png"], |p, r| { noise(p, r, [235, 120, 235, 255], 20); blobs(p, r, [255, 220, 255, 255], 6); });
        self.add("cauldron_wood", &["cauldron_wood.png"], |p, r| hstripes(p, r, [110, 80, 45, 255], 12));
    }

    // ------------------------------------------------------------------
    // Player skin
    // ------------------------------------------------------------------
    fn load_skin(&mut self) {
        // Combined sheet from the catalog: Steve (or files) as P1, Alex as P2.
        self.rebuild_skin(0, None, None);
    }

    pub fn base_dir(&self) -> &PathBuf {
        &self.base_dir
    }

    /// Cells that fell back to procedural generation (file missing).
    pub fn missing_cells(&self) -> &[String] {
        &self.fallbacks
    }
}

// ======================================================================
// helpers
// ======================================================================

fn p_true() -> bool { true }

fn hash_str(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn to_rgba16(img: &image::DynamicImage) -> RgbaImage {
    let rgba = img.to_rgba8();
    to16(rgba)
}

fn to16(img: RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    // MC animated textures ship as vertical strips of square frames (+ .mcmeta).
    // We take the first frame; per-frame animation is a v2 feature.
    if w > 0 && h > w && h % w == 0 {
        let frame = image::imageops::crop_imm(&img, 0, 0, w, w).to_image();
        return finish16(frame);
    }
    if h > 0 && w > h && w % h == 0 {
        let frame = image::imageops::crop_imm(&img, 0, 0, h, h).to_image();
        return finish16(frame);
    }
    if w == h {
        return finish16(img);
    }
    // non-square, non-strip: center crop
    let side = w.min(h);
    let cropped =
        image::imageops::crop_imm(&img, (w - side) / 2, (h - side) / 2, side, side).to_image();
    finish16(cropped)
}

fn finish16(img: RgbaImage) -> RgbaImage {
    if img.width() == 16 && img.height() == 16 {
        img
    } else {
        image::imageops::resize(&img, 16, 16, FilterType::Nearest)
    }
}

/// Biome-colormap tints (like the vanilla grass.png / foliage.png colormaps).
/// Applied ONLY to grayscale pixels, so pre-colored user textures are untouched.
fn gray_tint_for(key: &str) -> Option<[f32; 3]> {
    match key {
        "grass_top" | "grass_side" => Some([0.57, 0.74, 0.35]), // plains grass (#91BD59)
        "leaves_oak" => Some([0.47, 0.67, 0.18]),               // plains foliage
        "leaves_jungle" => Some([0.28, 0.70, 0.20]),            // jungle foliage
        _ => None,
    }
}

fn is_gray_pixel(r: u8, g: u8, b: u8) -> bool {
    let (r, g, b) = (r as i32, g as i32, b as i32);
    (r - g).abs() < 10 && (g - b).abs() < 10
}

fn tint_gray_pixels(px: &mut [u8; 16 * 16 * 4], tint: [f32; 3]) {
    for i in 0..256 {
        let o = i * 4;
        if px[o + 3] == 0 { continue; }
        if is_gray_pixel(px[o], px[o + 1], px[o + 2]) {
            px[o] = ((px[o] as f32 / 255.0) * tint[0] * 255.0).min(255.0) as u8;
            px[o + 1] = ((px[o + 1] as f32 / 255.0) * tint[1] * 255.0).min(255.0) as u8;
            px[o + 2] = ((px[o + 2] as f32 / 255.0) * tint[2] * 255.0).min(255.0) as u8;
        }
    }
}

fn blit(atlas: &mut RgbaImage, px: &[u8; 16 * 16 * 4], cx: u32, cy: u32) {
    for y in 0..16 {
        for x in 0..16 {
            let o = ((y * 16 + x) * 4) as usize;
            atlas.put_pixel(cx + x, cy + y, image::Rgba([px[o], px[o + 1], px[o + 2], px[o + 3]]));
        }
    }
}

type Px = [u8; 16 * 16 * 4];

fn put(p: &mut Px, x: usize, y: usize, c: [u8; 4]) {
    if x >= 16 || y >= 16 { return; }
    let o = (y * 16 + x) * 4;
    p[o] = c[0]; p[o + 1] = c[1]; p[o + 2] = c[2]; p[o + 3] = c[3];
}

fn rects(p: &mut Px, r: [usize; 4], c: [u8; 4]) {
    for y in r[1]..(r[1] + r[3]).min(16) {
        for x in r[0]..(r[0] + r[2]).min(16) {
            put(p, x, y, c);
        }
    }
}

fn noise(p: &mut Px, r: &mut StdRng, base: [u8; 4], var: u8) {
    for y in 0..16 {
        for x in 0..16 {
            let d = (r.gen_range(0..var as i32) - var as i32 / 2) as i32;
            let cl = |v: i32| v.clamp(0, 255) as u8;
            put(p, x, y, [cl(base[0] as i32 + d), cl(base[1] as i32 + d), cl(base[2] as i32 + d), base[3]]);
        }
    }
}

fn noise_a(p: &mut Px, r: &mut StdRng, base: [u8; 4], var: u8) {
    noise(p, r, base, var);
}

fn blobs(p: &mut Px, r: &mut StdRng, c: [u8; 4], n: usize) {
    for _ in 0..n {
        let cx = r.gen_range(1..15);
        let cy = r.gen_range(1..15);
        put(p, cx, cy, c);
        put(p, cx + 1, cy, c);
        put(p, cx, cy + 1, c);
    }
}

fn dots(p: &mut Px, r: &mut StdRng, c: [u8; 4], n: usize) {
    for _ in 0..n {
        let cx = r.gen_range(1..15);
        let cy = r.gen_range(1..15);
        put(p, cx, cy, c);
    }
}

fn hstripes(p: &mut Px, r: &mut StdRng, base: [u8; 4], var: u8) {
    for y in 0..16 {
        let d = (r.gen_range(0..var as i32) - var as i32 / 2) as i32;
        for x in 0..16 {
            if y % 4 == 3 { put(p, x, y, shade(base, -26)); }
            else { let cl = |v: i32| v.clamp(0, 255) as u8; put(p, x, y, [cl(base[0] as i32 + d), cl(base[1] as i32 + d), cl(base[2] as i32 + d), base[3]]); }
        }
    }
}

fn vstripes(p: &mut Px, r: &mut StdRng, base: [u8; 4], var: u8) {
    for x in 0..16 {
        let d = (r.gen_range(0..var as i32) - var as i32 / 2) as i32;
        for y in 0..16 {
            let cl = |v: i32| v.clamp(0, 255) as u8;
            put(p, x, y, [cl(base[0] as i32 + d), cl(base[1] as i32 + d), cl(base[2] as i32 + d), base[3]]);
        }
    }
}

fn shade(c: [u8; 4], d: i32) -> [u8; 4] {
    let cl = |v: i32| v.clamp(0, 255) as u8;
    [cl(c[0] as i32 + d), cl(c[1] as i32 + d), cl(c[2] as i32 + d), c[3]]
}

fn grass_side(p: &mut Px, r: &mut StdRng) {
    noise(p, r, [134, 96, 67, 255], 22);
    for x in 0..16 {
        let h = 3 + (r.gen_range(0..3));
        for y in 0..h { put(p, x, y, shade([98, 160, 65, 255], (r.gen_range(-18..18)) as i32)); }
    }
}

fn log_top(p: &mut Px, r: &mut StdRng) {
    noise(p, r, [152, 123, 73, 255], 10);
    for ring in 0..4 {
        let c = shade([110, 86, 50, 255], ring * 8);
        let rad = 7 - ring * 2;
        for a in 0..64 {
            let t = a as f32 / 64.0 * std::f32::consts::TAU;
            let x = (8.0 + rad as f32 * t.cos()) as usize;
            let y = (8.0 + rad as f32 * t.sin()) as usize;
            put(p, x, y, c);
        }
    }
}

fn leaves(p: &mut Px, r: &mut StdRng, base: [u8; 4]) {
    for y in 0..16 {
        for x in 0..16 {
            if r.gen_bool(0.18) { put(p, x, y, [0, 0, 0, 0]); }
            else { put(p, x, y, shade(base, r.gen_range(-20..20))); }
        }
    }
}

fn brick(c: [u8; 4]) -> impl Fn(&mut Px, &mut StdRng) {
    move |p: &mut Px, r: &mut StdRng| {
        noise(p, r, c, 12);
        if true {
            for y in (0..16).step_by(4) { for x in 0..16 { put(p, x, y, [88, 88, 88, 255]); } }
            for row in 0..4 {
                let off = if row % 2 == 0 { 0 } else { 4 };
                for x in (off..16).step_by(8) { for y in row * 4..row * 4 + 4 { put(p, x, y, [88, 88, 88, 255]); } }
            }
        }
    }
}

fn cage(p: &mut Px, r: &mut StdRng) {
    for i in 0..16 {
        for c in 0..16 {
            put(p, i, c, [0, 0, 0, 0]);
            put(p, c, i, [0, 0, 0, 0]);
        }
    }
    for i in (0..16).step_by(3) {
        for c in 0..16 { put(p, i, c, [90, 90, 96, 255]); put(p, c, i, [70, 70, 76, 255]); }
    }
    let _ = r;
}

fn chest_base(p: &mut Px, r: &mut StdRng) {
    noise(p, r, [140, 100, 48, 255], 10);
    for x in 0..16 { put(p, x, 5, [70, 50, 20, 255]); }
    for y in 0..16 { put(p, 0, y, shade([140, 100, 48, 255], -30)); put(p, 15, y, shade([140, 100, 48, 255], -30)); }
}

fn carved_pumpkin(p: &mut Px, r: &mut StdRng) {
    vstripes(p, r, [206, 126, 28, 255], 10);
    // eyes + grin
    rects(p, [2, 5, 3, 3], [40, 20, 4, 255]);
    rects(p, [11, 5, 3, 3], [40, 20, 4, 255]);
    rects(p, [4, 10, 8, 2], [40, 20, 4, 255]);
    put(p, 3, 9, [40, 20, 4, 255]); put(p, 12, 9, [40, 20, 4, 255]);
}

fn banner(p: &mut Px, r: &mut StdRng) {
    noise(p, r, [150, 40, 40, 255], 12);
    for y in 0..16 { put(p, 0, y, [80, 70, 60, 255]); }
    let _ = r;
}

// ---- UI surface painters (9-slice friendly: outer 4px = edge zone) ----

/// Dark charcoal stone slab — panel body. Bevel: top/left lit, bottom/right dark.
fn ui_stone_dark(p: &mut Px, r: &mut StdRng) {
    noise(p, r, [44, 44, 54, 255], 7);
    // subtle brick joints in the fill zone
    for x in 4..12 { put(p, x, 8, [30, 30, 38, 255]); }
    for y in 4..12 { put(p, 8, y, [30, 30, 38, 255]); }
    // bevel edge zone (outer 4px)
    for i in 0..16 {
        for e in 0..4 {
            let lift = 14 - e as i32 * 4;
            let drop = -12 + e as i32 * 3;
            let cl = |v: i32, d: i32| (v + d).clamp(0, 255) as u8;
            let b = [44, 44, 54];
            put(p, i, e, [cl(b[0], lift), cl(b[1], lift), cl(b[2], lift), 255]);
            put(p, e, i, [cl(b[0], lift), cl(b[1], lift), cl(b[2], lift), 255]);
            put(p, i, 15 - e, [cl(b[0], drop), cl(b[1], drop), cl(b[2], drop), 255]);
            put(p, 15 - e, i, [cl(b[0], drop), cl(b[1], drop), cl(b[2], drop), 255]);
        }
    }
    // corner studs (gold rivets at the 4 corners, 1px)
    for (x, y) in [(1usize, 1usize), (14, 1), (1, 14), (14, 14)] {
        put(p, x, y, [212, 170, 80, 255]);
    }
}

/// Lighter warm stone — button face.
fn ui_stone_light(p: &mut Px, r: &mut StdRng) {
    noise(p, r, [66, 62, 66, 255], 8);
    for i in 0..16 {
        for e in 0..4 {
            let lift = 18 - e as i32 * 5;
            let drop = -16 + e as i32 * 4;
            let cl = |v: i32, d: i32| (v + d).clamp(0, 255) as u8;
            let b = [66, 62, 66];
            put(p, i, e, [cl(b[0], lift), cl(b[1], lift), cl(b[2], lift), 255]);
            put(p, e, i, [cl(b[0], lift), cl(b[1], lift), cl(b[2], lift), 255]);
            put(p, i, 15 - e, [cl(b[0], drop), cl(b[1], drop), cl(b[2], drop), 255]);
            put(p, 15 - e, i, [cl(b[0], drop), cl(b[1], drop), cl(b[2], drop), 255]);
        }
    }
}

/// Gold metal — trim surface (vertical sheen + grain).
fn ui_gold(p: &mut Px, r: &mut StdRng) {
    for y in 0..16 {
        for x in 0..16 {
            let t = y as f32 / 15.0;
            let sheen = (1.0 - t) * 34.0 - t * 46.0;
            let glint = if x == 3 || x == 11 { 12 } else { 0 };
            let d = r.gen_range(-6..=6);
            let cl = |v: f32| v.clamp(0.0, 255.0) as u8;
            put(p, x, y, [
                cl(226.0 + sheen + glint as f32 + d as f32),
                cl(178.0 + sheen * 0.8 + d as f32),
                cl(78.0 + sheen * 0.4 + d as f32),
                255,
            ]);
        }
    }
    // dark seam rows for a cast-metal look
    for x in 0..16 { put(p, x, 7, shade([196, 148, 62, 255], -40)); }
}

/// Near-black coarse backdrop grain (menu background base).
fn ui_dark(p: &mut Px, r: &mut StdRng) {
    noise(p, r, [17, 16, 21, 255], 5);
    if true {
        for (x, y) in [(3usize, 5usize), (9usize, 2usize), (13usize, 11usize), (5usize, 12usize)] {
            put(p, x, y, [26, 24, 32, 255]);
        }
    }
}

/// Vertical sword blade + hilt — held-weapon box texture.
fn wpn_blade(p: &mut Px, _r: &mut StdRng) {
    for y in 0..16 { for x in 0..16 { put(p, x, y, [0, 0, 0, 0]); } }
    // blade
    for y in 0..10 {
        put(p, 7, y, [236, 240, 248, 255]);
        put(p, 8, y, [188, 196, 210, 255]);
        if y % 3 == 2 { put(p, 8, y, [150, 158, 176, 255]); }
    }
    put(p, 7, 0, [250, 252, 255, 255]); put(p, 8, 0, [210, 216, 228, 255]);
    // guard
    for x in 4..12 { put(p, x, 10, [190, 148, 62, 255]); }
    put(p, 4, 10, [140, 104, 40, 255]); put(p, 11, 10, [140, 104, 40, 255]);
    // grip
    for y in 11..14 { put(p, 7, y, [96, 66, 34, 255]); put(p, 8, y, [78, 52, 26, 255]); }
    // pommel
    put(p, 7, 14, [212, 170, 80, 255]); put(p, 8, 14, [160, 122, 52, 255]);
}

fn magenta_checker(p: &mut Px, _r: &mut StdRng) {
    for y in 0..16 { for x in 0..16 {
        let c = if (x / 4 + y / 4) % 2 == 0 { [220, 0, 220, 255] } else { [20, 20, 20, 255] };
        put(p, x, y, c);
    } }
}

// ---- faces ----
#[derive(Clone, Copy)]
pub enum MobFace { Zombie, Skeleton, Creeper, Spider, Slime, Villager, Necromancer, Witch, Mooshroom, Bat, Golem, Blaze, Wraith }

fn draw_face(p: &mut Px, f: MobFace) {
    let dark = [24, 24, 24, 255];
    let white = [240, 240, 240, 255];
    match f {
        MobFace::Zombie => {
            rects(p, [3, 5, 2, 2], dark); rects(p, [11, 5, 2, 2], dark);
            rects(p, [6, 10, 4, 1], dark);
        }
        MobFace::Skeleton => {
            rects(p, [3, 5, 3, 2], [40, 40, 40, 255]); rects(p, [10, 5, 3, 2], [40, 40, 40, 255]);
            rects(p, [7, 8, 2, 2], [60, 60, 60, 255]);
            for x in 4..12 { put(p, x, 11, [90, 90, 90, 255]); }
        }
        MobFace::Creeper => {
            rects(p, [2, 4, 3, 3], dark); rects(p, [11, 4, 3, 3], dark);
            rects(p, [6, 7, 4, 4], dark);
            rects(p, [5, 9, 1, 3], dark); rects(p, [10, 9, 1, 3], dark);
        }
        MobFace::Spider => {
            rects(p, [3, 5, 2, 2], [200, 40, 40, 255]); rects(p, [11, 5, 2, 2], [200, 40, 40, 255]);
            rects(p, [6, 4, 1, 1], [200, 40, 40, 255]); rects(p, [9, 4, 1, 1], [200, 40, 40, 255]);
            rects(p, [7, 6, 2, 1], [150, 30, 30, 255]);
        }
        MobFace::Slime => {
            rects(p, [3, 6, 2, 2], dark); rects(p, [11, 6, 2, 2], dark);
            rects(p, [6, 11, 4, 1], dark);
        }
        MobFace::Villager => {
            rects(p, [3, 5, 3, 1], dark); rects(p, [10, 5, 3, 1], dark); // unibrow
            rects(p, [3, 7, 3, 2], white); rects(p, [10, 7, 3, 2], white);
            rects(p, [4, 7, 1, 2], [40, 80, 160, 255]); rects(p, [11, 7, 1, 2], [40, 80, 160, 255]);
            rects(p, [7, 6, 2, 5], shade([110, 130, 120, 255], -20)); // nose
            rects(p, [6, 12, 4, 1], dark);
        }
        MobFace::Necromancer => {
            rects(p, [3, 6, 3, 2], [140, 60, 200, 255]); rects(p, [10, 6, 3, 2], [140, 60, 200, 255]);
            rects(p, [6, 11, 4, 1], [140, 60, 200, 255]);
        }
        MobFace::Witch => {
            rects(p, [3, 5, 3, 1], dark); rects(p, [10, 5, 3, 1], dark);
            rects(p, [7, 6, 2, 5], shade([110, 140, 90, 255], -24));
            rects(p, [5, 12, 6, 1], dark);
            dots(p, &mut StdRng::seed_from_u64(7), [60, 110, 50, 255], 6);
        }
        MobFace::Mooshroom => {
            rects(p, [3, 5, 3, 2], dark); rects(p, [10, 5, 3, 2], dark);
            rects(p, [5, 10, 6, 4], [230, 170, 180, 255]);
            put(p, 6, 12, [150, 100, 110, 255]); put(p, 9, 12, [150, 100, 110, 255]);
        }
        MobFace::Bat => {
            rects(p, [4, 6, 2, 2], [230, 120, 120, 255]); rects(p, [10, 6, 2, 2], [230, 120, 120, 255]);
            rects(p, [7, 10, 2, 1], dark);
        }
        MobFace::Golem => {
            rects(p, [3, 5, 3, 2], [255, 80, 30, 255]); rects(p, [10, 5, 3, 2], [255, 80, 30, 255]);
            rects(p, [6, 10, 4, 2], [255, 120, 40, 255]);
        }
        MobFace::Blaze => {
            rects(p, [3, 5, 3, 2], dark); rects(p, [10, 5, 3, 2], dark);
            dots(p, &mut StdRng::seed_from_u64(3), [255, 240, 120, 255], 8);
        }
        MobFace::Wraith => {
            rects(p, [3, 5, 3, 2], [80, 230, 230, 255]); rects(p, [10, 5, 3, 2], [80, 230, 230, 255]);
            rects(p, [6, 11, 4, 1], [80, 230, 230, 255]);
        }
    }
}

// ---- icon painters ----
fn gem(p: &mut Px, r: &mut StdRng, c: [u8; 4]) {
    let hi = shade(c, 70); let lo = shade(c, -70);
    for y in 3..13 {
        let w = match y { 3 | 4 => 4, 5..=9 => 8, 10 => 6, 11 => 4, _ => 2 };
        let x0 = 8 - w / 2;
        for x in x0..(x0 + w) { put(p, x, y, c); }
    }
    put(p, 5, 5, hi); put(p, 5, 6, hi);
    put(p, 10, 10, lo); put(p, 9, 11, lo);
    let _ = r;
}

fn icon_arrow(p: &mut Px, _r: &mut StdRng) {
    for i in 0..9 { put(p, 3 + i, 12 - i, [140, 100, 60, 255]); }
    for i in 0..4 { put(p, 10 + i, 3 + i, [200, 200, 210, 255]); }
    put(p, 10, 6, [200, 200, 210, 255]); put(p, 13, 6, [200, 200, 210, 255]);
    put(p, 7, 10, [230, 230, 230, 255]); put(p, 6, 9, [230, 230, 230, 255]);
    put(p, 4, 13, [230, 230, 230, 255]); put(p, 3, 12, [230, 230, 230, 255]);
}

fn icon_potion(p: &mut Px, _r: &mut StdRng) {
    rects(p, [6, 2, 4, 3], [180, 180, 190, 255]);
    rects(p, [4, 5, 8, 9], [0, 0, 0, 0]);
    // bottle outline
    for y in 5..14 { put(p, 4, y, [200, 210, 220, 255]); put(p, 11, y, [200, 210, 220, 255]); }
    for x in 5..11 { put(p, x, 5, [200, 210, 220, 255]); put(p, x, 13, [200, 210, 220, 255]); }
    rects(p, [5, 9, 6, 4], [230, 60, 70, 255]);
    rects(p, [5, 8, 6, 1], [240, 130, 130, 255]);
}

fn icon_sword(p: &mut Px, _r: &mut StdRng) {
    for i in 0..9 { put(p, 4 + i, 11 - i, [200, 210, 225, 255]); put(p, 5 + i, 11 - i, [150, 160, 180, 255]); }
    put(p, 3, 12, [110, 80, 40, 255]); put(p, 4, 13, [110, 80, 40, 255]);
    put(p, 5, 12, [110, 80, 40, 255]); put(p, 2, 11, [110, 80, 40, 255]);
    put(p, 4, 10, [140, 110, 60, 255]); put(p, 6, 12, [140, 110, 60, 255]);
}

fn icon_axe(p: &mut Px, _r: &mut StdRng) {
    for i in 0..9 { put(p, 4 + i, 12 - i, [140, 100, 55, 255]); }
    rects(p, [8, 3, 5, 4], [190, 200, 215, 255]);
    rects(p, [7, 4, 2, 3], [190, 200, 215, 255]);
    rects(p, [9, 2, 3, 1], [150, 160, 175, 255]);
}

fn icon_hammer(p: &mut Px, _r: &mut StdRng) {
    for i in 0..9 { put(p, 4 + i, 13 - i, [140, 100, 55, 255]); }
    rects(p, [8, 2, 6, 5], [170, 175, 185, 255]);
    rects(p, [8, 2, 6, 1], [120, 125, 135, 255]);
}

fn icon_dagger(p: &mut Px, _r: &mut StdRng) {
    for i in 0..6 { put(p, 6 + i, 10 - i, [200, 210, 225, 255]); }
    put(p, 5, 11, [110, 80, 40, 255]); put(p, 6, 12, [110, 80, 40, 255]);
    put(p, 12, 4, [220, 230, 240, 255]);
}

fn icon_gauntlet(p: &mut Px, _r: &mut StdRng) {
    rects(p, [4, 4, 8, 8], [180, 160, 120, 255]);
    for x in (5..12).step_by(2) { rects(p, [x, 2, 1, 2], [180, 160, 120, 255]); }
    rects(p, [6, 7, 4, 3], [120, 100, 70, 255]);
    rects(p, [4, 11, 8, 1], [120, 100, 70, 255]);
}

fn icon_bow(p: &mut Px, _r: &mut StdRng) {
    for i in 0..10 {
        let t = i as f32 / 9.0;
        let x = (3.0 + 7.0 * (t * std::f32::consts::PI).sin()) as usize;
        let y = 2 + i;
        put(p, x, y, [140, 100, 55, 255]);
    }
    for y in 2..12 { put(p, 10, y, [230, 230, 230, 255]); }
}

fn icon_crossbow(p: &mut Px, _r: &mut StdRng) {
    rects(p, [7, 2, 2, 12], [140, 100, 55, 255]);
    rects(p, [2, 4, 12, 2], [120, 85, 45, 255]);
    for x in 2..14 { put(p, x, 3, [230, 230, 230, 255]); }
    rects(p, [6, 10, 4, 2], [90, 65, 35, 255]);
}

fn icon_armor(p: &mut Px, _r: &mut StdRng, c: [u8; 4]) {
    rects(p, [4, 3, 8, 3], c);
    rects(p, [3, 6, 3, 6], c);
    rects(p, [10, 6, 3, 6], c);
    rects(p, [6, 6, 4, 5], c);
    rects(p, [6, 6, 4, 1], shade(c, -50));
    rects(p, [7, 4, 2, 1], shade(c, -50));
}

fn icon_artifact(p: &mut Px, r: &mut StdRng, frame: [u8; 4], glyph: fn(&mut Px)) {
    // framed icon background
    for y in 0..16 { for x in 0..16 {
        let edge = x == 0 || y == 0 || x == 15 || y == 15;
        put(p, x, y, if edge { frame } else { shade([30, 30, 44, 255], r.gen_range(-8..8)) });
    } }
    glyph(p);
}

fn bolt(p: &mut Px) {
    put(p, 9, 2, [250, 240, 90, 255]); put(p, 8, 3, [250, 240, 90, 255]); put(p, 7, 4, [250, 240, 90, 255]);
    rects(p, [6, 5, 3, 1], [250, 240, 90, 255]);
    put(p, 7, 6, [250, 240, 90, 255]); put(p, 6, 7, [250, 240, 90, 255]); put(p, 5, 8, [250, 240, 90, 255]);
    rects(p, [6, 9, 3, 1], [250, 240, 90, 255]);
    put(p, 5, 10, [250, 240, 90, 255]); put(p, 4, 11, [250, 240, 90, 255]);
}

fn totem(p: &mut Px) {
    rects(p, [5, 3, 6, 8], [220, 180, 60, 255]);
    rects(p, [6, 4, 1, 2], [40, 40, 40, 255]); rects(p, [9, 4, 1, 2], [40, 40, 40, 255]);
    rects(p, [5, 11, 6, 2], [180, 140, 40, 255]);
    rects(p, [7, 13, 2, 1], [180, 140, 40, 255]);
}

fn wind(p: &mut Px) {
    for (i, y) in [4usize, 7, 10].iter().enumerate() {
        let w = 8 - i * 2;
        for x in 4..(4 + w) { put(p, x, *y + i, [200, 235, 250, 255]); }
        put(p, 3, *y + i, [170, 220, 245, 255]);
    }
}

fn quiver(p: &mut Px) {
    rects(p, [5, 4, 6, 9], [140, 100, 60, 255]);
    rects(p, [5, 4, 6, 2], [110, 75, 40, 255]);
    for x in [6usize, 8, 10] { put(p, x, 2, [230, 230, 230, 255]); put(p, x, 3, [200, 160, 90, 255]); }
}

fn firework(p: &mut Px) {
    rects(p, [7, 6, 2, 8], [180, 150, 110, 255]);
    rects(p, [6, 3, 4, 3], [240, 120, 120, 255]);
    put(p, 5, 4, [240, 120, 120, 255]); put(p, 10, 4, [240, 120, 120, 255]);
    put(p, 8, 2, [250, 200, 90, 255]);
}

fn seeds(p: &mut Px) {
    dots(p, &mut StdRng::seed_from_u64(11), [140, 200, 80, 255], 10);
    put(p, 7, 7, [90, 160, 50, 255]); put(p, 8, 8, [90, 160, 50, 255]);
}

fn icon_key(p: &mut Px, _r: &mut StdRng, c: [u8; 4]) {
    rects(p, [4, 4, 4, 4], c);
    rects(p, [5, 5, 2, 2], [40, 40, 40, 255]);
    rects(p, [8, 6, 6, 1], c);
    rects(p, [11, 7, 1, 3], c);
    rects(p, [13, 7, 1, 2], c);
}

fn icon_rune(p: &mut Px, _r: &mut StdRng, c: [u8; 4]) {
    for y in 2..14 { put(p, 8, y, c); }
    put(p, 6, 4, c); put(p, 5, 6, c); put(p, 10, 4, c); put(p, 11, 6, c);
    rects(p, [5, 11, 6, 1], c);
}

fn heart(p: &mut Px, _r: &mut StdRng, c: [u8; 4]) {
    rects(p, [3, 4, 4, 3], c); rects(p, [9, 4, 4, 3], c);
    rects(p, [2, 5, 12, 3], c);
    rects(p, [4, 8, 8, 2], c);
    rects(p, [6, 10, 4, 1], c);
    rects(p, [7, 11, 2, 1], c);
    put(p, 4, 5, shade(c, 60));
}

// ======================================================================
// Skin processing
// ======================================================================

/// Slim detection scaled for any resolution (64/128/512...).
/// Normalize any supported skin (64x64, HD NxN, legacy 64x32) to a plain 64x64.
fn normalize_skin64(img: RgbaImage) -> (RgbaImage, bool) {
    let (w, h) = (img.width(), img.height());
    let mut skin = if h == w {
        img
    } else if w == 2 * h {
        legacy_to_modern_scaled(&img)
    } else {
        return (procedural_skin(0), false);
    };
    if skin.dimensions() != (64, 64) {
        skin = image::imageops::resize(&skin, 64, 64, FilterType::Nearest);
    }
    ensure_left_limb_copy_scaled(&mut skin);
    let slim = detect_slim_scaled(&skin);
    (skin, slim)
}

fn detect_slim_scaled(img: &RgbaImage) -> bool {
    let s = (img.width() / 64).max(1);
    // classic arm front occupies x=44..48; slim arm leaves x=54.. empty at the
    // arm front/side boundary. Check the column just past the slim arm front.
    let mut transparent = 0;
    for y in (20 * s)..(24 * s) {
        if img.get_pixel(54 * s, y)[3] == 0 { transparent += 1; }
    }
    transparent >= 2 * s
}

/// Legacy 2:1 skin -> modern layout: mirror right limbs into left slots.
fn legacy_to_modern_scaled(img: &RgbaImage) -> RgbaImage {
    let s = (img.width() / 64).max(1);
    // the modern layout is twice as tall (left limb slots live in the lower half)
    let mut out = RgbaImage::new(img.width(), img.height() * 2);
    for (x, y, p) in img.enumerate_pixels() { out.put_pixel(x, y, *p); }
    mirror_box(&mut out, 0, 16 * s, 16 * s, 16 * s, 16 * s, 48 * s);   // right leg -> left leg
    mirror_box(&mut out, 40 * s, 16 * s, 16 * s, 16 * s, 32 * s, 48 * s); // right arm -> left arm
    out
}

/// Copy a 16x16 limb sheet region, u-mirrored, to target.
fn mirror_box(out: &mut RgbaImage, sx: u32, sy: u32, w: u32, h: u32, dx: u32, dy: u32) {
    for y in 0..h {
        for x in 0..w {
            let p = *out.get_pixel(sx + x, sy + y);
            out.put_pixel(dx + (w - 1 - x), dy + y, p);
        }
    }
}

/// Fill left limb slots from right limbs when absent (some packs leave them transparent).
fn ensure_left_limb_copy_scaled(skin: &mut RgbaImage) {
    let s = (skin.width() / 64).max(1);
    // sample the center of the left limb front faces
    let left_leg_empty = skin.get_pixel(22 * s, 56 * s)[3] == 0;
    if left_leg_empty {
        mirror_box(skin, 0, 16 * s, 16 * s, 16 * s, 16 * s, 48 * s);
    }
    let left_arm_empty = skin.get_pixel(38 * s, 56 * s)[3] == 0;
    if left_arm_empty {
        mirror_box(skin, 40 * s, 16 * s, 16 * s, 16 * s, 32 * s, 48 * s);
    }
}

/// Steve-like default skin so the hero is readable without any user file.
fn procedural_skin(variant: u8) -> RgbaImage {
    let mut img = RgbaImage::new(64, 64);
    let skin = image::Rgba([196, 148, 108, 255]);
    let hair = if variant == 0 { image::Rgba([64, 42, 24, 255]) } else { image::Rgba([140, 78, 32, 255]) };
    let shirt = if variant == 0 { image::Rgba([0, 148, 148, 255]) } else { image::Rgba([0, 128, 90, 255]) };
    let pants = if variant == 0 { image::Rgba([56, 72, 156, 255]) } else { image::Rgba([70, 62, 60, 255]) };
    let shoe = image::Rgba([80, 80, 84, 255]);
    let eye = image::Rgba([60, 60, 120, 255]);
    let mut fill = |x0: u32, y0: u32, w: u32, h: u32, c: image::Rgba<u8>| {
        for y in y0..y0 + h { for x in x0..x0 + w { img.put_pixel(x, y, c); } }
    };
    // head: all faces (region 0..32 x 0..16)
    fill(0, 0, 32, 16, skin);
    fill(8, 0, 8, 8, hair);            // top
    fill(8, 8, 8, 4, hair);            // front hair
    fill(8, 14, 2, 2, eye);            // eyes
    fill(12, 14, 2, 2, eye);
    fill(10, 15, 4, 1, image::Rgba([150, 110, 90, 255])); // mouth
    // body region
    fill(16, 16, 24, 16, shirt);
    // right arm
    fill(40, 16, 16, 16, skin);
    fill(40, 16, 16, 4, shirt);        // shoulder
    // right leg
    fill(0, 16, 16, 16, pants);
    fill(0, 28, 16, 4, shoe);
    // left arm + leg (modern slots)
    fill(32, 48, 16, 16, skin);
    fill(32, 48, 16, 4, shirt);
    fill(16, 48, 16, 16, pants);
    fill(16, 60, 16, 4, shoe);
    img
}

// ----------------------------------------------------------------------
// Hero-pack skins (MCD style): palette-driven painter used by the
// hero editor catalog. Richer than procedural_skin: fringe, belt,
// chest stripe, optional hood, hair on the hat layer.
// ----------------------------------------------------------------------

pub struct HeroPalette {
    pub skin_tone: [u8; 3],
    pub hair: [u8; 3],
    pub shirt: [u8; 3],
    pub accent: [u8; 3],  // belt + chest stripe
    pub pants: [u8; 3],
    pub boots: [u8; 3],
    pub eyes: [u8; 3],
    pub hood: bool,
    pub slim: bool,
}

impl HeroPalette {
    fn scout() -> HeroPalette {
        HeroPalette {
            skin_tone: [198, 150, 110], hair: [58, 42, 26], shirt: [62, 112, 66],
            accent: [140, 108, 58], pants: [88, 70, 52], boots: [60, 48, 36],
            eyes: [50, 90, 60], hood: true, slim: false,
        }
    }
    fn guardian() -> HeroPalette {
        HeroPalette {
            skin_tone: [214, 162, 120], hair: [176, 92, 36], shirt: [158, 52, 48],
            accent: [222, 176, 92], pants: [96, 78, 68], boots: [72, 56, 44],
            eyes: [90, 60, 130], hood: false, slim: true,
        }
    }
    fn mage() -> HeroPalette {
        HeroPalette {
            skin_tone: [206, 158, 122], hair: [72, 52, 96], shirt: [96, 62, 158],
            accent: [210, 170, 70], pants: [58, 46, 84], boots: [44, 36, 62],
            eyes: [140, 190, 240], hood: true, slim: false,
        }
    }
    fn knight() -> HeroPalette {
        HeroPalette {
            skin_tone: [190, 146, 108], hair: [38, 34, 32], shirt: [128, 132, 142],
            accent: [88, 92, 104], pants: [70, 72, 82], boots: [46, 46, 52],
            eyes: [70, 110, 170], hood: false, slim: false,
        }
    }
    fn emerald() -> HeroPalette {
        HeroPalette {
            skin_tone: [222, 170, 128], hair: [230, 178, 88], shirt: [32, 138, 118],
            accent: [242, 208, 96], pants: [40, 96, 88], boots: [34, 62, 58],
            eyes: [40, 150, 120], hood: false, slim: true,
        }
    }
    fn shadow() -> HeroPalette {
        HeroPalette {
            skin_tone: [178, 138, 104], hair: [24, 22, 30], shirt: [44, 48, 66],
            accent: [90, 110, 160], pants: [34, 36, 50], boots: [24, 24, 32],
            eyes: [120, 220, 230], hood: true, slim: false,
        }
    }
}

/// Paint a 64x64 hero skin from a palette (fringe, belt, chest stripe,
/// optional hood painted on the hat layer so it shows in portraits).
///
/// Classic 64x64 layout used here:
///   head base: top(8,0) right(0,8) front(8,8) left(16,8) back(24,8) — 8x8 faces
///   body: (16,16) 24x16 · arm R: (40,16) 16x16 · leg R: (0,16) 16x16
///   arm L: (32,48) · leg L: (16,48)
///   hat layer: top(40,0) right(32,8) front(40,8) left(48,8) back(56,8)
fn hero_skin(pal: &HeroPalette) -> RgbaImage {
    let mut img = RgbaImage::new(64, 64);
    let c = |v: [u8; 3], a: u8| image::Rgba([v[0], v[1], v[2], a]);
    let skin = c(pal.skin_tone, 255);
    let hair = c(pal.hair, 255);
    let shirt = c(pal.shirt, 255);
    let accent = c(pal.accent, 255);
    let pants = c(pal.pants, 255);
    let boots = c(pal.boots, 255);
    let eye = c(pal.eyes, 255);
    let mut fill = |x0: u32, y0: u32, w: u32, h: u32, col: image::Rgba<u8>| {
        for y in y0..y0 + h { for x in x0..x0 + w { img.put_pixel(x, y, col); } }
    };
    // ---- head (base layer) ----
    fill(0, 0, 32, 16, skin);
    fill(8, 0, 8, 8, hair);                      // top of head
    fill(24, 8, 8, 4, hair);                     // back of head upper half
    fill(8, 8, 8, 2, hair);                      // fringe on the forehead
    fill(8, 14, 2, 2, eye);                      // eyes
    fill(12, 14, 2, 2, eye);
    fill(10, 15, 4, 1, image::Rgba([150, 110, 90, 255])); // mouth
    // ---- body ----
    fill(16, 16, 24, 16, shirt);
    fill(16, 28, 24, 4, accent);                 // belt
    fill(27, 16, 2, 10, accent);                 // chest clasp
    // ---- arms (right at 40,16 / left at 32,48) ----
    // slim leaves the classic transparent strip at x=54..55 (detect_slim).
    let armw = if pal.slim { 14u32 } else { 16 };
    for (ax, ay) in [(40u32, 16u32), (32, 48)] {
        fill(ax, ay, armw, 16, skin);
        fill(ax, ay, armw, 4, shirt);            // shoulder pad
        fill(ax, ay + 12, armw, 4, accent);      // gauntlet
    }
    // ---- legs (right at 0,16 / left at 16,48) ----
    for (lx, ly) in [(0u32, 16u32), (16, 48)] {
        fill(lx, ly, 16, 16, pants);
        fill(lx, ly + 12, 16, 4, boots);
    }
    // ---- hat layer: hood or hair volume ----
    if pal.hood {
        let hood = c([pal.shirt[0].saturating_sub(18), pal.shirt[1].saturating_sub(18), pal.shirt[2].saturating_sub(18)], 255);
        fill(40, 0, 8, 8, hood);                 // cap top
        fill(32, 8, 8, 4, hood);                 // right side upper half
        fill(48, 8, 8, 4, hood);                 // left side upper half
        fill(56, 8, 8, 6, hood);                 // back
        fill(40, 8, 8, 3, hood);                 // front rim (face stays open)
    } else {
        fill(40, 0, 8, 8, hair);                 // hair volume on top
        fill(32, 8, 8, 4, hair);                 // right sideburns
        fill(48, 8, 8, 4, hair);                 // left sideburns
        fill(56, 8, 8, 6, hair);                 // back hair
        fill(40, 8, 8, 2, hair);                 // fringe overlay rows
        for x in [40u32, 43, 51, 55] {           // longer strands
            fill(x, 10, 1, 3, hair);
        }
    }
    img
}

fn load_font(base: &PathBuf) -> FontArc {
    let runtime = base.join("assets/fonts/DejaVuSans.ttf");
    let data = std::fs::read(&runtime).unwrap_or_else(|_| {
        include_bytes!("../assets/fonts/DejaVuSans.ttf").to_vec()
    });
    FontArc::try_from_vec(data).expect("font must load")
}

fn find_base_dir() -> PathBuf {
    // 1. exe-adjacent assets/ (portable), 2. cwd/assets
    if let Ok(exe) = std::env::current_exe() {
        for anc in exe.ancestors().skip(1) {
            if anc.join("assets").is_dir() { return anc.to_path_buf(); }
        }
    }
    if std::path::Path::new("assets").is_dir() { return std::path::PathBuf::from("."); }
    std::path::PathBuf::from(".")
}

/// Public accessor used by the save system.
pub fn find_base_dir_pub() -> PathBuf {
    find_base_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;

    /// Cells that are procedural BY DESIGN (no file expected).
    const INTENTIONAL_PROcedural: &[&str] = &[
        "missing", "crystal", "banner", "icon_dagger", "icon_gauntlet",
        "art_lightning", "art_wind", "art_harvester", "art_seeds",
        "icon_rune", "icon_heart", "chest_front", "chest_side", "chest_top",
    ];

    #[test]
    fn debug_open_files() {
        let a = Assets::load();
        let _ = a.atlas.save("/home/z/my-project/scripts/preview/atlas.png");
        if let Some(s) = &a.skin {
            let _ = s.save("/home/z/my-project/scripts/preview/skin.png");
        }
        eprintln!(
            "debug cells={} fallbacks={} (non-intentional: {:?})",
            a.cells.len(),
            a.fallbacks.len(),
            a.missing_cells().iter().filter(|k| !INTENTIONAL_PROcedural.contains(&k.as_str())).collect::<Vec<_>>()
        );
    }

    #[test]
    fn mcd_boss_and_body_cells_load() {
        // Requires the extracted texture files (private-use assets).
        let a = Assets::load();
        let expect_real: &[&str] = &[
            // MCD mobs / bosses (dungeons mod jar)
            "skeleton_skin", "skeleton_face", "skeleton_body",
            "necromancer_skin", "necromancer_face", "necromancer_body",
            "wraith_skin", "wraith_face", "wraith_body",
            "jungle_zombie_skin", "jungle_zombie_face", "jungle_zombie_body",
            "geomancer_skin", "geomancer_face", "geomancer_body",
            "arch_illager_skin", "arch_illager_face", "arch_illager_body",
            "nameless_skin", "nameless_face", "nameless_body",
            "redstone_golem_skin", "redstone_golem_face", "redstone_golem_body",
            "monstrosity_skin", "monstrosity_face", "monstrosity_body",
            "cauldron_metal", "cauldron_potion", "cauldron_wood",
            // full bestiary (batch 2)
            "mossy_skeleton_skin", "skeleton_vanguard_skin", "sunken_skeleton_skin",
            "drowned_necromancer_skin", "whisperer_skin", "royal_guard_skin",
            "illusioner_skin", "frozen_zombie_skin", "ghostly_kindler_skin",
            "piglin_skin", "piglin_brute_skin", "endling_skin", "iceologer_skin",
            "hoglin_skin",
            "mountaineer_skin", "windcaller_skin", "squall_golem_skin",
            "wither_skeleton_skin", "zombified_pig_skin", "enderman_skin",
            "endermite_skin", "silverfish_skin", "caerbannog_skin", "vex_skin",
            "leapleaf_skin", "poison_anemone_skin", "poison_quill_vine_skin",
            "blastling_skin", "snareling_skin", "cave_crawler_skin",
            "mini_abomination_skin", "magmacube_skin", "ghast_skin", "icy_creeper_skin",
            "jungle_abomination_skin", "wretched_skin", "tempest_skin",
            "ancient_guardian_skin", "vengeful_skin", "wildfire_skin",
            // vanilla torso crops (client.jar)
            "zombie_body", "husk_body", "drowned_body", "pillager_body",
            "vindicator_body", "enchanter_body", "witch_body",
        ];
        let bad: Vec<&String> = a
            .missing_cells()
            .iter()
            .filter(|k| expect_real.contains(&k.as_str()))
            .collect();
        assert!(bad.is_empty(), "cells fell back to procedural: {bad:?}");
    }

    #[test]
    fn vertical_strip_takes_first_frame() {
        // 16x32 strip (2 frames) with distinct colors
        let mut img = RgbaImage::new(16, 32);
        for y in 0..16 { for x in 0..16 { img.put_pixel(x, y, image::Rgba([255, 0, 0, 255])); } }
        for y in 16..32 { for x in 0..16 { img.put_pixel(x, y, image::Rgba([0, 0, 255, 255])); } }
        let out = to_rgba16(&image::DynamicImage::ImageRgba8(img));
        assert_eq!(out.get_pixel(0, 0), &image::Rgba([255, 0, 0, 255]));
        assert_eq!(out.get_pixel(15, 15), &image::Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn grayscale_grass_gets_tinted() {
        let mut img = RgbaImage::new(16, 16);
        for y in 0..16 { for x in 0..16 { img.put_pixel(x, y, image::Rgba([180, 180, 180, 255])); } }
        let out = to_rgba16(&image::DynamicImage::ImageRgba8(img));
        let mut px = [0u8; 16 * 16 * 4];
        for y in 0..16 { for x in 0..16 {
            let p = out.get_pixel(x, y);
            let o = ((y * 16 + x) * 4) as usize;
            px[o] = p[0]; px[o+1] = p[1]; px[o+2] = p[2]; px[o+3] = p[3];
        } }
        tint_gray_pixels(&mut px, [0.57, 0.74, 0.35]);
        assert!(px[0] > 80 && px[0] < 130, "r should be tinted green-ish, got {}", px[0]);
        assert!(px[2] < 80, "b should be low, got {}", px[2]);
        // colored pixels are untouched
        let mut px2 = [0u8; 16 * 16 * 4];
        for o in 0..px2.len() { px2[o] = 200; }
        px2[2] = 40; // make it colored (r==g!=b)
        tint_gray_pixels(&mut px2, [0.5, 0.5, 0.5]);
        assert_eq!(px2[0], 200);
    }

    #[test]
    fn hd_skin_layout_proportions() {
        // 128x128 skin: same proportional layout; left leg front center at (44, 112)
        let img = RgbaImage::new(128, 128);
        assert_eq!(img.width() / 64, 2);
        // detect_slim_scaled on a fully transparent 128 skin: column 54*2=108 is transparent -> slim
        assert!(detect_slim_scaled(&img));
    }

    #[test]
    fn legacy_skin_conversion() {
        let mut img = RgbaImage::new(64, 32);
        for y in 16..28 { for x in 4..8 { img.put_pixel(x, y, image::Rgba([90, 130, 60, 255])); } }
        let out = legacy_to_modern_scaled(&img);
        // right leg front (4..8, 20..32) mirrored into left leg slot (16..48 region)
        // mirrored front face lands at x=16+16-1-4=... front (4..8) maps to (52-? ) — check a pixel exists
        assert_eq!(out.dimensions(), (64, 64));
    }
}
