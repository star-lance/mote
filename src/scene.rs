//! Scene objects, their animation, and mouse camera controls.

use glam::{EulerRot, Mat4, Quat, Vec3};
use ratatui::crossterm::event::MouseButton;

use crate::config::{ObjectConfig, SceneConfig};
use crate::geom::{Camera, Mesh, feature_edges};
use crate::input::Event;

#[derive(Debug, Clone)]
pub struct SceneObject {
    pub mesh: Mesh,
    pub edges: Vec<[u32; 2]>,
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: f32,
    pub spin: Vec3,
}

impl SceneObject {
    /// Builds a scene object.
    ///
    /// # Arguments
    /// * `c` - the object's config
    /// * `edge_angle_deg` - angle threshold for wireframe edges
    ///
    /// # Returns
    /// The object with its edges precomputed.
    pub fn from_config(c: &ObjectConfig, edge_angle_deg: f32) -> SceneObject {
        let mesh = Mesh::from_kind(&c.mesh);
        let edges = feature_edges(&mesh, edge_angle_deg);
        let [rx, ry, rz] = c.rotation.map(f32::to_radians);
        SceneObject {
            mesh,
            edges,
            position: Vec3::from(c.position),
            rotation: Quat::from_euler(EulerRot::XYZ, rx, ry, rz),
            scale: c.scale,
            spin: Vec3::from(c.spin.map(f32::to_radians)),
        }
    }

    pub fn model(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(Vec3::splat(self.scale), self.rotation, self.position)
    }
}

#[derive(Debug, Clone)]
pub struct Scene {
    pub camera: Camera,
    pub objects: Vec<SceneObject>,
}

impl Scene {
    /// Builds the scene.
    ///
    /// # Arguments
    /// * `c` - camera and object config
    /// * `edge_angle_deg` - angle threshold for wireframe edges
    ///
    /// # Returns
    /// The scene.
    pub fn from_config(c: &SceneConfig, edge_angle_deg: f32) -> Scene {
        Scene {
            camera: Camera::from_config(&c.camera),
            objects: c
                .objects
                .iter()
                .map(|o| SceneObject::from_config(o, edge_angle_deg))
                .collect(),
        }
    }

    /// Advances each object's spin.
    ///
    /// # Arguments
    /// * `dt` - elapsed time in seconds
    pub fn update(&mut self, dt: f32) {
        for o in &mut self.objects {
            let step = o.spin * dt;
            o.rotation =
                (o.rotation * Quat::from_euler(EulerRot::XYZ, step.x, step.y, step.z)).normalize();
        }
    }
}

#[derive(Debug, Default)]
pub struct CameraControls {
    drag_from: Option<(u16, u16)>,
}

const ORBIT_RAD_PER_CELL: f32 = 0.05;

impl CameraControls {
    /// Applies mouse input to the camera: drag orbits, scroll zooms.
    ///
    /// # Arguments
    /// * `ev` - the input event
    /// * `camera` - the camera to move
    pub fn handle_event(&mut self, ev: &Event, camera: &mut Camera) {
        match *ev {
            Event::MouseDown {
                col,
                row,
                button: MouseButton::Left,
            } => self.drag_from = Some((col, row)),
            Event::MouseDrag {
                col,
                row,
                button: MouseButton::Left,
            } => {
                if let Some((c0, r0)) = self.drag_from {
                    let dx = col as f32 - c0 as f32;
                    let dy = row as f32 - r0 as f32;
                    camera.orbit(-dx * ORBIT_RAD_PER_CELL, dy * ORBIT_RAD_PER_CELL);
                }
                self.drag_from = Some((col, row));
            }
            Event::MouseUp { .. } => self.drag_from = None,
            Event::Scroll { delta, .. } => camera.dolly(if delta < 0 { 0.9 } else { 1.1 }),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SceneConfig;
    use crate::input::Event;
    use ratatui::crossterm::event::MouseButton;

    #[test]
    fn scene_from_default_config() {
        let s = Scene::from_config(&SceneConfig::default(), 1.0);
        assert_eq!(s.objects.len(), 1);
        assert_eq!(s.objects[0].edges.len(), 2 * 24 * 12);
    }

    #[test]
    fn update_applies_spin() {
        let mut s = Scene::from_config(&SceneConfig::default(), 1.0);
        let before = s.objects[0].rotation;
        s.update(0.5);
        assert!(before.angle_between(s.objects[0].rotation) > 0.1);
    }

    #[test]
    fn drag_orbits_the_camera() {
        let mut s = Scene::from_config(&SceneConfig::default(), 1.0);
        let mut c = CameraControls::default();
        let p0 = s.camera.position;
        c.handle_event(
            &Event::MouseDown {
                col: 10,
                row: 10,
                button: MouseButton::Left,
            },
            &mut s.camera,
        );
        c.handle_event(
            &Event::MouseDrag {
                col: 20,
                row: 10,
                button: MouseButton::Left,
            },
            &mut s.camera,
        );
        assert!((s.camera.position - p0).length() > 0.1);
    }
}
