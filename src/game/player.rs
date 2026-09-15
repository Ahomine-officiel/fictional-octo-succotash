//! Player state and behavior: movement, melee/ranged combat, roll, artifacts,
//! potion, XP, downed/revive for local co-op.

use super::combat::{self, ProjKind, Projectile, Status, StatusKind, Team};
use super::enemy;
use super::items::{ArtifactKind, Enchant, ItemClass};
use super::items::artifact_def;
use super::{Game, PlayerInput};
use crate::consts;
use crate::models::AnimState;
use glam::Vec2;
use hecs::Entity;
use rand::Rng;

pub struct PlayerState {
    pub idx: usize,
    pub pos: Vec2,
    pub yaw: f32,
    pub hp: f32,
    pub arrows: u32,
    pub potions: u32,
    pub potion_cd: f32,
    pub roll_t: f32,
    pub roll_cd: f32,
    pub roll_dir: Vec2,
    pub attack_t: f32,
    pub attack_cd: f32,
    pub combo: u32,
    pub combo_reset_t: f32,
    pub iframes: f32,
    pub downed_t: Option<f32>,
    pub revive_progress: f32,
    pub artifact_cd: [f32; 3],
    pub knock_vel: Vec2,
    pub totem_t: f32,
    pub harvest_t: f32,
    pub status: Vec<Status>,
    pub anim: AnimState,
    pub inventory: super::inventory::Inventory,
    pub level: u32,
    pub xp: u32,
    pub swing_hit_done: bool,
}

impl PlayerState {
    pub fn new(idx: usize, pos: Vec2) -> PlayerState {
        PlayerState {
            idx,
            pos,
            yaw: 0.0,
            hp: 0.0, // fixed on spawn via max_hp
            arrows: consts::START_ARROWS,
            potions: consts::POTIONS_PER_MISSION,
            potion_cd: 0.0,
            roll_t: 0.0,
            roll_cd: 0.0,
            roll_dir: Vec2::ZERO,
            attack_t: 0.0,
            attack_cd: 0.0,
            combo: 0,
            combo_reset_t: 0.0,
            iframes: 0.0,
            downed_t: None,
            revive_progress: 0.0,
            artifact_cd: [0.0; 3],
            knock_vel: Vec2::ZERO,
            totem_t: 0.0,
            harvest_t: 0.0,
            status: Vec::new(),
            anim: AnimState::default(),
            inventory: super::inventory::Inventory::starting(),
            level: 1,
            xp: 0,
            swing_hit_done: false,
        }
    }

    /// Convention: yaw = rotation around Y; forward = (-sin(yaw), -cos(yaw)).
    pub fn forward(&self) -> Vec2 {
        Vec2::new(-self.yaw.sin(), -self.yaw.cos())
    }

    pub fn max_hp(&self) -> f32 {
        let armor_hp = self
            .inventory
            .armor
            .as_ref()
            .map(|a| a.armor_hp())
            .unwrap_or(0.0);
        consts::HERO_BASE_HP + armor_hp
    }

    pub fn move_mult(&self) -> f32 {
        self.inventory
            .armor
            .as_ref()
            .map(|a| a.armor_move())
            .unwrap_or(1.0)
    }

    /// returns true on level up
    pub fn add_xp(&mut self, amount: u32) -> bool {
        if self.downed_t.is_some() {
            return false;
        }
        self.xp += amount;
        let mut leveled = false;
        while self.xp >= consts::xp_for_level(self.level) {
            self.xp -= consts::xp_for_level(self.level);
            self.level += 1;
            self.inventory.enchant_points += consts::ENCHANT_POINT_PER_LEVEL;
            leveled = true;
        }
        leveled
    }

    fn slow_factor(&self) -> f32 {
        let mut s = 1.0;
        for st in &self.status {
            if st.kind == StatusKind::Slow {
                s *= st.slow_mult;
            }
        }
        s
    }
}

// ----------------------------------------------------------------------
// Per-frame update
// ----------------------------------------------------------------------

