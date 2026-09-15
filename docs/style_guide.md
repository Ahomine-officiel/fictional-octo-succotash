# Minecraft Dungeons — Guide de style visuel
> Synthèse de la lecture des 40 images de référence (`assets/reference/ref_01..40`).
> Sert de bible pour la refonte graphique, le HUD, et le bestiaire.

---

## 1. Caméra & cadrage (refs 04, 09, 18, 26, 29, 34, 35, 37, 38)
- Vue **3/4 isométrique** : pitch ≈ 55–60°, yaw fixe 45°, FOV serré (~46°) → pas de distorsion.
- Le héros occupe ~8–10 % de la hauteur d'écran ; on voit largement devant lui (découverte du décor).
- **Vignette cinématique** sur les bords + brouillard coloré assorti au biome dans les lointains.
- Caméra lissée (pas de secousse), recentrage doux sur le joueur 1 ; en coop l'écran reste partagé.

## 2. Éclairage & ambiance (référence absolue : refs 03, 17, 23, 24, 29)
- Contraste **chaud/froid** systématique : sources ponctuelles orange (campements, torches, lave)
  posées sur des ambiants froids (bleu nuit des bois, cyan glacé, rouge nether, violet End).
- Les **yeux des monstres s'allument** dans les scènes sombres (Enderman : violet, Refs 14 ; redstone : rouge).
- Saturation poussée + teinte ombres bleutée / hautes lumières chaudes (déjà câblé dans le shader).
- Particules omniprésentes : braises, lucioles, flocons, poussière dorée, éclats de loot.

## 3. Palettes par biome (échantillonnées)
| Biome | Ambiant | Accent chaud | Ref |
|---|---|---|---|
| Creeper Woods | teal-vert profond `#0d3a38` | feu `#ff9a2a` | 17, 24 |
| Pumpkin Pastures | automne rouille `#b35a1f` | blé doré `#e8b04a` | 18, 26 |
| Desert Temple | sable `#d9b36a` | bannières rouges `#a32020` | 39, 35 |
| Nether / Crimson | rouge saturé `#8f1616` | lave `#ffb02e` | 23 |
| End / Void | violet `#5a2a8a` | magenta lumineux `#e05aff` | 19, 29 |
| Glace / Hiver | bleu cyan `#2a5f8f` | intérieurs orangés `#ff8c3a` | 03, 25 |
| Château / Pinacle | gris pierre froide | tapis rouge + bougies dorées | 34 |

## 4. HUD en jeu (refs 18, 26, 29, 34, 35, 37, 38)
- **Barre noire centrale en bas**, slots carrés sombres à bord fin clair, ordre :
  `âmes (gauche, icône violette) | arme mêlée | arme distance | artefact ×2 | ♥ CŒUR ROUGE GÉANT (HP) | armure | artefact | carte | flèches`
- Compteur de **munitions en badge hexagonal** à droite + **émeraudes** (icône gemme verte) tout à droite.
- Barre d'**XP fine violet/bleu** sous le cœur, niveau `LV nn` en pixel-font blanche.
- **Objectif en haut à droite** : titre blanc + sous-titre orange (ex : « FIND THE VILLAGE / THE LAST HEARTH »).
- **Barre de boss** en haut au centre : fond rouge, losanges de palier, nom blanc en capitales + **icône crâne**.
- **Dégâts flottants blancs** au-dessus des ennemis (40, 2 754…), chiffres dorés pour critiques.
- Prompt d'interaction : petite boîte noire arrondie « Push [A] ».
- Enchantement actif affiché à gauche : « ×2 Fire Aspect » avec icône.
- Badges de touche (lettres clavier / boutons manette) SOUS chaque slot.

## 5. Menus (refs 10, 21, 02)
- Fond sombre pierre/fer texturé, panneaux à **coins carrés et liseré doré**, grain métallique.
- **Écran de mission** : carte en relief, marqueurs hexagonaux dorés ; panneau gauche avec
  piste de difficulté EASY→HARD (crânes), « Recommended power », récompense.
- **Inventaire** : grille 3×N de cartes d'objet ; cadre = rareté ; badge de niveau en haut ;
  à droite : stats avec icônes, texte de lore en gris clair, boutons contextuels en bas (Back/Salvage/Enchant).
