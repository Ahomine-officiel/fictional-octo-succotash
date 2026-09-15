//! Immediate-mode-ish UI drawn with instanced quads + glyph cache.
//! Each screen function returns its hotspot rects so the app can handle
//! mouse hover/clicks and keyboard/gamepad selection with one code path.

use crate::assets::Assets;
use crate::game::combat::{Floater, PickupKind};
use crate::game::items::{Enchant, Item, ItemClass, Rarity};
use crate::game::inventory::Inventory;
use crate::game::Game;
use crate::gfx::camera::Camera;
use crate::gfx::text::{GlyphCache, SIZE_SMALL, SIZE_TITLE};
use crate::gfx::{QuadInstance, Vec4};
use crate::world::Decor;
use hecs::Entity;

pub struct Ui<'a> {
    pub quads: &'a mut Vec<QuadInstance>,
    pub text: &'a mut Vec<QuadInstance>,
    pub glyphs: &'a GlyphCache,
    pub assets: &'a Assets,
    pub w: f32,
    pub h: f32,
    pub mouse: (f32, f32),
}

pub type Rect = [f32; 4]; // x, y, w, h

const PANEL: [f32; 4] = [0.07, 0.08, 0.12, 0.93];
const PANEL_DARK: [f32; 4] = [0.05, 0.05, 0.08, 0.95];
pub const BORDER: [f32; 4] = [0.72, 0.58, 0.28, 1.0];
pub const ACCENT: [f32; 4] = [0.95, 0.8, 0.35, 1.0];
pub const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const DIM: [f32; 4] = [0.72, 0.72, 0.78, 1.0];
const GREEN: [f32; 4] = [0.35, 0.85, 0.35, 1.0];
const RED: [f32; 4] = [0.9, 0.25, 0.25, 1.0];
const PURPLE: [f32; 4] = [0.6, 0.4, 0.95, 1.0];
/// MCD gold palette (bright face, dark bronze shadow)
pub const GOLD: [f32; 4] = [0.88, 0.69, 0.28, 1.0];
pub const GOLD_BRIGHT: [f32; 4] = [1.0, 0.86, 0.45, 1.0];
pub const BRONZE: [f32; 4] = [0.48, 0.35, 0.16, 1.0];

impl<'a> Ui<'a> {
    pub fn scale(&self) -> f32 {
        (self.h / 720.0).clamp(0.7, 2.0)
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: [f32; 4]) {
        self.quads.push(QuadInstance {
            pos: [x, y],
            size: [w, h],
            uv: [0.0; 4],
            color: c,
            flag: 0.0,
            _pad: 0.0,
        });
    }

    pub fn frame(&mut self, x: f32, y: f32, w: f32, h: f32, bg: [f32; 4], border: [f32; 4]) {
        self.rect(x, y, w, h, bg);
        let b = 2.0;
        self.rect(x, y, w, b, border);
        self.rect(x, y + h - b, w, b, border);
        self.rect(x, y, b, h, border);
        self.rect(x + w - b, y, b, h, border);
    }

    /// Push a textured quad sampling `sub` (cell-local 0..1 rect) of a cell.
    pub fn tex(&mut self, key: &str, sub: [f32; 4], x: f32, y: f32, w: f32, h: f32, tint: [f32; 4]) {
        let c = crate::gfx::rect_norm(self.assets.cell(key), crate::assets::ATLAS_DIM);
        let uv = [
            c[0] + sub[0] * c[2],
            c[1] + sub[1] * c[3],
            sub[2] * c[2],
            sub[3] * c[3],
        ];
        self.quads.push(QuadInstance {
            pos: [x, y],
            size: [w, h],
            uv,
            color: tint,
            flag: 1.0,
            _pad: 0.0,
        });
    }

    /// MCD stone panel: 9-slice dark stone + gold trim + corner studs.
    /// `sel` lifts the trim to bright gold (focused panel).
    pub fn panel(&mut self, x: f32, y: f32, w: f32, h: f32, sel: bool) {
        let s = self.scale();
        let b = (9.0 * s).max(6.0); // border thickness in px
        // drop shadow
        self.rect(x + 3.0, y + 4.0, w, h, [0.0, 0.0, 0.0, 0.45]);
        self.slice9("ui_stone", x, y, w, h, b, [1.0, 1.0, 1.0, 1.0]);
        // gold trim lines just inside the stone border
        let g = if sel { GOLD_BRIGHT } else { GOLD };
        let br = BRONZE;
        let t = 2.0 * s;
        let o = b * 0.55;
        // top + left bright (light source), bottom + right bronze (shadow)
        self.rect(x + o, y + o, w - 2.0 * o, t, g);
        self.rect(x + o, y + o, t, h - 2.0 * o, g);
        self.rect(x + o, y + h - o - t, w - 2.0 * o, t, br);
        self.rect(x + w - o - t, y + o, t, h - 2.0 * o, br);
        // corner plates
        let c = 5.0 * s;
        for (cx, cy) in [(x + o - 1.0, y + o - 1.0), (x + w - o + 1.0 - c, y + o - 1.0), (x + o - 1.0, y + h - o + 1.0 - c), (x + w - o + 1.0 - c, y + h - o + 1.0 - c)] {
            self.rect(cx, cy, c, c, g);
            self.rect(cx + 1.0, cy + 1.0, c - 2.0, c - 2.0, GOLD);
        }
    }

    /// 9-slice of a 16x16 cell whose outer 4px (cell space) are the edge zone.
    fn slice9(&mut self, key: &str, x: f32, y: f32, w: f32, h: f32, b: f32, tint: [f32; 4]) {
        let q = 4.0 / 16.0; // edge zone in cell space
        let m = 1.0 - 2.0 * q; // middle zone
        let (bw, bh) = (w - 2.0 * b, h - 2.0 * b);
        // corners (square)
        self.tex(key, [0.0, 0.0, q, q], x, y, b, b, tint);
        self.tex(key, [1.0 - q, 0.0, q, q], x + w - b, y, b, b, tint);
        self.tex(key, [0.0, 1.0 - q, q, q], x, y + h - b, b, b, tint);
        self.tex(key, [1.0 - q, 1.0 - q, q, q], x + w - b, y + h - b, b, b, tint);
        // edges (stretch along one axis)
        self.tex(key, [q, 0.0, m, q], x + b, y, bw, b, tint);
        self.tex(key, [q, 1.0 - q, m, q], x + b, y + h - b, bw, b, tint);
        self.tex(key, [0.0, q, q, m], x, y + b, b, bh, tint);
        self.tex(key, [1.0 - q, q, q, m], x + w - b, y + b, b, bh, tint);
        // center — tiled patches so the grain stays crisp on big panels
        let patch = 84.0;
        let nx = ((bw / patch).ceil() as usize).max(1);
        let ny = ((bh / patch).ceil() as usize).max(1);
        let pw = bw / nx as f32;
        let ph = bh / ny as f32;
        for iy in 0..ny {
            for ix in 0..nx {
                self.tex(key, [q, q, m, m], x + b + ix as f32 * pw, y + b + iy as f32 * ph, pw, ph, tint);
            }
        }
    }

    pub fn icon(&mut self, key: &str, x: f32, y: f32, size: f32, tint: [f32; 4]) {
        let cell = self.assets.cell(key);
        let n = crate::gfx::rect_norm(cell, crate::assets::ATLAS_DIM);
        self.quads.push(QuadInstance {
            pos: [x, y],
            size: [size, size],
            uv: n,
            color: tint,
            flag: 1.0,
            _pad: 0.0,
        });
    }

    pub fn measure(&self, s: &str, size: usize) -> f32 {
        self.glyphs.measure(s, size)
    }

    pub fn text(&mut self, s: &str, x: f32, y: f32, size: usize, c: [f32; 4]) -> f32 {
        // shadow + glyphs
        self.text_inner(s, x + 1.0, y + 1.0, size, [0.0, 0.0, 0.0, 0.7]);
        self.text_inner(s, x, y, size, c)
    }

    fn text_inner(&mut self, s: &str, x: f32, y: f32, size: usize, c: [f32; 4]) -> f32 {
        let mut pen = x;
        for ch in s.chars() {
            let g = self.glyphs.get(ch, size);
            if g.size[0] > 0.0 {
                self.text.push(QuadInstance {
                    pos: [pen + g.bearing[0], y + g.bearing[1]],
                    size: g.size,
                    uv: g.uv,
                    color: c,
                    flag: 1.0,
                    _pad: 0.0,
                });
            }
            pen += g.advance;
        }
        pen - x
    }

    pub fn text_centered(&mut self, s: &str, cx: f32, y: f32, size: usize, c: [f32; 4]) {
        let w = self.measure(s, size);
        self.text(s, cx - w / 2.0, y, size, c);
    }

    pub fn bar(&mut self, x: f32, y: f32, w: f32, h: f32, frac: f32, fg: [f32; 4], bg: [f32; 4]) {
        self.rect(x, y, w, h, bg);
        self.rect(x, y, w * frac.clamp(0.0, 1.0), h, fg);
        self.rect(x, y, w, 1.0, [0.0, 0.0, 0.0, 0.5]);
    }

