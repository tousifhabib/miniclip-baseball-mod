//! What the window does about each thing the system tells it: a resize,
//! a redraw, the pointer, a key.

use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{Key, NamedKey};
use winit::window::WindowId;

use super::app::App;
use crate::window::keys::{held, typed};
use crate::window::view::View;

impl App {
    /// The window has a new size: the surface is set up for it, and the
    /// stage drawn again.
    fn resized(&mut self, size: PhysicalSize<u32>) {
        if let Some(view) = &mut self.view {
            view.config.width = size.width.max(1);
            view.config.height = size.height.max(1);
            view.surface.configure(&view.renderer.device, &view.config);
            view.window.request_redraw();
        }
    }

    /// The window is to be drawn. Draws it, unless it has been drawn as
    /// often as was asked for, and says when it is to be drawn next.
    fn redraw_requested(&mut self, event_loop: &ActiveEventLoop) {
        if self
            .options
            .exit_after
            .is_some_and(|limit| self.drawn >= limit)
        {
            event_loop.exit();
            return;
        }
        if self.redraw() {
            // Drawing waits for the screen, so the next redraw can
            // be asked for at once.
            event_loop.set_control_flow(ControlFlow::Wait);
            if let Some(view) = &self.view {
                view.window.request_redraw();
            }
        } else {
            // Nothing was drawn: the window is hidden or covered,
            // and there is no screen to wait for. Asked again at
            // once it would be answered at once, over and over, so
            // the next is left until the game's next frame is due.
            let next = self.last_redraw + self.frame;
            event_loop.set_control_flow(ControlFlow::WaitUntil(next));
        }
    }

    /// The pointer's button has gone down or come up. `taken` is whether
    /// the inspector has kept the click for itself.
    fn button_changed(&mut self, down: bool, taken: bool) {
        // A release always goes through, so that a press which began
        // on the game is not left hanging.
        if !down || !taken {
            self.pressed = down;
            self.pointer_changed();
        }
    }

    /// A key has come up, and is no longer held as far as the game knows.
    fn key_released(&mut self, logical: &Key) {
        for key in held(logical) {
            self.runner.hold(key, false);
        }
    }

    /// A key has gone down. `text` is what the window system says the press
    /// typed, if anything.
    fn key_pressed(&mut self, event_loop: &ActiveEventLoop, logical: &Key, text: Option<&str>) {
        for key in held(logical) {
            self.runner.hold(key, true);
        }
        match logical {
            // The window's own keys, which no game is offered.
            Key::Named(NamedKey::F1) => self.inspector.open = !self.inspector.open,
            Key::Named(NamedKey::F2) => self.toggle_pause(),
            Key::Named(NamedKey::F3) => {
                if self.paused {
                    self.step();
                }
            }
            logical => {
                if !self.offer_to_the_game(logical, text) {
                    match logical {
                        Key::Named(NamedKey::Space) => self.toggle_pause(),
                        Key::Named(NamedKey::ArrowRight) if self.paused => self.step(),
                        Key::Named(NamedKey::Escape) => event_loop.exit(),
                        _ => {}
                    }
                }
            }
        }
    }

    /// Gives the game what a press of this key types. Returns whether the
    /// game had a use for any of it.
    fn offer_to_the_game(&mut self, logical: &Key, text: Option<&str>) -> bool {
        let mut used = false;
        for key in typed(logical, text) {
            used |= self.runner.key(key);
        }
        used
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.view.is_some() {
            return;
        }
        let stage = &self.runner.library().manifest.stage;
        match View::open(event_loop, &self.options.title, stage) {
            Ok(view) => {
                view.window.request_redraw();
                self.last_redraw = Instant::now();
                self.view = Some(view);
            }
            Err(error) => {
                self.error = Some(error);
                event_loop.exit();
            }
        }
    }

    fn new_events(&mut self, _: &ActiveEventLoop, cause: StartCause) {
        // The wait after a redraw that drew nothing is over.
        if matches!(cause, StartCause::ResumeTimeReached { .. })
            && let Some(view) = &self.view
        {
            view.window.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        // The inspector sees every event first, and keeps the ones meant for
        // it: a click on its panel must not reach the game underneath.
        let taken = match &mut self.view {
            Some(view) if self.inspector.open => {
                view.egui.on_window_event(&view.window, &event).consumed
            }
            _ => false,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => self.resized(size),
            WindowEvent::RedrawRequested => self.redraw_requested(event_loop),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as f32, position.y as f32);
                self.pointer_changed();
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => self.button_changed(state == ElementState::Pressed, taken),
            // A key that comes up always goes through, so that one which
            // went down on the game is not left held.
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Released => {
                self.key_released(&event.logical_key);
            }
            WindowEvent::Focused(false) => self.runner.stage.keys_let_go(),
            WindowEvent::KeyboardInput { event, .. }
                if !taken && event.state == ElementState::Pressed =>
            {
                self.key_pressed(event_loop, &event.logical_key, event.text.as_deref());
            }
            _ => {}
        }
    }
}
