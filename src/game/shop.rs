//! Camp vendors: merchant stock + blacksmith upgrade pricing.

use super::items::{gen_item, Item};
use rand::Rng;

pub struct Camp {
    /// (item, price) — restocked after each completed mission
    pub stock: Vec<(Item, u32)>,
    pub artifacts_stock: Vec<(Item, u32)>,
}

impl Camp {
    pub fn new() -> Camp {
        Camp { stock: Vec::new(), artifacts_stock: Vec::new() }
    }

    pub fn restock(&mut self, player_power: u32) {
        let mut rng = rand::thread_rng();
        self.stock.clear();
        for _ in 0..4 {
            let power = player_power + crate::consts::SHOP_POWER_BONUS + rng.gen_range(0..2);
            let item = gen_item(&mut rng, power.max(1), 0.02);
            let price = item.price();
            self.stock.push((item, price));
        }
        self.artifacts_stock.clear();
        for _ in 0..2 {
            let power = player_power + 1;
            let mut item = gen_item(&mut rng, power.max(1), 0.5);
            // force artifact
            item.class = super::items::ItemClass::Art(match rng.gen_range(0..6) {
                0 => super::items::ArtifactKind::Lightning,
                1 => super::items::ArtifactKind::Totem,
                2 => super::items::ArtifactKind::WindHorn,
                3 => super::items::ArtifactKind::Harvester,
                4 => super::items::ArtifactKind::Fireworks,
                _ => super::items::ArtifactKind::Seeds,
            });
            let price = item.price();
            self.artifacts_stock.push((item, price));
        }
    }
}

impl Default for Camp {
    fn default() -> Self {
        Self::new()
    }
}

/// Blacksmith: upgrade item power by 1, cost scales with rarity and power.
pub fn forge_price(item: &Item) -> u32 {
    let base = crate::consts::FORGE_COST_RARITY[match item.rarity {
        super::items::Rarity::Common => 0,
        super::items::Rarity::Rare => 1,
        super::items::Rarity::Unique => 2,
    }];
    base * (1 + item.power as u32 / 8)
}

pub fn forge_upgrade(item: &mut Item) {
    item.power += 1;
}
