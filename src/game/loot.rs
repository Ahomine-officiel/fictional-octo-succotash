//! Pickups (emeralds, arrows, hearts, items) and world interactions:
//! chests, captives, runes, levers, fountains, portals.

use super::combat::PickupKind;
use super::{Game, RunState};
use crate::consts;
use crate::world::Decor;
use hecs::Entity;
use glam::Vec2;
use glam::Vec3;
use rand::Rng;

pub fn update_pickups(game: &mut Game, dt: f32) {
    // snapshot alive player positions
    let players: Vec<(usize, Vec2)> = game
        .players
        .iter()
        .enumerate()
        .filter(|(_, p)| p.downed_t.is_none())
        .map(|(i, p)| (i, p.pos))
        .collect();
    let nearest = |pos: Vec2| -> Option<(usize, f32)> {
        players
            .iter()
            .map(|(i, p)| (*i, p.distance(pos)))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    };

    // move + mark collected
    let mut collected: Vec<PickupKind> = Vec::new();
    let pickups = std::mem::take(&mut game.pickups);
    for mut pk in pickups {
        pk.t += dt;
        if let Some((_, dist)) = nearest(pk.pos) {
            if dist < 2.5 {
                if let Some((pi, _)) = nearest(pk.pos) {
                    let dir = (players[pi].1 - pk.pos).normalize_or_zero();
                    pk.pos += dir * dt * (6.0 - dist * 1.5).max(2.0);
                }
            }
            if dist < 0.7 {
                collected.push(pk.kind);
                continue;
            }
        }
        game.pickups.push(pk);
    }

    // apply collection effects
    for kind in collected {
        match kind {
            PickupKind::Emerald(n) => {
                game.emeralds += n;
            }
            PickupKind::Arrows(n) => {
                if let Some(p) = game.players.first_mut() {
                    p.arrows = (p.arrows + n).min(999);
                }
            }
            PickupKind::Heart => {
                let pi = nearest(players.first().map(|p| p.1).unwrap_or(Vec2::ZERO)).map(|x| x.0).unwrap_or(0);
                super::combat::heal_player(game, pi, 15.0);
            }
            PickupKind::Item(item) => {
                let mut given = false;
                for p in game.players.iter_mut() {
                    if p.downed_t.is_none() && p.inventory.push_stash(item.clone()) {
                        given = true;
                        break;
                    }
                }
                if !given {
                    if let Some(p) = game.players.first_mut() {
                        if p.inventory.push_stash(item.clone()) {
                            given = true;
                        }
                    }
                }
                if !given {
                    game.emeralds += 5; // inventory full -> auto salvage
                }
            }
        }
    }
}

