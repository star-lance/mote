//! Generates a glyph atlas for `mote` from a font file.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context;
use clap::Parser;
use mote::atlas_gen::{GenParams, generate};
use mote::glyphs::{DenyList, Palette, parse_range};

#[derive(Parser)]
#[command(about = "Build a mote glyph atlas from a monospace font")]
struct Args {
    /// Font file (TTF/OTF).
    #[arg(long)]
    font: PathBuf,
    /// Font size in pixels (points × DPI / 72).
    #[arg(long, default_value_t = 16.0)]
    size_px: f32,
    /// Override the cell size, e.g. 10x22.
    #[arg(long, value_parser = parse_pair)]
    cell: Option<[u32; 2]>,
    /// Override the baseline, in pixels from the top of the cell.
    #[arg(long)]
    baseline: Option<u32>,
    /// Samples per cell, e.g. 8x16.
    #[arg(long, value_parser = parse_pair, default_value = "8x16")]
    sampling: [u32; 2],
    /// Palette: "ascii95" or a literal string of characters.
    #[arg(long, default_value = "ascii95")]
    palette: String,
    /// Extra denied ranges, e.g. U+0040 or U+2500-U+257F.
    #[arg(long)]
    deny: Vec<String>,
    /// Output path for the atlas JSON.
    #[arg(long)]
    out: PathBuf,
}

/// Parses a size written as `WxH`.
///
/// # Arguments
/// * `s` - the size text
///
/// # Returns
/// Width and height, or an error message.
fn parse_pair(s: &str) -> Result<[u32; 2], String> {
    let (a, b) = s
        .split_once(['x', 'X'])
        .ok_or_else(|| format!("expected WxH, got '{s}'"))?;
    let parse = |v: &str| v.trim().parse::<u32>().map_err(|e| format!("'{v}': {e}"));
    Ok([parse(a)?, parse(b)?])
}

/// Generates the atlas and prints a summary.
///
/// # Arguments
/// * `args` - command-line arguments
///
/// # Returns
/// An error if the font can't be read or used.
fn run(args: Args) -> anyhow::Result<()> {
    let font_bytes =
        std::fs::read(&args.font).with_context(|| format!("reading {}", args.font.display()))?;
    let extra = args
        .deny
        .iter()
        .map(|s| parse_range(s))
        .collect::<Result<Vec<_>, _>>()?;
    let params = GenParams {
        font_bytes,
        font_size_px: args.size_px,
        cell_px: args.cell,
        baseline_px: args.baseline,
        sampling: args.sampling,
        palette: Palette::parse(&args.palette)?,
        deny: DenyList::new(&extra),
    };
    let atlas = generate(&params)?;
    std::fs::write(&args.out, atlas.to_json())
        .with_context(|| format!("writing {}", args.out.display()))?;

    println!("font:      {} ({})", atlas.font_name, args.font.display());
    println!("size:      {} px", atlas.font_size_px);
    println!(
        "cell:      {}x{} px, baseline {}",
        atlas.cell_px[0], atlas.cell_px[1], atlas.baseline_px
    );
    println!("sampling:  {}x{}", atlas.sampling[0], atlas.sampling[1]);
    println!("glyphs:    {}", atlas.glyphs.len());
    for g in &atlas.glyphs {
        println!("  {:?} ink={:.3}", g.ch, g.ink);
    }
    println!("wrote {}", args.out.display());
    Ok(())
}

fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("glyphgen: {e:#}");
            ExitCode::FAILURE
        }
    }
}
