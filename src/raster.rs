//! Rasterization of the scene into a sub-cell coverage buffer.

use glam::{DVec2, Vec2};

use crate::geom::{Viewport, project_segment};
use crate::scene::Scene;

/// Coverage values in 0..1, one per sub-cell sample, row-major.
#[derive(Debug, Clone)]
pub struct CoverageBuffer {
    pub width: usize,
    pub height: usize,
    pub data: Vec<f32>,
}

impl CoverageBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![0.0; width * height],
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.data.clear();
        self.data.resize(width * height, 0.0);
    }

    pub fn clear(&mut self) {
        self.data.fill(0.0);
    }

    pub fn get(&self, x: usize, y: usize) -> f32 {
        self.data[y * self.width + x]
    }

    /// Raises the sample to `v` if `v` is larger. Out-of-bounds is ignored.
    pub fn blend_max(&mut self, x: i64, y: i64, v: f32) {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return;
        }
        let s = &mut self.data[y as usize * self.width + x as usize];
        *s = s.max(v);
    }
}

/// Draws every feature edge as an anti-aliased line, see-through.
#[derive(Debug, Clone, Copy)]
pub struct WireframeRasterizer {
    /// Line thickness in samples.
    pub thickness: f32,
}

impl WireframeRasterizer {
    /// Coverage per sample is `clamp(thickness/2 + 0.5 - d, 0, 1)`, where `d` is
    /// the distance from the sample center to the segment.
    pub fn draw_line(&self, buf: &mut CoverageBuffer, a: Vec2, b: Vec2) {
        let (a, b) = (a.as_dvec2(), b.as_dvec2());
        let half = self.thickness as f64 / 2.0;
        let pad = half + 1.0;
        // Clip the bounding box to the buffer first so huge coordinates stay cheap.
        let x0 = (a.x.min(b.x) - pad).floor().max(0.0);
        let y0 = (a.y.min(b.y) - pad).floor().max(0.0);
        let x1 = (a.x.max(b.x) + pad).ceil().min(buf.width as f64);
        let y1 = (a.y.max(b.y) + pad).ceil().min(buf.height as f64);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        let ab = b - a;
        let len = ab.length();
        for y in y0 as i64..y1 as i64 {
            for x in x0 as i64..x1 as i64 {
                let p = DVec2::new(x as f64 + 0.5, y as f64 + 0.5);
                let d = segment_distance(p, a, b, ab, len);
                let cov = (half + 0.5 - d).clamp(0.0, 1.0) as f32;
                if cov > 0.0 {
                    buf.blend_max(x, y, cov);
                }
            }
        }
    }

    pub fn rasterize(&self, scene: &Scene, vp: &Viewport, buf: &mut CoverageBuffer) {
        let view = scene.camera.view();
        for obj in &scene.objects {
            let model = obj.model();
            let world: Vec<_> = obj
                .mesh
                .vertices
                .iter()
                .map(|&v| model.transform_point3(v))
                .collect();
            for &[i, j] in &obj.edges {
                if let Some((a, b)) = project_segment(
                    &view,
                    &scene.camera,
                    vp,
                    world[i as usize],
                    world[j as usize],
                ) {
                    self.draw_line(buf, a, b);
                }
            }
        }
    }
}

/// Distance from `p` to segment `ab`; uses the perpendicular form inside the
/// segment so long segments stay precise.
fn segment_distance(p: DVec2, a: DVec2, b: DVec2, ab: DVec2, len: f64) -> f64 {
    if len == 0.0 {
        return p.distance(a);
    }
    let t = (p - a).dot(ab) / (len * len);
    if t <= 0.0 {
        p.distance(a)
    } else if t >= 1.0 {
        p.distance(b)
    } else {
        ab.perp_dot(p - a).abs() / len
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SceneConfig;
    use crate::geom::Viewport;
    use crate::scene::Scene;
    use glam::Vec2;

    #[test]
    fn horizontal_line_covers_its_row() {
        let mut b = CoverageBuffer::new(20, 10);
        WireframeRasterizer { thickness: 1.0 }.draw_line(
            &mut b,
            Vec2::new(2.0, 5.5),
            Vec2::new(18.0, 5.5),
        );
        assert!(b.get(10, 5) > 0.99);
        assert_eq!(b.get(10, 2), 0.0);
        assert_eq!(b.get(0, 5), 0.0);
    }

    #[test]
    fn offscreen_line_is_ignored() {
        let mut b = CoverageBuffer::new(10, 10);
        WireframeRasterizer { thickness: 2.0 }.draw_line(
            &mut b,
            Vec2::new(-50.0, -50.0),
            Vec2::new(-40.0, 300.0),
        );
        assert!(b.data.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn huge_coordinates_do_not_hang_or_panic() {
        let mut b = CoverageBuffer::new(10, 10);
        WireframeRasterizer { thickness: 1.0 }.draw_line(
            &mut b,
            Vec2::new(-1e7, 5.5),
            Vec2::new(1e7, 5.5),
        );
        assert!(b.get(5, 5) > 0.5);
    }

    #[test]
    fn scene_rasterizes_something_centered() {
        let s = Scene::from_config(&SceneConfig::default(), 1.0);
        let mut b = CoverageBuffer::new(640, 320);
        WireframeRasterizer { thickness: 1.5 }.rasterize(
            &s,
            &Viewport {
                width: 640.0,
                height: 320.0,
                sample_aspect: 1.0,
            },
            &mut b,
        );
        let total: f32 = b.data.iter().sum();
        assert!(total > 100.0);
        assert_eq!(b.get(0, 0), 0.0);
    }
}
