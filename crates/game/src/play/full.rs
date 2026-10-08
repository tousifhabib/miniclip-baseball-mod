//! A full match: every innings, against another side.
//!
//! Only the player's own innings are played. The other side's are made up
//! as they come round. This keeps both sides' scores, says when the match
//! is over and who has won it, and what the board between innings has to
//! tell.

use bb_engine::display::Path;
use bb_engine::library::Library;
use bb_engine::stage::Stage;

use super::overlay::Words;
use super::pitch::Point;
use super::{Match, Outcome, Parts, Phase};
use crate::art;
use crate::look::Rgb;
use crate::menu::Game;
use crate::mods::Mod;
use crate::rng::Rng;
use crate::rules::FullMatchRules;
use crate::settings::Difficulty;

/// How things stand when the player's side is out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// The other side has batted, and the player's side comes in again.
    Bat,
    Won,
    Lost,
}

/// What the board shows for one side's half of an innings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    /// Not yet come to.
    Blank,
    Runs(u32),
    /// Never played: the side batting last had won without it.
    NotNeeded,
}

/// What the board between innings has to say.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub heading: String,
    pub lines: Vec<String>,
}

/// A side's line on the board: what it is called, what it made in each
/// innings shown, and what it has made in all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub name: &'static str,
    /// Whether it is the player's side.
    pub ours: bool,
    pub cells: Vec<Cell>,
    pub runs: u32,
}

#[derive(Clone, Debug)]
pub struct FullMatch {
    /// The player's side is at home, and bats second in each innings.
    home: bool,
    rules: FullMatchRules,
    difficulty: Difficulty,
    /// How many innings' worth of runs the other side is given in each of
    /// its own.
    worth: u32,
    /// The runs each side made in each half it has finished.
    ours: Vec<u32>,
    theirs: Vec<u32>,
    /// The side batting last had won before its last half, which was never
    /// played.
    unneeded: bool,
    over: bool,
    /// What the other side's innings are drawn from.
    rng: Rng,
}

/// The most innings the board has room for. A longer match shows its last.
pub const COLUMNS: u32 = 9;

/// A number as the place it has in an order, in capitals: 1ST, 2ND, 11TH.
pub fn ordinal(number: u32) -> String {
    let ending = match (number % 10, number % 100) {
        (_, 11..=13) => "TH",
        (1, _) => "ST",
        (2, _) => "ND",
        (3, _) => "RD",
        _ => "TH",
    };
    format!("{number}{ending}")
}

/// A number of runs, in words fit for the board.
fn runs_words(runs: u32) -> String {
    match runs {
        0 => "NO RUNS".to_owned(),
        1 => "1 RUN".to_owned(),
        runs => format!("{runs} RUNS"),
    }
}

impl FullMatch {
    /// A match about to begin, with the player's side at home or away. If
    /// it is at home the other side has batted already when this returns.
    /// `zinger` is whether every ball the player hits is a home run.
    pub fn new(
        home: bool,
        rules: &FullMatchRules,
        difficulty: Difficulty,
        zinger: bool,
        seed: u64,
    ) -> FullMatch {
        let mut full = FullMatch {
            home,
            rules: rules.clone(),
            difficulty,
            worth: if zinger { rules.zinger_innings } else { 1 },
            ours: Vec::new(),
            theirs: Vec::new(),
            unneeded: false,
            over: false,
            rng: Rng::new(seed),
        };
        if home {
            let made = full.made();
            full.theirs.push(made);
        }
        full
    }

    /// The runs the other side makes in an innings left to run its course.
    fn made(&mut self) -> u32 {
        (0..self.worth)
            .map(|_| self.rules.runs_for(self.difficulty, self.rng.unit()))
            .sum()
    }

    pub fn at_home(&self) -> bool {
        self.home
    }

    /// The runs the player's side made in the halves it has finished.
    pub fn ours(&self) -> u32 {
        self.ours.iter().sum()
    }

    /// The other side's runs.
    pub fn theirs(&self) -> u32 {
        self.theirs.iter().sum()
    }

