//! The report on a session of batting practice, on the board the art has
//! for an innings that was tied: how the swings were timed, and where the
//! balls went.

use bb_engine::display::Path;
use bb_engine::stage::Stage;

use super::pager::Pager;
use super::spray::picture_of_the_field;
use super::tables::BUTTON;
use super::timing::timing_of;
use super::{BACKING, CREAM, GOLD, GREEN, MIDDLE, PANEL, PANEL_ALPHA, RED, WHITE};
use crate::art;
use crate::look::Rgb;
use crate::play::practice::{Sent, Session};
use crate::sheet::Sheet;

/// How far down a page's heading is, over its backing, and its size.
const HEADING: (f32, f32) = (58.0, 1.35);

/// The lines beside the picture of the field: how far along they begin, how
/// far down the first is, how far apart they are, how much further apart
/// where one lot ends and the next begins, and their size.
const LINES: (f32, f32) = (368.0, 96.0);
const LINE_PITCH: f32 = 16.5;
const GAP: f32 = 7.0;
const GAPS_AFTER: [usize; 3] = [3, 6, 8];
const LINE_SIZE: f32 = 0.68;
/// Where the line about what was let go by is, under all that, and its size.
const TAKING: (f32, f32, f32) = (MIDDLE, 294.0, 0.66);

/// What one of the report's pages is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    Timing,
    Field,
}

const PAGES: [Page; 2] = [Page::Timing, Page::Field];

/// The colour a ball is marked in, by what came of it.
fn mark(sent: &Sent) -> Rgb {
    if sent.foul {
        RED
    } else if sent.over_wall {
        GREEN
    } else {
        WHITE
    }
}

/// The report on the board, with arrows to turn its pages by.
pub struct Report {
    /// The clip it is all in, which lies over the whole stage.
    holder: Path,
    session: Session,
    page: usize,
    /// The clip the page that is up is written in.
    sheet: Option<Path>,
    pager: Pager,
}

impl Report {
    /// Puts the report up in the clip at `holder`, on its first page.
    pub fn new(session: &Session, holder: &[u16], stage: &mut Stage) -> Option<Report> {
        let pager = Pager::put(holder, stage)?;
        // What the board's button does here, written where the art has what
        // it does between innings.
        let mut sheet = Sheet::on(holder.to_vec(), 700);
        for (line, word) in ["MAIN", "MENU"].into_iter().enumerate() {
            let top = (BUTTON.0, BUTTON.1 + BUTTON.2 * line as f32);
            sheet.write(stage, "buttonWords", word, top, BUTTON.3, WHITE);
        }
        let mut report = Report {
            holder: holder.to_vec(),
            session: session.clone(),
            page: 0,
            sheet: None,
            pager,
        };
        report.draw(stage);
        Some(report)
    }

    /// Takes the report off the stage.
    pub fn take_down(self, stage: &mut Stage) {
        stage.remove(&self.holder);
    }

    /// Takes in a click on the button at `path`, which may be one of the
    /// arrows.
    pub fn clicked(&mut self, path: &[u16], stage: &mut Stage) {
        if let Some(page) = self.pager.turned(path, self.page, PAGES.len()) {
            self.page = page;
            self.draw(stage);
        }
    }

    /// Which page is up, in words, for a script or a test.
    pub fn describe(&self) -> String {
        format!(
            "{} pitches, page {} of {}",
            self.session.pitches(),
            self.page + 1,
            PAGES.len()
        )
    }

    /// Writes the page that is up, in place of the one that was.
    fn draw(&mut self, stage: &mut Stage) {
        if let Some(old) = self.sheet.take() {
            stage.remove(&old);
        }
        self.pager.say(self.page, PAGES.len(), stage);
        let Some(holder) = stage.attach(&self.holder, art::HOLDER, 10, "reportPage") else {
            return;
        };
        self.sheet = Some(holder.clone());
        let mut sheet = Sheet::on(holder, 1);
        sheet.block(stage, "pagePanel", PANEL, BACKING, PANEL_ALPHA);
        let page = PAGES[self.page];
        let heading = match page {
            Page::Timing => "YOUR TIMING",
            Page::Field => "WHERE YOU HIT IT",
        };
        let (down, size) = HEADING;
        sheet.write(stage, "pageHeading", heading, (MIDDLE, down), size, CREAM);
        match page {
            Page::Timing => {
                let none = "NOT A SWING ALL SESSION";
                timing_of(&self.session.timed(), none, &mut sheet, stage);
            }
            Page::Field => self.write_the_field(&mut sheet, stage),
        }
    }

    /// The page of where the balls went: the picture of the field with a
    /// mark for each, and what they come to beside it.
    fn write_the_field(&self, sheet: &mut Sheet, stage: &mut Stage) {
        // The fouls go on first and what went over the wall last, so that
        // the best are not hidden.
        let mut balls: Vec<Sent> = self.session.sent().collect();
        balls.sort_by_key(|sent| (!sent.foul, sent.over_wall));
        let marks = balls.iter().map(|sent| (sent.hit.at, mark(sent)));
        picture_of_the_field(marks, sheet, stage);
        for (row, line) in self.session.sent_lines().iter().enumerate() {
            let gaps = GAPS_AFTER.iter().filter(|&&after| row >= after).count();
            let down = LINES.1 + LINE_PITCH * row as f32 + GAP * gaps as f32;
            // The first three say what the colours of the marks are.
            let key = [GREEN, WHITE, RED].get(row).copied();
            if let Some(key) = key {
                sheet.dot(stage, "fieldKey", (LINES.0 - 12.0, down + 8.0), 1.4, key);
            }
            let colour = key.unwrap_or(CREAM);
            sheet.write_left(stage, "fieldLine", line, (LINES.0, down), LINE_SIZE, colour);
        }
        if let Some(taking) = self.session.taking_line() {
            let (across, down, size) = TAKING;
            sheet.write(stage, "takingLine", &taking, (across, down), size, GOLD);
        }
    }
}
