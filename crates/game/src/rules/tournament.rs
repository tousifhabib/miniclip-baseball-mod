//! The numbers of a tournament: how long its matches can be, who counts
//! among its best batters, and the sides there are to play against.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::faults::RulesFault;
use crate::look;

/// The fewest sides there can be: a cup, or two groups of four, takes
/// seven besides the player's.
const FEWEST_SIDES: usize = 7;
/// How strong a side can be, and how weak.
const STRENGTHS: std::ops::RangeInclusive<f32> = 0.25..=4.0;
/// How long a side's name can be, and the same in short.
const LONGEST_NAME: usize = 16;
const LONGEST_SHORT: usize = 4;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TournamentRules {
    /// The numbers of innings a tournament's matches can be chosen to
    /// have.
    pub innings: Vec<u32>,
    /// A batter's averages are listed among the best once he has had a
    /// turn for every this many innings his side has batted in.
    pub innings_a_turn: u32,
    /// The sides there are to draw from, each under the key it is kept by.
    pub sides: BTreeMap<String, SideRules>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SideRules {
    /// What the side is called, in capitals.
    pub name: String,
    /// The same in the few letters a scoreboard has room for.
    pub short: String,
    /// Its colour, written as `#rrggbb`.
    pub colour: String,
    /// How strong it is. At 1 it makes the runs the other side of a full
    /// match makes, above 1 more, and below it fewer.
    pub strength: f32,
}

impl TournamentRules {
    /// The key no side of the rules may have: the player's own side is
    /// kept under it.
    pub const PLAYERS_KEY: &str = "you";

    /// How strong the side kept under `key` is. One the rules do not have
    /// is a middling side.
    pub fn strength_of(&self, key: &str) -> f32 {
        self.sides.get(key).map_or(1.0, |side| side.strength)
    }

    /// Checks what a tournament takes for granted of its numbers and its
    /// sides.
    pub(super) fn can_be_played_by(&self) -> Result<(), RulesFault> {
        let fault = |which: String, why| Err(RulesFault { which, why });
        if self.innings.is_empty() || self.innings.contains(&0) {
            let why = "has to give at least one length of match, and none of no innings";
            return fault("tournament.innings".to_owned(), why);
        }
        if self.innings_a_turn == 0 {
            let which = "tournament.innings_a_turn".to_owned();
            return fault(which, "has to be at least 1");
        }
        if self.sides.len() < FEWEST_SIDES {
            let why = "has to have at least seven sides";
            return fault("tournament.sides".to_owned(), why);
        }
        for (index, (key, side)) in self.sides.iter().enumerate() {
            let of = |what: &str| format!("tournament.sides.{key}.{what}");
            if key == TournamentRules::PLAYERS_KEY {
                let why = "is what the player's own side is kept as";
                return fault(format!("tournament.sides.{key}"), why);
            }
            if !STRENGTHS.contains(&side.strength) {
                return fault(of("strength"), "has to be from 0.25 to 4");
            }
            if !in_capitals(&side.name, LONGEST_NAME, true) {
                let why = "has to be from 1 to 16 capital letters, with spaces between words";
                return fault(of("name"), why);
            }
            if !in_capitals(&side.short, LONGEST_SHORT, false) {
                return fault(of("short"), "has to be from 1 to 4 capital letters");
            }
            if look::rgb(&side.colour).is_none() {
                return fault(of("colour"), "is not a colour written as #rrggbb");
            }
            // Two sides that went by one name could not be told apart.
            let mut earlier = self.sides.values().take(index);
            if earlier.clone().any(|other| other.name == side.name) {
                return fault(of("name"), "is another side's name as well");
            }
            if earlier.any(|other| other.short == side.short) {
                return fault(of("short"), "is another side's short name as well");
            }
        }
        Ok(())
    }
}

/// Whether `name` is from one to `longest` capital letters, with single
/// spaces between its words if it may have more than one.
fn in_capitals(name: &str, longest: usize, words: bool) -> bool {
    let letters = |word: &str| !word.is_empty() && word.chars().all(|c| c.is_ascii_uppercase());
    let fits = (1..=longest).contains(&name.len());
    if words {
        fits && name.split(' ').all(letters)
    } else {
        fits && letters(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rules;

    #[test]
    fn the_built_in_sides_are_enough_for_any_tournament_and_differ_in_strength() {
        let rules = Rules::default().tournament;
        assert!(rules.sides.len() >= FEWEST_SIDES);
        assert_eq!(rules.innings, [3, 5, 9]);
        let strongest = rules.sides.values().map(|side| side.strength);
        let (most, least) = strongest.fold((f32::MIN, f32::MAX), |(most, least), strength| {
            (most.max(strength), least.min(strength))
        });
        assert!(least < 1.0 && 1.0 < most, "{least} to {most}");
        // A side the rules do not have is a middling one.
        assert_eq!(rules.strength_of("rooks"), 1.25);
        assert_eq!(rules.strength_of("nobody"), 1.0);
    }

    #[test]
    fn a_name_is_capitals_with_single_spaces_and_no_longer_than_there_is_room_for() {
        assert!(in_capitals("ASHCOMBE ROOKS", 16, true));
        assert!(in_capitals("ROOK", 4, false));
        for wrong in ["", "Rooks", "ROOKS ", " ROOKS", "TWO  SPACES", "ROOKS 2"] {
            assert!(!in_capitals(wrong, 16, true), "{wrong:?}");
        }
        assert!(!in_capitals("SEVENTEEN LETTERS", 16, true));
        assert!(!in_capitals("RO OK", 5, false));
        assert!(!in_capitals("ROOKS", 4, false));
    }

    #[test]
    fn a_tournament_that_could_not_be_played_is_refused_and_named() {
        let wrong = |layer: &str| {
            let error = Rules::layered(&[("a mod", layer)]).expect_err("rules to be refused");
            format!("{error:#}")
        };
        let of_the_rooks = |line: &str| wrong(&format!("[tournament.sides.rooks]\n{line}\n"));
        assert_eq!(
            wrong("[tournament]\ninnings = [3, 0]\n"),
            "in a mod: `tournament.innings` has to give at least one length of match, \
             and none of no innings"
        );
        assert_eq!(
            wrong("[tournament]\ninnings_a_turn = 0\n"),
            "in a mod: `tournament.innings_a_turn` has to be at least 1"
        );
        assert_eq!(
            of_the_rooks("strength = 9.0"),
            "in a mod: `tournament.sides.rooks.strength` has to be from 0.25 to 4"
        );
        assert_eq!(
            of_the_rooks("strength = nan"),
            "in a mod: `tournament.sides.rooks.strength` is not a number the game can do sums with"
        );
        assert_eq!(
            of_the_rooks("short = \"ROOKS\""),
            "in a mod: `tournament.sides.rooks.short` has to be from 1 to 4 capital letters"
        );
        assert_eq!(
            of_the_rooks("colour = \"grey\""),
            "in a mod: `tournament.sides.rooks.colour` is not a colour written as #rrggbb"
        );
        assert_eq!(
            of_the_rooks("name = \"FENMOUTH KITES\""),
            "in a mod: `tournament.sides.rooks.name` is another side's name as well"
        );
        let you = "[tournament.sides.you]\nname = \"US\"\nshort = \"US\"\n\
                   colour = \"#ffffff\"\nstrength = 1.0\n";
        assert_eq!(
            wrong(you),
            "in a mod: `tournament.sides.you` is what the player's own side is kept as"
        );
        // A layer can change one thing of one side and leave the rest.
        let rules = Rules::layered(&[("a mod", "[tournament.sides.toads]\nstrength = 2.0\n")])
            .expect("rules that read");
        assert_eq!(rules.tournament.strength_of("toads"), 2.0);
        assert_eq!(rules.tournament.sides["toads"].short, "TOAD");
    }
}
