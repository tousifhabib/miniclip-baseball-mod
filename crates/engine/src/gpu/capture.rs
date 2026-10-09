//! Draws a frame to a picture in memory, with no window.

use anyhow::{Context, Result};

use super::Renderer;
use super::targets::flat_texture;
use crate::display::Command;
use crate::library::Library;

/// How many times running a frame has to come out the same before the
/// picture of it is believed, and how many draws it is given to do that in.
/// The card has been seen to take three or four draws to settle, over its
/// first hundredth of a second, and then to draw the same for as long as it
/// was watched.
const SAME_RUNNING: usize = 6;
const MOST_DRAWS: usize = 30;

/// Draws with `draw` until what it draws has come out the same
/// `same_running` times running, and gives that back. After `most` draws it
/// gives back the last, settled or not.
fn settled<T: PartialEq>(
    mut draw: impl FnMut() -> Result<T>,
    same_running: usize,
    most: usize,
) -> Result<T> {
    let mut picture = draw()?;
    let mut same = 1;
    for _ in 1..most {
        if same >= same_running {
            break;
        }
        let again = draw()?;
        if again == picture {
            same += 1;
        } else {
            picture = again;
            same = 1;
        }
    }
    Ok(picture)
}

impl Renderer {
    /// Draws `commands` to a new image.
    ///
    /// A graphics card that has only just been put to work does not draw
    /// the same frame the same way every time. For its first few draws a
    /// pixel here and there on a gradient can come out a step of one colour
    /// away from where it settles, and which draws those are differs from
    /// one run to the next. Nothing that is sent to the card differs: it
    /// is the card's own doing, and has been seen on Apple's. A picture is
    /// for comparing with another, so the frame is drawn until it has come
    /// out the same several times running, and that is the picture.
    pub fn capture(
        &mut self,
        library: &Library,
        commands: &[Command],
        size: (u32, u32),
        background: [f64; 4],
    ) -> Result<image::RgbaImage> {
        let draw = || self.capture_once(library, commands, size, background);
        settled(draw, SAME_RUNNING, MOST_DRAWS)
    }

    /// Draws `commands` to a new image, once.
    fn capture_once(
        &mut self,
        library: &Library,
        commands: &[Command],
        size: (u32, u32),
        background: [f64; 4],
    ) -> Result<image::RgbaImage> {
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC;
        let texture =
            self.device
                .create_texture(&flat_texture("capture", size, 1, self.format, usage));
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.render(library, commands, &view, size, background, None);
        self.read_back(&texture, size)
    }

    /// Fetches what has been drawn to a texture of `size` pixels back from
    /// the graphics card, as an image.
    fn read_back(&self, texture: &wgpu::Texture, size: (u32, u32)) -> Result<image::RgbaImage> {
        let (width, height) = size;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Something that draws these, one after another, and counts how many
    /// it was asked for.
    fn drawing(
        pictures: &[u8],
    ) -> (
        impl FnMut() -> Result<u8> + '_,
        std::rc::Rc<std::cell::Cell<usize>>,
    ) {
        let drawn = std::rc::Rc::new(std::cell::Cell::new(0));
        let count = drawn.clone();
        let draw = move || {
            let next = pictures[count.get().min(pictures.len() - 1)];
            count.set(count.get() + 1);
            Ok(next)
        };
        (draw, drawn)
    }

    #[test]
    fn a_picture_is_what_came_out_the_same_enough_times_running() {
        // Out by a little for its first few draws, as a card just woken is.
        let (draw, drawn) = drawing(&[1, 2, 2, 1, 1, 1, 1, 9]);
        assert_eq!(settled(draw, 4, 30).unwrap(), 1);
        // And it is not drawn again once it has settled.
        assert_eq!(drawn.get(), 7);
    }

    #[test]
    fn one_that_is_steady_from_the_first_is_drawn_no_more_than_it_has_to_be() {
        let (draw, drawn) = drawing(&[5]);
        assert_eq!(settled(draw, 6, 30).unwrap(), 5);
        assert_eq!(drawn.get(), 6);
    }

    #[test]
    fn one_that_never_settles_is_given_up_on_and_the_last_drawn_is_had() {
        let (draw, drawn) = drawing(&[1, 2, 1, 2, 1, 2, 1, 2, 1, 2, 3]);
        assert_eq!(settled(draw, 3, 5).unwrap(), 1);
        assert_eq!(drawn.get(), 5);
    }
}
