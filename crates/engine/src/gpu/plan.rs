//! Sorts a frame's commands into what the graphics card is to do: layers
//! of draws, the frame itself first, and what each draw is told.

use bytemuck::Zeroable;

use super::pipelines::{Item, Mode};
use super::{Renderer, SLOT};
use crate::display::{Bounds, Command};
use crate::library::Library;
use crate::math::{ColorTransform, Matrix};
use crate::meshes::MeshKey;
use crate::tess::{Mesh, Paint, Spread};

/// Layer sizes are rounded up to a multiple of this, so that an object which
/// changes size a little from frame to frame can reuse its textures.
const LAYER_STEP: u32 = 64;
const MAX_LAYER: u32 = 4096;
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

/// Mask state, which starts afresh inside each layer.
#[derive(Clone, Copy)]
struct Masking {
    /// How many masks are in force.
    depth: u32,
    mode: Mode,
    stencil: u32,
}

impl Masking {
    const NONE: Masking = Masking {
        depth: 0,
        mode: Mode::Content,
        stencil: 0,
    };

    /// The draws that follow are the outline of a new mask.
    fn push(&mut self) {
        self.mode = Mode::MaskWrite;
        self.stencil = self.depth;
        self.depth += 1;
    }

    /// The outline is done: from here on, only what is inside it is drawn.
    fn activate(&mut self) {
        self.mode = Mode::Content;
        self.stencil = self.depth;
    }

    /// The draws that follow repeat the innermost mask's outline, to take
    /// it away.
    fn deactivate(&mut self) {
        self.mode = Mode::MaskClear;
        self.stencil = self.depth;
    }

