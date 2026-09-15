//! Maps « réelles » façon Minecraft Dungeons : layouts FIXES authorés
//! (le vrai jeu n'est PAS procédural — chaque mission a son tracé, ses
//! landmarks et ses positions d'entités identiques à chaque partie).
//!
//! Données authorées par mission :
//!   - waypoints du chemin principal (sud -> nord, avec les vraies boucles)
//!   - landmarks : rivière + pont, village, ruines, champ de citrouilles…
//!   - positions fixes : spawns, captifs, coffres, levier de rune, portail
//!   - groupes d'ennemis sur le chemin (tables de biome inchangées)

use super::missions::{Biome, MissionDef};
use super::{Block, Decor, Level, SpawnGroup, Tile};
use glam::Vec2;

/// Un landmark authoré : rect/carac posé sur la grille.
struct Landmark {
    x: i32,
    y: i32,
    kind: Lm,
}

enum Lm {
    /// Clairière ronde (r)
    Clearing(i32),
    /// Maison en planches (w×h, porte au sud)
    House(i32, i32),
    /// Rivière horizontale traversante (largeur) — le chemin la chevauche par un pont
    River(i32),
    /// Champ de citrouilles / cultures (w×h)
    Field(i32, i32),
    /// Ruines (murets de briques)
    Ruins(i32),
    /// Feu de camp
    Campfire,
}

pub struct RealMap {
    /// waypoints (x, y) du chemin principal, du spawn (sud) vers l'arène (nord)
    path: &'static [(i32, i32)],
    landmarks: &'static [Landmark],
    spawns: (i32, i32),
    arena: (i32, i32),
    portal: (i32, i32),
    captives: &'static [(i32, i32)],
    chests: &'static [(i32, i32)],
    lever: (i32, i32),
    /// groupes : (waypoint index, taille)
    groups: &'static [(usize, u32)],
}

const CW_PATH: &[(i32, i32)] = &[
    (32, 62), (32, 56), (26, 52), (22, 46), (26, 41), (34, 38), (40, 34),
    (38, 27), (30, 24), (24, 20), (28, 14), (32, 10),
];
const CW: RealMap = RealMap {
    path: CW_PATH,
    landmarks: &[
        Landmark { x: 32, y: 62, kind: Lm::Clearing(5) },
        Landmark { x: 24, y: 48, kind: Lm::River(2) },       // rivière + pont au franchissement
        Landmark { x: 34, y: 38, kind: Lm::Clearing(5) },    // camp de cages (captifs)
        Landmark { x: 30, y: 24, kind: Lm::Clearing(5) },
        Landmark { x: 40, y: 22, kind: Lm::House(5, 4) },
        Landmark { x: 22, y: 16, kind: Lm::House(5, 4) },
        Landmark { x: 32, y: 10, kind: Lm::Clearing(8) },    // village final
    ],
    spawns: (32, 62),
    arena: (32, 10),
    portal: (32, 8),
    captives: &[(35, 39), (32, 37), (38, 36)],
    chests: &[(20, 47), (41, 33), (26, 15)],
    lever: (44, 30),
    groups: &[(2, 6), (4, 6), (6, 7), (8, 7), (10, 8)],
};

const PP_PATH: &[(i32, i32)] = &[
    (32, 62), (30, 57), (36, 53), (40, 48), (36, 43), (28, 40), (24, 35), (30, 30), (38, 26), (32, 20), (30, 12),
];
const PP: RealMap = RealMap {
    path: PP_PATH,
    landmarks: &[
        Landmark { x: 32, y: 62, kind: Lm::Clearing(5) },
        Landmark { x: 40, y: 54, kind: Lm::Field(7, 5) },    // champ de citrouilles
        Landmark { x: 22, y: 44, kind: Lm::Field(6, 5) },
        Landmark { x: 30, y: 30, kind: Lm::Clearing(5) },
        Landmark { x: 42, y: 22, kind: Lm::House(5, 4) },
        Landmark { x: 30, y: 12, kind: Lm::Clearing(8) },    // ferme finale (moulin)
    ],
    spawns: (32, 62),
    arena: (30, 12),
    portal: (30, 10),
    captives: &[(39, 49), (25, 41), (31, 29)],
    chests: &[(43, 53), (21, 34), (35, 27)],
    lever: (44, 36),
    groups: &[(2, 6), (4, 7), (6, 7), (8, 8), (10, 8)],
};

