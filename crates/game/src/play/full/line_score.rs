//! What the scoreboard of a full match says: the runs of each innings,
//! the totals, and who won.

use super::FullMatch;

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
    pub name: String,
    /// Whether it is the player's side.
    pub ours: bool,
    pub cells: Vec<Cell>,
    pub runs: u32,
    pub hits: u32,
    /// The errors it made in the field.
    pub errors: u32,
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
pub fn runs_words(runs: u32) -> String {
    match runs {
        0 => "NO RUNS".to_owned(),
        1 => "1 RUN".to_owned(),
        runs => format!("{runs} RUNS"),
    }
}

/// A number of hits, the same way.
pub fn hits_words(hits: u32) -> String {
    match hits {
        0 => "NO HITS".to_owned(),
        1 => "1 HIT".to_owned(),
        hits => format!("{hits} HITS"),
    }
}

impl FullMatch {
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
            let side = if ours {
                &self.book.ours
            } else {
                &self.book.theirs
            };
            Line {
                name: if ours { "YOU" } else { self.them() }.to_owned(),
                ours,
                cells,
                runs: made.iter().sum(),
                hits: side.figures().hits,
                errors: side.errors,
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
        let made = format!(
            "{} ON {}",
            runs_words(self.theirs.last().copied().unwrap_or(0)),
            hits_words(self.book.theirs.hits_in(self.theirs.len() as u32))
        );
        let standing = match ours.cmp(&theirs) {
            std::cmp::Ordering::Greater => format!("YOU LEAD {ours} - {theirs}"),
            std::cmp::Ordering::Less => format!("YOU TRAIL {ours} - {theirs}"),
            std::cmp::Ordering::Equal => format!("IT IS LEVEL AT {ours} - {theirs}"),
        };
        // A side with a name is told of by it, and one without by where it
        // is playing.
        let they = |unnamed: &str| self.their_name().unwrap_or(unnamed).to_owned();
        if self.home {
            let mut lines = vec![
                format!("{} MADE {made}", they("THE VISITORS")),
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
            format!("{} MADE {made}", they("THE HOME SIDE")),
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
