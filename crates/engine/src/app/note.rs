//! A note of something that happened, kept for whoever asks what did.

use std::collections::VecDeque;

use bb_format::SymbolId;

use crate::display::{ButtonEvent, Path};
use crate::input::Key;

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

/// The most notes a runner keeps for whoever takes them. A window takes
/// them every time it draws and a script when it is asked what happened, so
/// neither comes near it. A runner that nobody asks keeps the latest, and
/// does not grow for as long as it runs.
pub const MOST_NOTES: usize = 65_536;

/// Keeps a note, letting the oldest go if there are as many as may be kept.
pub(super) fn keep(notes: &mut VecDeque<Note>, note: Note) {
    if notes.len() == MOST_NOTES {
        notes.pop_front();
    }
    notes.push_back(note);
}

#[cfg(test)]
mod tests {

    use super::*;

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
