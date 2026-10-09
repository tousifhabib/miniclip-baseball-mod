//! Hands the graphics card what a frame needs before it is drawn: the
//! meshes, what each layer and each draw is told, and any new gradients and
//! images.

use wgpu::util::DeviceExt;

use super::pipelines::{Globals, Item};
use super::plan::{Layer, Texture};
use super::targets::{Targets, attachment, flat_texture};
use super::{Renderer, SAMPLES, SLOT, STENCIL_FORMAT};
use crate::library::Library;
use crate::meshes::MeshKey;
use crate::tess::Mesh;

/// A mesh's corners and the order they are joined in, as the graphics card
/// holds them.
pub(super) struct Buffers {
    pub vertices: wgpu::Buffer,
    pub indices: wgpu::Buffer,
}

impl Renderer {
    /// Builds the mesh for `key` if it is not there yet, hands it to the
    /// graphics card if that has not been done, and returns it if it came
    /// to anything. `text` is what a [`MeshKey::Field`] says.
    ///
    /// A mesh may have been built already without having gone to the card:
    /// the pointer asks about shapes that have yet to be drawn.
    pub(super) fn ensure_mesh(
        &mut self,
        library: &Library,
        key: MeshKey,
        text: Option<&str>,
    ) -> Option<&Mesh> {
        let mesh = self.meshes.ensure(library, key, text)?;
        self.buffers.entry(key).or_insert_with(|| Buffers {
            vertices: self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("vertices"),
                    contents: bytemuck::cast_slice(&mesh.vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
            indices: self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("indices"),
                    contents: bytemuck::cast_slice(&mesh.indices),
                    usage: wgpu::BufferUsages::INDEX,
                }),
        });
        Some(mesh)
    }

    /// Sends this frame's uniforms, and any new ramps and images, to the GPU,
    /// and gives every layer textures to draw to.
    pub(super) fn upload(&mut self, layers: &mut [Layer], items: &[u8], size: (u32, u32)) {
        if layers.len() > self.globals_capacity {
            self.globals_capacity = layers.len().next_power_of_two();
            (self.globals, self.globals_bind) =
                slot_buffer::<Globals>(&self.device, &self.globals_layout, self.globals_capacity);
        }
        let mut globals = vec![0u8; layers.len() * SLOT];
        for (index, layer) in layers.iter().enumerate() {
            let (width, height) = (layer.size.0.max(1) as f32, layer.size.1.max(1) as f32);
            let (x, y) = (layer.origin.0 as f32, layer.origin.1 as f32);
            let data = Globals {
                view: [
                    2.0 / width,
                    -2.0 / height,
                    -1.0 - 2.0 * x / width,
                    1.0 + 2.0 * y / height,
                ],
                limits: [self.min_stroke / 2.0, 0.0, 0.0, 0.0],
            };
            let at = index * SLOT;
            globals[at..at + size_of::<Globals>()].copy_from_slice(bytemuck::bytes_of(&data));
        }
        self.queue.write_buffer(&self.globals, 0, &globals);

        let needed = items.len() / SLOT;
        if needed > self.items_capacity {
            self.items_capacity = needed.next_power_of_two();
            (self.items, self.items_bind) =
                slot_buffer::<Item>(&self.device, &self.item_layout, self.items_capacity);
        }
        if !items.is_empty() {
            self.queue.write_buffer(&self.items, 0, items);
        }

        let ramps = self.meshes.ramps();
        if ramps.len() > self.ramps_capacity {
            self.ramps_capacity = ramps.len().next_power_of_two();
            (self.ramps_texture, self.ramps_bind) = ramp_texture(
                &self.device,
                &self.texture_layout,
                &self.ramp_sampler,
                self.ramps_capacity,
            );
            self.ramps_uploaded = 0;
        }
        if ramps.len() > self.ramps_uploaded {
            let rows = &ramps[self.ramps_uploaded..];
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.ramps_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: self.ramps_uploaded as u32,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                bytemuck::cast_slice(rows),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256 * 4),
                    rows_per_image: None,
                },
                wgpu::Extent3d {
                    width: 256,
                    height: rows.len() as u32,
                    depth_or_array_layers: 1,
                },
            );
            self.ramps_uploaded = ramps.len();
        }

        for image in &self.meshes.images()[self.image_views.len()..] {
            let mut pixels = image.rgba.clone();
            for pixel in pixels.as_chunks_mut::<4>().0 {
                let alpha = u32::from(pixel[3]);
                for channel in &mut pixel[..3] {
                    *channel = ((u32::from(*channel) * alpha + 127) / 255) as u8;
                }
            }
            let texture = self.device.create_texture_with_data(
                &self.queue,
                &flat_texture(
                    "image",
                    (image.width, image.height),
                    1,
                    wgpu::TextureFormat::Rgba8Unorm,
                    wgpu::TextureUsages::TEXTURE_BINDING,
                ),
                wgpu::util::TextureDataOrder::LayerMajor,
                &pixels,
            );
            self.image_views
                .push(texture.create_view(&wgpu::TextureViewDescriptor::default()));
        }
        for (slot, view) in self.image_views.iter().enumerate() {
            for smooth in [false, true] {
                let key = Texture::Image { slot, smooth };
                if !self.image_binds.contains_key(&key) {
                    let sampler = if smooth {
                        &self.smooth_sampler
                    } else {
                        &self.crisp_sampler
                    };
                    let bind = texture_bind(&self.device, &self.texture_layout, view, sampler);
                    self.image_binds.insert(key, bind);
                }
            }
        }

        if self
            .targets
            .as_ref()
            .is_none_or(|targets| targets.size != size)
        {
            self.targets = Some(Targets {
                size,
                color: attachment(&self.device, size, self.format, SAMPLES),
                stencil: attachment(&self.device, size, STENCIL_FORMAT, SAMPLES),
            });
        }

        // Give each layer a set of textures of its size that no other layer
        // has taken this frame.
        for layer in layers.iter_mut().skip(1) {
            if layer.size == (0, 0) {
                continue;
            }
            let free = self
                .layer_targets
                .iter()
                .position(|target| target.size == layer.size && target.last_used != self.frame);
            let slot = match free {
                Some(slot) => slot,
                None => {
                    self.layer_targets.push(self.new_layer_target(layer.size));
                    self.layer_targets.len() - 1
                }
            };
            self.layer_targets[slot].last_used = self.frame;
            layer.target = Some(slot);
        }
    }
}

/// A uniform buffer holding `capacity` values of `T`, one per slot, bound so
/// that a draw picks its slot by offset.
pub(super) fn slot_buffer<T>(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    capacity: usize,
) -> (wgpu::Buffer, wgpu::BindGroup) {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("uniforms"),
        size: (capacity * SLOT) as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("uniforms"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &buffer,
                offset: 0,
                size: wgpu::BufferSize::new(size_of::<T>() as u64),
            }),
        }],
    });
    (buffer, bind)
}

/// A texture holding one gradient ramp per row.
pub(super) fn ramp_texture(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    rows: usize,
) -> (wgpu::Texture, wgpu::BindGroup) {
    let texture = device.create_texture(&flat_texture(
        "ramps",
        (256, rows as u32),
        1,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    ));
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind = texture_bind(device, layout, &view, sampler);
    (texture, bind)
}

pub(super) fn texture_bind(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("paint texture"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}
