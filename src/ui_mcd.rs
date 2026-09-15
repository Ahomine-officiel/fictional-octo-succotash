//! UI « à l'identique » Minecraft Dungeons — d'après les captures officielles
//! (HUD console 01/07/2020, menu 61193, Settings 66458, VICTORY 78672,
//! YOU DIED 65399). Même contrat de rects que l'ancienne UI (app.rs inchangé).

use crate::gfx::text::SIZE_SMALL;
use crate::gfx::Vec4;
use crate::world::Decor;
use crate::{
    game::Game,
    gfx::camera::Camera,
};
use crate::ui::{Rect, Ui, ACCENT, GOLD, GOLD_BRIGHT, WHITE};

const SIZE_LOGO: usize = 2; // 44 px

// ----------------------------------------------------------------------
// Primitives MCD
// ----------------------------------------------------------------------

/// Bloc arrondi noir translucide du HUD console (chaque groupe est séparé).
fn hud_block(ui: &mut Ui, x: f32, y: f32, w: f32, h: f32) {
    let s = ui.scale();
    ui.rounded(x, y, w, h, 10.0 * s, [0.02, 0.02, 0.03, 0.55]);
    ui.rounded(x + 2.0 * s, y + 2.0 * s, w - 4.0 * s, h - 4.0 * s, 8.0 * s, [0.06, 0.06, 0.09, 0.45]);
}

/// Pastille de touche colorée console (A vert, B rouge, X bleu, Y jaune).
fn pad_chip(ui: &mut Ui, x: f32, cy: f32, label: &str, kind: usize) -> f32 {
    let s = ui.scale();
    let w = ui.measure(label, SIZE_SMALL) + 16.0 * s;
    let h = 26.0 * s;
    let col = match kind {
        0 => [0.30, 0.72, 0.28, 1.0],   // A vert
        1 => [0.86, 0.26, 0.22, 1.0],   // B rouge
        2 => [0.26, 0.52, 0.90, 1.0],   // X bleu
        3 => [0.94, 0.78, 0.20, 1.0],   // Y jaune
        _ => [0.35, 0.35, 0.40, 1.0],   // gâchette grise
    };
    ui.rounded(x, cy - h / 2.0, w, h, h / 2.0, [0.0, 0.0, 0.0, 0.6]);
    ui.rounded(x + 2.0 * s, cy - h / 2.0 + 2.0 * s, w - 4.0 * s, h - 4.0 * s, (h - 4.0 * s) / 2.0, col);
    ui.text(label, x + 8.0 * s, cy - ui.glyphs.px(SIZE_SMALL) * 0.55, SIZE_SMALL, WHITE);
    w
}

/// Gros cœur pixel-art 3 teintes (rouge clair / rouge / brun), style MCD.
fn mc_heart(ui: &mut Ui, x: f32, y: f32, size: f32) {
    let u = size / 9.0; // dessin en grille 9x8
    let light = [1.0, 0.42, 0.42, 1.0];
    let mid = [0.86, 0.16, 0.20, 1.0];
    let dark = [0.52, 0.07, 0.10, 1.0];
    // lignes de la grille (x, w) par ligne y — silhouette de cœur MC
    let rows: [(usize, f32, f32); 8] = [
        (0, 1.0, 3.0), // dessus gauche + droit (2 blocs séparés)
        (1, 0.0, 8.0),
        (2, 0.0, 8.0),
        (3, 0.0, 8.0),
        (4, 1.0, 6.0),
        (5, 2.0, 4.0),
        (6, 3.0, 2.0),
        (7, 3.5, 1.0),
    ];
    for (ry, rx, rw) in rows {
        let yy = y + ry as f32 * u;
        if ry == 0 {
            ui.rect(x + rx * u, yy, 3.0 * u, u, mid);
            ui.rect(x + (rx + 5.0) * u, yy, 3.0 * u, u, mid);
            // reflet clair
            ui.rect(x + rx * u, yy, u, u, light);
            continue;
        }
        ui.rect(x + rx * u, yy, rw * u, u, mid);
        if ry <= 2 {
            ui.rect(x + rx * u, yy, rw * u * 0.35, u, light);
        }
        if ry >= 5 {
            ui.rect(x + rx * u, yy, rw * u, u, dark);
        }
    }
}

/// Titre or extrudé (ombre dure bronze sous texte doré).
fn pixel_title(ui: &mut Ui, txt: &str, cx: f32, y: f32, size: usize) {
    let s = ui.scale();
    let px = ui.glyphs.px(size);
    let w = ui.measure(txt, size);
    let x = cx - w / 2.0;
    for (i, c) in [(4.0, [0.30, 0.20, 0.05, 0.9]), (2.0, [0.55, 0.38, 0.10, 0.95])] {
        ui.text(txt, x + i * s, y + i * s, size, c);
    }
    ui.text(txt, x, y, size, GOLD_BRIGHT);
    let _ = px;
}

/// Cadre à COINS CARRÉS des dialogs MCD (Settings, résumé de mission).
fn md_frame(ui: &mut Ui, x: f32, y: f32, w: f32, h: f32) {
    let s = ui.scale();
    ui.rect(x, y, w, h, [0.10, 0.05, 0.05, 0.97]);
    // liseré interne clair + coins carrés
    ui.rect(x, y, w, 3.0 * s, [0.55, 0.44, 0.30, 1.0]);
    ui.rect(x, y + h - 3.0 * s, w, 3.0 * s, [0.32, 0.24, 0.16, 1.0]);
    ui.rect(x, y, 3.0 * s, h, [0.48, 0.38, 0.26, 1.0]);
    ui.rect(x + w - 3.0 * s, y, 3.0 * s, h, [0.48, 0.38, 0.26, 1.0]);
    for (cx, cy) in [(x, y), (x + w - 10.0 * s, y), (x, y + h - 10.0 * s), (x + w - 10.0 * s, y + h - 10.0 * s)] {
        ui.rect(cx, cy, 10.0 * s, 10.0 * s, [0.72, 0.60, 0.42, 1.0]);
    }
}