    /// The innings the player's side is batting in, or bats in next,
    /// counting from 1.
    pub fn innings(&self) -> u32 {
        self.ours.len() as u32 + 1
    }

    /// Whether the innings in hand is the last there has to be, or one
    /// added to it.
    fn last(&self, innings: u32) -> bool {
        innings >= self.rules.innings
    }

    /// Whether the player's side, batting now, wins the match the moment it
    /// is ahead: at home, with every innings there has to be all but
    /// played.
    pub fn sudden(&self) -> bool {
        self.home && !self.over && self.last(self.innings())
    }

    /// The player's side is out, having made `runs` in this half. The
    /// other side's next half is made up, and this says how things stand
    /// after it.
    pub fn side_out(&mut self, runs: u32) -> Next {
        self.ours.push(runs);
        let innings = self.ours.len() as u32;
        let last = self.last(innings);
        let next = if self.home {
            self.after_the_bottom(innings, last)
        } else {
            self.after_the_top(last)
        };
        self.over = next != Next::Bat;
        next
    }

    /// At home, the innings is over. Unless that decides the match, the
    /// visitors bat in the top of the next.
    fn after_the_bottom(&mut self, innings: u32, last: bool) -> Next {
        let (ours, theirs) = (self.ours(), self.theirs());
        if last && ours != theirs {
            return if ours > theirs { Next::Won } else { Next::Lost };
        }
        let made = self.made();
        self.theirs.push(made);
        // Ahead with only the bottom of the last innings to come, the home
        // side has no need of it.
        if self.last(innings + 1) && ours > self.theirs() {
            self.unneeded = true;
            return Next::Won;
        }
        Next::Bat
    }

    /// Away, the home side has the bottom of the innings to bat in, unless
    /// it is the last and they are ahead already.
    fn after_the_top(&mut self, last: bool) -> Next {
        let ours = self.ours();
        if last && self.theirs() > ours {
            self.unneeded = true;
            return Next::Lost;
        }
        let mut made = self.made();
        if last {
            // They stop as soon as they are ahead.
            made = made.min(ours - self.theirs() + 1);
        }
        self.theirs.push(made);
        match (last, self.theirs().cmp(&ours)) {
            (false, _) | (true, std::cmp::Ordering::Equal) => Next::Bat,
            (true, std::cmp::Ordering::Greater) => Next::Lost,
            (true, std::cmp::Ordering::Less) => Next::Won,
        }
    }

    /// The player's side, batting last, has gone ahead with `runs` in this
    /// half so far. That is the match.
    pub fn walked_off(&mut self, runs: u32) {
        self.ours.push(runs);
        self.over = true;
    }

    /// The innings the board shows: the first of them, and how many.
    pub fn shown(&self) -> (u32, u32) {
        let played = self.ours.len().max(self.theirs.len()) as u32;
        let all = played.max(self.rules.innings).max(1);
        let count = all.min(COLUMNS);
        (all - count + 1, count)
    }

    /// The two sides' lines on the board, the visitors' first.
    pub fn lines(&self) -> [Line; 2] {
        let (first, count) = self.shown();
        let line = |ours: bool| {
            let made = if ours { &self.ours } else { &self.theirs };
            // The side at home is the one that may not have needed its
            // last half.
            let at_home = ours == self.home;
            let cells = (first..first + count)
                .map(|innings| match made.get(innings as usize - 1) {
                    Some(&runs) => Cell::Runs(runs),
                    None if at_home && self.unneeded && innings as usize == made.len() + 1 => {
                        Cell::NotNeeded
                    }
                    None => Cell::Blank,
                })
                .collect();
            Line {
                name: if ours { "YOU" } else { "THEM" },
                ours,
                cells,
                runs: made.iter().sum(),
            }
        };
        if self.home {
            [line(false), line(true)]
        } else {
            [line(true), line(false)]
        }
    }

