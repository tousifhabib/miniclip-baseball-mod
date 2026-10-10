//! The tables of a tournament, on the board the art has for an innings
//! that was tied: how the sides stand, the knockout rounds, every round's
//! fixtures, the sides, the best of them and of their batters, and the
//! records. A match that has been played and a side can each be opened
//! from the page that lists it, and read.
//!
//! What there is to read is in sections, chosen by a row of boxes along
//! the top, and a section has pages, turned by the arrows under them. The
//! boxes are in `tabs`, what the pages are in `page`, and each kind of
//! page has a file: `standings`, `bracket`, `rounds`, `one_match`, `side`,
//! `leaders` and `records`. What every page says comes from the
//! tournament as rows, and is only laid out here.

mod bracket;
mod leaders;
mod one_match;
mod page;
mod records;
mod rounds;
mod side;
mod standings;
mod tabs;

use bb_engine::display::Path;
use bb_engine::stage::Stage;

use super::pager::Pager;
use super::{BACKING, CREAM, MIDDLE, PANEL, PANEL_ALPHA, WHITE};
use crate::art;
use crate::choice::Choice;
use crate::rules::TournamentRules;
use crate::sheet::Sheet;
use crate::tournament::Tournament;
use page::{Opened, Page};
use tabs::Section;

/// How far down a page's heading is, between the boxes and the page, and
/// its size, the lettering's own being 1.
const HEADING: (f32, f32) = (69.0, 0.8);

/// The words on the board's button: the middle of the top of the first,
/// how far apart the two are, and their size.
const BUTTON: (f32, f32, f32, f32) = (546.0, 344.0, 12.0, 0.62);

/// The tables of a tournament on the board, with the boxes that choose a
/// section and the arrows that turn its pages.
pub struct Tables {
    /// The clip it is all in, which lies over the whole stage.
    holder: Path,
    tournament: Tournament,
    /// The rules of a tournament, which say who has batted enough to be
    /// listed among the best.
    rules: TournamentRules,
    section: Section,
    /// What has been opened from a page of the section, if anything.
    opened: Option<Opened>,
    page: usize,
    /// The clip the page that is up is written in.
    sheet: Option<Path>,
    tabs: Choice<Section>,
    pager: Pager,
    /// The arrows on the page that is up that open something, each with
    /// what it opens.
    opens: Vec<(Path, Opened)>,
}

impl Tables {
    /// Puts the tables up in the clip at `holder`, on their first page.
    pub fn new(
        tournament: &Tournament,
        rules: &TournamentRules,
        holder: &[u16],
        stage: &mut Stage,
    ) -> Option<Tables> {
        let tabs = tabs::put(holder, stage);
        let pager = Pager::put(holder, stage)?;
        // What the board's button does here, written where the art has
        // what it does between innings.
        let mut sheet = Sheet::on(holder.to_vec(), 700);
        for (line, word) in ["CARRY", "ON"].into_iter().enumerate() {
            let top = (BUTTON.0, BUTTON.1 + BUTTON.2 * line as f32);
            sheet.write(stage, "buttonWords", word, top, BUTTON.3, WHITE);
        }
        let mut tables = Tables {
            holder: holder.to_vec(),
            tournament: tournament.clone(),
            rules: rules.clone(),
            section: Section::Table,
            opened: None,
            page: 0,
            sheet: None,
            tabs,
            pager,
            opens: Vec::new(),
        };
        tables.draw(stage);
        Some(tables)
    }

    /// Takes the tables off the stage.
    pub fn take_down(self, stage: &mut Stage) {
        stage.remove(&self.holder);
    }

    /// The pages there are to turn through: those of what has been
    /// opened, or of the section.
    fn pages(&self) -> Vec<Page> {
        let format = self.tournament.setup().format;
        page::pages(format, self.section, self.opened)
    }

    /// Takes in a click on the button at `path`, which may be one of the
    /// boxes that choose a section, one of the arrows that turn the pages,
    /// or an arrow that opens something.
    pub fn clicked(&mut self, path: &[u16], stage: &mut Stage) {
        let pages = self.pages().len();
        if let Some(section) = self.tabs.clicked(path) {
            // A section is begun again from its first page, with nothing
            // open.
            self.section = section;
            self.opened = None;
            self.page = self.first_page();
        } else if let Some(page) = self.pager.turned(path, self.page, pages) {
            self.page = page;
        } else if let Some((_, opened)) = self.opens.iter().find(|(arrow, _)| arrow == path) {
            self.opened = Some(*opened);
            self.page = 0;
        } else {
            return;
        }
        self.draw(stage);
    }

    /// The page a section opens on: of the matches, the round that is
    /// being played, or the last if they all have been.
    fn first_page(&self) -> usize {
        if self.section != Section::Matches {
            return 0;
        }
        let last = self.tournament.setup().format.rounds().saturating_sub(1);
        self.tournament.next().map_or(last, |next| next.round)
    }

    /// Which section is up, what is open in it, which page, counting from
    /// 1, and how many there are, for a script to read.
    pub fn describe(&self) -> String {
        let opened = match self.opened {
            Some(Opened::Match(fixture)) => format!(", match {}", fixture + 1),
            Some(Opened::Side(side)) => format!(", {}", self.tournament.name_of(side)),
            None => String::new(),
        };
        format!(
            "{}{opened}, page {} of {}",
            self.section.word(),
            self.page + 1,
            self.pages().len()
        )
    }

    /// Writes the page that is up, in place of the one that was.
    fn draw(&mut self, stage: &mut Stage) {
        if let Some(old) = self.sheet.take() {
            stage.remove(&old);
        }
        self.opens.clear();
        let pages = self.pages();
        self.page = self.page.min(pages.len().saturating_sub(1));
        self.pager.say(self.page, pages.len(), stage);
        self.tabs.show(self.section, stage);
        let Some(&page) = pages.get(self.page) else {
            return;
        };
        let Some(holder) = stage.attach(&self.holder, art::HOLDER, 10, "tablesPage") else {
            return;
        };
        self.sheet = Some(holder.clone());
        let mut sheet = Sheet::on(holder, 1);
        sheet.block(stage, "pagePanel", PANEL, BACKING, PANEL_ALPHA);
        let heading = page.heading(&self.tournament);
        let (down, size) = HEADING;
        sheet.write(stage, "pageHeading", &heading, (MIDDLE, down), size, CREAM);
        self.opens = page.write(&self.tournament, &self.rules, &mut sheet, stage);
    }
}
