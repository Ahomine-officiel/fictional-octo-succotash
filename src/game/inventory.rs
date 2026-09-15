//! Per-player inventory: equipment slots + stash + enchantment points.

use super::items::{Enchant, Item, ItemClass, Slot};

pub const STASH_CAP: usize = 21;

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Inventory {
    pub melee: Option<Item>,
    pub ranged: Option<Item>,
    pub armor: Option<Item>,
    pub artifacts: [Option<Item>; 3],
    pub stash: Vec<Item>,
    pub enchant_points: u32,
}

impl Inventory {
    pub fn starting() -> Inventory {
        Inventory {
            melee: Some(Item {
                name: String::new(),
                class: ItemClass::M(super::items::MeleeClass::Sword),
                power: 2,
                rarity: super::items::Rarity::Common,
                enchants: vec![(Enchant::Sharpness, 1)],
            }),
            ranged: Some(Item {
                name: String::new(),
                class: ItemClass::R(super::items::RangedClass::Bow),
                power: 2,
                rarity: super::items::Rarity::Common,
                enchants: Vec::new(),
            }),
            armor: Some(Item {
                name: String::new(),
                class: ItemClass::A(super::items::ArmorClass::Medium),
                power: 2,
                rarity: super::items::Rarity::Common,
                enchants: Vec::new(),
            }),
            artifacts: [
                Some(Item {
                    name: String::new(),
                    class: ItemClass::Art(super::items::ArtifactKind::Totem),
                    power: 2,
                    rarity: super::items::Rarity::Common,
                    enchants: Vec::new(),
                }),
                None,
                None,
            ],
            stash: Vec::new(),
            enchant_points: 1,
        }
    }

    pub fn display_name(item: &Item) -> String {
        if item.name.is_empty() {
            item.base_name().to_string()
        } else {
            item.name.clone()
        }
    }

    /// Player power = highest power item owned (per the design doc).
    pub fn power(&self) -> u32 {
        let mut p = 1;
        for it in self.all_items() {
            p = p.max(it.power);
        }
        p
    }

    pub fn all_items(&self) -> Vec<&Item> {
        let mut v = Vec::new();
        for it in [&self.melee, &self.ranged, &self.armor] {
            if let Some(i) = it { v.push(i); }
        }
        for it in &self.artifacts {
            if let Some(i) = it { v.push(i); }
        }
        for it in &self.stash {
            v.push(it);
        }
        v
    }

    pub fn slot_ref(&self, slot: Slot, art: usize) -> Option<&Item> {
        match slot {
            Slot::Melee => self.melee.as_ref(),
            Slot::Ranged => self.ranged.as_ref(),
            Slot::Armor => self.armor.as_ref(),
            Slot::Artifact => self.artifacts.get(art).and_then(|x| x.as_ref()),
        }
    }

    fn slot_set(&mut self, slot: Slot, art: usize, item: Option<Item>) -> Option<Item> {
        match slot {
            Slot::Melee => std::mem::replace(&mut self.melee, item),
            Slot::Ranged => std::mem::replace(&mut self.ranged, item),
            Slot::Armor => std::mem::replace(&mut self.armor, item),
            Slot::Artifact => self.artifacts.get_mut(art).and_then(|s| std::mem::replace(s, item)),
        }
    }

    /// Equip from stash (or push raw item). Returns displaced item into stash.
    pub fn equip(&mut self, item: Item) {
        let slot = item.slot();
        let art = if slot == Slot::Artifact {
            // first free artifact slot, else slot 0
            self.artifacts.iter().position(|a| a.is_none()).unwrap_or(0)
        } else {
            0
        };
        let old = self.slot_set(slot, art, Some(item));
        if let Some(o) = old {
            self.push_stash(o);
        }
    }

    pub fn push_stash(&mut self, item: Item) -> bool {
        if self.stash.len() >= STASH_CAP {
            return false; // inventory full — caller converts to emeralds
        }
        self.stash.push(item);
        true
    }

    /// Salvage: emeralds + refund enchantment points (like MCD salvage).
    pub fn salvage(&mut self, idx: usize) -> Option<(u32, u32)> {
        let item = self.stash.remove(idx);
        let mut emeralds = (item.price() as f32 * crate::consts::SALVAGE_PCT) as u32;
        let mut points = 0;
        for (e, t) in &item.enchants {
            points += Enchant::cost(*t);
            emeralds += 5 * (*t as u32);
            let _ = e;
        }
        self.enchant_points += points;
        Some((emeralds, points))
    }

    pub fn can_enchant(&self, item: &Item, e: Enchant, tier: u8) -> bool {
        if self.enchant_points < Enchant::cost(tier) {
            return false;
        }
        if item.enchants.len() >= item.rarity.slots() {
            return false;
        }
        if item.enchants.iter().any(|(en, _)| *en == e) {
            return false;
        }
        if !e.applies_to(item.slot()) {
            return false;
        }
        // upgrade existing tier not allowed to skip levels
        true
    }

    pub fn enchant(&mut self, stash_idx: usize, e: Enchant, tier: u8) -> bool {
        // validate first on an immutable borrow
        {
            let Some(item) = self.stash.get(stash_idx) else { return false };
            if !self.can_enchant(item, e, tier) {
                return false;
            }
        }
        let Some(item) = self.stash.get_mut(stash_idx) else { return false };
        let cost = Enchant::cost(tier);
        item.enchants.push((e, tier));
        self.enchant_points -= cost;
        true
    }

    pub fn stash_get(&self, idx: usize) -> Option<&Item> {
        self.stash.get(idx)
    }
}
