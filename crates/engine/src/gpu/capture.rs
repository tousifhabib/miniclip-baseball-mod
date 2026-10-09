//! Draws a frame to a picture in memory, with no window.

use anyhow::{Context, Result};

use super::Renderer;
use super::targets::flat_texture;
use crate::display::Command;
use crate::library::Library;

impl Renderer {
    /// Draws `commands` to a new image.
    pub fn capture(
        &mut self,
        library: &Library,
        commands: &[Command],
        size: (u32, u32),
        background: [f64; 4],
    ) -> Result<image::RgbaImage> {
        let (width, height) = size;
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC;
        let texture =
            self.device
                .create_texture(&flat_texture("capture", size, 1, self.format, usage));
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.render(library, commands, &view, size, background, None);

        // Rows in a copy must be a multiple of 256 bytes long.
        let row = (width as usize * 4).next_multiple_of(256);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("capture"),
            size: (row * height as usize) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row as u32),
                    rows_per_image: None,
                },
            },
            texture.size(),
        );
        self.queue.submit([encoder.finish()]);

        buffer.map_async(wgpu::MapMode::Read, .., |result| {
            result.expect("mapping the capture buffer");
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .context("waiting for the frame to finish")?;
        let data = buffer
            .get_mapped_range(..)
            .context("reading the capture buffer")?;
        let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
        for line in data.chunks_exact(row) {
            pixels.extend_from_slice(&line[..width as usize * 4]);
        }
        image::RgbaImage::from_raw(width, height, pixels).context("building the image")
    }
}