/// Crâne pixel-art (marqueur boss / difficulté).
fn skull(ui: &mut Ui, x: f32, y: f32, size: f32) {
    let u = size / 8.0;
    let bone = [0.92, 0.90, 0.84, 1.0];
    let dark = [0.10, 0.08, 0.08, 1.0];
    ui.rect(x + u, y, 6.0 * u, 5.0 * u, bone);          // boîte crânienne
    ui.rect(x + 2.0 * u, y + 5.0 * u, 4.0 * u, 2.0 * u, bone); // mâchoire
    ui.rect(x + 2.0 * u, y + 1.5 * u, 1.6 * u, 1.6 * u, dark); // œil G
    ui.rect(x + 4.4 * u, y + 1.5 * u, 1.6 * u, 1.6 * u, dark); // œil D
    ui.rect(x + 3.4 * u, y + 3.6 * u, 1.2 * u, 1.2 * u, dark); // nez
}

/// Losange dorée (puce d'événement / bannière héros).
fn draw_diamond(ui: &mut Ui, cx: f32, cy: f32, r: f32, col: [f32; 4]) {
    let n = 5;
    for i in 0..n {
        let w = r * 2.0 * (1.0 - i as f32 / n as f32);
        ui.rect(cx - w / 2.0, cy - r + i as f32 * (r / n as f32), w, r / n as f32 + 1.0, col);
        ui.rect(cx - w / 2.0, cy + r - (i + 1) as f32 * (r / n as f32), w, r / n as f32 + 1.0, col);
    }
}

/// Texte wrapé simple, retourne le y après la dernière ligne.
fn wrap_text(ui: &mut Ui, txt: &str, x: f32, y: f32, max_w: f32, size: usize, c: [f32; 4]) -> f32 {
    let px = ui.glyphs.px(size);
    let mut cy = y;
    for word_line in txt.split('\n') {
        let mut line = String::new();
        for word in word_line.split_whitespace() {
            let test = if line.is_empty() { word.to_string() } else { format!("{} {}", line, word) };
            if ui.measure(&test, size) > max_w && !line.is_empty() {
                ui.text(&line, x, cy, size, c);
                cy += px * 1.25;
                line = word.to_string();
            } else {
                line = test;
            }
        }
        ui.text(&line, x, cy, size, c);
        cy += px * 1.25;
    }
    cy
}

/// Logo MINECRAFT DUNGEONS blocky (blanc espacé + DUNGEONS or extrudé + accolades).
fn mcd_logo(ui: &mut Ui, cx: f32, y: f32, t: f32) {
    let s = ui.scale();
    let bob = (t * 1.4).sin() * 2.0 * s;
    // MINECRAFT : blanc, très espacé
    let l1 = "M I N E C R A F T";
    let w1 = ui.measure(l1, SIZE_SMALL);
    ui.text(l1, cx - w1 / 2.0 + 1.5 * s, y + bob + 1.5 * s, SIZE_SMALL, [0.0, 0.0, 0.0, 0.85]);
    ui.text(l1, cx - w1 / 2.0, y + bob, SIZE_SMALL, WHITE);
    // DUNGEONS : or extrudé 3 couches
    let l2 = "DUNGEONS";
    let w2 = ui.measure(l2, SIZE_LOGO);
    let x2 = cx - w2 / 2.0;
    let y2 = y + 26.0 * s + bob;
    ui.text(l2, x2 + 3.0 * s, y2 + 3.0 * s, SIZE_LOGO, [0.34, 0.22, 0.05, 0.95]);
    ui.text(l2, x2 + 1.5 * s, y2 + 1.5 * s, SIZE_LOGO, [0.60, 0.40, 0.10, 0.95]);
    ui.text(l2, x2, y2, SIZE_LOGO, GOLD_BRIGHT);
    // accolades { } décoratives
    let brace_y = y2 - 6.0 * s;
    ui.text("{", x2 - 34.0 * s, brace_y, SIZE_LOGO, GOLD);
    ui.text("}", x2 + w2 + 14.0 * s, brace_y, SIZE_LOGO, GOLD);
}

// ----------------------------------------------------------------------
// Menu principal (capture 61193)
// ----------------------------------------------------------------------