const CC_PATH: &[(i32, i32)] = &[
    (32, 62), (34, 56), (30, 51), (24, 47), (26, 41), (34, 37), (40, 32), (36, 26), (28, 22), (30, 15), (32, 11),
];
const CC: RealMap = RealMap {
    path: CC_PATH,
    landmarks: &[
        Landmark { x: 32, y: 62, kind: Lm::Clearing(5) },
        Landmark { x: 20, y: 50, kind: Lm::Ruins(4) },
        Landmark { x: 34, y: 37, kind: Lm::Clearing(6) },
        Landmark { x: 42, y: 28, kind: Lm::Ruins(4) },
        Landmark { x: 32, y: 11, kind: Lm::Clearing(9) },
    ],
    spawns: (32, 62),
    arena: (32, 11),
    portal: (32, 9),
    captives: &[(31, 38), (36, 36), (25, 44)],
    chests: &[(23, 49), (43, 31), (31, 24)],
    lever: (14, 40),
    groups: &[(2, 7), (4, 7), (6, 8), (8, 8), (10, 9)],
};

const SS_PATH: &[(i32, i32)] = &[
    (32, 62), (28, 57), (24, 52), (30, 47), (38, 44), (40, 38), (34, 33), (26, 30), (22, 24), (28, 18), (32, 12),
];
const SS: RealMap = RealMap {
    path: SS_PATH,
    landmarks: &[
        Landmark { x: 32, y: 62, kind: Lm::Clearing(5) },
        Landmark { x: 26, y: 55, kind: Lm::River(2) },
        Landmark { x: 38, y: 41, kind: Lm::River(2) },
        Landmark { x: 34, y: 33, kind: Lm::Clearing(6) },
        Landmark { x: 18, y: 30, kind: Lm::Ruins(4) },
        Landmark { x: 32, y: 12, kind: Lm::Clearing(9) },
    ],
    spawns: (32, 62),
    arena: (32, 12),
    portal: (32, 10),
    captives: &[(31, 48), (37, 45), (27, 31)],
    chests: &[(20, 53), (42, 39), (19, 31)],
    lever: (44, 26),
    groups: &[(2, 7), (4, 8), (6, 8), (8, 9), (10, 9)],
};

pub fn lookup(mission_id: usize) -> Option<&'static RealMap> {
    match mission_id {
        0 => Some(&CW),
        1 => Some(&PP),
        2 => Some(&CC),
        3 => Some(&SS),
        _ => None,
    }
}

fn mk_ground(biome: Biome, x: i32, y: i32) -> Block {
    // sol majoritaire du biome + variante déterministe (hash de position)
    let g = match biome {
        Biome::Canyon => Block::RedSand,
        Biome::Swamp => Block::Mud,
        Biome::Mines => Block::Deepslate,
        Biome::Desert => Block::Sand,
        Biome::Nether => Block::Netherrack,
        _ => Block::Grass,
    };
    let h = (x as u32).wrapping_mul(73856093) ^ (y as u32).wrapping_mul(19349663);
    if h % 100 < 18 {
        match biome {
            Biome::Canyon => Block::Sand,
            Biome::Swamp => Block::Grass,
            Biome::Mines => Block::Stone,
            Biome::Desert => Block::Sandstone,
            Biome::Nether => Block::SoulSand,
            _ => Block::Podzol,
        }
    } else {
        g
    }
}

