//! Converts a coverage buffer into characters.

use rayon::prelude::*;

use crate::atlas::Atlas;
use crate::raster::CoverageBuffer;

/// Cells whose brightest sample is below this are left blank.
const EMPTY_THRESHOLD: f32 = 0.02;

#[derive(Debug, Clone, PartialEq)]
pub struct CellGrid {
    pub cols: usize,
    pub rows: usize,
    pub cells: Vec<char>,
}

impl CellGrid {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            cols,
            rows,
            cells: vec![' '; cols * rows],
        }
    }

    pub fn get(&self, col: usize, row: usize) -> char {
        self.cells[row * self.cols + col]
    }

    /// Rows joined by newlines, trailing spaces kept, no trailing newline.
    pub fn to_text(&self) -> String {
        self.cells
            .chunks(self.cols.max(1))
            .map(|row| row.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Runs `pick(patch)` for every cell in parallel over rows. Requires
/// `buf.width == grid.cols * S_w` and `buf.height == grid.rows * S_h`.
fn for_each_cell(
    buf: &CoverageBuffer,
    grid: &mut CellGrid,
    sampling: [usize; 2],
    pick: impl Fn(&[f32]) -> char + Sync,
) {
    let [sw, sh] = sampling;
    let cols = grid.cols;
    if cols == 0 {
        return;
    }
    grid.cells
        .par_chunks_mut(cols)
        .enumerate()
        .for_each(|(row, out)| {
            let mut patch = vec![0.0f32; sw * sh];
            for (col, cell) in out.iter_mut().enumerate() {
                for py in 0..sh {
                    let start = (row * sh + py) * buf.width + col * sw;
                    patch[py * sw..(py + 1) * sw].copy_from_slice(&buf.data[start..start + sw]);
                }
                *cell = if patch.iter().all(|&v| v < EMPTY_THRESHOLD) {
                    ' '
                } else {
                    pick(&patch)
                };
            }
        });
}

/// Picks the glyph whose bitmap is closest to the cell (sum of squared differences).
pub struct GlyphMatcher {
    sampling: [usize; 2],
    /// Sorted by code point so ties resolve to the lowest.
    glyphs: Vec<(char, Vec<f32>)>,
}

impl GlyphMatcher {
    pub fn from_atlas(atlas: &Atlas) -> Self {
        let mut glyphs: Vec<(char, Vec<f32>)> = atlas
            .glyphs
            .iter()
            .map(|g| (g.ch, g.coverage.iter().map(|&v| v as f32 / 255.0).collect()))
            .collect();
        glyphs.sort_by_key(|(c, _)| *c);
        Self {
            sampling: atlas.sampling.map(|v| v as usize),
            glyphs,
        }
    }
}

impl GlyphMatcher {
    pub fn match_cells(&self, buf: &CoverageBuffer, grid: &mut CellGrid) {
        for_each_cell(buf, grid, self.sampling, |patch| {
            let mut best = (f32::INFINITY, ' ');
            for (ch, bitmap) in &self.glyphs {
                let err: f32 = patch
                    .iter()
                    .zip(bitmap)
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum();
                if err < best.0 {
                    best = (err, *ch);
                }
            }
            best.1
        });
    }
}

/// Maps mean cell coverage onto glyphs ordered by ink.
pub struct RampMatcher {
    sampling: [usize; 2],
    /// (ink, char), ascending by ink then code point.
    ramp: Vec<(f32, char)>,
}

impl RampMatcher {
    pub fn from_atlas(atlas: &Atlas) -> Self {
        let mut ramp: Vec<(f32, char)> = atlas.glyphs.iter().map(|g| (g.ink, g.ch)).collect();
        ramp.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        Self {
            sampling: atlas.sampling.map(|v| v as usize),
            ramp,
        }
    }
}

impl RampMatcher {
    pub fn match_cells(&self, buf: &CoverageBuffer, grid: &mut CellGrid) {
        let max_ink = self.ramp.last().map_or(0.0, |r| r.0);
        for_each_cell(buf, grid, self.sampling, |patch| {
            let target = patch.iter().sum::<f32>() / patch.len() as f32 * max_ink;
            let mut best = (f32::INFINITY, ' ');
            for &(ink, ch) in &self.ramp {
                let d = (ink - target).abs();
                if d < best.0 {
                    best = (d, ch);
                }
            }
            best.1
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::Atlas;
    use crate::raster::CoverageBuffer;

    fn atlas() -> Atlas {
        Atlas::embedded_default()
    }

    fn patch_buf(a: &Atlas, ch: char) -> CoverageBuffer {
        let [sw, sh] = a.sampling.map(|v| v as usize);
        let g = a.glyphs.iter().find(|g| g.ch == ch).unwrap();
        let mut b = CoverageBuffer::new(sw, sh);
        for (i, v) in g.coverage.iter().enumerate() {
            b.data[i] = *v as f32 / 255.0;
        }
        b
    }

    #[test]
    fn every_glyph_self_matches() {
        let a = atlas();
        let m = GlyphMatcher::from_atlas(&a);
        let mut grid = CellGrid::new(1, 1);
        let mut wrong = vec![];
        for g in &a.glyphs {
            m.match_cells(&patch_buf(&a, g.ch), &mut grid);
            // Glyphs with identical bitmaps may legitimately resolve to the lowest code point.
            let got = grid.get(0, 0);
            let same = a.glyphs.iter().find(|x| x.ch == got).unwrap().coverage == g.coverage;
            if !same {
                wrong.push((g.ch, got));
            }
        }
        assert!(wrong.is_empty(), "{wrong:?}");
    }

    #[test]
    fn empty_cell_is_space_in_both_modes() {
        let a = atlas();
        let b = CoverageBuffer::new(8, 16);
        let mut g = CellGrid::new(1, 1);
        GlyphMatcher::from_atlas(&a).match_cells(&b, &mut g);
        assert_eq!(g.get(0, 0), ' ');
        RampMatcher::from_atlas(&a).match_cells(&b, &mut g);
        assert_eq!(g.get(0, 0), ' ');
    }

    #[test]
    fn ramp_is_monotonic() {
        let a = atlas();
        let m = RampMatcher::from_atlas(&a);
        let ink = |c: char| a.glyphs.iter().find(|g| g.ch == c).unwrap().ink;
        let mut prev = -1.0;
        let mut g = CellGrid::new(1, 1);
        for step in 0..=10 {
            let mut b = CoverageBuffer::new(8, 16);
            b.data.fill(step as f32 / 10.0);
            m.match_cells(&b, &mut g);
            let i = ink(g.get(0, 0));
            assert!(i >= prev);
            prev = i;
        }
    }
}
