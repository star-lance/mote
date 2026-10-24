//! Rasterization of the scene into a sub-cell coverage buffer.

use glam::{DVec2, Vec2};

use crate::geom::{Viewport, project_segment};
use crate::scene::Scene;

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

    pub fn get(&self, x: usize, y: usize) -> f32 {
        self.data[y * self.width + x]
    }
}

/// Draws an anti-aliased line between two points in sample coordinates.
pub fn draw_line(buf: &mut CoverageBuffer, thickness: f32, a: Vec2, b: Vec2) {
    let (a, b) = (a.as_dvec2(), b.as_dvec2());
    let half = thickness as f64 / 2.0;
    let pad = half + 1.0;
    let x0 = (a.x.min(b.x) - pad).floor().max(0.0);
    let y0 = (a.y.min(b.y) - pad).floor().max(0.0);
    let x1 = (a.x.max(b.x) + pad).ceil().min(buf.width as f64);
    let y1 = (a.y.max(b.y) + pad).ceil().min(buf.height as f64);
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    let ab = b - a;
    for y in y0 as usize..y1 as usize {
        for x in x0 as usize..x1 as usize {
            let p = DVec2::new(x as f64 + 0.5, y as f64 + 0.5);
            let t = ((p - a).dot(ab) / ab.length_squared().max(f64::MIN_POSITIVE)).clamp(0.0, 1.0);
            let cov = (half + 0.5 - p.distance(a + ab * t)).clamp(0.0, 1.0) as f32;
            let s = &mut buf.data[y * buf.width + x];
            *s = s.max(cov);
        }
    }
}

/// Draws every wireframe edge of every object.
pub fn rasterize(scene: &Scene, vp: &Viewport, thickness: f32, buf: &mut CoverageBuffer) {
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
                draw_line(buf, thickness, a, b);
            }
        }
    }
}

// These tests were written by Claude Code to ensure code coverage.
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
        draw_line(&mut b, 1.0, Vec2::new(2.0, 5.5), Vec2::new(18.0, 5.5));
        assert!(b.get(10, 5) > 0.99);
        assert_eq!(b.get(10, 2), 0.0);
        assert_eq!(b.get(0, 5), 0.0);
    }

    #[test]
    fn offscreen_line_is_ignored() {
        let mut b = CoverageBuffer::new(10, 10);
        draw_line(
            &mut b,
            2.0,
            Vec2::new(-50.0, -50.0),
            Vec2::new(-40.0, 300.0),
        );
        assert!(b.data.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn huge_coordinates_do_not_hang_or_panic() {
        let mut b = CoverageBuffer::new(10, 10);
        draw_line(&mut b, 1.0, Vec2::new(-1e7, 5.5), Vec2::new(1e7, 5.5));
        assert!(b.get(5, 5) > 0.5);
    }

    #[test]
    fn scene_rasterizes_something_centered() {
        let s = Scene::from_config(&SceneConfig::default(), 1.0);
        let mut b = CoverageBuffer::new(640, 320);
        rasterize(
            &s,
            &Viewport {
                width: 640.0,
                height: 320.0,
                sample_aspect: 1.0,
            },
            1.5,
            &mut b,
        );
        let total: f32 = b.data.iter().sum();
        assert!(total > 100.0);
        assert_eq!(b.get(0, 0), 0.0);
    }
}
