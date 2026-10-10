//! Whether a set of rules can be played by, and what is wrong with one
//! that cannot.

use anyhow::Result;
use toml::{Table, Value};

use super::Rules;

impl Rules {
    /// Checks the numbers that the game's sums take for granted: a file
    /// may set any of them, and one set wrongly would stop the game in the
    /// middle of a pitch, far from the file that did it.
    pub(super) fn can_be_played_by(&self) -> Result<(), RulesFault> {
        let check = |holds: bool, which: &str, why: &'static str| {
            if holds {
                Ok(())
            } else {
                Err(RulesFault {
                    which: which.to_owned(),
                    why,
                })
            }
        };
        let levels = [
            ("pitch.easy.speed", &self.pitch.easy),
            ("pitch.medium.speed", &self.pitch.medium),
            ("pitch.hard.speed", &self.pitch.hard),
        ];
        for (which, pitch) in levels {
            check(pitch.speed.low >= 1, which, "has to be at least 1")?;
            check(
                pitch.speed.low <= pitch.speed.high,
                which,
                "has its low above its high",
            )?;
        }
        let pop = &self.steal.pop;
        check(
            pop.low <= pop.high,
            "steal.pop",
            "has its low above its high",
        )?;
        check(
            self.throw.approach > 0.0,
            "throw.approach",
            "has to be more than nought",
        )?;
        check(
            self.throw.fade > 0.0,
            "throw.fade",
            "has to be more than nought",
        )?;
        check(
            self.field.pace > 0.0,
            "field.pace",
            "has to be more than nought",
        )?;
        check(
            self.field.wall > 0.0,
            "field.wall",
            "has to be more than nought",
        )?;
        check(
            self.shift.most >= 0.0,
            "shift.most",
            "cannot be less than nought",
        )?;
        check(
            self.zinger.shape_reach >= 0.0,
            "zinger.shape_reach",
            "cannot be less than nought",
        )?;
        self.tournament.can_be_played_by()
    }
}

/// A number of the rules that the game cannot be played by: which it is,
/// and what is wrong with it.
#[derive(Debug, thiserror::Error)]
#[error("`{which}` {why}")]
pub struct RulesFault {
    pub(super) which: String,
    pub(super) why: &'static str,
}

/// Checks that every number in the table is a number: a file can say `nan`
/// or `inf`, and the game can do no sums with either.
pub(super) fn all_are_numbers(table: &Table, inside: &str) -> Result<(), RulesFault> {
    for (key, value) in table {
        let which = if inside.is_empty() {
            key.clone()
        } else {
            format!("{inside}.{key}")
        };
        let numbers = |value: &Value| match value {
            Value::Float(number) => number.is_finite(),
            Value::Array(all) => all
                .iter()
                .all(|each| each.as_float().is_none_or(f64::is_finite)),
            _ => true,
        };
        match value {
            Value::Table(under) => all_are_numbers(under, &which)?,
            value if !numbers(value) => {
                return Err(RulesFault {
                    which,
                    why: "is not a number the game can do sums with",
                });
            }
            _ => {}
        }
    }
    Ok(())
}
