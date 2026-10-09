//! Draws a frame's commands with wgpu.
//!
//! Meshes are built the first time a symbol is drawn and kept. Masks use the
//! stencil buffer: a mask's outline raises the stencil value of the pixels it
//! covers, and masked content only draws where the value matches.
//!
//! A blurred object is drawn to a layer of its own, a texture just big enough
//! to hold it. The layer is blurred and then drawn into its parent like any
//! other picture.
//!
//! A frame is drawn in three goes. Its commands are sorted into layers of
//! draws, in `plan`. What those need is handed to the graphics card, in
//! `upload`, onto textures kept in `targets`. Then the layers are drawn,
//! here. What the card is set up with to begin with is in `pipelines`,
//! finding and opening it is in `device`, and drawing to a picture in
//! memory is in `capture`.

mod capture;
mod device;
mod pipelines;
mod plan;
mod targets;
mod upload;

use std::collections::HashMap;

use bb_format::SymbolId;

use crate::display::Command;
use crate::input::Geometry;
use crate::library::Library;
use crate::meshes::{MeshKey, Meshes};
use pipelines::{Globals, Item, Layouts, Pipelines, Samplers};
use plan::{Layer, Step, Texture};
use targets::{LayerTarget, Targets};
use upload::{Buffers, ramp_texture, slot_buffer};

pub(crate) use device::open_device;

const SAMPLES: u32 = 4;
const STENCIL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;
/// Uniform data for one draw or one layer is placed at multiples of this, the
/// largest alignment any device asks for.
const SLOT: usize = 256;
/// How many meshes of changing text to keep before clearing them out.
const MAX_FIELD_MESHES: usize = 512;

/// What the last frame took to draw.
#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    pub draws: usize,
    pub layers: usize,
    pub meshes: usize,
}

pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    format: wgpu::TextureFormat,
    pipelines: Pipelines,
    layouts: Layouts,
    samplers: Samplers,
    /// What each layer is told, a slot to a layer.
    globals: wgpu::Buffer,
    globals_bind: wgpu::BindGroup,
    globals_capacity: usize,
    /// What each draw is told, a slot to a draw.
    items: wgpu::Buffer,
    items_bind: wgpu::BindGroup,
    items_capacity: usize,
    /// The triangles of everything drawn or pointed at so far.
    meshes: Meshes,
    /// What the graphics card holds of each mesh that has been drawn.
    buffers: HashMap<MeshKey, Buffers>,
    ramps_bind: wgpu::BindGroup,
    ramps_texture: wgpu::Texture,
    ramps_capacity: usize,
    ramps_uploaded: usize,
    image_views: Vec<wgpu::TextureView>,
    image_binds: HashMap<Texture, wgpu::BindGroup>,
    targets: Option<Targets>,
    layer_targets: Vec<LayerTarget>,
    frame: u64,
    /// The least width a stroke is drawn at, in pixels of the target.
    pub min_stroke: f32,
    pub stats: Stats,
}

/// What a layer is drawn onto, and the colour it starts as.
struct Onto<'a> {
    /// Multisampled, and resolved into `resolve` when the layer is done.
    color: &'a wgpu::TextureView,
    resolve: &'a wgpu::TextureView,
    stencil: &'a wgpu::TextureView,
    clear: wgpu::Color,
}