    /// MCD stone button: beveled slab, gold frame, glow when selected.
    pub fn button(&mut self, r: Rect, label: &str, selected: bool, enabled: bool) {
        let (x, y, w, h) = (r[0], r[1], r[2], r[3]);
        let s = self.scale();
        let b = (7.0 * s).max(5.0);
        if enabled {
            if selected {
                // soft outer glow
                self.rect(x - 3.0, y - 3.0, w + 6.0, h + 6.0, [1.0, 0.84, 0.4, 0.16]);
                self.slice9("ui_stone_light", x, y, w, h, b, [1.22, 1.16, 1.05, 1.0]);
            } else {
                self.slice9("ui_stone_light", x, y, w, h, b, [0.88, 0.86, 0.84, 1.0]);
            }
        } else {
            self.slice9("ui_stone_light", x, y, w, h, b, [0.45, 0.45, 0.48, 1.0]);
        }
        // gold frame (bright when selected)
        let frame_c = if !enabled {
            [0.3, 0.28, 0.24, 1.0]
        } else if selected {
            GOLD_BRIGHT
        } else {
            GOLD
        };
        let t = 2.0 * s;
        self.rect(x, y, w, t, frame_c);
        self.rect(x, y + h - t, w, t, frame_c);
        self.rect(x, y, t, h, frame_c);
        self.rect(x + w - t, y, t, h, frame_c);
        self.text_centered(
            label,
            x + w / 2.0,
            y + h / 2.0 - self.glyphs.px(SIZE_SMALL) * 0.6,
            SIZE_SMALL,
            if !enabled {
                DIM
            } else if selected {
                GOLD_BRIGHT
            } else {
                WHITE
            },
        );
    }

    pub fn dim_overlay(&mut self, a: f32) {
        self.rect(0.0, 0.0, self.w, self.h, [0.0, 0.0, 0.0, a]);
    }

    /// Rect aux coins adoucis (3 rects — suffisant à petit rayon).
    pub fn rounded(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, c: [f32; 4]) {
        self.rect(x + r, y, w - 2.0 * r, h, c);
        self.rect(x, y + r, r, h - 2.0 * r, c);
        self.rect(x + w - r, y + r, r, h - 2.0 * r, c);
    }

    /// Rect arrondi avec bordure (rounded extérieur + intérieur décalé).
    pub fn rounded_frame(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, border: [f32; 4], inner: [f32; 4], inset: f32) {
        self.rounded(x, y, w, h, r, border);
        self.rounded(x + inset, y + inset, w - 2.0 * inset, h - 2.0 * inset, (r - inset).max(1.0), inner);
    }

    /// Texte en "gras" : double frappe légèrement décalée (pas de graisse dans l'atlas).
    pub fn text_bold(&mut self, s: &str, x: f32, y: f32, size: usize, c: [f32; 4]) -> f32 {
        self.text_inner(s, x + 1.0, y + 1.0, size, [0.0, 0.0, 0.0, 0.8]);
        self.text_inner(s, x, y, size, c);
        self.text_inner(s, x + 0.8, y, size, c);
        self.measure(s, size)
    }

    /// Largeur d'une pastille de touche.
    pub fn chip_w(&self, label: &str) -> f32 {
        self.measure(label, SIZE_SMALL) + 14.0 * self.scale()
    }

    /// Pastille de touche façon MCD (fond arrondi + label blanc centré).
    pub fn key_chip(&mut self, x: f32, cy: f32, label: &str, dark: bool) -> f32 {
        let s = self.scale();
        let w = self.measure(label, SIZE_SMALL) + 14.0 * s;
        let h = 24.0 * s;
        let y = cy - h / 2.0;
        let bg = if dark { [0.0, 0.0, 0.0, 0.55] } else { [1.0, 1.0, 1.0, 0.16] };
        self.rounded(x, y, w, h, 5.0 * s, bg);
        self.text(label, x + 7.0 * s, y + h / 2.0 - self.glyphs.px(SIZE_SMALL) * 0.55, SIZE_SMALL, WHITE);
        w
    }

    /// Bouton sombre arrondi du vrai menu (Accessibilité / Options / Quitter /
    /// Réglages héros) : label à gauche, pastille de touche à droite.
    pub fn dark_button(&mut self, x: f32, y: f32, w: f32, h: f32, label: &str, key: &str, selected: bool) {
        let s = self.scale();
        let r = 8.0 * s;
        if selected {
            self.rounded(x - 3.0, y - 3.0, w + 6.0, h + 6.0, r, [1.0, 1.0, 1.0, 0.28]);
            self.rounded(x - 1.5, y - 1.5, w + 3.0, h + 3.0, r, [0.95, 0.95, 0.98, 0.6]);
        }
        let border = if selected { [0.95, 0.95, 0.97, 1.0] } else { [0.4, 0.4, 0.46, 1.0] };
        self.rounded_frame(x, y, w, h, r, border, [0.05, 0.05, 0.07, 0.93], 2.0);
        let ty = y + h / 2.0 - self.glyphs.px(SIZE_SMALL) * 0.55;
        self.text_bold(label, x + 14.0 * s, ty, SIZE_SMALL, WHITE);
        let cw = self.measure(key, SIZE_SMALL) + 14.0 * s;
        self.key_chip(x + w - cw - 10.0 * s, y + h / 2.0, key, true);
    }

    pub fn hotspot(&self, r: Rect) -> bool {
        let (mx, my) = self.mouse;
        mx >= r[0] && mx <= r[0] + r[2] && my >= r[1] && my <= r[1] + r[3]
    }

    pub fn item_name(&self, item: &Item) -> String {
        let base = Inventory::display_name(item);
        if item.rarity == Rarity::Unique {
            format!("✦ {}", base)
        } else {
            base
        }
    }
}

// ======================================================================
// Main menu
// ======================================================================

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

    // La scène 3D nocturne (héros + feu de camp + tente) est rendue derrière
    // (cf. app.rs) — voile bleuté de nuit profond, comme le vrai menu.
    ui.rect(0.0, 0.0, w, h, [0.008, 0.012, 0.04, 0.56]);
    ui.rect(0.0, h * 0.78, w, h * 0.22, [0.0, 0.0, 0.0, 0.30]);
    // poussière d'or dérivante (touche MCD)
    for i in 0..20 {
        let x = ((i as f32 * 137.5 + t * (6.0 + i as f32 * 0.5)) % (w + 40.0)) - 20.0;
        let y = h - ((i as f32 * 91.3 + t * (10.0 + i as f32 * 0.9)) % (h + 40.0)) + 20.0;
        let tw = 0.5 + 0.5 * (t * 2.0 + i as f32).sin();
        ui.rect(x, y, 3.0, 3.0, [1.0, 0.82, 0.4, 0.07 + 0.09 * tw]);
    }

    let mut rects = Vec::new();

    // ---- pilule titre en haut à gauche ----
    let tag = "MINECRAFT DUNGEONS — remake non officiel";
    let pill_w = ui.measure(tag, SIZE_SMALL) + 30.0 * s;
    ui.rounded_frame(
        16.0 * s,
        14.0 * s,
        pill_w,
        34.0 * s,
        17.0 * s,
        [0.0, 0.0, 0.0, 0.4],
        [0.07, 0.075, 0.1, 0.78],
        2.0,
    );
    ui.text(
        tag,
        31.0 * s,
        14.0 * s + 17.0 * s - pxs * 0.55,
        SIZE_SMALL,
        [0.88, 0.88, 0.92, 1.0],
    );

    // ---- bannière de saison en haut à droite (animée) ----
    season_banner(ui, w - 18.0 * s - 314.0 * s, 14.0 * s, 314.0 * s, 82.0 * s, t);

    // ---- stats du héros, à droite du personnage (comme le vrai menu) ----
    hero_stats(ui, cx + 115.0 * s, h * 0.385, level, power, emeralds);

    // quand l'éditeur de héros est ouvert, le bas du menu s'efface
    // (comme le vrai menu : l'éditeur prend toute la place)
    if hero_open {
        return rects;
    }

    // ---- gros bouton vert : COMMENCER LA PARTIE (bas gauche) ----
    {
        let pw = 316.0 * s;
        let ph_h = 40.0 * s;
        let gap = 6.0 * s;
        let pb_h = 74.0 * s;
        let px = 18.0 * s;
        let py = h - 18.0 * s - ph_h - gap - pb_h;
        // entête sombre : PARTIE HORS-LIGNE | CHANGER [A]
        ui.rounded_frame(px, py, pw, ph_h, 6.0 * s, [0.04, 0.16, 0.06, 1.0], [0.07, 0.25, 0.09, 1.0], 2.0);
        let hy = py + ph_h / 2.0 - pxs * 0.55;
        ui.text_bold("PARTIE HORS-LIGNE", px + 14.0 * s, hy, SIZE_SMALL, WHITE);
        let l1 = ui.measure("PARTIE HORS-LIGNE", SIZE_SMALL);
        ui.rect(px + 14.0 * s + l1 + 12.0 * s, py + 8.0 * s, 2.0, ph_h - 16.0 * s, [1.0, 1.0, 1.0, 0.25]);
        ui.text("CHANGER", px + 14.0 * s + l1 + 24.0 * s, hy, SIZE_SMALL, [0.82, 0.93, 0.82, 1.0]);
        let cw = ui.chip_w("A");
        ui.key_chip(px + pw - cw - 10.0 * s, py + ph_h / 2.0, "A", true);
        // bouton vert lumineux (pulse quand sélectionné)
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
        let cw = ui.chip_w("Entrée");
        ui.key_chip(px + pw - cw - 12.0 * s, by + pb_h / 2.0, "Entrée", true);
        rects.push([px, by, pw, pb_h]);
    }

    // ---- RÉGLAGES HÉROS sous le personnage ----
    {
        let hw = 252.0 * s;
        let hh = 46.0 * s;
        let hr = [cx - hw / 2.0, h * 0.575, hw, hh];
        ui.dark_button(hr[0], hr[1], hw, hh, "RÉGLAGES HÉROS", "H", sel == 1);
        rects.push(hr);
    }

    // ---- rangée sombre bas droite : Accessibilité / Options / Quitter ----
    {
        let items = [("Accessibilité", "F1", 2usize), ("Options", "F2", 3), ("Quitter", "Échap", 4)];
        let bh = 42.0 * s;
        let gap = 10.0 * s;
        let widths: Vec<f32> = items
            .iter()
            .map(|(l, k, _)| 14.0 * s + ui.measure(l, SIZE_SMALL) + 12.0 * s + ui.chip_w(k) + 12.0 * s)
            .collect();
        let total: f32 = widths.iter().sum::<f32>() + gap * (items.len() - 1) as f32;
        let mut bx = w - 18.0 * s - total;
        let by = h - 18.0 * s - bh;
        for ((l, k, idx), wd) in items.iter().zip(widths.iter()) {
            ui.dark_button(bx, by, *wd, bh, l, k, sel == *idx);
            rects.push([bx, by, *wd, bh]);
            bx += wd + gap;
        }
        ui.text_centered(
            "Co-op locale : une manette connectée active le Joueur 2   —   F3 : mode debug",
            cx,
            by - 26.0 * s,
            SIZE_SMALL,
            DIM,
        );
    }

    rects
}