    /// What the board says when the other side has batted and the player's
    /// side is to come in.
    pub fn report(&self) -> Report {
        let (ours, theirs) = (self.ours(), self.theirs());
        let innings = self.innings();
        let made = runs_words(self.theirs.last().copied().unwrap_or(0));
        let standing = match ours.cmp(&theirs) {
            std::cmp::Ordering::Greater => format!("YOU LEAD {ours} - {theirs}"),
            std::cmp::Ordering::Less => format!("YOU TRAIL {ours} - {theirs}"),
            std::cmp::Ordering::Equal => format!("IT IS LEVEL AT {ours} - {theirs}"),
        };
        if self.home {
            let mut lines = vec![
                format!("THE VISITORS MADE {made}"),
                standing,
                format!("YOU BAT IN THE BOTTOM OF THE {}", ordinal(innings)),
            ];
            if self.sudden() {
                let needed = runs_words(theirs + 1 - ours.min(theirs));
                lines.push(format!("{needed} WILL WIN THE MATCH"));
            }
            return Report {
                heading: format!("TOP OF THE {}", ordinal(innings)),
                lines,
            };
        }
        let mut lines = vec![
            format!("THE HOME SIDE MADE {made}"),
            standing,
            format!("YOU BAT IN THE TOP OF THE {}", ordinal(innings)),
        ];
        if innings > self.rules.innings {
            lines.push("THE MATCH GOES ON UNTIL IT IS WON".to_owned());
        }
        Report {
            heading: format!("END OF THE {}", ordinal(innings - 1)),
            lines,
        }
    }

    /// The match in a line, once it is over: who won, by what, and in how
    /// many innings if it took more than it had to.
    pub fn verdict(&self) -> String {
        let (ours, theirs) = (self.ours(), self.theirs());
        let how = match ours.cmp(&theirs) {
            std::cmp::Ordering::Greater => "YOU WON",
            std::cmp::Ordering::Less => "YOU LOST",
            std::cmp::Ordering::Equal => "LEVEL AT",
        };
        let played = self.ours.len().max(self.theirs.len()) as u32;
        let extra = if played > self.rules.innings {
            format!(" IN {played} INNINGS")
        } else {
            String::new()
        };
        format!("{how} {ours} - {theirs}{extra}")
    }

    /// Which half the player's side bats in, and of which innings.
    pub fn batting_in(&self) -> String {
        let half = if self.home { "bottom" } else { "top" };
        format!("{half} of innings {}", self.innings())
    }

    /// The same, as short as the corner of the batting view needs it.
    pub fn half_words(&self) -> String {
        let half = if self.home { "BOT" } else { "TOP" };
        format!("{half} {}", ordinal(self.innings()))
    }

    /// Every innings of both sides, for a script to read.
    pub fn describe(&self) -> String {
        let list = |made: &[u32], at_home: bool| {
            let mut all: Vec<String> = made.iter().map(u32::to_string).collect();
            if at_home && self.unneeded {
                all.push("x".to_owned());
            }
            if all.is_empty() {
                "-".to_owned()
            } else {
                all.join(" ")
            }
        };
        let (away, home) = if self.home {
            (&self.theirs, &self.ours)
        } else {
            (&self.ours, &self.theirs)
        };
        format!(
            "{}, visitors {}, home side {}",
            if self.home { "at home" } else { "away" },
            list(away, false),
            list(home, true)
        )
    }
}

/// Where the word that names the other side's score goes on a scoreboard,
/// which is over that score: the middle of its top edge, its size, the
/// lettering's own being 1, and its colour.
const THEM_AT: Point = (21.6, 2.4);
const THEM_SIZE: f32 = 10.0 / 18.0;
const THEM_COLOUR: Rgb = [0xfc, 0xf2, 0xa5];

/// The word a full match writes on a scoreboard over the other side's
/// score, in place of the art's own word for the score to beat.
pub(crate) struct Them {
    board: Path,
    words: Words,
    /// The board is the one over the field, which draws its home runs
    /// over its figures without taking them off.
    field: bool,
}

