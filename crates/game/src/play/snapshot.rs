//! How a game stands, set down as plain facts and written out as one line.
//!
//! A script, a test or an inspector asks the game where it is and gets this.
//! The line it prints as is read by the tests, a word at a time, so the
//! words and their order are fixed here and nowhere else: what is said, in
//! what order, and to how many places.

use std::fmt::{self, Display, Formatter};

use super::Phase;
use super::pitch::{Kind, Point};

/// Where a game is and how it stands.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Snapshot {
    pub phase: Phase,
    pub standing: Standing,
    /// The pitch in hand, while there is a batting view.
    pub pitch: Option<PitchSeen>,
    pub mods: ModsSeen,
}

/// The score, in whichever kind of game it is.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Standing {
    /// The arcade game has points, and pitches still to come.
    Arcade { points: u32, pitches_left: u32 },
    Match {
        score: Score,
        outs: u32,
        balls: u32,
        strikes: u32,
        /// Whether a runner stands on first, second and third.
        bases: [bool; 3],
        pitched: u32,
        /// In a full match, what each side made in every innings so far,
        /// as the match itself tells it.
        innings: Option<String>,
    },
}

/// What the runs are counted against.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Score {
    /// The last innings has a number of runs to reach.
    Of { score: u32, target: u32 },
    /// A full match has the other side's runs, and says which half of
    /// which innings is being played.
    Against {
        batting_in: String,
        score: u32,
        theirs: u32,
    },
}

/// What a script needs to know of a pitch to time a swing at it, and what
/// has come of it so far.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PitchSeen {
    /// Where it crosses the plate.
    pub crosses: Point,
    /// How many frames it takes to get there.
    pub frames: usize,
    pub in_zone: bool,
    /// With the timing bar up, the first and last of the steps it shows as
    /// the best to swing on.
    pub best: Option<(usize, usize)>,
    /// How far the ball went, if it was hit for a zinger.
    pub zinger_feet: Option<u32>,
    pub mystery: Option<Kind>,
    pub golden: bool,
    pub rebounds: u32,
    /// Where on the outfield the shot was called.
    pub called: Option<Point>,
    /// Where the ball first came down.
    pub came_down: Option<Point>,
}

/// What the mods that keep a tally have to say for themselves. Each is as
/// it would be with its mod off unless the mod is on and has something to
/// tell.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ModsSeen {
    /// How often the fielders have let the ball go.
    pub let_go: u32,
    pub heat: u32,
    pub hits_in_a_row: u32,
    pub rally: u32,
    pub clutch: bool,
    pub southpaw: bool,
    /// What is left on the bullet-time meter, and whether the ball is
    /// being held back this frame.
    pub bullet_time: Option<(u32, bool)>,
    /// Which sign is lit, counting from 0.
    pub sign_lit: Option<usize>,
    /// The sign the ball struck, counting from 0, and the runs it paid.
    pub sign_struck: Option<(usize, u32)>,
    /// The base each runner who is stealing is going to.
    pub stealing: Vec<u8>,
    pub stolen: u32,
    pub caught: u32,
    pub arm: Option<ArmSeen>,
    /// How far the fielders have shifted: to the left below nought, to the
    /// right above it.
    pub shifted: f32,
}

/// How the pitcher's arm is holding up.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ArmSeen {
    /// Pitches this pitcher has thrown.
    pub thrown: u32,
    /// How tired he is, from 0 to 1.
    pub tired: f32,
    /// How many pitchers have been taken off before him.
    pub relieved: u32,
}

impl Display for Snapshot {
    fn fmt(&self, out: &mut Formatter<'_>) -> fmt::Result {
        write!(out, "{:?}", self.phase)?;
        match &self.standing {
            Standing::Arcade {
                points,
                pitches_left,
            } => {
                write!(out, ", {points} points, {pitches_left} pitches left")?;
                if let Some(pitch) = &self.pitch {
                    write!(out, "{pitch}")?;
                }
                // The arcade game has no fielders, runners or runs, so of
                // all the mods only bullet time has anything to add.
                self.mods.bullet_time(out)
            }
            Standing::Match {
                score,
                outs,
                balls,
                strikes,
                bases,
                pitched,
                innings,
            } => {
                let bases: String = bases
                    .iter()
                    .map(|&taken| if taken { 'x' } else { '-' })
                    .collect();
                write!(
                    out,
                    ", {score}, outs {outs}, count {balls}-{strikes}, bases {bases}, pitched {pitched}"
                )?;
                if let Some(pitch) = &self.pitch {
                    write!(out, "{pitch}")?;
                }
                write!(out, "{}", self.mods)?;
                if let Some(innings) = innings {
                    write!(out, ", {innings}")?;
                }
                Ok(())
            }
        }
    }
}

