//! The scorebook of a full match: every pitch to every batter of both
//! sides, and the figures that come of them.
//!
//! The player's own side is written up as it is played. The other side's
//! innings are played out on paper, and written up the same way, so that
//! everything said of either side is worked out from the same kind of
//! record by the same sums.

mod figures;
#[cfg(test)]
mod properties;
mod rows;
mod sum;
mod turn;

pub use figures::{Figures, average, percent, tenths};
pub use rows::{
    BATTING_HEADS, Row, batting_cells, besides, hitting_rows, innings_pitched, pitcher,
    pitching_rows,
};
pub use turn::{End, Hit, Pitch, Steal, Thrown, Turn};

/// How many batters make up the order before it comes round again.
pub const ORDER: usize = 9;

/// A turn that is still going on.
#[derive(Clone, Debug, PartialEq)]
struct Open {
    innings: u32,
    order: usize,
    outs: u32,
    on: [bool; 3],
    pitches: Vec<Pitch>,
}

/// One side's part of the book.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Side {
    pub turns: Vec<Turn>,
    open: Option<Open>,
    /// The runs made by each place in the order.
    pub runs: [u32; ORDER],
    /// How many runners were left on base at the end of each half.
    pub left: Vec<u32>,
    /// The errors this side made in the field.
    pub errors: u32,
    /// Its runners' tries at stealing a base, in the order they were made.
    pub steals: Vec<Steal>,
}

impl Side {
    /// A batter comes up, unless one is up already.
    pub fn come_up(&mut self, innings: u32, order: usize, outs: u32, on: [bool; 3]) {
        if self.open.is_none() {
            self.open = Some(Open {
                innings,
                order,
                outs,
                on,
                pitches: Vec::new(),
            });
        }
    }

    /// A pitch to the batter who is up.
    pub fn pitch(&mut self, pitch: Pitch) {
        if let Some(open) = &mut self.open {
            open.pitches.push(pitch);
        }
    }

    /// The turn of the batter who is up is over.
    pub fn close(&mut self, end: End, ball: Option<Hit>, runs_in: u32, outs_made: u32) {
        if let Some(open) = self.open.take() {
            self.turns.push(Turn {
                innings: open.innings,
                order: open.order,
                outs: open.outs,
                on: open.on,
                pitches: open.pitches,
                end,
                ball,
                runs_in,
                outs_made,
            });
        }
    }

    /// A turn that was begun is not to be finished: the side is out, or
    /// the match is over.
    pub fn abandon(&mut self) {
        self.open = None;
    }

    /// The turns of one innings.
    pub fn innings(&self, innings: u32) -> impl Iterator<Item = &Turn> {
        self.turns
            .iter()
            .filter(move |turn| turn.innings == innings)
    }

    /// The hits made in one innings.
    pub fn hits_in(&self, innings: u32) -> u32 {
        self.innings(innings).filter(|turn| turn.end.hit()).count() as u32
    }

    /// A runner tried to steal a base, in the innings the side is batting
    /// in.
    pub fn stole(&mut self, innings: u32, order: usize, base: u8, safe: bool) {
        self.steals.push(Steal {
            innings,
            order,
            base,
            safe,
            at: self.turns.len(),
        });
    }

    /// How many of the side have been put out: at the plate, in the field
    /// and stealing.
    pub fn outs(&self) -> u32 {
        let batting: u32 = self.turns.iter().map(|turn| turn.outs_made).sum();
        batting + self.steals.iter().filter(|steal| !steal.safe).count() as u32
    }

    /// What happened in one innings, in order, a line for each batter's
    /// turn and each try at stealing a base, with whether it brought a run
    /// in.
    pub fn told(&self, innings: u32) -> Vec<(String, bool)> {
        let mut steals = self
            .steals
            .iter()
            .filter(|steal| steal.innings == innings)
            .peekable();
        let mut told = Vec::new();
        for (index, turn) in self.turns.iter().enumerate() {
            if turn.innings != innings {
                continue;
            }
            // A base stolen while he was up is told before he is.
            while let Some(steal) = steals.next_if(|steal| steal.at <= index) {
                told.push((steal.words(), false));
            }
            told.push((turn.words(), turn.runs_in > 0));
        }
        told.extend(steals.map(|steal| (steal.words(), false)));
        told
    }

    /// The figures of the whole side.
    pub fn figures(&self) -> Figures {
        let mut figures = Figures::of(self.turns.iter());
        figures.runs = self.runs.iter().sum();
        figures.left = self.left.iter().sum();
        figures.steal(self.steals.iter());
        figures
    }

    /// The figures of one place in the order.
    pub fn figures_of(&self, order: usize) -> Figures {
        let mut figures = Figures::of(self.turns.iter().filter(|turn| turn.order == order));
        figures.runs = self.runs.get(order).copied().unwrap_or(0);
        figures.steal(self.steals.iter().filter(|steal| steal.order == order));
        figures
    }
}

/// The book of both sides.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Book {
    pub ours: Side,
    pub theirs: Side,
}

#[cfg(test)]
mod tests;
