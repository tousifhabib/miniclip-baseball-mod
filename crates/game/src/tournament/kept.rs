//! A tournament as it is kept from one run to the next: a file beside the
//! scores, written whenever there is a result to put in it.
//!
//! What is written is what a tournament is: what was chosen, the sides as
//! they were drawn, and the cards. Reading it back plays the cards into a
//! tournament begun afresh, each in its turn, so that a file that has been
//! meddled with or damaged is found out, and reads as no tournament kept.
//! A side's strength is not written. It is the rules' to say, by the key
//! the side is kept under, each time a fixture is played.

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::{Card, Entrant, Format, Setup, Tournament};
use crate::kept;
use crate::look::{self, Rgb};
use crate::settings::Difficulty;

/// Which way of writing a tournament out this is. A file written another
/// way is not read.
const VERSION: u32 = 1;

/// A side as it is written out.
#[derive(Serialize, Deserialize)]
struct KeptSide {
    key: String,
    name: String,
    short: String,
    /// Its colour, as `#rrggbb`.
    colour: String,
}

/// A tournament as it is written out.
#[derive(Serialize, Deserialize)]
struct Kept {
    version: u32,
    /// Sixteen figures to the base of sixteen. A seed is a larger number
    /// than the file's kind of file has room for as a number.
    seed: String,
    format: String,
    innings: u32,
    skill: String,
    player: usize,
    begun: u32,
    sides: Vec<KeptSide>,
    #[serde(default)]
    cards: Vec<Card>,
}

/// A colour written as `#rrggbb`.
fn written([red, green, blue]: Rgb) -> String {
    format!("#{red:02x}{green:02x}{blue:02x}")
}

impl Tournament {
    /// Where a tournament is kept: in the game's folder under Application
    /// Support.
    pub fn usual_file() -> Option<PathBuf> {
        kept::usual_file("tournament.toml")
    }

    fn to_kept(&self) -> Kept {
        let side = |side: &Entrant| KeptSide {
            key: side.key.clone(),
            name: side.name.clone(),
            short: side.short.clone(),
            colour: written(side.colour),
        };
        Kept {
            version: VERSION,
            seed: format!("{:016x}", self.seed),
            format: self.setup.format.key().to_owned(),
            innings: self.setup.innings,
            skill: self.setup.skill.label().to_owned(),
            player: self.player,
            begun: self.begun,
            sides: self.sides.iter().map(side).collect(),
            cards: self.played.clone(),
        }
    }

    /// The tournament that was written out, if what was written is one:
    /// in this way of writing, of a shape and a skill level there are,
    /// with the sides that shape has and the player's own where it is said
    /// to be, and with cards that are each the next fixture's.
    fn from_kept(kept: Kept) -> Option<Tournament> {
        if kept.version != VERSION || kept.innings == 0 {
            return None;
        }
        let setup = Setup {
            format: Format::from_key(&kept.format)?,
            innings: kept.innings,
            skill: Difficulty::from_label(&kept.skill)?,
        };
        let sides: Vec<Entrant> = kept
            .sides
            .into_iter()
            .map(|side| {
                Some(Entrant {
                    key: side.key,
                    name: side.name,
                    short: side.short,
                    colour: look::rgb(&side.colour)?,
                })
            })
            .collect::<Option<_>>()?;
        let players = sides.iter().filter(|side| side.is_the_players()).count();
        let there = sides.get(kept.player).is_some_and(Entrant::is_the_players);
        if sides.len() != setup.format.sides() || players != 1 || !there {
            return None;
        }
        let mut tournament = Tournament {
            setup,
            seed: u64::from_str_radix(&kept.seed, 16).ok()?,
            sides,
            player: kept.player,
            played: Vec::new(),
            begun: kept.begun,
        };
        for card in kept.cards {
            tournament.take(card).ok()?;
        }
        Some(tournament)
    }

    /// Reads the tournament kept in `file`. `None` if there is none, or
    /// the file cannot be read as one: a damaged file should not stop
    /// anyone playing.
    pub fn load(file: &Path) -> Option<Tournament> {
        Tournament::from_kept(kept::read(file)?)
    }

