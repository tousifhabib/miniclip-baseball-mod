//! The screens of the game, each a frame of the art's main timeline.

use crate::play::Outcome;

/// What the player is looking at. Each is a labelled frame of the shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    /// The art has not reached the shell yet.
    Loading,
    Intro,
    Menu,
    Match,
    Arcade,
    MatchLost,
    MatchWon,
    InningsTied,
    ArcadeFinish,
    Instructions,
    /// A full match, while the player's side is batting. It is played on
    /// the match's own frame of the shell.
    FullMatch,
    /// The board a full match shows between innings, which is the one the
    /// art has for an innings that was tied.
    Interval,
}

impl Screen {
    /// Whether a game is being played on this screen.
    pub(super) fn is_game(self) -> bool {
        matches!(self, Screen::Match | Screen::FullMatch | Screen::Arcade)
    }

    const ALL: [Screen; 9] = [
        Screen::Intro,
        Screen::Menu,
        Screen::Match,
        Screen::Arcade,
        Screen::MatchLost,
        Screen::MatchWon,
        Screen::InningsTied,
        Screen::ArcadeFinish,
        Screen::Instructions,
    ];

    /// What a full match is called where a screen is asked for by its
    /// label, the art having none for it.
    pub const FULL_MATCH: &str = "fullMatch";

    /// The screen the shell shows on the frame with this label.
    pub fn from_label(label: &str) -> Option<Screen> {
        if label == Screen::FULL_MATCH {
            return Some(Screen::FullMatch);
        }
        Screen::ALL
            .into_iter()
            .find(|screen| screen.label() == Some(label))
    }

    /// The shell's label for this screen.
    pub(super) fn label(self) -> Option<&'static str> {
        Some(match self {
            Screen::Loading => return None,
            Screen::Intro => "intro",
            Screen::Menu => "menu",
            Screen::Match | Screen::FullMatch => "match",
            Screen::Arcade => "arcade",
            Screen::MatchLost => "matchLost",
            Screen::MatchWon => "matchWon",
            Screen::InningsTied | Screen::Interval => "inningsTied",
            Screen::ArcadeFinish => "arcadeFinish",
            Screen::Instructions => "instructionsAll",
        })
    }
}

/// The screen that shows how a game came out.
pub(super) fn screen_after(outcome: Outcome) -> Screen {
    match outcome {
        Outcome::Won => Screen::MatchWon,
        Outcome::Lost => Screen::MatchLost,
        Outcome::Tied => Screen::InningsTied,
        Outcome::ArcadeOver => Screen::ArcadeFinish,
        Outcome::Interval => Screen::Interval,
    }
}
