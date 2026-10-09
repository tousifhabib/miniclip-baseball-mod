//! Cuts an SVG shape into triangles, and works out the paint of each of
//! its fills and strokes.

mod paint;

use std::path::Path;

use anyhow::{Context, Result};
use lyon::tessellation::{LineCap, LineJoin, StrokeOptions};

use super::outline::{Segment, lyon_path};
use super::{Builder, Mesh, TOLERANCE, Tessellator};
use crate::math::Matrix;

impl Tessellator {
    /// Tessellates an SVG shape. `origin` is where the SVG's top-left corner
    /// sits in the shape's own coordinates.
    pub fn svg(&mut self, path: &Path, origin: (f32, f32)) -> Result<Mesh> {
        let data = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        let options = usvg::Options {
            // So that bitmap fills can load the images they point at.
            resources_dir: path.parent().map(Path::to_path_buf),
            ..usvg::Options::default()
        };
        let tree = usvg::Tree::from_data(&data, &options)
            .with_context(|| format!("parsing {}", path.display()))?;
        let mut builder = Builder::new();
        self.svg_group(tree.root(), origin, &mut builder)
            .with_context(|| format!("tessellating {}", path.display()))?;
        Ok(builder.build())
    }

    fn svg_group(
        &mut self,
        group: &usvg::Group,
        origin: (f32, f32),
        builder: &mut Builder,
    ) -> Result<()> {
        for node in group.children() {
            match node {
                usvg::Node::Group(group) => self.svg_group(group, origin, builder)?,
                usvg::Node::Path(path) if path.is_visible() => {
                    self.svg_path(path, origin, builder)?;
                }
                // Images and text outside a fill are not something the
                // extractor writes.
                _ => {}
            }
        }
        Ok(())
    }

    fn svg_path(
        &mut self,
        path: &usvg::Path,
        origin: (f32, f32),
        builder: &mut Builder,
    ) -> Result<()> {
        // From the path's own coordinates to the shape's.
        let to_shape =
            Matrix::translate(origin.0, origin.1).then_inner(matrix_of(path.abs_transform()));
        let Some(from_shape) = to_shape.inverse() else {
            return Ok(());
        };
        let outline = lyon_path(
            path.data().segments().map(|segment| {
                use usvg::tiny_skia_path::PathSegment as S;
                match segment {
                    S::MoveTo(p) => Segment::Move(p.x, p.y),
                    S::LineTo(p) => Segment::Line(p.x, p.y),
                    S::QuadTo(c, p) => Segment::Quad(c.x, c.y, p.x, p.y),
                    S::CubicTo(c1, c2, p) => Segment::Cubic(c1.x, c1.y, c2.x, c2.y, p.x, p.y),
                    S::Close => Segment::Close,
                }
            }),
            to_shape,
        );

        if let Some(fill) = path.fill() {
            let (color, paint) = self.svg_paint(fill.paint(), fill.opacity().get(), from_shape)?;
            builder.fill(&outline, color, paint)?;
        }
        if let Some(stroke) = path.stroke() {
            let (color, paint) =
                self.svg_paint(stroke.paint(), stroke.opacity().get(), from_shape)?;
            let options = StrokeOptions::tolerance(TOLERANCE)
                .with_line_width(stroke.width().get())
                .with_line_cap(match stroke.linecap() {
                    usvg::LineCap::Butt => LineCap::Butt,
                    usvg::LineCap::Round => LineCap::Round,
                    usvg::LineCap::Square => LineCap::Square,
                })
                .with_line_join(match stroke.linejoin() {
                    usvg::LineJoin::Miter | usvg::LineJoin::MiterClip => LineJoin::Miter,
                    usvg::LineJoin::Round => LineJoin::Round,
                    usvg::LineJoin::Bevel => LineJoin::Bevel,
                })
                .with_miter_limit(stroke.miterlimit().get().max(1.0));
            builder.stroke(&outline, &options, color, paint)?;
        }
        Ok(())
    }
}

/// One of the SVG reader's transforms as one of the engine's.
fn matrix_of(transform: usvg::Transform) -> Matrix {
    Matrix {
        a: transform.sx,
        b: transform.ky,
        c: transform.kx,
        d: transform.sy,
        tx: transform.tx,
        ty: transform.ty,
    }
}
