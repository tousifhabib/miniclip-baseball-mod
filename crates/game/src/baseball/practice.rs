//! The end of a session of batting practice: the report on it.

use bb_engine::stage::Stage;

use super::Baseball;
use super::screen::Screen;
use crate::art;
use crate::board;

impl Baseball {
    /// Takes the session in hand from a game being given up. Returns whether
    /// there was one with anything in it to report on, and if so goes to the
    /// report.
    pub(super) fn report_on_the_session(&mut self, stage: &mut Stage) -> bool {
        let session = self.play.as_mut().and_then(|play| play.take_session());
        self.session = session.filter(|session| session.pitches() > 0);
        if self.session.is_none() {
            return false;
        }
        self.show(Screen::PracticeReport, stage);
        true
    }

    /// While the report is showing, keeps the art's own words off the board,
    /// and puts the report on once the board has arrived.
    pub(super) fn show_report(&mut self, stage: &mut Stage) {
        if self.screen != Screen::PracticeReport {
            return;
        }
        let Some((_, arrived)) = Baseball::clear_the_board(stage) else {
            return;
        };
        // The board's button asks for the next innings, of which there is
        // none here. The report says what it does in its place.
        let words = stage.find_symbol(&[], art::NEXT_INNINGS_WORDS);
        if let Some(words) = words.and_then(|path| stage.child_mut(&path)) {
            words.set_visible(false);
        }
        if !arrived || self.report.is_some() {
            return;
        }
        let (Some(session), Some(shell)) = (&self.session, art::shell(stage)) else {
            return;
        };
        let depth = Stage::RULES_DEPTH + 400;
        let Some(holder) = stage.attach(&shell, art::HOLDER, depth, "practiceReport") else {
            return;
        };
        self.report = board::Report::new(session, &holder, stage);
    }
}