impl Them {
    /// Puts the word on both of a view's scoreboards. It is not seen until
    /// it is kept.
    pub fn put(parts: &Parts, stage: &mut Stage, library: &Library) -> Vec<Them> {
        let boards = [(&parts.scoreboard, false), (&parts.field_scoreboard, true)];
        let mut put = Vec::new();
        for (board, field) in boards {
            let Some(board) = board else {
                continue;
            };
            let depth = Stage::RULES_DEPTH + 1;
            let words = Words::new(
                board,
                depth,
                "themLabel",
                THEM_AT,
                THEM_SIZE,
                stage,
                library,
            );
            if let Some(words) = words {
                put.push(Them {
                    board: board.clone(),
                    words,
                    field,
                });
            }
        }
        put
    }

    /// Takes the art's word out of sight, and shows this one in its place
    /// for as long as the board is showing its figures.
    pub fn keep(&self, stage: &mut Stage) {
        let Some(board) = stage.clip(&self.board) else {
            return;
        };
        let labels: Vec<u16> = board
            .children
            .iter()
            .filter(|(_, child)| art::TARGET_LABEL.contains(&child.symbol))
            .map(|(&depth, _)| depth)
            .collect();
        let showing = !labels.is_empty() && (!self.field || board.frame == 1);
        for depth in labels {
            let mut path = self.board.clone();
            path.push(depth);
            if let Some(label) = stage.child_mut(&path) {
                label.set_visible(false);
            }
        }
        if showing {
            self.words.say("THEM", THEM_COLOUR, stage);
        } else {
            self.words.hide(stage);
        }
    }
}

/// What makes the other side's innings come out differently from the
/// pitches, which are drawn from the seed itself: the same pitches come
/// whichever side bats first.
const THEIR_SEED: u64 = 0x6a09_e667_f3bc_c908;

impl Match {
    /// A full match, with the player's side at home or away, in place of
    /// the last innings of one.
    pub fn new_full(game: &Game, home: bool, seed: u64, library: &Library) -> Match {
        let mut played = Match::new(game, seed, library);
        let full = FullMatch::new(
            home,
            &game.rules.full_match,
            game.settings.difficulty,
            game.mods.is_on(Mod::ZingerHit),
            seed ^ THEIR_SEED,
        );
        played.target = full.theirs() + 1;
        played.full = Some(full);
        played
    }

    /// The full match being played, if that is the game.
    pub fn full(&self) -> Option<&FullMatch> {
        self.full.as_ref()
    }

    /// In a full match, the half the player's side was batting in is over.
    /// It is written up and the other side has its turn, after which what
    /// was only a side being out may be the match won or lost.
    pub(crate) fn close_half(&mut self, outcome: Outcome) -> Outcome {
        let Some(full) = &mut self.full else {
            return outcome;
        };
        let runs = self.score.saturating_sub(full.ours());
        if outcome == Outcome::Won {
            full.walked_off(runs);
            return Outcome::Won;
        }
        match full.side_out(runs) {
            Next::Bat => Outcome::Interval,
            Next::Won => Outcome::Won,
            Next::Lost => Outcome::Lost,
        }
    }