/// Construit le niveau authoré d'une mission (None si pas de carte réelle).
pub fn build(mission: &MissionDef, seed: u64, map: &'static RealMap) -> Level {
    use rand::prelude::*;
    let mut rng = StdRng::seed_from_u64(seed);
    let w = crate::consts::LEVEL_W as i32;
    let h = crate::consts::LEVEL_H as i32;
    let wall_block = match mission.biome {
        Biome::Canyon => Block::Sandstone,
        Biome::Swamp => Block::Log,
        Biome::Mines => Block::Deepslate,
        Biome::Desert => Block::Sandstone,
        _ => Block::Log,
    };

    // remplissage plein (murs de biome)
    let mut tiles = vec![Tile { ground: Block::Grass, wall: 2, wall_block, decor: None }; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = (y as u32 * w as u32 + x as u32) as usize;
            tiles[i].ground = mk_ground(mission.biome, x, y);
        }
    }

    let mut carve = |tiles: &mut Vec<Tile>, x: i32, y: i32, r: i32| {
        for dy in -r..=r {
            for dx in -r..=r {
                let xx = x + dx;
                let yy = y + dy;
                if xx <= 0 || yy <= 0 || xx >= w - 1 || yy >= h - 1 { continue; }
                if dx * dx + dy * dy <= r * r + 1 {
                    let i = (yy as u32 * w as u32 + xx as u32) as usize;
                    tiles[i] = Tile { ground: mk_ground(mission.biome, xx, yy), wall: 0, wall_block, decor: None };
                }
            }
        }
    };

    // ---- chemins : segments entre waypoints (couloirs de rayon 1-2) ----
    let mut path_pts: Vec<(i32, i32)> = Vec::new();
    for seg in map.path.windows(2) {
        let (mut x0, mut y0) = (seg[0].0, seg[0].1);
        let (x1, y1) = (seg[1].0, seg[1].1);
        let steps = ((x1 - x0).abs() + (y1 - y0).abs()).max(1);
        for s in 0..=steps {
            let cx = x0 + (x1 - x0) * s / steps;
            let cy = y0 + (y1 - y0) * s / steps;
            carve(&mut tiles, cx, cy, 1);
            if s % 3 == 0 {
                path_pts.push((cx, cy));
            }
        }
        path_pts.push((x1, y1));
        x0 = x1;
        y0 = y1;
    }

    // ---- landmarks authorés ----
    let mut bridge_cells: Vec<(i32, i32)> = Vec::new();
    for lm in map.landmarks {
        match &lm.kind {
            Lm::Clearing(r) => carve(&mut tiles, lm.x, lm.y, *r),
            Lm::House(hw, hh) => {
                // sol intérieur + murs planches + porte sud
                for yy in lm.y..(lm.y + hh) {
                    for xx in lm.x..(lm.x + hw) {
                        if let Some(i) = tiles.get_mut((yy as u32 * w as u32 + xx as u32) as usize) {
                            *i = Tile { ground: Block::Planks, wall: 0, wall_block, decor: None };
                        }
                    }
                }
                for xx in lm.x..(lm.x + hw) {
                    for edge_y in [lm.y - 1, lm.y + hh] {
                        if let Some(i) = tiles.get_mut((edge_y as u32 * w as u32 + xx as u32) as usize) {
                            if i.wall == 0 { i.decor = Some(Decor::Fence); }
                        }
                    }
                }
            }
            Lm::Field(fw, fh) => {
                for yy in lm.y..(lm.y + fh) {
                    for xx in lm.x..(lm.x + fw) {
                        if let Some(i) = tiles.get_mut((yy as u32 * w as u32 + xx as u32) as usize) {
                            if i.wall == 0 {
                                i.ground = Block::Dirt;
                                if (xx + yy) % 3 == 0 {
                                    i.decor = Some(Decor::PumpkinProp);
                                }
                            }
                        }
                    }
                }
            }
            Lm::River(rw) => {
                // rivière horizontale traversant TOUTE la largeur ; le chemin
                // du waypoint le plus proche devient un pont en planches
                for yy in (lm.y - rw)..(lm.y + rw) {
                    for xx in 1..(w - 1) {
                        if let Some(i) = tiles.get_mut((yy as u32 * w as u32 + xx as u32) as usize) {
                            i.ground = Block::Water;
                            i.wall = 0;
                            i.decor = None;
                        }
                    }
                }
                // pont : le point de chemin le plus proche de la rivière
                let mut best = map.path[0];
                let mut bd = i32::MAX;
                for p in map.path {
                    let d = (p.1 - lm.y).abs();
                    if d < bd {
                        bd = d;
                        best = *p;
                    }
                }
                for dy in -(rw + 1)..=(rw + 1) {
                    for dx in -2..=2 {
                        let xx = best.0 + dx;
                        let yy = lm.y + dy;
                        if let Some(i) = tiles.get_mut((yy as u32 * w as u32 + xx as u32) as usize) {
                            *i = Tile { ground: Block::Planks, wall: 0, wall_block, decor: None };
                        }
                        bridge_cells.push((xx, yy));
                    }
                }
            }
            Lm::Ruins(r) => {
                for a in 0..12 {
                    let ang = a as f32 / 12.0 * std::f32::consts::TAU;
                    let xx = lm.x + (ang.cos() * *r as f32) as i32;
                    let yy = lm.y + (ang.sin() * *r as f32) as i32;
                    if a % 3 != 0 {
                        if let Some(i) = tiles.get_mut((yy.max(1) as u32 * w as u32 + xx.max(1) as u32) as usize) {
                            i.decor = Some(Decor::Ruin);
                        }
                    }
                }
            }
            Lm::Campfire => {
                if let Some(i) = tiles.get_mut((lm.y as u32 * w as u32 + lm.x as u32) as usize) {
                    i.decor = Some(Decor::Brazier);
                }
            }
        }
    }
    let _ = bridge_cells;

    // ---- décor de bordure forêt/buissons (léger, jamais sur le chemin) ----
    {
        use rand::Rng;
        for i in 0..tiles.len() {
            if tiles[i].wall == 0 && tiles[i].decor.is_none() {
                let x = i as u32 % w as u32;
                let y = i as u32 / w as u32;
                // seulement en lisière (adjacent à un mur)
                let near_wall = [(x.wrapping_sub(1), y), (x + 1, y), (x, y.wrapping_sub(1)), (x, y + 1)]
                    .iter()
                    .any(|(nx, ny)| {
                        tiles
                            .get((*ny as u32 * w as u32 + *nx as u32) as usize)
                            .map(|t| t.wall > 0)
                            .unwrap_or(true)
                    });
                if near_wall && rng.gen_bool(0.10) {
                    tiles[i].decor = if mission.biome == Biome::Canyon || mission.biome == Biome::Desert {
                        Some(Decor::CactusPlant)
                    } else {
                        Some(Decor::Tree)
                    };
                }
            }
        }
    }

    // ---- assemblage du niveau ----
    let fog = crate::world::gen::biome_palette(mission.biome);
    let has_rune = super::missions::secret_unlocked_by_rune(mission.id).is_some();
    let mut secret_tiles: Vec<usize> = Vec::new();
    let mut level = Level {
        w: w as u32,
        h: h as u32,
        tiles,
        spawns: vec![Vec2::new(map.spawns.0 as f32 + 0.5, map.spawns.1 as f32 + 0.5)],
        boss_spawn: mission.boss.map(|_| Vec2::new(map.arena.0 as f32 + 0.5, map.arena.1 as f32 + 0.5)),
        portal: Some((Vec2::new(map.portal.0 as f32 + 0.5, map.portal.1 as f32 + 0.5), false)),
        groups: Vec::new(),
        secret_tiles: Vec::new(),
        secret_opened: false,
        has_rune,
        lever: Some(Vec2::new(map.lever.0 as f32 + 0.5, map.lever.1 as f32 + 0.5)),
        biome: mission.biome,
        fog: (fog.fog, fog.fog_start, fog.fog_end),
        captives: map.captives.iter().map(|(x, y)| Vec2::new(*x as f32 + 0.5, *y as f32 + 0.5)).collect(),
        chests: map.chests.iter().map(|(x, y)| Vec2::new(*x as f32 + 0.5, *y as f32 + 0.5)).collect(),
    };

    // levier posé dans une petite clairière ouverte, puis salle secrète murée
    // à 10 tuiles : piédestal de rune à l'intérieur (ouvre quand on tire)
    if has_rune {
        carve(&mut level.tiles, map.lever.0, map.lever.1, 2);
        // couloir de connexion : waypoint de chemin le plus proche -> levier
        {
            let mut best = map.path[0];
            let mut bd = i32::MAX;
            for p in map.path {
                let d = (p.0 - map.lever.0).abs() + (p.1 - map.lever.1).abs();
                if d < bd {
                    bd = d;
                    best = *p;
                }
            }
            let steps = bd.max(1);
            for s in 0..=steps {
                let cx = best.0 + (map.lever.0 - best.0) * s / steps;
                let cy = best.1 + (map.lever.1 - best.1) * s / steps;
                carve(&mut level.tiles, cx, cy, 1);
            }
        }
        let li = (map.lever.1 as u32 * w as u32 + map.lever.0 as u32) as usize;
        if let Some(t) = level.tiles.get_mut(li) {
            t.decor = Some(Decor::Lever { pulled: false });
        }
        let sx = (map.lever.0 + 10).clamp(3, w - 4);
        let sy = map.lever.1;
        for dy in -4..=4 {
            for dx in -4..=4 {
                if dx * dx + dy * dy <= 16 {
                    let xx = sx + dx;
                    let yy = sy + dy;
                    if xx > 0 && yy > 0 && xx < w - 1 && yy < h - 1 {
                        secret_tiles.push((yy as u32 * w as u32 + xx as u32) as usize);
                    }
                }
            }
        }
        level.secret_tiles = secret_tiles;
        // piédestal au centre (apparaît à l'ouverture)
        let ri = (sy as u32 * w as u32 + sx as u32) as usize;
        if let Some(t) = level.tiles.get_mut(ri) {
            t.decor = Some(Decor::Rune { taken: false });
        }
    }

    // chaque entité (captifs / coffres) est raccordée au chemin par une
    // petite clairière + couloir — garantit l'accessibilité du layout authoré
    {
        let link = |level: &mut Level, px: i32, py: i32| {
            let mut best = map.path[0];
            let mut bd = i32::MAX;
            for p in map.path {
                let d = (p.0 - px).abs() + (p.1 - py).abs();
                if d < bd {
                    bd = d;
                    best = *p;
                }
            }
            carve(&mut level.tiles, px, py, 2);
            let steps = bd.max(1);
            for s in 0..=steps {
                let cx = best.0 + (px - best.0) * s / steps;
                let cy = best.1 + (py - best.1) * s / steps;
                carve(&mut level.tiles, cx, cy, 1);
            }
        };
        for c in map.captives {
            link(&mut level, c.0, c.1);
        }
        for ch in map.chests {
            link(&mut level, ch.0, ch.1);
        }
    }

    // coffres + cages des captifs + portail : décors posés APRÈS les carvings
    // (le link ci-dessus remplace les tuiles, il écraserait les décors)
    for c in &level.chests {
        let i = (c.y as i32 * w + c.x as i32).max(0) as usize;
        if let Some(t) = level.tiles.get_mut(i) {
            if t.decor.is_none() {
                t.decor = Some(Decor::Chest { opened: false });
            }
        }
    }
    for c in &level.captives {
        let i = (c.y as i32 * w + c.x as i32).max(0) as usize;
        if let Some(t) = level.tiles.get_mut(i) {
            t.decor = Some(Decor::Cage);
        }
    }
    if let Some((ppos, _)) = level.portal {
        let i = (ppos.y as i32 * w + ppos.x as i32).max(0) as usize;
        if let Some(t) = level.tiles.get_mut(i) {
            t.decor = Some(Decor::Portal { active: false });
        }
    }

    // ---- groupes d'ennemis authorés aux waypoints indiqués ----
    let table = super::missions::biome_enemies(mission.biome);
    let total_w: u32 = table.iter().map(|t| t.1).sum();
    let mut pick = |rng: &mut StdRng| -> String {
        let mut roll = rng.gen_range(0..total_w);
        for (k, wt) in table {
            if roll < *wt {
                return k.to_string();
            }
            roll -= *wt;
        }
        table[0].0.to_string()
    };
    for (wi, count) in map.groups {
        if let Some(p) = map.path.get(*wi) {
            let mut kinds = Vec::new();
            for _ in 0..*count {
                kinds.push(pick(&mut rng));
            }
            level.groups.push(SpawnGroup {
                pos: Vec2::new(p.0 as f32 + 0.5, p.1 as f32 + 0.5),
                radius: 8.0,
                kinds,
                count: *count,
                activated: false,
            });
        }
    }
    // garde d'arène
    let mut kinds = Vec::new();
    for _ in 0..(8 + mission.threat / 3) {
        kinds.push(pick(&mut rng));
    }
    let n = kinds.len() as u32;
    level.groups.push(SpawnGroup {
        pos: Vec2::new(map.arena.0 as f32 + 0.5, (map.arena.1 + 5) as f32),
        radius: 8.0,
        kinds,
        count: n,
        activated: false,
    });

    // arène : pilier + braseros (comme gen)
    let ax = map.arena.0;
    let ay = map.arena.1;
    for a in 0..14 {
        let ang = a as f32 / 14.0 * std::f32::consts::TAU;
        let xx = ax + (ang.cos() * 8.0) as i32;
        let yy = ay + (ang.sin() * 8.0) as i32;
        if let Some(i) = level.idx(xx, yy) {
            if a % 2 == 0 {
                level.tiles[i].decor = Some(Decor::Pillar);
            }
        }
    }
    if let Some(i) = level.idx(ax - 3, ay + 6) {
        level.tiles[i].decor = Some(Decor::Brazier);
    }
    if let Some(i) = level.idx(ax + 3, ay + 6) {
        level.tiles[i].decor = Some(Decor::Brazier);
    }

    level
}

