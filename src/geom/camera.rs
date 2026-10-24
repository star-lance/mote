//! The scene camera and perspective projection into sample space.

use glam::{Mat4, Vec2, Vec3, Vec4Swizzles};

use crate::config::CameraConfig;

#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub fov_y_deg: f32,
}

const NEAR: f32 = 0.05;
const MAX_PITCH: f32 = 89.0 * std::f32::consts::PI / 180.0;

impl Camera {
    pub fn from_config(c: &CameraConfig) -> Camera {
        Camera {
            position: Vec3::from(c.position),
            target: Vec3::from(c.target),
            fov_y_deg: c.fov_deg,
        }
    }

    pub fn view(&self) -> Mat4 {
        glam::camera::rh::view::look_at_mat4(self.position, self.target, Vec3::Y)
    }

    /// Rotates the camera around its target; pitch is clamped to ±89°.
    pub fn orbit(&mut self, yaw_rad: f32, pitch_rad: f32) {
        let offset = self.position - self.target;
        let r = offset.length();
        if r == 0.0 {
            return;
        }
        let yaw = offset.x.atan2(offset.z) + yaw_rad;
        let pitch =
            ((offset.y / r).clamp(-1.0, 1.0).asin() + pitch_rad).clamp(-MAX_PITCH, MAX_PITCH);
        self.position = self.target
            + r * Vec3::new(
                pitch.cos() * yaw.sin(),
                pitch.sin(),
                pitch.cos() * yaw.cos(),
            );
    }

    /// Multiplies the camera's distance to its target by `factor`.
    pub fn dolly(&mut self, factor: f32) {
        let offset = self.position - self.target;
        let dist = (offset.length() * factor).max(NEAR * 2.0);
        self.position = self.target + offset.normalize_or(Vec3::Z) * dist;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub width: f32,
    pub height: f32,
    pub sample_aspect: f32,
}

/// Projects a world-space segment into sample coordinates (x right, y down), clipped to the
/// near plane, or `None` if it is behind the camera.
pub fn project_segment(
    view: &Mat4,
    cam: &Camera,
    vp: &Viewport,
    a: Vec3,
    b: Vec3,
) -> Option<(Vec2, Vec2)> {
    let mut a = (*view * a.extend(1.0)).xyz();
    let mut b = (*view * b.extend(1.0)).xyz();
    let z_clip = -NEAR;
    match (a.z <= z_clip, b.z <= z_clip) {
        (false, false) => return None,
        (true, false) => b = a + (b - a) * ((z_clip - a.z) / (b.z - a.z)),
        (false, true) => a = b + (a - b) * ((z_clip - b.z) / (a.z - b.z)),
        (true, true) => {}
    }
    let f = 1.0 / (cam.fov_y_deg.to_radians() / 2.0).tan();
    let phys = vp.width * vp.sample_aspect / vp.height;
    let project = |p: Vec3| {
        let x_ndc = f * p.x / (-p.z) / phys;
        let y_ndc = f * p.y / (-p.z);
        Vec2::new(
            (x_ndc + 1.0) / 2.0 * vp.width,
            (1.0 - y_ndc) / 2.0 * vp.height,
        )
    };
    let (pa, pb) = (project(a), project(b));
    (pa.is_finite() && pb.is_finite()).then_some((pa, pb))
}

// These tests were written by Claude Code to ensure code coverage.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CameraConfig;
    use glam::{Vec2, Vec3};

    fn vp() -> Viewport {
        Viewport {
            width: 800.0,
            height: 320.0,
            sample_aspect: 1.0,
        }
    }

    #[test]
    fn target_projects_to_center() {
        let cam = Camera::from_config(&CameraConfig::default());
        let (a, _) = project_segment(&cam.view(), &cam, &vp(), Vec3::ZERO, Vec3::X).unwrap();
        assert!((a - Vec2::new(400.0, 160.0)).length() < 1e-3);
    }

    #[test]
    fn behind_camera_is_culled_and_crossing_is_clipped() {
        let cam = Camera::from_config(&CameraConfig::default());
        assert!(
            project_segment(
                &cam.view(),
                &cam,
                &vp(),
                Vec3::new(0., 0., 7.),
                Vec3::new(1., 0., 8.)
            )
            .is_none()
        );
        let (a, b) = project_segment(
            &cam.view(),
            &cam,
            &vp(),
            Vec3::new(0.5, 0., 0.),
            Vec3::new(0.5, 0., 10.),
        )
        .unwrap();
        assert!(a.is_finite() && b.is_finite());
    }

    #[test]
    fn orbit_keeps_distance_and_clamps_pitch() {
        let mut cam = Camera::from_config(&CameraConfig::default());
        cam.orbit(0.7, 10.0);
        assert!(((cam.position - cam.target).length() - 6.0).abs() < 1e-4);
        assert!(cam.view().is_finite());
    }
}
