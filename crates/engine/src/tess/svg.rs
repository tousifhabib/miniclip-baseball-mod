//! Cuts an SVG shape into triangles, and works out the paint of each of
//! its fills and strokes.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use bb_format as f;
use lyon::tessellation::{LineCap, LineJoin, StrokeOptions};

use super::outline::{Segment, lyon_path};
use super::{Builder, Image, Mesh, Paint, Spread, TOLERANCE, Tessellator};
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

    /// Converts an SVG paint. Returns the vertex colour and the paint.
    fn svg_paint(
        &mut self,
        paint: &usvg::Paint,
        opacity: f32,
        from_shape: Matrix,
    ) -> Result<([u8; 4], Paint)> {
        const WHITE: [u8; 4] = [255; 4];
        let alpha = |opacity: f32| (opacity * 255.0).round().clamp(0.0, 255.0) as u8;
        let spread = |method: usvg::SpreadMethod| match method {
            usvg::SpreadMethod::Pad => Spread::Pad,
            usvg::SpreadMethod::Reflect => Spread::Reflect,
            usvg::SpreadMethod::Repeat => Spread::Repeat,
        };
        let stops = |stops: &[usvg::Stop]| -> Vec<f::GradientStop> {
            stops
                .iter()
                .map(|stop| f::GradientStop {
                    offset: f64::from(stop.offset().get()),
                    color: f::Color {
                        r: stop.color().red,
                        g: stop.color().green,
                        b: stop.color().blue,
                        a: alpha(stop.opacity().get() * opacity),
                    },
                })
                .collect()
        };
        // From the shape's coordinates into a gradient's or pattern's own.
        let into = |transform: usvg::Transform| {
            matrix_of(transform)
                .inverse()
                .map(|inverse| inverse.then_inner(from_shape))
        };

        Ok(match paint {
            usvg::Paint::Color(color) => (
                [color.red, color.green, color.blue, alpha(opacity)],
                Paint::Solid,
            ),
            usvg::Paint::LinearGradient(gradient) => {
                let Some(matrix) = into(gradient.transform()) else {
                    return Ok(([0; 4], Paint::Solid));
                };
                let (dx, dy) = (gradient.x2() - gradient.x1(), gradient.y2() - gradient.y1());
                let length_squared = (dx * dx + dy * dy).max(1e-12);
                // Position along the line from the first point to the second.
                let along = Matrix {
                    a: dx / length_squared,
                    b: 0.0,
                    c: dy / length_squared,
                    d: 0.0,
                    tx: -(dx * gradient.x1() + dy * gradient.y1()) / length_squared,
                    ty: 0.0,
                };
                let paint = Paint::Linear {
                    ramp: self.ramp(&stops(gradient.stops())),
                    matrix: along.then_inner(matrix),
                    spread: spread(gradient.spread_method()),
                };
                (WHITE, paint)
            }
            usvg::Paint::RadialGradient(gradient) => {
                let Some(matrix) = into(gradient.transform()) else {
                    return Ok(([0; 4], Paint::Solid));
                };
                let r = gradient.r().get().max(1e-6);
                // Distance from the centre, in radii.
                let from_centre = Matrix::scale(1.0 / r, 1.0 / r)
                    .then_inner(Matrix::translate(-gradient.cx(), -gradient.cy()));
                let paint = Paint::Radial {
                    ramp: self.ramp(&stops(gradient.stops())),
                    matrix: from_centre.then_inner(matrix),
                    spread: spread(gradient.spread_method()),
                };
                (WHITE, paint)
            }
            usvg::Paint::Pattern(pattern) => {
                let Some(image) = first_image(pattern.root()) else {
                    bail!("a pattern fill has no image in it");
                };
                let Some(matrix) = into(pattern.transform()) else {
                    return Ok(([0; 4], Paint::Solid));
                };
                let size = image.size();
                let to_unit = Matrix::scale(1.0 / size.width(), 1.0 / size.height());
                let smooth = !matches!(
                    image.rendering_mode(),
                    usvg::ImageRendering::OptimizeSpeed
                        | usvg::ImageRendering::CrispEdges
                        | usvg::ImageRendering::Pixelated
                );
                let paint = Paint::Image {
                    image: self.image(image.kind())?,
                    matrix: to_unit.then_inner(matrix),
                    smooth,
                };
                ([255, 255, 255, alpha(opacity)], paint)
            }
        })
    }

    /// The slot of this image, decoding and adding it if it is new.
    fn image(&mut self, kind: &usvg::ImageKind) -> Result<usize> {
        let encoded: &Arc<Vec<u8>> = match kind {
            usvg::ImageKind::PNG(data)
            | usvg::ImageKind::JPEG(data)
            | usvg::ImageKind::GIF(data)
            | usvg::ImageKind::WEBP(data) => data,
            usvg::ImageKind::SVG(_) => bail!("an SVG inside a pattern fill is not supported"),
        };
        let mut hasher = DefaultHasher::new();
        encoded.hash(&mut hasher);
        let key = hasher.finish();
        if let Some(&slot) = self.image_slots.get(&key) {
            return Ok(slot);
        }
        let decoded = image::load_from_memory(encoded)
            .context("decoding an image used as a fill")?
            .to_rgba8();
        self.images.push(Image {
            width: decoded.width(),
            height: decoded.height(),
            rgba: decoded.into_raw(),
        });
        self.image_slots.insert(key, self.images.len() - 1);
        Ok(self.images.len() - 1)
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

fn first_image(group: &usvg::Group) -> Option<&usvg::Image> {
    group.children().iter().find_map(|node| match node {
        usvg::Node::Image(image) => Some(&**image),
        usvg::Node::Group(group) => first_image(group),
        _ => None,
    })
}
