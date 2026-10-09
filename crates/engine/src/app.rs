//! Runs a stage together with the game's own rules.
//!
//! The timelines only know how to play. Everything a script did in the
//! original, such as moving between screens, keeping score or reacting to a
//! click, is the job of a [`Logic`]. A [`Runner`] plays the stage frame by
//! frame and hands the logic everything that happens.

use std::sync::Arc;

use bb_format::SymbolId;

use crate::audio::Audio;
use crate::display::{ButtonEvent, Event, Path};
use crate::input::{Geometry, Key};
use crate::library::Library;
use crate::stage::Stage;

/// The most rounds of "the logic reacts, which causes more events" to follow
/// in one go, in case two handlers keep setting each other off.
const MAX_ROUNDS: usize = 32;

/// The game's rules. Every method has a default that does nothing.
pub trait Logic {
    /// Called once, before the first frame is played.
    fn start(&mut self, _stage: &mut Stage, _library: &Library) {}

    /// Called for each thing the stage reports: a button the pointer has
    /// touched, a scripted frame a clip has landed on, or a sound.
    fn event(&mut self, _event: &Event, _stage: &mut Stage, _library: &Library) {}

    /// Called once a frame, after the timelines have moved on.
    fn tick(&mut self, _stage: &mut Stage, _library: &Library) {}

    /// Called for each key the player presses that no text field has taken.
    /// Returns whether the rules had a use for it.
    fn key(&mut self, _key: &Key, _stage: &mut Stage, _library: &Library) -> bool {
        false
    }

    /// A short account of where the game is, for an inspector or a test.
    fn describe(&self) -> String {
        String::new()
    }
}

/// Something the runner saw happen, kept for an inspector, a script or a
/// test to read back. Each prints as a line that says what it was.
#[derive(Clone, Debug, PartialEq)]
pub enum Note {
    /// The player pressed a key.
    Key(Key),
    /// A sound was asked for, by a timeline, a button or the rules.
    Sound(SymbolId),
    /// The pointer did something to a button.
    Button {
        symbol: SymbolId,
        path: Path,
        event: ButtonEvent,
    },
    /// A clip landed on a frame where the original had a script. `symbol`
    /// is `None` for the main timeline.
    Frame {
        symbol: Option<SymbolId>,
        path: Path,
        frame: u16,
    },
}

impl std::fmt::Display for Note {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Note::Key(key) => write!(out, "key {key:?}"),
            Note::Sound(sound) => write!(out, "sound {sound}"),
            Note::Button {
                symbol,
                path,
                event,
            } => write!(out, "button {symbol} at {path:?}: {event:?}"),
            Note::Frame {
                symbol: Some(clip),
                path,
                frame,
            } => write!(out, "clip {clip} at {path:?}: frame {frame}"),
            Note::Frame {
                symbol: None,
                path,
                frame,
            } => write!(out, "main timeline at {path:?}: frame {frame}"),
        }
    }
}

/// Rules that do nothing: the timelines just play.
pub struct NoLogic;

impl Logic for NoLogic {}

pub struct Runner {
    /// The art, which nothing changes once it is loaded, so that several
    /// games may be played from one copy of it.
    pub library: Arc<Library>,
    pub stage: Stage,
    logic: Box<dyn Logic>,
    /// `None` to play in silence.
    pub audio: Option<Audio>,
    /// How many sounds have been asked for, heard or not.
    pub sounds_asked: u32,
    /// A line for each event since the last call to [`Runner::take_notes`],
    /// for an inspector to show.
    notes: Vec<Note>,
    started: bool,
}

impl Runner {
    pub fn new(
        library: impl Into<Arc<Library>>,
        stage: Stage,
        logic: Box<dyn Logic>,
        audio: Option<Audio>,
    ) -> Runner {
        Runner {
            library: library.into(),
            stage,
            logic,
            audio,
            sounds_asked: 0,
            notes: Vec::new(),
            started: false,
        }
    }

    /// Plays one frame.
    pub fn tick(&mut self, geometry: &mut dyn Geometry) {
        self.ensure_started();
        self.stage.advance(&self.library, geometry);
        self.react();
        self.logic.tick(&mut self.stage, &self.library);
        self.react();
        // The frame has been told of any click made before it.
        self.stage.forget_click();
    }

    /// Tells the stage where the pointer is, in stage coordinates, and
    /// whether its button is held.
    pub fn pointer(&mut self, x: f32, y: f32, down: bool, geometry: &mut dyn Geometry) {
        self.ensure_started();
        self.stage
            .pointer_changed(x, y, down, &self.library, geometry);
        self.react();
    }

