//! Runs a [`Runner`] in a window, with sound, a working pointer, and the
//! inspector.
//!
//! F1 opens the inspector, F2 pauses, and F3 steps one frame while paused.
//! Every other key goes to the game. If the game has no use for it, Space
//! pauses, the right arrow steps one frame while paused, and Escape quits.
//!
//! What the window is made of is in `view`, how the stage sits in it in
//! `layout`, which of its keys are which of the game's in `keys`, and the
//! drawing of a frame in `redraw`.

mod keys;
mod layout;
mod redraw;
mod view;

use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::WindowId;

use crate::app::Runner;
use crate::inspector::{Action, Inspector};
use crate::math::Matrix;
use crate::pace::Pace;
use keys::{held, typed};
use layout::stage_in_window;
use view::View;

pub struct Options {
    pub title: String,
    /// Open with the inspector showing.
    pub inspect: bool,
    /// Put the top clip's origin at the centre of the window, for looking at
    /// one clip on its own.
    pub centre_origin: bool,
    /// Quit after drawing this many frames. For testing.
    pub exit_after: Option<u32>,
}

/// What a run of the window amounted to.
pub struct Summary {
    pub frames_drawn: u32,
    pub sounds_asked: u32,
    /// Things that could not be drawn or played.
    pub problems: Vec<String>,
}

/// Opens a window and plays until it is closed.
pub fn run(runner: Runner, options: Options) -> Result<Summary> {
    let mut inspector = Inspector::new();
    inspector.open = options.inspect;
    let frame = Duration::from_secs_f64(1.0 / runner.library.manifest.stage.frame_rate);
    let mut app = App {
        runner,
        options,
        inspector,
        paused: false,
        last_redraw: Instant::now(),
        frame,
        pace: Pace::new(frame),
        frame_time: 1.0 / 60.0,
        drawn: 0,
        cursor: (0.0, 0.0),
        pressed: false,
        view: None,
        error: None,
    };
    let event_loop = EventLoop::new().context("starting the window system")?;
    event_loop.run_app(&mut app).context("running the window")?;
    if let Some(error) = app.error {
        return Err(error);
    }

    let audio_problems = app.runner.audio.iter().flat_map(|audio| &audio.problems);
    let draw_problems = app.view.iter().flat_map(|view| view.renderer.problems());
    Ok(Summary {
        frames_drawn: app.drawn,
        sounds_asked: app.runner.sounds_asked,
        problems: draw_problems.chain(audio_problems).cloned().collect(),
    })
}

struct App {
    runner: Runner,
    options: Options,
    inspector: Inspector,
    paused: bool,
    last_redraw: Instant,
    /// How long a frame of the game lasts.
    frame: Duration,
    /// When the game's frames are played.
    pace: Pace,
    /// A running average of the time between redraws.
    frame_time: f32,
    drawn: u32,
    /// Where the pointer is, in window pixels, and whether its button is held.
    cursor: (f32, f32),
    pressed: bool,
    view: Option<View>,
    error: Option<anyhow::Error>,
}

impl App {
    /// How the stage sits in a window of this size: the transform from stage
    /// coordinates to window pixels, and the rectangle the stage covers.
    fn layout(&self, width: u32, height: u32) -> (Matrix, [u32; 4]) {
        let stage = &self.runner.library.manifest.stage;
        stage_in_window(stage, self.options.centre_origin, width, height)
    }

    /// Tells the stage where the pointer now is.
    fn pointer_changed(&mut self) {
        let Some((width, height)) = self.size() else {
            return;
        };
        let (base, _) = self.layout(width, height);
        let Some(inverse) = base.inverse() else {
            return;
        };
        let (x, y) = inverse.apply(self.cursor.0, self.cursor.1);
        if let Some(view) = &mut self.view {
            self.runner.pointer(x, y, self.pressed, &mut view.renderer);
        }
    }

    fn size(&self) -> Option<(u32, u32)> {
        self.view
            .as_ref()
            .map(|view| (view.config.width, view.config.height))
    }

    fn apply(&mut self, action: Action) {
        let stage = &mut self.runner.stage;
        match action {
            Action::TogglePause => self.toggle_pause(),
            Action::Step => self.step(),
            Action::SetPlaying(path, playing) => {
                if let Some(clip) = stage.clip_mut(&path) {
                    clip.playing = playing;
                }
            }
            Action::SetVisible(path, visible) => {
                // Held, so that it stays hidden when its timeline loops.
                if let Some(child) = stage.child_mut(&path) {
                    child.set_visible(visible);
                }
            }
            Action::Goto(path, frame) => {
                stage.goto_clip(&path, frame, &self.runner.library);
                // Hold it there, or it would play straight on.
                if let Some(clip) = stage.clip_mut(&path) {
                    clip.playing = false;
                }
                self.runner.settle();
            }
        }
    }

    /// Stops the game where it is, or lets it go on.
    fn toggle_pause(&mut self) {
        self.paused = !self.paused;
        // A click made while it was stopped was not meant for the game as
        // it goes on. One made for a single step still is.
        if !self.paused {
            self.runner.stage.forget_click();
        }
    }

    /// Plays one frame.
    fn step(&mut self) {
        if let Some(view) = &mut self.view {
            self.runner.tick(&mut view.renderer);
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.view.is_some() {
            return;
        }
        match self.open(event_loop) {
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
            WindowEvent::Resized(size) => {
                if let Some(view) = &mut self.view {
                    view.config.width = size.width.max(1);
                    view.config.height = size.height.max(1);
                    view.surface.configure(&view.renderer.device, &view.config);
                    view.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
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
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as f32, position.y as f32);
                self.pointer_changed();
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                // A release always goes through, so that a press which began
                // on the game is not left hanging.
                let down = state == ElementState::Pressed;
                if !down || !taken {
                    self.pressed = down;
                    self.pointer_changed();
                }
            }
            // A key that comes up always goes through, so that one which
            // went down on the game is not left held.
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Released => {
                for key in held(&event.logical_key) {
                    self.runner.hold(key, false);
                }
            }
            WindowEvent::Focused(false) => self.runner.stage.keys_let_go(),
            WindowEvent::KeyboardInput { event, .. }
                if !taken && event.state == ElementState::Pressed =>
            {
                for key in held(&event.logical_key) {
                    self.runner.hold(key, true);
                }
                match &event.logical_key {
                    // The window's own keys, which no game is offered.
                    Key::Named(NamedKey::F1) => self.inspector.open = !self.inspector.open,
                    Key::Named(NamedKey::F2) => self.toggle_pause(),
                    Key::Named(NamedKey::F3) => {
                        if self.paused {
                            self.step();
                        }
                    }
                    logical => {
                        let mut used = false;
                        for key in typed(logical, event.text.as_deref()) {
                            used |= self.runner.key(key);
                        }
                        if !used {
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
            _ => {}
        }
    }
}
