//! Box-model definitions for players, mobs and bosses, with procedural animation.
//! Everything in Minecraft Dungeons is boxy — this keeps us faithful without
//! external model files. Player model maps a 64x64 MC skin directly.

use crate::assets::{Assets, Rect};
use crate::gfx::BoxInstance;
use glam::{Quat, Vec3, Vec4};

pub const FACE_TOP: usize = 0;
pub const FACE_BOTTOM: usize = 1;
pub const FACE_FRONT: usize = 2; // -Z
pub const FACE_BACK: usize = 3; // +Z
pub const FACE_RIGHT: usize = 4; // +X
pub const FACE_LEFT: usize = 5; // -X

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PartId {
    Head,
    Torso,
    ArmL,
    ArmR,
    LegL,
    LegR,
    Extra(u8),
}

#[derive(Clone)]
pub struct PartSpec {
    pub id: PartId,
    /// body-space pivot (rotation point), relative to feet origin
    pub pivot: Vec3,
    /// box center relative to pivot (body space, before part rotation)
    pub offset: Vec3,
    pub size: Vec3,
    /// atlas texture keys: [top, bottom, front(optional), side]
    pub uv_top: &'static str,
    pub uv_bottom: &'static str,
    pub uv_front: &'static str,
    pub uv_side: &'static str,
    pub emissive: f32,
}

#[derive(Clone)]
pub struct MobModel {
    pub parts: Vec<PartSpec>,
    /// y offset of the shadow disc / hover height
    pub hover: f32,
    pub scale: f32,
}

impl MobModel {
    fn human(
        skin: &'static str,
        face: &'static str,
        body: &'static str,
        thin: bool,
        robe: bool,
        scale: f32,
        emissive: f32,
    ) -> MobModel {
        let aw: f32 = if thin { 0.125 } else { 0.25 };
        let lw: f32 = if thin { 0.125 } else { 0.25 };
        let arm_x = 0.25 + aw / 2.0;
        let mut parts = vec![
            PartSpec {
                id: PartId::Head,
                pivot: Vec3::new(0.0, 1.5, 0.0),
                offset: Vec3::new(0.0, 0.25, 0.0),
                size: Vec3::new(0.5, 0.5, 0.5),
                uv_top: "", uv_bottom: "", uv_front: "", uv_side: "",
                emissive,
            },
            PartSpec {
                id: PartId::Torso,
                pivot: Vec3::new(0.0, 0.75, 0.0),
                offset: Vec3::new(0.0, 0.375, 0.0),
                size: Vec3::new(0.5, 0.75, if robe { 0.55 } else { 0.25 }),
                uv_top: "", uv_bottom: "", uv_front: "", uv_side: "",
                emissive,
            },
            PartSpec {
                id: PartId::ArmL,
                pivot: Vec3::new(-arm_x, 1.4, 0.0),
                offset: Vec3::new(0.0, -0.32, 0.0),
                size: Vec3::new(aw, 0.75, aw),
                uv_top: "", uv_bottom: "", uv_front: "", uv_side: "",
                emissive,
            },
            PartSpec {
                id: PartId::ArmR,
                pivot: Vec3::new(arm_x, 1.4, 0.0),
                offset: Vec3::new(0.0, -0.32, 0.0),
                size: Vec3::new(aw, 0.75, aw),
                uv_top: "", uv_bottom: "", uv_front: "", uv_side: "",
                emissive,
            },
            PartSpec {
                id: PartId::LegL,
                pivot: Vec3::new(-0.125, 0.75, 0.0),
                offset: Vec3::new(0.0, -0.375, 0.0),
                size: Vec3::new(lw, 0.75, lw),
                uv_top: "", uv_bottom: "", uv_front: "", uv_side: "",
                emissive,
            },
            PartSpec {
                id: PartId::LegR,
                pivot: Vec3::new(0.125, 0.75, 0.0),
                offset: Vec3::new(0.0, -0.375, 0.0),
                size: Vec3::new(lw, 0.75, lw),
                uv_top: "", uv_bottom: "", uv_front: "", uv_side: "",
                emissive,
            },
        ];
        for p in parts.iter_mut() {
            p.uv_top = skin; p.uv_bottom = skin; p.uv_front = skin; p.uv_side = skin;
        }
        // head front shows the face, torso uses the body-front crop
        parts[0].uv_front = face;
        let t = &mut parts[1];
        t.uv_top = body; t.uv_bottom = body; t.uv_front = body; t.uv_side = body;
        MobModel { parts, hover: 0.0, scale }
    }
}

// ----------------------------------------------------------------------
// Enemy models (keyed by the same names used in missions.rs spawn tables)
// ----------------------------------------------------------------------

