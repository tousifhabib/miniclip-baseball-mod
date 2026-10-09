//! The match put into words: what the tests read, and the corner of the
//! window when it is asked to say where the game is.

use super::Match;
use crate::play::mode::Mode;
use crate::play::snapshot::{ModsSeen, PitchSeen, Score, Snapshot, Standing};

impl Match {
    /// How the game stands, as plain facts.
    pub(crate) fn snapshot(&self) -> Snapshot {
        // Where this pitch crosses and how many frames it takes, which a
        // script needs to know to time a swing. With the timing bar up, the
        // steps it shows as the best to swing on are given too, and how far
        // the ball went if it was hit for a zinger.
        let pitch = self.at.as_ref().map(|at_bat| PitchSeen {
            crosses: at_bat.pitch.crosses,
            frames: at_bat.pitch.samples.len(),
            in_zone: at_bat.pitch.in_zone,
            best: at_bat.timing.as_ref().and_then(|bar| bar.timing.best()),
            zinger_feet: at_bat.zinger.map(|zinger| zinger.feet),
            mystery: at_bat.kind,
            golden: at_bat.golden,
            rebounds: at_bat.rebounds,
            called: at_bat.called.as_ref().map(|called| called.at),
            came_down: at_bat.came_down,
        });
        let stealing = self.runners.bases_being_stolen().collect();
        let mods = ModsSeen {
            let_go: self.mods.let_go(),
            heat: self.mods.heat(),
            hits_in_a_row: self.mods.hits_in_a_row(),
            rally: self.mods.in_a_row(),
            clutch: self.mods.clutch_this_pitch(),
            southpaw: self.mods.batting_left_handed(),
            bullet_time: self.mods.bullet_time(),
            sign_lit: self.mods.sign_lit(),
            sign_struck: self.mods.sign_struck(),
            stealing,
            stolen: self.mods.steals().0,
            caught: self.mods.steals().1,
            arm: self.mods.arm(),
            shifted: self.mods.shifted(),
        };
        let in_a_match = |score: Score, innings: Option<String>| Standing::Match {
            score,
            outs: self.outs,
            balls: self.count.balls,
            strikes: self.count.strikes,
            bases: [1, 2, 3].map(|base| self.runners.on_base(base).is_some()),
            pitched: self.pitched,
            innings,
        };
        let standing = match &self.mode {
            Mode::Arcade(arcade) => Standing::Arcade {
                points: arcade.points,
                pitches_left: arcade.left,
            },
            Mode::LastInnings => {
                let score = Score::Of {
                    score: self.score,
                    target: self.target,
                };
                in_a_match(score, None)
            }
            // A full match has no score to reach: it says which innings it
            // is and what both sides have made, and, at the end, what each
            // made in every innings so far.
            Mode::Full(full) => {
                let score = Score::Against {
                    batting_in: full.batting_in(),
                    score: self.score,
                    theirs: full.theirs(),
                };
                in_a_match(score, Some(full.describe()))
            }
        };
        Snapshot {
            phase: self.phase,
            standing,
            pitch,
            mods,
        }
    }

    /// How the game stands, in one line, for a script, a test or an
    /// inspector. The tests read it, so what it says is fixed where the
    /// snapshot is printed.
    pub fn describe(&self) -> String {
        self.snapshot().to_string()
    }
}
