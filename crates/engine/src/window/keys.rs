//! Says which of the window's keys are which of the game's.

use winit::keyboard::{Key, NamedKey};

use crate::input;

/// The game's key for one of the window's, where it is one of the keys the
/// game has a name for.
fn named(logical: &Key) -> Option<input::Key> {
    match logical {
        Key::Named(NamedKey::Backspace) => Some(input::Key::Backspace),
        Key::Named(NamedKey::Enter) => Some(input::Key::Enter),
        Key::Named(NamedKey::Tab) => Some(input::Key::Tab),
        Key::Named(NamedKey::Escape) => Some(input::Key::Escape),
        Key::Named(NamedKey::ArrowLeft) => Some(input::Key::Left),
        Key::Named(NamedKey::ArrowRight) => Some(input::Key::Right),
        Key::Named(NamedKey::ArrowUp) => Some(input::Key::Up),
        Key::Named(NamedKey::ArrowDown) => Some(input::Key::Down),
        _ => None,
    }
}

/// What a press of this key types, if anything, given the text the window
/// system says it made: a held Shift or Option has already been taken into
/// account there.
pub(super) fn typed(logical: &Key, text: Option<&str>) -> Vec<input::Key> {
    match named(logical) {
        Some(key) => vec![key],
        None => text
            .iter()
            .flat_map(|text| text.chars())
            .filter(|c| !c.is_control())
            .map(input::Key::Char)
            .collect(),
    }
}

/// The keys a game can be told are held down while this one is: the keys
/// it has names for, the space bar, and whatever letters the key types,
/// in small letters whether Shift is held or not.
pub(super) fn held(logical: &Key) -> Vec<input::Key> {
    match (named(logical), logical) {
        (Some(key), _) => vec![key],
        (None, Key::Named(NamedKey::Space)) => vec![input::Key::Char(' ')],
        (None, Key::Character(text)) => text
            .chars()
            .flat_map(char::to_lowercase)
            .map(input::Key::Char)
            .collect(),
        (None, _) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each key of the window's that the game has a name for, with the
    /// game's key.
    const NAMED: [(NamedKey, input::Key); 8] = [
        (NamedKey::Backspace, input::Key::Backspace),
        (NamedKey::Enter, input::Key::Enter),
        (NamedKey::Tab, input::Key::Tab),
        (NamedKey::Escape, input::Key::Escape),
        (NamedKey::ArrowLeft, input::Key::Left),
        (NamedKey::ArrowRight, input::Key::Right),
        (NamedKey::ArrowUp, input::Key::Up),
        (NamedKey::ArrowDown, input::Key::Down),
    ];

    fn letter(text: &str) -> Key {
        Key::Character(text.into())
    }

    #[test]
    fn a_key_the_game_has_a_name_for_is_that_key_pressed_or_held() {
        for (window, game) in NAMED {
            let key = Key::Named(window);
            assert_eq!(named(&key), Some(game));
            assert_eq!(held(&key), [game]);
            // Whatever text came with it is not typed as well.
            assert_eq!(typed(&key, None), [game]);
            assert_eq!(typed(&key, Some("\r")), [game]);
        }
        assert_eq!(named(&Key::Named(NamedKey::Space)), None);
        assert_eq!(named(&Key::Named(NamedKey::F1)), None);
        assert_eq!(named(&letter("a")), None);
    }

    #[test]
    fn a_press_types_the_text_it_made_and_nothing_that_cannot_be_seen() {
        // The text is as the window system gives it, Shift and all.
        assert_eq!(typed(&letter("a"), Some("A")), [input::Key::Char('A')]);
        let space = Key::Named(NamedKey::Space);
        assert_eq!(typed(&space, Some(" ")), [input::Key::Char(' ')]);
        // One press may make more than one letter.
        let both = [input::Key::Char('é'), input::Key::Char('a')];
        assert_eq!(typed(&letter("a"), Some("éa")), both);
        // A key that made no text types nothing, and nor does one whose
        // text is not a letter of any kind.
        assert!(typed(&letter("a"), None).is_empty());
        assert!(typed(&Key::Named(NamedKey::Shift), None).is_empty());
        assert!(typed(&Key::Named(NamedKey::Delete), Some("\u{7f}")).is_empty());
    }

    #[test]
    fn a_key_held_down_is_known_by_its_small_letter() {
        assert_eq!(held(&letter("a")), [input::Key::Char('a')]);
        assert_eq!(held(&letter("A")), [input::Key::Char('a')]);
        assert_eq!(held(&letter("7")), [input::Key::Char('7')]);
        let space = Key::Named(NamedKey::Space);
        assert_eq!(held(&space), [input::Key::Char(' ')]);
        // Keys the game knows nothing of are not held, as far as it knows.
        assert!(held(&Key::Named(NamedKey::Shift)).is_empty());
        assert!(held(&Key::Named(NamedKey::F1)).is_empty());
        assert!(held(&Key::Dead(None)).is_empty());
    }
}