pub fn mob_model(name: &str) -> MobModel {
    match name {
        "zombie" => MobModel::human("zombie_skin", "zombie_face", "zombie_body", false, false, 1.0, 0.0),
        "husk" => MobModel::human("husk_skin", "husk_face", "husk_body", false, false, 1.04, 0.0),
        "jungle_zombie" => MobModel::human("jungle_zombie_skin", "jungle_zombie_face", "jungle_zombie_body", false, false, 1.0, 0.0),
        "drowned" => MobModel::human("drowned_skin", "drowned_face", "drowned_body", false, false, 1.0, 0.0),
        "skeleton" => MobModel::human("skeleton_skin", "skeleton_face", "skeleton_body", true, false, 1.0, 0.0),
        "pillager" => MobModel::human("pillager_skin", "pillager_face", "pillager_body", false, false, 1.0, 0.0),
        "vindicator" => MobModel::human("vindicator_skin", "vindicator_face", "vindicator_body", false, false, 1.02, 0.0),
        "enchanter" => MobModel::human("enchanter_skin", "enchanter_face", "enchanter_body", false, true, 0.98, 0.0),
        "geomancer" => MobModel::human("geomancer_skin", "geomancer_face", "geomancer_body", false, true, 1.0, 0.0),
        "necromancer" => MobModel::human("necromancer_skin", "necromancer_face", "necromancer_body", true, true, 1.0, 0.0),
        "witch" => MobModel::human("witch_skin", "witch_face", "witch_body", false, true, 1.0, 0.0),
        "wraith" => {
            let mut m = MobModel::human("wraith_skin", "wraith_face", "wraith_body", true, true, 1.0, 0.0);
            m.hover = 0.35;
            m.parts.retain(|p| p.id != PartId::LegL && p.id != PartId::LegR);
            m
        }
        "blaze" => {
            let mut m = MobModel::human("blaze_skin", "blaze_face", "blaze_skin", true, false, 0.9, 0.35);
            m.parts.retain(|p| p.id != PartId::LegL && p.id != PartId::LegR);
            m.hover = 0.3;
            m
        }
        "creeper" => {
            let parts = vec![
                PartSpec {
                    id: PartId::Head, pivot: Vec3::new(0.0, 1.125, 0.0),
                    offset: Vec3::new(0.0, 0.25, 0.0), size: Vec3::new(0.5, 0.5, 0.5),
                    uv_top: "creeper_skin", uv_bottom: "creeper_skin",
                    uv_front: "creeper_face", uv_side: "creeper_skin", emissive: 0.0,
                },
                PartSpec {
                    id: PartId::Torso, pivot: Vec3::new(0.0, 0.375, 0.0),
                    offset: Vec3::new(0.0, 0.375, 0.0), size: Vec3::new(0.5, 0.75, 0.3),
                    uv_top: "creeper_skin", uv_bottom: "creeper_skin",
                    uv_front: "creeper_skin", uv_side: "creeper_skin", emissive: 0.0,
                },
                leg("creeper_skin", PartId::LegL, Vec3::new(-0.14, 0.375, 0.16), 0.375),
                leg("creeper_skin", PartId::LegR, Vec3::new(0.14, 0.375, 0.16), 0.375),
                leg("creeper_skin", PartId::Extra(0), Vec3::new(-0.14, 0.375, -0.16), 0.375),
                leg("creeper_skin", PartId::Extra(1), Vec3::new(0.14, 0.375, -0.16), 0.375),
            ];
            MobModel { parts, hover: 0.0, scale: 1.0 }
        }
        "spider" => {
            let mut parts = vec![
                PartSpec {
                    id: PartId::Head, pivot: Vec3::new(0.0, 0.45, -0.45),
                    offset: Vec3::new(0.0, 0.05, -0.2), size: Vec3::new(0.45, 0.4, 0.45),
                    uv_top: "spider_skin", uv_bottom: "spider_skin",
                    uv_front: "spider_face", uv_side: "spider_skin", emissive: 0.0,
                },
                PartSpec {
                    id: PartId::Torso, pivot: Vec3::new(0.0, 0.0, 0.0),
                    offset: Vec3::new(0.0, 0.35, 0.15), size: Vec3::new(0.7, 0.45, 0.9),
                    uv_top: "spider_skin", uv_bottom: "spider_skin",
                    uv_front: "spider_skin", uv_side: "spider_skin", emissive: 0.0,
                },
            ];
            for i in 0..4 {
                let side = if i % 2 == 0 { -1.0 } else { 1.0 };
                let z = -0.25 + (i as f32 / 2.0).floor() * 0.5;
                parts.push(PartSpec {
                    id: PartId::Extra(i as u8),
                    pivot: Vec3::new(side * 0.35, 0.35, z),
                    offset: Vec3::new(side * 0.4, -0.12, 0.0),
                    size: Vec3::new(0.75, 0.12, 0.12),
                    uv_top: "spider_skin", uv_bottom: "spider_skin",
                    uv_front: "spider_skin", uv_side: "spider_skin", emissive: 0.0,
                });
            }
            MobModel { parts, hover: 0.0, scale: 1.0 }
        }
        "slime" => MobModel {
            parts: vec![PartSpec {
                id: PartId::Torso, pivot: Vec3::new(0.0, 0.0, 0.0),
                offset: Vec3::new(0.0, 0.45, 0.0), size: Vec3::new(0.85, 0.85, 0.85),
                uv_top: "slime_skin", uv_bottom: "slime_skin",
                uv_front: "slime_face", uv_side: "slime_skin", emissive: 0.08,
            }],
            hover: 0.0,
            scale: 1.0,
        },
        "mooshroom" => {
            let mut parts = vec![
                PartSpec {
                    id: PartId::Head, pivot: Vec3::new(0.0, 0.9, -0.75),
                    offset: Vec3::new(0.0, 0.2, -0.1), size: Vec3::new(0.5, 0.45, 0.45),
                    uv_top: "mooshroom_skin", uv_bottom: "mooshroom_skin",
                    uv_front: "mooshroom_face", uv_side: "mooshroom_skin", emissive: 0.0,
                },
                PartSpec {
                    id: PartId::Torso, pivot: Vec3::new(0.0, 0.6, 0.0),
                    offset: Vec3::new(0.0, 0.3, 0.0), size: Vec3::new(0.8, 0.6, 1.3),
                    uv_top: "mooshroom_skin", uv_bottom: "mooshroom_skin",
                    uv_front: "mooshroom_skin", uv_side: "mooshroom_skin", emissive: 0.0,
                },
                leg("mooshroom_skin", PartId::LegL, Vec3::new(-0.25, 0.6, -0.4), 0.6),
                leg("mooshroom_skin", PartId::LegR, Vec3::new(0.25, 0.6, -0.4), 0.6),
                leg("mooshroom_skin", PartId::Extra(0), Vec3::new(-0.25, 0.6, 0.4), 0.6),
                leg("mooshroom_skin", PartId::Extra(1), Vec3::new(0.25, 0.6, 0.4), 0.6),
            ];
            // mushrooms on the back
            parts.push(PartSpec {
                id: PartId::Extra(2), pivot: Vec3::new(-0.2, 0.9, 0.2),
                offset: Vec3::new(0.0, 0.12, 0.0), size: Vec3::new(0.25, 0.2, 0.25),
                uv_top: "pumpkin_top", uv_bottom: "pumpkin_top",
                uv_front: "pumpkin_side", uv_side: "pumpkin_side", emissive: 0.0,
            });
            parts.push(PartSpec {
                id: PartId::Extra(3), pivot: Vec3::new(0.2, 0.9, 0.45),
                offset: Vec3::new(0.0, 0.09, 0.0), size: Vec3::new(0.2, 0.16, 0.2),
                uv_top: "pumpkin_top", uv_bottom: "pumpkin_top",
                uv_front: "pumpkin_side", uv_side: "pumpkin_side", emissive: 0.0,
            });
            MobModel { parts, hover: 0.0, scale: 1.0 }
        }
        "bat" => {
            let mut parts = vec![
                PartSpec {
                    id: PartId::Torso, pivot: Vec3::new(0.0, 0.0, 0.0),
                    offset: Vec3::new(0.0, 0.3, 0.0), size: Vec3::new(0.3, 0.3, 0.3),
                    uv_top: "bat_skin", uv_bottom: "bat_skin",
                    uv_front: "bat_face", uv_side: "bat_skin", emissive: 0.0,
                },
                wing(PartId::Extra(0), -1.0),
                wing(PartId::Extra(1), 1.0),
            ];
            parts[1].pivot = Vec3::new(-0.15, 0.35, 0.0);
            parts[2].pivot = Vec3::new(0.15, 0.35, 0.0);
            MobModel { parts, hover: 1.2, scale: 1.0 }
        }
        "redstone_golem" => {
            let mut m = MobModel::human("redstone_golem_skin", "redstone_golem_face", "redstone_golem_body", false, false, 1.0, 0.0);
            m.scale = 2.1;
            m
        }
        // ---------------- bosses ----------------
        "boss_cauldron" => MobModel {
            parts: vec![
                PartSpec {
                    id: PartId::Torso, pivot: Vec3::new(0.0, 0.0, 0.0),
                    offset: Vec3::new(0.0, 0.8, 0.0), size: Vec3::new(2.0, 1.6, 2.0),
                    uv_top: "cauldron_metal", uv_bottom: "cauldron_metal",
                    uv_front: "cauldron_metal", uv_side: "cauldron_metal", emissive: 0.05,
                },
                PartSpec {
                    id: PartId::Extra(0), pivot: Vec3::new(0.0, 1.6, 0.0),
                    offset: Vec3::new(0.0, 0.08, 0.0), size: Vec3::new(2.3, 0.16, 2.3),
                    uv_top: "cauldron_wood", uv_bottom: "cauldron_wood", uv_front: "cauldron_wood", uv_side: "cauldron_wood",
                    emissive: 0.0,
                },
                PartSpec {
                    id: PartId::Head, pivot: Vec3::new(0.0, 1.68, 0.0),
                    offset: Vec3::new(0.0, 0.1, 0.0), size: Vec3::new(1.6, 0.2, 1.6),
                    uv_top: "cauldron_potion", uv_bottom: "cauldron_potion",
                    uv_front: "cauldron_potion", uv_side: "cauldron_potion", emissive: 0.55,
                },
            ],
            hover: 0.0,
            scale: 1.0,
        },
        "boss_monstrosity" => {
            let mut m = MobModel::human("monstrosity_skin", "monstrosity_face", "monstrosity_body", false, false, 1.0, 0.25);
            m.scale = 3.2;
            m
        }
        "boss_nameless" => {
            let mut m = MobModel::human("nameless_skin", "nameless_face", "nameless_body", true, true, 1.0, 0.2);
            m.scale = 1.35;
            m
        }
        "boss_arch" => {
            let mut m = MobModel::human("arch_illager_skin", "arch_illager_face", "arch_illager_body", false, false, 1.0, 0.0);
            m.scale = 0.95;
            m.parts.push(PartSpec {
                id: PartId::Extra(0), pivot: Vec3::new(0.0, 2.0, 0.0),
                offset: Vec3::new(0.0, 0.08, 0.0), size: Vec3::new(0.6, 0.16, 0.6),
                uv_top: "icon_key", uv_bottom: "icon_key", uv_front: "icon_key",
                uv_side: "icon_key", emissive: 0.3,
            });
            m
        }
        "boss_heart" => MobModel {
            parts: vec![
                PartSpec {
                    id: PartId::Torso, pivot: Vec3::new(0.0, 0.0, 0.0),
                    offset: Vec3::new(0.0, 1.4, 0.0), size: Vec3::new(1.1, 1.5, 1.1),
                    uv_top: "obsidian", uv_bottom: "obsidian",
                    uv_front: "obsidian", uv_side: "obsidian", emissive: 0.3,
                },
                PartSpec {
                    id: PartId::Head, pivot: Vec3::new(0.0, 2.15, 0.0),
                    offset: Vec3::new(0.0, 0.2, 0.0), size: Vec3::new(0.6, 0.6, 0.6),
                    uv_top: "sculk", uv_bottom: "sculk", uv_front: "sculk", uv_side: "sculk",
                    emissive: 0.5,
                },
                tentacle(PartId::Extra(0), -0.6, 0.3),
                tentacle(PartId::Extra(1), 0.6, -0.2),
                tentacle(PartId::Extra(2), 0.1, 0.6),
            ],
            hover: 0.8,
            scale: 1.0,
        },
        "captive" => MobModel::human("pillager_skin", "pillager_face", "pillager_body", false, false, 0.9, 0.0),
        // =============================================================
        // Full bestiary additions (batch 2) — regular mobs
        // =============================================================
        "mossy_skeleton" => MobModel::human("mossy_skeleton_skin", "mossy_skeleton_face", "mossy_skeleton_body", true, false, 1.0, 0.0),
        "skeleton_vanguard" => MobModel::human("skeleton_vanguard_skin", "skeleton_vanguard_face", "skeleton_vanguard_body", true, false, 1.06, 0.0),
        "sunken_skeleton" => MobModel::human("sunken_skeleton_skin", "sunken_skeleton_face", "sunken_skeleton_body", true, false, 1.0, 0.0),
        "drowned_necromancer" => MobModel::human("drowned_necromancer_skin", "drowned_necromancer_face", "drowned_necromancer_body", true, true, 1.05, 0.0),
        "whisperer" => MobModel::human("whisperer_skin", "whisperer_face", "whisperer_body", true, true, 1.05, 0.0),
        "royal_guard" => MobModel::human("royal_guard_skin", "royal_guard_face", "royal_guard_body", false, false, 1.08, 0.0),
        "illusioner" => MobModel::human("illusioner_skin", "illusioner_face", "illusioner_body", false, true, 1.0, 0.0),
        "frozen_zombie" => MobModel::human("frozen_zombie_skin", "frozen_zombie_face", "frozen_zombie_body", false, false, 1.02, 0.0),
        "ghostly_kindler" => {
            let mut m = MobModel::human("ghostly_kindler_skin", "ghostly_kindler_face", "ghostly_kindler_body", true, true, 1.0, 0.15);
            m.hover = 0.35;
            m.parts.retain(|p| p.id != PartId::LegL && p.id != PartId::LegR);
            m
        }
        "piglin" => MobModel::human("piglin_skin", "piglin_face", "piglin_body", false, false, 1.0, 0.0),
        "piglin_brute" => MobModel::human("piglin_brute_skin", "piglin_brute_face", "piglin_brute_body", false, false, 1.1, 0.0),
        "endling" => MobModel::human("endling_skin", "endling_face", "endling_body", true, false, 1.08, 0.1),
        "iceologer" => MobModel::human("iceologer_skin", "iceologer_face", "iceologer_body", false, true, 1.05, 0.0),
        "mountaineer" => MobModel::human("mountaineer_skin", "mountaineer_face", "mountaineer_body", false, false, 1.05, 0.0),
        "windcaller" => MobModel::human("windcaller_skin", "windcaller_face", "windcaller_body", false, true, 1.05, 0.0),
        "squall_golem" => {
            let mut m = MobModel::human("squall_golem_skin", "squall_golem_face", "squall_golem_body", false, false, 1.0, 0.15);
            m.scale = 1.35;
            m
        }
        "wither_skeleton" => MobModel::human("wither_skeleton_skin", "wither_skeleton_face", "wither_skeleton_body", true, false, 1.15, 0.0),
        "zombified_pig" => MobModel::human("zombified_pig_skin", "zombified_pig_face", "zombified_pig_body", false, false, 1.0, 0.0),
        "enderman" => MobModel::human("enderman_skin", "enderman_face", "enderman_body", true, false, 1.42, 0.08),
        "caerbannog" => quadruped("caerbannog_skin", "caerbannog_face", 0.42, false),
        "leapleaf" => quadruped("leapleaf_skin", "leapleaf_face", 1.35, false),
        "hoglin" => quadruped("hoglin_skin", "hoglin_face", 1.25, false),
        "silverfish" => crawler("silverfish_skin", "silverfish_face", 0.42),
        "endermite" => crawler("endermite_skin", "endermite_face", 0.36),
        "cave_crawler" => crawler("cave_crawler_skin", "cave_crawler_face", 0.8),
        "mini_abomination" => crawler("mini_abomination_skin", "mini_abomination_face", 0.55),
        "vex" => {
            let mut parts = vec![
                PartSpec {
                    id: PartId::Torso, pivot: Vec3::new(0.0, 0.0, 0.0),
                    offset: Vec3::new(0.0, 0.3, 0.0), size: Vec3::new(0.35, 0.5, 0.25),
                    uv_top: "vex_skin", uv_bottom: "vex_skin", uv_front: "vex_skin", uv_side: "vex_skin",
                    emissive: 0.0,
                },
                PartSpec {
                    id: PartId::Head, pivot: Vec3::new(0.0, 0.62, 0.0),
                    offset: Vec3::new(0.0, 0.18, 0.0), size: Vec3::new(0.38, 0.38, 0.38),
                    uv_top: "vex_skin", uv_bottom: "vex_skin", uv_front: "vex_face", uv_side: "vex_skin",
                    emissive: 0.0,
                },
            ];
            for s in [-1.0f32, 1.0] {
                parts.push(PartSpec {
                    id: PartId::Extra((s + 1.0) as u8 / 2),
                    pivot: Vec3::new(s * 0.18, 0.55, 0.0),
                    offset: Vec3::new(s * 0.28, 0.06, 0.0), size: Vec3::new(0.45, 0.05, 0.3),
                    uv_top: "vex_skin", uv_bottom: "vex_skin", uv_front: "vex_skin", uv_side: "vex_skin",
                    emissive: 0.0,
                });
            }
            MobModel { parts, hover: 0.9, scale: 1.0 }
        }
        "ghast" => {
            let mut parts = vec![
                PartSpec {
                    id: PartId::Head, pivot: Vec3::new(0.0, 1.1, 0.0),
                    offset: Vec3::new(0.0, 0.4, 0.0), size: Vec3::new(1.5, 1.5, 1.5),
                    uv_top: "ghast_skin", uv_bottom: "ghast_skin", uv_front: "ghast_face", uv_side: "ghast_skin",
                    emissive: 0.1,
                },
            ];
            for i in 0..6 {
                let x = ((i % 3) as f32 - 1.0) * 0.45;
                let z = ((i / 3) as f32 - 0.5) * 0.9;
                parts.push(tentacle_key(PartId::Extra(i as u8), "ghast_skin", x, z, 1.1));
            }
            MobModel { parts, hover: 1.6, scale: 1.0 }
        }
        "magmacube" => MobModel {
            parts: vec![PartSpec {
                id: PartId::Torso, pivot: Vec3::new(0.0, 0.0, 0.0),
                offset: Vec3::new(0.0, 0.45, 0.0), size: Vec3::new(0.85, 0.85, 0.85),
                uv_top: "magmacube_skin", uv_bottom: "magmacube_skin",
                uv_front: "magmacube_face", uv_side: "magmacube_skin", emissive: 0.45,
            }],
            hover: 0.0,
            scale: 0.95,
        },
        "poison_anemone" => plant("poison_anemone_skin", "poison_anemone_face", 1.0),
        "poison_quill_vine" => plant("poison_quill_vine_skin", "poison_quill_vine_face", 1.15),
        "blastling" => MobModel::human("blastling_skin", "blastling_face", "blastling_body", true, true, 1.05, 0.15),
        "snareling" => MobModel::human("snareling_skin", "snareling_face", "snareling_body", true, true, 1.0, 0.1),
        "icy_creeper" => {
            let parts = vec![
                PartSpec {
                    id: PartId::Head, pivot: Vec3::new(0.0, 1.125, 0.0),
                    offset: Vec3::new(0.0, 0.25, 0.0), size: Vec3::new(0.5, 0.5, 0.5),
                    uv_top: "icy_creeper_skin", uv_bottom: "icy_creeper_skin",
                    uv_front: "icy_creeper_face", uv_side: "icy_creeper_skin", emissive: 0.05,
                },
                PartSpec {
                    id: PartId::Torso, pivot: Vec3::new(0.0, 0.375, 0.0),
                    offset: Vec3::new(0.0, 0.375, 0.0), size: Vec3::new(0.5, 0.75, 0.3),
                    uv_top: "icy_creeper_skin", uv_bottom: "icy_creeper_skin",
                    uv_front: "icy_creeper_skin", uv_side: "icy_creeper_skin", emissive: 0.05,
                },
                leg("icy_creeper_skin", PartId::LegL, Vec3::new(-0.14, 0.375, 0.16), 0.375),
                leg("icy_creeper_skin", PartId::LegR, Vec3::new(0.14, 0.375, 0.16), 0.375),
                leg("icy_creeper_skin", PartId::Extra(0), Vec3::new(-0.14, 0.375, -0.16), 0.375),
                leg("icy_creeper_skin", PartId::Extra(1), Vec3::new(0.14, 0.375, -0.16), 0.375),
            ];
            MobModel { parts, hover: 0.0, scale: 1.0 }
        }
        // =============================================================
        // DLC bosses (batch 2)
        // =============================================================
        "boss_jungle" => {
            let mut m = MobModel::human("jungle_abomination_skin", "jungle_abomination_face", "jungle_abomination_body", false, true, 1.0, 0.08);
            m.scale = 2.6;
            m
        }
        "boss_wretched" => {
            let mut m = MobModel::human("wretched_skin", "wretched_face", "wretched_body", true, true, 1.0, 0.35);
            m.scale = 2.3;
            m.hover = 0.5;
            m.parts.retain(|p| p.id != PartId::LegL && p.id != PartId::LegR);
            m
        }
        "boss_tempest" => {
            let mut m = MobModel::human("tempest_skin", "tempest_face", "tempest_body", false, false, 1.0, 0.2);
            m.scale = 3.0;
            m
        }
        "boss_ancient" => {
            let mut m = quadruped("ancient_guardian_skin", "ancient_guardian_face", 2.3, false);
            for p in m.parts.iter_mut() { p.emissive = 0.3; }
            m
        }
        "boss_wildfire" => {
            let mut m = MobModel::human("wildfire_skin", "wildfire_face", "wildfire_skin", true, false, 1.0, 0.5);
            m.scale = 2.6;
            m.hover = 0.6;
            m.parts.retain(|p| p.id != PartId::LegL && p.id != PartId::LegR);
            m
        }
        "boss_vengeful" => {
            let mut m = MobModel::human("vengeful_skin", "vengeful_face", "vengeful_body", true, true, 1.0, 0.45);
            m.scale = 2.2;
            m.hover = 0.4;
            m
        }
        _ => MobModel::human("zombie_skin", "zombie_face", "zombie_body", false, false, 1.0, 0.0),
    }
}