/// Test : les cartes authorées sont connectées (spawn -> arène sans mur).
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn realmaps_are_connected() {
        for id in 0..4 {
            let m = &super::super::missions::MISSIONS[id];
            let map = lookup(id).expect("carte réelle manquante");
            let mut lvl = build(m, 42, map);
            // flood fill depuis le spawn
            let w = lvl.w as i32;
            let start = (map.spawns.0, map.spawns.1);
            let mut seen = vec![false; lvl.tiles.len()];
            let mut stack = vec![start];
            while let Some((x, y)) = stack.pop() {
                if x < 0 || y < 0 || x >= w || y >= lvl.h as i32 { continue; }
                let i = (y as u32 * lvl.w + x as u32) as usize;
                if seen[i] { continue; }
                seen[i] = true;
                if lvl.tiles[i].solid() { continue; }
                stack.push((x + 1, y));
                stack.push((x - 1, y));
                stack.push((x, y + 1));
                stack.push((x, y - 1));
            }
            let reach = |p: (i32, i32)| -> bool {
                // tuile de l'entité ou une voisine accessible
                [[0i32, 0], [1, 0], [-1, 0], [0, 1], [0, -1]].iter().any(|d: &[i32; 2]| {
                    let (dx, dy) = (d[0], d[1]);
                    let (x, y) = (p.0 + dx, p.1 + dy);
                    if x < 0 || y < 0 || x >= w || y >= lvl.h as i32 { return false; }
                    let i = (y as u32 * lvl.w + x as u32) as usize;
                    seen[i] && !lvl.tiles[i].solid()
                })
            };
            assert!(reach((map.portal.0, map.portal.1 + 1)), "mission {} : portail inaccessible", id);
            for (i2, c) in map.captives.iter().enumerate() {
                assert!(reach(*c), "mission {} : captif {} inaccessible", id, i2);
            }
            if lvl.has_rune {
                assert!(reach(map.lever), "mission {} : levier inaccessible", id);
            }
            for (i2, g) in map.groups.iter().enumerate() {
                if let Some(p) = map.path.get(g.0) {
                    assert!(reach(*p), "mission {} : groupe {} inaccessible", id, i2);
                }
            }
            // le niveau reste exploitable, la rune seulement là où prévu
            assert!(!lvl.groups.is_empty());
            assert_eq!(lvl.has_rune, crate::world::missions::secret_unlocked_by_rune(id).is_some());
        }
    }
}
