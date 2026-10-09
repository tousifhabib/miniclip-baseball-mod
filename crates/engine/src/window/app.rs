//! The window's own state: the game being run, where it is drawn, and
//! whether it is paused.

use std::time::{Duration, Instant};

use super::Options;
use crate::app::Runner;
use crate::inspector::{Action, Inspector};
use crate::math::Matrix;
use crate::pace::Pace;
use crate::window::layout::stage_in_window;
use crate::window::view::View;

pub(super) struct App {
    pub(super) runner: Runner,
    pub(super) options: Options,
    pub(super) inspector: Inspector,
    pub(super) paused: bool,
    pub(super) last_redraw: Instant,
    /// How long a frame of the game lasts.
    pub(super) frame: Duration,
    /// When the game's frames are played.
    pub(super) pace: Pace,
    /// A running average of the time between redraws.
    pub(super) frame_time: f32,
    pub(super) drawn: u32,
    /// Where the pointer is, in window pixels, and whether its button is held.
    pub(super) cursor: (f32, f32),
    pub(super) pressed: bool,
    pub(super) view: Option<View>,
    pub(super) error: Option<anyhow::Error>,
}

impl App {
    /// How the stage sits in a window of this size: the transform from stage
    /// coordinates to window pixels, and the rectangle the stage covers.
    pub(super) fn layout(&self, width: u32, height: u32) -> (Matrix, [u32; 4]) {
        let stage = &self.runner.library().manifest.stage;
        stage_in_window(stage, self.options.centre_origin, width, height)
    }

    /// Tells the stage where the pointer now is.
    pub(super) fn pointer_changed(&mut self) {
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

    pub(super) fn size(&self) -> Option<(u32, u32)> {
        self.view
            .as_ref()
            .map(|view| (view.config.width, view.config.height))
    }

    pub(super) fn apply(&mut self, action: Action) {
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
                stage.goto_clip(&path, frame);
                // Hold it there, or it would play straight on.
                if let Some(clip) = stage.clip_mut(&path) {
                    clip.playing = false;
                }
                self.runner.settle();
            }
        }
    }

    /// Stops the game where it is, or lets it go on.
    pub(super) fn toggle_pause(&mut self) {
        self.paused = !self.paused;
        // A click made while it was stopped was not meant for the game as
        // it goes on. One made for a single step still is.
        if !self.paused {
            self.runner.stage.forget_click();
        }
    }

    /// Plays one frame.
    pub(super) fn step(&mut self) {
        if let Some(view) = &mut self.view {
            self.runner.tick(&mut view.renderer);
        }
    }
}
