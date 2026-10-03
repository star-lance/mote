//! Builds an [`Atlas`] from a font file.

use crate::atlas::{ATLAS_VERSION, Atlas, AtlasGlyph, sha256_hex};
use crate::glyphs::{DenyList, Palette};
use anyhow::{anyhow, bail};
use fontdue::{Font, FontSettings};

pub struct GenParams {
    pub font_bytes: Vec<u8>,
    pub font_size_px: f32,
    /// Overrides the cell size derived from font metrics.
    pub cell_px: Option<[u32; 2]>,
    /// Overrides the baseline derived from font metrics.
    pub baseline_px: Option<u32>,
    pub sampling: [u32; 2],
    pub palette: Palette,
    pub deny: DenyList,
}

/// Cell size and baseline (pixels from the top) as a terminal would lay out
/// `font` at `size_px`: whole-pixel advance and line height.
fn cell_metrics(font: &Font, size_px: f32) -> ([u32; 2], u32) {
    let width = font.metrics('M', size_px).advance_width.ceil() as u32;
    let (height, baseline) = match font.horizontal_line_metrics(size_px) {
        Some(m) => (
            (m.ascent - m.descent + m.line_gap).ceil() as u32,
            m.ascent.ceil() as u32,
        ),
        None => (size_px.ceil() as u32, (size_px * 0.8).ceil() as u32),
    };
    ([width.max(1), height.max(1)], baseline)
}

/// Fails if any palette character is denied or missing from the font.
pub fn generate(p: &GenParams) -> anyhow::Result<Atlas> {
    let font = Font::from_bytes(p.font_bytes.as_slice(), FontSettings::default())
        .map_err(|e| anyhow!("loading font: {e}"))?;
    if p.sampling.contains(&0) {
        bail!("sampling dimensions must be non-zero");
    }

    p.palette.validate(&p.deny)?;
    let missing: Vec<char> = p
        .palette
        .chars()
        .iter()
        .copied()
        .filter(|&c| c != ' ' && font.lookup_glyph_index(c) == 0)
        .collect();
    if !missing.is_empty() {
        bail!("font has no glyph for {missing:?}");
    }

    let (derived_cell, derived_baseline) = cell_metrics(&font, p.font_size_px);
    let cell_px = p.cell_px.unwrap_or(derived_cell);
    let baseline_px = p.baseline_px.unwrap_or(derived_baseline);

    let glyphs = p
        .palette
        .chars()
        .iter()
        .map(|&ch| {
            let canvas = render_cell(&font, ch, p.font_size_px, cell_px, baseline_px);
            let coverage: Vec<u8> = downsample(&canvas, cell_px, p.sampling)
                .into_iter()
                .map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
                .collect();
            let ink =
                coverage.iter().map(|&v| v as f32 / 255.0).sum::<f32>() / coverage.len() as f32;
            AtlasGlyph { ch, coverage, ink }
        })
        .collect();

    Ok(Atlas {
        version: ATLAS_VERSION,
        font_name: font.name().unwrap_or("unknown").to_string(),
        font_sha256: sha256_hex(&p.font_bytes),
        font_size_px: p.font_size_px,
        cell_px,
        baseline_px,
        sampling: p.sampling,
        glyphs,
    })
}

/// Rasterizes `ch` into a `cell_px` canvas of coverage values in 0..1.
fn render_cell(
    font: &Font,
    ch: char,
    size_px: f32,
    cell_px: [u32; 2],
    baseline_px: u32,
) -> Vec<f32> {
    let [w, h] = cell_px.map(|v| v as i32);
    let mut canvas = vec![0.0f32; (w * h) as usize];
    let (m, bitmap) = font.rasterize(ch, size_px);
    let x0 = m.xmin;
    let y0 = baseline_px as i32 - m.height as i32 - m.ymin;
    for gy in 0..m.height as i32 {
        for gx in 0..m.width as i32 {
            let (x, y) = (x0 + gx, y0 + gy);
            if (0..w).contains(&x) && (0..h).contains(&y) {
                canvas[(y * w + x) as usize] =
                    bitmap[(gy * m.width as i32 + gx) as usize] as f32 / 255.0;
            }
        }
    }
    canvas
}

/// Area-averages a `src_size` canvas into `dst_size` samples.
fn downsample(src: &[f32], src_size: [u32; 2], dst_size: [u32; 2]) -> Vec<f32> {
    let [sw, sh] = src_size.map(|v| v as usize);
    let [dw, dh] = dst_size.map(|v| v as usize);
    let sx = sw as f32 / dw as f32;
    let sy = sh as f32 / dh as f32;
    let mut out = vec![0.0f32; dw * dh];
    for oy in 0..dh {
        let (y_lo, y_hi) = (oy as f32 * sy, (oy + 1) as f32 * sy);
        for ox in 0..dw {
            let (x_lo, x_hi) = (ox as f32 * sx, (ox + 1) as f32 * sx);
            let mut sum = 0.0;
            for py in y_lo.floor() as usize..(y_hi.ceil() as usize).min(sh) {
                let wy = (y_hi.min(py as f32 + 1.0) - y_lo.max(py as f32)).max(0.0);
                for px in x_lo.floor() as usize..(x_hi.ceil() as usize).min(sw) {
                    let wx = (x_hi.min(px as f32 + 1.0) - x_lo.max(px as f32)).max(0.0);
                    sum += src[py * sw + px] * wx * wy;
                }
            }
            out[oy * dw + ox] = sum / (sx * sy);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::Atlas;
    use crate::glyphs::{DenyList, Palette};

    const FONT: &[u8] = include_bytes!("../assets/fonts/IoskeleyMonoTermNerdFontMono-Regular.ttf");

    fn params() -> GenParams {
        GenParams {
            font_bytes: FONT.to_vec(),
            font_size_px: 16.0,
            cell_px: None,
            baseline_px: None,
            sampling: [8, 16],
            palette: Palette::ascii95(),
            deny: DenyList::new(&[]),
        }
    }

    #[test]
    fn ascii95_fully_covered() {
        let atlas = generate(&params()).unwrap();
        assert_eq!(atlas.glyphs.len(), 95);
        assert!(atlas.glyphs.iter().all(|g| g.coverage.len() == 128));
    }

    #[test]
    fn space_is_empty_and_ink_ordering_sane() {
        let a = generate(&params()).unwrap();
        let ink = |c| a.glyphs.iter().find(|g| g.ch == c).unwrap().ink;
        assert_eq!(ink(' '), 0.0);
        assert!(ink('@') > ink('.'));
        assert!(ink('#') > ink('-'));
    }

    #[test]
    fn deterministic() {
        assert_eq!(
            generate(&params()).unwrap().to_json(),
            generate(&params()).unwrap().to_json()
        );
    }

    #[test]
    fn missing_glyph_reported() {
        let mut p = params();
        p.palette = Palette::parse("a\u{1D11E}").unwrap(); // musical G clef, absent from the font
        assert!(generate(&p).is_err());
    }

    #[test]
    fn denied_palette_char_rejected() {
        let mut p = params();
        p.palette = Palette::parse("a\u{2588}").unwrap();
        assert!(generate(&p).is_err());
    }

    #[test]
    fn json_roundtrip_and_shape_check() {
        let a = generate(&params()).unwrap();
        assert_eq!(Atlas::from_json(&a.to_json()).unwrap(), a);
        let bad = a.to_json().replacen("\"version\": 1", "\"version\": 2", 1);
        assert!(Atlas::from_json(&bad).is_err());
    }
}
