//! The mods that are in play in a match.
//!
//! Each mod has a file or a folder of its own here, named as the mod is:
//! what it keeps from one pitch to the next, its rules, and what it draws.
//! The game does not ask whether a mod is switched on. It asks the mods a
//! question, or tells them that something has happened, and each of those
//! is a function in `in_play` that names the mods with a say in it, in the
//! order they have it. A mod that is off is not here at all, and so has
//! nothing to say.

pub(crate) mod bullet_time;
mod butterfingers;
pub(crate) mod called_shot;
mod clutch;
mod golden_ball;
mod heat_check;
pub(crate) mod hit_the_sign;
mod hot_bat;
mod in_play;
mod knuckleball;
mod lone_pitcher;
mod moon_ball;
mod mystery_pitch;
pub(crate) mod night_game;
pub(crate) mod pinball_park;
mod rally;
pub(crate) mod southpaw;
pub(crate) mod stolen_bases;
mod sudden_death;
pub(crate) mod the_shift;
pub(crate) mod timing_indicator;
mod tired_arm;
mod turbo_runners;
pub(crate) mod zinger_hit;

pub(crate) use golden_ball::GoldenBall;
pub(crate) use in_play::ModsInPlay;
pub(crate) use tired_arm::TiredArm;

use crate::game::Game;
use crate::look::Rgb;
use crate::mods::{About, Mod};
use crate::rules::FieldRules;

/// What the menu and the files know a mod by. Each mod says its own, in
/// its file.
pub(crate) fn about(which: Mod) -> &'static About {
    match which {
        Mod::BulletTime => &bullet_time::ABOUT,
        Mod::Butterfingers => &butterfingers::ABOUT,
        Mod::CalledShot => &called_shot::ABOUT,
        Mod::Clutch => &clutch::ABOUT,
        Mod::GoldenBall => &golden_ball::ABOUT,
        Mod::HeatCheck => &heat_check::ABOUT,
        Mod::HitTheSign => &hit_the_sign::ABOUT,
        Mod::HotBat => &hot_bat::ABOUT,
        Mod::Knuckleball => &knuckleball::ABOUT,
        Mod::LonePitcher => &lone_pitcher::ABOUT,
        Mod::MoonBall => &moon_ball::ABOUT,
        Mod::MysteryPitch => &mystery_pitch::ABOUT,
        Mod::NightGame => &night_game::ABOUT,
        Mod::PinballPark => &pinball_park::ABOUT,
        Mod::Rally => &rally::ABOUT,
        Mod::Southpaw => &southpaw::ABOUT,
        Mod::StolenBases => &stolen_bases::ABOUT,
        Mod::SuddenDeath => &sudden_death::ABOUT,
        Mod::TheShift => &the_shift::ABOUT,
        Mod::TimingIndicator => &timing_indicator::ABOUT,
        Mod::TiredArm => &tired_arm::ABOUT,
        Mod::TurboRunners => &turbo_runners::ABOUT,
        Mod::ZingerHit => &zinger_hit::ABOUT,
    }
}

/// The numbers the ball flies by over the field in a match of this game:
/// the game's own, with what the mods that are on change of them laid
/// over, a pinball park first and the moon after. The arcade game does not
/// ask, since these mods leave it as it is.
pub(crate) fn field_in_a_match(game: &Game) -> FieldRules {
    let mut field = game.rules.field.clone();
    if game.mods.is_on(Mod::PinballPark) {
        let level = game.mods.level(Mod::PinballPark);
        field = pinball_park::bouncy(&game.rules.pinball, level, &field);
    }
    if game.mods.is_on(Mod::MoonBall) {
        let level = game.mods.level(Mod::MoonBall);
        field = moon_ball::floated(&game.rules.moon, level, &field);
    }
    field
}

/// The colour of something this hot, from warm to as hot as it gets: a bat
/// that keeps meeting the ball, or a rally that keeps going.
pub(crate) fn hot_colour(hot: u32, most: u32) -> Rgb {
    let share = hot as f32 / most.max(1) as f32;
    [0xff, (0xc8 as f32 - 0x98 as f32 * share) as u8, 0x20]
}

/// A line a mod writes in the corner of the batting view: what it is
/// called on the stage, what it says, and in what colour.
pub(crate) struct Line {
    pub name: &'static str,
    pub words: String,
    pub colour: Rgb,
}