fn leg(skin: &'static str, id: PartId, pivot: Vec3, h: f32) -> PartSpec {
    PartSpec {
        id,
        pivot,
        offset: Vec3::new(0.0, -h / 2.0, 0.0),
        size: Vec3::new(0.22, h, 0.22),
        uv_top: skin, uv_bottom: skin, uv_front: skin, uv_side: skin,
        emissive: 0.0,
    }
}

fn wing(id: PartId, side: f32) -> PartSpec {
    PartSpec {
        id,
        pivot: Vec3::new(side * 0.15, 0.35, 0.0),
        offset: Vec3::new(side * 0.25, 0.0, 0.0),
        size: Vec3::new(0.4, 0.04, 0.25),
        uv_top: "bat_skin", uv_bottom: "bat_skin", uv_front: "bat_skin", uv_side: "bat_skin",
        emissive: 0.0,
    }
}

fn tentacle(id: PartId, x: f32, z: f32) -> PartSpec {
    PartSpec {
        id,
        pivot: Vec3::new(x, 1.0, z),
        offset: Vec3::new(0.0, -0.5, 0.0),
        size: Vec3::new(0.22, 1.3, 0.22),
        uv_top: "obsidian", uv_bottom: "obsidian", uv_front: "sculk", uv_side: "obsidian",
        emissive: 0.2,
    }
}

