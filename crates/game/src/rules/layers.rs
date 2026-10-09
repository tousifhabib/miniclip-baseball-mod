//! Laying files of rules over one another: the last to name a number
//! wins.

use anyhow::{Context, Result};
use toml::{Table, Value};

use super::Rules;
use super::faults::all_are_numbers;

/// The file that holds every number, as built into the program.
pub(super) const BUILT_IN: &str = include_str!("../../../../data/rules.toml");

impl Rules {
    /// The built-in rules with each of `layers` laid over them in turn.
    /// Each layer is the name of where it came from, for reporting a
    /// mistake in it, and its text.
    pub fn layered(layers: &[(&str, &str)]) -> Result<Rules> {
        let mut all: Table = BUILT_IN.parse().context("reading the built-in rules")?;
        for (name, text) in layers {
            let layer: Table = text.parse().with_context(|| format!("reading {name}"))?;
            lay_over(&mut all, layer);
            // Checked after every layer, so that a mistake is laid at the
            // door of the file that made it.
            Rules::from_table(all.clone()).with_context(|| format!("in {name}"))?;
        }
        Rules::from_table(all).context("in the built-in rules")
    }

    fn from_table(table: Table) -> Result<Rules> {
        all_are_numbers(&table, "")?;
        let rules: Rules = table.try_into()?;
        rules.can_be_played_by()?;
        Ok(rules)
    }
}

/// Puts everything in `layer` into `base`. A table is merged with the table
/// already there, so that naming one number in it leaves its other numbers
/// alone. Anything else replaces what was there.
fn lay_over(base: &mut Table, layer: Table) {
    for (key, value) in layer {
        match (base.get_mut(&key), value) {
            (Some(Value::Table(under)), Value::Table(over)) => lay_over(under, over),
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
}
