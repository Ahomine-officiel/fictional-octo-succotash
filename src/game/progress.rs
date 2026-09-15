//! Save / load: JSON file next to the binary (saves/save.json).

use super::inventory::Inventory;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct PlayerSave {
    pub inventory: Inventory,
    pub level: u32,
    pub xp: u32,
    /// selected hero skin (index into Assets::skin_catalog)
    #[serde(default)]
    pub skin: usize,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Save {
    pub seed: u64,
    pub emeralds: u32,
    pub completed: Vec<usize>,
    pub secrets_found: Vec<usize>,
    /// selected threat tier 0=Default 1=Aventure 2=Apocalypse
    pub tier: usize,
    /// highest tier unlocked
    pub tier_unlocked: usize,
    pub players: Vec<PlayerSave>,
    pub missions_played: u32,
}

impl Save {
    pub fn new() -> Save {
        Save {
            seed: 123456789,
            emeralds: 50,
            completed: Vec::new(),
            secrets_found: Vec::new(),
            tier: 0,
            tier_unlocked: 0,
            players: vec![PlayerSave { inventory: Inventory::starting(), level: 1, xp: 0, skin: 0 }],
            missions_played: 0,
        }
    }

    pub fn is_completed(&self, mission: usize) -> bool {
        self.completed.contains(&mission)
    }

    pub fn is_unlocked(&self, mission: usize) -> bool {
        let def = &crate::world::missions::MISSIONS[mission];
        if def.is_secret {
            // secret: unlocked when the rune was found in the parent mission
            def.requires.iter().all(|r| self.secrets_found.contains(r))
        } else {
            def.requires.iter().all(|r| self.completed.contains(r))
        }
    }

    /// First unlocked-but-not-completed mission (for the camp preselection).
    pub fn next_mission(&self) -> usize {
        for i in 0..crate::world::missions::MISSIONS.len() {
            if !crate::world::missions::MISSIONS[i].is_secret
                && self.is_unlocked(i)
                && !self.is_completed(i)
            {
                return i;
            }
        }
        0
    }

    pub fn path() -> PathBuf {
        let base = crate::assets::find_base_dir_pub();
        base.join("saves").join("save.json")
    }

    pub fn load() -> Save {
        let p = Self::path();
        std::fs::read_to_string(&p)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(Self::new)
    }

    pub fn store(&self) {
        let p = Self::path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(p, json);
        }
    }
}
