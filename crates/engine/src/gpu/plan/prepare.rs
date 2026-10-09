//! Making the plan of a frame from the list of what to draw.

use super::{Plan, Texture};
use crate::display::{Bounds, Command};
use crate::gpu::Renderer;
use crate::gpu::pipelines::Item;
use crate::library::Library;
use crate::math::{ColorTransform, Matrix};
use crate::meshes::MeshKey;
use crate::tess::{Paint, Spread};

/// Layer sizes are rounded up to a multiple of this, so that an object which
/// changes size a little from frame to frame can reuse its textures.
const LAYER_STEP: u32 = 64;
const MAX_LAYER: u32 = 4096;

impl Renderer {
    /// Sorts `commands` into layers of draws, building any meshes they need.
    /// The first layer is the frame itself.
    pub(in crate::gpu) fn prepare(
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
pub(super) fn layer_area(
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

pub(super) fn item_for(paint: &Paint, world: Matrix, color: ColorTransform) -> (Item, Texture) {
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
