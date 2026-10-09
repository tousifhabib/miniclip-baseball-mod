//! What each mod in play keeps, and the questions the game asks of them.
//!
//! Which mods are in play is settled here, once, as a game begins. The
//! questions are in files by what they are about: the score in `scoring`,
//! the pitch in `pitching`, the swing in `batting`, the slowing of the ball
//! in `slowing`, the field in `fielding` and the runners in `running`.

mod batting;
mod fielding;
mod pitching;
mod running;
mod scoring;
mod slowing;

use crate::game::Game;
use crate::mods::Mod;
use crate::play::mods::bullet_time::BulletTime;
use crate::play::mods::butterfingers::Butterfingers;
use crate::play::mods::called_shot::CalledShot;
use crate::play::mods::clutch::Clutch;
use crate::play::mods::golden_ball::GoldenBall;
use crate::play::mods::heat_check::HeatCheck;
use crate::play::mods::hit_the_sign::HitTheSign;
use crate::play::mods::hot_bat::HotBat;
use crate::play::mods::knuckleball::Knuckleball;
use crate::play::mods::lone_pitcher::LonePitcher;
use crate::play::mods::mystery_pitch::MysteryPitch;
use crate::play::mods::night_game::NightGame;
use crate::play::mods::pinball_park::PinballPark;
use crate::play::mods::rally::Rally;
use crate::play::mods::southpaw::Southpaw;
use crate::play::mods::stolen_bases::StolenBases;
use crate::play::mods::sudden_death::SuddenDeath;
use crate::play::mods::the_shift::TheShift;
use crate::play::mods::timing_indicator::TimingIndicator;
use crate::play::mods::tired_arm::TiredArm;
use crate::play::mods::turbo_runners::TurboRunners;
use crate::play::mods::zinger_hit::ZingerHit;

/// What each mod in play keeps. `None` is a mod that is off.
#[derive(Default)]
pub(crate) struct ModsInPlay {
    /// Bullet time, whose meter the game puts up and whose key it reads in
    /// steps of its own.
    bullet_time: Option<BulletTime>,
    butterfingers: Option<Butterfingers>,
    called_shot: Option<CalledShot>,
    clutch: Option<Clutch>,
    golden_ball: Option<GoldenBall>,
    heat_check: Option<HeatCheck>,
    /// The signs on the wall, which the game lights and reads as the ball
    /// comes to them.
    hit_the_sign: Option<HitTheSign>,
    hot_bat: Option<HotBat>,
    /// The two mods with a hand in deciding the pitch, which the game asks
    /// one by one as it does so.
    pub(in crate::play) knuckleball: Option<Knuckleball>,
    lone_pitcher: Option<LonePitcher>,
    pub(in crate::play) mystery_pitch: Option<MysteryPitch>,
    night_game: Option<NightGame>,
    pinball_park: Option<PinballPark>,
    rally: Option<Rally>,
    /// The left-handed batter, whom the game stands at the plate and
    /// pitches to in steps of its own.
    pub(in crate::play) southpaw: Option<Southpaw>,
    /// Stolen bases, which the fielding counts and tells of as a steal
    /// comes off or fails.
    stolen_bases: Option<StolenBases>,
    sudden_death: Option<SuddenDeath>,
    /// The shift, which the game has move the fielders as the view is got
    /// ready.
    pub(in crate::play) the_shift: Option<TheShift>,
    /// The pitcher's arm, which the game gets ready before each pitch in
    /// several steps of its own.
    timing_indicator: Option<TimingIndicator>,
    pub(in crate::play) tired_arm: Option<TiredArm>,
    turbo_runners: Option<TurboRunners>,
    zinger_hit: Option<ZingerHit>,
}

impl ModsInPlay {
    /// The mods for a game that is about to start. Which are on does not
    /// change while a game is being played. `seed` is what the game's
    /// chances are worked out from. `arcade` is whether it is the arcade
    /// game, which has no runs, outs, runners or fielders, and so no place
    /// for the mods that act on those.
    pub fn for_game(game: &Game, seed: u64, arcade: bool) -> ModsInPlay {
        let on = |which: Mod| game.mods.is_on(which);
        let level = |which: Mod| game.mods.level(which);
        let rules = &game.rules;
        ModsInPlay {
            bullet_time: on(Mod::BulletTime).then(|| BulletTime::new(&rules.bullet_time)),
            // The arcade game has a target of its own, and no runs for a
            // called shot to be worth.
            called_shot: (on(Mod::CalledShot) && !arcade).then_some(CalledShot),
            // Nor any for a sign to be worth.
            hit_the_sign: (on(Mod::HitTheSign) && !arcade).then(|| HitTheSign::new(seed)),
            pinball_park: on(Mod::PinballPark).then_some(PinballPark),
            stolen_bases: on(Mod::StolenBases).then(StolenBases::default),
            timing_indicator: on(Mod::TimingIndicator).then_some(TimingIndicator),
            zinger_hit: on(Mod::ZingerHit).then(ZingerHit::default),
            butterfingers: on(Mod::Butterfingers)
                .then(|| Butterfingers::new(&rules.butterfingers, level(Mod::Butterfingers))),
            clutch: (on(Mod::Clutch) && !arcade).then(|| Clutch::new(&rules.clutch)),
            golden_ball: (on(Mod::GoldenBall) && !arcade).then(|| GoldenBall::new(&rules.golden)),
            heat_check: on(Mod::HeatCheck).then(|| HeatCheck::new(&rules.heat)),
            hot_bat: on(Mod::HotBat).then(|| HotBat::new(&rules.hot_bat)),
            knuckleball: on(Mod::Knuckleball).then(|| Knuckleball::new(&rules.knuckleball)),
            lone_pitcher: on(Mod::LonePitcher).then_some(LonePitcher),
            mystery_pitch: on(Mod::MysteryPitch).then(|| MysteryPitch::new(&rules.mystery)),
            night_game: on(Mod::NightGame).then(|| NightGame::new(&rules.night)),
            rally: (on(Mod::Rally) && !arcade).then(|| Rally::new(&rules.rally)),
            southpaw: on(Mod::Southpaw).then(Southpaw::default),
            sudden_death: on(Mod::SuddenDeath).then(|| SuddenDeath::new(&rules.sudden_death)),
            the_shift: (on(Mod::TheShift) && !arcade).then(|| TheShift::new(&rules.shift)),
            tired_arm: (on(Mod::TiredArm) && !arcade).then(|| TiredArm::new(&rules.tired_arm)),
            turbo_runners: on(Mod::TurboRunners)
                .then(|| TurboRunners::new(&rules.turbo, level(Mod::TurboRunners))),
        }
    }
}