pub fn update_player(game: &mut Game, idx: usize, input: &PlayerInput, dt: f32) {
    enum Act {
        Melee,
        Ranged,
        Artifact(usize),
        Potion,
        Interact,
        Revive(usize),
    }
    let mut acts: Vec<Act> = Vec::new();

    // ---------------- phase 1: state ----------------
    let others: Vec<(usize, Vec2)> = game
        .players
        .iter()
        .enumerate()
        .filter(|(j, _)| *j != idx)
        .map(|(j, o)| (j, o.pos))
        .collect();
    {
        let Some(p) = game.players.get_mut(idx) else { return };

        // status ticks
        for s in p.status.iter_mut() {
            s.t -= dt;
            if s.kind == StatusKind::Poison || s.kind == StatusKind::Burn {
                p.hp -= s.dps * dt;
            }
        }
        p.status.retain(|s| s.t > 0.0);

        // downed handling
        if p.downed_t.is_some() {
            p.downed_t = Some(p.downed_t.unwrap() - dt);
            p.anim.dead = 1.0;
            if p.downed_t.unwrap() <= 0.0 {
                // respawn at mission start with half HP
                p.downed_t = None;
                p.revive_progress = 0.0;
                p.hp = p.max_hp() * 0.5;
                p.pos = game.level.spawns[idx.min(game.level.spawns.len() - 1)];
                p.iframes = 2.0;
            }
            return;
        }

        p.iframes -= dt;
        p.roll_cd -= dt;
        p.potion_cd -= dt;
        p.attack_cd -= dt;
        for c in p.artifact_cd.iter_mut() {
            *c -= dt;
        }
        p.combo_reset_t -= dt;
        if p.combo_reset_t <= 0.0 && p.combo > 0 {
            p.combo = 0;
        }
        p.knock_vel *= (1.0 - dt * 6.0).max(0.0);

        // ---- movement ----
        let speed = consts::PLAYER_SPEED * p.move_mult() * p.slow_factor()
            * if game.level.water_at(p.pos.x, p.pos.y) { 0.6 } else { 1.0 };
        let vel;
        if input.roll && p.roll_cd <= 0.0 && p.roll_t <= 0.0 {
            p.roll_t = consts::ROLL_TIME;
            p.roll_cd = consts::ROLL_COOLDOWN;
            p.roll_dir = if input.mv.length_squared() > 0.01 { input.mv.normalize_or_zero() } else { p.forward() };
            p.iframes = p.iframes.max(consts::ROLL_TIME);
        }
        if p.roll_t > 0.0 {
            p.roll_t -= dt;
            p.anim.roll = 1.0 - (p.roll_t / consts::ROLL_TIME).max(0.0);
            // traînée de poussière PENDANT le salto (décollage + réception)
            let prev = p.roll_t + dt;
            if ((p.roll_t * 22.0) as i32) != ((prev * 22.0) as i32) {
                game.particles.spawn_burst(
                    glam::Vec3::new(p.pos.x, 0.12, p.pos.y),
                    2,
                    glam::Vec4::new(0.62, 0.52, 0.38, 0.65),
                    1.3,
                );
            }
            vel = p.roll_dir * consts::PLAYER_SPEED * consts::ROLL_SPEED_MULT;
        } else {
            p.anim.roll = 0.0;
            vel = input.mv * speed;
            if input.mv.length_squared() > 0.01 {
                p.yaw = (-input.mv.x).atan2(-input.mv.y);
            }
        }
        p.anim.moving = (vel.length() / consts::PLAYER_SPEED).min(1.0);
        p.anim.walk_phase += vel.length() * dt * 1.9;

        let mut newpos = p.pos + (vel + p.knock_vel) * dt;
        game.level.collide(&mut newpos, consts::PLAYER_RADIUS);
        // soft separation between players
        for (j, opos) in &others {
            let d = newpos.distance(*opos);
            if d < 0.7 && d > 0.001 {
                newpos += (newpos - *opos) / d * (0.7 - d) * 0.5;
            }
            let _ = j;
        }
        p.pos = newpos;

        // ---- revive downed partner ----
        for (j, opos) in &others {
            if p.pos.distance(*opos) < 2.2 {
                acts.push(Act::Revive(*j));
            }
        }

        // ---- actions ----
        if input.melee && p.attack_cd <= 0.0 && p.attack_t <= 0.0 {
            let spd = p.inventory.melee.as_ref().map(|m| m.melee_speed()).unwrap_or(2.0);
            p.attack_t = consts::SWING_TIME;
            p.swing_hit_done = false;
            p.attack_cd = 1.0 / spd;
        }
        if p.attack_t > 0.0 {
            p.attack_t -= dt;
            p.anim.swing = 1.0 - (p.attack_t / consts::SWING_TIME).max(0.0);
            if !p.swing_hit_done && p.attack_t <= consts::SWING_TIME - consts::SWING_HIT_T {
                p.swing_hit_done = true;
                acts.push(Act::Melee);
            }
        } else {
            p.anim.swing = 0.0;
        }

        if input.ranged && p.attack_cd <= 0.0 && p.arrows > 0 {
            acts.push(Act::Ranged);
        }
        for (i, pressed) in input.artifacts.iter().enumerate() {
            if *pressed && p.artifact_cd[i] <= 0.0 {
                acts.push(Act::Artifact(i));
            }
        }
        if input.potion && p.potion_cd <= 0.0 && p.potions > 0 {
            acts.push(Act::Potion);
        }
        if input.interact {
            acts.push(Act::Interact);
        }
    }

    // ---------------- phase 2: resolve actions ----------------
    for a in acts {
        match a {
            Act::Melee => melee_hit(game, idx),
            Act::Ranged => shoot_arrow(game, idx),
            Act::Artifact(i) => {
                let (has, kind, power) = {
                    let p = &game.players[idx];
                    match p.inventory.artifacts.get(i).and_then(|x| x.as_ref()) {
                        Some(item) => {
                            if let ItemClass::Art(k) = item.class {
                                (true, k, item.power)
                            } else {
                                (false, ArtifactKind::Totem, 1)
                            }
                        }
                        None => (false, ArtifactKind::Totem, 1),
                    }
                };
                if has && combat::use_artifact(game, idx, i) {
                    let p = &mut game.players[idx];
                    p.artifact_cd[i] = artifact_def(kind).cd * (1.0 - 0.01 * power as f32).max(0.7);
                }
            }
            Act::Potion => {
                let p = &mut game.players[idx];
                if p.potions > 0 && p.potion_cd <= 0.0 {
                    p.potions -= 1;
                    p.potion_cd = consts::POTION_COOLDOWN;
                    let heal = p.max_hp() * consts::POTION_HEAL_FRAC;
                    let pos = p.pos;
                    combat::heal_player(game, idx, heal);
                    game.particles.spawn_burst(glam::Vec3::new(pos.x, 1.0, pos.y), 14, glam::Vec4::new(0.9, 0.3, 0.5, 0.9), 2.0);
                }
            }
            Act::Interact => {
                super::loot::try_interact(game, idx);
            }
            Act::Revive(j) => {
                let near = game.players.get(idx).map(|p| {
                    game.players.get(j).map(|o| p.pos.distance(o.pos) < 2.2).unwrap_or(false)
                }).unwrap_or(false);
                if near {
                    let o = &mut game.players[j];
                    o.revive_progress += dt;
                    if o.revive_progress >= 3.0 {
                        o.downed_t = None;
                        o.revive_progress = 0.0;
                        o.hp = o.max_hp() * 0.5;
                        o.iframes = 2.0;
                        let pos = o.pos;
                        game.particles.spawn_burst(glam::Vec3::new(pos.x, 1.0, pos.y), 20, glam::Vec4::new(0.4, 1.0, 0.5, 1.0), 3.0);
                    }
                }
            }
        }
    }
}

