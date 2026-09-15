//! Combat systems: projectiles, damage pipeline, status effects, particles,
//! pickups, floating damage numbers, artifact effects and zones.

use super::enemy;
use super::items::{self, ArtifactKind, ItemClass};
use super::{Game, Health, HitFlash, Pos, RunState};
use crate::consts;
use crate::gfx::BillboardInstance;
use glam::{Vec2, Vec3, Vec4};
use hecs::Entity;
use rand::Rng;

#[derive(Clone, Copy, PartialEq)]
pub enum Team {
    Player,
    Enemy,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ProjKind {
    Arrow,
    Magic,
    Fire,
    Slime,
    Firework,
    Seeds,
    /// Iceologer / Wretched Wraith frost shard — chills the target.
    Ice,
}

pub struct Projectile {
    pub pos: Vec2,
    pub vel: Vec2,
    pub team: Team,
    pub dmg: f32,
    pub kind: ProjKind,
    pub life: f32,
    pub radius: f32,
    pub effects: Vec<Status>,
    pub explode: Option<(f32, f32)>, // (radius, dmg)
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum StatusKind {
    Poison,
    Burn,
    Slow,
}

#[derive(Clone, Copy)]
pub struct Status {
    pub kind: StatusKind,
    pub dps: f32,
    pub t: f32,
    pub slow_mult: f32,
}

impl Status {
    pub fn new(kind: StatusKind, dps: f32, t: f32) -> Status {
        Status { kind, dps, t, slow_mult: if kind == StatusKind::Slow { 0.45 } else { 1.0 } }
    }
}

#[derive(Clone)]
pub struct Zone {
    pub pos: Vec2,
    pub radius: f32,
    pub dps: f32,
    pub slow: bool,
    pub t: f32,
    pub team: Team,
    pub color: Vec4,
}

pub struct Particle {
    pub pos: Vec3,
    pub vel: Vec3,
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
    pub color: Vec4,
    pub gravity: f32,
}

pub struct ParticleSystem {
    pub parts: Vec<Particle>,
    rng: rand::rngs::StdRng,
    ambient_acc: f32,
    fire_acc: f32,
}

impl ParticleSystem {
    pub fn new() -> ParticleSystem {
        use rand::SeedableRng;
        ParticleSystem { parts: Vec::with_capacity(2048), rng: rand::rngs::StdRng::seed_from_u64(1234), ambient_acc: 0.0, fire_acc: 0.0 }
    }

    pub fn spawn(&mut self, p: Particle) {
        if self.parts.len() < 3000 {
            self.parts.push(p);
        }
    }

    pub fn spawn_burst(&mut self, pos: Vec3, n: usize, color: Vec4, speed: f32) {
        for _ in 0..n {
            let a = self.rng.gen_range(0.0..std::f32::consts::TAU);
            let s = speed * self.rng.gen_range(0.3..1.0);
            let up = self.rng.gen_range(0.5..3.0);
            let life = self.rng.gen_range(0.3..0.8);
            let size = self.rng.gen_range(0.1..0.3);
            self.spawn(Particle {
                pos,
                vel: Vec3::new(a.cos() * s, up, a.sin() * s),
                life,
                max_life: 0.8,
                size,
                color,
                gravity: 6.0,
            });
        }
    }

