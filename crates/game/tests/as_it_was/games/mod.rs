//! The games that are played and written down, in groups.
//!
//! Between them they go through every mod alone and the mods that meet one
//! another, in each kind of game, at each skill level, and round the menu.

mod menu;
mod whole;

use bb_game::mods::Mod;
use bb_game::rules::Rules;
use bb_game::settings::Ground;

use crate::players::{Batter, Monkey, Player, Then, Tricks, Written};
pub use menu::the_menu;
pub use whole::{finished_matches, monkeys, whole_matches};

/// Who plays a game.
pub enum By {
    Batter(Tricks),
    Monkey,
    /// Written steps and nothing after them.
    Written(String),
    /// Written steps, to get into a game by way of the menu, and then a
    /// batter.
    WrittenThenBatter(String),
}

/// One game to play.
pub struct Game {
    pub name: String,
    /// The label of the screen it starts on.
    pub screen: &'static str,
    pub seed: u64,
    pub mods: Vec<Mod>,
    /// The settings of the mods that have one, where not the usual.
    pub levels: Vec<(Mod, u8)>,
    pub ground: Option<Ground>,
    /// Numbers laid over the game's own, as a rules file would be.
    pub rules: &'static str,
    pub by: By,
    /// The most frames that are played of it.
    pub most: u32,
}

impl Game {
    fn new(name: String, screen: &'static str, seed: u64, mods: &[Mod]) -> Game {
        Game {
            name,
            screen,
            seed,
            mods: mods.to_vec(),
            levels: Vec::new(),
            ground: None,
            rules: "",
            by: By::Batter(tricks_for(mods)),
            most: 12_000,
        }
    }

    pub fn player(&self) -> Box<dyn Player> {
        // The players' dice are not the game's, but go by the same seed.
        match &self.by {
            By::Batter(tricks) => Box::new(Batter::new(self.seed, *tricks)),
            By::Monkey => Box::new(Monkey::new(self.seed)),
            By::Written(steps) => Box::new(Written::new(steps)),
            By::WrittenThenBatter(steps) => Box::new(Then(
                Box::new(Written::new(steps)),
                Box::new(Batter::new(self.seed, Tricks::default())),
            )),
        }
    }
}

/// What a batter has to do for these mods to have anything to act on.
fn tricks_for(mods: &[Mod]) -> Tricks {
    Tricks {
        slows: mods.contains(&Mod::BulletTime),
        calls: mods.contains(&Mod::CalledShot),
        steals: mods.contains(&Mod::StolenBases),
    }
}

/// A last innings that takes a good many pitches to win or lose, so that a
/// mod has time to do all it does: six outs to get nine runs in.
const A_LONG_INNINGS: &str =
    "[match]\nouts = 6\n[match.runs_down]\neasy = 9\nmedium = 9\nhard = 9\n";
/// A full match short enough to play through: two innings a side.
const TWO_INNINGS: &str = "[full_match]\ninnings = 2\n";
const ONE_INNINGS: &str = "[full_match]\ninnings = 1\n";

/// The kinds of game: what each is called here, the screen it starts on,
/// where a full match is played, the rules laid over it and how many frames
/// are enough for it.
const KINDS: [(&str, &str, Option<Ground>, &str, u32); 4] = [
    ("last innings", "match", None, A_LONG_INNINGS, 12_000),
    ("arcade", "arcade", None, "", 6_000),
    (
        "full match at home",
        "fullMatch",
        Some(Ground::Home),
        TWO_INNINGS,
        12_000,
    ),
    (
        "full match away",
        "fullMatch",
        Some(Ground::Away),
        TWO_INNINGS,
        12_000,
    ),
];

fn of_kind(kind: usize, name: &str, seed: u64, mods: &[Mod]) -> Game {
    let (called, screen, ground, rules, most) = KINDS[kind];
    Game {
        ground,
        rules,
        most,
        ..Game::new(format!("{called}, {name}"), screen, seed, mods)
    }
}

