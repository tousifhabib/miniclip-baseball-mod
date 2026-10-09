//! Reads an extracted folder back with the format's own types and checks
//! that everything one file says about another is true.

mod checker;

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Result, ensure};
use bb_engine::library::read_json;
use bb_format::{FORMAT_VERSION, Manifest};
use clap::Parser;

use checker::Checker;

#[derive(Parser)]
#[command(about = "Checks that an extracted folder is complete and consistent")]
struct Args {
    /// The folder that holds the art.
    dir: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let manifest: Manifest = read_json(&args.dir.join("manifest.json"))?;
    ensure!(
        manifest.format_version == FORMAT_VERSION,
        "the folder is format version {}, this tool reads version {FORMAT_VERSION}",
        manifest.format_version
    );

    let mut checker = Checker {
        dir: args.dir,
        manifest,
        glyph_counts: BTreeMap::new(),
        files: 1,
        references: 0,
        problems: Vec::new(),
    };
    checker.run()?;

    println!(
        "Read {} files and followed {} references.",
        checker.files, checker.references
    );
    if checker.problems.is_empty() {
        println!("No problems.");
        return Ok(());
    }
    println!("Problems ({}):", checker.problems.len());
    for problem in &checker.problems {
        println!("  {problem}");
    }
    std::process::exit(1);
}
