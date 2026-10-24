//! Glyph atlas: per-glyph coverage bitmaps at the sub-cell sampling resolution.

use std::path::Path;

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::glyphs::Palette;

pub const ATLAS_VERSION: u32 = 1;

/// Hashes bytes with SHA-256, as lowercase hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Atlas {
    pub version: u32,
    pub font_name: String,
    pub font_sha256: String,
    pub font_size_px: f32,
    pub cell_px: [u32; 2],
    pub baseline_px: u32,
    pub sampling: [u32; 2],
    pub glyphs: Vec<AtlasGlyph>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AtlasGlyph {
    pub ch: char,
    pub coverage: Vec<u8>,
    pub ink: f32,
}

impl Atlas {
    /// Parses an atlas, rejecting unsupported versions and wrongly sized bitmaps.
    pub fn from_json(s: &str) -> anyhow::Result<Atlas> {
        let atlas: Atlas = serde_json::from_str(s).context("parsing atlas")?;
        if atlas.version != ATLAS_VERSION {
            bail!(
                "unsupported atlas version {} (expected {ATLAS_VERSION})",
                atlas.version
            );
        }
        let n = (atlas.sampling[0] * atlas.sampling[1]) as usize;
        if n == 0 {
            bail!("atlas sampling must be non-zero");
        }
        if let Some(g) = atlas.glyphs.iter().find(|g| g.coverage.len() != n) {
            bail!(
                "atlas glyph {:?} has {} samples, expected {n}",
                g.ch,
                g.coverage.len()
            );
        }
        Ok(atlas)
    }

    /// Serializes the atlas as pretty-printed JSON ending in a newline.
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("atlas serializes");
        s.push('\n');
        s
    }

    /// Reads an atlas file.
    pub fn load(path: &Path) -> anyhow::Result<Atlas> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Self::from_json(&text).with_context(|| format!("in {}", path.display()))
    }

    /// Loads the bundled Ioskeley Mono Regular atlas for printable ASCII.
    pub fn embedded_default() -> Atlas {
        Self::from_json(include_str!("../assets/atlas/default.json"))
            .expect("bundled atlas is valid")
    }

    /// Returns a copy containing only the glyphs in `palette`.
    pub fn subset(&self, palette: &Palette) -> Atlas {
        let glyphs = self
            .glyphs
            .iter()
            .filter(|g| palette.chars().contains(&g.ch))
            .cloned()
            .collect();
        Atlas {
            glyphs,
            ..self.clone()
        }
    }

    pub fn cell_aspect(&self) -> f32 {
        self.cell_px[0] as f32 / self.cell_px[1] as f32
    }
}

// These tests were written by Claude Code to ensure code coverage.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_default_is_ascii95() {
        let a = Atlas::embedded_default();
        assert_eq!(a.glyphs.len(), 95);
        assert_eq!(a.sampling, [8, 16]);
    }

    #[test]
    fn load_round_trips_through_a_file() {
        let a = Atlas::embedded_default();
        let path =
            std::env::temp_dir().join(format!("mote-atlas-test-{}.json", std::process::id()));
        std::fs::write(&path, a.to_json()).unwrap();
        let loaded = Atlas::load(&path);
        std::fs::remove_file(&path).unwrap();
        assert_eq!(loaded.unwrap(), a);
    }

    #[test]
    fn rejects_wrongly_sized_bitmaps() {
        let mut a = Atlas::embedded_default();
        a.glyphs[0].coverage.pop();
        assert!(Atlas::from_json(&a.to_json()).is_err());
    }
}
