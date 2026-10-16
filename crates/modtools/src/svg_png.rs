//! Draws an SVG to a PNG of a given width, or to a Windows icon that holds a
//! picture of each of several widths. Used to make the app's icon in each
//! size the system wants.

use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use clap::Parser;
use image::ExtendedColorType;
use image::codecs::ico::{IcoEncoder, IcoFrame};
use resvg::{tiny_skia, usvg};

#[derive(Parser)]
#[command(about = "Draws an SVG to a PNG, or to a Windows icon")]
struct Args {
    svg: PathBuf,
    /// The picture's width in pixels. Its height follows the drawing's shape.
    /// An icon is given this once for each picture it is to hold.
    #[arg(long, required = true)]
    width: Vec<u32>,
    /// The file to write: an icon if its name ends in `.ico`, and a PNG if
    /// not.
    #[arg(long)]
    out: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let data = fs::read(&args.svg).with_context(|| format!("reading {}", args.svg.display()))?;
    let tree = usvg::Tree::from_data(&data, &usvg::Options::default())
        .with_context(|| format!("parsing {}", args.svg.display()))?;
    let icon = args
        .out
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("ico"));
    if icon {
        return write_icon(&tree, &args.width, &args.out);
    }
    let [width] = args.width[..] else {
        bail!(
            "a PNG is drawn at one width, and was given {}",
            args.width.len()
        );
    };
    drawn(&tree, width)?
        .save_png(&args.out)
        .map_err(|error| anyhow!("writing {}: {error}", args.out.display()))
}

/// The drawing at `width` pixels across.
fn drawn(tree: &usvg::Tree, width: u32) -> Result<tiny_skia::Pixmap> {
    let size = tree.size();
    let scale = width as f32 / size.width();
    let height = (size.height() * scale).round().max(1.0) as u32;
    let mut pixmap =
        tiny_skia::Pixmap::new(width.max(1), height).context("the picture is too large")?;
    resvg::render(
        tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap)
}

/// Writes an icon holding the drawing at each of `widths`, each picture kept
/// as a PNG inside it.
fn write_icon(tree: &usvg::Tree, widths: &[u32], out: &Path) -> Result<()> {
    let frames = widths
        .iter()
        .map(|&width| {
            let pixmap = drawn(tree, width)?;
            let png = pixmap
                .encode_png()
                .map_err(|error| anyhow!("drawing the icon at {width}: {error}"))?;
            IcoFrame::with_encoded(
                png,
                pixmap.width(),
                pixmap.height(),
                ExtendedColorType::Rgba8,
            )
            .with_context(|| format!("an icon cannot be {width} pixels across"))
        })
        .collect::<Result<Vec<_>>>()?;
    let file = fs::File::create(out).with_context(|| format!("writing {}", out.display()))?;
    IcoEncoder::new(BufWriter::new(file))
        .encode_images(&frames)
        .with_context(|| format!("writing {}", out.display()))
}
