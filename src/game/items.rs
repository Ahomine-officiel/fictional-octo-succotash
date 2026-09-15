//! Item definitions, generation and enchantments.
//! Balance follows Minecraft Dungeons conventions: power level drives stats,
//! rarity gives enchant slots, uniques carry names and a stat bonus.

use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Slot {
    Melee,
    Ranged,
    Armor,
    Artifact,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum MeleeClass {
    Sword,
    Axe,
    Hammer,
    Dagger,
    Gauntlet,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum RangedClass {
    Bow,
    Crossbow,
    HeavyCrossbow,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ArmorClass {
    Light,
    Medium,
    Heavy,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ArtifactKind {
    Lightning,
    Totem,
    WindHorn,
    Harvester,
    Fireworks,
    Seeds,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ItemClass {
    M(MeleeClass),
    R(RangedClass),
    A(ArmorClass),
    Art(ArtifactKind),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Rarity {
    Common,
    Rare,
    Unique,
}

impl Rarity {
    pub fn slots(&self) -> usize {
        match self {
            Rarity::Common => 1,
            Rarity::Rare => 2,
            Rarity::Unique => 3,
        }
    }
    pub fn color(&self) -> [f32; 4] {
        match self {
            Rarity::Common => [0.85, 0.85, 0.85, 1.0],
            Rarity::Rare => [0.5, 0.75, 1.0, 1.0],
            Rarity::Unique => [1.0, 0.75, 0.25, 1.0],
        }
    }
    pub fn fr(&self) -> &'static str {
        match self {
            Rarity::Common => "Commun",
            Rarity::Rare => "Rare",
            Rarity::Unique => "Unique",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Enchant {
    Sharpness,
    CriticalHit,
    Swirling,
    Thundering,
    Radiance,
    FireAspect,
    PoisonCloud,
    Gravity,
}

impl Enchant {
    pub fn name_fr(&self) -> &'static str {
        match self {
            Enchant::Sharpness => "Affûtage",
            Enchant::CriticalHit => "Coup critique",
            Enchant::Swirling => "Tourbillon",
            Enchant::Thundering => "Fulguration",
            Enchant::Radiance => "Éclat",
            Enchant::FireAspect => "Aspect ardent",
            Enchant::PoisonCloud => "Nuage poison",
            Enchant::Gravity => "Gravité",
        }
    }
    pub fn desc_fr(&self, tier: u8) -> String {
        match self {
            Enchant::Sharpness => format!("+{}% dégâts de mêlée", tier * 10),
            Enchant::CriticalHit => format!("+{}% chance critique", tier * 5),
            Enchant::Swirling => format!("Combo 3 : rafale de zone ({} dégâts)", 20 * tier as u32),
            Enchant::Thundering => format!("Critique mêlée : foudre ({} dégâts)", 25 * tier as u32),
            Enchant::Radiance => format!("{}% de chances de soigner 4 PV par coup", tier * 5),
            Enchant::FireAspect => format!("Chance d'enflammer ({}/s pendant 2s)", 6 * tier as u32),
            Enchant::PoisonCloud => format!("{}% de chances de nuage poison au tir", tier * 10),
            Enchant::Gravity => format!("{}% de chances d'aspirer les ennemis à l'impact", tier * 10),
        }
    }
    /// enchantment point cost per tier (1/2/3 like MCD)
    pub fn cost(tier: u8) -> u32 {
        tier as u32
    }
    pub fn applies_to(&self, slot: Slot) -> bool {
        match self {
            Enchant::Sharpness | Enchant::CriticalHit | Enchant::Swirling | Enchant::Thundering | Enchant::Radiance | Enchant::FireAspect => slot == Slot::Melee,
            Enchant::PoisonCloud | Enchant::Gravity => slot == Slot::Ranged,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub name: String,
    pub class: ItemClass,
    pub power: u32,
    pub rarity: Rarity,
    pub enchants: Vec<(Enchant, u8)>, // (enchant, tier 1..=3)
}

// ----------------------------------------------------------------------
// Class tables (wiki-calibrated melee/ranged/armor archetypes)
// ----------------------------------------------------------------------

pub struct MeleeDef {
    pub name_fr: &'static str,
    pub dmg: f32,
    pub speed: f32,   // attacks per second
    pub reach: f32,
    pub knockback: f32,
    pub icon: &'static str,
}

pub const MELEE: [(MeleeClass, MeleeDef); 5] = [
    (MeleeClass::Sword, MeleeDef { name_fr: "Épée", dmg: 34.0, speed: 2.4, reach: 2.0, knockback: 1.0, icon: "icon_sword" }),
    (MeleeClass::Axe, MeleeDef { name_fr: "Hache", dmg: 45.0, speed: 1.9, reach: 2.0, knockback: 1.3, icon: "icon_axe" }),
    (MeleeClass::Hammer, MeleeDef { name_fr: "Marteau", dmg: 62.0, speed: 1.45, reach: 2.1, knockback: 2.2, icon: "icon_hammer" }),
    (MeleeClass::Dagger, MeleeDef { name_fr: "Dague", dmg: 23.0, speed: 3.3, reach: 1.7, knockback: 0.6, icon: "icon_dagger" }),
    (MeleeClass::Gauntlet, MeleeDef { name_fr: "Poing d'acier", dmg: 19.0, speed: 3.8, reach: 1.6, knockback: 0.5, icon: "icon_gauntlet" }),
];

pub struct RangedDef {
    pub name_fr: &'static str,
    pub dmg: f32,
    pub speed: f32,
    pub icon: &'static str,
}

pub const RANGED: [(RangedClass, RangedDef); 3] = [
    (RangedClass::Bow, RangedDef { name_fr: "Arc", dmg: 27.0, speed: 1.25, icon: "icon_bow" }),
    (RangedClass::Crossbow, RangedDef { name_fr: "Arbalète", dmg: 36.0, speed: 0.95, icon: "icon_crossbow" }),
    (RangedClass::HeavyCrossbow, RangedDef { name_fr: "Arbalète lourde", dmg: 55.0, speed: 0.62, icon: "icon_crossbow" }),
];

pub struct ArmorDef {
    pub name_fr: &'static str,
    pub hp: f32,
    pub move_mult: f32,
    pub icon: &'static str,
}

pub const ARMOR: [(ArmorClass, ArmorDef); 3] = [
    (ArmorClass::Light, ArmorDef { name_fr: "Armure légère", hp: 34.0, move_mult: 1.09, icon: "icon_armor_light" }),
    (ArmorClass::Medium, ArmorDef { name_fr: "Armure de maille", hp: 62.0, move_mult: 1.0, icon: "icon_armor_medium" }),
    (ArmorClass::Heavy, ArmorDef { name_fr: "Armure lourde", hp: 98.0, move_mult: 0.9, icon: "icon_armor_heavy" }),
];

pub struct ArtifactDef {
    pub name_fr: &'static str,
    pub cd: f32,
    pub icon: &'static str,
    pub desc_fr: &'static str,
}

pub const ARTIFACTS: [(ArtifactKind, ArtifactDef); 6] = [
    (ArtifactKind::Lightning, ArtifactDef { name_fr: "Fulguro-lance", cd: 20.0, icon: "art_lightning", desc_fr: "Éclairs en chaîne sur 4 ennemis" }),
    (ArtifactKind::Totem, ArtifactDef { name_fr: "Totem de régénération", cd: 25.0, icon: "art_totem", desc_fr: "Aura de soin pendant 12 s" }),
    (ArtifactKind::WindHorn, ArtifactDef { name_fr: "Corne de vent", cd: 12.0, icon: "art_wind", desc_fr: "Repousse violemment les ennemis" }),
    (ArtifactKind::Harvester, ArtifactDef { name_fr: "Carquois récolteur", cd: 30.0, icon: "art_harvester", desc_fr: "Tir multiple pendant 5 s" }),
    (ArtifactKind::Fireworks, ArtifactDef { name_fr: "Flèche feu d'artifice", cd: 30.0, icon: "art_fireworks", desc_fr: "Projectile explosif de zone" }),
    (ArtifactKind::Seeds, ArtifactDef { name_fr: "Graines corrompues", cd: 18.0, icon: "art_seeds", desc_fr: "Zone qui empoisonne et ralentit" }),
];

pub fn unique_name(class: &ItemClass) -> Option<&'static str> {
    use ItemClass::*;
    Some(match class {
        M(MeleeClass::Sword) => "Faucon de marque",
        M(MeleeClass::Axe) => "Veuve joyeuse",
        M(MeleeClass::Hammer) => "Masse des templiers",
        M(MeleeClass::Dagger) => "Crocs de l'ombre",
        M(MeleeClass::Gauntlet) => "Poings du sphinx",
        R(RangedClass::Bow) => "Arc du gardien",
        R(RangedClass::Crossbow) => "Arbalète du foudre",
        R(RangedClass::HeavyCrossbow) => "Broyeuse d'os",
        A(ArmorClass::Light) => "Écaille du Mercure",
        A(ArmorClass::Medium) => "Cuirasse d'Adamant",
        A(ArmorClass::Heavy) => "Carapace du Tyran",
        Art(ArtifactKind::Lightning) => "Sceptre des tempêtes",
        Art(ArtifactKind::Totem) => "Totem d'éternité",
        Art(ArtifactKind::WindHorn) => "Souffle des cieux",
        Art(ArtifactKind::Harvester) => "Gibecière sans fin",
        Art(ArtifactKind::Fireworks) => "Fusée du solstice",
        Art(ArtifactKind::Seeds) => "Poche du verdoyant",
    })
}

pub fn melee_def(c: MeleeClass) -> &'static MeleeDef {
    &MELEE.iter().find(|(k, _)| *k == c).unwrap().1
}
pub fn ranged_def(c: RangedClass) -> &'static RangedDef {
    &RANGED.iter().find(|(k, _)| *k == c).unwrap().1
}
pub fn armor_def(c: ArmorClass) -> &'static ArmorDef {
    &ARMOR.iter().find(|(k, _)| *k == c).unwrap().1
}
pub fn artifact_def(k: ArtifactKind) -> &'static ArtifactDef {
    &ARTIFACTS.iter().find(|(a, _)| *a == k).unwrap().1
}

impl Item {
    pub fn slot(&self) -> Slot {
        match self.class {
            ItemClass::M(_) => Slot::Melee,
            ItemClass::R(_) => Slot::Ranged,
            ItemClass::A(_) => Slot::Armor,
            ItemClass::Art(_) => Slot::Artifact,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self.class {
            ItemClass::M(c) => melee_def(c).icon,
            ItemClass::R(c) => ranged_def(c).icon,
            ItemClass::A(c) => armor_def(c).icon,
            ItemClass::Art(k) => artifact_def(k).icon,
        }
    }

    pub fn base_name(&self) -> &'static str {
        match self.class {
            ItemClass::M(c) => melee_def(c).name_fr,
            ItemClass::R(c) => ranged_def(c).name_fr,
            ItemClass::A(c) => armor_def(c).name_fr,
            ItemClass::Art(k) => artifact_def(k).name_fr,
        }
    }

    pub fn unique_mult(&self) -> f32 {
        if self.rarity == Rarity::Unique { 1.2 } else { 1.0 }
    }

    pub fn melee_dmg(&self) -> f32 {
        match self.class {
            ItemClass::M(c) => melee_def(c).dmg * crate::consts::power_damage_scale(self.power) * self.unique_mult(),
            _ => 0.0,
        }
    }
    pub fn melee_speed(&self) -> f32 {
        match self.class {
            ItemClass::M(c) => melee_def(c).speed,
            _ => 2.0,
        }
    }
    pub fn melee_reach(&self) -> f32 {
        match self.class {
            ItemClass::M(c) => melee_def(c).reach,
            _ => 2.0,
        }
    }
    pub fn melee_knockback(&self) -> f32 {
        match self.class {
            ItemClass::M(c) => melee_def(c).knockback,
            _ => 1.0,
        }
    }
    pub fn ranged_dmg(&self) -> f32 {
        match self.class {
            ItemClass::R(c) => ranged_def(c).dmg * crate::consts::power_damage_scale(self.power) * self.unique_mult(),
            _ => 0.0,
        }
    }
    pub fn ranged_speed(&self) -> f32 {
        match self.class {
            ItemClass::R(c) => ranged_def(c).speed,
            _ => 1.0,
        }
    }
    pub fn armor_hp(&self) -> f32 {
        match self.class {
            ItemClass::A(c) => armor_def(c).hp * crate::consts::power_damage_scale(self.power) * self.unique_mult(),
            _ => 0.0,
        }
    }
    pub fn armor_move(&self) -> f32 {
        match self.class {
            ItemClass::A(c) => armor_def(c).move_mult,
            _ => 1.0,
        }
    }

    pub fn ench_level(&self, e: Enchant) -> u8 {
        self.enchants.iter().find(|(en, _)| *en == e).map(|(_, t)| *t).unwrap_or(0)
    }

    /// Shop / salvage price in emeralds.
    pub fn price(&self) -> u32 {
        let base = crate::consts::FORGE_COST_RARITY[
            match self.rarity { Rarity::Common => 0, Rarity::Rare => 1, Rarity::Unique => 2 }
        ];
        (base as f32 * (1.0 + self.power as f32 / 10.0)) as u32
    }
}

/// Random item generation (drops / shop).
pub fn gen_item(rng: &mut impl Rng, power: u32, luck: f32) -> Item {
    let slot_roll: f32 = rng.gen();
    let class = if slot_roll < 0.30 {
        ItemClass::M(match rng.gen_range(0..5) {
            0 => MeleeClass::Sword,
            1 => MeleeClass::Axe,
            2 => MeleeClass::Hammer,
            3 => MeleeClass::Dagger,
            _ => MeleeClass::Gauntlet,
        })
    } else if slot_roll < 0.55 {
        ItemClass::R(match rng.gen_range(0..3) {
            0 => RangedClass::Bow,
            1 => RangedClass::Crossbow,
            _ => RangedClass::HeavyCrossbow,
        })
    } else if slot_roll < 0.85 {
        ItemClass::A(match rng.gen_range(0..3) {
            0 => ArmorClass::Light,
            1 => ArmorClass::Medium,
            _ => ArmorClass::Heavy,
        })
    } else {
        ItemClass::Art(match rng.gen_range(0..6) {
            0 => ArtifactKind::Lightning,
            1 => ArtifactKind::Totem,
            2 => ArtifactKind::WindHorn,
            3 => ArtifactKind::Harvester,
            4 => ArtifactKind::Fireworks,
            _ => ArtifactKind::Seeds,
        })
    };
    let rarity_roll: f32 = rng.gen::<f32>() - luck;
    let rarity = if rarity_roll < 0.04 {
        Rarity::Unique
    } else if rarity_roll < 0.26 {
        Rarity::Rare
    } else {
        Rarity::Common
    };
    let name = if rarity == Rarity::Unique {
        unique_name(&class).unwrap_or("Relique").to_string()
    } else {
        String::new() // resolved at display: base_name
    };
    Item {
        name,
        class,
        power: power.max(1),
        rarity,
        enchants: Vec::new(),
    }
}