- Écran-titre : scène nocturne animée + feu de camp + « PRESS ANY BUTTON » clignotant (ref 17).
- Écran de chargement : artwork + « PROGRESS: NN% » orange pixel (ref 09).

## 6. Raretés des objets
| Rareté | Couleur cadre | Nom |
|---|---|---|
| Common | gris/blanc | — |
| Rare | vert | badge vert |
| Épique | bleu/violet | badge violet |
| **Unique** | **orange/doré** + tag « UNIQUE » | ref 10 |

## 7. Personnages & proportions
- Héros ≈ **2.1 têtes de haut**, bras trapus, épaulettes visibles, cape au vent pour l'Arch-Illageois.
- Illageois : robe sombre + grise pâle, yeux gris/rouges ; enchanteur : bras levés pour incanter.
- Mobs : proportions vanilla légèrement stylisées, textures **plus saturées** que vanilla.
- Boss = **échelle ×2.5 à ×3.5** du joueur, silhouette immédiatement lisible (Monstruosité Redstone :
  bloc gris/rouge géant, ref 01/22 ; Abomination jungle : mousse verte + lianes, ref 22).

## 8. Bestiaire — couverture demandée (refs 01, 02, 07, 14, 22, 23, 26)
Mobs visibles dans les refs et implantés : Zombie (+ variantes jungle/gelé/noyé), Squelette (+ moussu,
immergé, d'élite, wither), Creeper (+ glacé), Araignée, Slime/Magma, Pillard, Vindicte, Enchanteur,
Géomancien, Nécromancien, Sorcière, Illusionniste, Enderman (+ Endling, Endermite), Vex, Ghast, Blaze,
Piglin (+ brute, zombifié), Hoglin, Mooshroom, Chauve-souris, Poisson d'argent, Spectres (wraith,
wretched, kindler), plantes (anémone, liane à dards), Blastling/Snareling, golems (redstone, bourrasque,
tempête), leapleaf, caerbannog, alpiniste, glaciologue, siffle-vent…
Boss : Chaudron Corrompu, Monstruosité Redstone, Sans-Nom, Arch-Illageois → **Cœur d'Ender (phase 2)**,
Abomination de la Jungle, Spectre Repoussant, Golem Tempête, Gardien Antique, Feu Sauvage,
Cœur d'Ender Vengeur.

## 9. Décors & props récurrents
- Ponts de planches avec **barrières et torches** (ref 18), ruines moussues, statues brisées.
- **Bougies partout** dans les intérieurs (ref 34), tapis rouge orné des salles de trône.
- Branches/planches **cassables**, pots explosifs, tonneaux de poudre (TNT visible ref 13).
- Couloirs de donjon étroits débouchant sur des arènes larges pour les boss.

## 10. Checklist de conformité MCD (à valider en jeu)
- [x] Grade colorimétrique (saturation ×1.24, ombres froides, lumières chaudes) — shader.
- [x] Vignette + brouillard par biome.
- [x] Caméra iso 58°/45°.
- [x] HUD cœur central, objectif haut-droit, barre de boss crâne, dégâts flottants.
- [x] Correctif orientation : yaw mobs/objet/couleur face haut-bas des skins (UV MC unwrap).
- [x] Correctif tête : face du HAUT dé-miroirée (axe U) — cheveux/skins asymétriques OK.
- [x] Nappes de particules d'ambiance par biome (16 biomes : neige, braises, lucioles,
      poussière dorée, éclats de Vide…) + poussière or du menu.
- [x] Menus habillés pierre dorée : panneaux 9-slice (ui_stone + liseré or + studs),
      boutons pierre biseautés cadre or, fond = vraie scène 3D nocturne (camp Creeper
      Woods, orbite lente, feu de camp) — comme l'écran-titre MCD.
- [x] Combat lisible : épée en main (swing animé), arc de mêlée cyan (ref 46),
      bouffée de tir, dégâts flottants, bandeau « comment on combat » 14 s.
- [x] Niveaux façon MCD : canopée de feuilles sur le massif (biomes verts), murets bas
      le long des chemins, ponts de planches + rambardes, accotements gravel, ruines
      mossy/craquelées, barils/caisses, lanternes, braseros, piliers d'arène (ref 46),
      sols en plaques cohérentes (ref 04).
- [x] Mode debug F3 (FPS, XYZ, bloc, orientation, biome, entités, particules, seed).
