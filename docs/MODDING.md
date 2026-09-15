# Guide de modding

Tout le contenu « data » est du code Rust simple à modifier. Voici les points d'entrée.

## 1. Ajouter / modifier une mission

`src/world/missions.rs` — tableau `MISSIONS` :

```rust
MissionDef {
    id: 12,
    name: "Ma Mission",
    biome: Biome::Forest,
    threat: 25,              // niveau de menace => puissance du loot
    boss: Some(BossKind::NamelessOne),
    is_secret: false,
    requires: &[11],         // missions à terminer pour débloquer
}
```

Ajoute le biome à l'enum `Biome`, sa palette dans `world/gen.rs::biome_palette`
(blocs de sol, mur, densités d'arbres/eau, couleur de brouillard) et sa table
d'ennemis dans `missions.rs::biome_enemies`.

## 2. Ajouter un ennemi

1. `src/game/enemy.rs` — macro `def!` dans `DEFS` : PV, dégâts, vitesse, portée,
   cooldown, projectile éventuel, spécial (`None`, `Explode`, `Summon(..)`, `Charge`,
   `Volley(n)`), XP, résistance au knockback.
2. `src/models.rs::mob_model` — modèle boîtes (head/torso/arms/legs/extra) ;
   textures auto-générées `<nom>_skin` / `<nom>_face` (surchargeables via
   `assets/textures/override/`).
3. Ajoute-le aux tables `biome_enemies`.

## 3. Ajouter une arme / armure / artefact

- `src/game/items.rs` : tableaux `MELEE`, `RANGED`, `ARMOR`, `ARTIFACTS`
  (nom FR, stats, icône, cooldown), noms uniques dans `unique_name()`.
- Les icônes sont des clés d'atlas : dessine la tienne dans
  `assets.rs::load_all_icons` ou dépose `assets/textures/override/icon_machin.png`.

## 4. Ajouter un enchantement

`src/game/items.rs::Enchant` : variante + `name_fr`/`desc_fr`/`applies_to`,
puis effets dans `src/game/player.rs::melee_hit` / `shoot_arrow`.

## 5. Changer l'équilibrage

Tout est dans `src/consts.rs` (PV, vitesses, cooldowns, économie, XP, camera...).
Les multiplicateurs de difficulté `TIER_HP_MULT` / `TIER_ATK_MULT` proviennent des
tiers du wiki (55→210→500 PV du zombie).

## 6. Générer les niveaux autrement

`src/world/gen.rs::generate` : chemin sud→nord, salles, poches latérales, salle
secrète (levier + rune), arène, groupes de spawns déclenchés, coffres/captifs/
fontaines. La graine vient de `save.seed ⊕ mission ⊕ nb de parties`.

## 7. Rendu & modèles

- Boîtes : `gfx::BoxInstance` (position + quaternion + échelle + 6 UV + teinte +
  emissive). Tout le décor est « baké » dans `Level::bake_from`.
- Animation : `models.rs::AnimState` (marche, swing, roulade, mort, mèche creeper,
  incantation) appliquée par partie/bras/jambe via pivots.
- Shaders WGSL : `src/gfx/mod.rs` (const `BOX_SHADER`, `BILLBOARD_SHADER`, `QUAD_SHADER`).

## 8. Audio (v2)

`src/game/audio.rs` : brancher rodio — les appels `Audio::play(Sfx::...)` et
`set_music(Music::...)` sont déjà câblés partout dans le code.