fn tentacle_key(id: PartId, key: &'static str, x: f32, z: f32, h: f32) -> PartSpec {
    PartSpec {
        id,
        pivot: Vec3::new(x, 1.0, z),
        offset: Vec3::new(0.0, -h / 2.0, 0.0),
        size: Vec3::new(0.22, h, 0.22),
        uv_top: key, uv_bottom: key, uv_front: key, uv_side: key,
        emissive: 0.05,
    }
}

/// Multi-legged crawler body (spider family: silverfish, endermite, crawlers).
fn crawler(skin: &'static str, face: &'static str, scale: f32) -> MobModel {
    let mut parts = vec![
        PartSpec {
            id: PartId::Head, pivot: Vec3::new(0.0, 0.45, -0.45),
            offset: Vec3::new(0.0, 0.05, -0.2), size: Vec3::new(0.45, 0.4, 0.45),
            uv_top: skin, uv_bottom: skin, uv_front: face, uv_side: skin, emissive: 0.0,
        },
        PartSpec {
            id: PartId::Torso, pivot: Vec3::new(0.0, 0.0, 0.0),
            offset: Vec3::new(0.0, 0.35, 0.15), size: Vec3::new(0.7, 0.45, 0.9),
            uv_top: skin, uv_bottom: skin, uv_front: skin, uv_side: skin, emissive: 0.0,
        },
    ];
    for i in 0..4 {
        let side = if i % 2 == 0 { -1.0 } else { 1.0 };
        let z = -0.25 + (i as f32 / 2.0).floor() * 0.5;
        parts.push(PartSpec {
            id: PartId::Extra(i as u8),
            pivot: Vec3::new(side * 0.35, 0.35, z),
            offset: Vec3::new(side * 0.4, -0.12, 0.0),
            size: Vec3::new(0.75, 0.12, 0.12),
            uv_top: skin, uv_bottom: skin, uv_front: skin, uv_side: skin, emissive: 0.0,
        });
    }
    MobModel { parts, hover: 0.0, scale }
}

