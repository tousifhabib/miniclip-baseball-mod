//! The mods that are in play in a match.
//!
//! Each mod has a file of its own here, named as the mod is: what it keeps
//! from one pitch to the next, its rules, and what it draws. The game does
//! not ask whether a mod is switched on. It asks the mods a question, or
//! tells them that something has happened, and each of those is a function
//! below that names the mods with a say in it, in the order they have it.
//! A mod that is off is not here at all, and so has nothing to say.

mod clutch;
mod golden_ball;
mod heat_check;
mod hot_bat;
mod knuckleball;
mod mystery_pitch;
mod rally;
mod sudden_death;
mod tired_arm;

use clutch::Clutch;
pub(crate) use golden_ball::GoldenBall;
use heat_check::HeatCheck;
use hot_bat::HotBat;
use knuckleball::Knuckleball;
use mystery_pitch::MysteryPitch;
use rally::Rally;
use sudden_death::SuddenDeath;
pub(crate) use tired_arm::TiredArm;

use bb_engine::math::ColorTransform;

use crate::look::Rgb;
use crate::menu::Game;
use crate::mods::Mod;
use crate::play::snapshot::ArmSeen;
use crate::rules::PitchRules;

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

/// What each mod in play keeps. `None` is a mod that is off.
#[derive(Default)]
pub(crate) struct ModsInPlay {
    clutch: Option<Clutch>,
    golden_ball: Option<GoldenBall>,
    heat_check: Option<HeatCheck>,
    hot_bat: Option<HotBat>,
    /// The two mods with a hand in deciding the pitch, which the game asks
    /// one by one as it does so.
    pub(in crate::play) knuckleball: Option<Knuckleball>,
    pub(in crate::play) mystery_pitch: Option<MysteryPitch>,
    rally: Option<Rally>,
    sudden_death: Option<SuddenDeath>,
    /// The pitcher's arm, which the game gets ready before each pitch in
    /// several steps of its own.
    pub(in crate::play) tired_arm: Option<TiredArm>,
}

impl ModsInPlay {
    /// The mods for a game that is about to start. Which are on does not
    /// change while a game is being played. `arcade` is whether it is the
    /// arcade game, which has no runs, outs, runners or fielders, and so no
    /// place for the mods that act on those.
    pub fn for_game(game: &Game, arcade: bool) -> ModsInPlay {
        let on = |which: Mod| game.mods.is_on(which);
        let rules = &game.rules;
        ModsInPlay {
            clutch: (on(Mod::Clutch) && !arcade).then(|| Clutch::new(&rules.clutch)),
            golden_ball: (on(Mod::GoldenBall) && !arcade).then(|| GoldenBall::new(&rules.golden)),
            heat_check: on(Mod::HeatCheck).then(|| HeatCheck::new(&rules.heat)),
            hot_bat: on(Mod::HotBat).then(|| HotBat::new(&rules.hot_bat)),
            knuckleball: on(Mod::Knuckleball).then(|| Knuckleball::new(&rules.knuckleball)),
            mystery_pitch: on(Mod::MysteryPitch).then(|| MysteryPitch::new(&rules.mystery)),
            rally: (on(Mod::Rally) && !arcade).then(|| Rally::new(&rules.rally)),
            sudden_death: on(Mod::SuddenDeath).then(|| SuddenDeath::new(&rules.sudden_death)),
            tired_arm: (on(Mod::TiredArm) && !arcade).then(|| TiredArm::new(&rules.tired_arm)),
        }
    }

    /// How many strikes put a batter out: the `usual` number, unless a mod
    /// says otherwise.
    pub fn strikes_allowed(&self, usual: u32) -> u32 {
        self.sudden_death
            .as_ref()
            .map_or(usual, SuddenDeath::strikes)
    }

    /// Whether the pitch with this number, the first being 1, is a golden
    /// ball.
    pub fn is_golden(&self, pitch: u32) -> bool {
        self.golden_ball
            .as_ref()
            .is_some_and(|golden| golden.is_gold(pitch))
    }

    /// Whether a pitch thrown now is in the clutch, given how the game
    /// stands.
    pub fn in_the_clutch(&self, one_out_left: bool, runner_in_reach_of_home: bool) -> bool {
        self.clutch.is_some() && Clutch::is_now(one_out_left, runner_in_reach_of_home)
    }

