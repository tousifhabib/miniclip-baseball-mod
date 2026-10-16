//! The summary of a full match that is about to be played: the ground,
//! the skill, and the mods that are on.

use bb_engine::stage::Stage;

use super::{Menu, MenuPage};
use crate::art;
use crate::game::Game;
use crate::menu::summary::{SUMMARY_DOWN, SUMMARY_MIDDLE};
use crate::menu::{HEADING_SIZE, WHITE, WORDS_SIZE};
use crate::settings::Ground;
use crate::sheet::Sheet;

/// On the summary page: where the badge at the start of the heading is,
/// and the middle of the top of the heading's words.
const SUMMARY_BADGE: (f32, f32) = (333.0, 75.1);
const SUMMARY_HEADING: (f32, f32) = (421.0, 75.0);

impl Menu {
    /// The lines of the full match's summary page.
    pub fn summary_lines(game: &Game, home: bool) -> [String; 5] {
        let innings = game.rules.full_match.innings;
        let how = match game.settings.ground {
            Ground::Toss => "The coin is tossed, and",
            Ground::Home | Ground::Away => "As you asked,",
        };
        let side = if home {
            "you are at home and bat second."
        } else {
            "you are away and bat first."
        };
        [
            format!("IT'S A FULL MATCH OF {innings} INNINGS!"),
            how.to_owned(),
            side.to_owned(),
            "The other side's innings are".to_owned(),
            "played for you. You just bat.".to_owned(),
        ]
    }

    /// On the full match's summary page, takes the art's words about the
    /// last innings out of sight and writes what a full match is to be.
    pub(crate) fn show_summary(&mut self, game: &Game, stage: &mut Stage) {
        let (shown, fading) = (MenuPage::FullSummary, MenuPage::ToFull);
        let Some(menu) = self.ready_the_summary(shown, fading, stage) else {
            return;
        };
        let depth = Stage::RULES_DEPTH + 510;
        let Some(holder) = stage.attach(&menu, art::HOLDER, depth, "fullSummary") else {
            return;
        };
        let mut sheet = Sheet::on(holder.clone(), 1).lettered(art::MENU_FIELD);
        let badge = SUMMARY_BADGE;
        for symbol in art::BADGE {
            sheet.add(stage, symbol, "badge", badge, (1.0, 1.0));
        }
        let nine = (
            badge.0 + art::NINE_FROM_BADGE.0,
            badge.1 + art::NINE_FROM_BADGE.1,
        );
        sheet.add(stage, art::NINE, "nine", nine, (1.0, 1.0));
        let (heading, size) = (SUMMARY_HEADING, HEADING_SIZE);
        sheet.write(stage, "fullHeading", "FULL MATCH", heading, size, WHITE);
        let home = self.at_home(&game.settings);
        let lines = Menu::summary_lines(game, home);
        // The lines go on over all of that, from a depth of their own.
        sheet.depth = 10;
        for (line, down) in lines.iter().zip(SUMMARY_DOWN) {
            let top = (SUMMARY_MIDDLE, down);
            sheet.write(stage, "fullLine", line, top, WORDS_SIZE, WHITE);
        }
        self.summary = Some(holder);
    }
}
