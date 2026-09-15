//! Level representation: tile grid, blocks, decor, collision.

pub mod gen;
pub mod missions;

use crate::assets::Assets;
use crate::gfx::{BoxInstance, Vec4};
use glam::{Vec2, Vec3};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Block {
    Grass, Dirt, Stone, Cobble, Mossy, Planks, Log, Leaves, Sand, Sandstone,
    StoneBrick, MossyBrick, CrackedBrick, Water, Lava, Netherrack, NetherBrick,
    Obsidian, Purpur, Endstone, RedstoneOre, Glowstone, Mud, Deepslate, Sculk,
    Cactus, Pumpkin, Snow, Ice, Gravel, Clay, Moss, Crystal, SoulSand, Podzol, RedSand,
}

impl Block {
    /// (top, side, bottom) atlas keys
    pub fn tex(&self) -> (&'static str, &'static str, &'static str) {
        match self {
            Block::Grass => ("grass_top", "grass_side", "dirt"),
            Block::Dirt => ("dirt", "dirt", "dirt"),
            Block::Stone => ("stone", "stone", "stone"),
            Block::Cobble => ("cobble", "cobble", "cobble"),
            Block::Mossy => ("mossy", "mossy", "mossy"),
            Block::Planks => ("planks_oak", "planks_oak", "planks_oak"),
            Block::Log => ("log_top", "log_side", "log_top"),
            Block::Leaves => ("leaves_oak", "leaves_oak", "leaves_oak"),
            Block::Sand => ("sand", "sand", "sand"),
            Block::Sandstone => ("sandstone", "sandstone", "sandstone"),
            Block::StoneBrick => ("stonebrick", "stonebrick", "stonebrick"),
            Block::MossyBrick => ("mossy_stonebrick", "mossy_stonebrick", "mossy_stonebrick"),
            Block::CrackedBrick => ("cracked_stonebrick", "cracked_stonebrick", "cracked_stonebrick"),
            Block::Water => ("water", "water", "water"),
            Block::Lava => ("lava", "lava", "lava"),
            Block::Netherrack => ("netherrack", "netherrack", "netherrack"),
            Block::NetherBrick => ("netherbrick", "netherbrick", "netherbrick"),
            Block::Obsidian => ("obsidian", "obsidian", "obsidian"),
            Block::Purpur => ("purpur", "purpur", "purpur"),
            Block::Endstone => ("endstone", "endstone", "endstone"),
            Block::RedstoneOre => ("redstone_ore", "redstone_ore", "redstone_ore"),
            Block::Glowstone => ("glowstone", "glowstone", "glowstone"),
            Block::Mud => ("mud", "mud", "mud"),
            Block::Deepslate => ("deepslate", "deepslate", "deepslate"),
            Block::Sculk => ("sculk", "sculk", "sculk"),
            Block::Cactus => ("cactus_top", "cactus_side", "cactus_top"),
            Block::Pumpkin => ("pumpkin_top", "pumpkin_side", "pumpkin_top"),
            Block::Snow => ("snow", "snow", "snow"),
            Block::Ice => ("ice", "ice", "ice"),
            Block::Gravel => ("gravel", "gravel", "gravel"),
            Block::Clay => ("clay", "clay", "clay"),
            Block::Moss => ("moss", "moss", "moss"),
            Block::Crystal => ("crystal", "crystal", "crystal"),
            Block::SoulSand => ("soul_sand", "soul_sand", "soul_sand"),
            Block::Podzol => ("podzol", "podzol", "dirt"),
            Block::RedSand => ("red_sand", "red_sand", "red_sand"),
        }
    }