impl Renderer {
    /// `format` is the format of the textures this will draw to. It should not
    /// be an sRGB format: Flash blends colours as stored, without converting.
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, format: wgpu::TextureFormat) -> Renderer {
        let shader = pipelines::shader(&device);
        let layouts = Layouts::new(&device);
        let pipelines = Pipelines::new(&device, &shader, &layouts, format);
        let samplers = Samplers::new(&device);

        let globals_capacity = 16;
        let (globals, globals_bind) =
            slot_buffer::<Globals>(&device, &layouts.globals, globals_capacity);
        let items_capacity = 1024;
        let (items, items_bind) = slot_buffer::<Item>(&device, &layouts.item, items_capacity);
        let ramps_capacity = 64;
        let (ramps_texture, ramps_bind) =
            ramp_texture(&device, &layouts.texture, &samplers.ramp, ramps_capacity);

        Renderer {
            device,
            queue,
            format,
            pipelines,
            layouts,
            samplers,
            globals,
            globals_bind,
            globals_capacity,
            items,
            items_bind,
            items_capacity,
            meshes: Meshes::default(),
            buffers: HashMap::new(),
            ramps_bind,
            ramps_texture,
            ramps_capacity,
            ramps_uploaded: 0,
            image_views: Vec::new(),
            image_binds: HashMap::new(),
            targets: None,
            layer_targets: Vec::new(),
            frame: 0,
            min_stroke: 1.0,
            stats: Stats::default(),
        }
    }

    /// Takes up meshes that were built before there was a renderer, by a
    /// game that had only needed to know what was under its pointer. They
    /// go to the graphics card as each is first drawn.
    pub fn with_meshes(mut self, meshes: Meshes) -> Renderer {
        self.meshes = meshes;
        self
    }

    /// Symbols that could not be drawn, each reported once.
    pub fn problems(&self) -> &[String] {
        self.meshes.problems()
    }

    /// Draws `commands` into `target`, a texture of `size` pixels in the
    /// format this renderer was made for. With `scissor`, only the pixels
    /// inside that rectangle (x, y, width, height) are drawn.
    pub fn render(
        &mut self,
        library: &Library,
        commands: &[Command],
        target: &wgpu::TextureView,
        size: (u32, u32),
        background: [f64; 4],
        scissor: Option<[u32; 4]>,
    ) {
        self.frame += 1;
        // Text that changes often, such as a score, leaves a mesh behind for
        // every value it has shown. Clear them out now and then; the ones
        // still wanted are rebuilt as they are drawn.
        if self.meshes.forget_fields_over(MAX_FIELD_MESHES) {
            self.buffers
                .retain(|key, _| !matches!(key, MeshKey::Field(..)));
        }
        let mut plan = self.prepare(library, commands, size);
        self.upload(&mut plan, size);
        self.stats = Stats {
            draws: plan.layers.iter().map(|layer| layer.steps.len()).sum(),
            layers: plan.layers.len() - 1,
            meshes: self.meshes.len(),
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        // A layer is made after the layer it belongs to, so going backwards
        // finishes every layer before the one that draws it.
        for (index, layer) in plan.layers.iter().enumerate().rev() {
            let Some(onto) = self.onto(index, layer, target, background) else {
                continue;
            };
            self.draw_layer(&mut encoder, &plan.layers, index, &onto, scissor);
            self.blur_layer(&mut encoder, index, layer);
        }
        self.queue.submit([encoder.finish()]);
        self.forget_layer_targets_long_unused();
    }

    /// What the layer at `index` is drawn onto. The first is the frame
    /// itself, which goes to `target` and starts as `background`. Any other
    /// goes to the textures it was given and starts as nothing at all, and
    /// one that was given none, being out of view, is `None`.
    fn onto<'a>(
        &'a self,
        index: usize,
        layer: &Layer,
        target: &'a wgpu::TextureView,
        background: [f64; 4],
    ) -> Option<Onto<'a>> {
        let frame = self.targets.as_ref().expect("made by upload");
        match (index, layer.target) {
            (0, _) => {
                let [r, g, b, a] = background;
                Some(Onto {
                    color: &frame.color,
                    resolve: target,
                    stencil: &frame.stencil,
                    clear: wgpu::Color { r, g, b, a },
                })
            }
            (_, Some(slot)) => {
                let textures = &self.layer_targets[slot];
                Some(Onto {
                    color: &textures.color,
                    resolve: &textures.a,
                    stencil: &textures.stencil,
                    clear: wgpu::Color::TRANSPARENT,
                })
            }
            (_, None) => None,
        }
    }

    /// Draws the steps of the layer at `index` onto its textures. Only the
    /// frame itself is kept to `scissor`.
    fn draw_layer(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        layers: &[Layer],
        index: usize,
        onto: &Onto<'_>,
        scissor: Option<[u32; 4]>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("layer"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: onto.color,
                depth_slice: None,
                resolve_target: Some(onto.resolve),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(onto.clear),
                    store: wgpu::StoreOp::Discard,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: onto.stencil,
                depth_ops: None,
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0),
                    store: wgpu::StoreOp::Discard,
                }),
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &self.globals_bind, &[(index * SLOT) as u32]);
        if index == 0
            && let Some([x, y, width, height]) = scissor
        {
            pass.set_scissor_rect(x, y, width, height);
        }
        self.draw_steps(&mut pass, layers, &layers[index].steps);
    }

    /// Runs the blurs of the layer at `index`, once it has been drawn. Each
    /// reads one of the layer's two textures and writes the other.
    fn blur_layer(&self, encoder: &mut wgpu::CommandEncoder, index: usize, layer: &Layer) {
        let Some(slot) = layer.target else { return };
        let textures = &self.layer_targets[slot];
        for (round, &item) in layer.blurs.iter().enumerate() {
            let (source, destination) = if round.is_multiple_of(2) {
                (&textures.a_bind, &textures.b)
            } else {
                (&textures.b_bind, &textures.a)
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("blur"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: destination,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipelines.blur);
            pass.set_bind_group(0, &self.globals_bind, &[(index * SLOT) as u32]);
            pass.set_bind_group(1, &self.items_bind, &[(item * SLOT) as u32]);
            pass.set_bind_group(2, source, &[]);
            pass.draw(0..3, 0..1);
        }
    }

    fn draw_steps(&self, pass: &mut wgpu::RenderPass<'_>, layers: &[Layer], steps: &[Step]) {
        let mut mode = None;
        let mut key = None;
        let mut texture = None;
        for step in steps {
            let (Some(mesh), Some(buffers)) =
                (self.meshes.get(step.key), self.buffers.get(&step.key))
            else {
                continue;
            };
            let Some(bind) = self.bind_of(step.texture, layers) else {
                continue;
            };
            if mode != Some(step.mode) {
                pass.set_pipeline(&self.pipelines.shapes[step.mode as usize]);
                mode = Some(step.mode);
            }
            if key != Some(step.key) {
                pass.set_vertex_buffer(0, buffers.vertices.slice(..));
                pass.set_index_buffer(buffers.indices.slice(..), wgpu::IndexFormat::Uint32);
                key = Some(step.key);
            }
            if texture != Some(step.texture) {
                pass.set_bind_group(2, bind, &[]);
                texture = Some(step.texture);
            }
            pass.set_stencil_reference(step.stencil);
            pass.set_bind_group(1, &self.items_bind, &[(step.item * SLOT) as u32]);
            pass.draw_indexed(mesh.draws[step.draw].indices.clone(), 0, 0..1);
        }
    }

    /// The texture a draw reads its colours from, as the shader is handed
    /// it. `None` for a layer that was given no textures, being out of
    /// view: there is nothing of it to draw.
    fn bind_of(&self, texture: Texture, layers: &[Layer]) -> Option<&wgpu::BindGroup> {
        Some(match texture {
            Texture::Ramps => &self.ramps_bind,
            Texture::Layer(index) => {
                let layer = &layers[index];
                let textures = &self.layer_targets[layer.target?];
                // An odd number of blurs leaves the picture in the
                // second texture.
                if layer.blurs.len().is_multiple_of(2) {
                    &textures.a_bind
                } else {
                    &textures.b_bind
                }
            }
            image @ Texture::Image { .. } => &self.image_binds[&image],
        })
    }
}

impl Geometry for Renderer {
    fn contains(
        &mut self,
        library: &Library,
        symbol: SymbolId,
        ratio: u16,
        x: f32,
        y: f32,
    ) -> bool {
        self.meshes.contains(library, symbol, ratio, x, y)
    }
}
