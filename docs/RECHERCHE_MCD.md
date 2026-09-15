# Recherche Minecraft Dungeons — synthèse (2026-08-30)

## Sources clés (dans assets/reference/ : 40 images ref_01..ref_40)
- ref_16 : collage gameplay officiel — caméra top-down proche, héros avec ANNEAUX colorés
  sous les pieds (P1 or, P2 rose/rouge), saturation élevée, ombres marquées, vignette.
- ref_36 : temple désert — lumière dorée chaude, brouillard atmosphérique dense,
  émeraudes/veux émissifs, contrastes forts (sols éclairés / murs sombres).
- ref_08 : carte de mission (fond sombre, contours blancs, titre jaune, compteur secrets/coffres).
- ref_31 : (hors-sujet Java) — ignoré.

## Recherches web (scripts/ref_search/web_*.json)
1. **VFX Mojang** (vfxapprentice.com) : MCD = caméra top-down assumée pour simplifier ;
   VFX très exagérés (particules, cercles au sol, impacts).
2. **Talk lighting ArtStation** ("Minecraft Dungeons Lighting and Tech Art") :
   éclairage stylisé, pools de lumière chaude + ambiance froide dans les ombres.
3. **Mod "Dungeons Perspective"** (CurseForge/Modrinth) : réplique la caméra MCD
   → pitch ~45-55° vers le bas, FOV 45°, caméra rotative. Notre réglage
   (pitch 58°, FOV 46°, dist 12.5) donne des personnages bien plus gros qu'avant (dist 15).
4. **Style artistique** (reddit) : le rejet d'un style "washed out" confirme que la
   signature MCD = palette VIBRANTE + lighting contrasté. → saturation 1.24 + split-tone
   chaud/froid appliqués dans le shader.
5. **Skin UV** : la confusion bras gauche/droite est LE bug classique des moteurs
   (r/Minecraft) ; layout moderne 64x64 : bras droit (40,16), bras gauche (32,48),
   jambes R (0,16) / L (16,48).
6. **HUD "Utility Bar"** (minecraft.wiki) : une barre unique = cœurs/PV, flèches, âmes,
   émeraudes, potions — notre HUD est conforme (PV + émeraude + potion + artefacts).

## Corrections apportées suite au diagnostic
- **Skin inversé (models.rs + shader)** : (a) régions gauche/droite ÉCHANGÉES corrigées
  dans part_rects() — la face +X reçoit la région DROITE du skin, -X la GAUCHE ;
  (b) les 4 faces latérales étaient en miroir (FACE_UVC) → u inversé = non-miroité.
- **Look MCD (shader)** : saturation ×1.24, split-tone (hautes lumières chaudes,
  ombres froides), FACE_SHADE contrasté (1.0/0.42/0.86/0.72/0.80), vignette cinéma.
- **Caméra** : pitch 55→58°, distance 15→12.5 (personnages ~30% plus gros).
- **Brouillard** par biome resserré (~×0.6) → ambiance enfermée type donjon.
- **Anneaux de sélection** : disque or (J1) / rouge (J2) sous les joueurs.
- **Menu** : titre 3D en couches (corps brun-rouge, face dorée) + fond charbon
  avec lueur de sol chaude.
