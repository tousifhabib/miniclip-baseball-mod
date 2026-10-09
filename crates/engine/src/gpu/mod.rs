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
use crate::tess::Vertex;
use pipelines::{Globals, Item, Mode, SHADER};
use plan::{Layer, Step, Texture};
use targets::{LAYER_LIFETIME, LayerTarget, Targets};
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
    pipelines: [wgpu::RenderPipeline; 3],
    blur_pipeline: wgpu::RenderPipeline,
    globals_layout: wgpu::BindGroupLayout,
    globals: wgpu::Buffer,
    globals_bind: wgpu::BindGroup,
    globals_capacity: usize,
    item_layout: wgpu::BindGroupLayout,
    items: wgpu::Buffer,
    items_bind: wgpu::BindGroup,
    items_capacity: usize,
    texture_layout: wgpu::BindGroupLayout,
    ramp_sampler: wgpu::Sampler,
    smooth_sampler: wgpu::Sampler,
    crisp_sampler: wgpu::Sampler,
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

impl Renderer {
    /// `format` is the format of the textures this will draw to. It should not
    /// be an sRGB format: Flash blends colours as stored, without converting.
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, format: wgpu::TextureFormat) -> Renderer {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shapes"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let uniform_entry = |size: usize| wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: wgpu::BufferSize::new(size as u64),
            },
            count: None,
        };
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals"),
            entries: &[uniform_entry(size_of::<Globals>())],
        });
        let item_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("item"),
            entries: &[uniform_entry(size_of::<Item>())],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("paint texture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shapes"),
            bind_group_layouts: &[
                Some(&globals_layout),
                Some(&item_layout),
                Some(&texture_layout),
            ],
            immediate_size: 0,
        });

        let pipeline = |mode: Mode| {
            let (pass_op, write_mask) = match mode {
                Mode::Content => (wgpu::StencilOperation::Keep, wgpu::ColorWrites::ALL),
                Mode::MaskWrite => (
                    wgpu::StencilOperation::IncrementClamp,
                    wgpu::ColorWrites::empty(),
                ),
                Mode::MaskClear => (
                    wgpu::StencilOperation::DecrementClamp,
                    wgpu::ColorWrites::empty(),
                ),
            };
            let face = wgpu::StencilFaceState {
                compare: wgpu::CompareFunction::Equal,
                fail_op: wgpu::StencilOperation::Keep,
                depth_fail_op: wgpu::StencilOperation::Keep,
                pass_op,
            };
            // Colours are multiplied by alpha before blending.
            let blend = wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            };
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("shapes"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![
                            0 => Float32x2,
                            1 => Float32x2,
                            2 => Float32,
                            3 => Unorm8x4,
                        ],
                    })],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: STENCIL_FORMAT,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::Always),
                    stencil: wgpu::StencilState {
                        front: face,
                        back: face,
                        read_mask: 0xff,
                        write_mask: 0xff,
                    },
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: SAMPLES,
                    ..wgpu::MultisampleState::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState {
                            color: blend,
                            alpha: blend,
                        }),
                        write_mask,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let pipelines = [
            pipeline(Mode::Content),
            pipeline(Mode::MaskWrite),
            pipeline(Mode::MaskClear),
        ];
        let blur_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blur"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_blur"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_blur"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let sampler = |filter: wgpu::FilterMode, address: wgpu::AddressMode| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: address,
                address_mode_v: address,
                mag_filter: filter,
                min_filter: filter,
                ..wgpu::SamplerDescriptor::default()
            })
        };
        let ramp_sampler = sampler(wgpu::FilterMode::Linear, wgpu::AddressMode::ClampToEdge);
        let smooth_sampler = sampler(wgpu::FilterMode::Linear, wgpu::AddressMode::Repeat);
        let crisp_sampler = sampler(wgpu::FilterMode::Nearest, wgpu::AddressMode::Repeat);

        let globals_capacity = 16;
        let (globals, globals_bind) =
            slot_buffer::<Globals>(&device, &globals_layout, globals_capacity);
        let items_capacity = 1024;
        let (items, items_bind) = slot_buffer::<Item>(&device, &item_layout, items_capacity);
        let ramps_capacity = 64;
        let (ramps_texture, ramps_bind) =
            ramp_texture(&device, &texture_layout, &ramp_sampler, ramps_capacity);

        Renderer {
            device,
            queue,
            format,
            pipelines,
            blur_pipeline,
            globals_layout,
            globals,
            globals_bind,
            globals_capacity,
            item_layout,
            items,
            items_bind,
            items_capacity,
            texture_layout,
            ramp_sampler,
            smooth_sampler,
            crisp_sampler,
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
        let (mut layers, items) = self.prepare(library, commands, size);
        self.upload(&mut layers, &items, size);
        self.stats = Stats {
            draws: layers.iter().map(|layer| layer.steps.len()).sum(),
            layers: layers.len() - 1,
            meshes: self.meshes.len(),
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        // A layer is made after the layer it belongs to, so going backwards
        // finishes every layer before the one that draws it.
        for (index, layer) in layers.iter().enumerate().rev() {
            let frame = self.targets.as_ref().expect("made by upload");
            let (color, resolve, stencil, clear) = match (index, layer.target) {
                (0, _) => {
                    let [r, g, b, a] = background;
                    let clear = wgpu::Color { r, g, b, a };
                    (&frame.color, target, &frame.stencil, clear)
                }
                (_, Some(slot)) => {
                    let layer = &self.layer_targets[slot];
                    let clear = wgpu::Color::TRANSPARENT;
                    (&layer.color, &layer.a, &layer.stencil, clear)
                }
                (_, None) => continue,
            };
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("layer"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: color,
                        depth_slice: None,
                        resolve_target: Some(resolve),
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(clear),
                            store: wgpu::StoreOp::Discard,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: stencil,
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
                self.draw_steps(&mut pass, &layers, &layer.steps);
            }

            // Each blur reads one of the layer's two textures and writes the
            // other.
            let Some(slot) = layer.target else { continue };
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
                pass.set_pipeline(&self.blur_pipeline);
                pass.set_bind_group(0, &self.globals_bind, &[(index * SLOT) as u32]);
                pass.set_bind_group(1, &self.items_bind, &[(item * SLOT) as u32]);
                pass.set_bind_group(2, source, &[]);
                pass.draw(0..3, 0..1);
            }
        }
        self.queue.submit([encoder.finish()]);

        let frame = self.frame;
        self.layer_targets
            .retain(|target| frame - target.last_used < LAYER_LIFETIME);
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
            let bind = match step.texture {
                Texture::Ramps => &self.ramps_bind,
                Texture::Layer(index) => {
                    let layer = &layers[index];
                    let Some(slot) = layer.target else { continue };
                    let textures = &self.layer_targets[slot];
                    // An odd number of blurs leaves the picture in the
                    // second texture.
                    if layer.blurs.len().is_multiple_of(2) {
                        &textures.a_bind
                    } else {
                        &textures.b_bind
                    }
                }
                image @ Texture::Image { .. } => &self.image_binds[&image],
            };
            if mode != Some(step.mode) {
                pass.set_pipeline(&self.pipelines[step.mode as usize]);
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