    /// Takes in a key the player has pressed. A text field being typed in
    /// gets it first, and the logic gets it otherwise. Returns whether either
    /// had a use for it.
    pub fn key(&mut self, key: Key) -> bool {
        self.ensure_started();
        let used = self.stage.key(&key, &self.library)
            || self.logic.key(&key, &mut self.stage, &self.library);
        self.notes.push(Note::Key(key));
        self.react();
        used
    }

    /// Takes in that a key has gone down or come up again, for rules that
    /// care how long one is held. The logic is not told: it asks the stage
    /// whether a key is down when it wants to know.
    pub fn hold(&mut self, key: Key, down: bool) {
        self.stage.key_changed(key, down);
    }

    /// Lets the logic act on anything that has happened without playing a
    /// frame, for instance after an inspector has moved a clip.
    pub fn settle(&mut self) {
        self.ensure_started();
        self.react();
    }

    /// The logic's own account of where the game is.
    pub fn describe(&self) -> String {
        self.logic.describe()
    }

    /// What has happened since this was last asked, in the order it did.
    pub fn take_notes(&mut self) -> Vec<Note> {
        std::mem::take(&mut self.notes)
    }

    fn ensure_started(&mut self) {
        if !self.started {
            self.started = true;
            self.logic.start(&mut self.stage, &self.library);
            self.react();
        }
    }

    /// Hands the logic every waiting event, and then the events its
    /// reactions cause, until things are quiet.
    fn react(&mut self) {
        for _ in 0..MAX_ROUNDS {
            let events = self.stage.take_events();
            if events.is_empty() {
                return;
            }
            for event in &events {
                match event {
                    Event::Sound(start) => {
                        self.sounds_asked += 1;
                        if let Some(audio) = &mut self.audio {
                            audio.play(&self.library, start);
                        }
                        self.notes.push(Note::Sound(start.sound));
                    }
                    Event::Button {
                        symbol,
                        path,
                        event,
                    } => self.notes.push(Note::Button {
                        symbol: *symbol,
                        path: path.clone(),
                        event: *event,
                    }),
                    Event::Frame {
                        symbol,
                        path,
                        frame,
                    } => self.notes.push(Note::Frame {
                        symbol: *symbol,
                        path: path.clone(),
                        frame: *frame,
                    }),
                }
                self.logic.event(event, &mut self.stage, &self.library);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::testing::{Empty, frame, library_with};

    /// The click each frame was told of, if any.
    type Told = Vec<Option<(f32, f32)>>;

    /// Rules that write down the click each frame is told of.
    struct Clicks(Rc<RefCell<Told>>);

    impl Logic for Clicks {
        fn tick(&mut self, stage: &mut Stage, _: &Library) {
            self.0.borrow_mut().push(stage.pointer.went_down);
        }
    }

    #[test]
    fn a_frame_is_told_of_a_click_made_before_it_and_the_next_is_not() {
        let library = library_with(vec![frame(vec![])], vec![frame(vec![])]);
        let stage = Stage::new(None, &library);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let mut runner = Runner::new(library, stage, Box::new(Clicks(seen.clone())), None);
        runner.tick(&mut Empty);
        // The button goes down and comes up before a frame is played.
        runner.pointer(30.0, 40.0, false, &mut Empty);
        runner.pointer(30.0, 40.0, true, &mut Empty);
        runner.pointer(31.0, 41.0, false, &mut Empty);
        runner.tick(&mut Empty);
        runner.tick(&mut Empty);
        // A button held down over several frames is one click.
        runner.pointer(50.0, 60.0, true, &mut Empty);
        runner.tick(&mut Empty);
        runner.tick(&mut Empty);
        let told = [None, Some((30.0, 40.0)), None, Some((50.0, 60.0)), None];
        assert_eq!(*seen.borrow(), told);
    }

    #[test]
    fn a_note_prints_as_the_line_it_always_was() {
        // Scripts and tests read these lines, so they are kept to the letter.
        let lines = [
            (Note::Key(Key::Char('a')), "key Char('a')"),
            (Note::Key(Key::Enter), "key Enter"),
            (Note::Sound(84), "sound 84"),
            (
                Note::Button {
                    symbol: 1618,
                    path: vec![3, 16384],
                    event: ButtonEvent::Release,
                },
                "button 1618 at [3, 16384]: Release",
            ),
            (
                Note::Frame {
                    symbol: Some(2027),
                    path: vec![1],
                    frame: 91,
                },
                "clip 2027 at [1]: frame 91",
            ),
            (
                Note::Frame {
                    symbol: None,
                    path: vec![],
                    frame: 2,
                },
                "main timeline at []: frame 2",
            ),
        ];
        for (note, line) in lines {
            assert_eq!(note.to_string(), line);
        }
    }
}