    /// Feu de camp de la page titre : braises montantes + volutes de fumée
    /// (comme la capture du vrai menu MCD — étincelles qui grimpent dans la nuit).
    pub fn spawn_fire(&mut self, pos: Vec2, dt: f32) {
        self.fire_acc += dt * 46.0;
        while self.fire_acc >= 1.0 {
            self.fire_acc -= 1.0;
            let a = self.rng.gen_range(0.0..std::f32::consts::TAU);
            let r = self.rng.gen_range(0.0..0.28);
            if self.rng.gen_bool(0.16) {
                // fumée lente
                self.spawn(Particle {
                    pos: Vec3::new(pos.x + a.cos() * r, 0.8, pos.y + a.sin() * r),
                    vel: Vec3::new(a.cos() * 0.15, 0.7, a.sin() * 0.15),
                    life: 1.8,
                    max_life: 1.8,
                    size: 0.26,
                    color: Vec4::new(0.3, 0.3, 0.33, 0.3),
                    gravity: -0.5,
                });
            } else {
                // braise : monte en accélérant (gravité négative), vire au jaune
                let g = self.rng.gen_range(0.35..0.85);
                let up = self.rng.gen_range(1.0..2.4);
                let life = self.rng.gen_range(0.5..1.1);
                let size = self.rng.gen_range(0.05..0.12);
                self.spawn(Particle {
                    pos: Vec3::new(pos.x + a.cos() * r, 0.4, pos.y + a.sin() * r),
                    vel: Vec3::new(a.cos() * 0.4, up, a.sin() * 0.4),
                    life,
                    max_life: 1.1,
                    size,
                    color: Vec4::new(1.0, 0.35 + g * 0.5, 0.1, 0.95),
                    gravity: -1.4,
                });
            }
        }
    }

    pub fn spawn_ring(&mut self, pos: Vec3, radius: f32, n: usize, color: Vec4) {
        for i in 0..n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            self.spawn(Particle {
                pos: pos + Vec3::new(a.cos() * radius, 0.15, a.sin() * radius),
                vel: Vec3::new(a.cos() * 1.5, 0.6, a.sin() * 1.5),
                life: 0.5,
                max_life: 0.5,
                size: 0.18,
                color,
                gravity: 0.0,
            });
        }
    }

    /// MCD melee swing arc — bright crescent sweeping in front of the attacker
    /// (signature VFX, reference ref_46). `strong` = gold sparks on heavy hits.
    pub fn spawn_slash(&mut self, pos: Vec2, yaw: f32, strong: bool) {
        let fwd = Vec2::new(-yaw.sin(), -yaw.cos());
        let base = fwd.y.atan2(fwd.x);
        for i in 0..10 {
            let t = i as f32 / 9.0 - 0.5; // -0.5..0.5 across the arc
            let a = base + t * 1.9; // ±54°
            let dir = Vec2::new(a.cos(), a.sin());
            let perp = Vec2::new(-dir.y, dir.x);
            let sweep = if t > 0.0 { 1.0 } else { -1.0 };
            self.spawn(Particle {
                pos: Vec3::new(pos.x + dir.x * 1.2, 1.05, pos.y + dir.y * 1.2),
                vel: Vec3::new(dir.x * 2.0 + perp.x * sweep * 1.6, 0.35, dir.y * 2.0 + perp.y * sweep * 1.6),
                life: 0.24,
                max_life: 0.24,
                size: 0.17 + (1.0 - t.abs()) * 0.1,
                color: Vec4::new(0.62, 0.8, 1.0, 0.85),
                gravity: 0.0,
            });
        }
        if strong {
            self.spawn_burst(Vec3::new(pos.x + fwd.x * 1.2, 1.1, pos.y + fwd.y * 1.2), 6, Vec4::new(1.0, 0.85, 0.3, 1.0), 2.5);
        }
    }

    /// Bow / crossbow muzzle puff.
    pub fn spawn_shot(&mut self, pos: Vec2, yaw: f32) {
        let fwd = Vec2::new(-yaw.sin(), -yaw.cos());
        self.spawn_burst(Vec3::new(pos.x + fwd.x * 0.9, 1.15, pos.y + fwd.y * 0.9), 4, Vec4::new(0.85, 0.8, 0.6, 0.7), 1.2);
    }

