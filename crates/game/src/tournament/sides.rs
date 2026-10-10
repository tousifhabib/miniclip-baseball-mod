//! The sides of a tournament, and the drawing of them.

use crate::look::{self, Rgb};
use crate::rng::Rng;
use crate::rules::TournamentRules;

/// What the player's side is called when it has not been called anything.
const YOU: &str = "YOU";
/// How many letters a side's short name has, at the most.
const SHORT: usize = 4;
/// How long the name the player gives a side can be.
const LONGEST: usize = 16;

/// A side that is in a tournament.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entrant {
    /// What the rules keep it under, or the player's own key.
    pub key: String,
    /// What it is called, in capitals.
    pub name: String,
    /// The same in the few letters a scoreboard has room for.
    pub short: String,
    pub colour: Rgb,
}

impl Entrant {
    /// The player's own side. `typed` is what the player has called it,
    /// which may be nothing.
    pub fn the_players(typed: &str, colour: Rgb) -> Entrant {
        // In capitals, as every side is, with one space between words.
        let words: Vec<String> = typed
            .split_whitespace()
            .map(|word| {
                word.chars()
                    .filter(char::is_ascii_graphic)
                    .map(|letter| letter.to_ascii_uppercase())
                    .collect::<String>()
            })
            .filter(|word| !word.is_empty())
            .collect();
        let name: String = words.join(" ").chars().take(LONGEST).collect();
        let name = name.trim_end().to_owned();
        let short: String = name
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .take(SHORT)
            .collect();
        let (name, short) = if short.is_empty() {
            (YOU.to_owned(), YOU.to_owned())
        } else {
            (name, short)
        };
        Entrant {
            key: TournamentRules::PLAYERS_KEY.to_owned(),
            name,
            short,
            colour,
        }
    }

    /// Whether this is the player's own side.
    pub fn is_the_players(&self) -> bool {
        self.key == TournamentRules::PLAYERS_KEY
    }
}

/// Draws the sides of a tournament of `count`: the player's, and the rest
/// from the rules by chance. They come back in the order of the draw,
/// which settles who meets whom, with the place the player's side has in
/// it. `None` if the rules have too few sides.
pub fn draw(
    count: usize,
    player: Entrant,
    rules: &TournamentRules,
    rng: &mut Rng,
) -> Option<(Vec<Entrant>, usize)> {
    let mut others: Vec<Entrant> = rules
        .sides
        .iter()
        .map(|(key, side)| Entrant {
            key: key.clone(),
            name: side.name.clone(),
            short: side.short.clone(),
            colour: look::rgb(&side.colour).unwrap_or(look::CREAM),
        })
        .collect();
    if count == 0 || others.len() + 1 < count {
        return None;
    }
    shuffle(&mut others, rng);
    others.truncate(count - 1);
    let mut sides = vec![player];
    sides.append(&mut others);
    shuffle(&mut sides, rng);
    let place = sides.iter().position(Entrant::is_the_players)?;
    Some((sides, place))
}

/// Puts `all` in an order as likely as any other.
fn shuffle<T>(all: &mut [T], rng: &mut Rng) {
    for last in (1..all.len()).rev() {
        let other = rng.below(last as u32 + 1) as usize;
        all.swap(last, other);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::rules::Rules;

    fn drawn(count: usize, seed: u64) -> (Vec<Entrant>, usize) {
        let rules = Rules::default().tournament;
        let player = Entrant::the_players("", look::WHITE);
        draw(count, player, &rules, &mut Rng::new(seed)).expect("sides enough")
    }

    #[test]
    fn the_players_side_goes_by_what_it_was_called_or_by_you() {
        let named = |typed: &str| {
            let side = Entrant::the_players(typed, look::WHITE);
            assert!(side.is_the_players());
            (side.name, side.short)
        };
        assert_eq!(named(""), ("YOU".to_owned(), "YOU".to_owned()));
        assert_eq!(named("   "), ("YOU".to_owned(), "YOU".to_owned()));
        assert_eq!(named("Red Sox"), ("RED SOX".to_owned(), "REDS".to_owned()));
        assert_eq!(
            named("  the  9ers "),
            ("THE 9ERS".to_owned(), "THE9".to_owned())
        );
        assert_eq!(named("ab"), ("AB".to_owned(), "AB".to_owned()));
        // No longer than there is room for, and with no space left at
        // the end by the cutting.
        let (long, _) = named("Upper Netherfield Wanderers");
        assert_eq!(long, "UPPER NETHERFIEL");
        assert_eq!(named("Fifteen letters x y").0, "FIFTEEN LETTERS");
        // A name with no letter or figure in it is no name.
        assert_eq!(named("?!"), ("YOU".to_owned(), "YOU".to_owned()));
        assert_eq!(
            named("O'Neil's"),
            ("O'NEIL'S".to_owned(), "ONEI".to_owned())
        );
    }

    #[test]
    fn a_draw_has_the_player_and_as_many_others_as_it_wants_each_once() {
        for count in [6, 8] {
            for seed in 0..20 {
                let (sides, place) = drawn(count, seed);
                assert_eq!(sides.len(), count);
                assert!(sides[place].is_the_players());
                let keys: BTreeSet<&str> = sides.iter().map(|side| side.key.as_str()).collect();
                assert_eq!(keys.len(), count, "a side drawn twice");
            }
        }
    }

    #[test]
    fn the_same_seed_makes_the_same_draw_and_another_seed_another() {
        assert_eq!(drawn(8, 4), drawn(8, 4));
        let places: BTreeSet<usize> = (0..60).map(|seed| drawn(8, seed).1).collect();
        assert_eq!(places.len(), 8, "the player is never drawn in some place");
        let firsts: BTreeSet<String> = (0..200)
            .map(|seed| drawn(6, seed).0.swap_remove(0).key)
            .collect();
        assert_eq!(firsts.len(), 11, "some side is never drawn first");
    }

    #[test]
    fn rules_with_too_few_sides_make_no_draw() {
        let rules = Rules::default().tournament;
        let player = || Entrant::the_players("", look::WHITE);
        assert!(draw(11, player(), &rules, &mut Rng::new(1)).is_some());
        assert!(draw(12, player(), &rules, &mut Rng::new(1)).is_none());
        assert!(draw(0, player(), &rules, &mut Rng::new(1)).is_none());
    }
}
