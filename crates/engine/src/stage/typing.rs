//! The keys: which of them are held down, and typing into a text field.
//! Typing is which field has it, what a key does to what the field says,
//! and the caret that shows where the next letter goes.

use bb_format::SymbolId;

use super::Stage;
use crate::display::{CARET, Command, Path, text_key};
use crate::input::{Key, field_at};
use crate::library::Library;

/// A text field the player is typing in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Focus {
    pub path: Path,
    pub symbol: SymbolId,
}

/// How many frames the caret shows for, and then hides for.
const BLINK: u32 = 30;

impl Stage {
    /// Whether the player is holding this key down.
    pub fn key_down(&self, key: Key) -> bool {
        self.keys_down.contains(&key)
    }

    /// Takes in that a key has gone down, or has come up again.
    pub fn key_changed(&mut self, key: Key, down: bool) {
        self.keys_down.retain(|held| *held != key);
        if down {
            self.keys_down.push(key);
        }
    }

    /// No key is being held down any more, as far as can be told: the
    /// window has lost the keyboard, say.
    pub fn keys_let_go(&mut self) {
        self.keys_down.clear();
    }

    /// Takes in a key the player has pressed. Returns whether a text field
    /// used it, so that the caller knows not to act on it too.
    pub fn key(&mut self, key: &Key, library: &Library) -> bool {
        let Some(focus) = &self.focus else {
            return false;
        };
        let Some(field) = library.edit_texts.get(&focus.symbol) else {
            return false;
        };
        let variable = text_key(&field.variable).to_owned();
        let mut said = self
            .texts
            .get(&variable)
            .cloned()
            .or_else(|| field.initial_text.clone())
            .unwrap_or_default();
        match key {
            Key::Char(c) => {
                let room = field
                    .max_length
                    .is_none_or(|most| said.chars().count() < usize::from(most));
                // A field can only show the letters its font has.
                let drawable = field
                    .font
                    .and_then(|font| library.fonts.get(&font))
                    .is_none_or(|font| font.glyphs.iter().any(|glyph| glyph.char.starts_with(*c)));
                if room && drawable && !c.is_control() {
                    said.push(*c);
                }
            }
            Key::Backspace => {
                said.pop();
            }
            Key::Enter | Key::Escape | Key::Tab => {
                self.focus = None;
                return true;
            }
            // Other keys are not for the field, but while it has the typing
            // they are not for anything else either.
            _ => return true,
        }
        self.texts.insert(variable, said);
        self.ticks = 0;
        true
    }

    /// A field that has gone takes the typing with it.
    pub(super) fn end_typing_if_its_field_has_gone(&mut self) {
        if let Some(focus) = &self.focus
            && self
                .child(&focus.path)
                .is_none_or(|child| child.symbol != focus.symbol)
        {
            self.focus = None;
        }
    }

    /// A press on a field that can be typed in gives it the typing, and a
    /// press anywhere else takes the typing away. `x` and `y` are where the
    /// press was, in the top timeline's coordinates.
    pub(super) fn give_the_typing_to_what_was_pressed(
        &mut self,
        x: f32,
        y: f32,
        library: &Library,
    ) {
        self.focus = if self.pointer.on_button() {
            None
        } else {
            field_at(&self.root.children, x, y, library, &mut Path::new())
                .map(|(path, symbol)| Focus { path, symbol })
        };
        self.ticks = 0;
    }