fn lerp4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

/// Bannière de saison en haut à droite (comme la capture : carré bleu "50",
/// nom de saison en cyan, liseré orange, barre AP) — entièrement animée :
/// liseré qui pulse, balayage lumineux, particules qui montent, barre AP vivante.
fn season_banner(ui: &mut Ui, x: f32, y: f32, bw: f32, bh: f32, t: f32) {
    let s = ui.scale();
    // liseré orange qui pulse doucement (braise)
    let glow = 0.5 + 0.5 * (t * 2.1).sin();
    let border = lerp4([0.85, 0.45, 0.12, 1.0], [1.0, 0.68, 0.24, 1.0], glow);
    ui.rounded_frame(x, y, bw, bh, 10.0 * s, border, [0.045, 0.05, 0.075, 0.94], 2.5);

    // particules dorées qui montent à l'intérieur de la bannière
    let ih = bh - 20.0 * s;
    for i in 0..4 {
        let px = x + 24.0 * s + ((i as f32 * 67.0) % (bw - 48.0 * s));
        let cyc = (t * (9.0 + i as f32 * 2.7) + i as f32 * 31.0) % (ih - 8.0);
        let py = y + bh - 10.0 * s - cyc;
        let fade = 1.0 - cyc / (ih - 8.0);
        ui.rect(px, py, 2.0, 2.0, [1.0, 0.84, 0.45, 0.30 * fade + 0.10]);
    }

    // carré bleu "50" : halo pulsant + léger flottement
    let bs = bh - 20.0 * s;
    let bob = 1.2 * (t * 2.4).sin();
    let sqy = y + 10.0 * s + bob;
    ui.rounded(
        x + 10.0 * s - 3.0,
        sqy - 3.0,
        bs + 6.0,
        bs + 6.0,
        7.0 * s,
        [0.45, 0.7, 1.0, 0.14 + 0.14 * glow],
    );
    ui.rounded(x + 10.0 * s, sqy, bs, bs, 6.0 * s, [0.16, 0.4, 0.85, 1.0]);
    let pxs = ui.glyphs.px(SIZE_SMALL);
    ui.text_centered("50", x + 10.0 * s + bs / 2.0, sqy + bs / 2.0 - pxs * 0.55, SIZE_SMALL, WHITE);
    let tx = x + 10.0 * s + bs + 12.0 * s;
    ui.text_bold("SAISON 1", tx, y + 13.0 * s + bob * 0.4, SIZE_SMALL, WHITE);
    // sous-titre cyan qui scintille
    let cy = 0.55 + 0.35 * (t * 1.7).sin();
    ui.text_bold(
        "L'ASCENSION NUAGEUSE",
        tx,
        y + 35.0 * s + bob * 0.4,
        SIZE_SMALL,
        [0.38 + 0.25 * cy, 0.65 + 0.2 * cy, 1.0, 1.0],
    );
    // barre AP vivante : remplissage qui respire + reflet qui glisse
    let by = y + bh - 18.0 * s;
    ui.text("AP", tx, by - 2.0 * s, SIZE_SMALL, [1.0, 0.55, 0.15, 1.0]);
    let barx = tx + 30.0 * s;
    let barw = x + bw - barx - 12.0 * s;
    let frac = 0.35 + 0.05 * (t * 0.9).sin();
    ui.bar(barx, by + 3.0 * s, barw, 6.0 * s, frac, [0.95, 0.5, 0.1, 1.0], [0.16, 0.11, 0.06, 1.0]);
    // reflet clair qui parcourt la partie remplie
    let filled = barw * frac;
    if filled > 14.0 {
        let gx = barx + ((t * 26.0) % (filled - 10.0));
        let shine = 0.5 + 0.5 * (t * 6.0).sin();
        ui.rect(gx, by + 3.0 * s + 1.0, 6.0, 4.0, [1.0, 0.9, 0.6, 0.35 + 0.3 * shine]);
    }

    // balayage lumineux diagonal qui traverse la bannière toutes les ~4.5 s
    let period = 4.5;
    let sw = ((t % period) / period) * (bw + 180.0) - 90.0;
    let edge_fade = ((sw / 70.0).min(1.0)).max(0.0) * (((bw - sw) / 70.0).min(1.0)).max(0.0);
    if edge_fade > 0.0 {
        for k in -2i32..=2 {
            let sx = x + sw + k as f32 * 12.0 * s;
            let a = 0.10 * edge_fade * (1.0 - (k.abs() as f32) / 3.0);
            ui.rect(sx, y + 4.0 * s, 5.0 * s, bh - 8.0 * s, [1.0, 0.92, 0.7, a]);
        }
    }
}

// ======================================================================
// Hero editor (skin selection) — derrière "RÉGLAGES HÉROS"
// ======================================================================

