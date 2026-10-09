//! Gathering triangles into a mesh, with a draw for each run of them
//! that shares a paint.

use anyhow::{Result, anyhow};
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillRule, FillTessellator, FillVertex, StrokeOptions,
    StrokeTessellator, StrokeVertex, VertexBuffers,
};

use super::{Draw, Mesh, Paint, Vertex};

/// How far, in pixels at normal size, a flattened curve may stray from the
/// true curve.
pub(super) const TOLERANCE: f32 = 0.02;

/// A mesh under construction.
pub(super) struct Builder {
    buffers: VertexBuffers<Vertex, u32>,
    draws: Vec<Draw>,
}

impl Builder {
    pub(super) fn new() -> Builder {
        Builder {
            buffers: VertexBuffers::new(),
            draws: Vec::new(),
        }
    }

    pub(super) fn fill(
        &mut self,
        path: &lyon::path::Path,
        color: [u8; 4],
        paint: Paint,
    ) -> Result<()> {
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

    pub(super) fn stroke(
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

    pub(super) fn build(self) -> Mesh {
        Mesh {
            vertices: self.buffers.vertices,
            indices: self.buffers.indices,
            draws: self.draws,
        }
    }
}
