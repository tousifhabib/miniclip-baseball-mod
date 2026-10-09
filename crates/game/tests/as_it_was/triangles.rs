//! The triangles the art is cut into, written down as the games are.
//!
//! Every shape, every piece of fixed text, every text field and every morph
//! shape is cut up as it would be to draw it or to point at it, with no
//! graphics card, and a sum is taken of what comes out: each corner, the
//! order they are joined in, what paints each run of triangles, and the
//! gradients and images the paints use. This is the net under changes to the
//! code that does the cutting.

use bb_engine::library::Library;
use bb_engine::math::Matrix;
use bb_engine::tess::{Mesh, Paint, Spread, Tessellator};
use bb_format::SymbolInfo;

use crate::sums::{Sink, Sum};

/// How far through its change a morph shape is cut up: at each end, and at
/// three places between.
const RATIOS: [u16; 5] = [0, 16_384, 32_768, 49_152, 65_535];
/// What a text field is made to say, besides what it starts with: nothing,
/// and something with a space, capitals and a figure in it.
const SAID: [&str; 2] = ["", "Red Sox 9"];

fn matrix(matrix: Matrix, into: &mut Sum) {
    let Matrix { a, b, c, d, tx, ty } = matrix;
    for float in [a, b, c, d, tx, ty] {
        into.float("", float);
    }
}

fn bytes(bytes: &[u8], into: &mut Sum) {
    into.number("", bytes.len() as u64);
    for eight in bytes.chunks(8) {
        let mut word = [0; 8];
        word[..eight.len()].copy_from_slice(eight);
        into.number("", u64::from_le_bytes(word));
    }
}

fn mesh(mesh: &Mesh, into: &mut Sum) {
    into.number("", mesh.vertices.len() as u64);
    for vertex in &mesh.vertices {
        for float in [vertex.position, vertex.normal].as_flattened() {
            into.float("", *float);
        }
        into.float("", vertex.half_width);
        into.number("", u64::from(u32::from_le_bytes(vertex.color)));
    }
    into.number("", mesh.indices.len() as u64);
    for &index in &mesh.indices {
        into.number("", u64::from(index));
    }
    into.number("", mesh.draws.len() as u64);
    for draw in &mesh.draws {
        into.number("", u64::from(draw.indices.start));
        into.number("", u64::from(draw.indices.end));
        let spread_of = |spread: Spread| match spread {
            Spread::Pad => 0,
            Spread::Reflect => 1,
            Spread::Repeat => 2,
        };
        match &draw.paint {
            Paint::Solid => into.number("", 0),
            Paint::Linear {
                ramp,
                matrix: placed,
                spread,
            } => {
                into.number("", 1);
                into.number("", *ramp as u64);
                matrix(*placed, into);
                into.number("", spread_of(*spread));
            }
            Paint::Radial {
                ramp,
                matrix: placed,
                spread,
            } => {
                into.number("", 2);
                into.number("", *ramp as u64);
                matrix(*placed, into);
                into.number("", spread_of(*spread));
            }
            Paint::Image {
                image,
                matrix: placed,
                smooth,
            } => {
                into.number("", 3);
                into.number("", *image as u64);
                matrix(*placed, into);
                into.flag("", *smooth);
            }
        }
    }
}

/// The sum of everything one symbol is cut into. Each is cut by a tessellator
/// of its own, so that its sum is of that symbol and nothing before it.
fn cut(library: &Library, id: u16) -> Option<(String, String)> {
    let symbol = &library.manifest.symbols[&id];
    let mut tessellator = Tessellator::default();
    let mut sum = Sum::new();
    let take = |cut: anyhow::Result<Mesh>, sum: &mut Sum| match cut {
        Ok(cut) => mesh(&cut, sum),
        // What cannot be cut is written down as that, with why.
        Err(error) => sum.words("", &format!("{error:#}")),
    };
    let kind = match &symbol.info {
        SymbolInfo::Shape { bounds } => {
            let origin = (bounds.x_min as f32, bounds.y_min as f32);
            let file = library.dir.join(&symbol.file);
            take(tessellator.svg(&file, origin), &mut sum);
            "shape"
        }
        SymbolInfo::MorphShape => {
            let morph = library.morphs.get(&id)?;
            for ratio in RATIOS {
                take(tessellator.morph(morph, ratio), &mut sum);
            }
            "morph"
        }
        SymbolInfo::Text => {
            take(tessellator.text(library.texts.get(&id)?, library), &mut sum);
            "text"
        }
        SymbolInfo::EditText => {
            let field = library.edit_texts.get(&id)?;
            take(tessellator.edit_text(field, None, library), &mut sum);
            for said in SAID {
                take(tessellator.edit_text(field, Some(said), library), &mut sum);
            }
            "field"
        }
        _ => return None,
    };
    sum.number("", tessellator.ramps.len() as u64);
    for ramp in &tessellator.ramps {
        bytes(ramp.as_flattened(), &mut sum);
    }
    sum.number("", tessellator.images.len() as u64);
    for image in &tessellator.images {
        sum.number("", u64::from(image.width));
        sum.number("", u64::from(image.height));
        bytes(&image.rgba, &mut sum);
    }
    Some((format!("{kind} {id}"), sum.written()))
}

/// Every symbol that is cut into triangles, by its kind and number, with
/// the sum of what it is cut into.
pub fn sums(library: &Library) -> Vec<(String, String)> {
    let mut ids: Vec<u16> = library.manifest.symbols.keys().copied().collect();
    ids.sort_unstable();
    let workers = std::thread::available_parallelism().map_or(4, usize::from);
    let share = ids.len().div_ceil(workers).max(1);
    std::thread::scope(|scope| {
        let shares: Vec<_> = ids
            .chunks(share)
            .map(|ids| {
                scope.spawn(move || {
                    ids.iter()
                        .filter_map(|&id| cut(library, id))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        shares
            .into_iter()
            .flat_map(|share| share.join().expect("the cutting"))
            .collect()
    })
}