    pub fn save(&self, file: &Path) -> Result<()> {
        kept::write(file, &self.to_kept(), "the tournament")
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::rules::Rules;
    use crate::tournament::testing::{drawn, played_out};

    /// What a tournament is written out as.
    fn text(tournament: &Tournament) -> String {
        toml::to_string(&tournament.to_kept()).expect("a tournament that writes out")
    }

    /// The tournament that this text is, if it is one.
    fn read(text: &str) -> Option<Tournament> {
        Tournament::from_kept(toml::from_str(text).ok()?)
    }

    /// A league part of the way through: its first round played.
    fn begun() -> Tournament {
        let rules = Rules::default();
        let mut rounds = 0;
        let mut after_one = None;
        played_out(drawn(Format::League, 3, 5), &rules, |tournament| {
            if rounds == 1 {
                after_one = Some(tournament.clone());
            }
            rounds += 1;
        });
        after_one.expect("a second round")
    }

    #[test]
    fn a_tournament_reads_back_as_it_was_at_every_point_of_it() {
        let rules = Rules::default();
        for format in Format::ALL {
            let fresh = drawn(format, 3, 11);
            assert_eq!(read(&text(&fresh)), Some(fresh.clone()));
            let done = played_out(fresh, &rules, |tournament| {
                assert_eq!(read(&text(tournament)).as_ref(), Some(tournament));
            });
            assert_eq!(read(&text(&done)), Some(done));
        }
    }

    #[test]
    fn it_is_kept_in_a_file_and_a_file_that_is_missing_or_damaged_is_none_kept() {
        let folder = std::env::temp_dir().join(format!("bb-tournament-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let file = folder.join("tournament.toml");
        assert_eq!(Tournament::load(&file), None);
        let tournament = begun();
        tournament.save(&file).expect("a file that writes");
        assert_eq!(Tournament::load(&file), Some(tournament));
        std::fs::write(&file, "version = 1\nthis is = = not toml").expect("a file");
        assert_eq!(Tournament::load(&file), None);
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn what_is_not_a_tournament_is_not_read_as_one() {
        let tournament = begun();
        let good = text(&tournament);
        assert!(read(&good).is_some());
        let changed = |from: &str, to: &str| {
            assert!(good.contains(from), "{from:?} is not in the file");
            read(&good.replacen(from, to, 1))
        };
        // Another way of writing, a shape or a skill there is not, and a
        // match of no innings.
        assert_eq!(changed("version = 1", "version = 2"), None);
        assert_eq!(changed("format = \"league\"", "format = \"ladder\""), None);
        assert_eq!(changed("skill = \"medium\"", "skill = \"expert\""), None);
        assert_eq!(changed("innings = 3", "innings = 0"), None);
        // A league with the sides of a cup, and a player who is not
        // where the file says.
        assert_eq!(changed("format = \"league\"", "format = \"cup\""), None);
        let player = format!("player = {}", tournament.player());
        let elsewhere = format!("player = {}", (tournament.player() + 1) % 6);
        assert_eq!(changed(&player, &elsewhere), None);
        assert_eq!(changed("key = \"you\"", "key = \"me\""), None);
        // A seed and a colour that are not ones.
        let seed = format!("seed = \"{:016x}\"", 5);
        assert_eq!(changed(&seed, "seed = \"five\""), None);
        assert_eq!(changed("colour = \"#ffffff\"", "colour = \"white\""), None);
        // A card that is not the next fixture's.
        let first = tournament.cards()[0].fixture;
        let fixture = format!("fixture = {first}\n");
        assert_eq!(changed(&fixture, "fixture = 14\n"), None);
    }

    #[test]
    fn a_side_the_rules_no_longer_have_is_still_the_side_it_was() {
        let mut tournament = begun();
        let other = (tournament.player() + 1) % 6;
        tournament.sides[other].key = "wombats".to_owned();
        let back = read(&text(&tournament)).expect("a tournament still");
        assert_eq!(back.sides()[other].key, "wombats");
        // And is a middling side to play.
        assert_eq!(Rules::default().tournament.strength_of("wombats"), 1.0);
    }

    proptest! {
        #[test]
        fn any_seed_and_any_number_of_fixtures_begun_read_back_the_same(
            seed: u64,
            begun: u32,
            shape in 0usize..3,
        ) {
            let mut tournament = drawn(Format::ALL[shape], 5, seed);
            tournament.begun = begun;
            prop_assert_eq!(read(&text(&tournament)), Some(tournament));
        }
    }
}