/// Panneau pierre dorée façon MCD : carrousel de tenues (portraits), nom de la
/// tenue, boutons APPLIQUER / RETOUR. Le héros 3D du menu porte la tenue
/// prévisualisée en direct (cf. app.rs preview_skin).
/// Rects retournés : [0]=flèche gauche [1]=flèche droite [2]=APPLIQUER [3]=RETOUR.
pub fn draw_hero_editor(
    ui: &mut Ui,
    names: &[&str],
    sel: usize,
    cursor: usize,
    t: f32,
) -> Vec<Rect> {
    let s = ui.scale();
    let w = ui.w;
    let h = ui.h;
    let mut rects = Vec::new();

    ui.dim_overlay(0.42);

    // ---- panneau principal (pierre dorée, focus) ----
    let pw = (660.0 * s).min(w - 32.0 * s);
    let ph = 208.0 * s;
    let px = w / 2.0 - pw / 2.0;
    let py = h - ph - 20.0 * s;
    ui.panel(px, py, pw, ph, true);

    // ---- titre + filets décoratifs ----
    let title = "ÉDITEUR DE HÉROS";
    let tw = ui.measure(title, SIZE_SMALL);
    ui.text_bold(title, w / 2.0 - tw / 2.0, py + 16.0 * s, SIZE_SMALL, GOLD_BRIGHT);
    let ly = py + 16.0 * s + ui.glyphs.px(SIZE_SMALL) * 0.5;
    let lr = (pw - tw) / 2.0 - 24.0 * s;
    for (lx, lw2) in [(px + 24.0 * s, lr), (w / 2.0 + tw / 2.0 + 24.0 * s, lr)] {
        ui.rect(lx, ly - 1.0, lw2.max(0.0), 2.0, GOLD);
        ui.rect(lx + lw2.max(0.0) - 4.0 * s, ly - 3.0, 4.0 * s, 6.0 * s, GOLD_BRIGHT);
    }

    // ---- carrousel : flèche gauche / portrait / flèche droite ----
    let ps = 76.0 * s; // portrait size
    let cy = py + 50.0 * s + ps / 2.0; // centre vertical du carrousel
    let pulse = 0.5 + 0.5 * (t * 3.0).sin();

    // flèche gauche
    let aw = 56.0 * s;
    let ah = 64.0 * s;
    let ax = px + 40.0 * s;
    let ay = cy - ah / 2.0;
    draw_arrow_button(ui, ax, ay, aw, ah, true, cursor == 0, pulse);
    rects.push([ax, ay, aw, ah]);

    // flèche droite
    let bx = px + pw - 40.0 * s - aw;
    draw_arrow_button(ui, bx, ay, aw, ah, false, cursor == 1, pulse);
    rects.push([bx, ay, aw, ah]);

    // portrait encadré d'or (glow quand survol/sélection)
    let gx = w / 2.0 - ps / 2.0;
    let gy = cy - ps / 2.0;
    ui.rounded(gx - 4.0, gy - 4.0, ps + 8.0, ps + 8.0, 10.0 * s, [1.0, 0.86, 0.45, 0.10 + 0.12 * pulse]);
    ui.rounded_frame(gx, gy, ps, ps, 8.0 * s, GOLD_BRIGHT, [0.06, 0.06, 0.09, 0.96], 2.5);
    ui.icon(&format!("skinp_{}", sel), gx + 6.0 * s, gy + 6.0 * s, ps - 12.0 * s, WHITE);

    // nom de la tenue + compteur sous le portrait
    let name = names.get(sel).copied().unwrap_or("?");
    let label = format!("{}   —   {}/{}", name, sel + 1, names.len());
    let lw = ui.measure(&label, SIZE_SMALL);
    ui.text_bold(&label, w / 2.0 - lw / 2.0, gy + ps + 10.0 * s, SIZE_SMALL, WHITE);

    // ---- boutons APPLIQUER / RETOUR ----
    let bw2 = 216.0 * s;
    let bh2 = 44.0 * s;
    let gap = 16.0 * s;
    let total = bw2 * 2.0 + gap;
    let byy = py + ph - bh2 - 16.0 * s;
    // APPLIQUER (vert MCD)
    let apx = w / 2.0 - total / 2.0;
    {
        let selg = cursor == 2;
        let pulse2 = 0.5 + 0.5 * (t * 3.2).sin();
        if selg {
            ui.rounded(apx - 3.0, byy - 3.0, bw2 + 6.0, bh2 + 6.0, 9.0 * s, [1.0, 1.0, 1.0, 0.16 + 0.10 * pulse2]);
        }
        let face = if selg {
            lerp4([0.30, 0.62, 0.25, 1.0], [0.44, 0.80, 0.35, 1.0], pulse2)
        } else {
            [0.26, 0.53, 0.21, 1.0]
        };
        let edge = if selg { [0.88, 1.0, 0.85, 1.0] } else { [0.13, 0.33, 0.10, 1.0] };
        ui.rounded(apx, byy, bw2, bh2, 7.0 * s, edge);
        ui.rounded(apx + 2.5, byy + 2.5, bw2 - 5.0, bh2 - 5.0, 5.5 * s, face);
        let lbl = "APPLIQUER";
        let lw2 = ui.measure(lbl, SIZE_SMALL);
        ui.text_bold(lbl, apx + bw2 / 2.0 - lw2 / 2.0, byy + bh2 / 2.0 - ui.glyphs.px(SIZE_SMALL) * 0.55, SIZE_SMALL, WHITE);
        let cw = ui.chip_w("Entrée");
        ui.key_chip(apx + bw2 - cw - 10.0 * s, byy + bh2 / 2.0, "Entrée", true);
        rects.push([apx, byy, bw2, bh2]);
    }
    // RETOUR
    {
        let rtx = apx + bw2 + gap;
        ui.dark_button(rtx, byy, bw2, bh2, "RETOUR", "Échap", cursor == 3);
        rects.push([rtx, byy, bw2, bh2]);
    }

    // ---- aide discrète ----
    ui.text_centered(
        "< >  changer de tenue   —   le héros l'essaie en direct",
        w / 2.0,
        py - 22.0 * s,
        SIZE_SMALL,
        [0.85, 0.85, 0.9, 0.75],
    );

    rects
}

/// Bouton flèche (carrousel) : plaqué sombre + chevron doré, glow si sélectionné.
fn draw_arrow_button(ui: &mut Ui, x: f32, y: f32, w: f32, h: f32, left: bool, selected: bool, pulse: f32) {
    let s = ui.scale();
    if selected {
        ui.rounded(x - 3.0, y - 3.0, w + 6.0, h + 6.0, 9.0 * s, [1.0, 0.86, 0.45, 0.14 + 0.12 * pulse]);
    }
    let border = if selected { GOLD_BRIGHT } else { GOLD };
    ui.rounded_frame(x, y, w, h, 8.0 * s, border, [0.05, 0.05, 0.075, 0.94], 2.0);
    // chevron en escalier de petits carrés (net, style pixel)
    let u = 5.0 * s; // unité
    let dir: f32 = if left { -1.0 } else { 1.0 };
    let col = if selected { GOLD_BRIGHT } else { GOLD };
    // pointe du chevron côté direction, bras qui s'écartent vers l'autre bord
    let apex_x = x + w / 2.0 + dir * u * 1.5;
    let cy = y + h / 2.0 - u * 0.5;
    for i in 0..4 {
        ui.rect(apex_x - dir * i as f32 * u, cy - i as f32 * u, u, u, col);
        ui.rect(apex_x - dir * i as f32 * u, cy + i as f32 * u, u, u, col);
    }
}

/// Bloc stats à droite du héros : NV (pastille violette), puissance (épée),
/// émeraudes — exactement la disposition du vrai menu.
fn hero_stats(ui: &mut Ui, x: f32, y: f32, level: u32, power: u32, emeralds: u32) {
    let s = ui.scale();
    let ch = 34.0 * s;
    let row = 46.0 * s;
    let pxs = ui.glyphs.px(SIZE_SMALL);
    // NV
    ui.rounded(x, y, ch, ch, 9.0 * s, [0.36, 0.22, 0.75, 1.0]);
    ui.rounded(x + 2.0, y + 2.0, ch - 4.0, ch - 4.0, 7.5 * s, [0.5, 0.33, 0.92, 1.0]);
    ui.text_centered("NV", x + ch / 2.0, y + ch / 2.0 - pxs * 0.55, SIZE_SMALL, WHITE);
    ui.text_bold(&level.to_string(), x + ch + 10.0 * s, y + ch / 2.0 - pxs * 0.55, SIZE_SMALL, WHITE);
    // puissance
    let y2 = y + row;
    ui.rounded(x, y2, ch, ch, 9.0 * s, [0.0, 0.0, 0.0, 0.45]);
    ui.rounded(x + 2.0, y2 + 2.0, ch - 4.0, ch - 4.0, 7.5 * s, [0.14, 0.15, 0.18, 0.9]);
    ui.icon("icon_sword", x + 6.0 * s, y2 + 6.0 * s, ch - 12.0 * s, WHITE);
    ui.text_bold(&power.to_string(), x + ch + 10.0 * s, y2 + ch / 2.0 - pxs * 0.55, SIZE_SMALL, WHITE);
    // émeraudes
    let y3 = y + 2.0 * row;
    ui.icon("icon_emerald", x + 3.0 * s, y3 + 3.0 * s, ch - 6.0 * s, WHITE);
    ui.text_bold(&emeralds.to_string(), x + ch + 10.0 * s, y3 + ch / 2.0 - pxs * 0.55, SIZE_SMALL, WHITE);
}

// ======================================================================
// Camp / mission map
// ======================================================================

