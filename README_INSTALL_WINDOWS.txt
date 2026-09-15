# Minecraft Dungeons — Fan Remake (Rust + WGPU) — Guide Windows

## Ou est le Cargo.toml ?
A la racine de ce zip : `Cargo.toml` (a cote de `Cargo.lock`, `src/` et `assets/`).

## Installation sur Windows (5 etapes)

1. **Installer Rust** (si pas deja fait) : https://rustup.rs
   -> Telecharge et lance `rustup-init.exe` (options par defaut).

2. **Extraire ce zip** ou tu veux, ex : `C:\Users\Toi\mdungeons`
   Tu dois voir `Cargo.toml` a la racine du dossier extrait.

3. **Compiler** — ouvre un terminal (cmd ou PowerShell) dans le dossier et lance :
   ```
   cargo build --release
   ```
   (Premiere compilation : 3 a 8 minutes, ca telecharge les crates wgpu/winit/etc.)

4. **Lancer le jeu** :
   ```
   cargo run --release
   ```
   ou directement : `target\release\mdungeons.exe`

5. **Manette (optionnel)** : gilrs est supporte — branche une manette XInput/Xbox
   avant de lancer le jeu.

## Controles (clavier)
- Deplacement : ZQSD / WASD / Fleches (8 directions)
- Attaque melee : clic gauche ou J
- Tir arc : clic droit ou K
- Potion : Q ou L
- Roulee (esquive) : Espace ou Maj
- Interagir : F
- Inventaire : I (ou TAB)
- Carte des missions : M
- Pause : Echap

## Notes
- Textures : le jeu charge les PNG depuis `assets/textures/` ; s'il manque un
  fichier, un fallback procede est genere automatiquement.
- Sauvegardes : dossier `saves/` (JSON, cree automatiquement).
- Projet a usage prive (fan-made, non affilie a Mojang/Microsoft).
