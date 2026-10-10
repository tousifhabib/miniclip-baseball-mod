//! The summary of a tournament before its next fixture: what the game has
//! told the menu of it, with a box that opens its tables and one that
//! gives it up.

use bb_engine::display::Path;
use bb_engine::stage::Stage;

use crate::art;
use crate::menu::summary::{SUMMARY_DOWN, SUMMARY_MIDDLE};
use crate::menu::{CHOICE_SIZE, HEADING_SIZE, Leave, Menu, MenuPage, WHITE, WORDS_SIZE};
use crate::play::overlay::{Lettering, Words};
use crate::sheet::Sheet;

/// On the summary page: where the badge at the start of the heading is,
/// the middle of the top of the heading's words, and under the lines the
/// corners of the two boxes, with where each one's words are from it.
const BADGE: (f32, f32) = (316.0, 75.1);
const HEADING: (f32, f32) = (421.0, 75.0);
const TABLES: ((f32, f32), (f32, f32)) = ((298.0, 286.0), (70.0, -5.0));
const GIVE_UP: ((f32, f32), (f32, f32)) = ((444.0, 286.0), (58.0, -5.0));

/// The boxes on a tournament's summary page.
pub(in crate::menu) struct Boxes {
    /// The one that opens the tables.
    tables: Path,
    /// The one that gives the tournament up, with its words. There is
    /// none once the tournament is over.
    give_up: Option<(Path, Words)>,
    /// It has been clicked once, and asks to be clicked again.
    sure: bool,
    asked_again: bool,
}

impl Boxes {
    /// What a click on the button at `path` asks for, if it is one of the
    /// boxes. Giving a tournament up takes two clicks.
    pub(super) fn clicked(&mut self, path: &[u16]) -> Option<Leave> {
        if self.tables == path {
            return Some(Leave::Tables);
        }
        let (give_up, _) = self.give_up.as_ref()?;
        if give_up != path {
            return None;
        }
        if self.sure {
            return Some(Leave::GiveUp);
        }
        self.sure = true;
        None
    }
}

impl Menu {
    /// On a tournament's summary page, takes the art's words about the
    /// last innings out of sight and writes what the game has said of the
    /// tournament, with its boxes under it.
    pub(in crate::menu) fn show_told(&mut self, stage: &mut Stage) {
        let (shown, fading) = (MenuPage::TournamentSummary, MenuPage::ToTournament);
        let menu = self.ready_the_summary(shown, fading, stage);
        // The box that gives it up, clicked once, asks to be clicked
        // again.
        if let Some(boxes) = &mut self.boxes
            && boxes.sure
            && !boxes.asked_again
            && let Some((_, words)) = &boxes.give_up
        {
            words.say("Really?", WHITE, stage);
            boxes.asked_again = true;
        }
        let (Some(menu), Some(told)) = (menu, &self.told) else {
            return;
        };
        let depth = Stage::RULES_DEPTH + 510;
        let Some(holder) = stage.attach(&menu, art::HOLDER, depth, "tournamentSummary") else {
            return;
        };
        let mut sheet = Sheet::on(holder.clone(), 1).lettered(art::MENU_FIELD);
        for symbol in art::BADGE {
            sheet.add(stage, symbol, "badge", BADGE, (1.0, 1.0));
        }
        let cup = (
            BADGE.0 + art::NINE_FROM_BADGE.0,
            BADGE.1 + art::NINE_FROM_BADGE.1,
        );
        sheet.add(stage, art::CUP, "cup", cup, (1.0, 1.0));
        let size = HEADING_SIZE;
        sheet.write(
            stage,
            "tournamentHeading",
            "TOURNAMENT",
            HEADING,
            size,
            WHITE,
        );
        // The lines go on over that, from a depth of their own.
        sheet.depth = 10;
        for (line, down) in told.lines.iter().zip(SUMMARY_DOWN) {
            let top = (SUMMARY_MIDDLE, down);
            sheet.write(stage, "tournamentLine", line, top, WORDS_SIZE, WHITE);
        }
        // Each box goes on after its words, so that a click on them is a
        // click on it.
        let lettering = Lettering {
            field: art::MENU_FIELD,
            size: CHOICE_SIZE,
        };
        let mut boxed = |at: ((f32, f32), (f32, f32)), says: &str, depth: u16| {
            let ((left, top), (across, down)) = at;
            let beside = (left + across, top + down);
            let words = Words::in_field(lettering, &holder, depth, "boxWords", beside, stage)?;
            words.say(says, WHITE, stage);
            sheet.depth = depth + 2;
            let button = sheet.add(stage, art::CHOICE, "tournamentBox", (left, top), (1.0, 1.0))?;
            Some((button, words))
        };
        let tables = boxed(TABLES, "See the tables", 30);
        // There is nothing to give up of a tournament that is over.
        let give_up = if told.over {
            None
        } else {
            boxed(GIVE_UP, "Give it up", 40)
        };
        if let Some((tables, _)) = tables {
            self.boxes = Some(Boxes {
                tables,
                give_up,
                sure: false,
                asked_again: false,
            });
        }
        self.summary = Some(holder);
    }
}
