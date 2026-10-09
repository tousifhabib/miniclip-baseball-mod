//! A half as it is played out on paper: a turn at a time until the side
//! is out or has made what it was to make.

use super::pitches::Miss;
use super::{Half, MOST_TURNS, OUTS, Wanted};
use crate::play::book::{End, ORDER, Pitch, Steal, Turn};
use crate::play::field::Ground;
use crate::rng::Rng;
use crate::rules::{StealRules, TheirBattingRules};

/// One playing of a half.
pub(super) struct Play<'a> {
    made: u32,
    winning: bool,
    innings: u32,
    pub(super) lean: f32,
    pub(super) rules: &'a TheirBattingRules,
    /// What their runners steal by, if they steal at all.
    pub(super) steals: Option<&'a StealRules>,
    pub(super) ground: &'a Ground,
    /// Who is on first, second and third: his place in the order.
    pub(super) bases: [Option<usize>; 3],
    pub(super) outs: u32,
    runs: u32,
    up: usize,
    turns: Vec<Turn>,
    stolen: Vec<Steal>,
    by_order: [u32; ORDER],
    /// Runners who were on their way home when the match was won, and so
    /// never got there.
    stranded: u32,
}

impl<'a> Play<'a> {
    pub(super) fn new(
        wanted: Wanted,
        lean: f32,
        rules: &'a TheirBattingRules,
        ground: &'a Ground,
    ) -> Play<'a> {
        Play {
            made: wanted.made,
            winning: wanted.winning,
            innings: wanted.innings,
            lean,
            rules,
            steals: None,
            ground,
            bases: [None; 3],
            outs: 0,
            runs: 0,
            up: wanted.first_up % ORDER,
            turns: Vec::new(),
            stolen: Vec::new(),
            by_order: [0; ORDER],
            stranded: 0,
        }
    }

    pub(super) fn half(self) -> Half {
        Half {
            turns: self.turns,
            steals: self.stolen,
            left: self.bases.iter().flatten().count() as u32 + self.stranded,
            runs: self.by_order,
            next: self.up,
        }
    }

    /// Plays the half out. It is good if it comes to the runs it was to.
    pub(super) fn out(&mut self, rng: &mut Rng) -> Result<(), Miss> {
        while self.outs < OUTS {
            if self.turns.len() >= MOST_TURNS {
                return Err(Miss::TooMany);
            }
            // A runner thrown out stealing may be the last out there is.
            self.steal(rng);
            if self.outs >= OUTS {
                break;
            }
            self.turn(rng)?;
            if self.winning && self.runs >= self.made {
                return Ok(());
            }
            if self.runs > self.made {
                return Err(Miss::TooMany);
            }
        }
        if self.winning || self.runs < self.made {
            return Err(Miss::TooFew);
        }
        Ok(())
    }

    /// Before a batter's turn, a runner with the base in front of him empty
    /// may go for it: the one on second if there is one, and if he stays
    /// the one on first. He gets there or he is out.
    pub(super) fn steal(&mut self, rng: &mut Rng) {
        let Some(rules) = self.steals else {
            return;
        };
        let [first, second, third] = self.bases;
        let from = if second.is_some() && third.is_none() {
            rng.chance(rules.their_chance * rules.their_third)
                .then_some(1)
        } else if first.is_some() && second.is_none() {
            rng.chance(rules.their_chance).then_some(0)
        } else {
            None
        };
        let Some((from, runner)) = from.and_then(|from| Some((from, self.bases[from].take()?)))
        else {
            return;
        };
        let safe = rng.chance(rules.their_safe);
        if safe {
            self.bases[from + 1] = Some(runner);
        } else {
            self.outs += 1;
        }
        self.stolen.push(Steal {
            innings: self.innings,
            order: runner,
            base: from as u8 + 2,
            safe,
            at: self.turns.len(),
        });
    }

    /// One batter's turn.
    fn turn(&mut self, rng: &mut Rng) -> Result<(), Miss> {
        let order = self.up;
        let (outs, on) = (self.outs, self.bases.map(|base| base.is_some()));
        let (pitches, struck) = self.pitches(rng);
        let mut home: Vec<usize> = Vec::new();
        let mut outs_made = 0;
        let mut ball = None;
        let end = match struck {
            // Three strikes, or four balls.
            None if pitches.last().is_some_and(Pitch::strike) => {
                outs_made = 1;
                End::Strikeout
            }
            None => {
                self.walk(order, &mut home);
                End::Walk
            }
            Some(struck) => {
                let (end, hit) = self.put_in_play(struck, order, &mut home, &mut outs_made, rng);
                ball = Some(hit);
                end
            }
        };
        // The run that wins the match ends it: nobody behind it comes in,
        // unless the ball went out of the park, when they all do.
        let wanted = self.made.saturating_sub(self.runs) as usize;
        if self.winning && home.len() > wanted {
            if end == End::HomeRun {
                return Err(Miss::TooMany);
            }
            self.stranded += (home.len() - wanted) as u32;
            home.truncate(wanted);
        }
        for &scorer in &home {
            self.by_order[scorer] += 1;
        }
        self.runs += home.len() as u32;
        self.outs += outs_made;
        self.up = (order + 1) % ORDER;
        self.turns.push(Turn {
            innings: self.innings,
            order,
            outs,
            on,
            pitches,
            end,
            ball,
            runs_in: home.len() as u32,
            outs_made,
        });
        Ok(())
    }
}
