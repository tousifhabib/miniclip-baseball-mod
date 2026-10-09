//! What the player does to the game: the pointer, in `pointer`, the keys,
//! and which text field a click is on, in `fields`.

mod fields;
mod pointer;

use bb_format::SymbolId;

use crate::library::Library;

pub use fields::field_at;
pub use pointer::Pointer;

/// Answers whether a point lies inside a drawn symbol. The renderer does this
/// with the triangles it draws.
pub trait Geometry {
    /// `x` and `y` are in the symbol's own coordinates.
    fn contains(&mut self, library: &Library, symbol: SymbolId, ratio: u16, x: f32, y: f32)
    -> bool;
}

/// A key the player has pressed, as far as a game needs to know.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// Something typed: a letter, a digit, a space, punctuation.
    Char(char),
    Backspace,
    Enter,
    Tab,
    Escape,
    Left,
    Right,
    Up,
    Down,
}

impl Key {
    /// The key with this name, as a script writes it: one of the keys
    /// that have a name here, `space`, or a single letter or figure.
    pub fn named(name: &str) -> Option<Key> {
        Some(match name {
            "backspace" => Key::Backspace,
            "enter" => Key::Enter,
            "tab" => Key::Tab,
            "escape" => Key::Escape,
            "left" => Key::Left,
            "right" => Key::Right,
            "up" => Key::Up,
            "down" => Key::Down,
            "space" => Key::Char(' '),
            other => {
                let mut letters = other.chars();
                match (letters.next(), letters.next()) {
                    (Some(letter), None) => Key::Char(letter),
                    _ => return None,
                }
            }
        })
    }

    /// The name a script writes this key by, the other way about from
    /// [`Key::named`].
    pub fn name(self) -> String {
        let name = match self {
            Key::Backspace => "backspace",
            Key::Enter => "enter",
            Key::Tab => "tab",
            Key::Escape => "escape",
            Key::Left => "left",
            Key::Right => "right",
            Key::Up => "up",
            Key::Down => "down",
            Key::Char(' ') => "space",
            // A letter or a figure is its own name.
            Key::Char(letter) => return letter.to_string(),
        };
        name.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_is_known_by_the_name_a_script_gives_it() {
        let names = [
            ("backspace", Key::Backspace),
            ("enter", Key::Enter),
            ("tab", Key::Tab),
            ("escape", Key::Escape),
            ("left", Key::Left),
            ("right", Key::Right),
            ("up", Key::Up),
            ("down", Key::Down),
            ("space", Key::Char(' ')),
            // A letter or a figure is its own name.
            ("a", Key::Char('a')),
            ("Q", Key::Char('Q')),
            ("7", Key::Char('7')),
        ];
        for (name, key) in names {
            assert_eq!(Key::named(name), Some(key), "{name}");
            assert_eq!(key.name(), name);
        }
        // Neither a name nor a single letter.
        assert_eq!(Key::named("shift"), None);
        assert_eq!(Key::named("Enter"), None);
        assert_eq!(Key::named(""), None);
    }
}
