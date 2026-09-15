//! Glyph cache: pre-rasterizes the font into an atlas at startup.
//! Two sizes (HUD 16px, titles 26px), covers ASCII + French accents.

use ab_glyph::{Font, FontArc, Glyph, PxScale, ScaleFont};
use image::RgbaImage;
use std::collections::HashMap;

const ATLAS_W: u32 = 1024;
const ATLAS_H: u32 = 512;
pub const SIZE_SMALL: usize = 0; // 16 px
pub const SIZE_BIG: usize = 1; // 26 px
pub const SIZE_TITLE: usize = 2; // 44 px
const PX_SIZES: [f32; 3] = [16.0, 26.0, 44.0];

const CHARS: &str = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\
éèêëàâäîïôöûüçÉÈÊËÀÂÄÎÏÔÖÛÜÇ'’«»°…–—×";

#[derive(Clone, Copy)]
pub struct GlyphInfo {
    /// normalized uv rect in glyph atlas
    pub uv: [f32; 4],
    /// pixel size of the quad
    pub size: [f32; 2],
    /// offset from pen position to quad top-left
    pub bearing: [f32; 2],
    pub advance: f32,
}

pub struct GlyphCache {
    pub image: RgbaImage,
    map: HashMap<(char, usize), GlyphInfo>,
    /// per-size font ascent, so bearing[1] maps y from TOP of line
    asc: [f32; 3],
}

impl GlyphCache {
    pub fn new(font: &FontArc) -> GlyphCache {
        let mut img = RgbaImage::new(ATLAS_W, ATLAS_H);
        let mut map = HashMap::new();
        let mut cell_x = 1u32;
        let mut cell_y = 1u32;
        let mut row_h = 0u32;
        let mut asc = [0.0f32; 3];

        for (si, &px) in PX_SIZES.iter().enumerate() {
            let scale = PxScale::from(px);
            let sf = font.as_scaled(scale);
            asc[si] = sf.ascent();
            for c in CHARS.chars() {
                let gid = font.glyph_id(c);
                if gid.0 == 0 && c != ' ' {
                    continue;
                }
                let advance = sf.h_advance(gid);
                if c == ' ' {
                    map.insert((c, si), GlyphInfo {
                        uv: [0.0; 4], size: [0.0; 2], bearing: [0.0; 2], advance,
                    });
                    continue;
                }
                let g = Glyph {
                    id: gid,
                    scale,
                    position: ab_glyph::point(0.0, 0.0),
                };
                let outlined = match font.outline_glyph(g) {
                    Some(o) => o,
                    None => {
                        map.insert((c, si), GlyphInfo {
                            uv: [0.0; 4], size: [0.0; 2], bearing: [0.0; 2], advance,
                        });
                        continue;
                    }
                };
                let b = outlined.px_bounds();
                let bw = b.width().ceil() as u32 + 1;
                let bh = b.height().ceil() as u32 + 1;
                if cell_x + bw + 1 > ATLAS_W {
                    cell_x = 1;
                    cell_y += row_h + 1;
                    row_h = 0;
                }
                if cell_y + bh + 1 > ATLAS_H {
                    break; // atlas full
                }
                // position glyph so its bounds min lands exactly at (cell_x, cell_y)
                let g2 = Glyph {
                    id: gid,
                    scale,
                    position: ab_glyph::point(
                        cell_x as f32 - b.min.x,
                        cell_y as f32 - b.min.y,
                    ),
                };
                if let Some(o2) = font.outline_glyph(g2) {
                    // ab_glyph yields pixel coords RELATIVE to the outline bounds;
                    // the g2 position places the bounds min at (cell_x, cell_y).
                    o2.draw(|x, y, a| {
                        let px = cell_x + x;
                        let py = cell_y + y;
                        if px >= ATLAS_W || py >= ATLAS_H {
                            return;
                        }
                        let p = img.get_pixel_mut(px, py);
                        let old = p.0[3] as f32 / 255.0;
                        let na = (a + old).min(1.0);
                        *p = image::Rgba([255, 255, 255, (na * 255.0) as u8]);
                    });
                }
                map.insert((c, si), GlyphInfo {
                    uv: [
                        cell_x as f32 / ATLAS_W as f32,
                        cell_y as f32 / ATLAS_H as f32,
                        bw as f32 / ATLAS_W as f32,
                        bh as f32 / ATLAS_H as f32,
                    ],
                    size: [bw as f32, bh as f32],
                    // y measured from TOP of the line: ascent + bounds-min
                    bearing: [b.min.x, asc[si] + b.min.y],
                    advance,
                });
                cell_x += bw + 1;
                row_h = row_h.max(bh);
            }
        }
        log::debug!("GlyphCache: {} glyph entries packed to cell ({cell_x},{cell_y})", map.len());
        GlyphCache { image: img, map, asc }
    }

    #[inline]
    pub fn get(&self, c: char, size_idx: usize) -> GlyphInfo {
        *self.map.get(&(c, size_idx)).unwrap_or_else(|| {
            self.map.get(&('?', size_idx)).expect("glyph atlas empty")
        })
    }

    /// Width in pixels of a string at the given size index.
    pub fn measure(&self, text: &str, size_idx: usize) -> f32 {
        text.chars().map(|c| self.get(c, size_idx).advance).sum()
    }

    pub fn px(&self, size_idx: usize) -> f32 {
        PX_SIZES[size_idx]
    }
}
