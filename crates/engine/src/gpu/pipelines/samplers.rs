//! The ways a texture is read: smoothed or not, held at its edge or
//! repeated.

/// The ways a texture is read.
pub(in crate::gpu) struct Samplers {
    /// For the gradients' ramps and for finished layers: smoothed, and the
    /// edge colour carried on past the edge.
    pub ramp: wgpu::Sampler,
    /// For an image used as a fill, smoothed. It repeats past its edges.
    pub smooth: wgpu::Sampler,
    /// For an image used as a fill, with its pixels left square.
    pub crisp: wgpu::Sampler,
}

impl Samplers {
    pub(in crate::gpu) fn new(device: &wgpu::Device) -> Samplers {
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
