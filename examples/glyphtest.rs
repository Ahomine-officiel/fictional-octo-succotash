//! Standalone repro of GlyphCache::new rasterization logic.
use ab_glyph::{Font, FontArc, Glyph, PxScale, ScaleFont};

const CHARS: &str = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~éèêëàâäîïôöûüçÉÈÊËÀÂÄÎÏÔÖÛÜÇ'’«»°…–—×";

fn main() {
    let data = std::fs::read("assets/fonts/DejaVuSans.ttf").expect("font");
    let font = FontArc::try_from_vec(data).expect("parse");
    println!("font loaded, units_per_em={:?}", font.units_per_em());

    for &px in &[16.0f32, 44.0] {
        let scale = PxScale::from(px);
        let sf = font.as_scaled(scale);
        let mut ok = 0;
        let mut none = 0;
        let mut zero = 0;
        let mut sample = String::new();
        for c in CHARS.chars() {
            let gid = font.glyph_id(c);
            if gid.0 == 0 && c != ' ' {
                zero += 1;
                continue;
            }
            let g = Glyph { id: gid, scale, position: ab_glyph::point(0.0, 0.0) };
            match font.outline_glyph(g) {
                Some(o) => {
                    let b = o.px_bounds();
                    if ok < 3 {
                        sample.push_str(&format!(
                            "'{c}' bounds=({:.1},{:.1})-({:.1},{:.1}) adv={:.1} | ",
                            b.min.x, b.min.y, b.max.x, b.max.y, sf.h_advance(gid)
                        ));
                    }
                    ok += 1;
                }
                None => none += 1,
            }
        }
        println!("px={px}: rasterized={ok} outline_none={none} gid0_skipped={zero}");
        println!("  sample: {sample}");
    }

    // Now replicate the exact draw-into-atlas path for one glyph
    let scale = PxScale::from(44.0);
    let gid = font.glyph_id('M');
    let g = Glyph { id: gid, scale, position: ab_glyph::point(0.0, 0.0) };
    let outlined = font.outline_glyph(g).expect("M must outline");
    let b = outlined.px_bounds();
    println!("'M' at 44px bounds min=({},{}) size=({},{})", b.min.x, b.min.y, b.width(), b.height());
    let cell = (10u32, 10u32);
    let g2 = Glyph {
        id: gid,
        scale,
        position: ab_glyph::point(cell.0 as f32 - b.min.x, cell.1 as f32 - b.min.y),
    };
    let o2 = font.outline_glyph(g2).expect("M must outline 2");
    let mut count = 0u32;
    let mut minmax = (u32::MAX, u32::MAX, 0u32, 0u32);
    o2.draw(|x, y, a| {
        if count == 0 {
            println!("first pixel at ({x},{y}) alpha={a}");
        }
        count += 1;
        minmax.0 = minmax.0.min(x);
        minmax.1 = minmax.1.min(y);
        minmax.2 = minmax.2.max(x);
        minmax.3 = minmax.3.max(y);
    });
    println!("'M' drew {count} pixels, extent {minmax:?}");

    // ---- replicate the EXACT GlyphCache::new packing loop ----
    println!("--- exact cache loop ---");
    let mut cell_x = 1u32;
    let mut cell_y = 1u32;
    let mut row_h = 0u32;
    let mut inserted = 0usize;
    for (si, &px) in [16.0f32, 26.0, 44.0].iter().enumerate() {
        let scale = PxScale::from(px);
        for c in CHARS.chars() {
            let gid = font.glyph_id(c);
            if gid.0 == 0 && c != ' ' {
                continue;
            }
            if c == ' ' {
                inserted += 1;
                continue;
            }
            let g = Glyph { id: gid, scale, position: ab_glyph::point(0.0, 0.0) };
            let outlined = match font.outline_glyph(g) {
                Some(o) => o,
                None => {
                    inserted += 1;
                    continue;
                }
            };
            let b = outlined.px_bounds();
            let bw = b.width().ceil() as u32 + 1;
            let bh = b.height().ceil() as u32 + 1;
            if si == 0 && inserted < 8 {
                println!("si0 '{c}': cell=({cell_x},{cell_y}) bw={bw} bh={bh}");
            }
            if cell_x + bw + 1 > 1024 {
                cell_x = 1;
                cell_y += row_h + 1;
                row_h = 0;
            }
            if cell_y + bh + 1 > 512 {
                println!("BREAK at size idx {si} char '{c}' (code {}) cell_y={cell_y} bh={bh} inserted={inserted}", c as u32);
                return;
            }
            cell_x += bw + 1;
            row_h = row_h.max(bh);
            inserted += 1;
        }
        println!("size idx {si} done: cell=({cell_x},{cell_y}) inserted={inserted}");
    }
    println!("ALL INSERTED: {inserted}");
}