pub fn draw_camp(
    ui: &mut Ui,
    save: &crate::game::progress::Save,
    inv: &Inventory,
    camp: &crate::game::shop::Camp,
    tab: usize,
    sel: usize,
) -> Vec<Rect> {
    let s = ui.scale();
    // 3D backdrop visible through a light dim (menu scene lives behind)
    ui.dim_overlay(0.62);
    let mut rects = Vec::new();

    // top tabs
    let tabs = ["Carte", "Marchand", "Forgeron", "Inventaire"];
    for (i, tname) in tabs.iter().enumerate() {
        let r = [40.0 * s + i as f32 * 190.0 * s, 18.0 * s, 180.0 * s, 46.0 * s];
        ui.button(r, tname, tab == i, true);
        rects.push(r);
    }
    // emeralds + hero
    let pw = inv.power();
    ui.icon("icon_emerald", ui.w - 250.0 * s, 24.0 * s, 26.0 * s, WHITE);
    ui.text(&format!("{}", save.emeralds), ui.w - 214.0 * s, 28.0 * s, SIZE_SMALL, ACCENT);
    ui.text(
        &format!("Héros niv. {}  Puissance {}", save.players.first().map(|p| p.level).unwrap_or(1), pw),
        ui.w - 214.0 * s,
        56.0 * s,
        SIZE_SMALL,
        WHITE,
    );

    let y0 = 84.0 * s;
    match tab {
        0 => {
            // mission map
            let mx = 40.0 * s;
            let mw = 560.0 * s;
            ui.panel(mx, y0, mw, ui.h - y0 - 20.0 * s, sel >= 4 && sel < 4 + crate::world::missions::MISSIONS.len());
            ui.text("CAMPAGNE", mx + 16.0 * s, y0 + 12.0 * s, SIZE_SMALL, ACCENT);
            for (i, m) in crate::world::missions::MISSIONS.iter().enumerate() {
                let r = [
                    mx + 16.0 * s,
                    y0 + 40.0 * s + i as f32 * 33.0 * s,
                    mw - 32.0 * s,
                    28.0 * s,
                ];
                let unlocked = save.is_unlocked(i);
                let completed = save.is_completed(i);
                let label = if unlocked || completed {
                    format!("{}  —  menace {}", m.name, m.threat)
                } else if m.is_secret {
                    "? ? ?  (mission secrète)".to_string()
                } else {
                    format!("{}  (verrouillé)", m.name)
                };
                ui.button(r, &label, sel == 4 + i, unlocked);
                if completed {
                    ui.icon("icon_key", r[0] + r[2] - 26.0 * s, r[1] + 5.0 * s, 20.0 * s, ACCENT);
                }
                if unlocked && !completed && m.is_secret {
                    ui.icon("icon_rune", r[0] + r[2] - 26.0 * s, r[1] + 5.0 * s, 20.0 * s, GREEN);
                }
                rects.push(r);
            }
            // right panel: threat selection
            let rx = mx + mw + 20.0 * s;
            let rw = ui.w - rx - 40.0 * s;
            ui.panel(rx, y0, rw, 260.0 * s, save.tier <= save.tier_unlocked);
            ui.text("NIVEAU DE MENACE", rx + 16.0 * s, y0 + 12.0 * s, SIZE_SMALL, ACCENT);
            let tiers = ["Défaut", "Aventure", "Apocalypse"];
            for (i, t) in tiers.iter().enumerate() {
                let r = [rx + 16.0 * s, y0 + 44.0 * s + i as f32 * 48.0 * s, rw - 32.0 * s, 40.0 * s];
                let unlocked = save.tier_unlocked >= i;
                ui.button(r, t, save.tier == i, unlocked);
                rects.push(r);
            }
            ui.text(
                "Aventure / Apocalypse se débloquent en terminant la campagne.",
                rx + 16.0 * s,
                y0 + 200.0 * s,
                SIZE_SMALL,
                DIM,
            );
            // launch
            let lr = [rx, y0 + 280.0 * s, rw, 64.0 * s];
            ui.button(lr, "LANCER L'EXPÉDITION", sel == 4 + crate::world::missions::MISSIONS.len() + 3, true);
            rects.push(lr);
            // camp tip
            ui.panel(rx, y0 + 360.0 * s, rw, 150.0 * s, false);
            ui.text("Le camp :", rx + 16.0 * s, y0 + 372.0 * s, SIZE_SMALL, ACCENT);
            ui.text("— Marchand : équipement contre émeraudes", rx + 16.0 * s, y0 + 396.0 * s, SIZE_SMALL, WHITE);
            ui.text("— Forgeron : améliore la puissance d'un objet", rx + 16.0 * s, y0 + 418.0 * s, SIZE_SMALL, WHITE);
            ui.text("— Trouve les runes pour les missions secrètes !", rx + 16.0 * s, y0 + 440.0 * s, SIZE_SMALL, WHITE);
            ui.text("Échap : retour au menu principal", rx + 16.0 * s, y0 + 464.0 * s, SIZE_SMALL, DIM);
        }
        1 => {
            // merchant
            let mx = 40.0 * s;
            let mw = ui.w - 80.0 * s;
            ui.panel(mx, y0, mw, ui.h - y0 - 20.0 * s, false);
            ui.text("MARCHAND DU CAMP", mx + 16.0 * s, y0 + 12.0 * s, SIZE_SMALL, ACCENT);
            ui.text("Équipement renouvelé après chaque expédition.", mx + 16.0 * s, y0 + 34.0 * s, SIZE_SMALL, DIM);
            for (i, (item, price)) in camp.stock.iter().enumerate() {
                let r = [mx + 16.0 * s, y0 + 60.0 * s + i as f32 * 64.0 * s, mw - 32.0 * s, 56.0 * s];
                draw_item_row(ui, r, item, sel == 4 + i, save.emeralds >= *price);
                ui.icon("icon_emerald", r[0] + r[2] - 150.0 * s, r[1] + 16.0 * s, 22.0 * s, WHITE);
                ui.text(&format!("{}", price), r[0] + r[2] - 120.0 * s, r[1] + 18.0 * s, SIZE_SMALL, ACCENT);
                rects.push(r);
            }
        }
        2 => {
            // forge
            let mx = 40.0 * s;
            let mw = ui.w - 80.0 * s;
            ui.panel(mx, y0, mw, ui.h - y0 - 20.0 * s, false);
            ui.text("FORGERON", mx + 16.0 * s, y0 + 12.0 * s, SIZE_SMALL, ACCENT);
            ui.text("Améliore la puissance d'un objet de +1 (coût en émeraudes).", mx + 16.0 * s, y0 + 34.0 * s, SIZE_SMALL, DIM);
            for (i, item) in inv.stash.iter().enumerate() {
                let r = [mx + 16.0 * s, y0 + 60.0 * s + i as f32 * 64.0 * s, mw - 32.0 * s, 56.0 * s];
                let price = crate::game::shop::forge_price(item);
                draw_item_row(ui, r, item, sel == 4 + i, save.emeralds >= price);
                ui.icon("icon_emerald", r[0] + r[2] - 150.0 * s, r[1] + 16.0 * s, 22.0 * s, WHITE);
                ui.text(&format!("{}", price), r[0] + r[2] - 120.0 * s, r[1] + 18.0 * s, SIZE_SMALL, ACCENT);
                rects.push(r);
            }
        }
        _ => {}
    }
    rects
}

pub fn draw_item_row(ui: &mut Ui, r: Rect, item: &Item, selected: bool, enabled: bool) {
    let bg = if !enabled {
        [0.12, 0.12, 0.14, 0.9]
    } else if selected {
        [0.3, 0.26, 0.14, 0.95]
    } else {
        PANEL_DARK
    };
    ui.frame(r[0], r[1], r[2], r[3], bg, if selected { ACCENT } else { item.rarity.color() });
    ui.icon(item.icon(), r[0] + 10.0, r[1] + 8.0, 40.0, WHITE);
    let name = ui_item_name(ui, item);
    ui.text(&name, r[0] + 60.0, r[1] + 8.0, SIZE_SMALL, item.rarity.color());
    ui.text(
        &format!("Puissance {}", item.power),
        r[0] + 60.0,
        r[1] + 30.0,
        SIZE_SMALL,
        DIM,
    );
}

fn ui_item_name(_ui: &Ui, item: &Item) -> String {
    if item.name.is_empty() {
        item.base_name().to_string()
    } else {
        item.name.clone()
    }
}

// ======================================================================
// HUD (in-game)
// ======================================================================

