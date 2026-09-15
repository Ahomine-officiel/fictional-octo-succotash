//! Mission tree — faithful to Minecraft Dungeons campaign order (private use only).
//! Tree per the design doc:
//!   Creeper Woods -> Pumpkin Pastures -> Cacti Canyon
//!                          v
//!                    Soggy Swamp -> Soggy Cave (secret)
//!                          v
//!                    Redstone Mines
//!                          v
//!                    Desert Temple -> Arch Haven (secret)
//!                          v
//!                    Dingy Jungle
//!                          v
//!                    Stronghold
//!                          v
//!                    Nether Wastes
//!                          v
//!                    Obsidian Pinnacle
//!                          v
//!        DLC islands: Hiver Grondant / Pics Hurleurs / Profondeurs / Écho du Vide

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Biome {
    Forest,
    Plains,
    Canyon,
    Swamp,
    Cave,
    Mines,
    Desert,
    Haven,
    Jungle,
    Stronghold,
    Nether,
    Pinnacle,
    Winter,
    Peaks,
    Depths,
    Void,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum BossKind {
    CorruptedCauldron,
    RedstoneMonstrosity,
    NamelessOne,
    ArchIllager,
    MiniGolemHorde, // finale wave instead of a real boss
    JungleAbomination,
    WretchedWraith,
    TempestGolem,
    AncientGuardian,
    Wildfire,
    VengefulHeart,
}

#[derive(Clone, Copy)]
pub struct MissionDef {
    pub id: usize,
    pub name: &'static str,
    pub biome: Biome,
    /// threat level drives loot power + enemy scaling (like MCD power levels)
    pub threat: u32,
    pub boss: Option<BossKind>,
    pub is_secret: bool,
    /// mission ids that must be completed to unlock (secrets also need the rune found)
    pub requires: &'static [usize],
}

pub const MISSIONS: &[MissionDef] = &[
    MissionDef { id: 0, name: "Creeper Woods",     biome: Biome::Forest,     threat: 2,  boss: None,                            is_secret: false, requires: &[] },
    MissionDef { id: 1, name: "Pumpkin Pastures",  biome: Biome::Plains,     threat: 4,  boss: None,                            is_secret: false, requires: &[0] },
    MissionDef { id: 2, name: "Cacti Canyon",      biome: Biome::Canyon,     threat: 6,  boss: None,                            is_secret: false, requires: &[1] },
    MissionDef { id: 3, name: "Soggy Swamp",       biome: Biome::Swamp,      threat: 8,  boss: Some(BossKind::CorruptedCauldron), is_secret: false, requires: &[2] },
    MissionDef { id: 4, name: "Soggy Cave",        biome: Biome::Cave,       threat: 9,  boss: Some(BossKind::MiniGolemHorde),  is_secret: true,  requires: &[3] },
    MissionDef { id: 5, name: "Redstone Mines",    biome: Biome::Mines,      threat: 11, boss: Some(BossKind::RedstoneMonstrosity), is_secret: false, requires: &[3] },
    MissionDef { id: 6, name: "Desert Temple",     biome: Biome::Desert,     threat: 13, boss: Some(BossKind::NamelessOne),     is_secret: false, requires: &[5] },
    MissionDef { id: 7, name: "Arch Haven",        biome: Biome::Haven,      threat: 14, boss: None,                            is_secret: true,  requires: &[6] },
    MissionDef { id: 8, name: "Dingy Jungle",      biome: Biome::Jungle,     threat: 16, boss: Some(BossKind::JungleAbomination), is_secret: false, requires: &[6] },
    MissionDef { id: 9, name: "Stronghold",        biome: Biome::Stronghold, threat: 18, boss: Some(BossKind::MiniGolemHorde),  is_secret: false, requires: &[8] },
    MissionDef { id: 10, name: "Nether Wastes",    biome: Biome::Nether,     threat: 20, boss: Some(BossKind::Wildfire),        is_secret: false, requires: &[9] },
    MissionDef { id: 11, name: "Obsidian Pinnacle",biome: Biome::Pinnacle,   threat: 23, boss: Some(BossKind::ArchIllager),     is_secret: false, requires: &[10] },
    // DLC islands (batch 2 bosses)
    MissionDef { id: 12, name: "Hiver Grondant",   biome: Biome::Winter,     threat: 24, boss: Some(BossKind::WretchedWraith),  is_secret: false, requires: &[11] },
    MissionDef { id: 13, name: "Pics Hurleurs",    biome: Biome::Peaks,      threat: 25, boss: Some(BossKind::TempestGolem),    is_secret: false, requires: &[11] },
    MissionDef { id: 14, name: "Profondeurs Cachées", biome: Biome::Depths,  threat: 26, boss: Some(BossKind::AncientGuardian), is_secret: false, requires: &[11] },
    MissionDef { id: 15, name: "Écho du Vide",     biome: Biome::Void,       threat: 28, boss: Some(BossKind::VengefulHeart),   is_secret: false, requires: &[12, 13, 14] },
];

/// Missions whose level contains a hidden rune that unlocks the secret below them.
/// (level id) -> (secret mission id unlocked)
pub fn secret_unlocked_by_rune(mission_id: usize) -> Option<usize> {
    match mission_id {
        3 => Some(4), // Soggy Swamp -> Soggy Cave
        6 => Some(7), // Desert Temple -> Arch Haven
        _ => None,
    }
}

impl Biome {
    pub fn display(&self) -> &'static str {
        match self {
            Biome::Forest => "Bois",
            Biome::Plains => "Plaines",
            Biome::Canyon => "Canyon",
            Biome::Swamp => "Marais",
            Biome::Cave => "Caverne",
            Biome::Mines => "Mines",
            Biome::Desert => "Désert",
            Biome::Haven => "Refuge",
            Biome::Jungle => "Jungle",
            Biome::Stronghold => "Forteresse",
            Biome::Nether => "Nether",
            Biome::Pinnacle => "Pinacle",
            Biome::Winter => "Hiver",
            Biome::Peaks => "Pics",
            Biome::Depths => "Profondeurs",
            Biome::Void => "Vide",
        }
    }
}