fn melee_hit(game: &mut Game, idx: usize) {
    let (reach, dmg, crit_chance, kb, ench, combo, pos, yaw) = {
        let p = &game.players[idx];
        let Some(w) = p.inventory.melee.as_ref() else { return };
        let crit = consts::BASE_CRIT_CHANCE
            + 0.05 * w.ench_level(Enchant::CriticalHit) as f32;
        (
            w.melee_reach(),
            w.melee_dmg() * (1.0 + w.ench_level(Enchant::Sharpness) as f32 * 0.10),
            crit,
            w.melee_knockback(),
            w.clone(),
            p.combo,
            p.pos,
            p.yaw,
        )
    };
    let fwd = Vec2::new(-yaw.sin(), -yaw.cos());
    // MCD swing arc VFX — the attack is now visible (ref_46)
    game.particles.spawn_slash(pos, yaw, combo >= 2);
    // gather enemies in arc
    let mut hits: Vec<(hecs::Entity, f32)> = Vec::new();
    for (e, ep, h, en) in game
        .world
        .query::<(Entity, &super::Pos, &super::Health, &enemy::Enemy)>()
        .iter()
    {
        if h.hp <= 0.0 || en.captive {
            continue;
        }
        let d = ep.0 - pos;
        let dist = d.length();
        if dist > reach + 0.4 {
            continue;
        }
        let ang = fwd.angle_to(d.normalize_or_zero()).abs();
        if ang < consts::MELEE_ARC_DEG.to_radians() * 0.5 {
            hits.push((e, dist));
        }
    }
    if hits.is_empty() {
        return;
    }
    let combo_mult = 1.0 + (combo as f32).min(consts::COMBO_MAX as f32) * consts::COMBO_STEP_MULT;
    let mut healed = false;
    for (e, _) in &hits {
        let mut rng = rand::thread_rng();
        let crit = rng.gen::<f32>() < crit_chance;
        let mut amount = dmg * combo_mult * if crit { consts::CRIT_MULT } else { 1.0 };
        amount *= rng.gen_range(0.92..1.08);
        let mut statuses = Vec::new();
        // Fire Aspect
        let fa = ench.ench_level(Enchant::FireAspect);
        if fa > 0 && rng.gen_bool(0.25 + 0.15 * fa as f64) {
            statuses.push(Status::new(StatusKind::Burn, 6.0 * fa as f32, 2.0));
        }
        combat::damage_enemy(game, *e, amount, crit, pos, kb, statuses);
        // Thundering: on crit, lightning extra hit
        let th = ench.ench_level(Enchant::Thundering);
        if th > 0 && crit {
            combat::damage_enemy(game, *e, 25.0 * th as f32, false, pos, 0.0, Vec::new());
            if let Ok(ep) = game.world.get::<&super::Pos>(*e) {
                game.particles.spawn_burst(
                    glam::Vec3::new(ep.0.x, 1.5, ep.0.y),
                    10,
                    glam::Vec4::new(0.9, 0.95, 0.3, 1.0),
                    4.0,
                );
            }
        }
        // Radiance
        let ra = ench.ench_level(Enchant::Radiance);
        if ra > 0 && !healed && rng.gen_bool(0.05 * ra as f64) {
            healed = true;
            combat::heal_player(game, idx, 4.0);
        }
    }
    // Swirling: burst after 3rd combo hit
    let sw = ench.ench_level(Enchant::Swirling);
    if sw > 0 && (combo + 1) % 3 == 0 {
        let targets: Vec<hecs::Entity> = game
            .world
            .query::<(Entity, &super::Pos, &super::Health, &enemy::Enemy)>()
            .iter()
            .filter(|(_, ep, h, en)| h.hp > 0.0 && !en.captive && ep.0.distance(pos) < 2.6)
            .map(|(e, _, _, _)| e)
            .collect();
        for t in targets {
            combat::damage_enemy(game, t, 20.0 * sw as f32, false, pos, 1.0, Vec::new());
        }
        game.particles.spawn_ring(glam::Vec3::new(pos.x, 0.4, pos.y), 2.6, 16, glam::Vec4::new(0.7, 0.9, 1.0, 0.8));
    }
    let p = &mut game.players[idx];
    p.combo = (p.combo + 1).min(consts::COMBO_MAX);
    p.combo_reset_t = 3.0;
}

