//! The menu's pages for a tournament: on the setup page, the choice of
//! its shape and of how many innings its matches have, and the summary of
//! where it has got to, which is in `summary`.

mod summary;

use bb_engine::display::Path;
use bb_engine::stage::Stage;

use super::{CHOICE_SIZE, Menu, MenuPage, WHITE};
use crate::art;
use crate::choice::{Choice, Row};
use crate::game::Game;
use crate::look;
use crate::settings::Settings;
use crate::sheet::Sheet;
use crate::tournament::{Brief, Format};
pub(super) use summary::Boxes;

/// The size of the words the shapes of a tournament are chosen by, which
/// are longer than the others on the page, the lettering's own being 1.
const SHAPE_SIZE: f32 = 11.5 / 18.0;

/// On the setup page, under the skill levels: the row of boxes a
/// tournament's shape is chosen by. Its words are of different lengths,
/// and each begins a little after its box.
const SHAPES: Row = Row {
    first: (348.0, 284.0),
    pitch: 79.0,
    word: (19.0, -4.0),
    begins: false,
    a_letter: 4.6,
    size: SHAPE_SIZE,
    colour: WHITE,
    fill: look::NAVY,
    names: ["shapeWord", "shape", "shapeFill"],
};

/// Under that, the middle of the top of the word that says what the next
/// row is of, and the row, which is of how many innings a match has.
const INNINGS_WORD: (f32, f32) = (388.0, 306.0);
const INNINGS: Row = Row {
    first: (436.0, 311.0),
    pitch: 46.0,
    word: (24.0, -5.0),
    begins: false,
    a_letter: 0.0,
    size: CHOICE_SIZE,
    colour: WHITE,
    fill: look::NAVY,
    names: ["inningsWord", "innings", "inningsFill"],
};

/// What a tournament has put on the setup page: the clip it is all in,
/// and the two rows of boxes.
pub(super) struct Choices {
    holder: Path,
    shapes: Choice<Format>,
    innings: Choice<u32>,
}

/// How many innings the matches of a tournament drawn now would have: as
/// chosen, if the rules give that length, and if not the first they give.
pub fn innings_chosen(game: &Game) -> u32 {
    let lengths = &game.rules.tournament.innings;
    let chosen = game.settings.innings;
    if lengths.contains(&chosen) {
        chosen
    } else {
        lengths.first().copied().unwrap_or(chosen)
    }
}

impl Menu {
    /// Tells the menu of the tournament in hand, or that there is none.
    pub fn tell(&mut self, told: Option<Brief>) {
        self.told = told;
    }

    /// Whether there is a tournament in hand that has a result in it, or
    /// is over: one to be gone on with, and not set up afresh.
    pub(super) fn underway(&self) -> bool {
        self.told
            .as_ref()
            .is_some_and(|told| told.begun || told.over)
    }

    /// Whether the tournament in hand has been played to its end.
    pub(super) fn over(&self) -> bool {
        self.told.as_ref().is_some_and(|told| told.over)
    }

    /// Takes down what a tournament has put on the menu's pages, but for
    /// its summary, which goes as a full match's does.
    pub(super) fn clear_tournament(&mut self, stage: &mut Stage) {
        if let Some(choices) = self.choices.take() {
            stage.remove(&choices.holder);
        }
        self.boxes = None;
    }

    /// On a tournament's setup page, puts the choices of shape and of
    /// innings under the skill levels once the page is there, and keeps
    /// the boxes of the ones chosen filled.
    pub(super) fn show_choices(&mut self, game: &Game, stage: &mut Stage) {
        // The menu may have been left and come back to since.
        if let Some(choices) = &self.choices
            && stage.child(&choices.holder).is_none()
        {
            self.choices = None;
        }
        if self.page != MenuPage::TournamentSetup || self.arriving {
            return;
        }
        if self.choices.is_none() {
            let Some(menu) = art::in_shell(stage, art::MENU) else {
                return;
            };
            let depth = Stage::RULES_DEPTH + 520;
            let Some(holder) = stage.attach(&menu, art::HOLDER, depth, "tournamentChoices") else {
                return;
            };
            let mut sheet = Sheet::on(holder.clone(), 1).lettered(art::MENU_FIELD);
            let shapes = Format::ALL.map(|format| (format, format.word()));
            let shapes = Choice::put(&shapes, &SHAPES, 10, &mut sheet, stage);
            sheet.depth = 50;
            let size = CHOICE_SIZE;
            sheet.write(
                stage,
                "inningsHeading",
                "Innings:",
                INNINGS_WORD,
                size,
                WHITE,
            );
            // As many lengths of match as the rules give.
            let lengths = &game.rules.tournament.innings;
            let words: Vec<String> = lengths.iter().map(u32::to_string).collect();
            let lengths: Vec<(u32, &str)> = lengths
                .iter()
                .copied()
                .zip(words.iter().map(String::as_str))
                .collect();
            let innings = Choice::put(&lengths, &INNINGS, 60, &mut sheet, stage);
            self.choices = Some(Choices {
                holder,
                shapes,
                innings,
            });
        }
        if let Some(choices) = &self.choices {
            choices.shapes.show(game.settings.format, stage);
            choices.innings.show(innings_chosen(game), stage);
        }
    }

    /// Takes in a click on the button at `path`, which may be one of the
    /// boxes a tournament has put on its setup page or on its summary.
    pub(super) fn chose_of_a_tournament(&mut self, path: &[u16], settings: &mut Settings) {
        if let Some(choices) = &self.choices {
            if let Some(format) = choices.shapes.clicked(path) {
                settings.format = format;
            }
            if let Some(innings) = choices.innings.clicked(path) {
                settings.innings = innings;
            }
        }
        if let Some(asked) = self.boxes.as_mut().and_then(|boxes| boxes.clicked(path)) {
            self.asked = Some(asked);
        }
    }
}