pub fn draw_main_menu(
    ui: &mut Ui,
    sel: usize,
    t: f32,
    level: u32,
    power: u32,
    emeralds: u32,
    hero_open: bool,
) -> Vec<Rect> {
    let s = ui.scale();
    let cx = ui.w / 2.0;
    let (w, h) = (ui.w, ui.h);
    let pxs = ui.glyphs.px(SIZE_SMALL);

    // voile nuit + poussière d'or (la scène 3D est derrière, cf. app.rs)
    ui.rect(0.0, 0.0, w, h, [0.008, 0.012, 0.04, 0.56]);
    ui.rect(0.0, h * 0.78, w, h * 0.22, [0.0, 0.0, 0.0, 0.30]);
    for i in 0..20 {
        let x = ((i as f32 * 137.5 + t * (6.0 + i as f32 * 0.5)) % (w + 40.0)) - 20.0;
        let y = h - ((i as f32 * 91.3 + t * (10.0 + i as f32 * 0.9)) % (h + 40.0)) + 20.0;
        let tw = 0.5 + 0.5 * (t * 2.0 + i as f32).sin();
        ui.rect(x, y, 3.0, 3.0, [1.0, 0.82, 0.4, 0.07 + 0.09 * tw]);
    }

    let mut rects = Vec::new();

    // logo + bannière de saison (haut)
    mcd_logo(ui, cx, 14.0 * s, t);
    season_banner(ui, w - 18.0 * s - 300.0 * s, 14.0 * s, 300.0 * s, 76.0 * s, t);

    // bandeau co-op (haut droite, sous la bannière) + manette pixel
    {
        let txt = "Branchez deux manettes ou plus pour jouer en co-op locale";
        let tw = ui.measure(txt, SIZE_SMALL);
        let bw = tw + 58.0 * s;
        let bh = 38.0 * s;
        let bx = w - bw - 18.0 * s;
        let by = 14.0 * s + 76.0 * s + 8.0 * s;
        hud_block(ui, bx, by, bw, bh);
        let gx = bx + 12.0 * s;
        let gy = by + bh / 2.0;
        let gc = [0.92, 0.92, 0.95, 0.95];
        ui.rect(gx, gy - 6.0 * s, 26.0 * s, 12.0 * s, gc);
        ui.rounded(gx - 2.0 * s, gy - 9.0 * s, 8.0 * s, 16.0 * s, 3.0 * s, gc);
        ui.rounded(gx + 20.0 * s, gy - 9.0 * s, 8.0 * s, 16.0 * s, 3.0 * s, gc);
        let _ = ui;
        ui.rect(gx + 4.0 * s, gy - 3.5 * s, 6.0 * s, 2.0 * s, [0.05, 0.05, 0.08, 1.0]);
        ui.rect(gx + 16.0 * s, gy - 3.5 * s, 6.0 * s, 2.0 * s, [0.05, 0.05, 0.08, 1.0]);
        ui.text(txt, bx + 46.0 * s, by + bh / 2.0 - pxs * 0.55, SIZE_SMALL, WHITE);
    }

    // stats héros (droite du personnage) : NV hexagone / épée puissance / émeraude
    hero_stats(ui, cx + 115.0 * s, h * 0.36, level, power, emeralds);

    if hero_open {
        return rects;
    }

    // gros bouton vert COMMENCER LA PARTIE (bas gauche) + entête offline
    {
        let pw = 316.0 * s;
        let ph_h = 40.0 * s;
        let gap = 6.0 * s;
        let pb_h = 74.0 * s;
        let px = 18.0 * s;
        let py = h - 42.0 * s - ph_h - gap - pb_h;
        ui.rounded_frame(px, py, pw, ph_h, 6.0 * s, [0.04, 0.16, 0.06, 1.0], [0.07, 0.25, 0.09, 1.0], 2.0);
        let hy = py + ph_h / 2.0 - pxs * 0.55;
        ui.text_bold("PARTIE HORS-LIGNE", px + 14.0 * s, hy, SIZE_SMALL, WHITE);
        let l1 = ui.measure("PARTIE HORS-LIGNE", SIZE_SMALL);
        ui.rect(px + 14.0 * s + l1 + 12.0 * s, py + 8.0 * s, 2.0, ph_h - 16.0 * s, [1.0, 1.0, 1.0, 0.25]);
        ui.text("CHANGER", px + 14.0 * s + l1 + 24.0 * s, hy, SIZE_SMALL, [0.82, 0.93, 0.82, 1.0]);
        let cw = ui.measure("A", SIZE_SMALL) + 16.0 * s;
        pad_chip(ui, px + pw - cw - 10.0 * s, py + ph_h / 2.0, "A", 0);
        let by = py + ph_h + gap;
        let pulse = 0.5 + 0.5 * (t * 3.2).sin();
        if sel == 0 {
            ui.rounded(px - 3.0, by - 3.0, pw + 6.0, pb_h + 6.0, 9.0 * s, [1.0, 1.0, 1.0, 0.18 + 0.12 * pulse]);
        }
        let face = if sel == 0 {
            lerp4([0.34, 0.69, 0.28, 1.0], [0.47, 0.86, 0.38, 1.0], pulse)
        } else {
            [0.28, 0.58, 0.23, 1.0]
        };
        let edge = if sel == 0 { [0.88, 1.0, 0.85, 1.0] } else { [0.14, 0.36, 0.11, 1.0] };
        ui.rounded(px, by, pw, pb_h, 7.0 * s, edge);
        ui.rounded(px + 2.5, by + 2.5, pw - 5.0, pb_h - 5.0, 5.5 * s, face);
        let lbl = "COMMENCER LA PARTIE";
        let lw = ui.measure(lbl, SIZE_SMALL);
        ui.text_bold(lbl, px + pw / 2.0 - lw / 2.0, by + pb_h / 2.0 - pxs * 0.55, SIZE_SMALL, WHITE);
        let cwe = ui.measure("Entrée", SIZE_SMALL) + 16.0 * s;
        pad_chip(ui, px + pw - cwe - 12.0 * s, by + pb_h / 2.0, "Entrée", 4);
        rects.push([px, by, pw, pb_h]);
    }

    // version du jeu — coin bas-gauche, sous le bouton vert
    {
        let px = 18.0 * s;
        let ver = concat!("v", env!("CARGO_PKG_VERSION"));
        let vw = ui.measure(ver, SIZE_SMALL);
        ui.rect(px + 2.0, h - 30.0 * s - 2.0, vw + 10.0 * s, ui.glyphs.px(SIZE_SMALL) + 4.0, [0.0, 0.0, 0.0, 0.35]);
        ui.text(ver, px + 7.0 * s, h - 30.0 * s, SIZE_SMALL, [0.82, 0.82, 0.88, 0.95]);
    }

    // nom du héros + panneau CHANGER DE HÉROS (bas centre)
    {
        let hw = 262.0 * s;
        let hh = 46.0 * s;
        let hr = [cx - hw / 2.0, h * 0.575, hw, hh];
        let hero_name = "STEVE";
        ui.text_centered(hero_name, cx, hr[1] - 24.0 * s, SIZE_SMALL, [0.75, 0.75, 0.8, 1.0]);
        if sel == 1 {
            ui.rect(hr[0] - 3.0, hr[1] - 3.0, hw + 6.0, hh + 6.0, [0.0, 0.0, 0.0, 0.85]);
            ui.rect(hr[0] - 1.5, hr[1] - 1.5, hw + 3.0, hh + 3.0, [0.95, 0.95, 0.98, 0.92]);
        }
        ui.rounded_frame(hr[0], hr[1], hw, hh, 6.0 * s, [0.45, 0.45, 0.5, 1.0], [0.08, 0.08, 0.11, 0.95], 2.0);
        ui.text_bold("CHANGER DE HÉROS", hr[0] + 16.0 * s, hr[1] + hh / 2.0 - pxs * 0.55, SIZE_SMALL, WHITE);
        let ckh = ui.measure("H", SIZE_SMALL) + 16.0 * s;
        pad_chip(ui, hr[0] + hw - ckh - 10.0 * s, hr[1] + hh / 2.0, "H", 4);
        rects.push(hr);
    }

    // footer bas droite : Accessibilité / Options / Quitter
    {
        let items = [("Accessibilité", "F1"), ("Options", "F2"), ("Quitter", "Échap")];
        let bw = 150.0 * s;
        let bh = 44.0 * s;
        let mut bx = w - 18.0 * s - (bw * items.len() as f32 + 12.0 * s * (items.len() - 1) as f32);
        let by = h - 18.0 * s - bh;
        for (i, (label, key)) in items.iter().enumerate() {
            let r = [bx, by, bw, bh];
            ui.dark_button(r[0], r[1], r[2], r[3], label, key, false);
            bx += bw + 12.0 * s;
            let _ = i;
        }
    }

    rects
}

