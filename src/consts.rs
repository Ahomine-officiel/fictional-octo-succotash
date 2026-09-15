//! Balance constants calibrated on Minecraft Dungeons wiki data.
//! Sources: minecraft.wiki / minecraft.fandom.com (Dungeons sections)
//!  - Hero base HP = 100
//!  - Zombie HP: Tier1=55, Tier2=210, Tier3=500 ; attack: 70/85/100
//!  - Artifact cooldowns ~12-30s (harpoon quiver 30s, fireworks 30s)
//!  - Health potion: 3 per mission, ~25s cooldown
//!  - Roll: ~0.5s i-frames, ~2s cooldown
//!  - Enchantment points: 1 per hero level

use glam::Vec3;

// ---------------- Camera ----------------
pub const CAM_PITCH_DEG: f32 = 58.0;
pub const CAM_YAW_DEG: f32 = 45.0;
pub const CAM_DIST: f32 = 12.5;
pub const CAM_FOV_DEG: f32 = 46.0;
pub const CAM_SMOOTH: f32 = 6.0;

// ---------------- Player ----------------
pub const HERO_BASE_HP: f32 = 100.0;
pub const PLAYER_SPEED: f32 = 6.2;
pub const PLAYER_RADIUS: f32 = 0.42;
/// Seconds of invulnerability while rolling
pub const ROLL_TIME: f32 = 0.5;
pub const ROLL_COOLDOWN: f32 = 2.0;
pub const ROLL_SPEED_MULT: f32 = 2.1;
/// Melee swing duration (s); hit lands at SWING_HIT_T
pub const SWING_TIME: f32 = 0.38;
pub const SWING_HIT_T: f32 = 0.15;
pub const MELEE_ARC_DEG: f32 = 95.0;
pub const BASE_CRIT_CHANCE: f32 = 0.10;
pub const CRIT_MULT: f32 = 2.0;
pub const COMBO_STEP_MULT: f32 = 0.08;
pub const COMBO_MAX: u32 = 10;
/// Arrows start / drops
pub const START_ARROWS: u32 = 60;
pub const ARROW_SPEED: f32 = 24.0;
/// Health potions
pub const POTIONS_PER_MISSION: u32 = 3;
pub const POTION_COOLDOWN: f32 = 25.0;
pub const POTION_HEAL_FRAC: f32 = 0.75;
/// Downed / respawn in co-op
pub const DOWNED_TIME: f32 = 12.0;
/// I-frames after taking a hit (soft stagger protection)
pub const HIT_IFRAME: f32 = 0.35;
/// Interaction radius (chests, captives, levers, portal, shop bell)
pub const INTERACT_RANGE: f32 = 1.8;

// ---------------- Threat tiers (Default / Adventure / Apocalypse) ----------------
/// HP multipliers derived from wiki Zombie tiers: 55 -> 210 -> 500
pub const TIER_HP_MULT: [f32; 3] = [1.0, 3.8, 9.1];
/// Attack multipliers: 70 -> 85 -> 100
pub const TIER_ATK_MULT: [f32; 3] = [1.0, 1.21, 1.43];
/// Rééquilibrage « c'était trop facile » : les packs frappent plus fort et
/// encaissent mieux (multiplie les stats wiki de base).
pub const ENEMY_HP_MULT: f32 = 1.25;
pub const ENEMY_ATK_MULT: f32 = 1.35;

// ---------------- XP / progression ----------------
/// XP needed for level n -> n+1
pub fn xp_for_level(level: u32) -> u32 {
    20 + level * 14
}
pub const ENCHANT_POINT_PER_LEVEL: u32 = 1;
pub const MAX_DISPLAY_LEVEL: u32 = 55;

// ---------------- Items ----------------
/// Item power = mission threat level + jitter; shop sells power+1..+2
pub const SHOP_POWER_BONUS: u32 = 1;
/// Damage of a melee weapon at power 1, scaled by power curve
pub fn power_damage_scale(power: u32) -> f32 {
    1.0 + 0.18 * (power.saturating_sub(1)) as f32
}
pub fn power_hp_scale(power: u32) -> f32 {
    6.0 + 2.2 * power as f32
}

// ---------------- Economy ----------------
pub const EMERALD_DROP_MIN: u32 = 1;
pub const EMERALD_DROP_MAX: u32 = 4;
pub const CHEST_EMERALDS: (u32, u32) = (10, 28);
pub const SALVAGE_PCT: f32 = 0.35;
/// Blacksmith upgrade cost = base cost per rarity * (current power)
pub const FORGE_COST_RARITY: [u32; 3] = [60, 110, 190];

// ---------------- Combat ----------------
pub const KNOCKBACK_BASE: f32 = 4.5;
pub const STAGGER_BIG_HIT: f32 = 0.28; // seconds of stagger when a heavy hit lands
pub const DAMAGE_NUMBER_LIFE: f32 = 0.9;

// ---------------- AI ----------------
pub const AI_AGGRO_RANGE: f32 = 11.0;
pub const AI_LEASH_RANGE: f32 = 26.0;
pub const PROJECTILE_SPEED: f32 = 9.5;
pub const PROJECTILE_LIFE: f32 = 3.5;

// ---------------- World ----------------
pub const LEVEL_W: u32 = 76;
pub const LEVEL_H: u32 = 76;
pub const BLOCK: f32 = 1.0;

// ---------------- Audio (stub for v1) ----------------
pub const AUDIO_ENABLED: bool = true;

// ---------------- Lighting per-biome defaults live in world::gen ----------------
pub const LIGHT_DIR: Vec3 = Vec3::new(0.45, -0.85, 0.28);
pub const AMBIENT: f32 = 0.52;

// UI
pub const UI_SCALE: f32 = 1.0;
