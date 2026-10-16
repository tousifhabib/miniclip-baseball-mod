//! The pages of the tables: what each is of, which of them a section has
//! or a thing that has been opened, what each is headed, and the writing
//! of it.

use bb_engine::display::Path;
use bb_engine::stage::Stage;

use super::tabs::Section;
use super::{bracket, leaders, one_match, records, rounds, side, standings};
use crate::rules::TournamentRules;
use crate::sheet::Sheet;
use crate::tournament::stats::leaders as best;
use crate::tournament::{Format, Tournament};

/// How many of the best at a thing are listed.
const LISTED: usize = 5;

/// Something that has been opened from the page that lists it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Opened {
    /// The fixture with this number.
    Match(usize),
    /// The side at this place in the draw.
    Side(usize),
}

/// What one of the pages is of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Page {
    /// The table, or both groups' tables.
    Standings,
    /// The knockout rounds, side by side.
    Knockout,
    /// The fixtures of a round.
    Round(usize),
    /// A match that has been played: who won, and every innings.
    Score(usize),
    /// The two sides' figures in it, side by side.
    Figures(usize),
    /// What each batter of one of its sides did.
    Batting {
        fixture: usize,
        home: bool,
    },
    /// Every side, each to be opened.
    Sides,
    /// A side's fixtures, and how each came out.
    Results(usize),
    /// What each of its batters has done in every match it has played.
    SideBatting(usize),
    /// Its figures beside those of the sides it has played.
    SideFigures(usize),
    /// The best of the batters, and of the sides.
    Batters,
    BestSides,
    /// The most there has been of each thing in one match, and how much
    /// there has been of everything in all.
    Records,
    Totals,
}

/// The pages there are to turn through: those of what has been opened, or
/// of the section.
pub(super) fn pages(format: Format, section: Section, opened: Option<Opened>) -> Vec<Page> {
    match (opened, section) {
        (Some(Opened::Match(fixture)), _) => vec![
            Page::Score(fixture),
            Page::Figures(fixture),
            Page::Batting {
                fixture,
                home: false,
            },
            Page::Batting {
                fixture,
                home: true,
            },
        ],
        (Some(Opened::Side(side)), _) => vec![
            Page::Results(side),
            Page::SideBatting(side),
            Page::SideFigures(side),
        ],
        (None, Section::Table) => match format {
            Format::League => vec![Page::Standings],
            Format::Groups => vec![Page::Standings, Page::Knockout],
            Format::Cup => vec![Page::Knockout],
        },
        (None, Section::Matches) => (0..format.rounds()).map(Page::Round).collect(),
        (None, Section::Sides) => vec![Page::Sides],
        (None, Section::Leaders) => vec![Page::Batters, Page::BestSides],
        (None, Section::Records) => vec![Page::Records, Page::Totals],
    }
}

impl Page {
    /// What the page is headed.
    pub fn heading(self, tournament: &Tournament) -> String {
        match self {
            Page::Standings => standings::heading(tournament),
            Page::Knockout => "THE KNOCKOUT ROUNDS".to_owned(),
            Page::Round(round) => tournament.setup().format.round(round).words(),
            Page::Score(fixture) | Page::Figures(fixture) => {
                one_match::heading(tournament, fixture)
            }
            Page::Batting { fixture, home } => {
                one_match::batting_heading(tournament, fixture, home)
            }
            Page::Sides => "THE SIDES".to_owned(),
            Page::Results(which) => side::heading(tournament, which),
            Page::SideBatting(which) => side::batting_heading(tournament, which),
            Page::SideFigures(which) => side::figures_heading(tournament, which),
            Page::Batters => "THE BEST BATTERS".to_owned(),
            Page::BestSides => "THE BEST SIDES".to_owned(),
            Page::Records => "THE RECORDS".to_owned(),
            Page::Totals => "IN ALL".to_owned(),
        }
    }

    /// Writes the page under its heading. Returns the arrows on it that
    /// open something, each with what it opens.
    pub fn write(
        self,
        tournament: &Tournament,
        rules: &TournamentRules,
        sheet: &mut Sheet,
        stage: &mut Stage,
    ) -> Vec<(Path, Opened)> {
        match self {
            Page::Standings => standings::standings(tournament, sheet, stage),
            Page::Knockout => bracket::knockout(tournament, sheet, stage),
            Page::Round(round) => {
                let opens = rounds::round(tournament, round, sheet, stage);
                let a_match = |(arrow, fixture)| (arrow, Opened::Match(fixture));
                return opens.into_iter().map(a_match).collect();
            }
            Page::Score(fixture) => one_match::score(tournament, fixture, sheet, stage),
            Page::Figures(fixture) => one_match::figures(tournament, fixture, sheet, stage),
            Page::Batting { fixture, home } => {
                one_match::batting(tournament, fixture, home, sheet, stage);
            }
            Page::Sides => {
                let opens = side::list(tournament, sheet, stage);
                let a_side = |(arrow, which)| (arrow, Opened::Side(which));
                return opens.into_iter().map(a_side).collect();
            }
            Page::Results(which) => side::results(tournament, which, sheet, stage),
            Page::SideBatting(which) => side::batting(tournament, which, sheet, stage),
            Page::SideFigures(which) => side::figures(tournament, which, sheet, stage),
            Page::Batters => {
                let lists = best::batters(tournament, rules, LISTED);
                leaders::lists(&lists, sheet, stage);
            }
            Page::BestSides => {
                let lists = best::sides(tournament, LISTED);
                leaders::lists(&lists, sheet, stage);
            }
            Page::Records => records::records(tournament, sheet, stage),
            Page::Totals => records::totals(tournament, sheet, stage),
        }
        Vec::new()
    }
}