fn hero_stats(ui: &mut Ui, x: f32, y: f32, level: u32, power: u32, emeralds: u32) {
    let s = ui.scale();
    let row = |ui: &mut Ui, yy: f32, col: [f32; 4], val: &str| {
        // hexagone plein (NV) / losange épée / émeraude
        draw_diamond(ui, x + 12.0 * s, yy + 14.0 * s, 13.0 * s, col);
        ui.text_bold(val, x + 34.0 * s, yy + 4.0 * s, SIZE_SMALL, WHITE);
    };
    row(ui, y, [0.62, 0.45, 0.95, 1.0], &format!("NV {}", level));
    row(ui, y + 34.0 * s, [0.4, 0.75, 0.95, 1.0], &format!("{}", power));
    row(ui, y + 68.0 * s, [0.30, 0.85, 0.45, 1.0], &format!("{}", emeralds));
}

/// Bannière de saison (S1 L'Ascension Nuageuse) — version compacte.
fn season_banner(ui: &mut Ui, x: f32, y: f32, bw: f32, bh: f32, t: f32) {
    let s = ui.scale();
    let bob = (t * 1.2).sin() * 1.5 * s;
    let y = y + bob;
    ui.rect(x, y, bw, bh, [0.05, 0.07, 0.12, 0.85]);
    ui.rect(x, y, bw, 3.0 * s, GOLD);
    ui.rect(x, y + bh - 3.0 * s, bw, 3.0 * s, GOLD);
    ui.rect(x, y, 3.0 * s, bh, GOLD);
    ui.rect(x + bw - 3.0 * s, y, 3.0 * s, bh, GOLD);
    // médaillon "50" bleu
    let ms = 56.0 * s;
    ui.rounded(x + 10.0 * s, y + (bh - ms) / 2.0, ms, ms, 8.0 * s, [0.25, 0.45, 0.80, 1.0]);
    let n = "50";
    let nw = ui.measure(n, SIZE_SMALL);
    ui.text(n, x + 10.0 * s + ms / 2.0 - nw / 2.0, y + bh / 2.0 - ui.glyphs.px(SIZE_SMALL) * 0.55, SIZE_SMALL, WHITE);
    let tx = x + ms + 20.0 * s;
    ui.text_bold("SAISON 1", tx, y + 12.0 * s, SIZE_SMALL, GOLD_BRIGHT);
    ui.text("L'ASCENSION NUAGEUSE", tx, y + 32.0 * s, SIZE_SMALL, WHITE);
    ui.bar(tx, y + bh - 18.0 * s, bw - ms - 40.0 * s, 6.0 * s, 0.66, ACCENT, [0.15, 0.15, 0.2, 1.0]);
    ui.text("AP", tx + bw - ms - 60.0 * s, y + bh - 24.0 * s, SIZE_SMALL, GOLD);
}

fn lerp4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

// ----------------------------------------------------------------------
// HUD console MCD (captures 83444 / 80056 / 23197)
// ----------------------------------------------------------------------