/// Enemy spawn weights per biome. Keys are EnemyKind ids (see game/enemy.rs).
/// (kind_name, weight)
pub fn biome_enemies(biome: Biome) -> &'static [(&'static str, u32)] {
    match biome {
        Biome::Forest => &[("zombie", 6), ("skeleton", 3), ("creeper", 3), ("spider", 2), ("necromancer", 1), ("enderman", 1)],
        Biome::Plains => &[("zombie", 5), ("skeleton", 3), ("pillager", 3), ("creeper", 2), ("mooshroom", 1), ("illusioner", 1)],
        Biome::Canyon => &[("husk", 5), ("skeleton", 4), ("pillager", 3), ("vindicator", 2), ("geomancer", 1), ("cave_crawler", 2)],
        Biome::Swamp => &[("zombie", 4), ("slime", 5), ("witch", 3), ("spider", 3), ("drowned", 2), ("drowned_necromancer", 1), ("poison_anemone", 1)],
        Biome::Cave => &[("slime", 5), ("zombie", 3), ("skeleton", 3), ("witch", 2), ("bat", 3), ("silverfish", 3), ("cave_crawler", 2), ("endermite", 2)],
        Biome::Mines => &[("zombie", 3), ("skeleton", 3), ("pillager", 4), ("redstone_golem", 1), ("bat", 3), ("silverfish", 2), ("cave_crawler", 2)],
        Biome::Desert => &[("husk", 5), ("skeleton", 4), ("necromancer", 2), ("vindicator", 2), ("geomancer", 1), ("caerbannog", 1)],
        Biome::Haven => &[("mooshroom", 4), ("zombie", 2), ("pillager", 2), ("bat", 2), ("caerbannog", 1)],
        Biome::Jungle => &[("jungle_zombie", 5), ("spider", 4), ("witch", 2), ("creeper", 3), ("bat", 2), ("mossy_skeleton", 3), ("whisperer", 2), ("leapleaf", 2), ("poison_anemone", 1), ("poison_quill_vine", 1), ("mini_abomination", 1)],
        Biome::Stronghold => &[("vindicator", 5), ("pillager", 4), ("enchanter", 2), ("redstone_golem", 1), ("skeleton", 2), ("skeleton_vanguard", 2), ("royal_guard", 1), ("vex", 2), ("illusioner", 1)],
        Biome::Nether => &[("blaze", 4), ("husk", 3), ("wraith", 4), ("geomancer", 2), ("vindicator", 2), ("wither_skeleton", 3), ("zombified_pig", 3), ("piglin", 2), ("piglin_brute", 1), ("hoglin", 1), ("magmacube", 3), ("ghast", 1)],
        Biome::Pinnacle => &[("vindicator", 5), ("enchanter", 3), ("pillager", 4), ("redstone_golem", 1), ("necromancer", 2), ("skeleton_vanguard", 2), ("royal_guard", 1), ("vex", 2), ("enderman", 1)],
        Biome::Winter => &[("frozen_zombie", 5), ("icy_creeper", 3), ("iceologer", 1), ("ghostly_kindler", 2), ("skeleton", 3), ("husk", 2), ("wraith", 2)],
        Biome::Peaks => &[("mountaineer", 5), ("windcaller", 1), ("squall_golem", 1), ("skeleton", 2), ("vindicator", 2), ("iceologer", 1)],
        Biome::Depths => &[("sunken_skeleton", 5), ("drowned", 5), ("drowned_necromancer", 1), ("slime", 2), ("witch", 1)],
        Biome::Void => &[("enderman", 3), ("endermite", 4), ("endling", 3), ("blastling", 3), ("snareling", 3), ("vex", 1), ("wraith", 2)],
    }
}
