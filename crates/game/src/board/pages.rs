//! The pages a finished full match can be read through, and the buttons
//! that turn them.

use bb_engine::display::Path;
use bb_engine::stage::Stage;

use super::batting::batting;
use super::figures::figures;
use super::pager::Pager;
use super::spray::field;
use super::timing::timing;
use super::turns::turns;
use super::{
    BACKING, CREAM, MIDDLE, RESULT_INNINGS, TURN_ROWS, VERDICT_SIZE, VERDICT_TOP, innings,
};
use crate::art;
use crate::play::full::{FullMatch, ordinal};
use crate::play::paper;
use crate::sheet::Sheet;

/// The pages after the first have a backing of their own, under a heading:
/// its left, top, width and height, and how solid it is.
const PANEL: [f32; 4] = [28.0, 84.0, 534.0, 228.0];
const PANEL_ALPHA: f32 = 0.55;
const PAGE_HEADING: (f32, f32) = (58.0, 1.35);

/// How far down the line about zingers is on a full match's board.
const ZINGER_TOP: f32 = 143.0;

/// What one of the pages a finished match has is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    /// Who won, the art's own figures, and every innings.
    Score,
    /// What each batter of one side did.
    Batting { ours: bool },
    /// The two sides' figures, side by side.
    Figures,
    /// Where one side's hits went.
    Field { ours: bool },
    /// How the player's swings were timed.
    Timing,
    /// The turns of one innings, both halves. A long innings has more than
    /// one part.
    Innings { innings: u32, part: usize },
}

/// The pages of a finished full match on the board the game ends on, with
/// arrows to turn them by.
pub struct Pages {
    /// The clip it is all in, which lies over the whole stage.
    holder: Path,
    full: FullMatch,
    /// The outs the player's side had in an innings.
    our_outs: u32,
    /// What there is to say about zingers, if there were any.
    zingers: Option<String>,
    pages: Vec<Page>,
    page: usize,
    /// The clip the page that is up is written in.
    sheet: Option<Path>,
    pager: Pager,
}

impl Pages {
    /// Puts the first page up in the clip at `holder`. `our_outs` is how
    /// many outs the player's side had in an innings.
    pub fn new(
        full: &FullMatch,
        our_outs: u32,
        zingers: Option<String>,
        holder: &[u16],
        stage: &mut Stage,
    ) -> Option<Pages> {
        let mut pages = vec![
            Page::Score,
            Page::Batting { ours: true },
            Page::Batting { ours: false },
            Page::Figures,
            Page::Field { ours: true },
            Page::Field { ours: false },
            Page::Timing,
        ];
        let played = full.book.ours.turns.iter().chain(&full.book.theirs.turns);
        let last = played.map(|turn| turn.innings).max().unwrap_or(0);
        for innings in 1..=last {
            let most = [&full.book.ours, &full.book.theirs]
                .map(|side| side.told(innings).len())
                .into_iter()
                .max()
                .unwrap_or(0);
            for part in 0..most.div_ceil(TURN_ROWS).max(1) {
                pages.push(Page::Innings { innings, part });
            }
        }
        let pager = Pager::put(holder, stage)?;
        let mut pages = Pages {
            holder: holder.to_vec(),
            full: full.clone(),
            our_outs,
            zingers,
            pages,
            page: 0,
            sheet: None,
            pager,
        };
        pages.draw(stage);
        Some(pages)
    }

    /// Takes in a click on the button at `path`, which may be one of the
    /// arrows. They go round: back from the first page is the last.
    pub fn clicked(&mut self, path: &[u16], stage: &mut Stage) {
        let turned = self.pager.turned(path, self.page, self.pages.len());
        let Some(page) = turned else {
            return;
        };
        self.page = page;
        self.draw(stage);
    }

    /// Called every frame the board is up. The art's own figures are only
    /// for the first page.
    pub fn keep(&self, stage: &mut Stage) {
        let Some(shell) = art::shell(stage) else {
            return;
        };
        for figures in art::RESULT_FIGURES {
            let found = stage.find_symbol(&shell, figures);
            if let Some(figures) = found.and_then(|path| stage.child_mut(&path)) {
                figures.set_visible(self.page == 0);
            }
        }
    }

    /// Which page is up, counting from 1, and how many there are.
    pub fn at(&self) -> (usize, usize) {
        (self.page + 1, self.pages.len())
    }

    /// Writes the page that is up, in place of the one that was.
    fn draw(&mut self, stage: &mut Stage) {
        if let Some(old) = self.sheet.take() {
            stage.remove(&old);
        }
        self.pager.say(self.page, self.pages.len(), stage);
        let Some(holder) = stage.attach(&self.holder, art::HOLDER, 10, "resultPage") else {
            return;
        };
        self.sheet = Some(holder.clone());
        let mut sheet = Sheet::on(holder, 1);
        let full = &self.full;
        let page = self.pages[self.page];
        if page == Page::Score {
            let top = (MIDDLE, VERDICT_TOP);
            sheet.write(
                stage,
                "boardVerdict",
                &full.verdict(),
                top,
                VERDICT_SIZE,
                CREAM,
            );
            if let Some(zingers) = &self.zingers {
                let top = (MIDDLE, ZINGER_TOP);
                sheet.write(stage, "zingerLine", zingers, top, VERDICT_SIZE, CREAM);
            }
            let (down, size) = RESULT_INNINGS;
            return innings(full, &mut sheet, MIDDLE, down, size, stage);
        }
        sheet.block(stage, "pagePanel", PANEL, BACKING, PANEL_ALPHA);
        let side = |ours: bool| {
            if ours {
                &full.book.ours
            } else {
                &full.book.theirs
            }
        };
        // The other side goes by its name where it has one.
        let their = full.their_name();
        let heading = match page {
            Page::Score => String::new(),
            Page::Batting { ours: true } => "YOUR BATTING".to_owned(),
            Page::Batting { ours: false } => format!("{} BATTING", their.unwrap_or("THEIR")),
            Page::Figures => "THE FIGURES".to_owned(),
            Page::Field { ours: true } => "WHERE YOU HIT IT".to_owned(),
            Page::Field { ours: false } => format!("WHERE {} HIT IT", their.unwrap_or("THEY")),
            Page::Timing => "YOUR TIMING".to_owned(),
            Page::Innings { innings, part: 0 } => format!("THE {} INNINGS", ordinal(innings)),
            Page::Innings { innings, .. } => format!("THE {} INNINGS, GOING ON", ordinal(innings)),
        };
        let (down, size) = PAGE_HEADING;
        sheet.write(stage, "pageHeading", &heading, (MIDDLE, down), size, CREAM);
        match page {
            Page::Score => {}
            Page::Batting { ours } => {
                // The other side's innings are played on paper, by the
                // game's own old rules.
                let outs = if ours { self.our_outs } else { paper::OUTS };
                batting(side(ours), side(!ours), (ours, outs), &mut sheet, stage);
            }
            Page::Figures => {
                let (ours, theirs) = (&full.book.ours, &full.book.theirs);
                figures(ours, theirs, full.them(), &mut sheet, stage);
            }
            Page::Field { ours } => field(side(ours), &mut sheet, stage),
            Page::Timing => timing(&full.book.ours, &mut sheet, stage),
            Page::Innings { innings, part } => turns(full, innings, part, &mut sheet, stage),
        }
    }
}
