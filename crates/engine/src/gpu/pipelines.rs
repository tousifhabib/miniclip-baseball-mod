//! What the graphics card is set up with before anything is drawn: the
//! shader, what it is told for each layer and each draw, the ways a draw
//! may treat a mask, and the ways a texture may be read.

use bytemuck::{Pod, Zeroable};

use super::{SAMPLES, STENCIL_FORMAT};
use crate::tess::Vertex;

const SHADER: &str = include_str!("shader.wgsl");

/// What the shader is told once for each layer.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct Globals {
    pub view: [f32; 4],
    pub limits: [f32; 4],
}

/// What the shader is told for each draw, and for each blur.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct Item {
    pub world_abcd: [f32; 4],
    pub world_t: [f32; 4],
    pub color_mult: [f32; 4],
    pub color_add: [f32; 4],
    pub paint_abcd: [f32; 4],
    pub paint_t: [f32; 4],
    pub kind: [u32; 4],
}

/// How a draw treats the stencil buffer.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    /// Draws colour where the stencil matches.
    Content = 0,
    /// Raises the stencil where it matches, drawing no colour.
    MaskWrite = 1,
    /// Lowers the stencil where it matches, drawing no colour.
    MaskClear = 2,
}

/// The shader, made ready for the graphics card.
pub(super) fn shader(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("shapes"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    })
}

/// The shape of each of the three groups of things the shader is handed:
/// what a layer is told, what a draw is told, and a texture with the way to
/// read it.
pub(super) struct Layouts {
    pub globals: wgpu::BindGroupLayout,
    pub item: wgpu::BindGroupLayout,
    pub texture: wgpu::BindGroupLayout,
}

impl Layouts {
    pub(super) fn new(device: &wgpu::Device) -> Layouts {
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
        let globals = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals"),
            entries: &[uniform_entry(size_of::<Globals>())],
        });
        let item = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("item"),
            entries: &[uniform_entry(size_of::<Item>())],
        });
        let texture = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        Layouts {
            globals,
            item,
            texture,
        }
    }
}

/// The ways of drawing the graphics card is set up for.
pub(super) struct Pipelines {
    /// For drawing a mesh, one for each [`Mode`] and in their order.
    pub shapes: [wgpu::RenderPipeline; 3],
    /// For blurring a layer along one direction.
    pub blur: wgpu::RenderPipeline,
}

impl Pipelines {
    /// `format` is the format of the textures that will be drawn to.
    pub(super) fn new(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        layouts: &Layouts,
        format: wgpu::TextureFormat,
    ) -> Pipelines {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shapes"),
            bind_group_layouts: &[
                Some(&layouts.globals),
                Some(&layouts.item),
                Some(&layouts.texture),
            ],
            immediate_size: 0,
        });
        let shapes = [Mode::Content, Mode::MaskWrite, Mode::MaskClear]
            .map(|mode| shape_pipeline(device, &layout, shader, format, mode));
        let blur = blur_pipeline(device, &layout, shader, format);
        Pipelines { shapes, blur }
    }
}

/// The way of drawing a mesh that treats the stencil buffer as `mode` says.
fn shape_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    mode: Mode,
) -> wgpu::RenderPipeline {
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
    // Colours are multiplied by alpha before blending.
    let blend = wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("shapes"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
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
        depth_stencil: Some(stencil_where_it_matches(pass_op)),
        multisample: wgpu::MultisampleState {
            count: SAMPLES,
            ..wgpu::MultisampleState::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
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
}

/// A draw touches only the pixels whose stencil value is the one it names,
/// and does `pass_op` to the value there. Depth plays no part.
fn stencil_where_it_matches(pass_op: wgpu::StencilOperation) -> wgpu::DepthStencilState {
    let face = wgpu::StencilFaceState {
        compare: wgpu::CompareFunction::Equal,
        fail_op: wgpu::StencilOperation::Keep,
        depth_fail_op: wgpu::StencilOperation::Keep,
        pass_op,
    };
    wgpu::DepthStencilState {
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
    }
}

/// The way of blurring: one triangle over the whole of the texture drawn
/// to, with no mesh, no masks and nothing blended.
fn blur_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("blur"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_blur"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
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
    })
}

/// The ways a texture is read.
pub(super) struct Samplers {
    /// For the gradients' ramps and for finished layers: smoothed, and the
    /// edge colour carried on past the edge.
    pub ramp: wgpu::Sampler,
    /// For an image used as a fill, smoothed. It repeats past its edges.
    pub smooth: wgpu::Sampler,
    /// For an image used as a fill, with its pixels left square.
    pub crisp: wgpu::Sampler,
}

impl Samplers {
    pub(super) fn new(device: &wgpu::Device) -> Samplers {
        let sampler = |filter: wgpu::FilterMode, address: wgpu::AddressMode| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: address,
                address_mode_v: address,
                mag_filter: filter,
                min_filter: filter,
                ..wgpu::SamplerDescriptor::default()
            })
        };
        Samplers {
            ramp: sampler(wgpu::FilterMode::Linear, wgpu::AddressMode::ClampToEdge),
            smooth: sampler(wgpu::FilterMode::Linear, wgpu::AddressMode::Repeat),
            crisp: sampler(wgpu::FilterMode::Nearest, wgpu::AddressMode::Repeat),
        }
    }
}
