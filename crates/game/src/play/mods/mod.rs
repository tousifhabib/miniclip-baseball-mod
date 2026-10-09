//! The mods that are in play in a match.
//!
//! Each mod has a file of its own here, named as the mod is: what it keeps
//! from one pitch to the next, its rules, and what it draws. The game does
//! not ask whether a mod is switched on. It asks the mods a question, or
//! tells them that something has happened, and each of those is a function
//! below that names the mods with a say in it, in the order they have it.
//! A mod that is off is not here at all, and so has nothing to say.

pub(crate) mod bullet_time;
mod butterfingers;
pub(crate) mod called_shot;
mod clutch;
mod golden_ball;
mod heat_check;
pub(crate) mod hit_the_sign;
mod hot_bat;
mod knuckleball;
mod lone_pitcher;
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

use butterfingers::Butterfingers;
use clutch::Clutch;
pub(crate) use golden_ball::GoldenBall;
use heat_check::HeatCheck;
use hot_bat::HotBat;
use knuckleball::Knuckleball;
use lone_pitcher::LonePitcher;
use mystery_pitch::MysteryPitch;
use night_game::NightGame;
use rally::Rally;
use southpaw::Southpaw;
use sudden_death::SuddenDeath;
use the_shift::TheShift;
pub(crate) use tired_arm::TiredArm;
use turbo_runners::TurboRunners;

use bb_engine::math::ColorTransform;

use crate::look::Rgb;
use crate::menu::Game;
use crate::mods::Mod;
use crate::play::snapshot::ArmSeen;
use crate::rng::Rng;
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
    butterfingers: Option<Butterfingers>,
    clutch: Option<Clutch>,
    golden_ball: Option<GoldenBall>,
    heat_check: Option<HeatCheck>,
    hot_bat: Option<HotBat>,
    /// The two mods with a hand in deciding the pitch, which the game asks
    /// one by one as it does so.
    pub(in crate::play) knuckleball: Option<Knuckleball>,
    lone_pitcher: Option<LonePitcher>,
    pub(in crate::play) mystery_pitch: Option<MysteryPitch>,
    night_game: Option<NightGame>,
    rally: Option<Rally>,
    /// The left-handed batter, whom the game stands at the plate and
    /// pitches to in steps of its own.
    pub(in crate::play) southpaw: Option<Southpaw>,
    sudden_death: Option<SuddenDeath>,
    /// The shift, which the game has move the fielders as the view is got
    /// ready.
    pub(in crate::play) the_shift: Option<TheShift>,
    /// The pitcher's arm, which the game gets ready before each pitch in
    /// several steps of its own.
    pub(in crate::play) tired_arm: Option<TiredArm>,
    turbo_runners: Option<TurboRunners>,
}

impl ModsInPlay {
    /// The mods for a game that is about to start. Which are on does not
    /// change while a game is being played. `arcade` is whether it is the
    /// arcade game, which has no runs, outs, runners or fielders, and so no
    /// place for the mods that act on those.
    pub fn for_game(game: &Game, arcade: bool) -> ModsInPlay {
        let on = |which: Mod| game.mods.is_on(which);
        let level = |which: Mod| game.mods.level(which);
        let rules = &game.rules;
        ModsInPlay {
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

    /// Whether the pitcher is left to field every ball by himself: nobody
    /// else goes after it, goes back to watch it, or throws it on.
    pub fn the_pitcher_fields_alone(&self) -> bool {
        self.lone_pitcher.is_some()
    }

    /// Whether a fielder having a go at the ball lets it go. A number is
    /// drawn for the go only if the mod that makes them slip is in play.
    pub fn a_fielder_lets_go(&mut self, rng: &mut Rng) -> bool {
        self.butterfingers
            .as_mut()
            .is_some_and(|butter| butter.lets_go(rng))
    }

    /// How often the fielders have let the ball go.
    pub fn let_go(&self) -> u32 {
        self.butterfingers.as_ref().map_or(0, Butterfingers::slips)
    }

    /// Whether a runner on a base may be sent on at any time the ball is
    /// in play, and need not wait for it to come down or be caught.
    pub fn runners_may_go_at_any_time(&self) -> bool {
        self.turbo_runners.is_some()
    }

    /// How many frames more than the usual one the runners are moved on by
    /// this frame.
    pub fn hurry_the_runners(&mut self) -> u16 {
        self.turbo_runners.as_mut().map_or(0, TurboRunners::hurry)
    }

    /// A home run has been hit, or a zinger has come down.
    pub fn a_home_run_was_hit(&mut self) {
        if let Some(night) = &mut self.night_game {
            night.a_home_run_was_hit();
        }
    }

    /// How the stadium is to be lit this frame, if any mod has a say in
    /// it: dark by night, flashing for a home run, and cooler while the
    /// ball is being held back. `cooled` is whether it is, and whether the
    /// mod that holds it back is in play.
    pub fn lighting(&mut self, cooled: Option<bool>) -> Option<ColorTransform> {
        if self.night_game.is_none() && cooled.is_none() {
            return None;
        }
        let lighting = match &mut self.night_game {
            Some(night) => night.lighting(),
            None => night_game::DAY,
        };
        Some(if cooled == Some(true) {
            bullet_time::cool(lighting)
        } else {
            lighting
        })
    }

    /// A ball that was hit fair has come down this far across the field,
    /// from 0 at one foul line to 1 at the other.
    pub fn a_fair_ball_came_down(&mut self, across: f32) {
        if let Some(shift) = &mut self.the_shift {
            shift.remember(across);
        }
    }

    /// How far the fielders have shifted for the pitch in hand.
    pub fn shifted(&self) -> f32 {
        self.the_shift.as_ref().map_or(0.0, TheShift::by)
    }

    /// Whether the batter is batting left-handed, once he has taken his
    /// stand.
    pub fn batting_left_handed(&self) -> bool {
        self.southpaw.as_ref().is_some_and(Southpaw::has_stood)
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
