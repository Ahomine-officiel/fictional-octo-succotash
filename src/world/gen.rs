//! Procedural level generation per biome: path + rooms + arena, side rooms,
//! secret room (lever + rune), decor scatter, spawn trigger groups.

use super::missions::{Biome, MissionDef};
use super::{Block, Decor, Level, RectI, SpawnGroup, Tile};
use glam::Vec2;
use glam::Vec3;
use rand::prelude::*;
use rand::Rng;

pub struct Palette {
    pub grounds: Vec<(Block, u32)>,
    pub wall: Block,
    pub tree_density: f32,
    pub rock_density: f32,
    pub bush_density: f32,
    pub water: f32,
    pub fog: Vec3,
    pub fog_start: f32,
    pub fog_end: f32,
}

pub fn biome_palette(biome: Biome) -> Palette {
    match biome {
        Biome::Forest => Palette {
            grounds: vec![(Block::Grass, 8), (Block::Podzol, 2), (Block::Moss, 2), (Block::Dirt, 2)],
            wall: Block::Log, tree_density: 0.10, rock_density: 0.02, bush_density: 0.10,
            water: 0.01, fog: Vec3::new(0.55, 0.72, 0.52), fog_start: 15.0, fog_end: 38.0,
        },
        Biome::Plains => Palette {
            grounds: vec![(Block::Grass, 9), (Block::Dirt, 2), (Block::Gravel, 1)],
            wall: Block::Planks, tree_density: 0.03, rock_density: 0.02, bush_density: 0.06,
            water: 0.008, fog: Vec3::new(0.65, 0.78, 0.62), fog_start: 17.0, fog_end: 42.0,
        },
        Biome::Canyon => Palette {
            grounds: vec![(Block::RedSand, 7), (Block::Sand, 3), (Block::Sandstone, 2)],
            wall: Block::Sandstone, tree_density: 0.0, rock_density: 0.03, bush_density: 0.0,
            water: 0.0, fog: Vec3::new(0.85, 0.7, 0.5), fog_start: 18.0, fog_end: 45.0,
        },
        Biome::Swamp => Palette {
            grounds: vec![(Block::Mud, 5), (Block::Grass, 4), (Block::Water, 3)],
            wall: Block::Log, tree_density: 0.07, rock_density: 0.02, bush_density: 0.12,
            water: 0.05, fog: Vec3::new(0.35, 0.45, 0.35), fog_start: 10.0, fog_end: 30.0,
        },
        Biome::Cave => Palette {
            grounds: vec![(Block::Cobble, 6), (Block::Stone, 4), (Block::Water, 2)],
            wall: Block::Cobble, tree_density: 0.0, rock_density: 0.06, bush_density: 0.02,
            water: 0.03, fog: Vec3::new(0.12, 0.12, 0.16), fog_start: 8.0, fog_end: 22.0,
        },
        Biome::Mines => Palette {
            grounds: vec![(Block::Deepslate, 5), (Block::Stone, 4), (Block::RedstoneOre, 2), (Block::Gravel, 1)],
            wall: Block::Deepslate, tree_density: 0.0, rock_density: 0.05, bush_density: 0.0,
            water: 0.0, fog: Vec3::new(0.14, 0.12, 0.12), fog_start: 9.0, fog_end: 24.0,
        },
        Biome::Desert => Palette {
            grounds: vec![(Block::Sand, 7), (Block::Sandstone, 4)],
            wall: Block::Sandstone, tree_density: 0.0, rock_density: 0.03, bush_density: 0.0,
            water: 0.0, fog: Vec3::new(0.85, 0.75, 0.55), fog_start: 18.0, fog_end: 44.0,
        },
        Biome::Haven => Palette {
            grounds: vec![(Block::Grass, 8), (Block::Moss, 3), (Block::Snow, 1)],
            wall: Block::StoneBrick, tree_density: 0.06, rock_density: 0.02, bush_density: 0.1,
            water: 0.01, fog: Vec3::new(0.7, 0.85, 0.95), fog_start: 18.0, fog_end: 48.0,
        },
        Biome::Jungle => Palette {
            grounds: vec![(Block::Grass, 6), (Block::Podzol, 3), (Block::Moss, 3), (Block::Water, 2)],
            wall: Block::Log, tree_density: 0.14, rock_density: 0.02, bush_density: 0.12,
            water: 0.04, fog: Vec3::new(0.3, 0.5, 0.32), fog_start: 10.0, fog_end: 29.0,
        },
        Biome::Stronghold => Palette {
            grounds: vec![(Block::StoneBrick, 8), (Block::CrackedBrick, 3), (Block::MossyBrick, 2)],
            wall: Block::StoneBrick, tree_density: 0.0, rock_density: 0.03, bush_density: 0.0,
            water: 0.0, fog: Vec3::new(0.16, 0.15, 0.18), fog_start: 10.0, fog_end: 26.0,
        },
        Biome::Nether => Palette {
            grounds: vec![(Block::Netherrack, 8), (Block::SoulSand, 3), (Block::Lava, 1)],
            wall: Block::NetherBrick, tree_density: 0.0, rock_density: 0.05, bush_density: 0.0,
            water: 0.0, fog: Vec3::new(0.4, 0.12, 0.1), fog_start: 11.0, fog_end: 31.0,
        },
        Biome::Pinnacle => Palette {
            grounds: vec![(Block::Obsidian, 7), (Block::Purpur, 3), (Block::Endstone, 2), (Block::Sculk, 1)],
            wall: Block::Obsidian, tree_density: 0.0, rock_density: 0.03, bush_density: 0.0,
            water: 0.0, fog: Vec3::new(0.1, 0.07, 0.15), fog_start: 11.0, fog_end: 32.0,
        },
        // ---- DLC biomes (batch 2) ----
        Biome::Winter => Palette {
            grounds: vec![(Block::Snow, 8), (Block::Ice, 2), (Block::Grass, 2), (Block::Water, 1)],
            wall: Block::Snow, tree_density: 0.07, rock_density: 0.03, bush_density: 0.05,
            water: 0.03, fog: Vec3::new(0.75, 0.85, 0.95), fog_start: 9.0, fog_end: 26.0,
        },
        Biome::Peaks => Palette {
            grounds: vec![(Block::Snow, 5), (Block::Stone, 5), (Block::Gravel, 2), (Block::Ice, 1)],
            wall: Block::Stone, tree_density: 0.0, rock_density: 0.08, bush_density: 0.0,
            water: 0.0, fog: Vec3::new(0.65, 0.75, 0.85), fog_start: 8.0, fog_end: 24.0,
        },
        Biome::Depths => Palette {
            grounds: vec![(Block::Clay, 5), (Block::Sand, 4), (Block::Water, 4), (Block::MossyBrick, 1)],
            wall: Block::StoneBrick, tree_density: 0.0, rock_density: 0.04, bush_density: 0.06,
            water: 0.08, fog: Vec3::new(0.12, 0.3, 0.38), fog_start: 8.0, fog_end: 24.0,
        },
        Biome::Void => Palette {
            grounds: vec![(Block::Endstone, 6), (Block::Obsidian, 4), (Block::Sculk, 3)],
            wall: Block::Obsidian, tree_density: 0.0, rock_density: 0.04, bush_density: 0.0,
            water: 0.0, fog: Vec3::new(0.06, 0.04, 0.1), fog_start: 7.0, fog_end: 22.0,
        },
    }
}