pub fn draw_hud(ui: &mut Ui, game: &Game, cam: &Camera) -> Vec<Rect> {
    let s = ui.scale();
    let rects: Vec<Rect> = Vec::new();
    let m = &crate::world::missions::MISSIONS[game.mission_id];
    let p = match game.players.first() {
        Some(p) => p,
        None => return rects,
    };

    // ---- objectif (haut droite) : objectif blanc gras + nom de mission or ----
    {
        let total_captives = game.level.captives.len() as u32;
        let portal_active = game.level.portal.map(|(_, a)| a).unwrap_or(false);
        let obj = if total_captives > 0 && game.captives_rescued < total_captives {
            format!("LIBÉREZ LES VILLAGEOIS — {}/{}", game.captives_rescued, total_captives)
        } else if portal_active {
            "ENTREZ DANS LE PORTAIL".to_string()
        } else {
            "PROGRESSEZ DANS LA ZONE".to_string()
        };
        let ow = ui.measure(&obj, SIZE_SMALL);
        ui.text_bold(&obj, ui.w - ow - 22.0 * s, 14.0 * s, SIZE_SMALL, WHITE);
        let nw = ui.measure(m.name, SIZE_SMALL);
        ui.text(m.name, ui.w - nw - 22.0 * s, 34.0 * s, SIZE_SMALL, GOLD_BRIGHT);
    }

    // ---- bannière d'ouverture « NOUVEL OBJECTIF » (fade 5 s) ----
    if game.time < 5.0 {
        let a = ((5.0 - game.time) / 1.5).clamp(0.0, 1.0);
        let bw = 620.0 * s;
        let bx = (ui.w - bw) / 2.0;
        let by = ui.h * 0.13;
        ui.rect(bx, by, bw, 96.0 * s, [0.0, 0.0, 0.0, 0.62 * a]);
        ui.rect(bx, by, bw, 3.0 * s, [1.0, 1.0, 1.0, 0.85 * a]);
        ui.rect(bx, by + 93.0 * s, bw, 3.0 * s, [1.0, 1.0, 1.0, 0.85 * a]);
        let t1 = "NOUVEL OBJECTIF";
        let t1w = ui.measure(t1, SIZE_SMALL);
        ui.text(t1, (ui.w - t1w) / 2.0, by + 14.0 * s, SIZE_SMALL, GOLD_BRIGHT);
        let total_captives = game.level.captives.len() as u32;
        let t2 = if total_captives > 0 {
            format!("Libérez les villageois ({}/{})", game.captives_rescued, total_captives)
        } else {
            "Progressez dans la zone".to_string()
        };
        let t2w = ui.measure(&t2, SIZE_LOGO_HUD);
        ui.text(&t2, (ui.w - t2w) / 2.0, by + 38.0 * s, SIZE_LOGO_HUD, [1.0, 1.0, 1.0, a]);
    }

    // ---- toast d'événement (portail / boss) : losange or + « Nouvel événement » ----
    {
        let portal_active = game.level.portal.map(|(_, a)| a).unwrap_or(false);
        let boss_hurt = game
            .boss_entity
            .and_then(|e| game.world.get::<&crate::game::Health>(e).ok())
            .map(|h| h.hp < h.max)
            .unwrap_or(false);
        let msg = if portal_active {
            Some("Le portail s'est ouvert !")
        } else if boss_hurt {
            Some("Un boss approche…")
        } else {
            None
        };
        if let Some(msg) = msg {
            let bw = ui.measure(msg, SIZE_SMALL) + 70.0 * s;
            let bx = (ui.w - bw) / 2.0;
            let by = ui.h * 0.30;
            hud_block(ui, bx, by, bw, 46.0 * s);
            draw_diamond(ui, bx + 22.0 * s, by + 23.0 * s, 10.0 * s, GOLD_BRIGHT);
            ui.text_bold(msg, bx + 42.0 * s, by + 14.0 * s, SIZE_SMALL, WHITE);
        }
    }

    // ---- barre du bas : blocs SÉPARÉS façon console ----
    let bh = 88.0 * s;
    let by = ui.h - bh - 14.0 * s;
    let mut x = 14.0 * s;

    // 1) rond carte (M)
    {
        let bw = 74.0 * s;
        ui.rounded(x, by, bw, bh, bh / 2.0, [0.02, 0.02, 0.03, 0.55]);
        ui.rounded(x + 3.0 * s, by + 3.0 * s, bw - 6.0 * s, bh - 6.0 * s, (bh - 6.0 * s) / 2.0, [0.08, 0.09, 0.12, 0.6]);
        ui.icon("icon_map", x + 20.0 * s, by + 24.0 * s, 34.0 * s, WHITE);
        let cw = ui.measure("M", SIZE_SMALL) + 14.0 * s;
        pad_chip(ui, x + bw / 2.0 - cw / 2.0, by + bh - 2.0 * s, "M", 4);
        x += bw + 10.0 * s;
    }
    // 2) coffre (notifie quand du butin traîne au sol)
    {
        let bw = 62.0 * s;
        ui.rounded(x, by, bw, bh, 10.0 * s, [0.02, 0.02, 0.03, 0.55]);
        ui.rounded(x + 3.0 * s, by + 3.0 * s, bw - 6.0 * s, bh - 6.0 * s, 8.0 * s, [0.08, 0.09, 0.12, 0.6]);
        ui.icon("chest_front", x + 12.0 * s, by + 22.0 * s, 38.0 * s, WHITE);
        let loot_ground = game.pickups.iter().any(|pk| matches!(pk.kind, crate::game::combat::PickupKind::Item(_)));
        if loot_ground {
            // pastille jaune « ! »
            ui.rounded(x + bw - 16.0 * s, by - 6.0 * s, 22.0 * s, 22.0 * s, 11.0 * s, [0.95, 0.78, 0.15, 1.0]);
            ui.text_bold("!", x + bw - 16.0 * s + 7.0 * s, by - 6.0 * s + 3.0 * s, SIZE_SMALL, [0.1, 0.08, 0.0, 1.0]);
        }
        x += bw + 10.0 * s;
    }
    // 3) artefacts 1/2/3 avec pastilles à cheval sur le bas
    if let Some(pl) = game.players.first() {
        for (i, slot) in pl.inventory.artifacts.iter().enumerate() {
            let bw = 64.0 * s;
            ui.rounded(x, by, bw, bh, 10.0 * s, [0.02, 0.02, 0.03, 0.55]);
            ui.rounded(x + 3.0 * s, by + 3.0 * s, bw - 6.0 * s, bh - 6.0 * s, 8.0 * s, [0.08, 0.09, 0.12, 0.6]);
            if let Some(item) = slot {
                ui.icon(item.icon(), x + 13.0 * s, by + 20.0 * s, 38.0 * s, WHITE);
                let cd = pl.artifact_cd[i];
                if cd > 0.0 {
                    let maxcd = crate::game::items::artifact_def(match item.class {
                        crate::game::items::ItemClass::Art(k) => k,
                        _ => crate::game::items::ArtifactKind::Totem,
                    })
                    .cd;
                    ui.rect(x, by, bw, bh * (cd / maxcd).clamp(0.0, 1.0), [0.0, 0.0, 0.0, 0.6]);
                }
            }
            let key = ["1", "2", "3"][i];
            let cw = ui.measure(key, SIZE_SMALL) + 14.0 * s;
            pad_chip(ui, x + bw / 2.0 - cw / 2.0, by + bh - 2.0 * s, key, 4);
            x += bw + 10.0 * s;
        }
    }
    // 4) bloc de vie : GROS CŒUR + x3 potions dessous + LV + barre XP violette
    {
        let bw = 210.0 * s;
        ui.rounded(x, by, bw, bh, 10.0 * s, [0.02, 0.02, 0.03, 0.55]);
        ui.rounded(x + 3.0 * s, by + 3.0 * s, bw - 6.0 * s, bh - 6.0 * s, 8.0 * s, [0.08, 0.09, 0.12, 0.6]);
        // cœur dépassant à gauche (socle)
        let hs = 52.0 * s;
        let hx = x + 14.0 * s;
        let hy = by + 12.0 * s;
        // socle sombre
        ui.rounded(hx - 6.0 * s, hy - 6.0 * s, hs + 12.0 * s, hs + 12.0 * s, 10.0 * s, [0.0, 0.0, 0.0, 0.65]);
        mc_heart(ui, hx, hy, hs);
        // potions x3 (rose) sous le cœur
        ui.icon("icon_potion", hx - 2.0 * s, hy + hs + 2.0 * s, 20.0 * s, [1.0, 0.55, 0.75, 1.0]);
        ui.text(&format!("x{}", p.potions), hx + 22.0 * s, hy + hs + 6.0 * s, SIZE_SMALL, [1.0, 0.65, 0.8, 1.0]);
        // LV + barre XP violette pleine largeur à droite du cœur
        let vx = hx + hs + 12.0 * s;
        ui.text_bold(&format!("LV {}", p.level), vx, by + 14.0 * s, SIZE_SMALL, WHITE);
        ui.bar(vx, by + 36.0 * s, x + bw - vx - 12.0 * s, 10.0 * s,
            p.xp as f32 / crate::consts::xp_for_level(p.level) as f32, [0.62, 0.40, 0.95, 1.0], [0.12, 0.10, 0.16, 1.0]);
        // PV numériques fins
        ui.text(&format!("{}/{} PV", p.hp as i32, p.max_hp() as i32), vx, by + 52.0 * s, SIZE_SMALL, DIM2);
        x += bw + 10.0 * s;
    }
    // 5) potion (LB)
    {
        let bw = 64.0 * s;
        ui.rounded(x, by, bw, bh, 10.0 * s, [0.02, 0.02, 0.03, 0.55]);
        ui.rounded(x + 3.0 * s, by + 3.0 * s, bw - 6.0 * s, bh - 6.0 * s, 8.0 * s, [0.08, 0.09, 0.12, 0.6]);
        ui.icon("icon_potion", x + 14.0 * s, by + 20.0 * s, 38.0 * s, WHITE);
        if p.potion_cd > 0.0 {
            let cd = p.potion_cd / crate::consts::POTION_COOLDOWN;
            ui.rect(x, by, bw, bh * cd, [0.0, 0.0, 0.0, 0.6]);
        }
        let cw = ui.measure("LB", SIZE_SMALL) + 14.0 * s;
        pad_chip(ui, x + bw / 2.0 - cw / 2.0, by + bh - 2.0 * s, "LB", 4);
        x += bw + 10.0 * s;
    }
    // 6) arc (RB) + flèches AU-DESSUS + émeraude à droite
    {
        let bw = 74.0 * s;
        // compte de flèches AU-DESSUS du bloc
        let at = format!("{}", p.arrows);
        let atw = ui.measure(&at, SIZE_SMALL);
        ui.icon("icon_arrow", x + 6.0 * s, by - 26.0 * s, 22.0 * s, WHITE);
        ui.text_bold(&at, x + 32.0 * s, by - 24.0 * s, SIZE_SMALL, WHITE);
        ui.rounded(x, by, bw, bh, 10.0 * s, [0.02, 0.02, 0.03, 0.55]);
        ui.rounded(x + 3.0 * s, by + 3.0 * s, bw - 6.0 * s, bh - 6.0 * s, 8.0 * s, [0.08, 0.09, 0.12, 0.6]);
        ui.icon("icon_bow", x + 16.0 * s, by + 20.0 * s, 40.0 * s, WHITE);
        let cw = ui.measure("RB", SIZE_SMALL) + 14.0 * s;
        pad_chip(ui, x + bw / 2.0 - cw / 2.0, by + bh - 2.0 * s, "RB", 4);
        x += bw + 12.0 * s;
        // émeraude HORS bloc, compte vert
        ui.icon("icon_emerald", x + 2.0 * s, by + 26.0 * s, 32.0 * s, WHITE);
        let em = format!("{}", game.emeralds);
        ui.text_bold(&em, x + 40.0 * s, by + 32.0 * s, SIZE_SMALL, [0.35, 0.9, 0.45, 1.0]);
    }

    // ---- popup d'interaction central (sombre, style MCD) ----
    if let Some((msg, _)) = interact_hint(game) {
        let bwid = ui.measure(&msg, SIZE_SMALL) + 44.0 * s;
        let bx = (ui.w - bwid) / 2.0;
        let by2 = ui.h * 0.66;
        ui.rect(bx, by2, bwid, 44.0 * s, [0.0, 0.0, 0.0, 0.72]);
        ui.rect(bx, by2, bwid, 2.0 * s, [1.0, 1.0, 1.0, 0.5]);
        ui.rect(bx, by2 + 42.0 * s, bwid, 2.0 * s, [1.0, 1.0, 1.0, 0.5]);
        ui.text_centered(&msg, ui.w / 2.0, by2 + 13.0 * s, SIZE_SMALL, WHITE);
    }
    let _ = cam;
    rects
}

