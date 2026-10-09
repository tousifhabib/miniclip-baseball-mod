//! Sorts a frame's commands into what the graphics card is to do: layers
//! of draws, the frame itself first, and what each draw is told.

mod masking;
mod prepare;

use bytemuck::Zeroable;

use super::SLOT;
use super::pipelines::{Item, Mode};
use crate::display::Bounds;
use crate::math::{ColorTransform, Matrix};
use crate::meshes::MeshKey;
use crate::tess::Mesh;
use masking::Masking;
use prepare::{item_for, layer_area};

/// The widest blur the shader will sample, in pixels.
const MAX_BLUR: f32 = 127.0;

/// Which texture a draw reads its colours from.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Texture {
    Ramps,
    Image {
        slot: usize,
        smooth: bool,
    },
    /// The finished picture of a layer.
    Layer(usize),
}

/// One draw: a run of a mesh's triangles, with what it is told and how it
/// treats the masks.
pub(super) struct Step {
    pub mode: Mode,
    pub stencil: u32,
    pub key: MeshKey,
    pub draw: usize,
    pub item: usize,
    pub texture: Texture,
}

/// One picture being built: the frame itself, or a blurred object.
pub(super) struct Layer {
    pub steps: Vec<Step>,
    /// Where the layer sits in the frame, in pixels.
    pub origin: (i32, i32),
    pub size: (u32, u32),
    /// One entry per blur to run, in order: the item holding its settings.
    pub blurs: Vec<usize>,
    /// The textures it draws to. `None` for the frame, and for a layer that
    /// is entirely out of view.
    pub target: Option<usize>,
}

/// What a frame comes to once its commands are sorted out.
pub(super) struct Plan {
    /// The pictures to draw. The first is the frame itself, and each of the
    /// others is drawn into one that comes before it.
    pub layers: Vec<Layer>,
    /// What each draw and each blur is told, a slot to each.
    pub items: Vec<u8>,
    /// The layers being drawn into, outermost first, each with the mask
    /// state to go back to when it ends.
    open: Vec<(usize, Masking)>,
    /// The layer that draws go into now.
    current: usize,
    masking: Masking,
}

impl Plan {
    /// A plan for a frame of `size` pixels with nothing drawn in it yet.
    fn new(size: (u32, u32)) -> Plan {
        Plan {
            layers: vec![Layer {
                steps: Vec::new(),
                origin: (0, 0),
                size,
                blurs: Vec::new(),
                target: None,
            }],
            items: Vec::new(),
            open: Vec::new(),
            current: 0,
            masking: Masking::NONE,
        }
    }

    /// Adds what one draw or blur is told, and returns its slot.
    fn push_item(&mut self, item: Item) -> usize {
        let slot = self.items.len() / SLOT;
        self.items.extend_from_slice(bytemuck::bytes_of(&item));
        self.items.resize((slot + 1) * SLOT, 0);
        slot
    }

    /// Adds a draw to the layer being drawn into, under the masks in force.
    fn push_step(&mut self, key: MeshKey, draw: usize, item: usize, texture: Texture) {
        self.layers[self.current].steps.push(Step {
            mode: self.masking.mode,
            stencil: self.masking.stencil,
            key,
            draw,
            item,
            texture,
        });
    }

    /// Whether the layer being drawn into is out of view altogether, so
    /// that nothing drawn into it could be seen.
    fn out_of_view(&self) -> bool {
        self.layers[self.current].size == (0, 0)
    }

    /// Starts a layer for an object that covers `bounds` and is to be
    /// blurred. Draws go into it until it ends.
    fn begin_blur(&mut self, blur_x: f32, blur_y: f32, passes: u8, bounds: Bounds) {
        let (blur_x, blur_y) = (blur_x.min(MAX_BLUR), blur_y.min(MAX_BLUR));
        // Each pass spreads the picture by half the box's width.
        let reach = (blur_x.max(blur_y) / 2.0 * f32::from(passes)).ceil() + 1.0;
        let parent = &self.layers[self.current];
        let area = layer_area(bounds, reach, parent.origin, parent.size);
        let (origin, size) = area.unwrap_or(((0, 0), (0, 0)));
        let blurs = match area {
            Some(_) => self.push_blurs(blur_x, blur_y, passes, size),
            None => Vec::new(),
        };
        self.open.push((self.current, self.masking));
        self.current = self.layers.len();
        self.masking = Masking::NONE;
        self.layers.push(Layer {
            steps: Vec::new(),
            origin,
            size,
            blurs,
            target: None,
        });
    }

    /// Adds what each blur of a layer of `size` pixels is told: for every
    /// pass one across and one down, leaving out a direction whose box is
    /// too narrow to do anything. Returns their slots, in the order they
    /// are to be run.
    fn push_blurs(&mut self, blur_x: f32, blur_y: f32, passes: u8, size: (u32, u32)) -> Vec<usize> {
        let mut blurs = Vec::new();
        for _ in 0..passes {
            for (width, step) in [
                (blur_x, [1.0 / size.0 as f32, 0.0]),
                (blur_y, [0.0, 1.0 / size.1 as f32]),
            ] {
                if width > 1.0 {
                    blurs.push(self.push_item(Item {
                        paint_abcd: [step[0], step[1], 0.0, 0.0],
                        paint_t: [width, 0.0, 0.0, 0.0],
                        ..Item::zeroed()
                    }));
                }
            }
        }
        blurs
    }

    /// Ends the layer being drawn into and goes back to the one it belongs
    /// to, under the masks that were in force there. Returns the layer that
    /// has ended, if there is anything of it to draw.
    fn end_blur(&mut self) -> Option<usize> {
        let (parent, parent_masking) = self.open.pop()?;
        let finished = self.current;
        (self.current, self.masking) = (parent, parent_masking);
        (self.layers[finished].size != (0, 0)).then_some(finished)
    }

    /// Draws a finished layer into the one it belongs to: the unit square,
    /// stretched over the layer's place.
    fn draw_layer(&mut self, finished: usize) {
        let layer = &self.layers[finished];
        let item = self.push_item(Item {
            world_abcd: [layer.size.0 as f32, 0.0, 0.0, layer.size.1 as f32],
            world_t: [layer.origin.0 as f32, layer.origin.1 as f32, 0.0, 0.0],
            paint_abcd: [1.0, 0.0, 0.0, 1.0],
            kind: [4, 0, 0, 0],
            ..Item::zeroed()
        });
        self.push_step(MeshKey::Quad, 0, item, Texture::Layer(finished));
    }

    /// Draws a mesh at `matrix`, tinted by `color`: a step for each run of
    /// its triangles.
    fn draw_mesh(&mut self, key: MeshKey, mesh: &Mesh, matrix: Matrix, color: ColorTransform) {
        for (index, draw) in mesh.draws.iter().enumerate() {
            let (data, texture) = item_for(&draw.paint, matrix, color);
            let item = self.push_item(data);
            self.push_step(key, index, item, texture);
        }
    }
}

#[cfg(test)]
mod tests;