pub fn draw_hud(ui: &mut Ui, game: &Game, cam: &Camera) -> Vec<Rect> {
    let s = ui.scale();
    let mut rects = Vec::new();
    let _ = &mut rects;

    // --- player panels (bottom-left) ---
    for (i, p) in game.players.iter().enumerate() {
        let y = ui.h - (86.0 + i as f32 * 96.0) * s;
        let x = 18.0 * s;
        let w = 320.0 * s;
        let max = p.max_hp();
        let frac = (p.hp / max).clamp(0.0, 1.0);
        let col = if i == 0 { GREEN } else { [0.35, 0.6, 1.0, 1.0] };
        // hero level badge — big red heart like the MCD HUD center piece
        ui.frame(x, y, w, 76.0 * s, PANEL, BORDER);
        ui.icon("icon_heart", x + 8.0 * s, y + 8.0 * s, 30.0 * s, RED);
        ui.bar(x + 44.0 * s, y + 10.0 * s, w - 60.0 * s, 16.0 * s, frac, col, [0.15, 0.15, 0.15, 1.0]);
        ui.text(
            &format!("{}/{}", p.hp as i32, max as i32),
            x + 50.0 * s,
            y + 12.0 * s,
            SIZE_SMALL,
            WHITE,
        );
        // xp
        ui.bar(
            x + 44.0 * s,
            y + 30.0 * s,
            w - 60.0 * s,
            6.0 * s,
            p.xp as f32 / crate::consts::xp_for_level(p.level) as f32,
            PURPLE,
            [0.15, 0.15, 0.15, 1.0],
        );
        ui.text(&format!("J{}  Niv. {}", i + 1, p.level), x + 8.0 * s, y + 30.0 * s, SIZE_SMALL, WHITE);
        // potion
        let pot_r = [x + w + 8.0 * s, y + 8.0 * s, 40.0 * s, 40.0 * s];
        ui.frame(pot_r[0], pot_r[1], pot_r[2], pot_r[3], PANEL_DARK, BORDER);
        ui.icon("icon_potion", pot_r[0] + 6.0 * s, pot_r[1] + 6.0 * s, 28.0 * s, WHITE);
        if p.potion_cd > 0.0 {
            let cd = p.potion_cd / crate::consts::POTION_COOLDOWN;
            ui.rect(pot_r[0], pot_r[1], pot_r[2], pot_r[3] * cd, [0.0, 0.0, 0.0, 0.6]);
        }
        ui.text(&format!("x{} (F)", p.potions), pot_r[0], pot_r[1] + 44.0 * s, SIZE_SMALL, DIM);
        // downed overlay
        if p.downed_t.is_some() {
            ui.text("À TERRE !", x + 120.0 * s, y + 40.0 * s, SIZE_SMALL, RED);
        }
    }

    // --- artifacts + arrows (bottom-right, shifted left so emeralds sit in the corner) ---
    if let Some(p) = game.players.first() {
        let base_x = ui.w - (3.0 * 62.0 + 24.0 + 70.0) * s;
        let y = ui.h - 86.0 * s;
        for (i, slot) in p.inventory.artifacts.iter().enumerate() {
            let r = [base_x + i as f32 * 62.0 * s, y, 54.0 * s, 54.0 * s];
            ui.frame(r[0], r[1], r[2], r[3], PANEL_DARK, BORDER);
            if let Some(item) = slot {
                ui.icon(item.icon(), r[0] + 7.0 * s, r[1] + 7.0 * s, 40.0 * s, WHITE);
                let cd = p.artifact_cd[i];
                if cd > 0.0 {
                    let maxcd = crate::game::items::artifact_def(match item.class {
                        ItemClass::Art(k) => k,
                        _ => crate::game::items::ArtifactKind::Totem,
                    }).cd;
                    ui.rect(r[0], r[1], r[2], r[3] * (cd / maxcd).clamp(0.0, 1.0), [0.0, 0.0, 0.0, 0.65]);
                }
            }
            let key = ["1", "2", "3"][i];
            ui.text(key, r[0] + 4.0 * s, r[1] + r[3] + 2.0 * s, SIZE_SMALL, DIM);
        }
        // arrows
        let ar = [ui.w - 154.0 * s, y - 40.0 * s, 70.0 * s, 32.0 * s];
        ui.icon("icon_arrow", ar[0], ar[1], 24.0 * s, WHITE);
        ui.text(&format!("{}", p.arrows), ar[0] + 30.0 * s, ar[1] + 4.0 * s, SIZE_SMALL, WHITE);
    }

    // --- emeralds (bottom-right corner, green — like MCD) ---
    ui.icon("icon_emerald", ui.w - 98.0 * s, ui.h - 64.0 * s, 30.0 * s, WHITE);
    ui.text(&format!("{}", game.emeralds), ui.w - 62.0 * s, ui.h - 58.0 * s, SIZE_SMALL, GREEN);

    // --- mission objective (top-right, MCD layout: white goal + yellow mission name) ---
    let m = &crate::world::missions::MISSIONS[game.mission_id];
    let total_captives = game.level.captives.len() as u32;
    let portal_active = game.level.portal.map(|(_, a)| a).unwrap_or(false);
    let objective = if total_captives > 0 && game.captives_rescued < total_captives {
        format!("LIBÉREZ LES VILLAGEOIS — {}/{}", game.captives_rescued, total_captives)
    } else if portal_active {
        "ENTREZ DANS LE PORTAIL".to_string()
    } else {
        "PROGRESSEZ DANS LA ZONE".to_string()
    };
    let objective = objective.to_uppercase();
    ui.text(&objective, ui.w - ui.measure(&objective, SIZE_SMALL) - 22.0 * s, 16.0 * s, SIZE_SMALL, WHITE);
    ui.text(
        m.name.to_uppercase().as_str(),
        ui.w - ui.measure(m.name.to_uppercase().as_str(), SIZE_SMALL) - 22.0 * s,
        38.0 * s,
        SIZE_SMALL,
        ACCENT,
    );

    // --- secondary info (top-left, dim) ---
    ui.text(&format!("Menace {}", m.threat), 18.0 * s, 14.0 * s, SIZE_SMALL, DIM);
    if total_captives > 0 {
        ui.text(
            &format!("Captifs libérés : {}/{}", game.captives_rescued, total_captives),
            18.0 * s,
            36.0 * s,
            SIZE_SMALL,
            DIM,
        );
    }
    if game.level.has_rune {
        let rune_txt = if game.rune_found { "Rune : trouvée !" } else { "Rune : à trouver (levier caché)" };
        ui.text(rune_txt, 18.0 * s, 56.0 * s, SIZE_SMALL, if game.rune_found { GREEN } else { DIM });
    }

    // --- boss banner (MCD style: dark band, white name, red bordered bar) ---
    if let Some(be) = game.boss_entity {
        if let Ok(h) = game.world.get::<&crate::game::Health>(be) {
            if h.hp > 0.0 {
                let bw = 560.0 * s;
                let bx = (ui.w - bw) / 2.0;
                let by = 14.0 * s;
                let name = boss_display_name(game);
                // translucent dark band behind (like the official boss banner)
                ui.rect(bx - 34.0 * s, by, bw + 68.0 * s, 48.0 * s, [0.02, 0.02, 0.03, 0.72]);
                ui.icon("icon_heart", bx - 26.0 * s, by + 6.0 * s, 24.0 * s, RED);
                ui.text_centered(name, ui.w / 2.0, by + 8.0 * s, SIZE_SMALL, WHITE);
                // bordered red bar
                ui.rect(bx - 2.0, by + 34.0 * s - 2.0, bw + 4.0, 16.0 * s + 4.0, [0.04, 0.03, 0.03, 1.0]);
                ui.bar(bx, by + 34.0 * s, bw, 16.0 * s, h.hp / h.max, RED, [0.28, 0.09, 0.09, 1.0]);
                // phase tick marks (white diamonds in the official HUD)
                for f in [0.33f32, 0.66] {
                    ui.rect(bx + bw * f - 2.0 * s, by + 37.0 * s, 4.0 * s, 10.0 * s, [1.0, 1.0, 1.0, 0.85]);
                }
            }
        }
    }

    // --- portal hint ---
    if let Some((ppos, active)) = game.level.portal {
        if active {
            if let Some((sx, sy)) = cam.world_to_screen(glam::Vec3::new(ppos.x, 3.4, ppos.y), ui.w, ui.h) {
                ui.text_centered("Portail ouvert — Entre pour terminer !", sx, sy, SIZE_SMALL, GREEN);
            }
        }
    }

    // --- controls reminder at mission start (fade out) — "comment on combat ?" ---
    if game.time < 14.0 {
        let a = ((14.0 - game.time) / 3.0).clamp(0.0, 1.0);
        let s = ui.scale();
        let lines = [
            "Clic gauche : attaque mêlée    Clic droit : tir (visée auto)",
            "Espace : roulade (esquive)    E : interagir    F : potion",
        ];
        let lw = lines.iter().map(|l| ui.measure(l, SIZE_SMALL)).fold(0.0f32, f32::max);
        let bw = lw + 36.0 * s;
        let bx = (ui.w - bw) / 2.0;
        let by = ui.h - 150.0 * s;
        let mut c = [0.03, 0.03, 0.05, 0.78 * a];
        let _ = &mut c;
        ui.frame(bx, by, bw, 52.0 * s, [0.03, 0.03, 0.05, 0.78 * a], [GOLD[0], GOLD[1], GOLD[2], 0.85 * a]);
        for (i, l) in lines.iter().enumerate() {
            let mut col = WHITE;
            col[3] = a;
            ui.text_centered(l, ui.w / 2.0, by + (8.0 + i as f32 * 20.0) * s, SIZE_SMALL, col);
        }
    }

    // --- interact hint (P1) ---
    if let Some(hint) = interact_hint(game, 0) {
        let w = ui.measure(&hint.0, SIZE_SMALL) + 30.0 * s;
        let x = (ui.w - w) / 2.0;
        let y = ui.h - 140.0 * s;
        ui.frame(x, y, w, 30.0 * s, PANEL, ACCENT);
        ui.text(&hint.0, x + 10.0 * s, y + 6.0 * s, SIZE_SMALL, WHITE);
        let _ = hint.1;
    }

    // --- floating damage numbers ---
    for f in &game.floaters {
        if let Some((sx, sy)) = cam.world_to_screen(f.pos, ui.w, ui.h) {
            let a = (f.life / crate::consts::DAMAGE_NUMBER_LIFE).clamp(0.0, 1.0);
            let mut c = f.color.to_array();
            c[3] = a;
            ui.text_centered(&f.text, sx, sy, SIZE_SMALL, c);
        }
    }

    rects
}

fn boss_display_name(game: &Game) -> &'static str {
    if game.boss_phase >= 2 {
        "Cœur d'Ender"
    } else {
        game.boss_name().unwrap_or("Boss")
    }
}

