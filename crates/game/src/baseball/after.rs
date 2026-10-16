//! The screens a game leaves behind it: the board between innings, and
//! the lines of the result.

use bb_engine::display::Path;
use bb_engine::stage::Stage;

use super::Baseball;
use super::screen::Screen;
use crate::art;
use crate::board;
use crate::look::{self, Rgb};
use crate::play::Match;
use crate::play::overlay::Words;

/// How many frames into a result screen the line about zingers is written,
/// which is when the screen has got to its figures, and how far down the
/// screen: on the ones a match ends on, and on the arcade game's.
const ZINGER_LINE_AFTER: u32 = 260;
const ZINGER_LINE_TOP: (f32, f32) = (262.0, 325.0);
const ZINGER_LINE_SIZE: f32 = 0.8;
const ZINGER_LINE_COLOUR: Rgb = look::CREAM;

impl Baseball {
    /// Keeps the art's own words off the board it has for an innings that
    /// was tied, which a full match and a tournament each write their own
    /// on. Returns where the board is, and whether it has arrived.
    pub(super) fn clear_the_board(stage: &mut Stage) -> Option<(Path, bool)> {
        let path = art::in_shell(stage, art::BOARD)?;
        let clip = stage.clip(&path)?;
        let arrived = clip.frame >= art::BOARD_WORDS_FRAME;
        let theirs: Vec<u16> = clip
            .children
            .iter()
            .filter(|(_, child)| art::BOARD_WORDS.contains(&child.symbol))
            .map(|(&depth, _)| depth)
            .collect();
        for depth in theirs {
            let mut child = path.clone();
            child.push(depth);
            if let Some(child) = stage.child_mut(&child) {
                child.set_visible(false);
            }
        }
        Some((path, arrived))
    }

    /// While the board between innings is up, keeps the art's own words
    /// off it, and writes the full match's once the board has arrived.
    pub(super) fn show_interval(&mut self, stage: &mut Stage) {
        if self.screen != Screen::Interval {
            return;
        }
        let Some((path, arrived)) = Baseball::clear_the_board(stage) else {
            return;
        };
        if !arrived || self.board.is_some() {
            return;
        }
        if let Some(full) = self.play.as_ref().and_then(Match::full) {
            self.board = board::interval(full, &path, stage);
        }
    }

    /// Keeps a zinger that has beaten the longest there had been, as soon
    /// as it has, so that leaving the game early does not lose it.
    pub(super) fn keep_zinger_record(&mut self) {
        let Some(play) = &self.play else {
            return;
        };
        if play.zinger_record() <= self.scores.longest_zinger {
            return;
        }
        self.scores.longest_zinger = play.zinger_record();
        if let Some(file) = &self.scores_file
            && let Err(error) = self.scores.save(file)
        {
            eprintln!("The longest zinger could not be saved: {error:#}");
        }
    }

    /// Writes on a result screen what the art has no place for, once the
    /// screen has got to its figures: the pages of a full match, and the
    /// longest zinger of the game just finished with the longest there has
    /// ever been. A game with no zinger in it has no such line.
    pub(super) fn show_result_lines(&mut self, stage: &mut Stage) {
        if let Some(pages) = &self.pages {
            pages.keep(stage);
        }
        let top = match self.screen {
            Screen::MatchWon | Screen::MatchLost | Screen::InningsTied => ZINGER_LINE_TOP.0,
            Screen::ArcadeFinish => ZINGER_LINE_TOP.1,
            _ => return,
        };
        let nothing = self.last_zinger == 0 && self.finished.is_none();
        if nothing || self.result_lines.is_some() {
            return;
        }
        self.result_frames += 1;
        if self.result_frames < ZINGER_LINE_AFTER {
            return;
        }
        let Some(shell) = art::shell(stage) else {
            return;
        };
        let depth = Stage::RULES_DEPTH + 400;
        let Some(holder) = stage.attach(&shell, art::HOLDER, depth, "resultLines") else {
            return;
        };
        self.result_lines = Some(holder.clone());
        let zingers = (self.last_zinger > 0).then(|| {
            format!(
                "LONGEST ZINGER {} FT   BEST EVER {} FT",
                self.last_zinger, self.scores.longest_zinger
            )
        });
        if let Some(full) = &self.finished {
            // A full match has pages, and what there is to say of zingers
            // is on the first of them.
            let our_outs = self.playing.rules.game.outs;
            self.pages = board::Pages::new(full, our_outs, zingers, &holder, stage);
            return;
        }
        let Some(zingers) = zingers else {
            return;
        };
        let middle = stage.library().manifest.stage.width as f32 / 2.0;
        if let Some(words) = Words::new(
            &holder,
            1,
            "zingerLine",
            (middle, top),
            ZINGER_LINE_SIZE,
            stage,
        ) {
            words.say(&zingers, ZINGER_LINE_COLOUR, stage);
        }
    }

    /// Takes in a click on the button at `path`, which may be one of the
    /// arrows that turn the pages of a finished full match, or one of the
    /// buttons on a tournament's tables.
    pub(super) fn turn_page(&mut self, path: &[u16], stage: &mut Stage) {
        if let Some(pages) = &mut self.pages {
            pages.clicked(path, stage);
        }
        if let Some(tables) = &mut self.tables {
            tables.clicked(path, stage);
        }
    }
}
