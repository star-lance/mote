//! Triangle meshes and wireframe edge derivation.

use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

use glam::Vec3;

use crate::config::MeshKind;

/// A triangle mesh with counter-clockwise, outward-facing winding.
#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
}

impl Mesh {
    pub fn from_kind(kind: &MeshKind) -> Mesh {
        match *kind {
            MeshKind::Tetrahedron => Self::tetrahedron(),
            MeshKind::Cube => Self::cube(),
            MeshKind::Octahedron => Self::octahedron(),
            MeshKind::Icosahedron => Self::icosahedron(),
            MeshKind::Torus {
                major,
                minor,
                segments,
                sides,
            } => Self::torus(major, minor, segments, sides),
            MeshKind::Sphere { segments, rings } => Self::sphere(segments, rings),
        }
    }

    pub fn tetrahedron() -> Mesh {
        let vertices = [(1., 1., 1.), (1., -1., -1.), (-1., 1., -1.), (-1., -1., 1.)]
            .map(|(x, y, z)| Vec3::new(x, y, z).normalize())
            .to_vec();
        convex(vertices, vec![[0, 1, 2], [0, 1, 3], [0, 2, 3], [1, 2, 3]])
    }

    pub fn cube() -> Mesh {
        let vertices = [
            (-1., -1., -1.),
            (1., -1., -1.),
            (-1., 1., -1.),
            (1., 1., -1.),
            (-1., -1., 1.),
            (1., -1., 1.),
            (-1., 1., 1.),
            (1., 1., 1.),
        ]
        .map(|(x, y, z)| Vec3::new(x, y, z) * 0.6)
        .to_vec();
        // Each face is the four corners sharing one fixed coordinate, as two triangles.
        let quads = [
            [0, 1, 3, 2],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 3, 7, 6],
            [0, 2, 6, 4],
            [1, 3, 7, 5],
        ];
        let triangles = quads
            .iter()
            .flat_map(|q| [[q[0], q[1], q[2]], [q[0], q[2], q[3]]])
            .collect();
        convex(vertices, triangles)
    }

    pub fn octahedron() -> Mesh {
        let vertices = vec![Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z];
        let mut triangles = Vec::new();
        for x in [0, 1] {
            for y in [2, 3] {
                for z in [4, 5] {
                    triangles.push([x, y, z]);
                }
            }
        }
        convex(vertices, triangles)
    }

    pub fn icosahedron() -> Mesh {
        let t = (1.0 + 5f32.sqrt()) / 2.0;
        let mut vertices = Vec::new();
        for a in [-1.0, 1.0] {
            for b in [-t, t] {
                vertices.push(Vec3::new(0.0, a, b));
                vertices.push(Vec3::new(a, b, 0.0));
                vertices.push(Vec3::new(b, 0.0, a));
            }
        }
        let vertices: Vec<Vec3> = vertices.into_iter().map(Vec3::normalize).collect();
        // Faces are exactly the triples of mutually adjacent (shortest-edge) vertices.
        let edge = vertices[1..]
            .iter()
            .map(|v| v.distance(vertices[0]))
            .fold(f32::MAX, f32::min);
        let adjacent = |i: usize, j: usize| (vertices[i].distance(vertices[j]) - edge).abs() < 1e-3;
        let n = vertices.len();
        let mut triangles = Vec::new();
        for i in 0..n {
            for j in i + 1..n {
                for k in j + 1..n {
                    if adjacent(i, j) && adjacent(j, k) && adjacent(i, k) {
                        triangles.push([i as u32, j as u32, k as u32]);
                    }
                }
            }
        }
        convex(vertices, triangles)
    }

    /// A torus around the Y axis: `segments` around the ring, `sides` around the tube.
    pub fn torus(major: f32, minor: f32, segments: u32, sides: u32) -> Mesh {
        let (n, m) = (segments, sides);
        let mut vertices = Vec::with_capacity((n * m) as usize);
        for i in 0..n {
            let u = TAU * i as f32 / n as f32;
            for j in 0..m {
                let v = TAU * j as f32 / m as f32;
                let r = major + minor * v.cos();
                vertices.push(Vec3::new(r * u.cos(), minor * v.sin(), r * u.sin()));
            }
        }
        let idx = |i: u32, j: u32| (i % n) * m + (j % m);
        let mut triangles = Vec::new();
        for i in 0..n {
            for j in 0..m {
                let q = [idx(i, j), idx(i + 1, j), idx(i + 1, j + 1), idx(i, j + 1)];
                triangles.push([q[0], q[1], q[2]]);
                triangles.push([q[0], q[2], q[3]]);
            }
        }
        // "Inside" is the nearest point on the tube's center circle.
        let inside = |c: Vec3| Vec3::new(c.x, 0.0, c.z).normalize_or_zero() * major;
        orient(
            Mesh {
                vertices,
                triangles,
            },
            inside,
        )
    }

    /// A unit UV sphere: two poles plus `rings - 1` latitude rings of `segments` vertices.
    pub fn sphere(segments: u32, rings: u32) -> Mesh {
        let (n, r) = (segments, rings);
        let mut vertices = vec![Vec3::Y, -Vec3::Y];
        for k in 1..r {
            let theta = PI * k as f32 / r as f32;
            for i in 0..n {
                let phi = TAU * i as f32 / n as f32;
                vertices.push(Vec3::new(
                    theta.sin() * phi.cos(),
                    theta.cos(),
                    theta.sin() * phi.sin(),
                ));
            }
        }
        let ring = |k: u32, i: u32| 2 + (k - 1) * n + (i % n);
        let mut triangles = Vec::new();
        for i in 0..n {
            triangles.push([0, ring(1, i), ring(1, i + 1)]);
            triangles.push([1, ring(r - 1, i), ring(r - 1, i + 1)]);
            for k in 1..r - 1 {
                let q = [
                    ring(k, i),
                    ring(k, i + 1),
                    ring(k + 1, i + 1),
                    ring(k + 1, i),
                ];
                triangles.push([q[0], q[1], q[2]]);
                triangles.push([q[0], q[2], q[3]]);
            }
        }
        convex(vertices, triangles)
    }
}

