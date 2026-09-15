//! Enemy definitions, AI state machine, and boss patterns.
//! Base stats wiki-calibrated (zombie T1 = 55 HP), attack values re-scaled
//! for playability against 100+ player HP, then multiplied by threat tier.
//!
//! Borrow strategy: per-entity decisions are collected into `EnemyAction`s
//! while holding hecs mutable access; actions execute after access is dropped.

use super::combat::{self, Projectile, ProjKind, Status, StatusKind, Team};
use super::{Game, Health, HitFlash, Pos};
use crate::consts;
use crate::world::missions::BossKind;
use glam::{Vec2, Vec3, Vec4};
use hecs::Entity;
use rand::Rng;

pub const DEATH_TIME: f32 = 0.7;

#[derive(Clone, Copy)]
pub enum Special {
    None,
    Explode,
    Summon(&'static str, u32, f32),
    Charge,
    Volley(u32),
    /// Blinks near the target (enderman family), then strikes.
    Teleport,
    Boss(BossKind),
}

pub struct EnemyDef {
    pub id: &'static str,
    pub name_fr: &'static str,
    pub hp: f32,
    pub atk: f32,
    pub speed: f32,
    pub range: f32,
    pub atk_cd: f32,
    pub ranged: Option<ProjKind>,
    pub special: Special,
    pub xp: u32,
    pub knock_resist: f32,
}

macro_rules! def {
    ($id:literal, $name:literal, $hp:literal, $atk:literal, $speed:literal, $range:literal, $cd:literal, $ranged:expr, $special:expr, $xp:literal, $kr:literal) => {
        EnemyDef { id: $id, name_fr: $name, hp: $hp, atk: $atk, speed: $speed, range: $range, atk_cd: $cd, ranged: $ranged, special: $special, xp: $xp, knock_resist: $kr }
    };
}

pub const DEFS: &[EnemyDef] = &[
    def!("zombie", "Zombie", 55.0, 12.0, 2.6, 1.5, 1.2, None, Special::None, 2, 1.0),
    def!("husk", "Husk", 62.0, 14.0, 2.8, 1.5, 1.1, None, Special::None, 3, 1.0),
    def!("jungle_zombie", "Zombie de la jungle", 58.0, 11.0, 2.7, 1.5, 1.1, None, Special::None, 3, 1.0),
    def!("drowned", "Noyé", 60.0, 13.0, 2.7, 1.5, 1.2, None, Special::None, 3, 1.0),
    def!("skeleton", "Squelette", 40.0, 10.0, 2.4, 1.4, 1.9, Some(ProjKind::Arrow), Special::None, 3, 1.2),
    def!("creeper", "Creeper", 45.0, 36.0, 3.1, 2.0, 1.0, None, Special::Explode, 4, 1.3),
    def!("spider", "Araignée", 42.0, 9.0, 3.9, 1.4, 0.9, None, Special::None, 3, 1.0),
    def!("slime", "Slime", 48.0, 8.0, 2.2, 1.4, 1.3, None, Special::None, 2, 0.8),
    def!("pillager", "Pillard", 50.0, 12.0, 2.5, 1.5, 1.9, Some(ProjKind::Arrow), Special::None, 4, 1.0),
    def!("vindicator", "Vindicte", 85.0, 20.0, 3.0, 1.6, 1.3, None, Special::Charge, 5, 0.7),
    def!("enchanter", "Enchanteur", 55.0, 9.0, 2.2, 1.4, 2.6, Some(ProjKind::Magic), Special::Volley(3), 5, 1.0),
    def!("geomancer", "Géomancien", 70.0, 10.0, 2.0, 1.4, 2.4, Some(ProjKind::Magic), Special::Summon("slime", 1, 9.0), 6, 0.8),
    def!("necromancer", "Nécromancien", 60.0, 8.0, 2.1, 1.4, 2.8, Some(ProjKind::Magic), Special::Summon("skeleton", 2, 8.0), 6, 1.0),
    def!("witch", "Sorcière", 55.0, 9.0, 2.3, 1.4, 2.2, Some(ProjKind::Seeds), Special::None, 5, 1.0),
    def!("mooshroom", "Mooshroom", 75.0, 16.0, 2.9, 1.6, 1.4, None, Special::Charge, 4, 0.6),
    def!("bat", "Chauve-souris", 15.0, 5.0, 4.6, 1.2, 0.8, None, Special::None, 1, 1.6),
    def!("redstone_golem", "Golem Redstone", 190.0, 24.0, 2.2, 2.2, 1.8, None, Special::None, 10, 0.25),
    def!("blaze", "Blaze", 50.0, 11.0, 2.8, 1.4, 2.0, Some(ProjKind::Fire), Special::Volley(2), 5, 1.2),
    def!("wraith", "Spectre", 65.0, 14.0, 3.2, 1.5, 1.2, None, Special::None, 5, 1.0),
    def!("boss_cauldron", "Chaudron Corrompu", 620.0, 14.0, 0.0, 0.0, 4.0, Some(ProjKind::Slime), Special::Boss(BossKind::CorruptedCauldron), 60, 0.0),
    def!("boss_monstrosity", "Monstruosité Redstone", 950.0, 30.0, 2.4, 2.6, 1.9, None, Special::Boss(BossKind::RedstoneMonstrosity), 90, 0.0),
    def!("boss_nameless", "le Sans-Nom", 800.0, 18.0, 2.3, 1.6, 2.2, Some(ProjKind::Magic), Special::Boss(BossKind::NamelessOne), 80, 0.0),
    def!("boss_arch", "Arch-Illageois", 1000.0, 22.0, 2.6, 1.7, 1.7, Some(ProjKind::Magic), Special::Boss(BossKind::ArchIllager), 0, 0.0),
    def!("boss_heart", "Cœur d'Ender", 700.0, 24.0, 2.8, 1.6, 2.0, Some(ProjKind::Magic), Special::Boss(BossKind::ArchIllager), 0, 0.0),
    // ------------------------------------------------------------------
    // Bestiary batch 2 — regular mobs (MCD base game + DLC)
    // ------------------------------------------------------------------
    def!("mossy_skeleton", "Squelette Moussu", 45.0, 11.0, 2.4, 13.0, 1.9, Some(ProjKind::Arrow), Special::None, 4, 1.1),
    def!("skeleton_vanguard", "Squelette d'Élite", 95.0, 19.0, 2.7, 1.6, 1.2, None, Special::None, 6, 0.6),
    def!("sunken_skeleton", "Squelette Immergé", 50.0, 11.0, 2.4, 13.0, 2.0, Some(ProjKind::Arrow), Special::None, 4, 1.1),
    def!("drowned_necromancer", "Nécromancien Noyé", 130.0, 12.0, 2.2, 1.4, 2.6, Some(ProjKind::Magic), Special::Summon("drowned", 2, 8.0), 10, 0.7),
    def!("whisperer", "Murmureur", 90.0, 11.0, 2.2, 12.0, 2.3, Some(ProjKind::Seeds), Special::Summon("poison_anemone", 1, 12.0), 8, 0.9),
    def!("royal_guard", "Garde Royal", 170.0, 24.0, 2.6, 1.7, 1.4, None, Special::Charge, 10, 0.15),
    def!("illusioner", "Illusionniste", 65.0, 10.0, 2.2, 1.4, 2.5, Some(ProjKind::Magic), Special::Volley(3), 6, 1.0),
    def!("frozen_zombie", "Zombie Gelé", 66.0, 15.0, 2.5, 1.5, 1.2, None, Special::None, 3, 1.0),
    def!("ghostly_kindler", "Rêveur Spectral", 70.0, 14.0, 3.0, 1.5, 1.2, None, Special::None, 6, 1.0),
    def!("piglin", "Piglin", 70.0, 16.0, 2.9, 1.5, 1.2, None, Special::Charge, 5, 0.7),
    def!("piglin_brute", "Brute Pigline", 115.0, 24.0, 2.8, 1.6, 1.3, None, Special::None, 7, 0.4),
    def!("endling", "Endling", 85.0, 20.0, 3.2, 1.6, 1.1, None, Special::Teleport, 8, 0.6),
    def!("iceologer", "Glaciologue", 140.0, 16.0, 2.1, 1.4, 2.6, Some(ProjKind::Ice), Special::Volley(2), 12, 0.7),
    def!("mountaineer", "Alpiniste", 82.0, 17.0, 2.9, 1.6, 1.2, None, Special::Charge, 6, 0.6),
    def!("windcaller", "Siffle-Vent", 155.0, 16.0, 2.3, 1.4, 2.4, Some(ProjKind::Magic), Special::Volley(4), 12, 0.7),
    def!("squall_golem", "Golem Bourrasque", 200.0, 22.0, 2.2, 2.0, 1.7, None, Special::None, 10, 0.25),
    def!("wither_skeleton", "Squelette Wither", 95.0, 21.0, 2.8, 1.6, 1.2, None, Special::None, 7, 0.5),
    def!("zombified_pig", "Piglin Zombifié", 75.0, 18.0, 2.8, 1.5, 1.2, None, Special::None, 5, 0.8),
    def!("enderman", "Enderman", 130.0, 26.0, 3.4, 1.8, 1.0, None, Special::Teleport, 10, 0.5),
    def!("endermite", "Endermite", 18.0, 6.0, 4.4, 1.2, 0.8, None, Special::None, 1, 1.5),
    def!("silverfish", "Poisson d'Argent", 16.0, 5.0, 4.2, 1.1, 0.8, None, Special::None, 1, 1.6),
    def!("caerbannog", "Lapin Meurtrier", 30.0, 14.0, 4.8, 1.2, 0.7, None, Special::Charge, 3, 1.6),
    def!("vex", "Vex", 35.0, 12.0, 4.0, 1.3, 0.9, None, Special::None, 4, 1.4),
    def!("leapleaf", "Sautefeuille", 115.0, 22.0, 2.6, 1.8, 1.3, None, Special::Charge, 8, 0.4),
    def!("poison_anemone", "Anémone Vénéneuse", 60.0, 8.0, 0.0, 11.0, 2.0, Some(ProjKind::Seeds), Special::None, 5, 0.0),
    def!("poison_quill_vine", "Liane à Dards", 70.0, 10.0, 0.0, 12.0, 1.8, Some(ProjKind::Arrow), Special::None, 5, 0.0),
    def!("blastling", "Blastlin", 70.0, 13.0, 2.6, 13.0, 1.9, Some(ProjKind::Magic), Special::Volley(3), 6, 1.0),
    def!("snareling", "Snarelin", 75.0, 12.0, 2.9, 12.0, 1.7, Some(ProjKind::Seeds), Special::None, 6, 1.0),
    def!("cave_crawler", "Rampant des Cavernes", 55.0, 13.0, 3.6, 1.4, 1.0, None, Special::None, 4, 1.0),
    def!("mini_abomination", "Mini-Abomination", 40.0, 10.0, 3.4, 1.3, 0.9, None, Special::None, 3, 1.0),
    def!("magmacube", "Cube de Magma", 60.0, 12.0, 2.4, 1.4, 1.3, None, Special::None, 4, 0.7),
    def!("ghast", "Ghast", 80.0, 18.0, 1.8, 15.0, 2.6, Some(ProjKind::Fire), Special::Volley(2), 8, 1.0),
    def!("icy_creeper", "Creeper Glacé", 46.0, 34.0, 3.0, 2.0, 1.0, None, Special::Explode, 4, 1.3),
    // ------------------------------------------------------------------
    // DLC bosses (batch 2)
    // ------------------------------------------------------------------
    def!("boss_jungle", "Abomination de la Jungle", 1100.0, 26.0, 2.3, 2.4, 1.7, None, Special::Boss(BossKind::JungleAbomination), 90, 0.0),
    def!("boss_wretched", "Spectre Repoussant", 880.0, 20.0, 2.6, 1.8, 1.8, Some(ProjKind::Ice), Special::Boss(BossKind::WretchedWraith), 85, 0.0),
    def!("boss_tempest", "Golem Tempête", 1150.0, 28.0, 2.2, 2.4, 1.7, None, Special::Boss(BossKind::TempestGolem), 90, 0.0),
    def!("boss_ancient", "Gardien Antique", 1000.0, 24.0, 1.6, 13.0, 2.0, Some(ProjKind::Magic), Special::Boss(BossKind::AncientGuardian), 85, 0.0),
    def!("boss_wildfire", "Feu Sauvage", 980.0, 22.0, 2.5, 14.0, 2.2, Some(ProjKind::Fire), Special::Boss(BossKind::Wildfire), 88, 0.0),
    def!("boss_vengeful", "Cœur d'Ender Vengeur", 1300.0, 28.0, 2.7, 1.8, 1.7, Some(ProjKind::Magic), Special::Boss(BossKind::VengefulHeart), 100, 0.0),
    def!("captive", "Villageois captif", 20.0, 0.0, 0.0, 0.0, 99.0, None, Special::None, 0, 0.2),
];

pub fn def_for(kind: &str) -> &'static EnemyDef {
    DEFS.iter().find(|d| d.id == kind).unwrap_or(&DEFS[0])
}