    /// The board has been read: the player's side comes in to bat, with
    /// nobody out and nobody on base.
    pub fn bat_again(&mut self) {
        let Some(full) = &self.full else {
            return;
        };
        self.target = full.theirs() + 1;
        // What each batter made is kept under his place in the order.
        for runner in std::mem::take(&mut self.runners) {
            if self.tally.len() <= runner.order {
                self.tally.resize(runner.order + 1, 0);
            }
            self.tally[runner.order] += runner.runs;
        }
        self.outs_before += self.outs.min(self.max_outs);
        self.outs = 0;
        self.clear_count();
        self.announce = false;
        self.at = None;
        self.cues.clear();
        self.phase = Phase::Arriving;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rules;

    /// A match in which the other side makes `their` runs in every innings.
    fn against(home: bool, their: u32, innings: u32) -> FullMatch {
        let mut rules = Rules::default().full_match;
        rules.innings = innings;
        let mut chances = vec![0; their as usize + 1];
        chances[their as usize] = 1;
        rules.runs.easy = chances.clone();
        rules.runs.medium = chances.clone();
        rules.runs.hard = chances;
        FullMatch::new(home, &rules, Difficulty::Medium, false, 7)
    }

    #[test]
    fn numbers_are_given_their_places() {
        let all: Vec<String> = [1, 2, 3, 4, 9, 10, 11, 12, 13, 21, 22, 23, 101, 111]
            .map(ordinal)
            .to_vec();
        assert_eq!(
            all,
            [
                "1ST", "2ND", "3RD", "4TH", "9TH", "10TH", "11TH", "12TH", "13TH", "21ST", "22ND",
                "23RD", "101ST", "111TH"
            ]
        );
    }

    #[test]
    fn the_other_sides_runs_follow_the_chances_given() {
        let rules = Rules::default().full_match;
        // The first share of the draw is for no runs, and the last for the
        // most there can be.
        assert_eq!(rules.runs_for(Difficulty::Easy, 0.0), 0);
        assert_eq!(rules.runs_for(Difficulty::Easy, 0.999), 5);
        assert_eq!(rules.runs_for(Difficulty::Hard, 0.999), 8);
        // On easy, forty-four innings in a hundred are noughts, and the
        // next thirty are ones.
        assert_eq!(rules.runs_for(Difficulty::Easy, 0.43), 0);
        assert_eq!(rules.runs_for(Difficulty::Easy, 0.45), 1);
        assert_eq!(rules.runs_for(Difficulty::Easy, 0.75), 2);
        // The harder the level, the more runs on the whole.
        let mean = |difficulty| {
            (0..1000)
                .map(|step| rules.runs_for(difficulty, step as f32 / 1000.0))
                .sum::<u32>()
        };
        assert!(mean(Difficulty::Easy) < mean(Difficulty::Medium));
        assert!(mean(Difficulty::Medium) < mean(Difficulty::Hard));
    }

    #[test]
    fn away_the_match_is_won_by_being_ahead_when_the_home_side_is_out() {
        let mut full = against(false, 1, 3);
        assert!(!full.at_home() && !full.sudden());
        assert_eq!(full.batting_in(), "top of innings 1");
        assert_eq!(full.side_out(2), Next::Bat);
        assert_eq!((full.ours(), full.theirs()), (2, 1));
        assert_eq!(full.side_out(0), Next::Bat);
        // Three to two up going into their last half, in which they make
        // only the one: level, so on it goes.
        assert_eq!(full.side_out(1), Next::Bat);
        assert_eq!((full.ours(), full.theirs()), (3, 3));
        assert_eq!(full.batting_in(), "top of innings 4");
        assert!(
            full.report()
                .lines
                .contains(&"THE MATCH GOES ON UNTIL IT IS WON".to_owned())
        );
        // Two more is one more than they can answer with.
        assert_eq!(full.side_out(2), Next::Won);
        assert_eq!(full.describe(), "away, visitors 2 0 1 2, home side 1 1 1 1");
        assert_eq!(full.verdict(), "YOU WON 5 - 4 IN 4 INNINGS");
    }

    #[test]
    fn away_the_home_side_stops_as_soon_as_it_is_ahead() {
        let mut full = against(false, 4, 2);
        assert_eq!(full.side_out(6), Next::Bat);
        // Six to four behind in the bottom of the last, they would make
        // four, and stop at the three that win it.
        assert_eq!(full.side_out(0), Next::Lost);
        assert_eq!(full.describe(), "away, visitors 6 0, home side 4 3");
        assert_eq!(full.verdict(), "YOU LOST 6 - 7");
    }

    #[test]
    fn away_the_home_side_does_not_bat_last_when_it_is_ahead() {
        let mut full = against(false, 3, 2);
        assert_eq!(full.side_out(1), Next::Bat);
        assert_eq!(full.side_out(1), Next::Lost);
        assert_eq!(full.describe(), "away, visitors 1 1, home side 3 x");
        let [visitors, home] = full.lines();
        assert_eq!(visitors.cells, [Cell::Runs(1), Cell::Runs(1)]);
        assert_eq!(home.cells, [Cell::Runs(3), Cell::NotNeeded]);
        assert_eq!((visitors.runs, home.runs), (2, 3));
    }

    #[test]
    fn at_home_the_visitors_have_batted_before_the_first_ball() {
        let full = against(true, 2, 9);
        assert!(full.at_home());
        assert_eq!((full.ours(), full.theirs()), (0, 2));
        assert_eq!(full.batting_in(), "bottom of innings 1");
        assert_eq!(full.half_words(), "BOT 1ST");
        let report = full.report();
        assert_eq!(report.heading, "TOP OF THE 1ST");
        assert_eq!(
            report.lines,
            [
                "THE VISITORS MADE 2 RUNS",
                "YOU TRAIL 0 - 2",
                "YOU BAT IN THE BOTTOM OF THE 1ST"
            ]
        );
        let [visitors, home] = full.lines();
        assert_eq!((visitors.name, home.name), ("THEM", "YOU"));
        assert_eq!(visitors.cells[0], Cell::Runs(2));
        assert!(home.cells.iter().all(|cell| *cell == Cell::Blank));
    }

    #[test]
    fn at_home_the_last_half_is_not_needed_when_ahead() {
        let mut full = against(true, 1, 2);
        // Three to one up after the first, and three to two after the top
        // of the second: there is nothing left to bat for.
        assert_eq!(full.side_out(3), Next::Won);
        assert_eq!(full.describe(), "at home, visitors 1 1, home side 3 x");
    }

    #[test]
    fn at_home_getting_ahead_in_the_last_innings_wins_there_and_then() {
        let mut full = against(true, 1, 2);
        assert!(!full.sudden());
        assert_eq!(full.side_out(1), Next::Bat);
        // One behind after the top of the last.
        assert!(full.sudden());
        let report = full.report();
        assert_eq!(report.heading, "TOP OF THE 2ND");
        assert_eq!(report.lines[1], "YOU TRAIL 1 - 2");
        assert_eq!(report.lines[3], "2 RUNS WILL WIN THE MATCH");
        full.walked_off(2);
        assert!(!full.sudden());
        assert_eq!(full.describe(), "at home, visitors 1 1, home side 1 2");
    }

    #[test]
    fn at_home_a_level_match_goes_on_and_one_behind_is_lost() {
        let mut full = against(true, 1, 1);
        assert!(full.sudden());
        assert_eq!(full.side_out(1), Next::Bat);
        assert_eq!(full.batting_in(), "bottom of innings 2");
        assert_eq!(full.side_out(0), Next::Lost);
        assert_eq!(full.describe(), "at home, visitors 1 1, home side 1 0");
    }

    #[test]
    fn a_long_match_shows_its_last_nine_innings() {
        let mut full = against(false, 0, 9);
        assert_eq!(full.shown(), (1, 9));
        for _ in 0..11 {
            assert_eq!(full.side_out(0), Next::Bat);
        }
        assert_eq!(full.shown(), (3, 9));
        let [visitors, _] = full.lines();
        assert_eq!(visitors.cells.len(), 9);
        // A short one has no more columns than innings.
        assert_eq!(against(true, 0, 3).shown(), (1, 3));
    }

    #[test]
    fn with_every_hit_a_home_run_the_other_side_makes_more() {
        let rules = Rules::default().full_match;
        let total = |zinger: bool| {
            (0..40)
                .map(|seed| FullMatch::new(true, &rules, Difficulty::Hard, zinger, seed).theirs())
                .sum::<u32>()
        };
        assert!(total(true) > total(false) * 3 / 2);
    }

    #[test]
    fn the_same_seed_gives_the_same_match() {
        let rules = Rules::default().full_match;
        let play = |seed| {
            let mut full = FullMatch::new(false, &rules, Difficulty::Medium, false, seed);
            for _ in 0..8 {
                full.side_out(1);
            }
            full.describe()
        };
        assert_eq!(play(3), play(3));
        assert_ne!(play(3), play(4));
    }
}
