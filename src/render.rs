//! Orchestrates rasterization and matching for one frame.

use crate::atlas::Atlas;
use crate::config::{Config, MatcherMode};
use crate::geom::Viewport;
use crate::glyphs::Palette;
use crate::matcher::{CellGrid, GlyphMatcher, RampMatcher};
use crate::raster::{CoverageBuffer, WireframeRasterizer};
use crate::scene::Scene;

pub struct Renderer {
    rasterizer: WireframeRasterizer,
    glyph: GlyphMatcher,
    ramp: RampMatcher,
    pub mode: MatcherMode,
    sampling: [usize; 2],
    buf: CoverageBuffer,
    grid: CellGrid,
}

impl Renderer {
    /// The matchers only ever choose characters from `palette`.
    pub fn new(cfg: &Config, atlas: &Atlas, palette: &Palette) -> Self {
        let atlas = atlas.subset(palette);
        Self {
            rasterizer: WireframeRasterizer {
                thickness: cfg.render.line_thickness,
            },
            glyph: GlyphMatcher::from_atlas(&atlas),
            ramp: RampMatcher::from_atlas(&atlas),
            mode: cfg.matcher.mode,
            sampling: atlas.sampling.map(|v| v as usize),
            buf: CoverageBuffer::new(0, 0),
            grid: CellGrid::new(0, 0),
        }
    }

    /// Renders `scene` to a `cols` × `rows` grid. `cell_aspect` is the
    /// terminal cell's pixel width / height.
    pub fn render(&mut self, scene: &Scene, cols: u16, rows: u16, cell_aspect: f32) -> &CellGrid {
        let (cols, rows) = (cols as usize, rows as usize);
        let [sw, sh] = self.sampling;
        if self.grid.cols != cols || self.grid.rows != rows {
            self.grid = CellGrid::new(cols, rows);
            self.buf.resize(cols * sw, rows * sh);
        } else {
            self.buf.clear();
        }
        if cols == 0 || rows == 0 {
            return &self.grid;
        }
        let vp = Viewport {
            width: (cols * sw) as f32,
            height: (rows * sh) as f32,
            sample_aspect: cell_aspect * sh as f32 / sw as f32,
        };
        self.rasterizer.rasterize(scene, &vp, &mut self.buf);
        match self.mode {
            MatcherMode::Match => self.glyph.match_cells(&self.buf, &mut self.grid),
            MatcherMode::Ramp => self.ramp.match_cells(&self.buf, &mut self.grid),
        }
        &self.grid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::Atlas;
    use crate::config::{Config, SceneConfig};
    use crate::glyphs::Palette;
    use crate::scene::Scene;

    #[test]
    fn output_uses_only_the_configured_palette() {
        let palette = Palette::parse(" .-|/\\_").unwrap();
        let mut r = Renderer::new(&Config::default(), &Atlas::embedded_default(), &palette);
        let s = Scene::from_config(&SceneConfig::default(), 1.0);
        let grid = r.render(&s, 80, 24, 0.5);
        let outside: Vec<char> = grid
            .cells
            .iter()
            .copied()
            .filter(|c| !palette.chars().contains(c))
            .collect();
        assert!(outside.is_empty(), "{outside:?}");
        assert!(grid.cells.iter().any(|&c| c != ' '));
    }

    #[test]
    fn zero_size_render_is_empty() {
        let mut r = Renderer::new(
            &Config::default(),
            &Atlas::embedded_default(),
            &Palette::ascii95(),
        );
        let s = Scene::from_config(&SceneConfig::default(), 1.0);
        assert_eq!(r.render(&s, 0, 0, 0.5).cells.len(), 0);
        assert_eq!(r.render(&s, 1, 1, 0.5).cells.len(), 1);
    }
}
