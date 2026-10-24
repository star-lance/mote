//! Golden frame tests; regenerate with `UPDATE_GOLDEN=1 cargo test --test golden`.
//! These tests were written by Claude Code to ensure code coverage.

use std::path::PathBuf;

use mote::atlas::Atlas;
use mote::config::{Config, MatcherMode, MeshKind, ObjectConfig, SceneConfig};
use mote::glyphs::Palette;
use mote::render::Renderer;
use mote::scene::Scene;

fn check(name: &str, actual: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(format!("{name}.txt"));
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing golden {}", path.display()));
    assert_eq!(actual, expected, "golden mismatch for {name}");
}

fn render(mesh: MeshKind, rotation: [f32; 3], mode: MatcherMode) -> String {
    let atlas = Atlas::embedded_default();
    let mut cfg = Config::default();
    cfg.matcher.mode = mode;
    cfg.scene = SceneConfig {
        objects: vec![ObjectConfig {
            mesh,
            rotation,
            ..ObjectConfig::default()
        }],
        ..SceneConfig::default()
    };
    let scene = Scene::from_config(&cfg.scene, cfg.render.feature_edge_angle_deg);
    let mut r = Renderer::new(&cfg, &atlas, &Palette::ascii95());
    r.render(&scene, 80, 24, atlas.cell_aspect()).to_text()
}

#[test]
fn golden_frames() {
    let torus = MeshKind::Torus {
        major: 1.5,
        minor: 0.6,
        segments: 24,
        sides: 12,
    };
    let cases = [
        (
            "cube_match",
            MeshKind::Cube,
            [25.0, 35.0, 0.0],
            MatcherMode::Match,
        ),
        (
            "cube_ramp",
            MeshKind::Cube,
            [25.0, 35.0, 0.0],
            MatcherMode::Ramp,
        ),
        ("torus_match", torus, [60.0, 0.0, 20.0], MatcherMode::Match),
        (
            "icosahedron_match",
            MeshKind::Icosahedron,
            [10.0, 20.0, 0.0],
            MatcherMode::Match,
        ),
        (
            "dodecahedron_match",
            MeshKind::Dodecahedron,
            [20.0, 30.0, 0.0],
            MatcherMode::Match,
        ),
    ];
    for (name, mesh, rotation, mode) in cases {
        check(name, &render(mesh, rotation, mode));
    }
}
