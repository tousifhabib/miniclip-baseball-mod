//! A snapshot written out. This is the one place that fixes the order
//! its parts are said in.

use std::fmt::{self, Display, Formatter};

use super::{ModsSeen, PitchSeen, Score, Snapshot, Standing};

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