/// Interact with the nearest interactable (chest, captive, rune, lever,
/// fountain, portal) within INTERACT_RANGE.
pub fn try_interact(game: &mut Game, pidx: usize) {
    let Some(p) = game.players.get(pidx) else { return };
    if p.downed_t.is_some() {
        return;
    }
    let pos = p.pos;
    let r = consts::INTERACT_RANGE;

    // chests (decor lookup near player)
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            let tx = pos.x.floor() as i32 + dx;
            let ty = pos.y.floor() as i32 + dy;
            let Some(i) = game.level.idx(tx, ty) else { continue };
            let center = Vec3::new(tx as f32 + 0.5, 0.5, ty as f32 + 0.5);
            let dist2 = (pos - glam::Vec2::new(center.x, center.z)).length_squared();
            if dist2 > r * r {
                continue;
            }
            match game.level.tiles[i].decor {
                Some(Decor::Chest { opened: false }) => {
                    game.level.tiles[i].decor = Some(Decor::Chest { opened: true });
                    game.opened.insert((tx, ty));
                    let mut rng = rand::thread_rng();
                    let emeralds = rng.gen_range(consts::CHEST_EMERALDS.0..=consts::CHEST_EMERALDS.1);
                    game.pickups.push(super::combat::Pickup {
                        pos: glam::Vec2::new(center.x, center.z),
                        kind: PickupKind::Emerald(emeralds),
                        t: 0.0,
                    });
                    let power = (game.total_threat() + rng.gen_range(0..3)).max(1);
                    let n_items = 1 + rng.gen_range(0..=2);
                    for k in 0..n_items {
                        let item = super::items::gen_item(&mut rng, power + k, 0.05);
                        game.items_found.push(item.clone());
                        game.pickups.push(super::combat::Pickup {
                            pos: glam::Vec2::new(center.x + 0.3 * k as f32, center.z + 0.2),
                            kind: PickupKind::Item(item),
                            t: 0.0,
                        });
                    }
                    if rng.gen_bool(0.4) {
                        game.pickups.push(super::combat::Pickup {
                            pos: glam::Vec2::new(center.x - 0.4, center.z),
                            kind: PickupKind::Arrows(rng.gen_range(5..=12)),
                            t: 0.0,
                        });
                    }
                    game.particles.spawn_burst(center, 16, glam::Vec4::new(0.3, 1.0, 0.5, 1.0), 2.5);
                    return;
                }
                Some(Decor::Lever { pulled: false }) => {
                    game.level.tiles[i].decor = Some(Decor::Lever { pulled: true });
                    game.level.open_secret();
                    game.particles.spawn_burst(center, 20, glam::Vec4::new(0.4, 0.9, 0.7, 1.0), 3.0);
                    game.floaters.push(super::combat::Floater {
                        pos: center,
                        text: "Passage secret ouvert !".to_string(),
                        color: glam::Vec4::new(0.4, 1.0, 0.7, 1.0),
                        life: 2.0,
                    });
                    return;
                }
                Some(Decor::Rune { taken: false }) => {
                    game.level.tiles[i].decor = Some(Decor::Rune { taken: true });
                    game.rune_found = true;
                    for p in game.players.iter_mut() {
                        p.add_xp(15);
                    }
                    game.particles.spawn_burst(center, 24, glam::Vec4::new(0.3, 1.0, 0.8, 1.0), 3.5);
                    game.floaters.push(super::combat::Floater {
                        pos: center,
                        text: "Rune trouvée !".to_string(),
                        color: glam::Vec4::new(0.3, 1.0, 0.8, 1.0),
                        life: 2.0,
                    });
                    return;
                }
                Some(Decor::Fountain { used: false }) => {
                    let p = &mut game.players[pidx];
                    if p.hp < p.max_hp() {
                        p.hp = p.max_hp();
                        game.level.tiles[i].decor = Some(Decor::Fountain { used: true });
                        game.particles.spawn_burst(center, 18, glam::Vec4::new(0.4, 1.0, 1.0, 1.0), 2.5);
                    }
                    return;
                }
                Some(Decor::Portal { active: true }) => {
                    game.state = RunState::Victory;
                    return;
                }
                _ => {}
            }
        }
    }

    // captives: free nearest captive entity
    let mut best: Option<(hecs::Entity, f32)> = None;
    for (e, ep, en) in game
        .world
        .query::<(Entity, &super::Pos, &super::enemy::Enemy)>()
        .iter()
    {
        if !en.captive {
            continue;
        }
        let d = ep.0.distance(pos);
        if d < r && best.map(|(_, bd)| d < bd).unwrap_or(true) {
            best = Some((e, d));
        }
    }
    if let Some((e, _)) = best {
        let cpos = game.world.get::<&super::Pos>(e).map(|p| p.0).unwrap_or(pos);
        let _ = game.world.despawn(e);
        game.captives_rescued += 1;
        for p in game.players.iter_mut() {
            p.add_xp(10);
        }
        game.emeralds += 10;
        game.particles.spawn_burst(Vec3::new(cpos.x, 1.0, cpos.y), 20, glam::Vec4::new(1.0, 0.9, 0.4, 1.0), 3.0);
        game.floaters.push(super::combat::Floater {
            pos: Vec3::new(cpos.x, 1.6, cpos.y),
            text: "Villageois libéré !".to_string(),
            color: glam::Vec4::new(1.0, 0.9, 0.4, 1.0),
            life: 2.0,
        });
    }
}
