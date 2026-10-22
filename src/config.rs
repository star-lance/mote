//! TOML configuration: loading, defaults, and validation.

use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use ratatui::crossterm::event::KeyCode;
use ratatui::style::Color;
use serde::Deserialize;

use crate::glyphs::{DenyList, Palette, parse_range};
use crate::input::KeyBinding;

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Config {
    pub fps: u32,
    pub render: RenderConfig,
    pub matcher: MatcherConfig,
    pub glyphs: GlyphConfig,
    pub colors: ColorConfig,
    pub scene: SceneConfig,
    pub ui: UiConfig,
    pub keys: KeyConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fps: 30,
            render: RenderConfig::default(),
            matcher: MatcherConfig::default(),
            glyphs: GlyphConfig::default(),
            colors: ColorConfig::default(),
            scene: SceneConfig::default(),
            ui: UiConfig::default(),
            keys: KeyConfig::default(),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct RenderConfig {
    pub line_thickness: f32,
    pub feature_edge_angle_deg: f32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            line_thickness: 1.5,
            feature_edge_angle_deg: 1.0,
        }
    }
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MatcherMode {
    Match,
    Ramp,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct MatcherConfig {
    pub mode: MatcherMode,
    pub sampling: [u32; 2],
}

impl Default for MatcherConfig {
    fn default() -> Self {
        Self {
            mode: MatcherMode::Match,
            sampling: [8, 16],
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct GlyphConfig {
    pub palette: String,
    pub deny: Vec<String>,
    pub atlas: Option<PathBuf>,
    pub font: Option<PathBuf>,
}

impl Default for GlyphConfig {
    fn default() -> Self {
        Self {
            palette: "ascii95".into(),
            deny: Vec::new(),
            atlas: None,
            font: None,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ColorConfig {
    pub fg: Color,
    pub bg: Color,
    pub ui_fg: Color,
    pub ui_accent: Color,
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            fg: Color::Rgb(0xb0, 0xb0, 0xb0),
            bg: Color::Reset,
            ui_fg: Color::Rgb(0xd0, 0xd0, 0xd0),
            ui_accent: Color::White,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct SceneConfig {
    pub camera: CameraConfig,
    pub objects: Vec<ObjectConfig>,
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            camera: CameraConfig::default(),
            objects: vec![ObjectConfig {
                mesh: MeshKind::Torus {
                    major: default_major(),
                    minor: default_minor(),
                    segments: default_torus_segments(),
                    sides: default_sides(),
                },
                spin: [20.0, 35.0, 0.0],
                ..ObjectConfig::default()
            }],
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct CameraConfig {
    pub position: [f32; 3],
    pub target: [f32; 3],
    pub fov_deg: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 6.0],
            target: [0.0; 3],
            fov_deg: 50.0,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ObjectConfig {
    pub mesh: MeshKind,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: f32,
    pub spin: [f32; 3],
}

impl Default for ObjectConfig {
    fn default() -> Self {
        Self {
            mesh: MeshKind::Cube,
            position: [0.0; 3],
            rotation: [0.0; 3],
            scale: 1.0,
            spin: [0.0; 3],
        }
    }
}

fn default_major() -> f32 {
    1.5
}
fn default_minor() -> f32 {
    0.6
}
fn default_torus_segments() -> u32 {
    24
}
fn default_sides() -> u32 {
    12
}
fn default_sphere_segments() -> u32 {
    16
}
fn default_rings() -> u32 {
    10
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MeshKind {
    Tetrahedron,
    Cube,
    Octahedron,
    Icosahedron,
    Torus {
        #[serde(default = "default_major")]
        major: f32,
        #[serde(default = "default_minor")]
        minor: f32,
        #[serde(default = "default_torus_segments")]
        segments: u32,
        #[serde(default = "default_sides")]
        sides: u32,
    },
    Sphere {
        #[serde(default = "default_sphere_segments")]
        segments: u32,
        #[serde(default = "default_rings")]
        rings: u32,
    },
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct UiConfig {
    pub login: LoginConfig,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LoginPosition {
    #[default]
    Center,
    Top,
    Bottom,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct LoginConfig {
    pub enabled: bool,
    pub position: LoginPosition,
}

impl Default for LoginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            position: LoginPosition::Center,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct KeyConfig {
    pub quit: KeyBinding,
    pub focus_scene: KeyBinding,
    pub focus_login: KeyBinding,
    pub toggle_matcher: KeyBinding,
    pub toggle_debug: KeyBinding,
}

impl Default for KeyConfig {
    fn default() -> Self {
        Self {
            quit: KeyBinding::new(KeyCode::Char('q')),
            focus_scene: KeyBinding::new(KeyCode::Esc),
            focus_login: KeyBinding::new(KeyCode::Tab),
            toggle_matcher: KeyBinding::new(KeyCode::F(2)),
            toggle_debug: KeyBinding::new(KeyCode::F(3)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValidatedGlyphs {
    pub palette: Palette,
    pub deny: DenyList,
}

impl Config {
    /// Checks settings the types can't express and builds the glyph sets.
    ///
    /// # Returns
    /// The palette and denylist, or an error naming the first invalid setting.
    pub fn validate(&self) -> anyhow::Result<ValidatedGlyphs> {
        if self.fps == 0 {
            bail!("fps must be at least 1");
        }
        if self.matcher.sampling.contains(&0) {
            bail!("matcher.sampling dimensions must be non-zero");
        }
        for (i, obj) in self.scene.objects.iter().enumerate() {
            if obj.scale.is_nan() || obj.scale <= 0.0 {
                bail!("scene.objects[{i}].scale must be positive");
            }
            match obj.mesh {
                MeshKind::Torus {
                    segments, sides, ..
                } if segments < 3 || sides < 3 => {
                    bail!("scene.objects[{i}]: torus segments and sides must be at least 3")
                }
                MeshKind::Sphere { segments, rings } if segments < 3 || rings < 2 => {
                    bail!("scene.objects[{i}]: sphere needs segments >= 3 and rings >= 2")
                }
                _ => {}
            }
        }
        let extra = self
            .glyphs
            .deny
            .iter()
            .map(|s| parse_range(s))
            .collect::<Result<Vec<_>, _>>()
            .context("glyphs.deny")?;
        let deny = DenyList::new(&extra);
        let palette = Palette::parse(&self.glyphs.palette).context("glyphs.palette")?;
        palette.validate(&deny).context("glyphs.palette")?;
        Ok(ValidatedGlyphs { palette, deny })
    }
}

/// Parses config text.
///
/// # Arguments
/// * `s` - TOML text
///
/// # Returns
/// The config and the paths of any unknown keys.
pub(crate) fn load_str(s: &str) -> anyhow::Result<(Config, Vec<String>)> {
    let value: toml::Value = toml::from_str(s).context("parsing config")?;
    let mut warnings = Vec::new();
    let config = serde_ignored::deserialize(value, |path| warnings.push(path.to_string()))
        .context("reading config")?;
    Ok((config, warnings))
}

fn default_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("mote").join("config.toml"))
}

/// Loads the config file.
///
/// # Arguments
/// * `path` - file to read, or `None` for `$XDG_CONFIG_HOME/mote/config.toml`
///
/// # Returns
/// The config and the paths of any unknown keys; defaults if the default file doesn't exist.
pub fn load(path: Option<&Path>) -> anyhow::Result<(Config, Vec<String>)> {
    let path = match path {
        Some(p) => p.to_path_buf(),
        None => match default_path() {
            Some(p) if p.exists() => p,
            _ => return Ok((Config::default(), Vec::new())),
        },
    };
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    load_str(&text).with_context(|| format!("in {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> (Config, Vec<String>) {
        load_str(s).unwrap()
    }

    #[test]
    fn empty_config_is_defaults() {
        let (c, w) = parse("");
        assert_eq!(c.fps, 30);
        assert_eq!(c.matcher.sampling, [8, 16]);
        assert_eq!(c.glyphs.palette, "ascii95");
        assert!(w.is_empty());
        assert_eq!(c.scene.objects.len(), 1);
    }

    #[test]
    fn unknown_key_warns() {
        let (_, w) = parse("fps = 20\nbogus = 1\n[render]\nwat = 2\n");
        assert_eq!(w.len(), 2);
        assert!(w.iter().any(|k| k.contains("render.wat")));
    }

    #[test]
    fn objects_parse() {
        let (c, _) = parse(
            r#"
            [[scene.objects]]
            mesh = { type = "cube" }
            position = [2, 0, 0]
            [[scene.objects]]
            mesh = { type = "torus", segments = 8 }
        "#,
        );
        assert_eq!(c.scene.objects.len(), 2);
        assert!(matches!(
            c.scene.objects[1].mesh,
            MeshKind::Torus {
                segments: 8,
                sides: 12,
                ..
            }
        ));
    }

    #[test]
    fn validate_rejects_denied_palette() {
        let (c, _) = parse("[glyphs]\npalette = \"ab▀\"\n");
        assert!(c.validate().is_err());
    }

    #[test]
    fn validate_applies_extra_deny() {
        let (c, _) = parse("[glyphs]\ndeny = [\"U+0040\"]\n");
        assert!(c.validate().is_err());
    }

    #[test]
    fn rejects_zero_fps_bad_color_and_bad_key() {
        assert!(parse("fps = 0").0.validate().is_err());
        assert!(load_str("[colors]\nfg = \"notacolor\"").is_err());
        assert!(load_str("[keys]\nquit = \"hyper+q\"").is_err());
    }

    #[test]
    fn colors_and_keys_parse() {
        let (c, _) = parse("[colors]\nfg = \"#ff0000\"\n[keys]\nquit = \"ctrl+x\"\n");
        assert_eq!(c.colors.fg, Color::Rgb(255, 0, 0));
        assert_eq!(c.keys.quit, KeyBinding::parse("ctrl+x").unwrap());
    }

    #[test]
    fn missing_explicit_path_errors() {
        assert!(load(Some(Path::new("/nonexistent/mote.toml"))).is_err());
    }
}
