//! Draws a frame in the window: the stage, and the inspector over it.

use std::time::Instant;

use winit::window::CursorIcon;

use super::App;
use crate::inspector::Info;

impl App {
    /// Plays what frames are due and draws the stage. Returns whether it
    /// was drawn: a window that is hidden or covered is not.
    pub(super) fn redraw(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now - self.last_redraw;
        self.last_redraw = now;
        self.frame_time += (elapsed.as_secs_f32() - self.frame_time) * 0.1;
        // The pace is kept while paused too, so that it still knows the
        // screen's rate when play goes on.
        let frames = self.pace.frames(elapsed);
        if !self.paused {
            for _ in 0..frames {
                self.step();
            }
        }
        for note in self.runner.take_notes() {
            self.inspector.note(note.to_string());
        }

        let Some((width, height)) = self.size() else {
            return false;
        };
        let (base, scissor) = self.layout(width, height);
        let Some(view) = &mut self.view else {
            return false;
        };

        let texture = match view.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            // The window is out of sight.
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                self.pace.undrawn();
                return false;
            }
            // The window changed under us: set the surface up again and
            // draw on the next round.
            _ => {
                view.surface.configure(&view.renderer.device, &view.config);
                self.pace.undrawn();
                return false;
            }
        };
        // What to draw is only worked out once there is something to draw
        // it on.
        let library = &self.runner.library;
        let commands = self.runner.stage.commands(base, library);
        let background = library.background();
        let target = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        // One point on screen, which is more than one pixel on a dense display.
        view.renderer.min_stroke = view.window.scale_factor() as f32;
        view.renderer.render(
            library,
            &commands,
            &target,
            (width, height),
            background,
            Some(scissor),
        );

        // The inspector goes on top, in a pass of its own.
        let info = Info {
            stage: &self.runner.stage,
            library,
            stats: view.renderer.stats,
            paused: self.paused,
            frames_per_second: 1.0 / self.frame_time.max(1e-6),
            redraws_to_a_frame: self.pace.in_step(),
            base,
        };
        let input = view.egui.take_egui_input(&view.window);
        let context = view.egui.egui_ctx().clone();
        let mut actions = Vec::new();
        let output = context.run_ui(input, |ui| {
            actions = self.inspector.ui(ui.ctx(), &info);
        });
        view.egui
            .handle_platform_output(&view.window, output.platform_output);
        let jobs = context.tessellate(output.shapes, output.pixels_per_point);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point: output.pixels_per_point,
        };
        let (device, queue) = (&view.renderer.device, &view.renderer.queue);
        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                view.egui_renderer.update_texture(device, queue, *id, delta);
            }
        }
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut buffers =
            view.egui_renderer
                .update_buffers(device, queue, &mut encoder, &jobs, &screen);
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("inspector"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &target,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            view.egui_renderer.render(&mut pass, &jobs, &screen);
        }
        buffers.push(encoder.finish());
        queue.submit(buffers);
        for id in &output.textures_delta.free {
            view.egui_renderer.free_texture(id);
        }

        view.renderer.queue.present(texture);
        let cursor = if self.runner.stage.pointer.on_button() {
            CursorIcon::Pointer
        } else {
            CursorIcon::Default
        };
        view.window.set_cursor(cursor);
        // Never while the inspector is up: it needs a pointer to be used.
        view.window
            .set_cursor_visible(!self.runner.stage.hide_pointer || self.inspector.open);
        self.drawn += 1;

        for action in actions {
            self.apply(action);
        }
        true
    }
}
