//! 3D geometry: meshes, camera, and projection.

pub mod camera;
pub mod mesh;

pub use camera::{Camera, Viewport, project_segment};
pub use mesh::{Mesh, feature_edges};