pub struct Enemy {
    pub kind: String,
    pub def: &'static EnemyDef,
    pub yaw: f32,
    pub aggro: bool,
    pub attack_t: f32,
    pub cd: f32,
    pub fuse_t: f32,
    pub charge_t: f32,
    pub charge_cd: f32,
    pub telegraph_t: f32,
    pub summon_cd: f32,
    pub volley_cd: f32,
    pub knock_vel: Vec2,
    pub status: Vec<Status>,
    pub dying_t: f32,
    pub captive: bool,
    pub wander_t: f32,
    pub wander_dir: Vec2,
    pub phase2: bool,
    pub poison_touch: bool,
    pub pending_strike: Option<(usize, f32)>,
}

impl Enemy {
    /// Friendly captive villager entity.
    pub fn captive() -> Enemy {
        let mut e = Enemy::new("captive");
        e.captive = true;
        e
    }

    fn new(kind: &str) -> Enemy {
        let def = def_for(kind);
        Enemy {
            kind: kind.to_string(),
            def,
            yaw: 0.0,
            aggro: false,
            attack_t: 0.0,
            cd: 1.0,
            fuse_t: 0.0,
            charge_t: 0.0,
            charge_cd: 3.0,
            telegraph_t: 0.0,
            summon_cd: 5.0,
            volley_cd: 4.0,
            knock_vel: Vec2::ZERO,
            status: Vec::new(),
            dying_t: DEATH_TIME,
            captive: kind == "captive",
            wander_t: 0.0,
            wander_dir: Vec2::ZERO,
            phase2: false,
            poison_touch: kind == "jungle_zombie" || kind == "spider",
            pending_strike: None,
        }
    }
}