fn pick_ground(rng: &mut StdRng, pal: &Palette) -> Block {
    let total: u32 = pal.grounds.iter().map(|g| g.1).sum();
    let mut roll = rng.gen_range(0..total.max(1));
    for (b, w) in &pal.grounds {
        if roll < *w { return *b; }
        roll -= *w;
    }
    pal.grounds[0].0
}

pub fn generate(mission: &MissionDef, seed: u64) -> Level {
    let mut rng = StdRng::seed_from_u64(seed);
    let pal = biome_palette(mission.biome);
    let w = super::super::consts::LEVEL_W as i32;
    let h = super::super::consts::LEVEL_H as i32;

    // fill solid
    let mut tiles = vec![
        Tile { ground: pick_ground(&mut rng, &pal), wall: 2, wall_block: pal.wall, decor: None };
        (w * h) as usize
    ];

    let carve = |tiles: &mut Vec<Tile>, x: i32, y: i32, r: i32, rng: &mut StdRng, pal: &Palette| {
        for dy in -r..=r {
            for dx in -r..=r {
                let xx = x + dx;
                let yy = y + dy;
                if xx <= 0 || yy <= 0 || xx >= w - 1 || yy >= h - 1 { continue; }
                let d2 = dx * dx + dy * dy;
                if d2 <= r * r + ((dx + dy) % 2) {
                    let i = (yy as u32 * w as u32 + xx as u32) as usize;
                    tiles[i] = Tile { ground: pick_ground(rng, pal), wall: 0, wall_block: pal.wall, decor: None };
                }
            }
        }
    };

    // ---- main path: south to north with wander ----
    let mut px = w / 2;
    let mut py = h - 8;
    let mut path_points: Vec<(i32, i32)> = vec![(px, py)];
    // corridor tiles (main path only) — used to place narrow plank bridges
    let mut corridor: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::new();
    let target_y = 10;
    while py > target_y {
        let steps = rng.gen_range(4..8);
        for _ in 0..steps {
            if py <= target_y { break; }
            py -= 1;
            px = (px + rng.gen_range(-1..=1)).clamp(4, w - 5);
            carve(&mut tiles, px, py, 1, &mut rng, &pal);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    corridor.insert((px + dx, py + dy));
                }
            }
        }
        // room
        let room_r = rng.gen_range(3..6);
        carve(&mut tiles, px, py, room_r, &mut rng, &pal);
        path_points.push((px, py));
    }

    // ---- arena at the end ----
    let arena = (px, target_y.max(8));
    carve(&mut tiles, arena.0, arena.1, 9, &mut rng, &pal);
    path_points.push(arena);

    // ---- side pockets with loot ----
    let mut side_rooms: Vec<(i32, i32)> = Vec::new();
    for _ in 0..3 {
        if path_points.len() < 3 { break; }
        let pp = path_points[rng.gen_range(1..path_points.len() - 1)];
        let dir = if rng.gen_bool(0.5) { 1 } else { -1 };
        let sx = (pp.0 + dir * rng.gen_range(8..12)).clamp(5, w - 6);
        let sy = (pp.1 + rng.gen_range(-4..=4)).clamp(5, h - 6);
        // corridor
        let steps = 10;
        for s in 0..steps {
            let cx = pp.0 + (sx - pp.0) * s / steps;
            let cy = pp.1 + (sy - pp.1) * s / steps;
            carve(&mut tiles, cx, cy, 1, &mut rng, &pal);
        }
        carve(&mut tiles, sx, sy, 4, &mut rng, &pal);
        side_rooms.push((sx, sy));
    }

    // ---- secret room (hidden behind lever) ----
    let mut secret_tiles: Vec<usize> = Vec::new();
    let has_rune = crate::world::missions::secret_unlocked_by_rune(mission.id).is_some();
    let mut lever_pos = None;
    if has_rune {
        let pp = path_points[path_points.len() / 2];
        let dir = if rng.gen_bool(0.5) { 1 } else { -1 };
        let sx = (pp.0 + dir * 10).clamp(4, w - 5);
        let sy = pp.1;
        // corridor (hidden: stays walled until lever pulled)
        for s in 0..10 {
            let cx = pp.0 + (sx - pp.0) * s / 10;
            let cy = pp.1 + (sy - pp.1) * s / 10;
            let i = (cy as u32 * w as u32 + cx as u32) as usize;
            if tiles[i].wall == 0 {
                // keep track only of currently-solid tiles on the corridor
            }
            secret_tiles.push(i);
        }
        carve(&mut tiles, sx, sy, 4, &mut rng, &pal);
        // rune pedestal in the middle
        let ri = (sy as u32 * w as u32 + sx as u32) as usize;
        tiles[ri].decor = Some(Decor::Rune { taken: false });
        // lever placed in a random side room (or path room)
        let lp = if !side_rooms.is_empty() { side_rooms[rng.gen_range(0..side_rooms.len())] } else { path_points[1] };
        lever_pos = Some(Vec2::new(lp.0 as f32 + 0.5, lp.1 as f32 + 0.5));
        let li = (lp.1 as u32 * w as u32 + lp.0 as u32) as usize;
        if tiles[li].decor.is_none() {
            tiles[li].decor = Some(Decor::Lever { pulled: false });
        }
    }

    // ---- decor scatter ----
    let open = |tiles: &Vec<Tile>, x: i32, y: i32| -> bool {
        x > 0 && y > 0 && x < w - 1 && y < h - 1 && tiles[(y as u32 * w as u32 + x as u32) as usize].wall == 0
    };
    let path_set: std::collections::HashSet<(i32, i32)> =
        path_points.iter().map(|p| *p).collect();
    let decor_guards = |x: i32, y: i32, rng: &mut StdRng, pal: &Palette, tiles: &mut Vec<Tile>| {
        let i = (y as u32 * w as u32 + x as u32) as usize;
        if tiles[i].wall > 0 || tiles[i].decor.is_some() { return; }
        let roll: f32 = rng.gen();
        let d = if roll < pal.tree_density {
            Some(Decor::Tree)
        } else if roll < pal.tree_density + pal.rock_density {
            Some(Decor::Rock)
        } else if roll < pal.tree_density + pal.rock_density + pal.bush_density {
            Some(Decor::Bush)
        } else if mission.biome == Biome::Canyon && roll < 0.06 {
            Some(Decor::CactusPlant)
        } else if mission.biome == Biome::Desert && roll < 0.04 {
            Some(Decor::CactusPlant)
        } else if mission.biome == Biome::Cave && roll < 0.05 {
            Some(Decor::CrystalCluster)
        } else if mission.biome == Biome::Pinnacle && roll < 0.04 {
            Some(Decor::CrystalCluster)
        } else {
            None
        };
        if let Some(d) = d {
            // keep spawn/path breathing room
            let near_path = path_set.iter().any(|p| (p.0 - x).abs() < 2 && (p.1 - y).abs() < 2);
            if !near_path {
                tiles[i].decor = Some(d);
            }
        }
    };
    for y in 0..h {
        for x in 0..w {
            decor_guards(x, y, &mut rng, &pal, &mut tiles);
        }
    }

    // torches along room edges (deterministic sprinkle)
    for pp in &path_points {
        for (dx, dy) in [(-3, -3), (3, -3), (-3, 3), (3, 3)] {
            let x = pp.0 + dx;
            let y = pp.1 + dy;
            if open(&tiles, x, y) {
                let i = (y as u32 * w as u32 + x as u32) as usize;
                if tiles[i].decor.is_none() && rng.gen_bool(0.4) {
                    tiles[i].decor = Some(Decor::Torch);
                }
            }
        }
    }

    // ------------------------------------------------------------
    // MCD terrain dressing (refs 04/18/46/48) :
    // sols en plaques cohérentes, ponts de planches, accotements,
    // murets bas le long des chemins + murailles hautes en fond.
    // ------------------------------------------------------------

    // 1. ground patches — coherent plates instead of per-tile confetti
    let mut patches: Vec<(i32, i32, i32, Block)> = Vec::new();
    for _ in 0..55 {
        let b = pick_ground(&mut rng, &pal);
        patches.push((rng.gen_range(2..w - 2), rng.gen_range(2..h - 2), rng.gen_range(2..5), b));
    }
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let i = (y as u32 * w as u32 + x as u32) as usize;
            if tiles[i].wall > 0 { continue; }
            let mut best: Option<(i32, Block)> = None;
            for (qx, qy, qr, qb) in &patches {
                let d = (qx - x).abs() + (qy - y).abs();
                if d <= *qr && best.map(|(bd, _)| d < bd).unwrap_or(true) {
                    best = Some((d, *qb));
                }
            }
            if let Some((_, b)) = best {
                tiles[i].ground = b;
            }
        }
    }

    // 2. plank bridges where the main corridor crosses water (MCD docks/bridges)
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let i = (y as u32 * w as u32 + x as u32) as usize;
            if tiles[i].wall != 0 || tiles[i].ground != Block::Water { continue; }
            if corridor.contains(&(x, y)) {
                tiles[i].ground = Block::Planks;
            }
        }
    }

    // 3. trampled shoulders along the walls
    let path_blocks: Vec<Block> = match mission.biome {
        Biome::Stronghold | Biome::Pinnacle => vec![Block::CrackedBrick, Block::MossyBrick],
        Biome::Nether => vec![Block::SoulSand, Block::Netherrack],
        Biome::Canyon | Biome::Desert => vec![Block::Sandstone, Block::Gravel],
        Biome::Winter | Biome::Peaks => vec![Block::Gravel, Block::Stone],
        _ => vec![Block::Gravel, Block::Cobble],
    };
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let i = (y as u32 * w as u32 + x as u32) as usize;
            if tiles[i].wall > 0 || tiles[i].ground == Block::Water { continue; }
            let near_wall = [(-1i32, 0), (1, 0), (0, -1), (0, 1)].iter().any(|(dx, dy)| {
                let xx = x + dx;
                let yy = y + dy;
                xx >= 0 && yy >= 0 && xx < w && yy < h
                    && tiles[(yy as u32 * w as u32 + xx as u32) as usize].wall > 0
            });
            if near_wall && rng.gen_bool(0.5) {
                tiles[i].ground = path_blocks[rng.gen_range(0..path_blocks.len())];
            }
        }
    }

    // 4. low kerb walls facing paths, tall canopy/ridge backdrop beyond
    for y in 0..h {
        for x in 0..w {
            let i = (y as u32 * w as u32 + x as u32) as usize;
            if tiles[i].wall == 0 { continue; }
            let open_nb = [(-1i32, 0), (1, 0), (0, -1), (0, 1)].iter().any(|(dx, dy)| {
                let xx = x + dx;
                let yy = y + dy;
                xx >= 0 && yy >= 0 && xx < w && yy < h
                    && tiles[(yy as u32 * w as u32 + xx as u32) as usize].wall == 0
            });
            tiles[i].wall = if open_nb {
                // MCD low kerb along the paths (refs 04/18)
                if rng.gen_bool(0.75) { 1 } else { 2 }
            } else {
                // deep mass: rolling ridge / tree canopy backdrop (3..5 high)
                3 + rng.gen_range(0..3)
            };
        }
    }

    // ---- spawns / exit / boss ----
    let start = path_points[0];
    let spawns = vec![
        Vec2::new(start.0 as f32 - 0.5, start.1 as f32 + 0.5),
        Vec2::new(start.0 as f32 + 1.5, start.1 as f32 + 0.5),
    ];

    let boss_spawn = mission.boss.map(|_| Vec2::new(arena.0 as f32 + 0.5, arena.1 as f32 + 0.5));
    let portal = Some((
        Vec2::new(arena.0 as f32 + 0.5, (arena.1 - 4).max(6) as f32),
        false,
    ));
    let pi = ((portal.unwrap().0.y.floor() as i32) * w + (portal.unwrap().0.x.floor() as i32)) as usize;
    tiles[pi].decor = Some(Decor::Portal { active: false });

    // ---- chests, emerald piles, captives, fountain ----
    let mut chests = Vec::new();
    let mut captives = Vec::new();
    let place = |tiles: &mut Vec<Tile>, x: i32, y: i32, d: Decor, w: i32, chests: &mut Vec<Vec2>, captives: &mut Vec<Vec2>| {
        let i = (y as u32 * w as u32 + x as u32) as usize;
        if tiles[i].wall == 0 && tiles[i].decor.is_none() {
            tiles[i].decor = Some(d);
            let v = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            if matches!(d, Decor::Chest { .. }) { chests.push(v); }
            if matches!(d, Decor::Cage) { captives.push(v); }
        }
    };
    // chests: in side rooms + arena-ish
    for &(sx, sy) in &side_rooms {
        place(&mut tiles, sx + 1, sy, Decor::Chest { opened: false }, w, &mut chests, &mut captives);
        if rng.gen_bool(0.5) {
            place(&mut tiles, sx - 1, sy + 1, Decor::EmeraldPile, w, &mut chests, &mut captives);
        }
    }
    for pp in path_points.iter().skip(1).step_by(2) {
        let dx = rng.gen_range(-2..=2);
        let dy = rng.gen_range(-2..=2);
        place(&mut tiles, pp.0 + dx, pp.1 + dy, Decor::Chest { opened: false }, w, &mut chests, &mut captives);
    }
    // captives with cages
    for _ in 0..2 + rng.gen_range(0..2) {
        let pp = path_points[rng.gen_range(1..path_points.len())];
        let x = (pp.0 + rng.gen_range(-3..=3)).clamp(2, w - 3);
        let y = (pp.1 + rng.gen_range(-3..=3)).clamp(2, h - 3);
        place(&mut tiles, x, y, Decor::Cage, w, &mut chests, &mut captives);
        place(&mut tiles, x, y + 1, Decor::EmeraldPile, w, &mut chests, &mut captives);
    }
    // fountain in mid path
    {
        let pp = path_points[path_points.len() / 2];
        place(&mut tiles, pp.0 + 2, pp.1, Decor::Fountain { used: false }, w, &mut chests, &mut captives);
    }
    // hero starting camp: campfire by the spawn (MCD intro camps, refs 17/24)
    place(&mut tiles, start.0 + 2, start.1 - 2, Decor::Campfire, w, &mut chests, &mut captives);
    // banners near arena
    for (dx, dy) in [(-4, 2), (4, 2)] {
        let x = arena.0 + dx;
        let y = arena.1 + dy;
        if open(&tiles, x, y) {
            let i = (y as u32 * w as u32 + x as u32) as usize;
            if tiles[i].decor.is_none() { tiles[i].decor = Some(Decor::Banner); }
        }
    }
    // illager camp before the arena: tents + campfire (refs 17/24/26)
    place(&mut tiles, arena.0 - 7, arena.1 - 3, Decor::Tent, w, &mut chests, &mut captives);
    place(&mut tiles, arena.0 - 9, arena.1 - 1, Decor::Campfire, w, &mut chests, &mut captives);
    place(&mut tiles, arena.0 + 7, arena.1 - 4, Decor::Tent, w, &mut chests, &mut captives);
    // occasional campfires along the path (waypoint camps)
    for pp in path_points.iter().step_by(6).skip(1) {
        let (x, y) = (pp.0 + 2, pp.1 - 2);
        if open(&tiles, x, y) {
            let i = (y as u32 * w as u32 + x as u32) as usize;
            if tiles[i].decor.is_none() { tiles[i].decor = Some(Decor::Campfire); }
        }
    }

    // ---- MCD props : rails de ponts, barils/caisses, lanternes, ruines ----
    let mut prop_at = |tiles: &mut Vec<Tile>, x: i32, y: i32, d: Decor| {
        if x <= 0 || y <= 0 || x >= w - 1 || y >= h - 1 { return; }
        let i = (y as u32 * w as u32 + x as u32) as usize;
        if tiles[i].wall == 0 && tiles[i].decor.is_none() {
            tiles[i].decor = Some(d);
        }
    };
    // fence rails on bridge sides (over the water)
    let mut fences = 0;
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let i = (y as u32 * w as u32 + x as u32) as usize;
            if tiles[i].wall != 0 || tiles[i].ground != Block::Planks { continue; }
            let nb_water = [(-1i32, 0), (1, 0), (0, -1), (0, 1)].iter().any(|(dx, dy)| {
                let xx = x + dx;
                let yy = y + dy;
                xx > 0 && yy > 0 && xx < w - 1 && yy < h - 1
                    && tiles[(yy as u32 * w as u32 + xx as u32) as usize].ground == Block::Water
            });
            if !nb_water { continue; }
            for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                let xx = x + dx;
                let yy = y + dy;
                if xx <= 0 || yy <= 0 || xx >= w - 1 || yy >= h - 1 { continue; }
                let j = (yy as u32 * w as u32 + xx as u32) as usize;
                if tiles[j].ground == Block::Water && tiles[j].wall == 0
                    && tiles[j].decor.is_none() && fences < 46 && rng.gen_bool(0.6)
                {
                    tiles[j].decor = Some(Decor::Fence);
                    fences += 1;
                }
            }
        }
    }
    // clutter clusters in rooms: barrels, crates, lanterns, ruins
    for pp in path_points.iter().skip(1) {
        if rng.gen_bool(0.5) {
            for _ in 0..rng.gen_range(1..=3) {
                let x = pp.0 + rng.gen_range(-3..=3);
                let y = pp.1 + rng.gen_range(-3..=3);
                let d = if rng.gen_bool(0.5) { Decor::Barrel } else { Decor::Crate };
                prop_at(&mut tiles, x, y, d);
            }
        }
        if rng.gen_bool(0.4) {
            let x = pp.0 + rng.gen_range(-2..=2);
            let y = pp.1 + rng.gen_range(-2..=2);
            prop_at(&mut tiles, x, y, Decor::Lantern);
        }
        if rng.gen_bool(0.3) {
            let x = pp.0 + rng.gen_range(-3..=3);
            let y = pp.1 + rng.gen_range(-3..=3);
            prop_at(&mut tiles, x, y, Decor::Ruin);
        }
    }
    // arena dressing: pillar ring + entrance braziers (ref_46)
    for k in 0..8 {
        let a = k as f32 / 8.0 * std::f32::consts::TAU;
        let x = arena.0 + (a.cos() * 7.5).round() as i32;
        let y = arena.1 + (a.sin() * 7.5).round() as i32;
        prop_at(&mut tiles, x, y, Decor::Pillar);
    }
    prop_at(&mut tiles, arena.0 - 3, arena.1 + 6, Decor::Brazier);
    prop_at(&mut tiles, arena.0 + 3, arena.1 + 6, Decor::Brazier);

    // ---- enemy spawn trigger groups along the path ----
    let mut groups = Vec::new();
    let table = super::missions::biome_enemies(mission.biome);
    let total_w: u32 = table.iter().map(|t| t.1).sum();
    let pick_kind = |rng: &mut StdRng| -> String {
        let mut roll = rng.gen_range(0..total_w);
        for (k, wt) in table {
            if roll < *wt { return k.to_string(); }
            roll -= *wt;
        }
        table[0].0.to_string()
    };
    for pp in path_points.iter().skip(2) {
        if rng.gen_bool(0.9) {
            // packs plus larges et plus fréquents (rééquilibrage difficulté)
            let count = 4 + (mission.threat / 5).min(5) + rng.gen_range(0..3);
            let mut kinds = Vec::new();
            for _ in 0..count { kinds.push(pick_kind(&mut rng)); }
            groups.push(SpawnGroup {
                pos: Vec2::new(pp.0 as f32 + 0.5, pp.1 as f32 + 0.5),
                radius: 8.0,
                kinds,
                count: count as u32,
                activated: false,
            });
        }
    }
    // group guarding the arena entrance
    if mission.boss.is_none() {
        let mut kinds = Vec::new();
        for _ in 0..(8 + mission.threat / 3) { kinds.push(pick_kind(&mut rng)); }
        let n = kinds.len() as u32;
        groups.push(SpawnGroup {
            pos: Vec2::new(arena.0 as f32 + 0.5, (arena.1 + 7) as f32),
            radius: 8.0,
            kinds,
            count: n,
            activated: false,
        });
    }

    Level {
        w: w as u32,
        h: h as u32,
        tiles,
        spawns,
        boss_spawn,
        portal,
        groups,
        secret_tiles,
        secret_opened: false,
        has_rune,
        lever: lever_pos,
        biome: mission.biome,
        fog: (pal.fog, pal.fog_start, pal.fog_end),
        captives,
        chests,
    }
}

impl Level {
    /// Open the secret corridor (lever pulled).
    pub fn open_secret(&mut self) {
        if self.secret_opened { return; }
        self.secret_opened = true;
        for &i in &self.secret_tiles {
            self.tiles[i].wall = 0;
        }
    }

    pub fn secret_room_bounds(&self) -> RectI {
        // unused for v1 minimap; kept for future map rendering
        RectI { x: 0, y: 0, w: 0, h: 0 }
    }
}
