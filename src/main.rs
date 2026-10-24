//! mote: an ASCII wireframe renderer with a login panel.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, bail};
use clap::Parser;
use mote::atlas::{Atlas, sha256_hex};
use mote::{app, config, term};

#[derive(Parser)]
#[command(about = "ASCII wireframe renderer")]
struct Args {
    /// Config file (default: $XDG_CONFIG_HOME/mote/config.toml).
    #[arg(long)]
    config: Option<PathBuf>,
}

/// Loads the config and atlas, then starts the app.
fn run(args: Args) -> anyhow::Result<()> {
    let (cfg, warnings) = config::load(args.config.as_deref())?;
    for key in warnings {
        eprintln!("mote: warning: unknown config key '{key}'");
    }
    let glyphs = cfg.validate()?;

    let atlas = match &cfg.glyphs.atlas {
        Some(path) => Atlas::load(path)?,
        None => Atlas::embedded_default(),
    };
    let missing: Vec<char> = glyphs
        .palette
        .chars()
        .iter()
        .copied()
        .filter(|&c| !atlas.glyphs.iter().any(|g| g.ch == c))
        .collect();
    if !missing.is_empty() {
        bail!(
            "palette characters {missing:?} are not in the atlas; regenerate it with glyphgen --palette"
        );
    }
    if atlas.sampling != cfg.matcher.sampling {
        bail!(
            "atlas sampling {:?} differs from matcher.sampling {:?}; regenerate it with glyphgen --sampling",
            atlas.sampling,
            cfg.matcher.sampling
        );
    }
    if let Some(font) = &cfg.glyphs.font {
        let bytes = std::fs::read(font).with_context(|| format!("reading {}", font.display()))?;
        if sha256_hex(&bytes) != atlas.font_sha256 {
            eprintln!(
                "mote: warning: atlas was generated from a different font than {}",
                font.display()
            );
        }
    }
    if let Some([w, h]) = term::cell_pixel_size()
        && [w as u32, h as u32] != atlas.cell_px
    {
        eprintln!(
            "mote: warning: terminal cell is {w}x{h}px but the atlas was built for {}x{}px; regenerate it with glyphgen --cell {w}x{h}",
            atlas.cell_px[0], atlas.cell_px[1]
        );
    }
    app::run(cfg, glyphs, atlas)
}

fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("mote: {e:#}");
            ExitCode::FAILURE
        }
    }
}