    /// The field being typed in shows a caret after its text, on and off:
    /// it is added to what the draws of that field in `list` say.
    pub(super) fn add_the_caret(&self, list: &mut [Command], library: &Library) {
        if let Some(focus) = &self.focus
            && (self.ticks / BLINK).is_multiple_of(2)
            && let Some(field) = library.edit_texts.get(&focus.symbol)
        {
            for command in list {
                if let Command::Draw { symbol, text, .. } = command
                    && *symbol == focus.symbol
                {
                    let mut said = text
                        .take()
                        .or_else(|| field.initial_text.clone())
                        .unwrap_or_default();
                    said.push(CARET);
                    *text = Some(said);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bb_format::{FieldFlag, Op, Place, PlaceAction};

    use super::*;
    use crate::math::Matrix;
    use crate::testing::{Empty, FIELD, add_field, frame, library_with, place};

    fn click(stage: &mut Stage, library: &Library, x: f32, y: f32) {
        stage.pointer_changed(x, y, false, library, &mut Empty);
        stage.pointer_changed(x, y, true, library, &mut Empty);
        stage.pointer_changed(x, y, false, library, &mut Empty);
    }

    /// A main timeline with a text field at (100, 100) that can be typed in.
    fn with_field() -> Library {
        let at = Place {
            matrix: Some([1.0, 0.0, 0.0, 1.0, 100.0, 100.0]),
            ..place(1, PlaceAction::Place(FIELD))
        };
        let mut library = library_with(vec![frame(vec![Op::Place(Box::new(at))])], vec![]);
        add_field(&mut library, "_root.teamName", &[]);
        library.edit_texts.get_mut(&FIELD).unwrap().initial_text = None;
        library.edit_texts.get_mut(&FIELD).unwrap().max_length = Some(5);
        library
    }

    #[test]
    fn typing_goes_to_the_field_that_was_clicked() {
        let library = with_field();
        let mut stage = Stage::new(None, &library);
        // Nothing has the typing yet, so the key is free for the rules.
        assert!(!stage.key(&Key::Char('a'), &library));
        assert_eq!(stage.text("teamName"), None);

        click(&mut stage, &library, 110.0, 110.0);
        assert_eq!(stage.focus.as_ref().map(|focus| focus.symbol), Some(FIELD));
        for c in "abcdefg".chars() {
            assert!(stage.key(&Key::Char(c), &library));
        }
        // The field holds five letters at most.
        assert_eq!(stage.text("teamName"), Some("abcde"));
        stage.key(&Key::Backspace, &library);
        assert_eq!(stage.text("teamName"), Some("abcd"));

        // A click anywhere else ends the typing.
        click(&mut stage, &library, 10.0, 10.0);
        assert_eq!(stage.focus, None);
        assert!(!stage.key(&Key::Char('z'), &library));
        assert_eq!(stage.text("teamName"), Some("abcd"));
    }

    #[test]
    fn a_field_that_only_shows_text_cannot_be_typed_in() {
        let mut library = with_field();
        library.edit_texts.get_mut(&FIELD).unwrap().flags = vec![FieldFlag::ReadOnly];
        let mut stage = Stage::new(None, &library);
        click(&mut stage, &library, 110.0, 110.0);
        assert_eq!(stage.focus, None);
    }

    #[test]
    fn the_field_being_typed_in_shows_a_caret_that_blinks() {
        let library = with_field();
        let mut stage = Stage::new(None, &library);
        click(&mut stage, &library, 110.0, 110.0);
        stage.key(&Key::Char('a'), &library);
        let said = |stage: &Stage| match &stage.commands(Matrix::IDENTITY, &library)[0] {
            Command::Draw { text, .. } => text.clone(),
            other => panic!("expected a draw, found {other:?}"),
        };
        assert_eq!(said(&stage), Some(format!("a{CARET}")));
        for _ in 0..BLINK {
            stage.advance(&library, &mut Empty);
        }
        assert_eq!(said(&stage), Some("a".to_owned()));
        // The caret is only drawn: it is no part of what the field says.
        assert_eq!(stage.text("teamName"), Some("a"));
    }

    #[test]
    fn enter_ends_the_typing() {
        let library = with_field();
        let mut stage = Stage::new(None, &library);
        click(&mut stage, &library, 110.0, 110.0);
        assert!(stage.key(&Key::Enter, &library));
        assert_eq!(stage.focus, None);
    }
}
