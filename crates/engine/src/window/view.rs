//! What the window is made of: the window itself, the surface that is drawn
//! on, the renderer, and the inspector's own drawing.

use std::sync::Arc;

use anyhow::{Context, Result};
use winit::dpi::LogicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

use super::App;
use crate::gpu::{Renderer, open_device};

/// An open window, and everything that draws to it.
pub(super) struct View {
    pub window: Arc<Window>,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub renderer: Renderer,
    pub egui: egui_winit::State,
    pub egui_renderer: egui_wgpu::Renderer,
}

impl App {
    pub(super) fn open(&self, event_loop: &ActiveEventLoop) -> Result<View> {
        let stage = &self.runner.library.manifest.stage;
        let attributes = Window::default_attributes()
            .with_title(&self.options.title)
            .with_inner_size(LogicalSize::new(stage.width, stage.height));
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .context("opening the window")?,
        );

        let display = Box::new(event_loop.owned_display_handle());
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(display));
        let surface = instance
            .create_surface(window.clone())
            .context("attaching to the window")?;
        let (adapter, device, queue) = open_device(&instance, Some(&surface))?;

        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .context("the window cannot be drawn to")?;
        // Flash blends colours as stored, so prefer a format that does not
        // convert them.
        let formats = surface.get_capabilities(&adapter).formats;
        if let Some(format) = formats.into_iter().find(|format| !format.is_srgb()) {
            config.format = format;
        }
        // Keep one finished picture waiting for the screen and no more. A
        // frame takes a small part of a redraw to draw, so a longer queue
        // would only put more time between a click and what it does.
        config.desired_maximum_frame_latency = 1;
        surface.configure(&device, &config);

        let egui = egui_winit::State::new(
            egui::Context::default(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            config.format,
            egui_wgpu::RendererOptions::default(),
        );

        Ok(View {
            renderer: Renderer::new(device, queue, config.format),
            window,
            surface,
            config,
            egui,
            egui_renderer,
        })
    }
}
