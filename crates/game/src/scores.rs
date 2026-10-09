//! The high-score table.
//!
//! The original's table was kept on its publisher's servers and shown by a
//! file loaded from there, which only happened on their site. This one is a
//! file on the player's own machine.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::locate::APP_ID;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub name: String,
    pub points: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Scores {
    /// The longest zinger there has been, in feet. Nought until there has
    /// been one.
    #[serde(default)]
    pub longest_zinger: u32,
    /// The best first.
    #[serde(default)]
    pub entries: Vec<Entry>,
}

impl Scores {
    /// How many scores are kept.
    pub const KEPT: usize = 10;

    /// Where the table is kept: in the game's folder under Application
    /// Support.
    pub fn usual_file() -> Option<PathBuf> {
        let home = std::env::var_os("HOME")?;
        Some(
            PathBuf::from(home)
                .join("Library/Application Support")
                .join(APP_ID)
                .join("scores.toml"),
        )
    }

    /// Reads the table. A file that is missing or cannot be read is an empty
    /// table: a damaged file should not stop anyone playing.
    pub fn load(file: &Path) -> Scores {
        std::fs::read_to_string(file)
            .ok()
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Puts a score in its place. Among equal scores the earlier stays
    /// ahead. Returns whether it made the table.
    pub fn add(&mut self, name: &str, points: u32) -> bool {
        let place = self
            .entries
            .iter()
            .position(|entry| entry.points < points)
            .unwrap_or(self.entries.len());
        if place >= Scores::KEPT {
            return false;
        }
        self.entries.insert(
            place,
            Entry {
                name: name.to_owned(),
                points,
            },
        );
        self.entries.truncate(Scores::KEPT);
        true
    }

    pub fn save(&self, file: &Path) -> Result<()> {
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder)
                .with_context(|| format!("making {}", folder.display()))?;
        }
        let text = toml::to_string(self).context("writing out the scores")?;
        std::fs::write(file, text).with_context(|| format!("writing {}", file.display()))
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn scores_are_kept_best_first_and_no_more_than_ten() {
        let mut scores = Scores::default();
        for points in [50, 300, 100, 300, 0] {
            assert!(scores.add(&format!("p{points}"), points));
        }
        let order: Vec<u32> = scores.entries.iter().map(|entry| entry.points).collect();
        assert_eq!(order, [300, 300, 100, 50, 0]);
        for points in 1..=20 {
            scores.add("more", 1000 + points);
        }
        assert_eq!(scores.entries.len(), Scores::KEPT);
        assert_eq!(scores.entries[0].points, 1020);
        // Too low for a full table.
        assert!(!scores.add("late", 5));
    }

    #[test]
    fn of_two_equal_scores_the_earlier_stays_ahead() {
        let mut scores = Scores::default();
        scores.add("first", 100);
        scores.add("second", 100);
        assert_eq!(scores.entries[0].name, "first");
    }

    #[test]
    fn the_table_comes_back_as_it_was_saved() {
        let file = std::env::temp_dir().join(format!("bb-scores-{}.toml", std::process::id()));
        let mut scores = Scores::default();
        scores.add("Red Sox", 450);
        scores.add("A \"quoted\" name", 75);
        scores.longest_zinger = 812;
        scores.save(&file).unwrap();
        assert_eq!(Scores::load(&file), scores);
        std::fs::remove_file(&file).unwrap();
        // No file, or nonsense in it, is an empty table.
        assert_eq!(Scores::load(&file), Scores::default());
        std::fs::write(&file, "not a table at all [").unwrap();
        assert_eq!(Scores::load(&file), Scores::default());
        std::fs::remove_file(&file).unwrap();
    }

    proptest! {
        #[test]
        fn the_table_is_the_ten_best_of_all_that_were_added_the_earlier_first_among_equals(
            // Few enough names and points that many scores are level, and
            // some are the same score by the same name.
            added in prop::collection::vec(("[A-D]{1,2}", 0u32..12), 0..40),
        ) {
            let mut scores = Scores::default();
            for (number, (name, points)) in added.iter().enumerate() {
                // A score makes the table unless ten before it were as good
                // or better, and goes in behind the ones that were.
                let ahead = added[..number]
                    .iter()
                    .filter(|(_, earlier)| earlier >= points)
                    .count();
                let made_it = scores.add(name, *points);
                prop_assert_eq!(made_it, ahead < Scores::KEPT);
                if made_it {
                    let entry = &scores.entries[ahead];
                    prop_assert_eq!((&entry.name, entry.points), (name, *points));
                }
            }
            // Sorting leaves level scores in the order they came in.
            let mut best = added.clone();
            best.sort_by_key(|&(_, points)| std::cmp::Reverse(points));
            best.truncate(Scores::KEPT);
            let table: Vec<(String, u32)> = scores
                .entries
                .iter()
                .map(|entry| (entry.name.clone(), entry.points))
                .collect();
            prop_assert_eq!(table, best);
        }

        #[test]
        fn any_table_written_out_reads_back_as_it_was(
            longest_zinger: u32,
            // Names of any letters and marks at all, the kinds that have to
            // be written specially among them.
            entries in prop::collection::vec(
                (prop::collection::vec(any::<char>(), 0..12), any::<u32>()),
                0..12,
            ),
        ) {
            let entries = entries
                .into_iter()
                .map(|(name, points)| Entry {
                    name: name.into_iter().collect(),
                    points,
                })
                .collect();
            let scores = Scores {
                longest_zinger,
                entries,
            };
            // What `save` puts in the file, and what `load` makes of it,
            // with no file between them.
            let written = toml::to_string(&scores).unwrap();
            let read: Scores = toml::from_str(&written).unwrap();
            prop_assert_eq!(read, scores, "written as {:?}", written);
        }
    }
}