/// (text, world_pos) of the closest interactable to player 0.
fn interact_hint(game: &Game, pidx: usize) -> Option<(String, glam::Vec2)> {
    let p = game.players.get(pidx)?;
    if p.downed_t.is_some() {
        return None;
    }
    let pos = p.pos;
    let mut best: Option<(String, glam::Vec2, f32)> = None;
    let consider = |best: &mut Option<(String, glam::Vec2, f32)>, txt: String, wp: glam::Vec2, d: f32| {
        if d < crate::consts::INTERACT_RANGE && best.as_ref().map(|b| d < b.2).unwrap_or(true) {
            *best = Some((txt, wp, d));
        }
    };
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            let tx = pos.x.floor() as i32 + dx;
            let ty = pos.y.floor() as i32 + dy;
            let Some(i) = game.level.idx(tx, ty) else { continue };
            let wp = glam::Vec2::new(tx as f32 + 0.5, ty as f32 + 0.5);
            let d = wp.distance(pos);
            match game.level.tiles[i].decor {
                Some(Decor::Chest { opened: false }) => consider(&mut best, "E — Ouvrir le coffre".into(), wp, d),
                Some(Decor::Lever { pulled: false }) => consider(&mut best, "E — Tirer le levier".into(), wp, d),
                Some(Decor::Rune { taken: false }) => consider(&mut best, "E — Prendre la rune".into(), wp, d),
                Some(Decor::Fountain { used: false }) => consider(&mut best, "E — Boire à la fontaine".into(), wp, d),
                Some(Decor::Portal { active: true }) => consider(&mut best, "E — Entrer dans le portail".into(), wp, d),
                _ => {}
            }
        }
    }
    for (e, ep, en) in game.world.query::<(Entity, &crate::game::Pos, &crate::game::enemy::Enemy)>().iter() {
        if en.captive {
            let d = ep.0.distance(pos);
            consider(&mut best, "E — Libérer le villageois".into(), ep.0, d);
            let _ = e;
        }
    }
    best.map(|(t, wp, _)| (t, wp))
}

// ======================================================================
// Inventory screen
// ======================================================================

pub fn draw_inventory(
    ui: &mut Ui,
    inv: &Inventory,
    player_idx: usize,
    sel: usize,
    enchant_mode: Option<Vec<(Enchant, u8, u32)>>,
) -> Vec<Rect> {
    let s = ui.scale();
    ui.dim_overlay(0.55);
    let mut rects = Vec::new();
    let pw = 1100.0 * s;
    let ph = 620.0 * s;
    let px = (ui.w - pw) / 2.0;
    let py = (ui.h - ph) / 2.0;
    ui.panel(px, py, pw, ph, true);
    ui.text(
        &format!("INVENTAIRE — Joueur {}", player_idx + 1),
        px + 18.0 * s,
        py + 14.0 * s,
        SIZE_SMALL,
        ACCENT,
    );
    ui.text(
        &format!("Points d'enchantement : {}", inv.enchant_points),
        px + pw - 320.0 * s,
        py + 14.0 * s,
        SIZE_SMALL,
        PURPLE,
    );

    // equipment column
    let ex = px + 18.0 * s;
    ui.text("ÉQUIPEMENT", ex, py + 44.0 * s, SIZE_SMALL, DIM);
    let slots: [(String, &Option<Item>); 6] = [
        ("Mêlée".into(), &inv.melee),
        ("Distance".into(), &inv.ranged),
        ("Armure".into(), &inv.armor),
        ("Artefact 1".into(), &inv.artifacts[0]),
        ("Artefact 2".into(), &inv.artifacts[1]),
        ("Artefact 3".into(), &inv.artifacts[2]),
    ];
    for (i, (label, slot)) in slots.iter().enumerate() {
        let r = [ex, py + 68.0 * s + i as f32 * 62.0 * s, 300.0 * s, 54.0 * s];
        match slot {
            Some(item) => {
                draw_item_row(ui, r, item, sel == i && enchant_mode.is_none(), true);
            }
            None => {
                ui.frame(r[0], r[1], r[2], r[3], PANEL_DARK, BORDER);
                ui.text(label, r[0] + 12.0 * s, r[1] + 18.0 * s, SIZE_SMALL, DIM);
            }
        }
        rects.push(r);
    }

    // stash grid
    let gx = ex + 330.0 * s;
    ui.text("SAC (21 emplacements)", gx, py + 44.0 * s, SIZE_SMALL, DIM);
    for i in 0..crate::game::inventory::STASH_CAP {
        let col = i % 3;
        let row = i / 3;
        let r = [
            gx + col as f32 * 118.0 * s,
            py + 68.0 * s + row as f32 * 70.0 * s,
            110.0 * s,
            62.0 * s,
        ];
        let idx = 6 + i;
        if let Some(item) = inv.stash.get(i) {
            draw_item_row(ui, r, item, sel == idx && enchant_mode.is_none(), true);
        } else {
            ui.frame(r[0], r[1], r[2], r[3], [0.06, 0.06, 0.09, 0.9], [0.3, 0.3, 0.35, 1.0]);
            if sel == idx {
                ui.frame(r[0], r[1], r[2], r[3], [0.0, 0.0, 0.0, 0.0], ACCENT);
            }
        }
        rects.push(r);
    }

    // details + actions (right)
    let dx = px + pw - 300.0 * s;
    let sel_item: Option<(Option<&Item>, bool)> = if sel < 6 {
        slots[sel].1.as_ref().map(|it| (Some(it), true))
    } else {
        inv.stash.get(sel - 6).map(|it| (Some(it), false))
    };
    ui.frame(dx, py + 44.0 * s, 280.0 * s, 330.0 * s, PANEL_DARK, BORDER);
    if let Some((Some(item), equipped)) = sel_item {
        let name = ui_item_name(ui, item);
        ui.text(&name, dx + 12.0 * s, py + 56.0 * s, SIZE_SMALL, item.rarity.color());
        ui.text(item.rarity.fr(), dx + 12.0 * s, py + 78.0 * s, SIZE_SMALL, DIM);
        ui.text(&format!("Puissance {}", item.power), dx + 12.0 * s, py + 98.0 * s, SIZE_SMALL, WHITE);
        let mut yy = py + 120.0 * s;
        match item.class {
            ItemClass::M(c) => {
                let d = crate::game::items::melee_def(c);
                ui.text(&format!("Dégâts : {}", item.melee_dmg() as i32), dx + 12.0 * s, yy, SIZE_SMALL, WHITE);
                yy += 20.0 * s;
                ui.text(&format!("Vitesse : {:.1}/s", d.speed), dx + 12.0 * s, yy, SIZE_SMALL, WHITE);
                yy += 20.0 * s;
            }
            ItemClass::R(c) => {
                let d = crate::game::items::ranged_def(c);
                ui.text(&format!("Dégâts : {}", item.ranged_dmg() as i32), dx + 12.0 * s, yy, SIZE_SMALL, WHITE);
                yy += 20.0 * s;
                ui.text(&format!("Cadence : {:.1}/s", d.speed), dx + 12.0 * s, yy, SIZE_SMALL, WHITE);
                yy += 20.0 * s;
            }
            ItemClass::A(c) => {
                let d = crate::game::items::armor_def(c);
                ui.text(&format!("PV bonus : {}", item.armor_hp() as i32), dx + 12.0 * s, yy, SIZE_SMALL, WHITE);
                yy += 20.0 * s;
                ui.text(&format!("Vitesse x{:.2}", d.move_mult), dx + 12.0 * s, yy, SIZE_SMALL, WHITE);
                yy += 20.0 * s;
            }
            ItemClass::Art(k) => {
                let d = crate::game::items::artifact_def(k);
                ui.text(d.desc_fr, dx + 12.0 * s, yy, SIZE_SMALL, WHITE);
                yy += 20.0 * s;
            }
        }
        for (en, tier) in &item.enchants {
            ui.text(
                &format!("{} {}", en.name_fr(), "*".repeat(*tier as usize)),
                dx + 12.0 * s,
                yy,
                SIZE_SMALL,
                PURPLE,
            );
            yy += 20.0 * s;
        }
        // actions
        if enchant_mode.is_none() {
            let actions: Vec<(&str, bool)> = if equipped {
                vec![("Ôter", true)]
            } else {
                vec![("Équiper", true), ("Enchanter", inv.enchant_points > 0), ("Rebattre", true)]
            };
            for (ai, (label, enabled)) in actions.iter().enumerate() {
                let r = [dx + 12.0 * s, py + 300.0 * s + ai as f32 * 44.0 * s, 256.0 * s, 38.0 * s];
                ui.button(r, label, sel == 100 + ai, *enabled);
                rects.push(r);
            }
        }
    } else {
        ui.text("Sélectionne un objet", dx + 12.0 * s, py + 56.0 * s, SIZE_SMALL, DIM);
    }

    // enchant overlay
    if let Some(list) = enchant_mode {
        let ew = 420.0 * s;
        let eh = (90.0 + list.len() as f32 * 40.0) * s;
        let ex2 = (ui.w - ew) / 2.0;
        let ey2 = (ui.h - eh) / 2.0;
        ui.frame(ex2, ey2, ew, eh, PANEL, ACCENT);
        ui.text("ENCHANTER (choisis un rang)", ex2 + 14.0 * s, ey2 + 12.0 * s, SIZE_SMALL, ACCENT);
        for (i, (en, tier, cost)) in list.iter().enumerate() {
            let r = [ex2 + 14.0 * s, ey2 + 40.0 * s + i as f32 * 40.0 * s, ew - 28.0 * s, 34.0 * s];
            let label = format!("{} {} — {} pt", en.name_fr(), "*".repeat(*tier as usize), cost);
            ui.button(r, &label, sel == 200 + i, inv.enchant_points >= *cost);
            rects.push(r);
        }
    }
    rects
}

