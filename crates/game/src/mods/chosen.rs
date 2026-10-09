//! Which mods are switched on and how each is set, kept between runs.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path as FilePath, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::Mod;
use crate::kept;
use crate::rules::Rules;

/// The mods that are switched on, and what their settings are at. They
/// last from one game to the next.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    on: BTreeSet<Mod>,
    /// The levels that have been set. A mod not here is at its usual one.
    pub(super) levels: BTreeMap<Mod, u8>,
}

/// The mods as they are written to their file: the keys of the ones that
/// are on, and the levels that have been set, by key.
#[derive(Default, Serialize, Deserialize)]
struct Saved {
    #[serde(default)]
    on: Vec<String>,
    #[serde(default)]
    levels: BTreeMap<String, u8>,
}

impl Mods {
    pub fn is_on(&self, which: Mod) -> bool {
        self.on.contains(&which)
    }

    pub fn set(&mut self, which: Mod, on: bool) {
        if on {
            self.on.insert(which);
        } else {
            self.on.remove(&which);
        }
    }

    /// Switches a mod over, and returns whether that left it on.
    pub fn toggle(&mut self, which: Mod) -> bool {
        let on = !self.is_on(which);
        self.set(which, on);
        on
    }

    /// The mods that are on, in the menu's order.
    pub fn all_on(&self) -> impl Iterator<Item = Mod> + '_ {
        self.on.iter().copied()
    }

    /// The level a mod's setting is at, counted from 1. It keeps its level
    /// while the mod is off.
    pub fn level(&self, which: Mod) -> u8 {
        self.levels
            .get(&which)
            .copied()
            .unwrap_or_else(|| which.usual_level())
    }

    /// Puts a mod's setting at a level, counted from 1. Nothing here knows
    /// how many levels the rules give it: see [`Mods::keep_within`].
    pub fn set_level(&mut self, which: Mod, level: u8) {
        self.levels.insert(which, level.max(1));
    }

    /// Brings every setting back within the levels these rules give it. A
    /// level asked for on the command line, or kept in a file written by
    /// hand or for other rules, may be more than there are.
    pub fn keep_within(&mut self, rules: &Rules) {
        for (which, level) in &mut self.levels {
            *level = (*level).clamp(1, which.levels(rules).max(1));
        }
    }

    /// Where the choice is kept: beside the scores, in the game's folder
    /// under Application Support.
    pub fn usual_file() -> Option<PathBuf> {
        kept::usual_file("mods.toml")
    }

    /// Reads which mods are on. A file that is missing or cannot be read
    /// leaves every mod off, and a mod the game no longer has is passed
    /// over: neither should stop anyone playing.
    pub fn load(file: &FilePath) -> Mods {
        let saved: Saved = kept::read(file).unwrap_or_default();
        Mods {
            on: saved
                .on
                .iter()
                .filter_map(|key| Mod::from_key(key))
                .collect(),
            levels: saved
                .levels
                .iter()
                .filter_map(|(key, &level)| Some((Mod::from_key(key)?, level.max(1))))
                .collect(),
        }
    }

    pub fn save(&self, file: &FilePath) -> Result<()> {
        let saved = Saved {
            on: self.all_on().map(|each| each.key().to_owned()).collect(),
            levels: self
                .levels
                .iter()
                .map(|(which, &level)| (which.key().to_owned(), level))
                .collect(),
        };
        kept::write(file, &saved, "the mods")
    }
}