fn shoot_arrow(game: &mut Game, idx: usize) {
    let (dmg, effects, harvest, pos, yaw) = {
        let p = &game.players[idx];
        let Some(w) = p.inventory.ranged.as_ref() else { return };
        let mut effects = Vec::new();
        let pc = w.ench_level(Enchant::PoisonCloud);
        if pc > 0 && rand::thread_rng().gen_bool(0.10 * pc as f64) {
            effects.push(Status::new(StatusKind::Poison, 6.0, 3.0));
        }
        let gr = w.ench_level(Enchant::Gravity);
        if gr > 0 {
            effects.push(Status::new(StatusKind::Slow, 0.0, 1.0 + 0.5 * gr as f32));
        }
        (w.ranged_dmg(), effects, p.harvest_t > 0.0, p.pos, p.yaw)
    };
    let fwd = Vec2::new(-yaw.sin(), -yaw.cos());
    // auto-aim: nearest enemy within 20 units in a 100-degree cone
    let mut best: Option<(hecs::Entity, f32)> = None;
    for (e, ep, h, en) in game
        .world
        .query::<(Entity, &super::Pos, &super::Health, &enemy::Enemy)>()
        .iter()
    {
        if h.hp <= 0.0 || en.captive {
            continue;
        }
        let d = ep.0 - pos;
        let dist = d.length();
        if dist > 20.0 {
            continue;
        }
        let ang = fwd.angle_to(d.normalize_or_zero()).abs();
        if ang < 50.0f32.to_radians() && best.map(|(_, bd)| dist < bd).unwrap_or(true) {
            best = Some((e, dist));
        }
    }
    let dir = match best {
        Some((e, _)) => {
            let ep = game.world.get::<&super::Pos>(e).map(|p| p.0).unwrap_or(pos + fwd);
            (ep - pos).normalize_or_zero()
        }
        None => fwd,
    };
    let p = &mut game.players[idx];
    p.arrows -= 1;
    p.attack_cd = 1.0 / p.inventory.ranged.as_ref().map(|w| w.ranged_speed()).unwrap_or(1.0);
    p.yaw = (-dir.x).atan2(-dir.y);
    let shot_pos = p.pos;
    let shot_yaw = p.yaw;
    game.particles.spawn_shot(shot_pos, shot_yaw);
    let n = if harvest { 5 } else { 1 };
    for k in 0..n {
        let spread = if n > 1 {
            (k as f32 - (n - 1) as f32 / 2.0) * 0.12
        } else {
            0.0
        };
        let d = Vec2::new(
            dir.x * spread.cos() - dir.y * spread.sin(),
            dir.x * spread.sin() + dir.y * spread.cos(),
        );
        combat::spawn_projectile(
            game,
            Projectile {
                pos: pos + d * 0.6,
                vel: d * consts::ARROW_SPEED,
                team: Team::Player,
                dmg,
                kind: ProjKind::Arrow,
                life: 1.6,
                radius: 0.25,
                effects: effects.clone(),
                explode: None,
            },
        );
    }
}