    /// Settles whether the pitch in hand is thrown in the clutch, and
    /// returns what the corner of the view says if it is.
    pub fn settle_the_clutch(&mut self, in_it: bool) -> Option<Line> {
        let clutch = self.clutch.as_mut()?;
        clutch.this_pitch = in_it;
        in_it.then(|| clutch.line())
    }

    /// Whether the pitch in hand was thrown in the clutch.
    pub fn clutch_this_pitch(&self) -> bool {
        self.clutch.as_ref().is_some_and(|clutch| clutch.this_pitch)
    }

    /// How many times over a run counts on the pitch about to be thrown,
    /// which is a golden ball or is not, and in the clutch or is not. Each
    /// mod that makes runs worth more multiplies what the others make of
    /// them.
    pub fn worth_of_a_run(&self, golden: bool, in_the_clutch: bool) -> u32 {
        let for_gold = match &self.golden_ball {
            Some(ball) if golden => ball.runs(),
            _ => 1,
        };
        let for_the_clutch = match &self.clutch {
            Some(clutch) if in_the_clutch => clutch.runs(),
            _ => 1,
        };
        for_gold
            * self.sudden_death.as_ref().map_or(1, SuddenDeath::runs)
            * self.rally.as_ref().map_or(1, Rally::worth)
            * for_the_clutch
    }

    /// Takes in the runs scored since the last pitch and makes the coming
    /// one faster by the heat that is on. Returns what the corner of the
    /// view says of it.
    pub fn heat_the_pitch(&mut self, score: u32, table: &mut PitchRules) -> Option<Line> {
        let heat = self.heat_check.as_mut()?;
        heat.warm(score, table);
        heat.line()
    }

    /// How much heat is on.
    pub fn heat(&self) -> u32 {
        self.heat_check.as_ref().map_or(0, HeatCheck::heat)
    }

    /// Widens the window the coming pitch can be met in, for a bat that is
    /// hot. Returns what the corner of the view says of it.
    pub fn widen_for_a_hot_bat(&self, table: &mut PitchRules) -> Option<Line> {
        let bat = self.hot_bat.as_ref()?;
        bat.widen(table);
        bat.line()
    }

    /// What the mark on the bat is tinted, while the bat is hot.
    pub fn glow_of_the_bat(&self) -> Option<ColorTransform> {
        self.hot_bat.as_ref().and_then(HotBat::glow)
    }

    /// How many swings in a row have met the ball.
    pub fn hits_in_a_row(&self) -> u32 {
        self.hot_bat.as_ref().map_or(0, HotBat::streak)
    }

    /// The ball has left the pitcher's hand.
    pub fn the_ball_was_thrown(&mut self) {
        if let Some(arm) = &mut self.tired_arm {
            arm.threw();
        }
    }

    /// How the pitcher's arm is holding up.
    pub fn arm(&self) -> Option<ArmSeen> {
        self.tired_arm.as_ref().and_then(TiredArm::seen)
    }

    /// The bat has met the ball.
    pub fn the_bat_met_the_ball(&mut self) {
        if let Some(bat) = &mut self.hot_bat {
            bat.met();
        }
    }

    /// A strike has been called on the batter, swung at or not.
    pub fn a_strike_was_called(&mut self) {
        if let Some(bat) = &mut self.hot_bat {
            bat.missed();
        }
        self.cool();
    }

    /// A foul has counted as a strike against the batter.
    pub fn a_foul_took_a_strike(&mut self) {
        self.cool();
    }

    fn cool(&mut self) {
        if let Some(heat) = &mut self.heat_check {
            heat.cool();
        }
    }

    /// Somebody has been put out: at the plate, or on the bases.
    pub fn somebody_is_out(&mut self) {
        if let Some(rally) = &mut self.rally {
            rally.broken();
        }
    }

    /// The batter has got to a base, or all the way round.
    pub fn the_batter_reached_base(&mut self) {
        if let Some(rally) = &mut self.rally {
            rally.kept_up();
        }
    }

    /// What the corner of the view says of a rally, while one is on.
    pub fn rally_line(&self) -> Option<Line> {
        self.rally.as_ref().and_then(Rally::line)
    }

    /// How many batters in a row have reached base, as far as it counts.
    pub fn in_a_row(&self) -> u32 {
        self.rally.as_ref().map_or(0, Rally::in_a_row)
    }
}