/// Four-legged beast (hoglin, leapleaf, rabbit, guardian).
fn quadruped(skin: &'static str, face: &'static str, scale: f32, mushrooms: bool) -> MobModel {
    let mut parts = vec![
        PartSpec {
            id: PartId::Head, pivot: Vec3::new(0.0, 0.9, -0.75),
            offset: Vec3::new(0.0, 0.2, -0.1), size: Vec3::new(0.5, 0.45, 0.45),
            uv_top: skin, uv_bottom: skin, uv_front: face, uv_side: skin, emissive: 0.0,
        },
        PartSpec {
            id: PartId::Torso, pivot: Vec3::new(0.0, 0.6, 0.0),
            offset: Vec3::new(0.0, 0.3, 0.0), size: Vec3::new(0.8, 0.6, 1.3),
            uv_top: skin, uv_bottom: skin, uv_front: skin, uv_side: skin, emissive: 0.0,
        },
        leg(skin, PartId::LegL, Vec3::new(-0.25, 0.6, -0.4), 0.6),
        leg(skin, PartId::LegR, Vec3::new(0.25, 0.6, -0.4), 0.6),
        leg(skin, PartId::Extra(0), Vec3::new(-0.25, 0.6, 0.4), 0.6),
        leg(skin, PartId::Extra(1), Vec3::new(0.25, 0.6, 0.4), 0.6),
    ];
    if mushrooms {
        parts.push(PartSpec {
            id: PartId::Extra(2), pivot: Vec3::new(-0.2, 0.9, 0.2),
            offset: Vec3::new(0.0, 0.12, 0.0), size: Vec3::new(0.25, 0.2, 0.25),
            uv_top: "pumpkin_top", uv_bottom: "pumpkin_top",
            uv_front: "pumpkin_side", uv_side: "pumpkin_side", emissive: 0.0,
        });
    }
    MobModel { parts, hover: 0.0, scale }
}

/// Rooted plant mob (anemone, quill vine): stalk + glowing head.
fn plant(skin: &'static str, face: &'static str, scale: f32) -> MobModel {
    let parts = vec![
        PartSpec {
            id: PartId::Torso, pivot: Vec3::new(0.0, 0.0, 0.0),
            offset: Vec3::new(0.0, 0.55, 0.0), size: Vec3::new(0.4, 1.1, 0.4),
            uv_top: skin, uv_bottom: skin, uv_front: skin, uv_side: skin, emissive: 0.05,
        },
        PartSpec {
            id: PartId::Head, pivot: Vec3::new(0.0, 1.1, 0.0),
            offset: Vec3::new(0.0, 0.25, 0.0), size: Vec3::new(0.7, 0.5, 0.7),
            uv_top: face, uv_bottom: skin, uv_front: face, uv_side: skin, emissive: 0.3,
        },
    ];
    MobModel { parts, hover: 0.0, scale }
}

