//! The numbers the game is played by, read from data files so that they can
//! be changed without touching the code.
//!
//! `data/rules.toml` holds every number and is built into the program. A mod
//! supplies a file of the same shape holding only what it changes, and the
//! files are laid over one another in order: the last to name a number wins.

use anyhow::{Context, Result};
use serde::Deserialize;
use toml::{Table, Value};

use crate::play::pitch::Quality;
use crate::settings::Difficulty;

/// The file that holds every number, as built into the program.
const BUILT_IN: &str = include_str!("../../../data/rules.toml");

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    /// The match: batting in the last innings to overtake the other side.
    #[serde(rename = "match")]
    pub game: MatchRules,
    pub count: CountRules,
    /// The pitch, for each skill level.
    pub pitch: BySkillRef<PitchRules>,
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
    pub arcade: ArcadeRules,
    pub team: TeamRules,
    pub sound: SoundRules,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinballRules {
    /// The share of its speed the ball keeps when it bounces, for each
    /// level the mod can be set to, the lowest first.
    pub keeps: Vec<f32>,
    /// The most a bounce can send the ball up by.
    pub hop: f32,
    /// How low a ball has to be for a fielder to get hold of it.
    pub low: f32,
}

impl PinballRules {
    /// The numbers the ball flies by in a pinball park at this level,
    /// counting from 1, given the ones it flies by as the game was.
    pub fn park(&self, level: u8, field: &FieldRules) -> FieldRules {
        let keeps = level_of(&self.keeps, level).unwrap_or(field.bounce_run);
        FieldRules {
            bounce_run: keeps,
            bounce_lift: keeps,
            wall_bounce: keeps,
            bounce_cap: self.hop,
            ..field.clone()
        }
    }
}

