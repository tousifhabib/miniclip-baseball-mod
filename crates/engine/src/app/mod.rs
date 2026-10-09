//! Runs a stage together with the game's own rules.
//!
//! The timelines only know how to play. Everything a script did in the
//! original, such as moving between screens, keeping score or reacting to a
//! click, is the job of a [`Logic`]. A [`Runner`] plays the stage frame by
//! frame and hands the logic everything that happens.

mod note;

use std::collections::VecDeque;
use std::sync::Arc;

use crate::audio::Audio;
use crate::display::Event;
use crate::input::{Geometry, Key};
use crate::library::Library;
use crate::stage::Stage;
use note::keep;
pub use note::{MOST_NOTES, Note};

/// The most rounds of "the logic reacts, which causes more events" to follow
/// in one go, in case two handlers keep setting each other off.
const MAX_ROUNDS: usize = 32;

/// The game's rules. Every method has a default that does nothing.
pub trait Logic {
    /// Called once, before the first frame is played.
    fn start(&mut self, _stage: &mut Stage) {}

    /// Called for each thing the stage reports: a button the pointer has
    /// touched, a scripted frame a clip has landed on, or a sound.
    fn event(&mut self, _event: &Event, _stage: &mut Stage) {}

    /// Called once a frame, after the timelines have moved on.
    fn tick(&mut self, _stage: &mut Stage) {}

    /// Called for each key the player presses that no text field has taken.
    /// Returns whether the rules had a use for it.
    fn key(&mut self, _key: &Key, _stage: &mut Stage) -> bool {
        false
    }

    /// A short account of where the game is, for an inspector or a test.
    fn describe(&self) -> String {
        String::new()
    }
}

/// Rules that do nothing: the timelines just play.
pub struct NoLogic;

impl Logic for NoLogic {}

pub struct Runner {
    /// What is being played, which carries the art it is played from.
    pub stage: Stage,
    logic: Box<dyn Logic>,
    /// `None` to play in silence.
    pub audio: Option<Audio>,
    /// How many sounds have been asked for, heard or not.
    pub sounds_asked: u32,
    /// A line for each event since the last call to [`Runner::take_notes`],
    /// for an inspector to show. No more than [`MOST_NOTES`] are kept.
    notes: VecDeque<Note>,
    started: bool,
}

impl Runner {
    pub fn new(stage: Stage, logic: Box<dyn Logic>, audio: Option<Audio>) -> Runner {
        Runner {
            stage,
            logic,
            audio,
            sounds_asked: 0,
            notes: VecDeque::new(),
            started: false,
        }
    }

    /// The art being played, which is the stage's.
    pub fn library(&self) -> &Arc<Library> {
        self.stage.library()
    }

    /// Plays one frame.
    pub fn tick(&mut self, geometry: &mut dyn Geometry) {
        self.ensure_started();
        self.stage.advance(geometry);
        self.react();
        self.logic.tick(&mut self.stage);
        self.react();
        // The frame has been told of any click made before it.
        self.stage.forget_click();
    }

    /// Tells the stage where the pointer is, in stage coordinates, and
    /// whether its button is held.
    pub fn pointer(&mut self, x: f32, y: f32, down: bool, geometry: &mut dyn Geometry) {
        self.ensure_started();
        self.stage.pointer_changed(x, y, down, geometry);
        self.react();
    }

    /// Takes in a key the player has pressed. A text field being typed in
    /// gets it first, and the logic gets it otherwise. Returns whether either
    /// had a use for it.
    pub fn key(&mut self, key: Key) -> bool {
        self.ensure_started();
        let used = self.stage.key(&key) || self.logic.key(&key, &mut self.stage);
        keep(&mut self.notes, Note::Key(key));
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
    /// If nobody has asked for a very long while, only the latest of it.
    pub fn take_notes(&mut self) -> Vec<Note> {
        std::mem::take(&mut self.notes).into()
    }

    fn ensure_started(&mut self) {
        if !self.started {
            self.started = true;
            self.logic.start(&mut self.stage);
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
                            audio.play(self.stage.library(), start);
                        }
                        keep(&mut self.notes, Note::Sound(start.sound));
                    }
                    Event::Button {
                        symbol,
                        path,
                        event,
                    } => {
                        let button = Note::Button {
                            symbol: *symbol,
                            path: path.clone(),
                            event: *event,
                        };
                        keep(&mut self.notes, button);
                    }
                    Event::Frame {
                        symbol,
                        path,
                        frame,
                    } => {
                        let reached = Note::Frame {
                            symbol: *symbol,
                            path: path.clone(),
                            frame: *frame,
                        };
                        keep(&mut self.notes, reached);
                    }
                }
                self.logic.event(event, &mut self.stage);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::note::MOST_NOTES;
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::testing::{Empty, frame, library_with};

    /// The click each frame was told of, if any.
    type Told = Vec<Option<(f32, f32)>>;

    /// Rules that write down the click each frame is told of.
    struct Clicks(Rc<RefCell<Told>>);

    impl Logic for Clicks {
        fn tick(&mut self, stage: &mut Stage) {
            self.0.borrow_mut().push(stage.pointer.went_down);
        }
    }

    /// Rules that do nothing.
    struct Idle;

    impl Logic for Idle {}

    #[test]
    fn a_runner_nobody_asks_keeps_the_latest_of_what_happened_and_no_more() {
        let library = library_with(vec![frame(vec![])], vec![frame(vec![])]);
        let stage = Stage::new(None, library);
        let mut runner = Runner::new(stage, Box::new(Idle), None);
        for _ in 0..10 {
            runner.key(Key::Char('a'));
        }
        for _ in 0..MOST_NOTES {
            runner.key(Key::Char('b'));
        }
        let notes = runner.take_notes();
        assert_eq!(notes.len(), MOST_NOTES);
        // The ten oldest have gone, and what is left is in order.
        assert!(notes.iter().all(|note| *note == Note::Key(Key::Char('b'))));
        // Taken, they are gone, and a few more are kept as they come.
        runner.key(Key::Enter);
        assert_eq!(runner.take_notes(), [Note::Key(Key::Enter)]);
    }

    #[test]
    fn a_frame_is_told_of_a_click_made_before_it_and_the_next_is_not() {
        let library = library_with(vec![frame(vec![])], vec![frame(vec![])]);
        let stage = Stage::new(None, library);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let mut runner = Runner::new(stage, Box::new(Clicks(seen.clone())), None);
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
}
