//! The lines round a shape: the path data the extractor writes, read into
//! segments, and segments made into a path for lyon to cut up.

use anyhow::{Context, Result, bail};
use lyon::math::point;

use crate::math::Matrix;

/// One command of an outline, with absolute coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Segment {
    Move(f32, f32),
    Line(f32, f32),
    Quad(f32, f32, f32, f32),
    Cubic(f32, f32, f32, f32, f32, f32),
    Close,
}

/// Parses the path data the extractor writes: `M`, `L`, `Q` and `Z` with
/// absolute coordinates.
pub(super) fn parse_path(data: &str) -> Result<Vec<Segment>> {
    let mut segments = Vec::new();
    let mut rest = data.trim_start();
    while let Some(command) = rest.chars().next() {
        rest = &rest[command.len_utf8()..];
        let mut number = || -> Result<f32> {
            rest = rest.trim_start();
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            let (text, tail) = rest.split_at(end);
            rest = tail;
            text.parse()
                .with_context(|| format!("bad number {text:?} in path data"))
        };
        segments.push(match command {
            'M' => Segment::Move(number()?, number()?),
            'L' => Segment::Line(number()?, number()?),
            'Q' => Segment::Quad(number()?, number()?, number()?, number()?),
            'Z' => Segment::Close,
            other => bail!("unexpected {other:?} in path data"),
        });
        rest = rest.trim_start();
    }
    Ok(segments)
}

/// Builds a lyon path from segments, moving every point through `matrix`.
pub(super) fn lyon_path(
    segments: impl Iterator<Item = Segment>,
    matrix: Matrix,
) -> lyon::path::Path {
    let at = |x: f32, y: f32| {
        let (x, y) = matrix.apply(x, y);
        point(x, y)
    };
    let mut builder = lyon::path::Path::builder();
    let mut open = false;
    // Where the current outline began, for a line that follows a close.
    let mut start = point(0.0, 0.0);
    let ensure_open = |builder: &mut lyon::path::Builder, open: &mut bool, start| {
        if !*open {
            builder.begin(start);
            *open = true;
        }
    };
    for segment in segments {
        match segment {
            Segment::Move(x, y) => {
                if open {
                    builder.end(false);
                }
                start = at(x, y);
                builder.begin(start);
                open = true;
            }
            Segment::Line(x, y) => {
                ensure_open(&mut builder, &mut open, start);
                builder.line_to(at(x, y));
            }
            Segment::Quad(cx, cy, x, y) => {
                ensure_open(&mut builder, &mut open, start);
                builder.quadratic_bezier_to(at(cx, cy), at(x, y));
            }
            Segment::Cubic(c1x, c1y, c2x, c2y, x, y) => {
                ensure_open(&mut builder, &mut open, start);
                builder.cubic_bezier_to(at(c1x, c1y), at(c2x, c2y), at(x, y));
            }
            Segment::Close => {
                if open {
                    builder.end(true);
                    open = false;
                }
            }
        }
    }
    if open {
        builder.end(false);
    }
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_data_parses_into_segments() {
        assert_eq!(
            parse_path("M0 0 L1.5 0 Q2 -1 3 0 Z").unwrap(),
            [
                Segment::Move(0.0, 0.0),
                Segment::Line(1.5, 0.0),
                Segment::Quad(2.0, -1.0, 3.0, 0.0),
                Segment::Close,
            ]
        );
        assert!(parse_path("M0 0 C1 1 2 2 3 3").is_err());
    }
}