impl Display for Score {
    fn fmt(&self, out: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Score::Of { score, target } => write!(out, "score {score} of {target}"),
            Score::Against {
                batting_in,
                score,
                theirs,
            } => write!(out, "{batting_in}, score {score} to {theirs}"),
        }
    }
}

impl Display for PitchSeen {
    fn fmt(&self, out: &mut Formatter<'_>) -> fmt::Result {
        let (x, y) = self.crosses;
        write!(out, ", crossing {x:.0},{y:.0} after {} frames", self.frames)?;
        if !self.in_zone {
            write!(out, " outside the zone")?;
        }
        if let Some((first, last)) = self.best {
            write!(out, ", best swung on steps {first} to {last}")?;
        }
        if let Some(feet) = self.zinger_feet {
            write!(out, ", a zinger of {feet} feet")?;
        }
        if let Some(kind) = self.mystery {
            write!(out, ", mystery {}", kind.words().to_lowercase())?;
        }
        if self.golden {
            write!(out, ", golden")?;
        }
        if self.rebounds > 0 {
            write!(out, ", rebounds {}", self.rebounds)?;
        }
        if let Some((x, y)) = self.called {
            write!(out, ", called {x:.0},{y:.0}")?;
        }
        if let Some((x, y)) = self.came_down {
            write!(out, ", came down at {x:.0},{y:.0}")?;
        }
        Ok(())
    }
}

impl ModsSeen {
    fn bullet_time(&self, out: &mut Formatter<'_>) -> fmt::Result {
        if let Some((left, slowed)) = self.bullet_time {
            write!(out, ", bullet time {left}")?;
            if slowed {
                write!(out, " slowed")?;
            }
        }
        Ok(())
    }
}

