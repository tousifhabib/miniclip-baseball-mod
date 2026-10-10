//! The game put into words: which screen is showing and how the game on
//! it stands, for a script, a test or the inspector.

use super::Baseball;
use super::screen::Screen;

impl Baseball {
    /// Where the game is, in one line. The tests read it, and so does the
    /// record of whole games, so what it says of a screen is not to change.
    pub(super) fn in_words(&self) -> String {
        match self.screen {
            Screen::Menu => {
                // The mods that are on are named, when any are, each with
                // the level its setting is at if it has one.
                let mods: Vec<String> = self
                    .game
                    .mods
                    .all_on()
                    .map(|which| match which.setting() {
                        Some(_) => format!("{}={}", which.key(), self.game.mods.level(which)),
                        None => which.key().to_owned(),
                    })
                    .collect();
                let mods = if mods.is_empty() {
                    String::new()
                } else {
                    format!(", with {}", mods.join(" and "))
                };
                format!(
                    "Menu, {:?}, {:?}{mods}",
                    self.menu.page(),
                    self.game.settings.difficulty
                )
            }
            screen => {
                // A game says how it stands, and the screen a full match
                // ended on says how it went.
                let play = match (&self.play, &self.finished) {
                    (Some(play), _) => format!(": {}", play.describe()),
                    (None, Some(full)) => {
                        let page = self.pages.as_ref().map_or(String::new(), |pages| {
                            let (page, of) = pages.at();
                            format!(", page {page} of {of}")
                        });
                        format!(": {}, {}{page}", full.verdict(), full.describe())
                    }
                    (None, None) => String::new(),
                };
                format!("{screen:?}, {:?}{play}", self.game.settings.difficulty)
            }
        }
    }
}