/// What a list with a number for each level of a mod's setting has for
/// this level, counting from 1. A level there is none of is taken as the
/// nearest there is.
pub fn level_of(levels: &[f32], level: u8) -> Option<f32> {
    let last = levels.len().checked_sub(1)?;
    levels.get(usize::from(level.max(1) - 1).min(last)).copied()
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoldenRules {
    /// One pitch in this many is gold.
    pub every: u32,
    /// How many each run scored off a golden ball counts for.
    pub runs: u32,
}

impl GoldenRules {
    /// Whether a pitch is gold: `number` is which pitch of the game it is,
    /// counting from 1.
    pub fn is_gold(&self, number: u32) -> bool {
        self.every > 0 && number > 0 && number.is_multiple_of(self.every)
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuddenDeathRules {
    /// Strikes that put a batter out.
    pub strikes: u32,
    /// How many each run counts for.
    pub runs: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HotBatRules {
    /// The most frames hits in a row can add to each end of the window.
    pub most: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalledShotRules {
    /// The part of the field a shot can be called for.
    pub area: Area,
    /// How large the target is, the arcade game's being 1.
    pub size: f32,
    /// The runs each of its rings is worth, from the centre out.
    pub runs: Vec<u32>,
    pub told_time: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MysteryRules {
    /// How long a fastball and a change-up take, the usual being 1.
    pub fast: f32,
    pub slow: f32,
    /// How much more a curve swings and drops each frame.
    pub curve_swing: f32,
    pub curve_dip: f32,
    pub told_time: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeatRules {
    /// The share each run's worth of heat takes off the time a pitch
    /// takes.
    pub step: f32,
    /// The most heat there can be.
    pub most: u32,
}

impl HeatRules {
    /// How long a pitch takes with this much heat on, the time it takes
    /// with none being 1.
    pub fn time(&self, heat: u32) -> f32 {
        (1.0 - self.step * heat.min(self.most) as f32).max(0.1)
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnuckleballRules {
    /// How far the ball sways to either side as it comes by the batter.
    pub sway: f32,
    /// How many times it goes from side to side and back on the way in.
    pub turns: f32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ButterfingersRules {
    /// How often a fielder lets the ball go, out of every hundred goes at
    /// it, for each level the mod can be set to, the lowest first.
    pub chance: Vec<u32>,
    pub roll: f32,
    pub pop: f32,
    pub fumble_time: u32,
    pub gather_time: u32,
    pub told_time: u32,
}

impl ButterfingersRules {
    /// How many levels the mod can be set to.
    pub fn levels(&self) -> u8 {
        self.chance.len().min(usize::from(u8::MAX)) as u8
    }

    /// How often a fielder lets the ball go at this level, counting from
    /// 1. A level there is none of is taken as the nearest there is.
    pub fn chance_at(&self, level: u8) -> u32 {
        let last = self.chance.len().saturating_sub(1);
        let index = usize::from(level.max(1) - 1).min(last);
        self.chance.get(index).copied().unwrap_or(0)
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZingerRules {
    /// How far the ball comes down at the least, the distance to the wall
    /// being 1.
    pub carry_worst: f32,
    /// And at the most, for each skill level.
    pub carry_best: BySkill<f32>,
    /// How much of the most is for holding the ring on the ball.
    pub aim: f32,
    pub aim_reach: f32,
    /// Frames a ball hit level is in the air.
    pub hang: ByTiming,
    pub power: ByTiming,
    pub level_peak: f32,
    pub shape_reach: f32,
    pub sky: Shape,
    pub drive: Shape,
    pub stands: f32,
    pub ball_size: f32,
    pub wall_feet: f32,
    /// How many runs behind a match starts with the mod on.
    pub runs_down: BySkill<u32>,
}

/// What holding the ring well above or well below the ball does to a
/// zinger's flight.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shape {
    /// How many times as long the ball is in the air as one hit level.
    pub hang: f32,
    /// How high it goes, in pixels of the field.
    pub peak: f32,
}

/// A number that goes by how well a swing was timed.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ByTiming {
    /// For the worst-timed swing that still meets the ball.
    pub worst: f32,
    pub best: f32,
}

impl ByTiming {
    /// The number for a swing timed this near the best, from 0 to 1.
    pub fn at(&self, timed: f32) -> f32 {
        self.worst + (self.best - self.worst) * timed.clamp(0.0, 1.0)
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundRules {
    /// How loud each sound is, by its name in the art, from 0 to 1.
    pub levels: std::collections::BTreeMap<String, f32>,
    pub music: String,
    pub crowd: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeamRules {
    /// Colours written as `#rrggbb`.
    pub skins: Vec<String>,
    /// Frame labels of the art's logo clip.
    pub logos: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArcadeRules {
    pub pitches: u32,
    pub watch: u32,
    pub target: Area,
    pub depth_weight: f32,
    /// From the centre out.
    pub rings: Vec<Ring>,
    pub first_bonus: u32,
    pub multiplier: BySkill<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ring {
    pub over: f32,
    pub within: f32,
    pub points: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountRules {
    pub strikes: u32,
    pub balls: u32,
}

/// A whole number picked from `low` to `high`, both included.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Span {
    pub low: u32,
    pub high: u32,
}

impl Span {
    /// The span with both its ends this many times what they were, to the
    /// nearest whole number, and never less than 1.
    pub fn times(self, by: f32) -> Span {
        let times = |number: u32| ((number as f32 * by).round() as u32).max(1);
        Span {
            low: times(self.low),
            high: times(self.high),
        }
    }
}

/// A number worked out as `base + over / n`, with n picked from 1 to
/// `parts`.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Curve {
    pub base: f32,
    pub over: f32,
    pub parts: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Area {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Band {
    pub top: f32,
    pub bottom: f32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PitchRules {
    pub speed: Span,
    pub swing: Curve,
    pub dip: Curve,
    pub target: Area,
    pub marker_frame: u16,
    pub aim_ease: f32,
    pub show_zone: bool,
    pub band: Band,
    /// Frames after the swing, how well the ball is met, and the power.
    pub window: Vec<(u32, Quality, f32)>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThrowRules {
    pub settle: u32,
    pub release_frame: u16,
    pub swing_lead: f32,
    pub dip_lead: f32,
    pub approach: f32,
    pub size: f32,
    pub growth: f32,
    pub fade: f32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HitRules {
    pub pull: f32,
    pub pointer_before_pitch: bool,
    pub watch: u32,
    pub walk_wait: u32,
    pub lift: f32,
    pub lift_aim: f32,
    pub power_drag: f32,
    pub gravity: f32,
    pub bounce: f32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldRules {
    pub x: f32,
    pub pace: f32,
    pub aim_share: f32,
    pub lift_share: f32,
    pub gravity: f32,
    pub drag: f32,
    pub drag_aim: f32,
    pub drag_reach: f32,
    pub bounce_run: f32,
    pub bounce_lift: f32,
    pub bounce_cap: f32,
    pub bounce_loss: f32,
    pub wall: f32,
    pub clear: f32,
    pub wall_bounce: f32,
    pub fielder_reach: f32,
    pub catch_height: f32,
    pub throw_speed: f32,
    pub throw_near: f32,
    pub pick_time: u32,
    pub throw_time: u32,
    pub longest: u32,
    pub fielder_speed: BySkill<f32>,
}

/// A table that differs with the skill level chosen.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BySkillRef<T> {
    pub easy: T,
    pub medium: T,
    pub hard: T,
}

impl<T> BySkillRef<T> {
    pub fn at(&self, difficulty: Difficulty) -> &T {
        match difficulty {
            Difficulty::Easy => &self.easy,
            Difficulty::Medium => &self.medium,
            Difficulty::Hard => &self.hard,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchRules {
    /// Outs a side gets in its innings.
    pub outs: u32,
    /// How many runs behind the player starts.
    pub runs_down: BySkill<u32>,
}

/// A number that differs with the skill level chosen.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BySkill<T> {
    pub easy: T,
    pub medium: T,
    pub hard: T,
}

impl<T: Copy> BySkill<T> {
    pub fn at(&self, difficulty: Difficulty) -> T {
        match difficulty {
            Difficulty::Easy => self.easy,
            Difficulty::Medium => self.medium,
            Difficulty::Hard => self.hard,
        }
    }
}

impl Default for Rules {
    /// The rules as built in, with nothing laid over them.
    fn default() -> Rules {
        Rules::layered(&[]).expect("the built-in rules are tested to be sound")
    }
}

impl Rules {
    /// The built-in rules with each of `layers` laid over them in turn.
    /// Each layer is the name of where it came from, for reporting a
    /// mistake in it, and its text.
    pub fn layered(layers: &[(&str, &str)]) -> Result<Rules> {
        let mut all: Table = BUILT_IN.parse().context("reading the built-in rules")?;
        for (name, text) in layers {
            let layer: Table = text.parse().with_context(|| format!("reading {name}"))?;
            lay_over(&mut all, layer);
            // Checked after every layer, so that a mistake is laid at the
            // door of the file that made it.
            Rules::from_table(all.clone()).with_context(|| format!("in {name}"))?;
        }
        Rules::from_table(all).context("in the built-in rules")
    }

    fn from_table(table: Table) -> Result<Rules> {
        Ok(table.try_into()?)
    }
}

/// Puts everything in `layer` into `base`. A table is merged with the table
/// already there, so that naming one number in it leaves its other numbers
/// alone. Anything else replaces what was there.
fn lay_over(base: &mut Table, layer: Table) {
    for (key, value) in layer {
        match (base.get_mut(&key), value) {
            (Some(Value::Table(under)), Value::Table(over)) => lay_over(under, over),
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_rules_are_sound() {
        let rules = Rules::layered(&[]).unwrap();
        assert_eq!(rules.game.outs, 3);
        assert_eq!(rules.game.runs_down.at(Difficulty::Hard), 3);
    }

    #[test]
    fn butterfingers_has_a_chance_for_each_of_its_levels() {
        let rules = Rules::default().butterfingers;
        assert_eq!(rules.levels(), 5);
        assert_eq!(rules.chance_at(1), 20);
        assert_eq!(rules.chance_at(5), 100);
        // A level there is none of is the nearest there is.
        assert_eq!(rules.chance_at(0), 20);
        assert_eq!(rules.chance_at(9), 100);
        let none = ButterfingersRules {
            chance: Vec::new(),
            ..rules
        };
        assert_eq!((none.levels(), none.chance_at(3)), (0, 0));
    }

    #[test]
    fn heat_takes_time_off_a_pitch_up_to_the_most_there_can_be() {
        let rules = Rules::default().heat;
        assert_eq!(rules.time(0), 1.0);
        assert!((rules.time(1) - 0.94).abs() < 1e-6);
        assert!((rules.time(8) - 0.52).abs() < 1e-6);
        assert_eq!(rules.time(8), rules.time(30));
        let span = Span { low: 35, high: 49 };
        assert_eq!(span.times(0.5), Span { low: 18, high: 25 });
        assert_eq!(span.times(0.0), Span { low: 1, high: 1 });
    }

    #[test]
    fn every_fifth_pitch_is_gold() {
        let rules = Rules::default().golden;
        let gold: Vec<u32> = (0..=16).filter(|&number| rules.is_gold(number)).collect();
        assert_eq!(gold, [5, 10, 15]);
        let never = GoldenRules { every: 0, ..rules };
        assert!(!never.is_gold(5));
    }

    #[test]
    fn a_pinball_park_changes_how_the_ball_bounces_and_nothing_else() {
        let rules = Rules::default();
        let park = rules.pinball.park(5, &rules.field);
        assert_eq!(
            (park.bounce_run, park.wall_bounce, park.bounce_lift),
            (0.9, 0.9, 0.9)
        );
        assert_eq!(park.bounce_cap, 1.5);
        assert_eq!(rules.pinball.park(1, &rules.field).wall_bounce, 0.6);
        let back = FieldRules {
            bounce_run: rules.field.bounce_run,
            bounce_lift: rules.field.bounce_lift,
            wall_bounce: rules.field.wall_bounce,
            bounce_cap: rules.field.bounce_cap,
            ..park
        };
        assert_eq!(back, rules.field);
        assert_eq!(level_of(&[1.0, 2.0], 9), Some(2.0));
        assert_eq!(level_of(&[], 1), None);
    }

    #[test]
    fn a_layer_changes_only_what_it_names() {
        let rules = Rules::layered(&[("a mod", "[match.runs_down]\nhard = 5\n")]).unwrap();
        assert_eq!(rules.game.runs_down.at(Difficulty::Hard), 5);
        assert_eq!(rules.game.runs_down.at(Difficulty::Easy), 1);
        assert_eq!(rules.game.outs, 3);
    }

    #[test]
    fn the_last_layer_to_name_a_number_wins() {
        let rules = Rules::layered(&[
            ("first", "[match]\nouts = 1\n"),
            ("second", "[match]\nouts = 2\n"),
        ])
        .unwrap();
        assert_eq!(rules.game.outs, 2);
    }

    #[test]
    fn a_number_the_game_does_not_have_is_refused_by_name() {
        let error = Rules::layered(&[("typo.toml", "[match]\nouts_allowed = 4\n")]).unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("typo.toml"), "{message}");
        assert!(message.contains("outs_allowed"), "{message}");
    }

    #[test]
    fn a_value_of_the_wrong_kind_is_refused() {
        let error = Rules::layered(&[("wrong.toml", "[match]\nouts = \"three\"\n")]).unwrap_err();
        assert!(format!("{error:#}").contains("wrong.toml"));
    }

    #[test]
    fn a_file_that_is_not_toml_is_refused_with_its_name() {
        let error = Rules::layered(&[("broken.toml", "[match\nouts = 3")]).unwrap_err();
        assert!(format!("{error:#}").contains("broken.toml"));
    }

    #[test]
    fn a_mistake_is_blamed_on_the_layer_that_made_it() {
        let error = Rules::layered(&[
            ("good.toml", "[match]\nouts = 4\n"),
            ("bad.toml", "[match]\nnonsense = 1\n"),
        ])
        .unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("bad.toml"), "{message}");
        assert!(!message.contains("good.toml"), "{message}");
    }
}