// ----------------------------------------------------------------------
// Animation state -> per-part transforms
// ----------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
pub struct AnimState {
    pub walk_phase: f32,
    pub moving: f32, // 0..1 blend
    /// 0..1 progress of melee swing
    pub swing: f32,
    /// 0..1 progress of roll (full body flip)
    pub roll: f32,
    /// 0..1 death fade (fall over)
    pub dead: f32,
    /// creeper fuse blink 0..1
    pub fuse: f32,
    pub cast: f32, // 0..1 channeling cast (arms up)
}

/// Player model built from the skin: per-part per-face uv rects (normalized).
pub struct SkinRects {
    /// [part_index][face] -> rect; part order: Head, Torso, ArmL, ArmR, LegL, LegR, HatOverlay, BodyOverlay, ArmLOverlay, ArmROverlay, LegLOverlay, LegROverlay
    pub parts: Vec<[Rect; 6]>,
    pub slim: bool,
}

fn part_rects(u0: u32, v0: u32, w: u32, h: u32, d: u32) -> [Rect; 6] {
    // Standard MC skin box unwrap, second row: [right][front][left][back]
    // ("right"/"left" = the CHARACTER's right/left when viewed from outside).
    // Returned slots follow FACE_* order:
    //   0 top(u0+d, v0)  1 bottom(u0+d+w, v0)  2 front(u0+d, v0+d)
    //   3 back(u0+d+w+d, v0+d)  4 +X = RIGHT side region(u0, v0+d)
    //   5 -X = LEFT side region(u0+d+w, v0+d)
    // combined skin texture is 64x128 (P1 at v=0, P2 at v=64)
    let (tex_w, tex_h) = (64.0f32, 128.0f32);
    let n = |x: u32, y: u32, w: u32, h: u32| -> Rect {
        [
            (x as f32 / tex_w * 65536.0) as u16,
            (y as f32 / tex_h * 65536.0) as u16,
            (w as f32 / tex_w * 65536.0) as u16,
            (h as f32 / tex_h * 65536.0) as u16,
        ]
    };
    [
        n(u0 + d, v0, w, d),
        n(u0 + d + w, v0, w, d),
        n(u0 + d, v0 + d, w, h),
        n(u0 + d + w + d, v0 + d, w, h),
        n(u0, v0 + d, d, h),
        n(u0 + d + w, v0 + d, d, h),
    ]
}

impl SkinRects {
    /// Build normalized uv rects for the standard MC skin layout (64x64).
    /// Rects are stored as u16 fixed point (uv * 65536) to stay Pod-friendly;
    /// the caller converts to f32 when filling instance data.
    /// `v_row` selects the player's half of the combined 64x128 skin texture
    /// (0.0 for player 1, 64.0 for player 2).
    pub fn from_skin_at(slim: bool, v_row: u32) -> SkinRects {
        let aw = if slim { 3 } else { 4 };
        let r = |u0: u32, v0: u32, w: u32, h: u32, d: u32| part_rects(u0, v0 + v_row, w, h, d);
        let parts = vec![
            r(0, 0, 8, 8, 8),          // Head
            r(16, 16, 8, 12, 4),       // Torso
            r(32, 48, aw, 12, 4),      // ArmL (modern left slot)
            r(40, 16, aw, 12, 4),      // ArmR
            r(16, 48, 4, 12, 4),       // LegL
            r(0, 16, 4, 12, 4),        // LegR
            r(32, 0, 8, 8, 8),         // Hat overlay
            r(16, 32, 8, 12, 4),       // Jacket overlay
            r(48, 48, aw, 12, 4),      // Sleeve L overlay
            r(40, 32, aw, 12, 4),      // Sleeve R overlay
            r(0, 48, 4, 12, 4),        // Pants L overlay
            r(0, 32, 4, 12, 4),        // Pants R overlay
        ];
        SkinRects { parts, slim }
    }
}

pub fn u16rect_to_f32(r: Rect) -> [f32; 4] {
    [
        r[0] as f32 / 65536.0,
        r[1] as f32 / 65536.0,
        r[2] as f32 / 65536.0,
        r[3] as f32 / 65536.0,
    ]
}

/// Player box model geometry (body-space), matching the skin layout.
/// 12 parts: 6 base + 6 overlay (overlay scaled 1.09, only pushed if used).
pub fn player_model(slim: bool) -> Vec<(PartId, Vec3, Vec3, Vec3)> {
    let aw = if slim { 0.1875 } else { 0.25 };
    let arm_x = 0.25 + aw / 2.0;
    vec![
        (PartId::Head, Vec3::new(0.0, 1.5, 0.0), Vec3::new(0.0, 0.25, 0.0), Vec3::new(0.5, 0.5, 0.5)),
        (PartId::Torso, Vec3::new(0.0, 0.75, 0.0), Vec3::new(0.0, 0.375, 0.0), Vec3::new(0.5, 0.75, 0.25)),
        (PartId::ArmL, Vec3::new(-arm_x, 1.4, 0.0), Vec3::new(0.0, -0.32, 0.0), Vec3::new(aw, 0.75, aw)),
        (PartId::ArmR, Vec3::new(arm_x, 1.4, 0.0), Vec3::new(0.0, -0.32, 0.0), Vec3::new(aw, 0.75, aw)),
        (PartId::LegL, Vec3::new(-0.125, 0.75, 0.0), Vec3::new(0.0, -0.375, 0.0), Vec3::new(0.25, 0.75, 0.25)),
        (PartId::LegR, Vec3::new(0.125, 0.75, 0.0), Vec3::new(0.0, -0.375, 0.0), Vec3::new(0.25, 0.75, 0.25)),
        // overlays (same pivots, inflated size)
        (PartId::Head, Vec3::new(0.0, 1.5, 0.0), Vec3::new(0.0, 0.25, 0.0), Vec3::new(0.53, 0.53, 0.53)),
        (PartId::Torso, Vec3::new(0.0, 0.75, 0.0), Vec3::new(0.0, 0.375, 0.0), Vec3::new(0.53, 0.78, 0.28)),
        (PartId::ArmL, Vec3::new(-arm_x, 1.4, 0.0), Vec3::new(0.0, -0.32, 0.0), Vec3::new(aw + 0.03, 0.78, aw + 0.03)),
        (PartId::ArmR, Vec3::new(arm_x, 1.4, 0.0), Vec3::new(0.0, -0.32, 0.0), Vec3::new(aw + 0.03, 0.78, aw + 0.03)),
        (PartId::LegL, Vec3::new(-0.125, 0.75, 0.0), Vec3::new(0.0, -0.375, 0.0), Vec3::new(0.28, 0.78, 0.28)),
        (PartId::LegR, Vec3::new(0.125, 0.75, 0.0), Vec3::new(0.0, -0.375, 0.0), Vec3::new(0.28, 0.78, 0.28)),
    ]
}