const SIZE_LOGO_HUD: usize = 1; // ~24 px (taille intermédiaire de la glyphe cache)
const DIM2: [f32; 4] = [0.75, 0.75, 0.8, 1.0];

/// Indication d'interaction central (portail / captif) — version HUD console.
fn interact_hint(game: &Game) -> Option<(String, f32)> {
    let p = game.players.first()?;
    let dist = |a: glam::Vec2, b: glam::Vec2| (a - b).length();
    if let Some((ppos, active)) = game.level.portal {
        if active && dist(p.pos, ppos) < 3.0 {
            return Some(("Appuyez sur E — Entrer dans le portail".to_string(), 0.0));
        }
    }
    for c in &game.level.captives {
        if dist(p.pos, *c) < 2.6 {
            return Some(("Maintenez E — Libérer le villageois".to_string(), 0.0));
        }
    }
    None
}

// ----------------------------------------------------------------------
// Pause façon Settings MCD (fond rouge sombre, cadre à coins carrés)
// ----------------------------------------------------------------------

pub fn draw_pause(ui: &mut Ui, sel: usize) -> Vec<Rect> {
    let s = ui.scale();
    ui.rect(0.0, 0.0, ui.w, ui.h, [0.16, 0.04, 0.04, 0.86]);
    let pw = 560.0 * s;
    let ph = 520.0 * s;
    let px = (ui.w - pw) / 2.0;
    let py = (ui.h - ph) / 2.0;
    md_frame(ui, px, py, pw, ph);
    pixel_title(ui, "PAUSE", ui.w / 2.0, py + 34.0 * s, SIZE_LOGO_HUD + 1);
    let items = ["Reprendre", "Inventaire", "Abandonner la mission"];
    let mut rects = Vec::new();
    for (i, label) in items.iter().enumerate() {
        let r = [px + 60.0 * s, py + 120.0 * s + i as f32 * 74.0 * s, pw - 120.0 * s, 56.0 * s];
        // barre grise MCD : sélection = liseré blanc + pastille A
        if sel == i {
            ui.rect(r[0] - 3.0, r[1] - 3.0, r[2] + 6.0, r[3] + 6.0, [0.95, 0.95, 0.97, 1.0]);
        }
        ui.rect(r[0], r[1], r[2], r[3], [0.32, 0.30, 0.31, 1.0]);
        ui.rect(r[0], r[1], r[2], 3.0 * s, [0.45, 0.43, 0.44, 1.0]);
        let ty = r[1] + r[3] / 2.0 - ui.glyphs.px(SIZE_SMALL) * 0.55;
        ui.text_bold(label, r[0] + 18.0 * s, ty, SIZE_SMALL, WHITE);
        if sel == i {
            let cw = ui.measure("A", SIZE_SMALL) + 16.0 * s;
            pad_chip(ui, r[0] + r[2] - cw - 12.0 * s, r[1] + r[3] / 2.0, "A", 0);
        }
        rects.push(r);
    }
    // pied : Retour B
    let back = "Retour";
    let bw = ui.measure(back, SIZE_SMALL) + 40.0 * s;
    pad_chip(ui, px + pw / 2.0 - bw / 2.0, py + ph - 34.0 * s, "B", 1);
    ui.text(back, px + pw / 2.0 - bw / 2.0 + 24.0 * s, py + ph - 43.0 * s, SIZE_SMALL, WHITE);
    rects
}

