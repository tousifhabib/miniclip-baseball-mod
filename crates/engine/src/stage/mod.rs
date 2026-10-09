//! A running movie: the display tree, the pointer, what the text fields say,
//! and a record of what has happened for the caller to act on.
//!
//! The stage carries the art it plays from. Whoever is handed the stage
//! can steer it, and can ask it for the art, without being handed the art
//! beside it.
//!
//! Finding a thing on the stage is in `find`, and what the rules do to
//! one in `steer`. The sounds the rules ask for are in `sound`, and the
//! keys and typing into a text field are in `typing`.

mod find;
mod sound;
mod steer;
mod typing;

use std::sync::Arc;

use bb_format::SymbolId;

use crate::display::{ClipState, Command, Event, Path, Texts, commands_upright, text_key};
use crate::input::{Geometry, Key, Pointer};
use crate::library::Library;
use crate::math::Matrix;

pub use crate::display::CARET;
pub use typing::Focus;

pub struct Stage {
    /// The art the stage plays from. Nothing changes it once it is loaded,
    /// so several stages may play from one copy of it.
    library: Arc<Library>,
    pub root: ClipState,
    pub pointer: Pointer,
    /// What the text fields say, by the variable each one shows.
    pub texts: Texts,
    /// The text field that typing goes to, if the player has clicked on one.
    pub focus: Option<Focus>,
    /// The game wants the system's pointer out of sight, because it is
    /// drawing something of its own where the pointer is.
    pub hide_pointer: bool,
    /// The keys the player is holding down.
    keys_down: Vec<Key>,
    /// Whether what is written is drawn the right way round even where the
    /// game has mirrored the clip it is in.
    pub upright_text: bool,
    /// Frames played, for blinking the caret.
    ticks: u32,
    /// How loud each sound is to be played, where the game has said.
    levels: std::collections::HashMap<SymbolId, f32>,
    events: Vec<Event>,
}

impl Stage {
    /// Starts playing a clip of this art, or its main timeline for `None`.
    pub fn new(clip: Option<SymbolId>, library: impl Into<Arc<Library>>) -> Stage {
        let library = library.into();
        let mut events = Vec::new();
        Stage {
            root: ClipState::new(clip, &library, &mut events, &mut Path::new()),
            library,
            pointer: Pointer::default(),
            texts: Texts::new(),
            focus: None,
            hide_pointer: false,
            keys_down: Vec::new(),
            upright_text: false,
            ticks: 0,
            levels: std::collections::HashMap::new(),
            events,
        }
    }

    /// The lowest depth for objects the rules add. It is above every depth
    /// a timeline uses, as the depths `attachMovie` gave were in Flash.
    pub const RULES_DEPTH: u16 = 16384;

    /// The art the stage plays from. It is the stage's to hand out: whoever
    /// has the stage has no need to be handed the art beside it.
    pub fn library(&self) -> &Arc<Library> {
        &self.library
    }

    /// Plays one frame.
    pub fn advance(&mut self, geometry: &mut dyn Geometry) {
        self.ticks = self.ticks.wrapping_add(1);
        self.root
            .advance(&self.library, &mut self.events, &mut Path::new());
        self.end_typing_if_its_field_has_gone();
        // Things have moved, so what is under the pointer may have changed.
        let (x, y, down) = (self.pointer.x, self.pointer.y, self.pointer.down);
        self.pointer_changed(x, y, down, geometry);
    }

    /// Tells the stage where the pointer is, in the top timeline's
    /// coordinates, and whether its button is held.
    pub fn pointer_changed(&mut self, x: f32, y: f32, down: bool, geometry: &mut dyn Geometry) {
        let pressed = down && !self.pointer.down;
        self.pointer.update(
            &mut self.root,
            (x, y),
            down,
            &self.library,
            geometry,
            &mut self.events,
        );
        if pressed {
            self.give_the_typing_to_what_was_pressed(x, y);
        }
    }

    /// What a text field showing `variable` says now.
    pub fn text(&self, variable: &str) -> Option<&str> {
        self.texts.get(text_key(variable)).map(String::as_str)
    }

    /// Forgets a click that was being kept for the next frame to act on:
    /// it has been acted on, or was never meant for the game.
    pub fn forget_click(&mut self) {
        self.pointer.went_down = None;
    }

    /// Adds something for the caller to act on, as if the timelines had
    /// reported it.
    fn push_event(&mut self, event: Event) {
        self.events.push(event);
    }

    /// Lists what to draw, back to front.
    pub fn commands(&self, base: Matrix) -> Vec<Command> {
        let upright = self.upright_text;
        let mut list = commands_upright(&self.root, base, &self.library, &self.texts, upright);
        self.add_the_caret(&mut list);
        list
    }

    /// What has happened since the last call: sounds to play, buttons the
    /// pointer has touched, and scripted frames that clips have landed on.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
}

#[cfg(test)]
mod tests {
    use bb_format::{Op, Place, PlaceAction};

    use super::*;
    use crate::testing::{INNER, SHAPE, frame, library_with, place, put};

    #[test]
    fn a_point_is_carried_into_and_out_of_a_nested_clip() {
        // A clip at (100, 50), twice the size, holding a shape at (10, 10).
        let outer = Place {
            matrix: Some([2.0, 0.0, 0.0, 2.0, 100.0, 50.0]),
            ..place(3, PlaceAction::Place(INNER))
        };
        let inner = Place {
            matrix: Some([1.0, 0.0, 0.0, 1.0, 10.0, 10.0]),
            ..place(1, PlaceAction::Place(SHAPE))
        };
        let library = library_with(
            vec![frame(vec![Op::Place(Box::new(outer))])],
            vec![frame(vec![Op::Place(Box::new(inner))])],
        );
        let stage = Stage::new(None, library);
        assert_eq!(stage.to_stage(&[]), Some(Matrix::IDENTITY));
        assert_eq!(stage.to_stage(&[3]).unwrap().apply(0.0, 0.0), (100.0, 50.0));
        assert_eq!(
            stage.to_stage(&[3, 1]).unwrap().apply(0.0, 0.0),
            (120.0, 70.0)
        );
        assert_eq!(stage.from_stage(&[3], 120.0, 70.0), Some((10.0, 10.0)));
        assert_eq!(stage.to_stage(&[9]), None);
    }

    #[test]
    fn the_rules_can_add_an_object_and_take_it_away() {
        let library = library_with(vec![frame(vec![put(1, SHAPE)])], vec![frame(vec![])]);
        let mut stage = Stage::new(None, library);
        let path = stage
            .attach(&[], INNER, Stage::RULES_DEPTH, "extra")
            .unwrap();
        assert_eq!(path, [Stage::RULES_DEPTH]);
        assert_eq!(stage.find_named(&[], "extra"), Some(path.clone()));
        assert!(stage.remove(&path));
        assert!(!stage.remove(&path));
        assert_eq!(stage.find_named(&[], "extra"), None);
        // There is no such clip to add to.
        assert_eq!(stage.attach(&[7], INNER, 1, "lost"), None);
    }
}