// ======================================================================
// Pause / end screens
// ======================================================================

pub fn draw_pause(ui: &mut Ui, sel: usize) -> Vec<Rect> {
    ui.dim_overlay(0.6);
    let s = ui.scale();
    let cx = ui.w / 2.0;
    ui.text_centered("PAUSE", cx, ui.h * 0.3, SIZE_TITLE, ACCENT);
    let buttons = ["Reprendre", "Inventaire", "Abandonner la mission"];
    let mut rects = Vec::new();
    for (i, b) in buttons.iter().enumerate() {
        let r = [cx - 170.0 * s, ui.h * 0.42 + i as f32 * 64.0 * s, 340.0 * s, 52.0 * s];
        ui.button(r, b, sel == i, true);
        rects.push(r);
    }
    rects
}

pub fn draw_end(
    ui: &mut Ui,
    game: &Game,
    victory: bool,
    sel: usize,
    emeralds_before: u32,
    xp_before: u32,
) -> Vec<Rect> {
    ui.dim_overlay(0.7);
    let s = ui.scale();
    let cx = ui.w / 2.0;
    let mut rects = Vec::new();
    if victory {
        ui.text_centered("MISSION RÉUSSIE !", cx, ui.h * 0.18, SIZE_TITLE, ACCENT);
    } else {
        ui.text_centered("ÉCHEC DE LA MISSION", cx, ui.h * 0.18, SIZE_TITLE, RED);
    }
    let m = &crate::world::missions::MISSIONS[game.mission_id];
    ui.text_centered(&m.name, cx, ui.h * 0.18 + 50.0 * s, SIZE_SMALL, WHITE);
    // stats
    let stats = format!(
        "Ennemis vaincus : {}   Captifs : {}   Émeraudes : +{}",
        game.kills,
        game.captives_rescued,
        game.emeralds
    );
    ui.text_centered(&stats, cx, ui.h * 0.32, SIZE_SMALL, WHITE);
    // items found
    ui.text_centered("Objets trouvés :", cx, ui.h * 0.40, SIZE_SMALL, DIM);
    for (i, item) in game.items_found.iter().take(8).enumerate() {
        let col = i % 4;
        let row = i / 4;
        let r = [
            cx - (4.0 * 130.0 - 10.0) * s / 2.0 + col as f32 * 130.0 * s,
            ui.h * 0.45 + row as f32 * 110.0 * s,
            120.0 * s,
            100.0 * s,
        ];
        ui.frame(r[0], r[1], r[2], r[3], PANEL_DARK, item.rarity.color());
        ui.icon(item.icon(), r[0] + 40.0 * s, r[1] + 6.0 * s, 40.0 * s, WHITE);
        let name = ui_item_name(ui, item);
        let short: String = name.chars().take(16).collect();
        ui.text_centered(&short, r[0] + r[2] / 2.0, r[1] + 52.0 * s, SIZE_SMALL, item.rarity.color());
        ui.text_centered(&format!("P{}", item.power), r[0] + r[2] / 2.0, r[1] + 70.0 * s, SIZE_SMALL, DIM);
    }
    let (lvl, xp, need) = game
        .players
        .first()
        .map(|p| (p.level, p.xp, crate::consts::xp_for_level(p.level)))
        .unwrap_or((1, 0, 1));
    ui.text_centered(
        &format!("Héros : niveau {}   ({} / {} XP)", lvl, xp, need),
        cx,
        ui.h * 0.78,
        SIZE_SMALL,
        PURPLE,
    );
    let _ = emeralds_before;
    let r = [cx - 150.0 * s, ui.h * 0.84, 300.0 * s, 52.0 * s];
    ui.button(r, "Retour au camp", sel == 0, true);
    rects.push(r);
    rects
}

/// Pickup toast list (bottom center) — items auto-collected this session.
pub fn draw_pickup_toasts(ui: &mut Ui, game: &Game) {
    let _ = (ui, game);
    let _ = PickupKind::Heart;
    let _ = Floater { pos: glam::Vec3::ZERO, text: String::new(), color: Vec4::ONE, life: 0.0 };
}

// ======================================================================
// F3 debug overlay (Minecraft-style)
// ======================================================================

pub struct DebugInfo {
    pub fps: f32,
    pub screen: &'static str,
    pub boxes: usize,
    pub billboards: usize,
}

pub fn draw_debug(ui: &mut Ui, game: Option<&Game>, info: &DebugInfo) {
    let s = ui.scale();
    let lh = 18.0 * s;
    let x = 8.0;
    let mut y = 8.0;
    let mut line = |ui: &mut Ui, txt: String, x: f32, y: f32| {
        let w = ui.measure(&txt, SIZE_SMALL) + 8.0;
        ui.rect(x - 2.0, y - 2.0, w, lh, [0.0, 0.0, 0.0, 0.4]);
        ui.text(&txt, x + 2.0, y, SIZE_SMALL, WHITE);
    };

    let ms = if info.fps > 0.0 { 1000.0 / info.fps } else { 0.0 };
    line(ui, format!("Minecraft Dungeons — remake non officiel v{} ({})", env!("CARGO_PKG_VERSION"), info.screen), x, y);
    y += lh;
    line(ui, format!("{:.0} fps ({:.1} ms)  rendu : {} boîtes, {} billboards", info.fps, ms, info.boxes, info.billboards), x, y);
    y += lh;
    line(ui, format!("Résolution : {}x{}  échelle UI : {:.2}", ui.w as i32, ui.h as i32, ui.scale()), x, y);
    y += lh;

    if let Some(g) = game {
        let m = &crate::world::missions::MISSIONS[g.mission_id];
        // player 0 position / facing / tile
        if let Some(p) = g.players.first() {
            let (tx, ty) = (p.pos.x.floor() as i32, p.pos.y.floor() as i32);
            let tile = g.level.idx(tx, ty).map(|i| &g.level.tiles[i]);
            let solid = tile.map(|t| t.solid()).unwrap_or(true);
            let water = tile.map(|t| t.is_water()).unwrap_or(false);
            let facing = compass(p.yaw);
            line(ui, format!("XYZ : {:.2} / {:.2} / {:.2}", p.pos.x, 0.0, p.pos.y), x, y);
            y += lh;
            line(ui, format!("Bloc : {} {}  (index {})  {}{}", tx, ty, tx + ty * g.level.w as i32,
                if solid { "solide" } else { "libre" }, if water { " — eau" } else { "" }), x, y);
            y += lh;
            line(ui, format!("Orientation : {} (yaw {:.0}°)  niveau héros {}", facing, p.yaw.to_degrees().rem_euclid(360.0), p.level), x, y);
            y += lh;
            line(ui, format!("PV : {:.0}/{}  flèches : {}  potions : {}  combo : x{}", p.hp, p.max_hp() as i32, p.arrows, p.potions, p.combo), x, y);
            y += lh;
        }
        line(ui, format!("Biome : {}  —  Mission : {} (menace {})", m.biome.display(), m.name, m.threat), x, y);
        y += lh;
        // entity counts
        let mut enemies = 0usize;
        let mut alive = 0usize;
        for (h, en) in g.world.query::<(&crate::game::Health, &crate::game::enemy::Enemy)>().iter() {
            enemies += 1;
            if h.hp > 0.0 { alive += 1; }
        }
        line(ui, format!(
            "Entités : {}  ennemis : {} (vivants {})  —  projectiles : {}  particules : {}  groupes : {}/{}",
            g.world.iter().count(), enemies, alive, g.projectiles.len(), g.particles.parts.len(),
            g.level.groups.iter().filter(|gr| gr.activated).count(), g.level.groups.len()), x, y);
        y += lh;
        line(ui, format!(
            "Émeraudes : {}  kills : {}  captifs : {}  rune : {}  boss : {}",
            g.emeralds, g.kills, g.captives_rescued, if g.rune_found { "oui" } else { "non" },
            g.boss_entity.map(|_| "présent").unwrap_or("aucun")), x, y);
        y += lh;
        line(ui, format!("Seed : {}  —  palier de menace : {}", g.seed, ["Défaut", "Aventure", "Apocalypse"][g.tier.min(2)]), x, y);
        y += lh;
    }
    line(ui, "F3 : fermer le mode debug".to_string(), x, y);
}

fn compass(yaw: f32) -> &'static str {
    // forward = (-sin yaw, -cos yaw) ; -Z = nord, +X = est
    let f = glam::Vec2::new(-yaw.sin(), -yaw.cos());
    if f.y < -0.707 { "nord (-Z)" }
    else if f.y > 0.707 { "sud (+Z)" }
    else if f.x > 0.0 { "est (+X)" }
    else { "ouest (-X)" }
}