/// Push all boxes of a mob model with animation applied.
pub fn push_model(
    out: &mut Vec<crate::gfx::BoxInstance>,
    model: &MobModel,
    pos: Vec3,
    yaw: f32,
    anim: &AnimState,
    assets: &Assets,
    tint: Vec4,
) {
    let q_yaw = Quat::from_rotation_y(yaw);
    let scale = model.scale;

    // roll: full body flip around local X (forward axis)
    let (q_body, body_drop) = if anim.roll > 0.0 {
        (
            q_yaw * Quat::from_axis_angle(Vec3::X, -anim.roll * std::f32::consts::TAU),
            -0.2 * (1.0 - (anim.roll * 2.0 - 1.0).abs()),
        )
    } else if anim.dead > 0.0 {
        (q_yaw * Quat::from_axis_angle(Vec3::X, anim.dead * 1.4), -0.25 * anim.dead)
    } else {
        (q_yaw, 0.0)
    };

    let walk = anim.walk_phase;
    let mv = anim.moving;
    let swing_a = if anim.swing > 0.0 {
        let t = anim.swing;
        -2.1 * (1.0 - t) * t * 4.0 // fast up-down arc
    } else {
        0.0
    };
    let cast_a = anim.cast * 2.4;

    let part_angle = |id: PartId| -> f32 {
        match id {
            PartId::LegL => walk.sin() * 0.7 * mv,
            PartId::LegR => -walk.sin() * 0.7 * mv,
            PartId::ArmL => -walk.sin() * 0.55 * mv - cast_a,
            PartId::ArmR => walk.sin() * 0.55 * mv + swing_a.max(-2.1) * if anim.swing > 0.0 { 1.0 } else { 0.0 },
            PartId::Extra(0) | PartId::Extra(1) => walk.sin() * 0.5 * mv, // spider legs / creeper back legs
            PartId::Head => anim.dead * -0.5,
            _ => 0.0,
        }
    };

    for part in &model.parts {
        let local_a = part_angle(part.id);
        let q_loc = Quat::from_axis_angle(Vec3::X, local_a);
        let mut q = q_body * q_loc;
        // bat wings flap around Z; spider legs wiggle around Y a bit
        if matches!(part.id, PartId::Extra(0) | PartId::Extra(1)) && model.hover > 0.5 {
            q = q_body * Quat::from_axis_angle(Vec3::Z, walk.sin() * 0.9);
        }
        let center = pos
            + Vec3::new(0.0, model.hover * (1.0 + (walk * 2.0).sin() * 0.06) + body_drop, 0.0)
            + q_yaw * (part.pivot * scale + q_loc * (part.offset * scale));
        let s = part.size * scale;
        // cell() returns PIXEL rects in the 512x512 atlas; normalize to uv here.
        // (u16rect_to_f32 is only for the pre-normalized player-skin rects.)
        let uv = |key: &str| crate::gfx::rect_norm(assets.cell(key), crate::assets::ATLAS_DIM);
        let uv_top = uv(part.uv_top);
        let uv_bot = uv(part.uv_bottom);
        let uv_front = uv(part.uv_front);
        let uv_side = uv(part.uv_side);
        let mut tint_p = tint;
        if anim.fuse > 0.0 {
            let flash = (anim.fuse * 24.0).sin().abs();
            tint_p = Vec4::new(1.0 + flash * 0.9, 1.0 - flash * 0.4, 1.0 - flash * 0.4, tint.w);
        }
        out.push(crate::gfx::BoxInstance::new(
            center,
            q,
            s,
            [uv_top, uv_bot, uv_front, uv_side, uv_side, uv_side],
            tint_p,
            part.emissive,
        ));
    }
}

/// Push the player model from a 64x64 skin texture (uv rects normalized directly).
pub fn push_player(
    out: &mut Vec<crate::gfx::BoxInstance>,
    skin: &SkinRects,
    pos: Vec3,
    yaw: f32,
    anim: &AnimState,
    tint: Vec4,
) {
    let q_yaw = Quat::from_rotation_y(yaw);
    let (q_body, body_drop) = if anim.roll > 0.0 {
        (
            q_yaw * Quat::from_axis_angle(Vec3::X, -anim.roll * std::f32::consts::TAU),
            -0.2 * (1.0 - (anim.roll * 2.0 - 1.0).abs()),
        )
    } else if anim.dead > 0.0 {
        (q_yaw * Quat::from_axis_angle(Vec3::X, anim.dead * 1.4), -0.25 * anim.dead)
    } else {
        (q_yaw, 0.0)
    };
    let walk = anim.walk_phase;
    let mv = anim.moving;
    let swing_a = if anim.swing > 0.0 {
        let t = anim.swing;
        -2.1 * (1.0 - t) * t * 4.0
    } else {
        0.0
    };

    let geometry = player_model(skin.slim);
    let part_angle = |id: PartId| -> f32 {
        match id {
            PartId::LegL => walk.sin() * 0.7 * mv,
            PartId::LegR => -walk.sin() * 0.7 * mv,
            PartId::ArmL => -walk.sin() * 0.55 * mv,
            PartId::ArmR => walk.sin() * 0.55 * mv + swing_a,
            _ => 0.0,
        }
    };

    for (i, (id, pivot, offset, size)) in geometry.iter().enumerate() {
        // overlay part alpha check is skipped in v1: overlays drawn always (alpha discard handles empty)
        if i >= 6 {
            // overlays only drawn when a full skin is present; cheap: always draw (discard empty texels)
        }
        let local_a = part_angle(*id);
        let q_loc = Quat::from_axis_angle(Vec3::X, local_a);
        let q = q_body * q_loc;
        let center = pos + Vec3::new(0.0, body_drop, 0.0) + q_yaw * (*pivot + q_loc * *offset);
        let rects = &skin.parts[i];
        out.push(crate::gfx::BoxInstance::new(
            center,
            q,
            *size,
            [
                u16rect_to_f32(rects[0]),
                u16rect_to_f32(rects[1]),
                u16rect_to_f32(rects[2]),
                u16rect_to_f32(rects[3]),
                u16rect_to_f32(rects[4]),
                u16rect_to_f32(rects[5]),
            ],
            tint,
            0.0,
        ));
    }
}

// ----------------------------------------------------------------------
// Page titre MCD : camp statique derrière le héros (feu de camp + tente
// + caisses), d'après la capture du vrai menu (nuit, braseros, tente éclairée).
// ----------------------------------------------------------------------

/// Distance latérale du feu de camp par rapport au héros (axe "droite caméra").
pub const MENU_FIRE_OFF: f32 = 2.5;
/// Profondeur du feu (vers le fond de scène).
pub const MENU_FIRE_LOOK: f32 = 1.1;
/// (latéral, profondeur) de la tente par rapport au héros.
pub const MENU_TENT_OFF: (f32, f32) = (3.4, 2.2);