    pub fn emissive(&self) -> f32 {
        match self {
            Block::Lava => 0.85,
            Block::Glowstone => 0.7,
            Block::Crystal => 0.45,
            Block::Sculk => 0.25,
            _ => 0.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Decor {
    Tree,
    Rock,
    Bush,
    CactusPlant,
    CrystalCluster,
    Torch,
    PumpkinProp,
    Chest { opened: bool },
    EmeraldPile,
    Lever { pulled: bool },
    Rune { taken: bool },
    Fountain { used: bool },
    Portal { active: bool },
    Banner,
    Cage,
    /// Warm campfire with ground light pool (MCD camps, refs 17/24/26).
    Campfire,
    /// Illager camp tent (orange canvas, refs 17/24).
    Tent,
    /// Barrel prop (rooms / docks) — solid.
    Barrel,
    /// Wooden crate prop — solid.
    Crate,
    /// Post lantern (warm glow, walkable) — MCD villages refs 18/48.
    Lantern,
    /// Broken wall segment (mossy/cracked ruins) — solid.
    Ruin,
    /// Tall stone pillar (arena ring, ref_46) — solid.
    Pillar,
    /// Fire bowl on a pedestal (ref_46 braziers) — walkable, emits light.
    Brazier,
    /// Thin wooden rail post (bridge fences, ref_18) — walkable.
    Fence,
}

impl Decor {
    pub fn solid(&self) -> bool {
        matches!(
            self,
            Decor::Tree | Decor::Rock | Decor::CactusPlant | Decor::CrystalCluster
                | Decor::Chest { .. } | Decor::Cage | Decor::Portal { .. } | Decor::Tent
                | Decor::Barrel | Decor::Crate | Decor::Ruin | Decor::Pillar
        )
    }
}

#[derive(Clone, Copy)]
pub struct Tile {
    pub ground: Block,
    pub wall: u8, // 0..=2 stacked blocks above ground
    pub wall_block: Block,
    pub decor: Option<Decor>,
}

impl Tile {
    pub fn solid(&self) -> bool {
        self.wall > 0 || self.decor.map(|d| d.solid()).unwrap_or(false)
    }
    pub fn is_water(&self) -> bool {
        self.ground == Block::Water
    }
}

#[derive(Clone)]
pub struct SpawnGroup {
    pub pos: Vec2,
    pub radius: f32,
    pub kinds: Vec<String>,
    pub count: u32,
    pub activated: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub struct RectI {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

pub struct Level {
    pub w: u32,
    pub h: u32,
    pub tiles: Vec<Tile>,
    pub spawns: Vec<Vec2>,
    pub boss_spawn: Option<Vec2>,
    pub portal: Option<(Vec2, bool)>, // pos, active
    pub groups: Vec<SpawnGroup>,
    pub secret_tiles: Vec<usize>,
    pub secret_opened: bool,
    pub has_rune: bool,
    pub lever: Option<Vec2>,
    pub biome: crate::world::missions::Biome,
    pub fog: (Vec3, f32, f32),
    pub captives: Vec<Vec2>,
    pub chests: Vec<Vec2>,
}

impl Level {
    /// Overgrown biomes whose deep wall mass is capped with a leaf canopy.
    pub fn canopy_biome(&self) -> bool {
        use crate::world::missions::Biome;
        matches!(self.biome, Biome::Forest | Biome::Jungle | Biome::Swamp | Biome::Plains | Biome::Haven)
    }

    pub fn idx(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return None;
        }
        Some((y as u32 * self.w + x as u32) as usize)
    }

    pub fn tile_at(&self, wx: f32, wz: f32) -> Option<&Tile> {
        let x = wx.floor() as i32;
        let y = wz.floor() as i32;
        self.idx(x, y).map(|i| &self.tiles[i])
    }

    /// Dégager une clairière circulaire (page titre : camp du héros, vrai menu).
    /// Sol conservé (l'eau devient de l'herbe), murs et décors supprimés.
    pub fn carve_plaza(&mut self, center: Vec2, radius: f32) {
        let cx = center.x.floor() as i32;
        let cy = center.y.floor() as i32;
        let r2 = radius * radius;
        let reach = radius.ceil() as i32 + 1;
        for y in (cy - reach)..=(cy + reach) {
            for x in (cx - reach)..=(cx + reach) {
                if let Some(i) = self.idx(x, y) {
                    let dx = x as f32 + 0.5 - center.x;
                    let dy = y as f32 + 0.5 - center.y;
                    if dx * dx + dy * dy <= r2 {
                        let t = &mut self.tiles[i];
                        if t.ground == Block::Water {
                            t.ground = Block::Grass;
                        }
                        t.wall = 0;
                        t.decor = None;
                    }
                }
            }
        }
    }

    pub fn solid_at(&self, wx: f32, wz: f32) -> bool {
        match self.tile_at(wx, wz) {
            Some(t) => t.solid(),
            None => true,
        }
    }

    pub fn water_at(&self, wx: f32, wz: f32) -> bool {
        self.tile_at(wx, wz).map(|t| t.is_water()).unwrap_or(false)
    }

    /// Circle-vs-grid collision resolve for an entity at pos with radius r.
    pub fn collide(&self, pos: &mut Vec2, r: f32) {
        let offsets = [
            Vec2::new(1.0, 0.0), Vec2::new(-1.0, 0.0),
            Vec2::new(0.0, 1.0), Vec2::new(0.0, -1.0),
        ];
        for _ in 0..2 {
            for d in offsets {
                let probe = *pos + d * r;
                if self.solid_at(probe.x, probe.y) {
                    // push back to tile edge
                    let tx = probe.x.floor();
                    let ty = probe.y.floor();
                    if d.x > 0.0 { pos.x = tx - r - 0.01; }
                    if d.x < 0.0 { pos.x = tx + 1.0 + r + 0.01; }
                    if d.y > 0.0 { pos.y = ty - r - 0.01; }
                    if d.y < 0.0 { pos.y = ty + 1.0 + r + 0.01; }
                }
            }
        }
    }

    /// Bake all static geometry into box instances.
    pub fn bake_from(&self, assets: &Assets, out: &mut Vec<BoxInstance>) {
        let norm = |key: &str| crate::gfx::cell_norm(assets, key);
        let white = Vec4::new(1.0, 1.0, 1.0, 1.0);
        for y in 0..self.h as i32 {
            for x in 0..self.w as i32 {
                let i = self.idx(x, y).unwrap();
                let t = self.tiles[i];
                let cx = x as f32 + 0.5;
                let cz = y as f32 + 0.5;
                // ground
                let (top, side, bottom) = t.ground.tex();
                if t.ground == Block::Water {
                    out.push(BoxInstance::cube(
                        Vec3::new(cx, -0.36, cz),
                        Vec3::new(1.0, 0.72, 1.0),
                        norm(side), Vec4::new(0.85, 0.95, 1.0, 0.85), t.ground.emissive(),
                    ));
                } else {
                    let uv_top = norm(top);
                    let uv_side = norm(side);
                    let uv_bot = norm(bottom);
                    out.push(BoxInstance::new(
                        Vec3::new(cx, -0.5, cz),
                        glam::Quat::IDENTITY,
                        Vec3::ONE,
                        [uv_top, uv_bot, uv_side, uv_side, uv_side, uv_side],
                        white,
                        t.ground.emissive(),
                    ));
                }
                // walls — the top layer of deep mass gets a leafy canopy in
                // overgrown biomes (MCD forest look: playfield ringed by treetops)
                let canopy = self.canopy_biome();
                for wl in 0..t.wall as i32 {
                    let last = wl == t.wall as i32 - 1;
                    if canopy && last && t.wall > 2 {
                        let lv = norm("leaves_oak");
                        out.push(BoxInstance::new(
                            Vec3::new(cx, 0.5 + wl as f32, cz),
                            glam::Quat::IDENTITY,
                            Vec3::new(1.06, 1.0, 1.06),
                            [lv, lv, lv, lv, lv, lv],
                            Vec4::new(0.92, 1.05, 0.88, 1.0),
                            0.0,
                        ));
                    } else {
                        let (wtop, wside, wbot) = t.wall_block.tex();
                        let uv_top = norm(wtop);
                        let uv_side = norm(wside);
                        let uv_bot = norm(wbot);
                        out.push(BoxInstance::new(
                            Vec3::new(cx, 0.5 + wl as f32, cz),
                            glam::Quat::IDENTITY,
                            Vec3::ONE,
                            [uv_top, uv_bot, uv_side, uv_side, uv_side, uv_side],
                            white,
                            t.wall_block.emissive(),
                        ));
                    }
                }
                // decor
                if let Some(d) = t.decor {
                    self.bake_decor(d, cx, cz, &norm, out);
                }
            }
        }
    }

    fn bake_decor(
        &self,
        d: Decor,
        cx: f32,
        cz: f32,
        norm: &dyn Fn(&str) -> [f32; 4],
        out: &mut Vec<BoxInstance>,
    ) {
        let white = Vec4::new(1.0, 1.0, 1.0, 1.0);
        macro_rules! cube {
            ($pos:expr, $s:expr, $key:expr, $tint:expr, $e:expr) => {
                out.push(BoxInstance::cube($pos, $s, norm($key), $tint, $e))
            };
        }
        macro_rules! box6 {
            ($pos:expr, $s:expr, $top:expr, $side:expr, $bot:expr, $tint:expr, $e:expr) => {
                out.push(BoxInstance::new(
                    $pos, glam::Quat::IDENTITY, $s,
                    [norm($top), norm($bot), norm($side), norm($side), norm($side), norm($side)],
                    $tint, $e,
                ))
            };
        }
        match d {
            Decor::Tree => {
                box6!(Vec3::new(cx, 1.1, cz), Vec3::new(0.45, 2.2, 0.45), "log_top", "log_side", "log_top", white, 0.0);
                cube!(Vec3::new(cx, 2.35, cz), Vec3::new(2.1, 0.85, 2.1), "leaves_oak", white, 0.0);
                cube!(Vec3::new(cx, 3.1, cz), Vec3::new(1.4, 0.75, 1.4), "leaves_oak", white, 0.0);
                cube!(Vec3::new(cx, 3.7, cz), Vec3::new(0.7, 0.55, 0.7), "leaves_oak", white, 0.0);
            }
            Decor::Rock => cube!(Vec3::new(cx, 0.27, cz), Vec3::new(0.85, 0.55, 0.85), "cobble", white, 0.0),
            Decor::Bush => cube!(Vec3::new(cx, 0.3, cz), Vec3::new(0.85, 0.6, 0.85), "leaves_oak", white, 0.0),
            Decor::CactusPlant => box6!(Vec3::new(cx, 0.8, cz), Vec3::new(0.55, 1.6, 0.55), "cactus_top", "cactus_side", "cactus_top", white, 0.0),
            Decor::CrystalCluster => {
                box6!(Vec3::new(cx, 0.6, cz), Vec3::new(0.4, 1.2, 0.4), "crystal", "crystal", "crystal", white, 0.5);
                box6!(Vec3::new(cx + 0.3, 0.35, cz + 0.2), Vec3::new(0.25, 0.7, 0.25), "crystal", "crystal", "crystal", white, 0.5);
            }
            Decor::Torch => {
                cube!(Vec3::new(cx, 0.55, cz), Vec3::new(0.12, 1.1, 0.12), "log_side", white, 0.0);
                cube!(Vec3::new(cx, 1.2, cz), Vec3::new(0.22, 0.25, 0.22), "lava", white, 0.9);
            }
            Decor::PumpkinProp => box6!(Vec3::new(cx, 0.42, cz), Vec3::new(0.8, 0.8, 0.8), "pumpkin_top", "carved_pumpkin", "pumpkin_top", white, 0.05),
            Decor::Chest { .. } => {
                box6!(Vec3::new(cx, 0.33, cz), Vec3::new(0.85, 0.66, 0.6), "chest_top", "chest_front", "planks_oak", white, 0.0);
            }
            Decor::EmeraldPile => {
                cube!(Vec3::new(cx, 0.16, cz), Vec3::new(0.55, 0.3, 0.55), "icon_emerald", white, 0.25);
                cube!(Vec3::new(cx + 0.2, 0.42, cz - 0.1), Vec3::new(0.24, 0.24, 0.24), "icon_emerald", white, 0.25);
            }
            Decor::Lever { pulled } => {
                cube!(Vec3::new(cx, 0.25, cz), Vec3::new(0.34, 0.5, 0.34), "cobble", white, 0.0);
                let tilt = if pulled { 0.9 } else { -0.9 };
                out.push(BoxInstance::new(
                    Vec3::new(cx, 0.62, cz),
                    glam::Quat::from_axis_angle(Vec3::X, tilt),
                    Vec3::new(0.12, 0.5, 0.12),
                    [norm("planks_oak"); 6], white, 0.0,
                ));
            }
            Decor::Rune { taken } => {
                if !taken {
                    box6!(Vec3::new(cx, 0.5, cz), Vec3::new(0.42, 1.0, 0.42), "stonebrick", "stonebrick", "stonebrick", white, 0.0);
                    out.push(BoxInstance::new(
                        Vec3::new(cx, 1.45, cz),
                        glam::Quat::from_rotation_y(0.7),
                        Vec3::splat(0.38),
                        [norm("icon_rune"); 6], white, 0.6,
                    ));
                }
            }
            Decor::Fountain { .. } => {
                for (dx, dz) in [(-0.5, -0.5), (0.5, -0.5), (-0.5, 0.5), (0.5, 0.5)] {
                    cube!(Vec3::new(cx + dx, 0.25, cz + dz), Vec3::new(0.45, 0.5, 0.45), "stonebrick", white, 0.0);
                }
                out.push(BoxInstance::cube(
                    Vec3::new(cx, 0.1, cz), Vec3::new(0.9, 0.25, 0.9), norm("water"),
                    Vec4::new(0.7, 1.0, 1.0, 0.9), 0.3,
                ));
            }
            Decor::Portal { active } => {
                let e = if active { 0.6 } else { 0.0 };
                for dx in [-1.1f32, 1.1] {
                    box6!(Vec3::new(cx + dx, 1.5, cz), Vec3::new(0.45, 3.0, 0.45), "obsidian", "obsidian", "obsidian", white, 0.0);
                }
                box6!(Vec3::new(cx, 3.15, cz), Vec3::new(2.65, 0.45, 0.45), "obsidian", "obsidian", "obsidian", white, 0.0);
                out.push(BoxInstance::cube(
                    Vec3::new(cx, 1.45, cz), Vec3::new(1.7, 2.5, 0.14), norm("portal"),
                    if active { Vec4::new(0.7, 1.0, 0.9, 0.9) } else { Vec4::new(0.3, 0.3, 0.4, 0.9) }, e,
                ));
            }
            Decor::Banner => {
                cube!(Vec3::new(cx, 1.5, cz), Vec3::new(0.12, 3.0, 0.12), "log_side", white, 0.0);
                out.push(BoxInstance::new(
                    Vec3::new(cx + 0.5, 2.2, cz),
                    glam::Quat::IDENTITY,
                    Vec3::new(0.95, 1.35, 0.09),
                    [norm("banner"); 6], white, 0.0,
                ));
            }
            Decor::Cage => {
                for (dx, dz) in [(-0.35, -0.35), (0.35, -0.35), (-0.35, 0.35), (0.35, 0.35)] {
                    cube!(Vec3::new(cx + dx, 1.1, cz + dz), Vec3::new(0.14, 2.2, 0.14), "anvil", white, 0.0);
                }
                cube!(Vec3::new(cx, 2.28, cz), Vec3::new(0.95, 0.14, 0.95), "anvil", white, 0.0);
                for i in 0..3 {
                    let o = -0.35 + i as f32 * 0.35;
                    cube!(Vec3::new(cx + o, 1.1, cz - 0.35), Vec3::new(0.06, 2.2, 0.06), "anvil", white, 0.0);
                }
            }
            Decor::Campfire => {
                // crossed logs
                out.push(BoxInstance::new(
                    Vec3::new(cx, 0.12, cz), glam::Quat::from_rotation_y(0.6),
                    Vec3::new(1.05, 0.2, 0.2), [norm("log_side"); 6], white, 0.0,
                ));
                out.push(BoxInstance::new(
                    Vec3::new(cx, 0.12, cz), glam::Quat::from_rotation_y(-0.6),
                    Vec3::new(1.05, 0.2, 0.2), [norm("log_side"); 6], white, 0.0,
                ));
                // stone ring
                for (dx, dz) in [(-0.62, -0.42), (0.62, -0.42), (-0.62, 0.42), (0.62, 0.42), (0.0, -0.72), (0.0, 0.72)] {
                    cube!(Vec3::new(cx + dx, 0.1, cz + dz), Vec3::new(0.26, 0.18, 0.26), "cobble", white, 0.0);
                }
                // flames (two stacked emissive cubes)
                cube!(Vec3::new(cx, 0.42, cz), Vec3::new(0.52, 0.62, 0.52), "lava", white, 0.95);
                cube!(Vec3::new(cx, 0.78, cz), Vec3::new(0.28, 0.34, 0.28), "lava", white, 0.95);
                // warm ground light pool — the signature MCD camp glow
                out.push(BoxInstance::cube(
                    Vec3::new(cx, 0.045, cz), Vec3::new(2.7, 0.06, 2.7), norm("lava"),
                    Vec4::new(1.0, 0.6, 0.22, 0.4), 0.75,
                ));
            }
            Decor::Tent => {
                let tint = Vec4::new(1.0, 0.78, 0.5, 1.0);
                // canvas base + narrower roof ridge
                box6!(Vec3::new(cx, 0.5, cz), Vec3::new(2.0, 1.0, 1.8), "banner", "banner", "banner", tint, 0.0);
                box6!(Vec3::new(cx, 1.22, cz), Vec3::new(1.9, 0.44, 1.0), "banner", "banner", "banner", tint, 0.0);
                // dark doorway
                out.push(BoxInstance::cube(
                    Vec3::new(cx, 0.4, cz + 0.93), Vec3::new(0.72, 0.8, 0.08), norm("planks_oak"),
                    Vec4::new(0.06, 0.045, 0.04, 1.0), 0.0,
                ));
            }
            Decor::Barrel => {
                box6!(Vec3::new(cx, 0.4, cz), Vec3::new(0.62, 0.8, 0.62), "log_top", "log_side", "log_top", Vec4::new(0.95, 0.82, 0.6, 1.0), 0.0);
                // metal hoops
                out.push(BoxInstance::cube(Vec3::new(cx, 0.28, cz), Vec3::new(0.66, 0.07, 0.66), norm("anvil"), white, 0.0));
                out.push(BoxInstance::cube(Vec3::new(cx, 0.56, cz), Vec3::new(0.66, 0.07, 0.66), norm("anvil"), white, 0.0));
            }
            Decor::Crate => {
                box6!(Vec3::new(cx, 0.32, cz), Vec3::new(0.68, 0.64, 0.68), "planks_oak", "planks_oak", "planks_oak", Vec4::new(0.8, 0.68, 0.5, 1.0), 0.0);
                // cross bracing
                out.push(BoxInstance::new(
                    Vec3::new(cx, 0.32, cz), glam::Quat::from_rotation_y(0.8),
                    Vec3::new(0.9, 0.1, 0.08), [norm("log_side"); 6], white, 0.0,
                ));
            }
            Decor::Lantern => {
                cube!(Vec3::new(cx, 0.55, cz), Vec3::new(0.1, 1.1, 0.1), "log_side", white, 0.0);
                out.push(BoxInstance::cube(
                    Vec3::new(cx, 1.22, cz), Vec3::new(0.26, 0.26, 0.26), norm("glowstone"),
                    Vec4::new(1.15, 1.05, 0.85, 1.0), 0.85,
                ));
            }
            Decor::Ruin => {
                // broken wall chunk: mossy or cracked, slight deterministic rotation
                let rot = ((cx * 7.31 + cz * 3.17).fract() - 0.5) * 0.7;
                let key = if ((cx * 5.0).floor() as i64 + (cz * 9.0).floor() as i64) % 2 == 0 { "mossy_stonebrick" } else { "cracked_stonebrick" };
                out.push(BoxInstance::new(
                    Vec3::new(cx, 0.32, cz), glam::Quat::from_rotation_y(rot),
                    Vec3::new(0.95, 0.64, 0.34), [norm(key); 6], white, 0.0,
                ));
            }
            Decor::Pillar => {
                box6!(Vec3::new(cx, 1.25, cz), Vec3::new(0.66, 2.5, 0.66), "stonebrick", "stonebrick", "stonebrick", white, 0.0);
                // cap + base slabs
                out.push(BoxInstance::cube(Vec3::new(cx, 2.56, cz), Vec3::new(0.92, 0.18, 0.92), norm("stonebrick"), white, 0.0));
                out.push(BoxInstance::cube(Vec3::new(cx, 0.09, cz), Vec3::new(0.92, 0.18, 0.92), norm("stonebrick"), white, 0.0));
            }
            Decor::Brazier => {
                cube!(Vec3::new(cx, 0.22, cz), Vec3::new(0.52, 0.44, 0.52), "cobble", white, 0.0);
                cube!(Vec3::new(cx, 0.55, cz), Vec3::new(0.38, 0.24, 0.38), "stonebrick", white, 0.0);
                cube!(Vec3::new(cx, 0.82, cz), Vec3::new(0.3, 0.34, 0.3), "lava", white, 0.95);
                // warm ground pool (mini campfire glow)
                out.push(BoxInstance::cube(
                    Vec3::new(cx, 0.04, cz), Vec3::new(1.7, 0.05, 1.7), norm("lava"),
                    Vec4::new(1.0, 0.62, 0.25, 0.32), 0.7,
                ));
            }
            Decor::Fence => {
                cube!(Vec3::new(cx, 0.5, cz), Vec3::new(0.14, 1.0, 0.14), "planks_oak", white, 0.0);
                cube!(Vec3::new(cx, 0.86, cz), Vec3::new(0.1, 0.08, 0.1), "log_side", white, 0.0);
            }
        }
    }

    /// Convert atlas-cell uv rects (pixels) to normalized — helper for bakes.
    pub fn cell_cache(assets: &Assets) -> HashMap<String, [f32; 4]> {
        let mut m = HashMap::new();
        for (k, r) in assets.cells.iter() {
            m.insert(k.clone(), crate::gfx::rect_norm(*r, crate::assets::ATLAS_DIM));
        }
        m
    }
}
