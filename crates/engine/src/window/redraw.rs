//! Draws a frame in the window: the stage, and the inspector over it.

use std::time::Instant;

use winit::window::CursorIcon;

use super::App;
use super::view::View;
use crate::display::Command;
use crate::inspector::{Action, Info, Inspector};
use crate::library::Library;
use crate::stage::Stage;

impl App {
    /// Plays what frames are due and draws the stage. Returns whether it
    /// was drawn: a window that is hidden or covered is not.
    pub(super) fn redraw(&mut self) -> bool {
        self.play_frames_due();

        let Some((width, height)) = self.size() else {
            return false;
        };
        let (base, scissor) = self.layout(width, height);
        let Some(view) = &mut self.view else {
            return false;
        };
        let Some(picture) = view.next_picture() else {
            self.pace.undrawn();
            return false;
        };
        // What to draw is only worked out once there is something to draw
        // it on.
        let library = &self.runner.library;
        let commands = self.runner.stage.commands(base, library);
        let target = picture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        view.draw_the_stage(library, &commands, &target, scissor);

        let info = Info {
            stage: &self.runner.stage,
            library,
            stats: view.renderer.stats,
            paused: self.paused,
            frames_per_second: 1.0 / self.frame_time.max(1e-6),
            redraws_to_a_frame: self.pace.in_step(),
            base,
        };
        let actions = view.draw_the_inspector(&mut self.inspector, &info, &target);

        view.renderer.queue.present(picture);
        view.show_the_pointer(&self.runner.stage, self.inspector.open);
        self.drawn += 1;

        for action in actions {
            self.apply(action);
        }
        true
    }

    /// Plays the frames that have fallen due since the last redraw, and
    /// passes what happened in them on to the inspector.
    fn play_frames_due(&mut self) {
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
    }
}

impl View {
    /// The picture the window will show next, to be drawn on. `None` when
    /// there is none to be had this time round.
    fn next_picture(&mut self) -> Option<wgpu::SurfaceTexture> {
        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => Some(texture),
            // The window is out of sight.
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => None,
            // The window changed under us: set the surface up again and
            // draw on the next round.
            _ => {
                self.surface.configure(&self.renderer.device, &self.config);
                None
            }
        }
    }

    /// Draws the stage onto `target`, which is the whole of the window.
    /// Only the pixels inside `scissor`, the rectangle the stage covers,
    /// are drawn.
    fn draw_the_stage(
        &mut self,
        library: &Library,
        commands: &[Command],
        target: &wgpu::TextureView,
        scissor: [u32; 4],
    ) {
        let background = library.background();
        // One point on screen, which is more than one pixel on a dense display.
        self.renderer.min_stroke = self.window.scale_factor() as f32;
        self.renderer.render(
            library,
            commands,
            target,
            (self.config.width, self.config.height),
            background,
            Some(scissor),
        );
    }

    /// Draws the inspector on top of what is on `target`, in a pass of its
    /// own. Returns what was asked for in it.
    fn draw_the_inspector(
        &mut self,
        inspector: &mut Inspector,
        info: &Info<'_>,
        target: &wgpu::TextureView,
    ) -> Vec<Action> {
        let input = self.egui.take_egui_input(&self.window);
        let context = self.egui.egui_ctx().clone();
        let mut actions = Vec::new();
        let output = context.run_ui(input, |ui| {
            actions = inspector.ui(ui.ctx(), info);
        });
        self.egui
            .handle_platform_output(&self.window, output.platform_output);
        let jobs = context.tessellate(output.shapes, output.pixels_per_point);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: output.pixels_per_point,
        };
        self.paint_the_inspector(&jobs, &output.textures_delta, &screen, target);
        actions
    }

    /// Hands the graphics card the inspector's triangles and the textures
    /// they use, and draws them over what is on `target`.
    fn paint_the_inspector(
        &mut self,
        jobs: &[egui::ClippedPrimitive],
        textures: &egui::TexturesDelta,
        screen: &egui_wgpu::ScreenDescriptor,
        target: &wgpu::TextureView,
    ) {
        let (device, queue) = (&self.renderer.device, &self.renderer.queue);
        for (id, deltas) in &textures.set {
            for delta in deltas {
                self.egui_renderer.update_texture(device, queue, *id, delta);
            }
        }
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut buffers =
            self.egui_renderer
                .update_buffers(device, queue, &mut encoder, jobs, screen);
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("inspector"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target,
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
            self.egui_renderer.render(&mut pass, jobs, screen);
        }
        buffers.push(encoder.finish());
        queue.submit(buffers);
        for id in &textures.free {
            self.egui_renderer.free_texture(id);
        }
    }

    /// Shows the pointer as a hand while it is on one of the game's
    /// buttons, and not at all where the game draws one of its own.
    fn show_the_pointer(&self, stage: &Stage, inspecting: bool) {
        let cursor = if stage.pointer.on_button() {
            CursorIcon::Pointer
        } else {
            CursorIcon::Default
        };
        self.window.set_cursor(cursor);
        // Never while the inspector is up: it needs a pointer to be used.
        self.window
            .set_cursor_visible(!stage.hide_pointer || inspecting);
    }
}