/// No mods, and then each mod on its own, in one kind of game. Each game has
/// a seed of its own, so that no two go the same way.
fn each_mod(kind: usize) -> Vec<Game> {
    let seed = |index: usize| (1 + kind * 30 + index) as u64;
    let alone = Mod::ALL
        .iter()
        .enumerate()
        .map(|(index, &which)| of_kind(kind, which.key(), seed(1 + index), &[which]));
    std::iter::once(of_kind(kind, "no mods", seed(0), &[]))
        .chain(alone)
        .collect()
}

pub fn last_innings() -> Vec<Game> {
    // And the last innings as it is, three outs and no more.
    let as_it_is = Game {
        rules: "",
        ..of_kind(0, "as it is", 29, &[])
    };
    std::iter::once(as_it_is).chain(each_mod(0)).collect()
}

pub fn arcade() -> Vec<Game> {
    each_mod(1)
}

pub fn full_match_at_home() -> Vec<Game> {
    each_mod(2)
}

pub fn full_match_away() -> Vec<Game> {
    each_mod(3)
}

/// Mods that change what one another do, each set in every kind of game.
pub fn mods_together() -> Vec<Game> {
    let sets: [(&str, &[Mod]); 7] = [
        ("zinger and timing", &[Mod::ZingerHit, Mod::TimingIndicator]),
        (
            "what a run is worth",
            &[Mod::GoldenBall, Mod::Clutch, Mod::Rally, Mod::SuddenDeath],
        ),
        ("steals and turbo", &[Mod::StolenBases, Mod::TurboRunners]),
        (
            "pinball and lone pitcher",
            &[Mod::PinballPark, Mod::LonePitcher],
        ),
        ("bullet time at night", &[Mod::BulletTime, Mod::NightGame]),
        (
            "what shapes the pitch",
            &[
                Mod::HeatCheck,
                Mod::TiredArm,
                Mod::HotBat,
                Mod::MysteryPitch,
            ],
        ),
        ("every mod", &Mod::ALL),
    ];
    let mut games = Vec::new();
    for (index, (name, mods)) in sets.into_iter().enumerate() {
        for kind in 0..KINDS.len() {
            let seed = 300 + (index * KINDS.len() + kind) as u64;
            games.push(of_kind(kind, name, seed, mods));
        }
    }
    games
}

/// The mods that have a setting, at its least and at its most.
pub fn levels() -> Vec<Game> {
    let rules = Rules::default();
    let mut games = Vec::new();
    for (index, which) in [
        Mod::Butterfingers,
        Mod::PinballPark,
        Mod::MoonBall,
        Mod::TurboRunners,
    ]
    .into_iter()
    .enumerate()
    {
        let most = which.levels(&rules);
        let seed = 340 + index as u64 * 3;
        for (kind, level, seed) in [(0, 1, seed), (0, most, seed + 1), (2, most, seed + 2)] {
            let name = format!("{} at {level}", which.key());
            games.push(Game {
                levels: vec![(which, level)],
                ..of_kind(kind, &name, seed, &[which])
            });
        }
    }
    games
}

/// Where things are on the menu's pages, in stage pixels.
const BOTTOM_OF_THE_NINTH: &str = "click 200 181";
const FULL_MATCH: &str = "click 200 216";
const ARCADE: &str = "click 200 252";
const MODS: &str = "click 330 360";
const NEXT: &str = "click 490 362";
const BACK: &str = "click 290 362";
const PLAY_BALL: &str = "click 480 362";

/// The easy and the hard game, which can only be chosen on a setup page, so
/// each of these goes in by way of the menu.
pub fn skills() -> Vec<Game> {
    let mut games = Vec::new();
    for (row, called, most) in [
        (BOTTOM_OF_THE_NINTH, "last innings", 12_000),
        (ARCADE, "arcade", 6_600),
        (FULL_MATCH, "full match", 12_000),
    ] {
        for skill in ["EASY", "HARD"] {
            let steps = format!(
                "wait 60; {row}; wait 60; find {skill}; wait 10; {NEXT}; wait 60; {PLAY_BALL}; \
                 wait 60"
            );
            games.push(Game {
                rules: TWO_INNINGS,
                by: By::WrittenThenBatter(steps),
                most,
                ..Game::new(
                    format!("{called}, {}", skill.to_lowercase()),
                    "menu",
                    360 + games.len() as u64,
                    &[],
                )
            });
        }
    }
    games
}
