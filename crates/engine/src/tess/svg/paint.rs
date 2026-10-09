//! What an SVG shape is filled or drawn with, made into a paint: a plain
//! colour, a gradient, or a picture.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use bb_format as f;

use super::matrix_of;
use crate::math::Matrix;
use crate::tess::{Image, Paint, Spread, Tessellator};

/// What a paint comes to when it has been squashed flat and cannot be placed
/// in the shape: nothing that can be seen.
const UNSEEN: ([u8; 4], Paint) = ([0; 4], Paint::Solid);

/// What the corners carry when the paint brings colours of its own: white,
/// which leaves those colours as they are.
const WHITE: [u8; 4] = [255; 4];

impl Tessellator {
    /// Converts an SVG paint. Returns the vertex colour and the paint.
    pub(super) fn svg_paint(
        &mut self,
        paint: &usvg::Paint,
        opacity: f32,
        from_shape: Matrix,
    ) -> Result<([u8; 4], Paint)> {
        Ok(match paint {
            usvg::Paint::Color(color) => (
                [color.red, color.green, color.blue, alpha(opacity)],
                Paint::Solid,
            ),
            usvg::Paint::LinearGradient(gradient) => {
                self.linear_gradient(gradient, opacity, from_shape)
            }
            usvg::Paint::RadialGradient(gradient) => {
                self.radial_gradient(gradient, opacity, from_shape)
            }
            usvg::Paint::Pattern(pattern) => self.pattern(pattern, opacity, from_shape)?,
        })
    }

    /// The paint of a gradient that runs along a line, from its first point
    /// to its second.
    fn linear_gradient(
        &mut self,
        gradient: &usvg::LinearGradient,
        opacity: f32,
        from_shape: Matrix,
    ) -> ([u8; 4], Paint) {
        let Some(matrix) = into_paint(gradient.transform(), from_shape) else {
            return UNSEEN;
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
            ramp: self.ramp(&stops(gradient.stops(), opacity)),
            matrix: along.then_inner(matrix),
            spread: spread(gradient.spread_method()),
        };
        (WHITE, paint)
    }

    /// The paint of a gradient that runs outwards from a centre.
    fn radial_gradient(
        &mut self,
        gradient: &usvg::RadialGradient,
        opacity: f32,
        from_shape: Matrix,
    ) -> ([u8; 4], Paint) {
        let Some(matrix) = into_paint(gradient.transform(), from_shape) else {
            return UNSEEN;
        };
        let r = gradient.r().get().max(1e-6);
        // Distance from the centre, in radii.
        let from_centre = Matrix::scale(1.0 / r, 1.0 / r)
            .then_inner(Matrix::translate(-gradient.cx(), -gradient.cy()));
        let paint = Paint::Radial {
            ramp: self.ramp(&stops(gradient.stops(), opacity)),
            matrix: from_centre.then_inner(matrix),
            spread: spread(gradient.spread_method()),
        };
        (WHITE, paint)
    }

    /// The paint of a pattern, which is how an SVG says a shape is filled
    /// with an image.
    fn pattern(
        &mut self,
        pattern: &usvg::Pattern,
        opacity: f32,
        from_shape: Matrix,
    ) -> Result<([u8; 4], Paint)> {
        let Some(image) = first_image(pattern.root()) else {
            bail!("a pattern fill has no image in it");
        };
        let Some(matrix) = into_paint(pattern.transform(), from_shape) else {
            return Ok(UNSEEN);
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
        Ok(([255, 255, 255, alpha(opacity)], paint))
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

/// How solid, from 0 to 255, for an opacity from 0 to 1.
fn alpha(opacity: f32) -> u8 {
    (opacity * 255.0).round().clamp(0.0, 255.0) as u8
}

/// How an SVG's gradient goes on past its ends, in the engine's words.
fn spread(method: usvg::SpreadMethod) -> Spread {
    match method {
        usvg::SpreadMethod::Pad => Spread::Pad,
        usvg::SpreadMethod::Reflect => Spread::Reflect,
        usvg::SpreadMethod::Repeat => Spread::Repeat,
    }
}

/// An SVG gradient's stops as a ramp is made from them, each made less
/// solid by the `opacity` of the fill or stroke they paint.
fn stops(stops: &[usvg::Stop], opacity: f32) -> Vec<f::GradientStop> {
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
}

/// From the shape's coordinates into a gradient's or pattern's own, given
/// the transform that places it. `None` if that transform cannot be undone.
fn into_paint(transform: usvg::Transform, from_shape: Matrix) -> Option<Matrix> {
    matrix_of(transform)
        .inverse()
        .map(|inverse| inverse.then_inner(from_shape))
}

fn first_image(group: &usvg::Group) -> Option<&usvg::Image> {
    group.children().iter().find_map(|node| match node {
        usvg::Node::Image(image) => Some(&**image),
        usvg::Node::Group(group) => first_image(group),
        _ => None,
    })
}
