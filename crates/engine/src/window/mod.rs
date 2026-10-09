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

mod app;
mod events;
mod keys;
mod layout;
mod redraw;
mod view;

use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use winit::event_loop::EventLoop;

use crate::app::Runner;
use crate::inspector::Inspector;
use crate::pace::Pace;
use app::App;

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