fn convex(vertices: Vec<Vec3>, triangles: Vec<[u32; 3]>) -> Mesh {
    let center = vertices.iter().copied().sum::<Vec3>() / vertices.len() as f32;
    orient(
        Mesh {
            vertices,
            triangles,
        },
        |_| center,
    )
}

/// Flips triangles so each faces away from `inside(triangle centroid)`.
fn orient(mut mesh: Mesh, inside: impl Fn(Vec3) -> Vec3) -> Mesh {
    for t in &mut mesh.triangles {
        let [a, b, c] = t.map(|i| mesh.vertices[i as usize]);
        let centroid = (a + b + c) / 3.0;
        if (b - a).cross(c - a).dot(centroid - inside(centroid)) < 0.0 {
            t.swap(1, 2);
        }
    }
    mesh
}

/// Undirected edges (`[min, max]`, sorted) that are boundary edges or whose
/// adjacent faces meet at more than `angle_deg`. Edges between coplanar
/// triangles (e.g. a cube face's diagonal) are dropped.
pub fn feature_edges(mesh: &Mesh, angle_deg: f32) -> Vec<[u32; 2]> {
    let mut normals: HashMap<[u32; 2], Vec<Vec3>> = HashMap::new();
    for t in &mesh.triangles {
        let [a, b, c] = t.map(|i| mesh.vertices[i as usize]);
        let n = (b - a).cross(c - a).normalize_or_zero();
        for (p, q) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            normals.entry([p.min(q), p.max(q)]).or_default().push(n);
        }
    }
    let threshold = angle_deg.to_radians();
    let mut edges: Vec<[u32; 2]> = normals
        .into_iter()
        .filter(|(_, ns)| ns.len() != 2 || ns[0].angle_between(ns[1]) > threshold)
        .map(|(e, _)| e)
        .collect();
    edges.sort_unstable();
    edges
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn feature_edge_counts() {
        assert_eq!(feature_edges(&Mesh::tetrahedron(), 1.0).len(), 6);
        assert_eq!(feature_edges(&Mesh::cube(), 1.0).len(), 12);
        assert_eq!(feature_edges(&Mesh::octahedron(), 1.0).len(), 12);
        assert_eq!(feature_edges(&Mesh::icosahedron(), 1.0).len(), 30);
        assert_eq!(
            feature_edges(&Mesh::torus(1.5, 0.6, 24, 12), 1.0).len(),
            2 * 24 * 12
        );
        assert_eq!(
            feature_edges(&Mesh::sphere(16, 10), 1.0).len(),
            16 * (2 * 10 - 1)
        );
    }

    #[test]
    fn convex_meshes_wound_outward() {
        for m in [
            Mesh::tetrahedron(),
            Mesh::cube(),
            Mesh::octahedron(),
            Mesh::icosahedron(),
            Mesh::sphere(8, 6),
        ] {
            let c = m.vertices.iter().copied().sum::<Vec3>() / m.vertices.len() as f32;
            for t in &m.triangles {
                let [a, b, d] = t.map(|i| m.vertices[i as usize]);
                assert!((b - a).cross(d - a).dot((a + b + d) / 3.0 - c) > 0.0);
            }
        }
    }
}
