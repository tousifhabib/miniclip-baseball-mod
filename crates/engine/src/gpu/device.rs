//! Finds a graphics card and opens it, for a window or for none.

use anyhow::{Context, Result};

use super::Renderer;

impl Renderer {
    /// Opens the default graphics device, with no window.
    pub fn headless() -> Result<Renderer> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (_adapter, device, queue) = open_device(&instance, None)?;
        Ok(Renderer::new(
            device,
            queue,
            wgpu::TextureFormat::Rgba8Unorm,
        ))
    }
}

/// Finds a graphics adapter and opens the device it gives. With `surface`,
/// the adapter is one that can draw to that window.
pub(crate) fn open_device(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue)> {
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: surface,
        ..wgpu::RequestAdapterOptions::default()
    }))
    .context("finding a graphics adapter")?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .context("opening the graphics device")?;
    Ok((adapter, device, queue))
}