// ----------------------------------------------------------------------
// Écrans de fin — VICTORY ! / VOUS ÊTES MORT (78672 / 65399)
// ----------------------------------------------------------------------

pub fn draw_end(ui: &mut Ui, game: &Game, victory: bool, sel: usize, _emeralds_before: u32, _xp_before: u32) -> Vec<Rect> {
    let s = ui.scale();
    let cx = ui.w / 2.0;
    let mut rects = Vec::new();
    let m = &crate::world::missions::MISSIONS[game.mission_id];
    if victory {
        ui.rect(0.0, 0.0, ui.w, ui.h, [0.04, 0.05, 0.10, 0.88]);
        // lauriers dorés autour de VICTORY !
        pixel_title(ui, "VICTORY !", cx, ui.h * 0.14, SIZE_LOGO);
        // lauriers simplifiés : arcs de feuilles dorées de part et d'autre
        let tw = ui.measure("VICTORY !", SIZE_LOGO);
        for side in [-1.0f32, 1.0] {
            let lx = cx + side * (tw / 2.0 + 46.0 * s);
            for i in 0..6 {
                let yy = ui.h * 0.14 + 20.0 * s + i as f32 * 12.0 * s;
                let xx = lx + side * (i as f32 * 6.0 * s);
                ui.rect(xx, yy, 12.0 * s, 7.0 * s, lerp4(GOLD, [1.0, 0.9, 0.5, 1.0], i as f32 / 5.0));
            }
        }
        ui.text_centered("LEVEL COMPLETE", cx, ui.h * 0.14 + 64.0 * s, SIZE_SMALL, GOLD_BRIGHT);
        let nm = m.name.to_uppercase();
        ui.text_centered(&nm, cx, ui.h * 0.14 + 92.0 * s, SIZE_LOGO_HUD + 1, WHITE);
        // bannière héros : losange dorée + nom
        draw_diamond(ui, cx, ui.h * 0.40, 16.0 * s, GOLD_BRIGHT);
        ui.text_centered("HÉROS — STEVE", cx, ui.h * 0.40 + 26.0 * s, SIZE_SMALL, WHITE);
        // RÉSUMÉ DE MISSION : 4 cartes
        let t1 = "RÉSUMÉ DE MISSION";
        let t1w = ui.measure(t1, SIZE_SMALL);
        ui.text(t1, cx - t1w / 2.0, ui.h * 0.52, SIZE_SMALL, GOLD_BRIGHT);
        let stats = [
            (format!("{}", game.kills), "ENNEMIS VAINCUS"),
            (format!("{}", game.emeralds), "ÉMERAUDES"),
            (format!("{}", game.captives_rescued), "VILLAGEOIS LIBÉRÉS"),
            (if game.rune_found { "OUI".to_string() } else { "NON".to_string() }, "RUNE TROUVÉE"),
        ];
        let cwid = 200.0 * s;
        let total = cwid * 4.0 + 18.0 * s * 3.0;
        let mut sx = cx - total / 2.0;
        for (val, label) in stats {
            ui.rect(sx, ui.h * 0.56, cwid, 90.0 * s, [0.02, 0.02, 0.03, 0.6]);
            ui.rect(sx, ui.h * 0.56, cwid, 3.0 * s, GOLD);
            let vw2 = ui.measure(&val, SIZE_LOGO_HUD + 1);
            ui.text(&val, sx + cwid / 2.0 - vw2 / 2.0, ui.h * 0.56 + 14.0 * s, SIZE_LOGO_HUD + 1, GOLD_BRIGHT);
            let lw2 = ui.measure(label, SIZE_SMALL);
            ui.text(label, sx + cwid / 2.0 - lw2 / 2.0, ui.h * 0.56 + 52.0 * s, SIZE_SMALL, WHITE);
            sx += cwid + 18.0 * s;
        }
        // boutons
        let labels = ["Continuer", "Rejouer"];
        for (i, label) in labels.iter().enumerate() {
            let r = [cx - 170.0 * s + i as f32 * 360.0 * s, ui.h * 0.80, 340.0 * s, 54.0 * s];
            let selr = sel == i;
            if selr {
                ui.rect(r[0] - 3.0, r[1] - 3.0, r[2] + 6.0, r[3] + 6.0, [0.95, 0.95, 0.97, 1.0]);
            }
            ui.rect(r[0], r[1], r[2], r[3], [0.32, 0.30, 0.31, 1.0]);
            let ty = r[1] + r[3] / 2.0 - ui.glyphs.px(SIZE_SMALL) * 0.55;
            ui.text_bold(label, r[0] + 18.0 * s, ty, SIZE_SMALL, WHITE);
            rects.push(r);
        }
    } else {
        ui.rect(0.0, 0.0, ui.w, ui.h, [0.10, 0.02, 0.02, 0.90]);
        // losange + crâne
        draw_diamond(ui, cx, ui.h * 0.24, 26.0 * s, [0.25, 0.05, 0.05, 1.0]);
        skull(ui, cx - 24.0 * s, ui.h * 0.24 - 22.0 * s, 48.0 * s);
        // VOUS ÊTES MORT — ombre rouge dure
        let t1 = "VOUS ÊTES MORT";
        let t1w = ui.measure(t1, SIZE_LOGO);
        ui.text(t1, cx - t1w / 2.0 + 4.0 * s, ui.h * 0.36 + 4.0 * s, SIZE_LOGO, [0.30, 0.02, 0.02, 1.0]);
        ui.text(t1, cx - t1w / 2.0, ui.h * 0.36, SIZE_LOGO, [0.92, 0.28, 0.25, 1.0]);
        ui.text_centered(&m.name, cx, ui.h * 0.36 + 62.0 * s, SIZE_SMALL, DIM2);
        // vies restantes : totem « 0 »
        ui.text_centered("VIES RESTANTES", cx, ui.h * 0.50, SIZE_SMALL, DIM2);
        ui.text_centered("0", cx, ui.h * 0.54, SIZE_LOGO, WHITE);
        // boutons Réessayer / Camp
        let labels = ["Réessayer", "Camp"];
        for (i, label) in labels.iter().enumerate() {
            let r = [cx - 170.0 * s + i as f32 * 360.0 * s, ui.h * 0.72, 340.0 * s, 54.0 * s];
            let selr = sel == i;
            if selr {
                ui.rect(r[0] - 3.0, r[1] - 3.0, r[2] + 6.0, r[3] + 6.0, [0.95, 0.95, 0.97, 1.0]);
            }
            ui.rect(r[0], r[1], r[2], r[3], [0.32, 0.30, 0.31, 1.0]);
            let ty = r[1] + r[3] / 2.0 - ui.glyphs.px(SIZE_SMALL) * 0.55;
            ui.text_bold(label, r[0] + 18.0 * s, ty, SIZE_SMALL, WHITE);
            rects.push(r);
        }
    }
    rects
}