pub fn model_for(kind: &str) -> &'static crate::models::MobModel {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<String, &'static crate::models::MobModel>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut c = cache.lock().unwrap();
    *c.entry(kind.to_string()).or_insert_with(|| Box::leak(Box::new(crate::models::mob_model(kind))))
}

#[derive(Clone, Copy)]
pub struct Attack(pub f32);

pub fn spawn_enemy(world: &mut hecs::World, kind: &str, pos: Vec2, tier: usize) -> Entity {
    let def = def_for(kind);
    let tier = tier.min(2);
    let hp = def.hp * consts::TIER_HP_MULT[tier];
    let atk = def.atk * consts::TIER_ATK_MULT[tier];
    world.spawn((
        Enemy::new(kind),
        Pos(pos),
        Health { hp, max: hp },
        super::Anim(crate::models::AnimState::default()),
        HitFlash(0.0),
        Attack(atk),
    ))
}

enum Action {
    HitPlayer(usize, f32, Vec2),
    Shoot { from: Vec2, to: Vec2, kind: ProjKind, dmg: f32, effects: Vec<Status> },
    Summon { kind: &'static str, count: u32, at: Vec2 },
    Volley { from: Vec2, to: Vec2, kind: ProjKind, dmg: f32, n: u32 },
    Explode { at: Vec2, radius: f32, dmg: f32 },
    SpawnZone { at: Vec2, radius: f32, dps: f32 },
    Die { at: Vec2 },
}

// ----------------------------------------------------------------------
// Update
// ----------------------------------------------------------------------

pub fn update_enemies(game: &mut Game, dt: f32) {
    let snap: Vec<(Entity, Vec2)> = game
        .world
        .query::<(Entity, &Pos, &Health)>()
        .iter()
        .map(|(e, p, _)| (e, p.0))
        .collect();

    let player_pos: Vec<(usize, Vec2)> = game
        .players
        .iter()
        .enumerate()
        .filter(|(_, p)| p.downed_t.is_none())
        .map(|(i, p)| (i, p.pos))
        .collect();

    let entities: Vec<Entity> = game.world.query::<(Entity, &Enemy)>().iter().map(|(e, _)| e).collect();
    let mut to_remove: Vec<Entity> = Vec::new();

    for e in entities {
        let mut actions: Vec<Action> = Vec::new();
        let mut remove_this = false;

        {
            let mut q = game
                .world
                .query_one::<(&mut Enemy, &mut Pos, &mut Health, &Attack, &mut super::Anim)>(e);
            let Ok((en, pos, health, atk, anim)) = q.get() else { continue };

            // dying
            if health.hp <= 0.0 {
                en.dying_t -= dt;
                anim.0.dead = ((DEATH_TIME - en.dying_t) / DEATH_TIME).clamp(0.0, 1.0);
                if en.dying_t <= 0.0 {
                    remove_this = true;
                }
                continue;
            }
            if en.captive {
                anim.0.walk_phase = (game.time * 2.0).sin() * 0.2;
                anim.0.moving = 0.15;
                continue;
            }

            // status
            let mut slow = 1.0;
            for s in en.status.iter_mut() {
                s.t -= dt;
                match s.kind {
                    StatusKind::Poison | StatusKind::Burn => health.hp -= s.dps * dt,
                    StatusKind::Slow => slow *= s.slow_mult,
                }
            }
            en.status.retain(|s| s.t > 0.0);
            if health.hp <= 0.0 {
                continue; // will be handled as death next pass via hp check
            }

            // target
            let Some(&(tidx, tpos)) = player_pos
                .iter()
                .min_by(|a, b| {
                    a.1.distance_squared(pos.0)
                        .partial_cmp(&b.1.distance_squared(pos.0))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            else { continue };
            let dist = pos.0.distance(tpos);
            if !en.aggro && dist < consts::AI_AGGRO_RANGE {
                en.aggro = true;
            }

            let speed = en.def.speed * slow;
            let mut mv = Vec2::ZERO;
            anim.0.moving = 0.0;
            en.cd -= dt;
            en.summon_cd -= dt;
            en.volley_cd -= dt;
            en.charge_cd -= dt;
            en.knock_vel *= (1.0 - dt * 6.0).max(0.0);

            if en.aggro {
                // forward = (-sin yaw, -cos yaw): face the target
                en.yaw = (pos.0.x - tpos.x).atan2(pos.0.y - tpos.y);
            }

            if en.aggro {
                match en.def.special {
                    Special::Explode => {
                        if dist < 2.2 {
                            en.fuse_t += dt;
                            anim.0.fuse = (en.fuse_t / 1.2).min(1.0);
                            if en.fuse_t >= 1.2 {
                                actions.push(Action::Explode { at: pos.0, radius: 2.6, dmg: atk.0 });
                                actions.push(Action::Die { at: pos.0 });
                                remove_this = true;
                            }
                        } else {
                            en.fuse_t = (en.fuse_t - dt * 2.0).max(0.0);
                            if dist > 4.0 {
                                mv = (tpos - pos.0).normalize_or_zero() * speed;
                                anim.0.moving = 1.0;
                            }
                        }
                    }
                    Special::Charge => {
                        if en.charge_t > 0.0 {
                            en.charge_t -= dt;
                            mv = Vec2::new(-en.yaw.sin(), -en.yaw.cos()) * speed * 3.4;
                            anim.0.moving = 1.0;
                            if dist < 1.5 {
                                actions.push(Action::HitPlayer(tidx, atk.0, pos.0));
                                en.charge_t = 0.0;
                            }
                        } else if en.telegraph_t > 0.0 {
                            en.telegraph_t -= dt;
                            anim.0.cast = (1.0 - en.telegraph_t / 0.6).max(0.0);
                            if en.telegraph_t <= 0.0 {
                                en.charge_t = 0.55;
                            }
                        } else if dist < 11.0 && dist > 2.5 && en.charge_cd <= 0.0 {
                            en.telegraph_t = 0.6;
                            en.charge_cd = 5.0;
                            game.particles.spawn_ring(Vec3::new(pos.0.x, 0.2, pos.0.y), 1.2, 8, Vec4::new(1.0, 0.5, 0.2, 0.8));
                        } else if dist > en.def.range {
                            mv = (tpos - pos.0).normalize_or_zero() * speed;
                            anim.0.moving = 1.0;
                        } else {
                            try_melee(en, anim, tidx, atk.0, dist, &mut actions);
                        }
                    }
                    Special::Teleport => {
                        if en.charge_cd <= 0.0 && dist > 3.0 && dist < 18.0 {
                            // blink next to the target, then strike
                            en.charge_cd = 3.2;
                            let dir = (tpos - pos.0).normalize_or_zero();
                            let mut dest = tpos - dir * 1.4;
                            game.level.collide(&mut dest, 0.4);
                            game.particles.spawn_burst(Vec3::new(pos.0.x, 1.0, pos.0.y), 12, Vec4::new(0.55, 0.2, 0.8, 0.9), 3.0);
                            pos.0 = dest;
                            game.particles.spawn_burst(Vec3::new(dest.x, 1.0, dest.y), 12, Vec4::new(0.8, 0.35, 1.0, 0.9), 3.0);
                            en.cd = en.cd.min(0.35);
                            anim.0.cast = 1.0;
                        } else if dist > en.def.range {
                            mv = (tpos - pos.0).normalize_or_zero() * speed;
                            anim.0.moving = 1.0;
                        } else {
                            try_melee(en, anim, tidx, atk.0, dist, &mut actions);
                        }
                    }
                    Special::Summon(kind, count, cd) => {
                        if dist < 5.0 {
                            mv = (pos.0 - tpos).normalize_or_zero() * speed;
                            anim.0.moving = 1.0;
                        } else if dist > 11.0 {
                            mv = (tpos - pos.0).normalize_or_zero() * speed;
                            anim.0.moving = 1.0;
                        }
                        if en.summon_cd <= 0.0 && dist < 14.0 {
                            en.summon_cd = cd;
                            anim.0.cast = 1.0;
                            actions.push(Action::Summon { kind, count, at: pos.0 });
                        }
                        if let Some(pk) = en.def.ranged {
                            if en.cd <= 0.0 && dist < 13.0 {
                                actions.push(Action::Shoot { from: pos.0, to: tpos, kind: pk, dmg: atk.0, effects: Vec::new() });
                                en.cd = en.def.atk_cd;
                                anim.0.swing = 1.0;
                            }
                        }
                    }
                    Special::Volley(n) => {
                        if dist > 7.0 {
                            mv = (tpos - pos.0).normalize_or_zero() * speed;
                            anim.0.moving = 1.0;
                        } else if dist < 4.0 {
                            mv = (pos.0 - tpos).normalize_or_zero() * speed;
                            anim.0.moving = 1.0;
                        }
                        if let Some(pk) = en.def.ranged {
                            if en.cd <= 0.0 && dist < 14.0 {
                                actions.push(Action::Shoot { from: pos.0, to: tpos, kind: pk, dmg: atk.0, effects: witch_effects(en) });
                                en.cd = en.def.atk_cd;
                            }
                            if en.volley_cd <= 0.0 && dist < 15.0 {
                                en.volley_cd = 7.0;
                                anim.0.cast = 1.0;
                                actions.push(Action::Volley { from: pos.0, to: tpos, kind: pk, dmg: atk.0, n });
                            }
                        }
                    }
                    Special::Boss(bk) => {
                        let hp_frac = health.hp / health.max;
                        match bk {
                            BossKind::CorruptedCauldron => {
                                if en.cd <= 0.0 {
                                    en.cd = if hp_frac < 0.5 { 2.8 } else { 4.0 };
                                    actions.push(Action::Volley { from: pos.0, to: tpos, kind: ProjKind::Slime, dmg: atk.0, n: 10 });
                                    anim.0.cast = 1.0;
                                }
                                if en.summon_cd <= 0.0 {
                                    en.summon_cd = 8.0;
                                    actions.push(Action::Summon { kind: "slime", count: 2, at: pos.0 });
                                }
                                if en.volley_cd <= 0.0 {
                                    en.volley_cd = 9.0;
                                    actions.push(Action::SpawnZone { at: tpos, radius: 2.2, dps: atk.0 * 0.5 });
                                }
                            }
                            BossKind::RedstoneMonstrosity => {
                                if dist > en.def.range {
                                    mv = (tpos - pos.0).normalize_or_zero() * en.def.speed * (if hp_frac < 0.5 { 1.3 } else { 1.0 });
                                    anim.0.moving = 1.0;
                                } else {
                                    try_melee(en, anim, tidx, atk.0, dist, &mut actions);
                                }
                                if en.volley_cd <= 0.0 && dist < 5.0 {
                                    en.volley_cd = 7.5;
                                    en.telegraph_t = 0.8;
                                    anim.0.cast = 1.0;
                                    actions.push(Action::Explode { at: pos.0, radius: 4.0, dmg: atk.0 * 1.4 });
                                }
                                if en.charge_cd <= 0.0 && dist < 12.0 && dist > 3.0 {
                                    en.charge_cd = 8.0;
                                    en.telegraph_t = 0.6;
                                    en.charge_t = 0.55;
                                    game.particles.spawn_ring(Vec3::new(pos.0.x, 0.2, pos.0.y), 1.2, 8, Vec4::new(1.0, 0.5, 0.2, 0.8));
                                }
                                if en.charge_t > 0.0 {
                                    en.charge_t -= dt;
                                    mv = Vec2::new(-en.yaw.sin(), -en.yaw.cos()) * en.def.speed * 3.2;
                                    anim.0.moving = 1.0;
                                    if dist < 1.8 {
                                        actions.push(Action::HitPlayer(tidx, atk.0, pos.0));
                                        en.charge_t = 0.0;
                                    }
                                }
                                if en.summon_cd <= 0.0 && (hp_frac < 0.66 || hp_frac < 0.33) {
                                    en.summon_cd = 30.0;
                                    actions.push(Action::Summon { kind: "redstone_golem", count: 1, at: pos.0 });
                                }
                            }
                            BossKind::NamelessOne => {
                                if dist > 8.0 {
                                    mv = (tpos - pos.0).normalize_or_zero() * en.def.speed;
                                    anim.0.moving = 1.0;
                                }
                                if en.cd <= 0.0 && dist < 14.0 {
                                    en.cd = if hp_frac < 0.5 { 1.6 } else { 2.4 };
                                    actions.push(Action::Shoot { from: pos.0, to: tpos, kind: ProjKind::Magic, dmg: atk.0, effects: Vec::new() });
                                }
                                if en.volley_cd <= 0.0 {
                                    en.volley_cd = if hp_frac < 0.5 { 5.0 } else { 7.0 };
                                    actions.push(Action::Volley { from: pos.0, to: tpos, kind: ProjKind::Magic, dmg: atk.0, n: if hp_frac < 0.5 { 8 } else { 5 } });
                                }
                                if en.summon_cd <= 0.0 {
                                    en.summon_cd = if hp_frac < 0.5 { 6.0 } else { 9.0 };
                                    actions.push(Action::Summon { kind: "skeleton", count: 3, at: pos.0 });
                                }
                            }
                            BossKind::ArchIllager => {
                                if en.kind == "boss_arch" {
                                    if dist > en.def.range {
                                        mv = (tpos - pos.0).normalize_or_zero() * en.def.speed;
                                        anim.0.moving = 1.0;
                                    } else {
                                        try_melee(en, anim, tidx, atk.0, dist, &mut actions);
                                    }
                                    if en.cd <= 0.0 && dist < 14.0 {
                                        en.cd = 3.5;
                                        actions.push(Action::Shoot { from: pos.0, to: tpos, kind: ProjKind::Magic, dmg: atk.0, effects: Vec::new() });
                                    }
                                    if en.volley_cd <= 0.0 {
                                        en.volley_cd = 6.0;
                                        actions.push(Action::Volley { from: pos.0, to: tpos, kind: ProjKind::Magic, dmg: atk.0 * 0.8, n: 12 });
                                    }
                                    if en.summon_cd <= 0.0 {
                                        en.summon_cd = 12.0;
                                        actions.push(Action::Summon { kind: "vindicator", count: 2, at: pos.0 });
                                    }
                                } else {
                                    // Heart of Ender (phase 2)
                                    mv = (tpos - pos.0).normalize_or_zero() * en.def.speed;
                                    anim.0.moving = 1.0;
                                    if en.cd <= 0.0 {
                                        en.cd = 2.0;
                                        for k in 0..3u32 {
                                            let spread = (k as f32 - 1.0) * 0.25;
                                            let dir = (tpos - pos.0).normalize_or_zero();
                                            actions.push(Action::Shoot {
                                                from: pos.0,
                                                to: pos.0 + Vec2::new(dir.x + spread, dir.y - spread) * 10.0,
                                                kind: ProjKind::Magic,
                                                dmg: atk.0,
                                                effects: Vec::new(),
                                            });
                                        }
                                    }
                                    if en.volley_cd <= 0.0 {
                                        en.volley_cd = 6.0;
                                        actions.push(Action::SpawnZone { at: tpos, radius: 2.4, dps: atk.0 * 0.6 });
                                    }
                                }
                            }
                            BossKind::MiniGolemHorde => {
                                if dist > en.def.range {
                                    mv = (tpos - pos.0).normalize_or_zero() * en.def.speed;
                                    anim.0.moving = 1.0;
                                } else {
                                    try_melee(en, anim, tidx, atk.0, dist, &mut actions);
                                }
                            }
                            BossKind::JungleAbomination => {
                                if en.charge_t > 0.0 {
                                    en.charge_t -= dt;
                                    mv = Vec2::new(-en.yaw.sin(), -en.yaw.cos()) * en.def.speed * 3.0;
                                    anim.0.moving = 1.0;
                                    if dist < 2.0 {
                                        actions.push(Action::HitPlayer(tidx, atk.0, pos.0));
                                        en.charge_t = 0.0;
                                    }
                                } else if dist > en.def.range {
                                    mv = (tpos - pos.0).normalize_or_zero() * en.def.speed;
                                    anim.0.moving = 1.0;
                                } else {
                                    try_melee(en, anim, tidx, atk.0, dist, &mut actions);
                                }
                                if en.volley_cd <= 0.0 && dist < 6.0 {
                                    en.volley_cd = 7.0;
                                    en.telegraph_t = 0.7;
                                    anim.0.cast = 1.0;
                                    actions.push(Action::Explode { at: pos.0, radius: 4.2, dmg: atk.0 * 1.3 });
                                }
                                if en.charge_cd <= 0.0 && dist < 14.0 && dist > 3.0 {
                                    en.charge_cd = 9.0;
                                    en.telegraph_t = 0.6;
                                    en.charge_t = 0.55;
                                    game.particles.spawn_ring(Vec3::new(pos.0.x, 0.2, pos.0.y), 1.4, 10, Vec4::new(0.3, 0.7, 0.2, 0.8));
                                }
                                if en.summon_cd <= 0.0 && hp_frac < 0.6 {
                                    en.summon_cd = 14.0;
                                    actions.push(Action::Summon { kind: "poison_anemone", count: 2, at: pos.0 });
                                }
                            }
                            BossKind::WretchedWraith => {
                                if dist > 9.0 {
                                    mv = (tpos - pos.0).normalize_or_zero() * en.def.speed;
                                    anim.0.moving = 1.0;
                                } else if dist < 4.0 {
                                    mv = (pos.0 - tpos).normalize_or_zero() * en.def.speed;
                                    anim.0.moving = 1.0;
                                }
                                if en.cd <= 0.0 && dist < 14.0 {
                                    en.cd = if hp_frac < 0.5 { 1.8 } else { 2.6 };
                                    actions.push(Action::Shoot { from: pos.0, to: tpos, kind: ProjKind::Ice, dmg: atk.0, effects: vec![Status::new(StatusKind::Slow, 4.0, 0.55)] });
                                    anim.0.cast = 1.0;
                                }
                                if en.volley_cd <= 0.0 {
                                    en.volley_cd = if hp_frac < 0.5 { 5.5 } else { 8.0 };
                                    actions.push(Action::Volley { from: pos.0, to: tpos, kind: ProjKind::Ice, dmg: atk.0 * 0.8, n: if hp_frac < 0.5 { 7 } else { 4 } });
                                }
                                if en.summon_cd <= 0.0 {
                                    en.summon_cd = 7.0;
                                    actions.push(Action::SpawnZone { at: tpos, radius: 2.4, dps: atk.0 * 0.4 });
                                }
                            }
                            BossKind::TempestGolem => {
                                if en.charge_t > 0.0 {
                                    en.charge_t -= dt;
                                    mv = Vec2::new(-en.yaw.sin(), -en.yaw.cos()) * en.def.speed * 3.0;
                                    anim.0.moving = 1.0;
                                    if dist < 2.0 {
                                        actions.push(Action::HitPlayer(tidx, atk.0, pos.0));
                                        en.charge_t = 0.0;
                                    }
                                } else if dist > en.def.range {
                                    mv = (tpos - pos.0).normalize_or_zero() * en.def.speed;
                                    anim.0.moving = 1.0;
                                } else {
                                    try_melee(en, anim, tidx, atk.0, dist, &mut actions);
                                }
                                if en.volley_cd <= 0.0 && dist < 13.0 {
                                    en.volley_cd = 8.0;
                                    anim.0.cast = 1.0;
                                    actions.push(Action::Volley { from: pos.0, to: tpos, kind: ProjKind::Magic, dmg: atk.0 * 0.7, n: 8 });
                                }
                                if en.charge_cd <= 0.0 && dist < 15.0 && dist > 3.0 {
                                    en.charge_cd = 8.0;
                                    en.telegraph_t = 0.6;
                                    en.charge_t = 0.55;
                                    game.particles.spawn_ring(Vec3::new(pos.0.x, 0.2, pos.0.y), 1.4, 10, Vec4::new(0.5, 0.8, 1.0, 0.8));
                                }
                                if en.summon_cd <= 0.0 && hp_frac < 0.4 {
                                    en.summon_cd = 20.0;
                                    actions.push(Action::Summon { kind: "squall_golem", count: 1, at: pos.0 });
                                }
                            }
                            BossKind::AncientGuardian => {
                                if dist < 5.0 {
                                    mv = (pos.0 - tpos).normalize_or_zero() * en.def.speed;
                                    anim.0.moving = 1.0;
                                } else if dist > 10.0 {
                                    mv = (tpos - pos.0).normalize_or_zero() * en.def.speed;
                                    anim.0.moving = 1.0;
                                }
                                if en.cd <= 0.0 && dist < 15.0 {
                                    en.cd = if hp_frac < 0.5 { 1.4 } else { 2.1 };
                                    actions.push(Action::Shoot { from: pos.0, to: tpos, kind: ProjKind::Magic, dmg: atk.0, effects: Vec::new() });
                                    anim.0.cast = 1.0;
                                }
                                if en.volley_cd <= 0.0 && dist < 16.0 {
                                    en.volley_cd = if hp_frac < 0.5 { 4.5 } else { 6.5 };
                                    actions.push(Action::Volley { from: pos.0, to: tpos, kind: ProjKind::Magic, dmg: atk.0 * 0.8, n: 3 });
                                }
                                if en.summon_cd <= 0.0 {
                                    en.summon_cd = 8.0;
                                    actions.push(Action::SpawnZone { at: tpos, radius: 2.0, dps: atk.0 * 0.6 });
                                }
                                if en.charge_cd <= 0.0 && hp_frac < 0.5 && dist < 14.0 && dist > 3.0 {
                                    en.charge_cd = 10.0;
                                    en.telegraph_t = 0.6;
                                    en.charge_t = 0.55;
                                    game.particles.spawn_ring(Vec3::new(pos.0.x, 0.2, pos.0.y), 1.2, 8, Vec4::new(0.9, 0.3, 0.9, 0.8));
                                }
                                if en.charge_t > 0.0 {
                                    en.charge_t -= dt;
                                    mv = Vec2::new(-en.yaw.sin(), -en.yaw.cos()) * en.def.speed * 2.6;
                                    anim.0.moving = 1.0;
                                    if dist < 1.8 {
                                        actions.push(Action::HitPlayer(tidx, atk.0, pos.0));
                                        en.charge_t = 0.0;
                                    }
                                }
                            }
                            BossKind::Wildfire => {
                                if dist > 10.0 {
                                    mv = (tpos - pos.0).normalize_or_zero() * en.def.speed;
                                    anim.0.moving = 1.0;
                                } else if dist < 5.0 {
                                    mv = (pos.0 - tpos).normalize_or_zero() * en.def.speed;
                                    anim.0.moving = 1.0;
                                }
                                if en.cd <= 0.0 && dist < 16.0 {
                                    en.cd = if hp_frac < 0.5 { 1.5 } else { 2.2 };
                                    actions.push(Action::Shoot { from: pos.0, to: tpos, kind: ProjKind::Fire, dmg: atk.0, effects: Vec::new() });
                                    anim.0.cast = 1.0;
                                }
                                if en.volley_cd <= 0.0 {
                                    en.volley_cd = if hp_frac < 0.5 { 4.5 } else { 7.0 };
                                    actions.push(Action::Volley { from: pos.0, to: tpos, kind: ProjKind::Fire, dmg: atk.0 * 0.75, n: if hp_frac < 0.5 { 8 } else { 6 } });
                                }
                                if en.summon_cd <= 0.0 {
                                    en.summon_cd = 11.0;
                                    actions.push(Action::Summon { kind: "blaze", count: 2, at: pos.0 });
                                }
                                if en.charge_cd <= 0.0 && dist < 4.0 {
                                    en.charge_cd = 10.0;
                                    anim.0.cast = 1.0;
                                    actions.push(Action::Explode { at: pos.0, radius: 4.0, dmg: atk.0 * 1.2 });
                                }
                            }
                            BossKind::VengefulHeart => {
                                mv = (tpos - pos.0).normalize_or_zero() * en.def.speed;
                                anim.0.moving = 1.0;
                                if en.cd <= 0.0 {
                                    en.cd = if hp_frac < 0.5 { 1.5 } else { 2.0 };
                                    for k in 0..3u32 {
                                        let spread = (k as f32 - 1.0) * 0.28;
                                        let dir = (tpos - pos.0).normalize_or_zero();
                                        actions.push(Action::Shoot {
                                            from: pos.0,
                                            to: pos.0 + Vec2::new(dir.x + spread, dir.y - spread) * 10.0,
                                            kind: ProjKind::Magic,
                                            dmg: atk.0,
                                            effects: Vec::new(),
                                        });
                                    }
                                    anim.0.cast = 1.0;
                                }
                                if en.volley_cd <= 0.0 {
                                    en.volley_cd = if hp_frac < 0.33 { 4.0 } else { 6.0 };
                                    actions.push(Action::Volley { from: pos.0, to: tpos, kind: ProjKind::Magic, dmg: atk.0 * 0.7, n: if hp_frac < 0.33 { 14 } else { 8 } });
                                }
                                if en.summon_cd <= 0.0 {
                                    en.summon_cd = if hp_frac < 0.66 { 12.0 } else { 18.0 };
                                    if hp_frac < 0.66 {
                                        actions.push(Action::Summon { kind: "endling", count: 2, at: pos.0 });
                                    } else {
                                        actions.push(Action::SpawnZone { at: tpos, radius: 2.6, dps: atk.0 * 0.6 });
                                    }
                                }
                            }
                        }
                    }
                    Special::None => {
                        if let Some(pk) = en.def.ranged {
                            if dist < 6.0 {
                                mv = (pos.0 - tpos).normalize_or_zero() * speed;
                                anim.0.moving = 1.0;
                            } else if dist > 11.0 {
                                mv = (tpos - pos.0).normalize_or_zero() * speed;
                                anim.0.moving = 1.0;
                            }
                            if en.cd <= 0.0 && dist < 13.0 {
                                actions.push(Action::Shoot { from: pos.0, to: tpos, kind: pk, dmg: atk.0, effects: witch_effects(en) });
                                en.cd = en.def.atk_cd;
                                anim.0.swing = 1.0;
                            }
                        } else if dist > en.def.range {
                            mv = (tpos - pos.0).normalize_or_zero() * speed;
                            anim.0.moving = 1.0;
                        } else {
                            try_melee(en, anim, tidx, atk.0, dist, &mut actions);
                        }
                    }
                }
            } else {
                en.wander_t -= dt;
                if en.wander_t <= 0.0 {
                    en.wander_t = 2.0 + rand::thread_rng().gen::<f32>() * 2.0;
                    let a = rand::thread_rng().gen_range(0.0..std::f32::consts::TAU);
                    en.wander_dir = Vec2::new(a.cos(), a.sin());
                }
                mv = en.wander_dir * speed * 0.25;
                en.yaw = (-mv.x).atan2(-mv.y);
                anim.0.moving = 0.3;
            }

            // integrate movement
            let mut newpos = pos.0 + (mv + en.knock_vel) * dt;
            game.level.collide(&mut newpos, 0.4);
            for (oe, opos) in &snap {
                if *oe == e { continue; }
                let d = newpos.distance(*opos);
                if d < 0.8 && d > 0.001 {
                    newpos += (newpos - *opos) / d * (0.8 - d) * 0.5;
                }
            }
            pos.0 = newpos;
            anim.0.walk_phase += mv.length() * dt * 2.2;
            anim.0.swing = if en.attack_t > 0.0 {
                let v = 1.0 - en.attack_t / 0.35;
                en.attack_t -= dt;
                if en.attack_t <= 0.0 {
                    if let Some((idx, dmg)) = en.pending_strike.take() {
                        if game.players.get(idx).map(|p| p.pos.distance(newpos) < en.def.range + 0.9).unwrap_or(false) {
                            actions.push(Action::HitPlayer(idx, dmg, newpos));
                            if en.poison_touch {
                                actions.push(Action::HitPlayer(idx, 0.0, newpos)); // 0 dmg = poison tick marker
                            }
                        }
                    }
                }
                v.max(0.0)
            } else {
                0.0
            };
            anim.0.cast = (anim.0.cast - dt * 2.0).max(0.0);
        }

        if remove_this {
            to_remove.push(e);
        }

        // execute actions after dropping hecs access
        for a in actions {
            match a {
                Action::HitPlayer(idx, dmg, from) => {
                    if dmg > 0.0 {
                        combat::damage_player(game, idx, dmg, from);
                    } else if let Some(p) = game.players.get_mut(idx) {
                        // poison touch
                        p.status.push(Status::new(StatusKind::Poison, 5.0, 3.0));
                    }
                }
                Action::Shoot { from, to, kind, dmg, effects } => {
                    let dir = (to - from).normalize_or_zero();
                    combat::spawn_projectile(game, Projectile {
                        pos: from + dir * 0.6,
                        vel: dir * consts::PROJECTILE_SPEED,
                        team: Team::Enemy,
                        dmg,
                        kind,
                        life: consts::PROJECTILE_LIFE,
                        radius: 0.3,
                        effects,
                        explode: None,
                    });
                }
                Action::Summon { kind, count, at } => {
                    let mut rng = rand::thread_rng();
                    for _ in 0..count {
                        let sp = at + Vec2::new(rng.gen_range(-2.0..2.0), rng.gen_range(-2.0..2.0));
                        spawn_enemy(&mut game.world, kind, sp, game.tier);
                        game.particles.spawn_burst(Vec3::new(sp.x, 0.6, sp.y), 10, Vec4::new(0.5, 0.3, 0.7, 0.9), 2.5);
                    }
                }
                Action::Volley { from, to, kind, dmg, n } => {
                    let dir = (to - from).normalize_or_zero();
                    let mut rng = rand::thread_rng();
                    for _ in 0..n {
                        let spread = Vec2::new(dir.y, -dir.x) * rng.gen_range(-0.6..0.6);
                        combat::spawn_projectile(game, Projectile {
                            pos: from,
                            vel: (dir + spread).normalize_or_zero() * consts::PROJECTILE_SPEED * 1.4,
                            team: Team::Enemy,
                            dmg,
                            kind,
                            life: consts::PROJECTILE_LIFE,
                            radius: 0.3,
                            effects: Vec::new(),
                            explode: None,
                        });
                    }
                }
                Action::Explode { at, radius, dmg } => {
                    combat::explode_at(game, at, radius, dmg, Team::Enemy);
                }
                Action::SpawnZone { at, radius, dps } => {
                    game.zones.push(combat::Zone {
                        pos: at,
                        radius,
                        dps,
                        slow: false,
                        t: 5.0,
                        team: Team::Enemy,
                        color: Vec4::new(0.4, 0.2, 0.6, 0.6),
                    });
                }
                Action::Die { at } => {
                    game.particles.spawn_burst(Vec3::new(at.x, 0.9, at.y), 20, Vec4::new(0.3, 0.6, 0.3, 0.9), 4.0);
                }
            }
        }
    }

    for e in to_remove {
        let _ = game.world.despawn(e);
    }
}

fn witch_effects(en: &Enemy) -> Vec<Status> {
    match en.kind.as_str() {
        "witch" => vec![Status::new(StatusKind::Poison, 6.0, 3.0)],
        "snareling" => vec![Status::new(StatusKind::Slow, 4.0, 0.5)],
        "mossy_skeleton" | "poison_anemone" | "whisperer" => {
            vec![Status::new(StatusKind::Poison, 4.0, 2.0)]
        }
        _ => Vec::new(),
    }
}

fn try_melee(en: &mut Enemy, anim: &mut super::Anim, tidx: usize, atk: f32, dist: f32, _actions: &mut Vec<Action>) {
    if en.cd <= 0.0 && dist < en.def.range + 0.4 {
        en.cd = en.def.atk_cd;
        en.attack_t = 0.35;
        en.pending_strike = Some((tidx, atk));
        anim.0.swing = 0.01;
    }
}

