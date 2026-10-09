//! Turns vector art into triangles: SVG shapes, text and morph shapes.
//!
//! What a mesh is made of is here, with the builder that fills one. Each
//! kind of art is cut up in a file of its own: `svg`, `text` and `morph`.
//! They share `outline`, for the lines round a shape, and `ramp`, for the
//! colours along a gradient.

mod morph;
mod outline;
mod ramp;
mod svg;
mod text;

use std::collections::HashMap;
use std::ops::Range;

use anyhow::{Result, anyhow};
use bytemuck::{Pod, Zeroable};
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillRule, FillTessellator, FillVertex, StrokeOptions,
    StrokeTessellator, StrokeVertex, VertexBuffers,
};

use crate::math::Matrix;

/// How far, in pixels at normal size, a flattened curve may stray from the
/// true curve.
const TOLERANCE: f32 = 0.02;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    /// For a stroke, the point on the centre line this vertex belongs to.
    pub position: [f32; 2],
    /// For a stroke, the direction to move out from the centre line; one
    /// unit of it is half the stroke's width. Zero for fills.
    pub normal: [f32; 2],
    /// Half the stroke's width. Zero for fills.
    pub half_width: f32,
    /// Used by solid paint. Not multiplied by alpha.
    pub color: [u8; 4],
}

/// What colours a run of triangles. Each matrix maps the shape's own
/// coordinates into the paint's.
#[derive(Clone, Debug, PartialEq)]
pub enum Paint {
    /// Every vertex carries its own colour.
    Solid,
    /// The colour is ramp `ramp` at position x.
    Linear {
        ramp: usize,
        matrix: Matrix,
        spread: Spread,
    },
    /// The colour is ramp `ramp` at the distance from the origin.
    Radial {
        ramp: usize,
        matrix: Matrix,
        spread: Spread,
    },
    /// The colour is image `image` at (x, y), each from 0 to 1.
    Image {
        image: usize,
        matrix: Matrix,
        smooth: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spread {
    Pad,
    Reflect,
    Repeat,
}

#[derive(Clone, Debug)]
pub struct Draw {
    pub indices: Range<u32>,
    pub paint: Paint,
}

#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    /// In drawing order.
    pub draws: Vec<Draw>,
}

impl Mesh {
    /// Whether any triangle covers the point, which is in the mesh's own
    /// coordinates. Strokes count at the width they were made with.
    pub fn contains(&self, x: f32, y: f32) -> bool {
        let corner = |index: u32| {
            let vertex = &self.vertices[index as usize];
            (
                vertex.position[0] + vertex.normal[0] * vertex.half_width,
                vertex.position[1] + vertex.normal[1] * vertex.half_width,
            )
        };
        self.indices.as_chunks::<3>().0.iter().any(|triangle| {
            let [a, b, c] = triangle.map(corner);
            // The point is inside when it is on the same side of all three
            // edges.
            let side =
                |p: (f32, f32), q: (f32, f32)| (q.0 - p.0) * (y - p.1) - (q.1 - p.1) * (x - p.0);
            let (ab, bc, ca) = (side(a, b), side(b, c), side(c, a));
            (ab >= 0.0 && bc >= 0.0 && ca >= 0.0) || (ab <= 0.0 && bc <= 0.0 && ca <= 0.0)
        })
    }
}

/// 256 colours along a gradient, not multiplied by alpha.
pub type Ramp = [[u8; 4]; 256];

/// Pixels in RGBA order, not multiplied by alpha.
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Builds meshes, and collects the gradient ramps and images they refer to.
#[derive(Default)]
pub struct Tessellator {
    pub ramps: Vec<Ramp>,
    pub images: Vec<Image>,
    ramp_rows: HashMap<Ramp, usize>,
    image_slots: HashMap<u64, usize>,
}

/// A mesh under construction.
struct Builder {
    buffers: VertexBuffers<Vertex, u32>,
    draws: Vec<Draw>,
}

impl Builder {
    fn new() -> Builder {
        Builder {
            buffers: VertexBuffers::new(),
            draws: Vec::new(),
        }
    }

    fn fill(&mut self, path: &lyon::path::Path, color: [u8; 4], paint: Paint) -> Result<()> {
        let start = self.buffers.indices.len() as u32;
        let options = FillOptions::tolerance(TOLERANCE).with_fill_rule(FillRule::EvenOdd);
        FillTessellator::new()
            .tessellate_path(
                path,
                &options,
                &mut BuffersBuilder::new(&mut self.buffers, |vertex: FillVertex<'_>| Vertex {
                    position: vertex.position().to_array(),
                    normal: [0.0, 0.0],
                    half_width: 0.0,
                    color,
                }),
            )
            .map_err(|error| anyhow!("tessellating a fill: {error:?}"))?;
        self.finish_draw(start, paint);
        Ok(())
    }

