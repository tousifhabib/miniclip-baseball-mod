//! Drawing a frame from its plan: each layer into a texture of its own,
//! the blurs of those that have one, and then the screen.

use super::{Renderer, SLOT, Stats};
use crate::display::Command;
use crate::gpu::plan::{Layer, Step, Texture};
use crate::library::Library;
use crate::meshes::MeshKey;

/// How many meshes of changing text to keep before clearing them out.
const MAX_FIELD_MESHES: usize = 512;

/// What a layer is drawn onto, and the colour it starts as.
struct Onto<'a> {
    /// Multisampled, and resolved into `resolve` when the layer is done.
    color: &'a wgpu::TextureView,
    resolve: &'a wgpu::TextureView,
    stencil: &'a wgpu::TextureView,
    clear: wgpu::Color,
}

impl Renderer {
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
