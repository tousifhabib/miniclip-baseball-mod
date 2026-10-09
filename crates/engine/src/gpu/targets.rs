//! The textures that are drawn to: the frame's own, and the ones a blurred
//! layer is drawn to and blurred between.

use super::upload::texture_bind;
use super::{Renderer, SAMPLES, STENCIL_FORMAT};

/// A layer's textures are dropped after going unused for this many frames.
pub(super) const LAYER_LIFETIME: u64 = 300;

/// The textures a layer draws to and is blurred between.
pub(super) struct LayerTarget {
    pub size: (u32, u32),
    /// Multisampled, resolved into `a`.
    pub color: wgpu::TextureView,
    pub stencil: wgpu::TextureView,
    pub a: wgpu::TextureView,
    pub a_bind: wgpu::BindGroup,
    pub b: wgpu::TextureView,
    pub b_bind: wgpu::BindGroup,
    pub last_used: u64,
}

/// The textures the frame itself is drawn to, before it is handed over.
pub(super) struct Targets {
    pub size: (u32, u32),
    pub color: wgpu::TextureView,
    pub stencil: wgpu::TextureView,
}

impl Renderer {
    pub(super) fn new_layer_target(&self, size: (u32, u32)) -> LayerTarget {
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        let sampled = || {
            self.device
                .create_texture(&flat_texture("layer", size, 1, self.format, usage))
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        let (a, b) = (sampled(), sampled());
        let bind = |view: &wgpu::TextureView| {
            texture_bind(&self.device, &self.texture_layout, view, &self.ramp_sampler)
        };
        LayerTarget {
            size,
            color: attachment(&self.device, size, self.format, SAMPLES),
            stencil: attachment(&self.device, size, STENCIL_FORMAT, SAMPLES),
            a_bind: bind(&a),
            b_bind: bind(&b),
            a,
            b,
            last_used: 0,
        }
    }
}

/// What the graphics card is asked for when a texture is wanted: one flat
/// picture of `size` pixels, with `samples` to a pixel and no smaller copies
/// of itself kept beside it.
pub(super) fn flat_texture(
    label: &'static str,
    size: (u32, u32),
    samples: u32,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
) -> wgpu::TextureDescriptor<'static> {
    wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: samples,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    }
}

/// A texture to draw into and nothing else.
pub(super) fn attachment(
    device: &wgpu::Device,
    size: (u32, u32),
    format: wgpu::TextureFormat,
    samples: u32,
) -> wgpu::TextureView {
    let usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
    device
        .create_texture(&flat_texture("attachment", size, samples, format, usage))
        .create_view(&wgpu::TextureViewDescriptor::default())
}
