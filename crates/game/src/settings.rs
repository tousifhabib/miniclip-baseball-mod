//! What the player chose before the game began.

use crate::look::Rgb;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl Difficulty {
    /// The art's name for this difficulty, as it labels the frames of the
    /// marker on the setup pages.
    pub fn label(self) -> &'static str {
        match self {
            Difficulty::Easy => "easy",
            Difficulty::Medium => "medium",
            Difficulty::Hard => "hard",
        }
    }
}

/// Where the player's side is to play a full match.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Ground {
    /// Wherever the toss of a coin says.
    #[default]
    Toss,
    /// At home, batting second in each innings.
    Home,
    /// Away, batting first.
    Away,
}

impl Ground {
    pub const ALL: [Ground; 3] = [Ground::Home, Ground::Away, Ground::Toss];

    /// The word for this choice on the setup page.
    pub fn word(self) -> &'static str {
        match self {
            Ground::Toss => "Toss",
            Ground::Home => "Home",
            Ground::Away => "Away",
        }
    }

    /// The choice with this name, as it is given on the command line.
    pub fn from_word(word: &str) -> Option<Ground> {
        Ground::ALL
            .into_iter()
            .find(|ground| ground.word().eq_ignore_ascii_case(word))
    }
}

/// The choices made on the setup pages. They last from one game to the next.
#[derive(Clone, Debug)]
pub struct Settings {
    pub difficulty: Difficulty,
    /// Where the side plays a full match.
    pub ground: Ground,
    /// The colour of the team's shirts and helmets, if one has been picked.
    pub clothes: Option<Rgb>,
    /// The arcade batter's skin, if one has been picked.
    pub skin: Option<Rgb>,
    /// The logo on the arcade batter's bat, if one has been picked.
    pub logo: Option<String>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            difficulty: Difficulty::Medium,
            ground: Ground::Toss,
            clothes: None,
            skin: None,
            logo: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ground_is_known_by_its_word_however_it_is_written() {
        assert_eq!(Ground::from_word("home"), Some(Ground::Home));
        assert_eq!(Ground::from_word("AWAY"), Some(Ground::Away));
        assert_eq!(Ground::from_word("Toss"), Some(Ground::Toss));
        assert_eq!(Ground::from_word("neutral"), None);
        // Left alone, it is the toss.
        assert_eq!(Settings::default().ground, Ground::Toss);
    }
}