    /// The innermost mask has gone.
    fn pop(&mut self) {
        self.depth = self.depth.saturating_sub(1);
        self.mode = Mode::Content;
        self.stencil = self.depth;
    }
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

impl Renderer {
    /// Sorts `commands` into layers of draws, building any meshes they need.
    /// The first layer is the frame itself.
    pub(super) fn prepare(
        &mut self,
        library: &Library,
        commands: &[Command],
        size: (u32, u32),
    ) -> Plan {
        let mut plan = Plan::new(size);
        for command in commands {
            match command {
                Command::PushMask => plan.masking.push(),
                Command::ActivateMask => plan.masking.activate(),
                Command::DeactivateMask => plan.masking.deactivate(),
                Command::PopMask => plan.masking.pop(),
                Command::BeginBlur {
                    blur_x,
                    blur_y,
                    passes,
                    bounds,
                } => plan.begin_blur(*blur_x, *blur_y, *passes, *bounds),
                Command::EndBlur => {
                    if let Some(finished) = plan.end_blur() {
                        self.ensure_mesh(library, MeshKey::Quad, None);
                        plan.draw_layer(finished);
                    }
                }
                Command::Draw {
                    symbol,
                    ratio,
                    matrix,
                    color,
                    text,
                } => {
                    if plan.out_of_view() {
                        continue;
                    }
                    let text = text.as_deref();
                    let Some(key) = MeshKey::of(library, *symbol, *ratio, text) else {
                        continue;
                    };
                    let Some(mesh) = self.ensure_mesh(library, key, text) else {
                        continue;
                    };
                    plan.draw_mesh(key, mesh, *matrix, *color);
                }
            }
        }
        plan
    }
}

/// Where a blurred object's layer goes: `bounds` grown by `reach`, cut down
/// to the part that can affect the parent layer, on whole pixels. Returns the
/// top-left corner and the size, or `None` if nothing of it can be seen.
fn layer_area(
    bounds: Bounds,
    reach: f32,
    parent_origin: (i32, i32),
    parent_size: (u32, u32),
) -> Option<((i32, i32), (u32, u32))> {
    let reach_px = reach as i32;
    let left = ((bounds[0] - reach).floor() as i32).max(parent_origin.0 - reach_px);
    let top = ((bounds[1] - reach).floor() as i32).max(parent_origin.1 - reach_px);
    let right =
        ((bounds[2] + reach).ceil() as i32).min(parent_origin.0 + parent_size.0 as i32 + reach_px);
    let bottom =
        ((bounds[3] + reach).ceil() as i32).min(parent_origin.1 + parent_size.1 as i32 + reach_px);
    if right <= left || bottom <= top {
        return None;
    }
    let round = |length: i32| (length as u32).next_multiple_of(LAYER_STEP).min(MAX_LAYER);
    Some(((left, top), (round(right - left), round(bottom - top))))
}

fn item_for(paint: &Paint, world: Matrix, color: ColorTransform) -> (Item, Texture) {
    let spread_code = |spread: Spread| match spread {
        Spread::Pad => 0,
        Spread::Reflect => 1,
        Spread::Repeat => 2,
    };
    let (kind, spread, matrix, row, texture) = match *paint {
        Paint::Solid => (0, 0, Matrix::IDENTITY, 0.0, Texture::Ramps),
        Paint::Linear {
            ramp,
            matrix,
            spread,
        } => (1, spread_code(spread), matrix, ramp as f32, Texture::Ramps),
        Paint::Radial {
            ramp,
            matrix,
            spread,
        } => (2, spread_code(spread), matrix, ramp as f32, Texture::Ramps),
        Paint::Image {
            image,
            matrix,
            smooth,
        } => (
            3,
            0,
            matrix,
            0.0,
            Texture::Image {
                slot: image,
                smooth,
            },
        ),
    };
    let item = Item {
        world_abcd: [world.a, world.b, world.c, world.d],
        world_t: [world.tx, world.ty, 0.0, 0.0],
        color_mult: color.mult,
        color_add: color.add,
        paint_abcd: [matrix.a, matrix.b, matrix.c, matrix.d],
        paint_t: [matrix.tx, matrix.ty, row, 0.0],
        kind: [kind, spread, 0, 0],
    };
    (item, texture)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How draws treat the masks just now: the mode, by its number, and the
    /// stencil value a draw is to match.
    fn masks(plan: &Plan) -> (u32, u32) {
        (plan.masking.mode as u32, plan.masking.stencil)
    }

    #[test]
    fn masks_count_up_as_one_is_set_inside_another_and_down_as_each_ends() {
        let (content, write, clear) = (0, 1, 2);
        let mut plan = Plan::new((590, 400));
        assert_eq!(masks(&plan), (content, 0));
        // The outline is drawn where there is no mask yet, and what it
        // covers is drawn where there is now one.
        plan.masking.push();
        assert_eq!(masks(&plan), (write, 0));
        plan.masking.activate();
        assert_eq!(masks(&plan), (content, 1));
        // A second inside the first.
        plan.masking.push();
        assert_eq!(masks(&plan), (write, 1));
        plan.masking.activate();
        assert_eq!(masks(&plan), (content, 2));
        // Each is taken away from where it was put, innermost first.
        plan.masking.deactivate();
        assert_eq!(masks(&plan), (clear, 2));
        plan.masking.pop();
        assert_eq!(masks(&plan), (content, 1));
        plan.masking.deactivate();
        assert_eq!(masks(&plan), (clear, 1));
        plan.masking.pop();
        assert_eq!(masks(&plan), (content, 0));
        // One end too many changes nothing.
        plan.masking.pop();
        assert_eq!(masks(&plan), (content, 0));
    }

    #[test]
    fn a_blurred_object_gets_a_layer_of_its_own_where_masks_start_afresh() {
        let mut plan = Plan::new((590, 400));
        plan.masking.push();
        plan.masking.activate();
        // Twice over, with a box 4 wide across and 1 down.
        plan.begin_blur(4.0, 1.0, 2, [100.0, 100.0, 150.0, 140.0]);
        assert_eq!((plan.layers.len(), plan.current), (2, 1));
        // The picture spreads by 4 in all, and there is one more to spare.
        let layer = &plan.layers[1];
        assert_eq!((layer.origin, layer.size), ((95, 95), (64, 64)));
        // A box one pixel wide does nothing, so each time over is a blur
        // across and none down: a step of one pixel of the 64.
        assert_eq!(layer.blurs, [0, 1]);
        let told: Item = bytemuck::pod_read_unaligned(&plan.items[..size_of::<Item>()]);
        assert_eq!(told.paint_abcd, [1.0 / 64.0, 0.0, 0.0, 0.0]);
        assert_eq!(told.paint_t, [4.0, 0.0, 0.0, 0.0]);
        assert_eq!(masks(&plan), (0, 0));

        // When it ends, draws go where they went before, under the mask
        // that was in force there.
        assert_eq!(plan.end_blur(), Some(1));
        assert_eq!(plan.current, 0);
        assert_eq!(masks(&plan), (0, 1));
        plan.draw_layer(1);
        let step = &plan.layers[0].steps[0];
        assert!(step.texture == Texture::Layer(1));
        assert_eq!((step.item, step.stencil), (2, 1));
        // An end with nothing begun is passed over.
        assert_eq!(plan.end_blur(), None);
    }

    #[test]
    fn nothing_is_drawn_of_a_blurred_object_that_is_out_of_view() {
        let mut plan = Plan::new((590, 400));
        assert!(!plan.out_of_view());
        plan.begin_blur(4.0, 4.0, 1, [700.0, 10.0, 800.0, 40.0]);
        assert!(plan.out_of_view());
        assert!(plan.layers[1].blurs.is_empty());
        assert!(plan.items.is_empty());
        // There is nothing of it to draw into the frame, either.
        assert_eq!(plan.end_blur(), None);
        assert!(!plan.out_of_view());
    }

    #[test]
    fn a_layer_covers_its_object_and_the_reach_of_the_blur() {
        let area = layer_area([100.0, 100.0, 150.0, 140.0], 4.0, (0, 0), (590, 400));
        // 96 to 154 across and 96 to 144 down, each rounded up to 64.
        assert_eq!(area, Some(((96, 96), (64, 64))));
    }

    #[test]
    fn a_layer_is_cut_down_to_what_can_reach_the_view() {
        let area = layer_area([-500.0, 10.0, 30.0, 40.0], 4.0, (0, 0), (590, 400));
        assert_eq!(area, Some(((-4, 6), (64, 64))));
    }

    #[test]
    fn an_object_wholly_out_of_view_gets_no_layer() {
        assert_eq!(
            layer_area([700.0, 10.0, 800.0, 40.0], 4.0, (0, 0), (590, 400)),
            None
        );
    }

    /// Where something is drawn, how it is tinted and where its paint sits
    /// in it, with every number different so that none can be taken for
    /// another.
    const WORLD: Matrix = Matrix {
        a: 1.0,
        b: 2.0,
        c: 3.0,
        d: 4.0,
        tx: 5.0,
        ty: 6.0,
    };
    const TINT: ColorTransform = ColorTransform {
        mult: [0.1, 0.2, 0.3, 0.4],
        add: [0.5, 0.6, 0.7, 0.8],
    };
    const PLACED: Matrix = Matrix {
        a: 7.0,
        b: 8.0,
        c: 9.0,
        d: 10.0,
        tx: 11.0,
        ty: 12.0,
    };

    #[test]
    fn every_paint_is_told_where_it_is_drawn_and_how_it_is_tinted() {
        let paints = [
            Paint::Solid,
            Paint::Linear {
                ramp: 7,
                matrix: PLACED,
                spread: Spread::Pad,
            },
            Paint::Radial {
                ramp: 3,
                matrix: PLACED,
                spread: Spread::Pad,
            },
            Paint::Image {
                image: 5,
                matrix: PLACED,
                smooth: true,
            },
        ];
        for paint in paints {
            let (item, _) = item_for(&paint, WORLD, TINT);
            assert_eq!(item.world_abcd, [1.0, 2.0, 3.0, 4.0]);
            assert_eq!(item.world_t, [5.0, 6.0, 0.0, 0.0]);
            assert_eq!(item.color_mult, TINT.mult);
            assert_eq!(item.color_add, TINT.add);
        }
    }

    #[test]
    fn a_solid_paint_sits_nowhere_of_its_own_and_reads_no_row_of_the_ramps() {
        let (item, texture) = item_for(&Paint::Solid, WORLD, TINT);
        assert_eq!(item.paint_abcd, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(item.paint_t, [0.0; 4]);
        assert_eq!(item.kind, [0; 4]);
        // The ramps are bound all the same: a draw must have some texture.
        assert!(texture == Texture::Ramps);
    }

    #[test]
    fn a_gradient_is_told_its_row_of_the_ramps_and_how_it_goes_on_past_its_ends() {
        let ways = [(Spread::Pad, 0), (Spread::Reflect, 1), (Spread::Repeat, 2)];
        for (spread, code) in ways {
            let linear = Paint::Linear {
                ramp: 7,
                matrix: PLACED,
                spread,
            };
            let (item, texture) = item_for(&linear, WORLD, TINT);
            assert_eq!(item.paint_abcd, [7.0, 8.0, 9.0, 10.0]);
            assert_eq!(item.paint_t, [11.0, 12.0, 7.0, 0.0]);
            assert_eq!(item.kind, [1, code, 0, 0]);
            assert!(texture == Texture::Ramps);

            let radial = Paint::Radial {
                ramp: 3,
                matrix: PLACED,
                spread,
            };
            let (item, texture) = item_for(&radial, WORLD, TINT);
            assert_eq!(item.paint_abcd, [7.0, 8.0, 9.0, 10.0]);
            assert_eq!(item.paint_t, [11.0, 12.0, 3.0, 0.0]);
            assert_eq!(item.kind, [2, code, 0, 0]);
            assert!(texture == Texture::Ramps);
        }
    }

    #[test]
    fn an_image_is_read_from_a_texture_of_its_own_smoothed_or_not() {
        for smooth in [false, true] {
            let image = Paint::Image {
                image: 5,
                matrix: PLACED,
                smooth,
            };
            let (item, texture) = item_for(&image, WORLD, TINT);
            assert_eq!(item.paint_abcd, [7.0, 8.0, 9.0, 10.0]);
            assert_eq!(item.paint_t, [11.0, 12.0, 0.0, 0.0]);
            assert_eq!(item.kind, [3, 0, 0, 0]);
            assert!(texture == Texture::Image { slot: 5, smooth });
        }
    }
}
