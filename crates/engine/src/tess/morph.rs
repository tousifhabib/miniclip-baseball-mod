//! Cuts a morph shape into triangles, part of the way from the shape it
//! starts as to the shape it ends as.

use anyhow::{Result, bail};
use bb_format as f;
use lyon::tessellation::{LineCap, LineJoin, StrokeOptions};

use super::outline::{Segment, lyon_path, parse_path};
use super::ramp::blend_color;
use super::{Builder, Mesh, Paint, Spread, TOLERANCE, Tessellator, lerp};
use crate::math::Matrix;

impl Tessellator {
    /// Tessellates a morph shape part of the way through its blend, where
    /// `ratio` runs from 0 (the start) to 65535 (the end).
    pub fn morph(&mut self, morph: &f::MorphShape, ratio: u16) -> Result<Mesh> {
        let t = f32::from(ratio) / 65535.0;
        let mut builder = Builder::new();
        for path in &morph.paths {
            let start = parse_path(&path.start)?;
            let end = parse_path(&path.end)?;
            if start.len() != end.len() {
                bail!(
                    "morph shape {}: a path's two ends differ in length",
                    morph.id
                );
            }
            let blended = start.iter().zip(&end).map(|(a, b)| a.blend(b, t));
            let outline = lyon_path(blended, Matrix::IDENTITY);
            match &path.style {
                f::MorphStyle::Fill {
                    start_paint,
                    end_paint,
                } => {
                    let (color, paint) = self.blended_paint(start_paint, end_paint, t)?;
                    builder.fill(&outline, color, paint)?;
                }
                f::MorphStyle::Stroke {
                    start_width,
                    end_width,
                    start_paint,
                    end_paint,
                } => {
                    let width = lerp(*start_width as f32, *end_width as f32, t);
                    let options = StrokeOptions::tolerance(TOLERANCE)
                        .with_line_width(width)
                        .with_line_cap(LineCap::Round)
                        .with_line_join(LineJoin::Round);
                    let (color, paint) = self.blended_paint(start_paint, end_paint, t)?;
                    builder.stroke(&outline, &options, color, paint)?;
                }
            }
        }
        Ok(builder.build())
    }

    /// A paint part of the way between two paints of the same kind.
    fn blended_paint(&mut self, a: &f::Paint, b: &f::Paint, t: f32) -> Result<([u8; 4], Paint)> {
        const WHITE: [u8; 4] = [255; 4];
        let blend_matrix = |a: &f::Matrix, b: &f::Matrix| {
            let mut out = [0.0; 6];
            for i in 0..6 {
                out[i] = a[i] + (b[i] - a[i]) * f64::from(t);
            }
            Matrix::from(out)
        };
        let blend_stops = |a: &[f::GradientStop], b: &[f::GradientStop]| -> Vec<f::GradientStop> {
            a.iter()
                .zip(b)
                .map(|(a, b)| f::GradientStop {
                    offset: a.offset + (b.offset - a.offset) * f64::from(t),
                    color: blend_color(a.color, b.color, t),
                })
                .collect()
        };
        // A gradient is defined on a square 1638.4 pixels wide, centred on
        // the origin, before its matrix is applied.
        const EXTENT: f32 = 819.2;
        Ok(match (a, b) {
            (f::Paint::Solid { color: a }, f::Paint::Solid { color: b }) => {
                let c = blend_color(*a, *b, t);
                ([c.r, c.g, c.b, c.a], Paint::Solid)
            }
            (
                f::Paint::LinearGradient {
                    matrix: ma,
                    stops: sa,
                },
                f::Paint::LinearGradient {
                    matrix: mb,
                    stops: sb,
                },
            ) => {
                let Some(inverse) = blend_matrix(ma, mb).inverse() else {
                    return Ok(([0; 4], Paint::Solid));
                };
                // x from -EXTENT to EXTENT becomes 0 to 1.
                let along = Matrix {
                    a: 0.5 / EXTENT,
                    b: 0.0,
                    c: 0.0,
                    d: 0.0,
                    tx: 0.5,
                    ty: 0.0,
                };
                let paint = Paint::Linear {
                    ramp: self.ramp(&blend_stops(sa, sb)),
                    matrix: along.then_inner(inverse),
                    spread: Spread::Pad,
                };
                (WHITE, paint)
            }
            (
                f::Paint::RadialGradient {
                    matrix: ma,
                    stops: sa,
                    ..
                },
                f::Paint::RadialGradient {
                    matrix: mb,
                    stops: sb,
                    ..
                },
            ) => {
                let Some(inverse) = blend_matrix(ma, mb).inverse() else {
                    return Ok(([0; 4], Paint::Solid));
                };
                let paint = Paint::Radial {
                    ramp: self.ramp(&blend_stops(sa, sb)),
                    matrix: Matrix::scale(1.0 / EXTENT, 1.0 / EXTENT).then_inner(inverse),
                    spread: Spread::Pad,
                };
                (WHITE, paint)
            }
            _ => bail!("a morph shape blends between paints the engine cannot mix"),
        })
    }
}

impl Segment {
    /// The segment part of the way to `other`, which must be the same kind.
    fn blend(&self, other: &Segment, t: f32) -> Segment {
        match (*self, *other) {
            (Segment::Move(x, y), Segment::Move(x2, y2)) => {
                Segment::Move(lerp(x, x2, t), lerp(y, y2, t))
            }
            (Segment::Line(x, y), Segment::Line(x2, y2)) => {
                Segment::Line(lerp(x, x2, t), lerp(y, y2, t))
            }
            (Segment::Quad(a, b, c, d), Segment::Quad(a2, b2, c2, d2)) => Segment::Quad(
                lerp(a, a2, t),
                lerp(b, b2, t),
                lerp(c, c2, t),
                lerp(d, d2, t),
            ),
            (same, _) => same,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_morph_segment_blends_part_way() {
        let a = Segment::Quad(0.0, 0.0, 10.0, 0.0);
        let b = Segment::Quad(0.0, 10.0, 10.0, 20.0);
        assert_eq!(a.blend(&b, 0.5), Segment::Quad(0.0, 5.0, 10.0, 10.0));
    }
}