    fn stroke(
        &mut self,
        path: &lyon::path::Path,
        options: &StrokeOptions,
        color: [u8; 4],
        paint: Paint,
    ) -> Result<()> {
        let start = self.buffers.indices.len() as u32;
        let half_width = options.line_width / 2.0;
        StrokeTessellator::new()
            .tessellate_path(
                path,
                options,
                &mut BuffersBuilder::new(&mut self.buffers, |vertex: StrokeVertex<'_, '_>| {
                    Vertex {
                        position: vertex.position_on_path().to_array(),
                        normal: vertex.normal().to_array(),
                        half_width,
                        color,
                    }
                }),
            )
            .map_err(|error| anyhow!("tessellating a stroke: {error:?}"))?;
        self.finish_draw(start, paint);
        Ok(())
    }

    fn finish_draw(&mut self, start: u32, paint: Paint) {
        let end = self.buffers.indices.len() as u32;
        if end == start {
            return;
        }
        // Solid paint reads its colour from the vertices, so neighbouring
        // solid runs can go out in one draw.
        if paint == Paint::Solid
            && let Some(last) = self.draws.last_mut()
            && last.paint == Paint::Solid
            && last.indices.end == start
        {
            last.indices.end = end;
            return;
        }
        self.draws.push(Draw {
            indices: start..end,
            paint,
        });
    }

    fn build(self) -> Mesh {
        Mesh {
            vertices: self.buffers.vertices,
            indices: self.buffers.indices,
            draws: self.draws,
        }
    }
}

/// The number part of the way from `a` to `b`: all the way at a `t` of 1.
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::outline::{lyon_path, parse_path};
    use super::*;

    /// The total area of a mesh's triangles.
    fn area(mesh: &Mesh) -> f32 {
        mesh.indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|triangle| {
                let [a, b, c] = [0, 1, 2].map(|i| mesh.vertices[triangle[i] as usize].position);
                ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])).abs() / 2.0
            })
            .sum()
    }

    fn filled(data: &str) -> Mesh {
        let mut builder = Builder::new();
        let outline = lyon_path(parse_path(data).unwrap().into_iter(), Matrix::IDENTITY);
        builder.fill(&outline, [255; 4], Paint::Solid).unwrap();
        builder.build()
    }

    #[test]
    fn a_square_fills_its_whole_area() {
        let mesh = filled("M0 0 L10 0 L10 10 L0 10 Z");
        assert!((area(&mesh) - 100.0).abs() < 0.01);
    }

    #[test]
    fn an_inner_outline_cuts_a_hole() {
        let mesh = filled("M0 0 L10 0 L10 10 L0 10 Z M2 2 L8 2 L8 8 L2 8 Z");
        assert!((area(&mesh) - 64.0).abs() < 0.01);
    }

    #[test]
    fn a_mesh_knows_which_points_it_covers() {
        let mesh = filled("M0 0 L10 0 L10 10 L0 10 Z M2 2 L8 2 L8 8 L2 8 Z");
        assert!(mesh.contains(1.0, 1.0));
        // Inside the hole.
        assert!(!mesh.contains(5.0, 5.0));
        assert!(!mesh.contains(11.0, 5.0));
    }

    #[test]
    fn neighbouring_solid_fills_share_one_draw() {
        let mut builder = Builder::new();
        for data in ["M0 0 L1 0 L1 1 Z", "M5 5 L6 5 L6 6 Z"] {
            let outline = lyon_path(parse_path(data).unwrap().into_iter(), Matrix::IDENTITY);
            builder.fill(&outline, [255; 4], Paint::Solid).unwrap();
        }
        let mesh = builder.build();
        assert_eq!(mesh.draws.len(), 1);
        assert_eq!(mesh.draws[0].indices, 0..6);
    }

    #[test]
    fn a_stroke_keeps_its_centre_line_and_points_outwards() {
        let mut builder = Builder::new();
        let outline = lyon_path(
            parse_path("M0 0 L10 0").unwrap().into_iter(),
            Matrix::IDENTITY,
        );
        let options = StrokeOptions::tolerance(TOLERANCE).with_line_width(4.0);
        builder
            .stroke(&outline, &options, [255; 4], Paint::Solid)
            .unwrap();
        let mesh = builder.build();
        assert!(!mesh.indices.is_empty());
        for vertex in &mesh.vertices {
            // Every vertex sits on the line, and knows which way is out.
            assert!(vertex.position[1].abs() < 1e-4);
            assert!((vertex.normal[1].abs() - 1.0).abs() < 1e-4);
            assert_eq!(vertex.half_width, 2.0);
        }
    }
}
