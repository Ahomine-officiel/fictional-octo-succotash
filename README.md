# MINECRAFT DUNGEONS — Remake non officiel (Rust + WGPU)

> **Fan project, usage privé uniquement.** Non affilié à Mojang Studios / Double Eleven / Microsoft.
> Les textures de ce dossier `assets/` ont été **extraites de fichiers fournis par
> l'utilisateur** : `client.jar` officiel 1.21.1 (Mojang) + `dungeons_1.19_mc1.21.1.jar`
> (mod « Dungeons », Modrinth R0ToJjk1) — usage privé uniquement, ne pas redistribuer.
> Sans ces fichiers, le jeu retombe proprement sur du pixel-art procédural.

Dungeon crawler action-RPG isométrique **1 à 2 joueurs en co-op locale**, inspiré fidèlement
de Minecraft Dungeons : camp de base, carte de campagne, menace croissante, émeraudes,
enchantements, artefacts, boss à phases et missions secrètes.

---

## 1. Build & lancement

Prérequis : [Rust](https://rustup.rs) (édition 2021, testé avec 1.98), et sur Linux
`pkg-config` + `libudev` (headers de dev : paquet `libudev-dev` / `libudev1-dev`).

```bash
cargo build --release
./target/release/mdungeons          # Linux
# Windows : target\release\mdungeons.exe
```

- **Linux** : dépendances système — `sudo apt install libudev-dev pkg-config` suffit
  (Wayland et X11 supportés automatiquement par winit).
- **Windows** : rien à installer de plus, `cargo build --release` fonctionne tel quel
  (XInput pour la manette).
- **macOS** : devrait compiler sans dépendance supplémentaire (non testé ici).

Binaire fourni : `target/release/mdungeons` (Linux x86_64, ~14 Mo, vérifié).

Tests : `cargo test --release` — 6 tests, dont un qui garantit que les cellules
de l'atlas (boss MCD, torsos, blocs, icônes) chargent de vrais fichiers et ne
retombent pas sur le procédural.

## 2. ASSETS INTÉGRÉS (textures Minecraft + skins MCD)

L'atlas embarque désormais **les vraies textures** :

- **Blocs vanilla** (16²) extraits du `client.jar` fourni : herbe, terre, souches,
  feuilles, pierre, grès, briques, netherrack, obsidienne, sculk... placés dans
  `assets/textures/minecraft/block/` ; icônes d'objets dans `item/` ;
- **Blocs stylisés Minecraft Dungeons** (101 textures) : les overrides du mod Dungeons
  remplacent les équivalents vanilla (terre, herbe, sable, pavé, briques de pierre,
  planches chêne, glace, neige, soulsand, netherrack, eau MCD...) — le jeu ressemble
  désormais au vrai MC Dungeons ;
- **Armes & artefacts MCD authentiques** : Épée en Diamant (diamond_sword_dungeon),
  Hache Double, Grand Marteau, dagues, gantelets, arc de chasse, émeraude MCD,
  potions, armures (Barde/Champion/Full Métal), Foudre (lightning_rod), Cor de Vent,
  Moissonneuse, Fûts Corrompus, feu d'artifice, clés MCD, cœur animé —
  recâblés sur les icônes `icon_*` / `art_*` du HUD et de l'inventaire ;
- **Skins des mobs Minecraft Dungeons** extraits du mod Dungeons fourni : squelette MCD,
  nécromancien, spectre, zombie des jungles, géomancien, **Arch-Illageois**, **Sans-Nom**,
  **Monstruosité Redstone**, **Chaudron Corrompu** (métal/potion/bois), golem redstone —
  recadrés en cellules 16² `<mob>_skin / _face / _body` (visage + torse + membres) ;
- **Skins joueurs officiels** : `assets/skins/steve.png` / `alex.png` (64×64 extraits
  du client.jar 1.21.1, overlays rendus — Steve et Alex authentiques dans l'éditeur héros).

Tu peux toujours **tout surcharger** en déposant un PNG dans `assets/textures/override/<clé>.png`
(ex. `override/zombie_face.png`). Tout fichier manquant est régénéré en pixel-art procédural.

### 2.1 Compatibilité fichiers vanilla

Dépose tes PNG (extraits de ta version du jeu, ou d'un resource pack) dans :

```
assets/textures/minecraft/
```

**Compatibilité totale avec les fichiers vanilla** (cf. fonctionnement des textures MC) :

- **Résolutions HD** : 32², 64², 128²... redimensionnées proprement en nearest ;
- **Textures animées** (eau, lave...) : MC les stocke en **bandes verticales de frames**
  (ex. `water_still.png` = 16×512) + un `.mcmeta` — le loader détecte la bande et prend
  la frame 0 (l'animation elle-même est prévue en v2) ;
- **Colormaps de biome** : `grass_block_top.png` et les feuilles vanilla sont en niveaux
  de gris, colorées par `colormap/grass.png` / `foliage.png` selon le biome — notre
  loader détecte les pixels gris et applique automatiquement les couleurs plaines
  (#91BD59) / feuillage ; si ta texture est déjà colorée, elle n'est pas retouchée ;
- **Skins** : 64×64 moderne, **HD 128²/512²**, legacy 64×32 (membres gauches recopiés),
  bras fins détectés, overlays rendus.

Noms reconnus (noms modernes 1.13+, fallback sur anciens noms) :

| clé du jeu       | fichiers acceptés                          |
|------------------|--------------------------------------------|
| herbe (dessus)   | `grass_block_top.png` / `grass.png`        |
| herbe (côté)     | `grass_block_side.png`                     |
| terre            | `dirt.png`                                 |
| pierre           | `stone.png`                                |
| pavé             | `cobblestone.png`                          |
| pavé moussu      | `mossy_cobblestone.png`                    |
| planches         | `oak_planks.png` / `planks_oak.png`        |
| bûche            | `oak_log.png` / `log_oak.png`              |
| feuilles         | `oak_leaves.png` / `jungle_leaves.png`     |
| sable            | `sand.png` / `red_sand.png`                |
| grès             | `sandstone_top.png`                        |
| briques pierre   | `stone_bricks.png` (+ `mossy_` / `cracked_`)|
| eau / lave       | `water_still.png` / `lava_still.png`       |
| nether           | `netherrack.png`, `nether_bricks.png`, `soul_sand.png` |
| pinacle          | `obsidian.png`, `purpur_block.png`, `end_stone.png` |
| mines            | `redstone_ore.png`, `glowstone.png`, `deepslate.png` |
| marais           | `mud.png`, `podzol_top.png`                |
| pièges/props     | `spawner.png`, `anvil.png`, `campfire.png`, `chest_front.png`... |

Tout ce qui manque est généré en pixel-art procédural (blocs « hors Minecraft » :
cristaux, cages, portails, coffres de donjon, bannières...).

**Priorité absolue** : tu peux surcharger N'IMPORTE QUELLE texture (même procédurale,
même les icônes d'objets/arts) en déposant un PNG dans :

```
assets/textures/override/<clé>.png      ex: override/zombie_face.png
```

### 2.2 TON skin Minecraft

```
assets/skins/skin.png
```

- Format standard **64×64** (les skins 64×32 anciens sont convertis automatiquement).
- Bras fins (slim/Alex) **détectés automatiquement**.
- Les calques overlay (chapeau, veste, manches, pantalon) sont rendus.
- Deux joueurs = skins distincts (P1 au choix, P2 = Alex), écran scindé en co-op.

### 2.3 Police

`assets/fonts/DejaVuSans.ttf` est incluse (licence libre Bitstream Vera).
Tu peux la remplacer par un autre TTF du même nom.

## 3. Commandes

**Joueur 1 (clavier/souris)** — les touches sont physiques : WASD marche aussi en AZERTY (ZQSD).

| Action            | Touche                     |
|-------------------|----------------------------|
| Déplacement       | ZQSD / WASD / flèches      |
| Attaque mêlée     | Clic gauche (maintenir = combo) |
| Tir (visée auto)  | Clic droit                 |
| Roulade (0,5 s d'invulnérabilité) | Espace |
| Artefacts 1/2/3   | 1 / 2 / 3                  |
| Potion de soin    | F                          |
| Interagir         | E ou X                     |
| Inventaire        | I ou Tab                   |
| Pause             | Échap                      |
| **Mode debug**    | **F3** (style Minecraft : FPS, XYZ, bloc, orientation, biome, entités, particules, groupes, seed) |

Menus : souris (survol + clic), clavier (flèches/WASD + Entrée, Échap = retour) ou manette
(croix-directionnelle + A, B = annuler). Échap depuis le camp revient au menu principal.
**RÉGLAGES HÉROS** (bouton du menu ou touche **H**) ouvre l'éditeur de héros : carrousel
de 8 tenues (Steve, Alex + 6 héros du pack MCD) avec aperçu **en direct** sur le héros 3D
du menu, puis APPLIQUER (Entrée) enregistre le choix dans la sauvegarde, RETOUR (Échap)
annule. La tenue choisie est portée dans le menu ET en jeu.

**Variables d'environnement (tests)** : `MD_AUTO=<id>` lance la mission `<id>` au démarrage,
`MD_BOT` (ou `MD_BOT=south`) fait marcher/attaquer le J1 tout seul (captures),
`MD_SCREEN=camp` ouvre directement le camp, `MD_SCREEN=hero` ouvre l'éditeur de héros,
`MD_DEBUG=1` active F3 au lancement, `MD_NOTERR` vide le décor, `MD_COOP` force le co-op,
`MD_MUTE` coupe l'audio.

**Joueur 2 (clavier) — ÉCRAN SCINDÉ** : activer « CO-OP ÉCRAN SCINDÉ » dans le camp
(onglet Carte) ou brancher une manette. L'écran se coupe en deux moitiés, chacune avec
sa caméra, son HUD compact et ses touches affichées. Les flèches passent alors du J1 au J2 :

| Action        | Touche (stable AZERTY/QWERTY) |
|---------------|-------------------------------|
| Déplacement   | Flèches                       |
| Mêlée         | U                             |
| Tir           | O                             |
| Roulade       | P                             |
| Artefacts     | J / K / L                     |
| Potion        | H                             |
| Interagir     | Y                             |

**Joueur 2 (manette)** — détection automatique, le joueur 2 rejoint à la création de la mission :

| Action        | Bouton (Xbox)            |
|---------------|--------------------------|
| Déplacement   | Stick gauche             |
| Mêlée         | A                        |
| Tir           | B                        |
| Roulade       | RB                       |
| Artefacts     | X / Y / L2               |
| Potion        | LB                       |
| Interagir     | Haut (croix directionnelle) |
| Menus         | Stick/Croix + A valider, B annuler |

## 4. Contenu du jeu (squelette complet)

- **Campagne fidèle** : Creeper Woods → Pumpkin Pastures → Cacti Canyon → Soggy Swamp
  (boss : Chaudron Corrompu) → **Soggy Cave (secrète)** → Redstone Mines (Monstruosité
  Redstone) → Desert Temple (le Sans-Nom) → **Arch Haven (secrète)** → Dingy Jungle
  (**Abomination de la Jungle**) → Stronghold → Nether Wastes (**Feu Sauvage**) →
  Obsidian Pinnacle (**Arch-Illageois**, 2 phases avec Cœur d'Ender).
- **Îles DLC** (débloquées après le Pinacle) : Hiver Grondant (**Spectre Repoussant**),
  Pics Hurleurs (**Golem Tempête**), Profondeurs Cachées (**Gardien Antique**) et
  Écho du Vide (**Cœur d'Ender Vengeur**, débloqué en finissant les trois autres).
- **16 biomes** avec palettes, brouillard, décoration et faune distincts.
- **54 types d'ennemis** — le bestiaire complet : zombie, husk, squelette, creeper,
  araignée, slime, pillard, vindicte, enchanteur, géomancien, nécromancien, sorcière,
  mooshroom, chauve-souris, golem redstone, blaze, spectre, noyé, zombie de la jungle,
  puis squelette moussu, squelette d'élite, squelette immergé, nécromancien noyé,
  murmureur, garde royal, illusionniste, zombie gelé, rêveur spectral, piglin,
  brute pigline, endling, glaciologue, alpiniste, siffle-vent, golem bourrasque,
  squelette wither, piglin zombifié, enderman, endermite, poisson d'argent,
  lapin meurtrier (Caerbannog), vex, sautefeuille, anémone vénéneuse, liane à dards,
  blastlin, snarelin, rampant des cavernes, mini-abomination, cube de magma, ghast,
  creeper glacé, hoglin — et **11 boss** à patterns dédiés.
- **Combat** : mêlée (5 archétypes d'armes, combos, critiques jaunes, étourdissement,
  repoussoir), distance (arcs/arbalètes, visée auto), roulade i-frames, 6 artefacts
  (Fulguro-lance, Totem de régénération, Corne de vent, Carquois récolteur, Flèche feu
  d'artifice, Graines corrompues), potion 3/mission.
- **Progression** : niveau de héros (+1 point d'enchantement/niveau, cap affiché 55),
  power = meilleur objet possédé, loot et ennemis adaptés à la menace, 3 niveaux de
  difficulté (Défaut/Aventure/Apocalypse — les deux derniers se débloquent en finissant).
- **8 enchantements** (Affûtage, Coup critique, Tourbillon, Fulguration, Éclat, Aspect
  ardent, Nuage poison, Gravité), 1 à 3 slots selon rareté, remboursés au rebattement.
- **Économie** : émeraudes (drops, coffres, captifs), marchand du camp (rechargé après
  chaque mission), forgeron (+1 puissance), objets communs/rare/uniques avec noms propres.
- **Co-op locale** : PV séparés, allié à terre relevable en 3 s, résurrection auto,
  émeraudes partagées, loot pour tous.
- **Sauvegarde** : `saves/save.json` (JSON, à côté du binaire).

## 5. Architecture (voir `docs/ARCHITECTURE.md` et `docs/MODDING.md`)

```
src/
├── main.rs            point d'entrée
├── app.rs             machine à états (menu → camp → mission → fin) + boucle winit
├── assets.rs          atlas de textures, loaders MC/skin, pixel-art procédural
├── consts.rs          balance (calée sur les wikis MC Dungeons)
├── gfx/               WGPU : pipelines (boîtes instanciées, billboards, UI), caméra, glyphes
├── models.rs          modèles boîtes (joueur skin, 54 mobs, 11 boss) + animation procédurale
├── input.rs           clavier/souris + manette gilrs
├── ui.rs              tous les écrans (HUD, inventaire, boutique, carte...)
├── world/             niveau (grille de blocs), génération procédurale, arbre des missions
└── game/              joueur, IA ennemis + boss, combat, loot, objets, boutique, sauvegarde
```

Rendu : **tout est instancié** — le monde entier est des boîtes texturées (comme le jeu
original), un draw call par texture, pas de vertex buffer (géométrie du cube générée dans
le shader). Cible 60 FPS sur matériel moyen.

## 6. Limitations connues (v1 « squelette complet »)

- Audio : architecture prête (`game/audio.rs`) mais muet — brancher rodio en v2.
- La coop en ligne et l'écran partagé scindé ne sont pas implémentés (écran partagé
  logique : une seule caméra qui cadre les deux joueurs).
- Les stats sont « calées sur le wiki » pour les ordres de grandeur (PV héros 100,
  zombie 55 PV, multiplicateurs de tier 1/3.8/9.1) mais ré-équilibrées pour la jouabilité.
- Pas de mini-carte ni de niveaux « à la MCD » en segments pré-authored : génération
  procédurale (chemin + salles + arène) avec graine stable par sauvegarde, habillée
  façon MCD (canopée, murets bas, ponts de planches, ruines, barils, lanternes,
  piliers d'arène, nappes de particules par biome).

## 7. Licence & mentions

- **Code** : MIT (le tien, modifie tout).
- **Police DejaVu** : licence Bitstream Vera (incluse avec la police).
- **Minecraft Dungeons** est une marque de Mojang/Microsoft. Ce projet n'en réutilise
  **aucun asset** ; les textures/skins restent la propriété de leurs auteurs et ne
  doivent pas être redistribués avec ce code. Usage privé uniquement.
