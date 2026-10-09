//! The numbers the game is played by, read from data files so that they can
//! be changed without touching the code.
//!
//! `data/rules.toml` holds every number and is built into the program. A mod
//! supplies a file of the same shape holding only what it changes, and the
//! files are laid over one another in order: the last to name a number wins.
//!
//! The whole set is here. The numbers are in a file for what they are
//! numbers of: the game's in `game`, the ball's in `ball`, and the mods' in
//! `plate_mods` and `field_mods`. The shapes a number comes in are in
//! `shapes`, the laying of files over one another in `layers`, and the
//! check that a set can be played by in `faults`.

mod ball;
mod faults;
mod field_mods;
mod game;
mod layers;
mod plate_mods;
#[cfg(test)]
mod properties;
mod shapes;

use std::sync::LazyLock;

use serde::Deserialize;

pub use ball::{FieldRules, HitRules, PitchRules, ThrowRules};
pub use faults::RulesFault;
pub use field_mods::{
    ButterfingersRules, CalledShotRules, MoonRules, NightRules, PinballRules, ShiftRules,
    SignRules, StealRules, TurboRules, ZingerRules,
};
pub use game::{
    ArcadeRules, CountRules, FullMatchRules, MatchRules, SoundRules, TeamRules, TheirBattingRules,
};
pub use plate_mods::{
    BulletTimeRules, ClutchRules, GoldenRules, HeatRules, HotBatRules, KnuckleballRules,
    MysteryRules, RallyRules, SuddenDeathRules, TiredArmRules,
};
pub use shapes::{Area, Band, BySkill, ByTiming, Curve, Levels, Ring, Shape, Span};

/// The built-in rules, read the first time they are wanted and kept. They
/// are asked for whenever a game or a test is set up, and the file does not
/// change while the program runs.
static AS_BUILT_IN: LazyLock<Rules> =
    LazyLock::new(|| Rules::layered(&[]).expect("the built-in rules are tested to be sound"));

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    /// The match: batting in the last innings to overtake the other side.
    #[serde(rename = "match")]
    pub game: MatchRules,
    pub count: CountRules,
    /// The full match: every innings, against a side whose own are made up.
    pub full_match: FullMatchRules,
    /// The pitch, for each skill level.
    pub pitch: BySkill<PitchRules>,
    /// What every pitch shares.
    pub throw: ThrowRules,
    /// The ball off the bat.
    pub hit: HitRules,
    /// The ball over the field.
    pub field: FieldRules,
    /// The zinger hit mod.
    pub zinger: ZingerRules,
    /// The butterfingers mod.
    pub butterfingers: ButterfingersRules,
    /// The knuckleball mod.
    pub knuckleball: KnuckleballRules,
    /// The heat check mod.
    pub heat: HeatRules,
    /// The mystery pitch mod.
    pub mystery: MysteryRules,
    /// The called shot mod.
    pub called_shot: CalledShotRules,
    /// The hot bat mod.
    pub hot_bat: HotBatRules,
    /// The sudden death mod.
    pub sudden_death: SuddenDeathRules,
    /// The golden ball mod.
    pub golden: GoldenRules,
    /// The pinball park mod.
    pub pinball: PinballRules,
    /// The moon ball mod.
    pub moon: MoonRules,
    /// The turbo runners mod.
    pub turbo: TurboRules,
    /// The night game mod.
    pub night: NightRules,
    /// The shift mod.
    pub shift: ShiftRules,
    /// The tired arm mod.
    pub tired_arm: TiredArmRules,
    /// The stolen bases mod.
    pub steal: StealRules,
    /// The hit the sign mod.
    pub sign: SignRules,
    /// The rally mod.
    pub rally: RallyRules,
    /// The clutch mod.
    pub clutch: ClutchRules,
    /// The bullet time mod.
    pub bullet_time: BulletTimeRules,
    pub arcade: ArcadeRules,
    pub team: TeamRules,
    pub sound: SoundRules,
}

impl Default for Rules {
    /// The rules as built in, with nothing laid over them.
    fn default() -> Rules {
        AS_BUILT_IN.clone()
    }
}

#[cfg(test)]
mod tests;