    /// Continuous per-biome ambient particle nappe (MCD vfx — style guide §2 :
    /// braises, lucioles, flocons, poussière dorée, éclats). ~16 particles/s
    /// around `center`, cheap (cap 3000 shared with combat particles).
    pub fn spawn_ambient(&mut self, biome: crate::world::missions::Biome, center: Vec2, dt: f32) {
        use crate::world::missions::Biome;
        self.ambient_acc += dt * 16.0;
        while self.ambient_acc >= 1.0 {
            self.ambient_acc -= 1.0;
            // (mode, palette, gravity) — Fall = flocons/feuilles, Rise = braises/lucioles
            enum M { Fall, Rise, Drift }
            let (mode, colors, grav, life): (M, &[Vec4], f32, std::ops::Range<f32>) = match biome {
                Biome::Forest => (M::Fall, &[Vec4::new(0.35, 0.66, 0.3, 0.75), Vec4::new(0.5, 0.75, 0.3, 0.7)], 1.4, 3.5..6.0),
                Biome::Plains => (M::Drift, &[Vec4::new(0.96, 0.92, 0.55, 0.6), Vec4::new(0.9, 0.95, 0.6, 0.55)], -0.06, 3.0..5.0),
                Biome::Canyon | Biome::Desert => (M::Drift, &[Vec4::new(0.88, 0.72, 0.45, 0.5), Vec4::new(0.8, 0.65, 0.4, 0.45)], -0.03, 2.5..4.5),
                Biome::Swamp => (M::Rise, &[Vec4::new(0.7, 0.95, 0.3, 0.8), Vec4::new(0.55, 0.9, 0.4, 0.7)], -0.25, 3.0..5.5),
                Biome::Cave => (M::Rise, &[Vec4::new(0.4, 0.9, 1.0, 0.7), Vec4::new(0.55, 0.8, 1.0, 0.6)], -0.2, 3.0..5.0),
                Biome::Mines => (M::Rise, &[Vec4::new(1.0, 0.4, 0.25, 0.75), Vec4::new(0.8, 0.7, 0.6, 0.5)], -0.3, 2.0..4.0),
                Biome::Haven => (M::Drift, &[Vec4::new(1.0, 0.86, 0.5, 0.65), Vec4::new(0.95, 0.9, 0.7, 0.55)], -0.05, 3.0..5.5),
                Biome::Jungle => (M::Rise, &[Vec4::new(0.75, 1.0, 0.4, 0.8), Vec4::new(0.4, 0.8, 0.35, 0.7)], -0.18, 3.0..5.5),
                Biome::Stronghold => (M::Fall, &[Vec4::new(0.62, 0.62, 0.68, 0.5), Vec4::new(0.5, 0.5, 0.56, 0.45)], 0.9, 3.0..5.0),
                Biome::Nether => (M::Rise, &[Vec4::new(1.0, 0.45, 0.12, 0.9), Vec4::new(1.0, 0.65, 0.2, 0.8)], -0.7, 2.0..4.0),
                Biome::Pinnacle | Biome::Void => (M::Rise, &[Vec4::new(0.72, 0.36, 1.0, 0.8), Vec4::new(0.88, 0.35, 1.0, 0.7)], -0.45, 2.5..5.0),
                Biome::Winter => (M::Fall, &[Vec4::new(0.95, 0.97, 1.0, 0.85), Vec4::new(0.85, 0.92, 1.0, 0.8)], 0.55, 4.0..7.0),
                Biome::Peaks => (M::Fall, &[Vec4::new(0.9, 0.95, 1.0, 0.75), Vec4::new(1.0, 1.0, 1.0, 0.7)], 1.2, 3.0..5.0),
                Biome::Depths => (M::Rise, &[Vec4::new(0.5, 0.85, 0.95, 0.7), Vec4::new(0.6, 0.9, 1.0, 0.6)], -0.5, 2.5..4.5),
            };
            let a = self.rng.gen_range(0.0..std::f32::consts::TAU);
            let d = self.rng.gen_range(1.5..12.0);
            let (x, z) = (center.x + a.cos() * d, center.y + a.sin() * d);
            let (y, vy) = match mode {
                M::Fall => (self.rng.gen_range(3.5..6.5), -0.2),
                M::Rise => (self.rng.gen_range(0.15..1.6), self.rng.gen_range(0.15..0.5)),
                M::Drift => (self.rng.gen_range(0.6..2.4), self.rng.gen_range(-0.05..0.1)),
            };
            let c = colors[self.rng.gen_range(0..colors.len())];
            let sway = match mode { M::Fall => 0.55, _ => 0.3 };
            let life_v = self.rng.gen_range(life.start..life.end);
            let size = self.rng.gen_range(0.07..0.16);
            let vx = self.rng.gen_range(-sway..sway);
            let vz = self.rng.gen_range(-sway..sway);
            self.spawn(Particle {
                pos: Vec3::new(x, y, z),
                vel: Vec3::new(vx, vy, vz),
                life: life_v,
                max_life: life.end,
                size,
                color: c,
                gravity: grav,
            });
        }
    }