pub fn push_menu_camp(
    out: &mut Vec<crate::gfx::BoxInstance>,
    assets: &Assets,
    hero: Vec3,
    cam_yaw: f32,
    t: f32,
) {
    let right = Vec3::new(cam_yaw.cos(), 0.0, -cam_yaw.sin());
    let look = Vec3::new(-cam_yaw.sin(), 0.0, -cam_yaw.cos());
    let fire = hero - right * MENU_FIRE_OFF + look * MENU_FIRE_LOOK;
    let tent = hero + right * MENU_TENT_OFF.0 + look * MENU_TENT_OFF.1;
    let cell = |k: &str| crate::gfx::cell_norm(assets, k);

    // ---------------- feu de camp ----------------
    // double flaque de lumière chaude au sol (halo large + cœur vif)
    out.push(BoxInstance::new(
        Vec3::new(fire.x, 0.03, fire.z),
        Quat::IDENTITY,
        Vec3::new(1.7, 0.025, 1.7),
        [cell("lava"); 6],
        Vec4::new(0.22, 0.1, 0.03, 1.0),
        0.3,
    ));
    out.push(BoxInstance::new(
        Vec3::new(fire.x, 0.045, fire.z),
        Quat::IDENTITY,
        Vec3::new(1.05, 0.02, 1.05),
        [cell("lava"); 6],
        Vec4::new(0.55, 0.26, 0.07, 1.0),
        0.55,
    ));
    // charbon central
    out.push(BoxInstance::new(
        fire + Vec3::new(0.0, 0.05, 0.0),
        Quat::IDENTITY,
        Vec3::new(0.5, 0.07, 0.5),
        [cell("log_side"); 6],
        Vec4::new(0.14, 0.11, 0.09, 1.0),
        0.0,
    ));
    // anneau de pierres
    for i in 0..7 {
        let a = i as f32 / 7.0 * std::f32::consts::TAU;
        let s = 0.17 + 0.05 * ((i * 37) % 3) as f32;
        out.push(BoxInstance::new(
            fire + Vec3::new(a.cos() * 0.52, 0.08, a.sin() * 0.52),
            Quat::from_rotation_y(a),
            Vec3::splat(s),
            [cell("cobble"); 6],
            Vec4::new(0.6, 0.6, 0.64, 1.0),
            0.0,
        ));
    }
    // bûches croisées
    for i in 0..3 {
        let a = i as f32 / 3.0 * std::f32::consts::TAU + 0.5;
        out.push(BoxInstance::new(
            fire + Vec3::new(0.0, 0.15, 0.0),
            Quat::from_rotation_y(a),
            Vec3::new(0.8, 0.13, 0.13),
            [cell("log_side"); 6],
            Vec4::new(0.48, 0.34, 0.2, 1.0),
            0.0,
        ));
    }
    // flammes : plans croisés emissifs qui vacillent et tournent lentement
    let f1 = 0.88 + 0.12 * (t * 12.0).sin();
    let f2 = 0.6 + 0.1 * (t * 15.7 + 1.3).sin();
    let rot = t * 0.6;
    let flames: [(f32, f32, Vec4, f32); 4] = [
        (f1, 0.34, Vec4::new(1.0, 0.48, 0.1, 1.0), 0.95),
        (f1 * 0.9, 0.3, Vec4::new(1.0, 0.62, 0.16, 1.0), 0.95),
        (f2, 0.22, Vec4::new(1.0, 0.85, 0.35, 1.0), 1.0),
        (f2 * 0.85, 0.16, Vec4::new(1.0, 0.95, 0.6, 1.0), 1.0),
    ];
    for (i, (h, w, c, e)) in flames.into_iter().enumerate() {
        let yaw = rot + i as f32 * std::f32::consts::FRAC_PI_4;
        out.push(BoxInstance::new(
            fire + Vec3::new(0.0, 0.14 + h * 0.5, 0.0),
            Quat::from_rotation_y(yaw),
            Vec3::new(w, h, 0.06),
            [cell("lava"); 6],
            c,
            e,
        ));
    }

    // ---------------- tente (A-frame, façade vers la caméra) ----------------
    let tent_yaw = cam_yaw + std::f32::consts::PI;
    let qy = Quat::from_rotation_y(tent_yaw);
    let slope = 0.66f32;
    // toile : clay teintée orange = tissu
    for side in [-1.0f32, 1.0] {
        let ql = Quat::from_axis_angle(Vec3::X, side * slope);
        out.push(BoxInstance::new(
            tent + qy * Vec3::new(0.0, 0.72, side * -0.6),
            qy * ql,
            Vec3::new(2.5, 1.55, 0.07),
            [cell("clay"); 6],
            Vec4::new(1.0, 0.5, 0.18, 1.0),
            0.0,
        ));
    }
    // paroi arrière (plus sombre)
    out.push(BoxInstance::new(
        tent + qy * Vec3::new(0.0, 0.62, 1.0),
        qy,
        Vec3::new(2.15, 1.24, 0.07),
        [cell("clay"); 6],
        Vec4::new(0.72, 0.36, 0.13, 1.0),
        0.0,
    ));
    // lanterne à l'intérieur — lumière qui fuit par l'ouverture
    out.push(BoxInstance::new(
        tent + qy * Vec3::new(0.0, 0.45, 0.35),
        qy * Quat::from_rotation_y(t * 0.3),
        Vec3::splat(0.28),
        [cell("glowstone"); 6],
        Vec4::ONE,
        0.85,
    ));
    // flaque de lumière devant l'ouverture
    out.push(BoxInstance::new(
        tent + qy * Vec3::new(0.0, 0.025, -0.95),
        Quat::IDENTITY,
        Vec3::new(1.3, 0.022, 1.2),
        [cell("lava"); 6],
        Vec4::new(0.38, 0.2, 0.075, 1.0),
        0.4,
    ));

    // ---------------- caisses près du héros ----------------
    let c1 = hero + right * 2.0 - look * 1.4;
    out.push(BoxInstance::new(
        c1 + Vec3::new(0.0, 0.32, 0.0),
        Quat::from_rotation_y(0.42),
        Vec3::splat(0.64),
        [cell("planks_oak"); 6],
        Vec4::new(0.8, 0.62, 0.38, 1.0),
        0.0,
    ));
    let c2 = c1 + Vec3::new(0.1, 0.0, -0.75);
    out.push(BoxInstance::new(
        c2 + Vec3::new(0.0, 0.24, 0.0),
        Quat::from_rotation_y(-0.3),
        Vec3::splat(0.48),
        [cell("planks_oak"); 6],
        Vec4::new(0.72, 0.55, 0.33, 1.0),
        0.0,
    ));
}