impl Display for ModsSeen {
    fn fmt(&self, out: &mut Formatter<'_>) -> fmt::Result {
        if self.let_go > 0 {
            write!(out, ", let go {}", self.let_go)?;
        }
        if self.heat > 0 {
            write!(out, ", heat {}", self.heat)?;
        }
        if self.hits_in_a_row > 0 {
            write!(out, ", hits in a row {}", self.hits_in_a_row)?;
        }
        if self.rally > 0 {
            write!(out, ", rally {}", self.rally)?;
        }
        if self.clutch {
            write!(out, ", clutch")?;
        }
        if self.southpaw {
            write!(out, ", southpaw")?;
        }
        self.bullet_time(out)?;
        // Signs are counted from 1 where they are told of.
        if let Some(lit) = self.sign_lit {
            write!(out, ", sign {} lit", lit + 1)?;
        }
        if let Some((sign, runs)) = self.sign_struck {
            write!(out, ", struck sign {} for {runs}", sign + 1)?;
        }
        for to in &self.stealing {
            write!(out, ", stealing {to}")?;
        }
        if self.stolen + self.caught > 0 {
            write!(out, ", stolen {}, caught {}", self.stolen, self.caught)?;
        }
        if let Some(arm) = &self.arm {
            write!(out, ", arm {} tired {:.2}", arm.thrown, arm.tired)?;
            if arm.relieved > 0 {
                write!(out, ", pitcher {}", arm.relieved + 1)?;
            }
        }
        if self.shifted != 0.0 {
            let way = if self.shifted < 0.0 { "left" } else { "right" };
            write!(out, ", shifted {way} {:.2}", self.shifted.abs())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_match(phase: Phase) -> Snapshot {
        Snapshot {
            phase,
            standing: Standing::Match {
                score: Score::Of {
                    score: 0,
                    target: 3,
                },
                outs: 0,
                balls: 0,
                strikes: 0,
                bases: [false; 3],
                pitched: 1,
                innings: None,
            },
            pitch: Some(a_pitch()),
            mods: ModsSeen::default(),
        }
    }

    fn a_pitch() -> PitchSeen {
        PitchSeen {
            crosses: (309.4, 196.6),
            frames: 66,
            in_zone: true,
            best: None,
            zinger_feet: None,
            mystery: None,
            golden: false,
            rebounds: 0,
            called: None,
            came_down: None,
        }
    }

    #[test]
    fn a_match_with_no_mods_says_the_score_the_count_and_the_pitch() {
        assert_eq!(
            a_match(Phase::Flight { step: 25 }).to_string(),
            "Flight { step: 25 }, score 0 of 3, outs 0, count 0-0, bases ---, pitched 1, \
             crossing 309,197 after 66 frames"
        );
    }

    #[test]
    fn a_game_between_pitches_says_nothing_of_a_pitch() {
        let between = Snapshot {
            pitch: None,
            ..a_match(Phase::Arriving)
        };
        assert_eq!(
            between.to_string(),
            "Arriving, score 0 of 3, outs 0, count 0-0, bases ---, pitched 1"
        );
    }

    #[test]
    fn the_bases_are_marked_from_first_to_third() {
        let mut snapshot = a_match(Phase::Ready);
        let Standing::Match {
            bases,
            outs,
            balls,
            strikes,
            ..
        } = &mut snapshot.standing
        else {
            unreachable!("a match was made");
        };
        (*bases, *outs, *balls, *strikes) = ([true, false, true], 2, 3, 1);
        snapshot.pitch = None;
        assert_eq!(
            snapshot.to_string(),
            "Ready, score 0 of 3, outs 2, count 3-1, bases x-x, pitched 1"
        );
    }

    #[test]
    fn everything_about_a_pitch_is_said_in_its_order() {
        let pitch = PitchSeen {
            in_zone: false,
            best: Some((42, 43)),
            zinger_feet: Some(497),
            mystery: Some(Kind::ChangeUp),
            golden: true,
            rebounds: 2,
            called: Some((200.0, 160.4)),
            came_down: Some((310.5, 88.0)),
            ..a_pitch()
        };
        assert_eq!(
            pitch.to_string(),
            ", crossing 309,197 after 66 frames outside the zone, best swung on steps 42 to 43, \
             a zinger of 497 feet, mystery change-up, golden, rebounds 2, called 200,160, \
             came down at 310,88"
        );
    }

    #[test]
    fn every_mod_with_something_to_tell_tells_it_in_its_order() {
        let mods = ModsSeen {
            let_go: 3,
            heat: 2,
            hits_in_a_row: 4,
            rally: 1,
            clutch: true,
            southpaw: true,
            bullet_time: Some((60, true)),
            sign_lit: Some(1),
            sign_struck: Some((0, 3)),
            stealing: vec![2, 3],
            stolen: 1,
            caught: 2,
            arm: Some(ArmSeen {
                thrown: 12,
                tired: 0.1,
                relieved: 1,
            }),
            shifted: -0.125,
        };
        assert_eq!(
            mods.to_string(),
            ", let go 3, heat 2, hits in a row 4, rally 1, clutch, southpaw, \
             bullet time 60 slowed, sign 2 lit, struck sign 1 for 3, stealing 2, stealing 3, \
             stolen 1, caught 2, arm 12 tired 0.10, pitcher 2, shifted left 0.12"
        );
    }

    #[test]
    fn mods_with_nothing_to_tell_say_nothing() {
        assert_eq!(ModsSeen::default().to_string(), "");
        // A fresh arm is told of, and a first pitcher is not numbered.
        let fresh = ModsSeen {
            arm: Some(ArmSeen {
                thrown: 0,
                tired: 0.0,
                relieved: 0,
            }),
            shifted: 0.25,
            bullet_time: Some((120, false)),
            ..ModsSeen::default()
        };
        assert_eq!(
            fresh.to_string(),
            ", bullet time 120, arm 0 tired 0.00, shifted right 0.25"
        );
    }

    #[test]
    fn a_full_match_says_the_innings_both_scores_and_how_it_has_gone() {
        let snapshot = Snapshot {
            standing: Standing::Match {
                score: Score::Against {
                    batting_in: "top of innings 1".to_owned(),
                    score: 0,
                    theirs: 0,
                },
                outs: 0,
                balls: 0,
                strikes: 0,
                bases: [false; 3],
                pitched: 1,
                innings: Some("away".to_owned()),
            },
            mods: ModsSeen {
                heat: 1,
                ..ModsSeen::default()
            },
            ..a_match(Phase::Flight { step: 2 })
        };
        assert_eq!(
            snapshot.to_string(),
            "Flight { step: 2 }, top of innings 1, score 0 to 0, outs 0, count 0-0, bases ---, \
             pitched 1, crossing 309,197 after 66 frames, heat 1, away"
        );
    }

    #[test]
    fn the_arcade_game_says_its_points_its_pitches_and_only_bullet_time() {
        let snapshot = Snapshot {
            phase: Phase::Settling { left: 92 },
            standing: Standing::Arcade {
                points: 50,
                pitches_left: 10,
            },
            pitch: Some(PitchSeen {
                best: Some((42, 43)),
                ..a_pitch()
            }),
            // Anything else is left out, whatever it holds.
            mods: ModsSeen {
                heat: 3,
                southpaw: true,
                bullet_time: Some((120, false)),
                ..ModsSeen::default()
            },
        };
        assert_eq!(
            snapshot.to_string(),
            "Settling { left: 92 }, 50 points, 10 pitches left, crossing 309,197 after 66 frames, \
             best swung on steps 42 to 43, bullet time 120"
        );
    }
}
