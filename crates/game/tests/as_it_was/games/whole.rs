//! The games of the record that are played to their end, by a batter or
//! by a monkey.

use bb_game::mods::Mod;
use bb_game::rules::Rules;
use bb_game::settings::Ground;

use super::{By, Game, ONE_INNINGS, TWO_INNINGS};
use crate::players::Dice;

/// A one-innings full match with every mod on, played to its end and its
/// pages turned: the batter stays on the result screen and presses what he
/// finds there.
pub fn finished_matches() -> Vec<Game> {
    [Ground::Home, Ground::Away]
        .into_iter()
        .enumerate()
        .flat_map(|(index, ground)| {
            [("no mods", &[][..]), ("every mod", &Mod::ALL[..])]
                .into_iter()
                .enumerate()
                .map(move |(with, (name, mods))| Game {
                    ground: Some(ground),
                    rules: ONE_INNINGS,
                    most: 18_000,
                    ..Game::new(
                        format!("one innings {}, {name}", ground.word().to_lowercase()),
                        "fullMatch",
                        380 + (index * 2 + with) as u64,
                        mods,
                    )
                })
        })
        .collect()
}

/// Games played by a monkey: a dozen of each kind and of the menu, each with
/// whatever mods its own dice turn up.
pub fn monkeys() -> Vec<Game> {
    let rules = Rules::default();
    let mut games = Vec::new();
    for (called, screen) in [
        ("last innings", "match"),
        ("arcade", "arcade"),
        ("full match", "fullMatch"),
        ("menu", "menu"),
    ] {
        for monkey in 0..12 {
            let seed = 400 + games.len() as u64;
            let mut dice = Dice::new(seed ^ 0x6d6f_6e6b_6579);
            let mods: Vec<Mod> = Mod::ALL.into_iter().filter(|_| dice.chance(30)).collect();
            let levels = mods
                .iter()
                .filter(|which| which.levels(&rules) > 0)
                .map(|&which| (which, 1 + dice.below(u32::from(which.levels(&rules))) as u8))
                .collect();
            games.push(Game {
                levels,
                rules: TWO_INNINGS,
                by: By::Monkey,
                most: 9_600,
                ..Game::new(format!("monkey {monkey}, {called}"), screen, seed, &mods)
            });
        }
    }
    games
}

/// Whole matches of nine innings, at home and away, with no mods and with
/// all of them. These are only played when the whole record is asked for.
pub fn whole_matches() -> Vec<Game> {
    [Ground::Home, Ground::Away]
        .into_iter()
        .enumerate()
        .flat_map(|(index, ground)| {
            [("no mods", &[][..]), ("every mod", &Mod::ALL[..])]
                .into_iter()
                .enumerate()
                .map(move |(with, (name, mods))| Game {
                    ground: Some(ground),
                    most: 150_000,
                    ..Game::new(
                        format!("nine innings {}, {name}", ground.word().to_lowercase()),
                        "fullMatch",
                        390 + (index * 2 + with) as u64,
                        mods,
                    )
                })
        })
        .collect()
}
