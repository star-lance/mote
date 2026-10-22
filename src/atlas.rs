//! Glyph atlas: per-glyph coverage bitmaps at the sub-cell sampling resolution.

use std::path::Path;

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::glyphs::Palette;

pub const ATLAS_VERSION: u32 = 1;

/// Hashes bytes with SHA-256.
///
/// # Arguments
/// * `bytes` - the data to hash
///
/// # Returns
/// The digest as lowercase hex.
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
    /// Parses and checks an atlas.
    ///
    /// # Arguments
    /// * `s` - atlas JSON
    ///
    /// # Returns
    /// The atlas, or an error for an unsupported version or wrongly sized bitmaps.
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

    /// Serializes the atlas.
    ///
    /// # Returns
    /// Pretty-printed JSON ending in a newline.
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("atlas serializes");
        s.push('\n');
        s
    }

    /// Reads an atlas file.
    ///
    /// # Arguments
    /// * `path` - the JSON file
    ///
    /// # Returns
    /// The atlas, or an error if it can't be read or parsed.
    pub fn load(path: &Path) -> anyhow::Result<Atlas> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Self::from_json(&text).with_context(|| format!("in {}", path.display()))
    }

    /// Loads the atlas built into the binary.
    ///
    /// # Returns
    /// The Ioskeley Mono Regular atlas for printable ASCII.
    pub fn embedded_default() -> Atlas {
        Self::from_json(include_str!("../assets/atlas/default.json"))
            .expect("bundled atlas is valid")
    }

    pub fn chars(&self) -> Vec<char> {
        self.glyphs.iter().map(|g| g.ch).collect()
    }

    /// Restricts the atlas to a palette.
    ///
    /// # Arguments
    /// * `palette` - the characters to keep
    ///
    /// # Returns
    /// A copy containing only glyphs in `palette`.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_default_is_ascii95() {
        let a = Atlas::embedded_default();
        assert_eq!(a.glyphs.len(), 95);
        assert_eq!(a.sampling, [8, 16]);
    }
}