    pub fn update(&mut self, dt: f32) {
        for p in self.parts.iter_mut() {
            p.life -= dt;
            p.vel.y -= p.gravity * dt;
            p.pos += p.vel * dt;
            if p.pos.y < 0.1 {
                p.pos.y = 0.1;
                p.vel.y *= -0.3;
                p.vel.x *= 0.8;
                p.vel.z *= 0.8;
            }
        }
        self.parts.retain(|p| p.life > 0.0);
    }

    pub fn render(&self, billboards: &mut Vec<BillboardInstance>) {
        for p in &self.parts {
            let a = (p.life / p.max_life).clamp(0.0, 1.0);
            let mut c = p.color;
            c.w *= a;
            billboards.push(BillboardInstance::new(p.pos, [p.size, p.size], c, 1.0));
        }
    }
}

pub enum PickupKind {
    Emerald(u32),
    Arrows(u32),
    Heart,
    Item(items::Item),
}

pub struct Pickup {
    pub pos: Vec2,
    pub kind: PickupKind,
    pub t: f32,
}

pub struct Floater {
    pub pos: Vec3,
    pub text: String,
    pub color: Vec4,
    pub life: f32,
}

// ----------------------------------------------------------------------
// Damage pipeline
// ----------------------------------------------------------------------

pub fn damage_enemy(
    game: &mut Game,
    e: Entity,
    amount: f32,
    crit: bool,
    from: Vec2,
    knockback: f32,
    statuses: Vec<Status>,
) -> bool {
    let mut died = false;
    let mut arch_transform = false;
    let fpos;
    {
        let mut q = game
            .world
            .query_one::<(&Pos, &mut Health, &mut HitFlash, &mut enemy::Enemy)>(e);
        let Ok((pos, health, flash, en)) = q.get() else { return false };
        if health.hp <= 0.0 {
            return false;
        }
        health.hp -= amount;
        flash.0 = 0.8;
        en.aggro = true;
        let dir = pos.0 - from;
        let dist = dir.length().max(0.001);
        en.knock_vel += dir / dist * knockback * consts::KNOCKBACK_BASE;
        for s in statuses {
            en.status.push(s);
        }
        fpos = pos.0;
        if health.hp <= 0.0 {
            // Arch-Illager phase 2: transform into Heart of Ender instead of dying
            if en.def.id == "boss_arch" && !en.phase2 {
                let def = enemy::def_for("boss_heart");
                en.kind = "boss_heart".to_string();
                en.def = def;
                en.phase2 = true;
                health.hp = def.hp * consts::TIER_HP_MULT[game.tier.min(2)];
                health.max = health.hp;
                arch_transform = true;
            } else {
                died = true;
            }
        }
    }

    game.floaters.push(Floater {
        pos: Vec3::new(fpos.x, 1.8, fpos.y),
        text: format!("{}", amount as i32),
        color: if crit { Vec4::new(1.0, 0.85, 0.2, 1.0) } else { Vec4::ONE },
        life: consts::DAMAGE_NUMBER_LIFE,
    });
    game.particles.spawn_burst(
        Vec3::new(fpos.x, 1.0, fpos.y),
        if crit { 10 } else { 5 },
        if crit { Vec4::new(1.0, 0.9, 0.3, 1.0) } else { Vec4::new(0.9, 0.3, 0.3, 0.9) },
        2.5,
    );
    if arch_transform {
        game.boss_phase = 2;
        game.particles.spawn_burst(Vec3::new(fpos.x, 1.5, fpos.y), 40, Vec4::new(0.4, 0.1, 0.6, 1.0), 5.0);
        game.particles.spawn_ring(Vec3::new(fpos.x, 0.3, fpos.y), 3.0, 24, Vec4::new(0.6, 0.2, 0.8, 0.9));
        game.floaters.push(Floater {
            pos: Vec3::new(fpos.x, 2.5, fpos.y),
            text: "Coeur d'Ender !".to_string(),
            color: Vec4::new(0.8, 0.3, 1.0, 1.0),
            life: 2.0,
        });
        return false;
    }
    if died {
        on_enemy_death(game, e);
        return true;
    }
    false
}

pub fn damage_player(game: &mut Game, idx: usize, amount: f32, from: Vec2) {
    let Some(p) = game.players.get_mut(idx) else { return };
    if p.downed_t.is_some() || p.iframes > 0.0 {
        return;
    }
    p.hp -= amount;
    p.iframes = consts::HIT_IFRAME;
    game.floaters.push(Floater {
        pos: Vec3::new(p.pos.x, 2.0, p.pos.y),
        text: format!("-{}", amount as i32),
        color: Vec4::new(1.0, 0.3, 0.3, 1.0),
        life: 0.8,
    });
    game.particles.spawn_burst(Vec3::new(p.pos.x, 1.0, p.pos.y), 6, Vec4::new(0.9, 0.2, 0.2, 0.9), 2.0);
    let dir = (p.pos - from).normalize_or_zero();
    p.knock_vel += dir * 2.5;
    if p.hp <= 0.0 {
        p.hp = 0.0;
        p.downed_t = Some(consts::DOWNED_TIME);
        game.particles.spawn_burst(Vec3::new(p.pos.x, 1.2, p.pos.y), 20, Vec4::new(0.9, 0.2, 0.2, 1.0), 3.0);
        if game.players.iter().all(|p| p.downed_t.is_some()) {
            game.state = RunState::Failed;
        }
    }
}

pub fn heal_player(game: &mut Game, idx: usize, amount: f32) {
    let Some(p) = game.players.get_mut(idx) else { return };
    if p.downed_t.is_some() { return; }
    let before = p.hp;
    p.hp = (p.hp + amount).min(p.max_hp());
    if p.hp > before + 0.01 {
        game.floaters.push(Floater {
            pos: Vec3::new(p.pos.x, 2.0, p.pos.y),
            text: format!("+{}", (p.hp - before) as i32),
            color: Vec4::new(0.4, 1.0, 0.5, 1.0),
            life: 0.8,
        });
        game.particles.spawn_burst(Vec3::new(p.pos.x, 1.0, p.pos.y), 4, Vec4::new(0.4, 1.0, 0.5, 0.9), 1.5);
    }
}

fn on_enemy_death(game: &mut Game, e: Entity) {
    let (pos, xp) = {
        let mut q = game.world.query_one::<(&Pos, &enemy::Enemy)>(e);
        let Ok((pos, en)) = q.get() else { return };
        (pos.0, en.def.xp)
    };
    game.kills += 1;
    let mut rng = rand::thread_rng();
    let tier_mult = 1.0 + game.tier as f32 * 0.5;
    let emeralds = rng.gen_range(consts::EMERALD_DROP_MIN..=consts::EMERALD_DROP_MAX);
    game.pickups.push(Pickup {
        pos,
        kind: PickupKind::Emerald((emeralds as f32 * tier_mult) as u32),
        t: 0.0,
    });
    if rng.gen_bool(0.18) {
        game.pickups.push(Pickup {
            pos: pos + Vec2::new(0.3, 0.2),
            kind: PickupKind::Arrows(rng.gen_range(3..=8)),
            t: 0.0,
        });
    }
    if rng.gen_bool(0.06) {
        game.pickups.push(Pickup { pos: pos + Vec2::new(-0.3, 0.1), kind: PickupKind::Heart, t: 0.0 });
    }
    if rng.gen_bool(0.10 + game.tier as f64 * 0.02) {
        let power = (game.total_threat() + rng.gen_range(0..3)).max(1);
        let item = items::gen_item(&mut rng, power, 0.0);
        game.items_found.push(item.clone());
        game.pickups.push(Pickup { pos: pos + Vec2::new(0.1, -0.3), kind: PickupKind::Item(item), t: 0.0 });
    }
    for p in game.players.iter_mut() {
        p.add_xp(xp);
    }
    game.particles.spawn_burst(Vec3::new(pos.x, 0.9, pos.y), 16, Vec4::new(0.35, 0.35, 0.4, 0.9), 3.5);
}

// ----------------------------------------------------------------------
// Projectiles
// ----------------------------------------------------------------------

pub fn spawn_projectile(game: &mut Game, p: Projectile) {
    if game.projectiles.len() < 256 {
        game.projectiles.push(p);
    }
}

pub fn update_projectiles(game: &mut Game, dt: f32) {
    // move
    for pr in game.projectiles.iter_mut() {
        pr.life -= dt;
        pr.pos += pr.vel * dt;
    }

    // resolve hits without holding borrows
    let prs = std::mem::take(&mut game.projectiles);
    let mut keep: Vec<Projectile> = Vec::with_capacity(prs.len());
    let mut hits_enemy: Vec<(Entity, f32, Vec<Status>, Vec2)> = Vec::new();
    let mut hits_player: Vec<(usize, f32, Vec2, Vec<Status>)> = Vec::new();

    for pr in prs {
        let mut consumed = pr.life <= 0.0;
        if !consumed && game.level.solid_at(pr.pos.x, pr.pos.y) {
            consumed = true;
            game.particles.spawn_burst(Vec3::new(pr.pos.x, 1.0, pr.pos.y), 4, Vec4::new(0.7, 0.7, 0.7, 0.8), 1.5);
        }
        if !consumed {
            match pr.team {
                Team::Player => {
                    let mut found: Option<Entity> = None;
                    for (e, epos, h, en) in game.world.query::<(Entity, &Pos, &Health, &enemy::Enemy)>().iter() {
                        if h.hp > 0.0 && !en.captive && epos.0.distance(pr.pos) < 0.5 + pr.radius {
                            found = Some(e);
                            break;
                        }
                    }
                    if let Some(e) = found {
                        consumed = true;
                        hits_enemy.push((e, pr.dmg, pr.effects.clone(), pr.pos - pr.vel.normalize_or_zero() * 0.5));
                    }
                }
                Team::Enemy => {
                    for (pi, p) in game.players.iter().enumerate() {
                        if p.downed_t.is_none() && p.pos.distance(pr.pos) < 0.45 + pr.radius {
                            consumed = true;
                            hits_player.push((pi, pr.dmg, pr.pos - pr.vel.normalize_or_zero() * 0.5, pr.effects.clone()));
                            break;
                        }
                    }
                }
            }
        }
        if consumed {
            if let Some((r, d)) = pr.explode {
                explode_at(game, pr.pos, r, d, pr.team);
            }
        } else {
            keep.push(pr);
        }
    }
    game.projectiles = keep;

    for (e, dmg, effects, from) in hits_enemy {
        damage_enemy(game, e, dmg, false, from, 0.4, effects);
    }
    for (idx, dmg, from, effects) in hits_player {
        damage_player(game, idx, dmg, from);
        if !effects.is_empty() {
            if let Some(p) = game.players.get_mut(idx) {
                for s in effects {
                    p.status.push(s);
                }
            }
        }
    }
}

pub fn explode_at(game: &mut Game, pos: Vec2, radius: f32, dmg: f32, team: Team) {
    game.particles.spawn_burst(Vec3::new(pos.x, 0.8, pos.y), 30, Vec4::new(1.0, 0.7, 0.25, 1.0), 6.0);
    game.particles.spawn_ring(Vec3::new(pos.x, 0.3, pos.y), radius, 18, Vec4::new(1.0, 0.6, 0.2, 0.8));
    match team {
        Team::Player => {
            let targets: Vec<(Entity, f32)> = game
                .world
                .query::<(Entity, &Pos, &Health)>()
                .iter()
                .filter(|(_, p, h)| h.hp > 0.0 && p.0.distance(pos) < radius)
                .map(|(e, p, _)| (e, dmg * (1.0 - p.0.distance(pos) / radius * 0.5)))
                .collect();
            for (e, d) in targets {
                damage_enemy(game, e, d, false, pos, 1.5, Vec::new());
            }
        }
        Team::Enemy => {
            for pi in 0..game.players.len() {
                if let Some(p) = game.players.get(pi) {
                    if p.downed_t.is_none() && p.pos.distance(pos) < radius {
                        damage_player(game, pi, dmg, pos);
                    }
                }
            }
        }
    }
}

// ----------------------------------------------------------------------
// Zones (poison clouds, cursed ground)
// ----------------------------------------------------------------------

pub fn update_zones(game: &mut Game, dt: f32) {
    let zones: Vec<Zone> = game.zones.clone();
    for z in &zones {
        if rand::thread_rng().gen_bool(0.4) {
            game.particles.spawn_burst(
                Vec3::new(
                    z.pos.x + rand::thread_rng().gen_range(-z.radius..z.radius),
                    0.4,
                    z.pos.y + rand::thread_rng().gen_range(-z.radius..z.radius),
                ),
                2,
                z.color,
                0.8,
            );
        }
        match z.team {
            Team::Player => {
                let hits: Vec<Entity> = game
                    .world
                    .query::<(Entity, &Pos, &Health, &enemy::Enemy)>()
                    .iter()
                    .filter(|(_, p, h, en)| h.hp > 0.0 && !en.captive && p.0.distance(z.pos) < z.radius)
                    .map(|(e, _, _, _)| e)
                    .collect();
                let mut statuses = vec![Status::new(StatusKind::Poison, z.dps, 0.6)];
                if z.slow {
                    statuses.push(Status::new(StatusKind::Slow, 0.0, 0.6));
                }
                for e in hits {
                    damage_enemy(game, e, z.dps * dt, false, z.pos, 0.0, statuses.clone());
                }
            }
            Team::Enemy => {
                for pi in 0..game.players.len() {
                    let (in_zone, ppos) = match game.players.get(pi) {
                        Some(p) => (p.downed_t.is_none() && p.pos.distance(z.pos) < z.radius, p.pos),
                        None => continue,
                    };
                    if in_zone {
                        damage_player(game, pi, z.dps * dt, ppos);
                    }
                }
            }
        }
    }
    for z in game.zones.iter_mut() {
        z.t -= dt;
    }
    game.zones.retain(|z| z.t > 0.0);
}

// ----------------------------------------------------------------------
// Artifacts
// ----------------------------------------------------------------------

pub fn use_artifact(game: &mut Game, pidx: usize, slot: usize) -> bool {
    let (item_power, kind, pos, facing) = {
        let Some(p) = game.players.get(pidx) else { return false };
        let Some(Some(item)) = p.inventory.artifacts.get(slot) else { return false };
        let ItemClass::Art(kind) = item.class else { return false };
        (item.power, kind, p.pos, Vec2::new(-p.yaw.sin(), -p.yaw.cos()))
    };
    let power_mult = 1.0 + 0.1 * item_power as f32;
    match kind {
        ArtifactKind::Lightning => {
            let mut candidates: Vec<(Entity, f32)> = game
                .world
                .query::<(Entity, &Pos, &Health, &enemy::Enemy)>()
                .iter()
                .filter(|(_, _p, h, en)| h.hp > 0.0 && !en.captive)
                .map(|(e, p, _, _)| (e, p.0.distance(pos)))
                .collect();
            candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            let mut chained = 0;
            let mut last = pos;
            for (e, _) in candidates {
                if chained >= 4 { break; }
                let d = match game.world.get::<&Pos>(e) {
                    Ok(p) => p.0.distance(last),
                    Err(_) => continue,
                };
                if d < 14.0 {
                    damage_enemy(game, e, 35.0 * power_mult, true, last, 0.5, Vec::new());
                    if let Ok(pp) = game.world.get::<&Pos>(e) {
                        game.particles.spawn_burst(Vec3::new(pp.0.x, 1.5, pp.0.y), 12, Vec4::new(0.9, 0.95, 0.3, 1.0), 4.0);
                        last = pp.0;
                    }
                    chained += 1;
                }
            }
            if chained == 0 {
                return false; // no target: don't consume cooldown
            }
        }
        ArtifactKind::Totem => {
            if let Some(p) = game.players.get_mut(pidx) {
                p.totem_t = 12.0;
            }
            game.particles.spawn_ring(Vec3::new(pos.x, 0.2, pos.y), 4.5, 24, Vec4::new(0.5, 1.0, 0.6, 0.9));
        }
        ArtifactKind::WindHorn => {
            let targets: Vec<Entity> = game
                .world
                .query::<(Entity, &Pos, &Health, &enemy::Enemy)>()
                .iter()
                .filter(|(_, ep, h, en)| h.hp > 0.0 && !en.captive && ep.0.distance(pos) < 6.0)
                .map(|(e, _, _, _)| e)
                .collect();
            for e in targets {
                damage_enemy(game, e, 8.0, false, pos, 3.5, Vec::new());
            }
            game.particles.spawn_burst(Vec3::new(pos.x, 0.8, pos.y), 25, Vec4::new(0.7, 0.9, 1.0, 0.8), 7.0);
        }
        ArtifactKind::Harvester => {
            if let Some(p) = game.players.get_mut(pidx) {
                p.harvest_t = 5.0;
            }
            game.particles.spawn_burst(Vec3::new(pos.x, 1.2, pos.y), 10, Vec4::new(0.9, 0.8, 0.6, 0.9), 2.0);
        }
        ArtifactKind::Fireworks => {
            spawn_projectile(game, Projectile {
                pos,
                vel: facing * 14.0,
                team: Team::Player,
                dmg: 20.0,
                kind: ProjKind::Firework,
                life: 2.0,
                radius: 0.35,
                effects: Vec::new(),
                explode: Some((3.2, 55.0 * power_mult)),
            });
        }
        ArtifactKind::Seeds => {
            let land = pos + facing * 4.0;
            game.zones.push(Zone {
                pos: land,
                radius: 3.0,
                dps: 14.0 * power_mult,
                slow: true,
                t: 4.0,
                team: Team::Player,
                color: Vec4::new(0.3, 0.8, 0.25, 0.7),
            });
        }
    }
    true
}

/// Totem auras + harvest timers.
pub fn update_artifact_auras(game: &mut Game, dt: f32) {
    let totem_players: Vec<(usize, Vec2)> = game
        .players
        .iter()
        .enumerate()
        .filter(|(_, p)| p.totem_t > 0.0 && p.downed_t.is_none())
        .map(|(i, p)| (i, p.pos))
        .collect();
    for (i, tpos) in totem_players {
        heal_player(game, i, 2.4 * dt);
        for j in 0..game.players.len() {
            if j != i {
                let near = game.players.get(j).map(|o| o.pos.distance(tpos) < 4.5 && o.downed_t.is_none()).unwrap_or(false);
                if near {
                    heal_player(game, j, 2.4 * dt);
                }
            }
        }
        if game.time % 0.5 < dt {
            game.particles.spawn_ring(Vec3::new(tpos.x, 0.2, tpos.y), 4.5, 10, Vec4::new(0.5, 1.0, 0.6, 0.5));
        }
    }
    for p in game.players.iter_mut() {
        p.totem_t = (p.totem_t - dt).max(0.0);
        p.harvest_t = (p.harvest_t - dt).max(0.0);
    }
}
