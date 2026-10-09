//! The numbers the game is played by, read from data files so that they can
//! be changed without touching the code.
//!
//! `data/rules.toml` holds every number and is built into the program. A mod
//! supplies a file of the same shape holding only what it changes, and the
//! files are laid over one another in order: the last to name a number wins.

use std::sync::LazyLock;

use anyhow::{Context, Result};
use serde::Deserialize;
use toml::{Table, Value};

use crate::play::pitch::Quality;
use crate::settings::Difficulty;

/// The file that holds every number, as built into the program.
const BUILT_IN: &str = include_str!("../../../data/rules.toml");

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

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BulletTimeRules {
    /// While the ball is held back it moves on one frame in this many.
    pub slow: u32,
    /// It can be held back for this many of the last frames of its flight.
    pub near: u32,
    /// How many frames of holding back the meter has in it when it is
    /// full.
    pub full: u32,
    /// The share of that a hit puts back, a home run filling it.
    pub hit: f32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClutchRules {
    /// How many each run counts for with one out left and a runner on
    /// second or third.
    pub runs: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RallyRules {
    /// The most batters in a row that count towards what a run is worth.
    pub most: u32,
}

impl RallyRules {
    /// How many a run counts for when this many batters in a row have
    /// reached base.
    pub fn worth(&self, in_a_row: u32) -> u32 {
        1 + in_a_row.min(self.most)
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignRules {
    /// How many signs the wall has.
    pub count: u32,
    /// How far across the field the middle of the first is, and of the
    /// last, from 0 on the left foul line to 1 on the right, and how wide
    /// each is, the same way.
    pub first: f32,
    pub last: f32,
    pub width: f32,
    /// How high a ball can be at the wall and still strike a sign.
    pub high: f32,
    /// The runs a ball that strikes the lit sign is worth, and one that
    /// strikes any other.
    pub lit: u32,
    pub unlit: u32,
    /// Frames the word that a sign was struck stays up.
    pub told_time: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StealRules {
    /// Frames the catcher takes to let go of the ball once the pitch is in
    /// his glove, picked afresh each time, and how fast his throw goes.
    pub pop: Span,
    pub throw_speed: f32,
    /// Frames the word of how a steal came out stays up.
    pub told_time: u32,
    /// In a full match, how often a runner of the other side goes for
    /// second before a batter's turn, how much of that often one goes for
    /// third, and how often either gets there.
    pub their_chance: f32,
    pub their_third: f32,
    pub their_safe: f32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TiredArmRules {
    /// How many pitches a pitcher throws before he begins to tire, and by
    /// how many he is spent.
    pub fresh: u32,
    pub spent: u32,
    /// How many he throws before another comes in for him.
    pub relief: u32,
    /// Spent, how many times as long his pitches take, and how many times
    /// as wide and as high the area he aims at is.
    pub slow: f32,
    pub wild: f32,
    /// Frames the word of a new pitcher stays up.
    pub told_time: u32,
}

impl TiredArmRules {
    /// How tired a pitcher is who has thrown this many pitches: from 0, as
    /// good as ever, to 1, spent.
    pub fn tired(&self, thrown: u32) -> f32 {
        let over = thrown.saturating_sub(self.fresh) as f32;
        let all = self.spent.saturating_sub(self.fresh).max(1) as f32;
        (over / all).clamp(0.0, 1.0)
    }

    /// The table a pitch is picked from when the pitcher is this tired,
    /// given the one it is picked from when he is not: slower, and aimed
    /// less surely at the same place.
    pub fn pitch(&self, table: &PitchRules, tired: f32) -> PitchRules {
        let tired = tired.clamp(0.0, 1.0);
        let slow = 1.0 + (self.slow - 1.0) * tired;
        let wild = 1.0 + (self.wild - 1.0) * tired;
        let target = table.target;
        let (width, height) = (target.width * wild, target.height * wild);
        PitchRules {
            speed: table.speed.times(slow),
            target: Area {
                x: target.x - (width - target.width) / 2.0,
                y: target.y - (height - target.height) / 2.0,
                width,
                height,
            },
            ..table.clone()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShiftRules {
    /// How many of the last fair balls the fielders go by, and how many
    /// there have to have been before they move.
    pub memory: u32,
    pub least: u32,
    /// The share of the way from the middle of the field to where those
    /// balls went that the fielders' middle moves, and the most it moves,
    /// the width of the field being 1.
    pub follow: f32,
    pub most: f32,
    /// How far it has to have moved for the player to be told.
    pub told: f32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NightRules {
    /// How much is left of the red, green and blue of the stadium.
    pub dark: [f32; 3],
    /// Frames a home run flashes the lights for, and how many of them
    /// each turn of lit or dark lasts.
    pub flash_time: u32,
    pub flash_every: u32,
    /// How much brighter than by day everything is when it is lit.
    pub glare: f32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TurboRules {
    /// How many times as fast runners go, for each level the mod can be
    /// set to, the lowest first.
    pub speed: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoonRules {
    /// How many times as long the ball takes over its flight, for each
    /// level the mod can be set to, the lowest first.
    pub slow: Vec<f32>,
}

impl MoonRules {
    /// The numbers the ball flies by on the moon at this level, counting
    /// from 1, given the ones it flies by as the game was: the same flight
    /// in every way but the time it takes.
    pub fn float(&self, level: u8, field: &FieldRules) -> FieldRules {
        let slow = level_of(&self.slow, level).unwrap_or(1.0).max(0.01);
        FieldRules {
            // It sets off this many times slower, along and up, and what
            // pulls it down and holds it back is as much weaker as keeps it
            // to the path it would have taken.
            pace: field.pace * slow,
            lift_share: field.lift_share / slow,
            gravity: field.gravity / (slow * slow),
            drag: field.drag / slow,
            bounce_cap: field.bounce_cap / slow,
            bounce_loss: field.bounce_loss / slow,
            ..field.clone()
        }
    }
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

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FullMatchRules {
    /// The innings each side has, before any that a level match adds.
    pub innings: u32,
    /// How many innings' worth of runs the other side is given in each of
    /// theirs with the zinger hit mod on.
    pub zinger_innings: u32,
    /// How likely the other side is to make each number of runs in an
    /// innings, from none up.
    pub runs: BySkillRef<Vec<u32>>,
    /// The chances their innings are played out on paper by.
    pub their_batting: TheirBattingRules,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TheirBattingRules {
    /// The share of the pitches to them that are in the strike zone.
    pub zone: f32,
    /// How often they swing at a pitch in the zone, and at one outside it.
    pub swing: [f32; 2],
    /// How often such a swing meets the ball.
    pub contact: [f32; 2],
    /// The share of the balls they meet that go foul.
    pub foul: f32,
    /// How likely a ball put in play is to come to each of these.
    pub ground_out: f32,
    pub fly_out: f32,
    pub single: f32,
    pub double: f32,
    pub triple: f32,
    pub home_run: f32,
}

impl FullMatchRules {
    /// The runs the other side makes in an innings, given a number from
    /// nought up to but not including one, drawn evenly.
    pub fn runs_for(&self, difficulty: Difficulty, drawn: f32) -> u32 {
        let chances = self.runs.at(difficulty);
        let all: u32 = chances.iter().sum();
        let mut left = drawn.clamp(0.0, 1.0) * all as f32;
        for (runs, &chance) in chances.iter().enumerate() {
            left -= chance as f32;
            if left < 0.0 {
                return runs as u32;
            }
        }
        // A table of nothing but noughts is a side that never scores.
        chances.iter().rposition(|&chance| chance > 0).unwrap_or(0) as u32
    }
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
        AS_BUILT_IN.clone()
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
        all_are_numbers(&table, "")?;
        let rules: Rules = table.try_into()?;
        rules.can_be_played_by()?;
        Ok(rules)
    }

    /// Checks the numbers that the game's sums take for granted: a file
    /// may set any of them, and one set wrongly would stop the game in the
    /// middle of a pitch, far from the file that did it.
    fn can_be_played_by(&self) -> Result<(), RulesFault> {
        let check = |holds: bool, which: &str, why: &'static str| {
            if holds {
                Ok(())
            } else {
                Err(RulesFault {
                    which: which.to_owned(),
                    why,
                })
            }
        };
        let levels = [
            ("pitch.easy.speed", &self.pitch.easy),
            ("pitch.medium.speed", &self.pitch.medium),
            ("pitch.hard.speed", &self.pitch.hard),
        ];
        for (which, pitch) in levels {
            check(pitch.speed.low >= 1, which, "has to be at least 1")?;
            check(
                pitch.speed.low <= pitch.speed.high,
                which,
                "has its low above its high",
            )?;
        }
        let pop = &self.steal.pop;
        check(
            pop.low <= pop.high,
            "steal.pop",
            "has its low above its high",
        )?;
        check(
            self.throw.approach > 0.0,
            "throw.approach",
            "has to be more than nought",
        )?;
        check(
            self.throw.fade > 0.0,
            "throw.fade",
            "has to be more than nought",
        )?;
        check(
            self.field.pace > 0.0,
            "field.pace",
            "has to be more than nought",
        )?;
        check(
            self.field.wall > 0.0,
            "field.wall",
            "has to be more than nought",
        )?;
        check(
            self.shift.most >= 0.0,
            "shift.most",
            "cannot be less than nought",
        )?;
        check(
            self.zinger.shape_reach >= 0.0,
            "zinger.shape_reach",
            "cannot be less than nought",
        )
    }
}

/// A number of the rules that the game cannot be played by: which it is,
/// and what is wrong with it.
#[derive(Debug, thiserror::Error)]
#[error("`{which}` {why}")]
pub struct RulesFault {
    which: String,
    why: &'static str,
}

/// Checks that every number in the table is a number: a file can say `nan`
/// or `inf`, and the game can do no sums with either.
fn all_are_numbers(table: &Table, inside: &str) -> Result<(), RulesFault> {
    for (key, value) in table {
        let which = if inside.is_empty() {
            key.clone()
        } else {
            format!("{inside}.{key}")
        };
        let numbers = |value: &Value| match value {
            Value::Float(number) => number.is_finite(),
            Value::Array(all) => all
                .iter()
                .all(|each| each.as_float().is_none_or(f64::is_finite)),
            _ => true,
        };
        match value {
            Value::Table(under) => all_are_numbers(under, &which)?,
            value if !numbers(value) => {
                return Err(RulesFault {
                    which,
                    why: "is not a number the game can do sums with",
                });
            }
            _ => {}
        }
    }
    Ok(())
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
    use std::fmt::Write;

    use proptest::prelude::*;

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
    fn a_pitcher_tires_between_his_fresh_pitches_and_the_ones_that_spend_him() {
        let rules = Rules::default();
        let arm = &rules.tired_arm;
        assert_eq!(arm.tired(0), 0.0);
        assert_eq!(arm.tired(arm.fresh), 0.0);
        assert_eq!(arm.tired(arm.spent), 1.0);
        assert_eq!(arm.tired(arm.spent + 50), 1.0);
        let half = arm.tired((arm.fresh + arm.spent) / 2);
        assert!((half - 0.5).abs() < 0.05, "{half}");
        // Fresh, he pitches as he always did.
        let usual = rules.pitch.at(Difficulty::Medium);
        assert_eq!(arm.pitch(usual, 0.0), *usual);
        // Spent, he is slower, and aims at more than the strike zone
        // about the same middle.
        let spent = arm.pitch(usual, 1.0);
        assert_eq!(
            spent.speed.low,
            (usual.speed.low as f32 * arm.slow).round() as u32
        );
        let middle = |area: &Area| (area.x + area.width / 2.0, area.y + area.height / 2.0);
        assert_eq!(middle(&spent.target), middle(&usual.target));
        assert_eq!(spent.target.width, usual.target.width * arm.wild);
        assert_eq!(spent.target.height, usual.target.height * arm.wild);
        assert_eq!(spent.window, usual.window);
    }

    #[test]
    fn a_rally_makes_a_run_worth_one_more_for_each_batter_up_to_the_most() {
        let rules = Rules::default().rally;
        assert_eq!(rules.worth(0), 1);
        assert_eq!(rules.worth(1), 2);
        assert_eq!(rules.worth(rules.most), rules.most + 1);
        assert_eq!(rules.worth(rules.most + 7), rules.most + 1);
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
    fn a_moon_ball_goes_where_it_would_have_gone_and_takes_longer_over_it() {
        use crate::play::field::{Ball, Contact, Happened};
        let rules = Rules::default();
        let (home, mark) = ((240.8, 336.85), (303.8, 168.7));
        // A well-timed hit, and one topped a little.
        for under in [0.0, -12.0] {
            let contact = Contact {
                power: 17.0,
                under,
                aside: 0.0,
            };
            let lands = |field: &FieldRules| {
                let mut ball = Ball::hit(home, mark, &contact, &rules.hit, field);
                let mut frames = 1;
                while ball.step(home, contact.miss(), field) != Happened::Landed {
                    frames += 1;
                }
                (ball.at, frames)
            };
            let (usual, quick) = lands(&rules.field);
            for (level, slow) in [(1, 1.5), (2, 2.0), (5, 5.0)] {
                let (floated, frames) = lands(&rules.moon.float(level, &rules.field));
                let off = (floated.0 - usual.0).hypot(floated.1 - usual.1);
                assert!(off < 6.0, "level {level}: {off} from {usual:?}");
                let longer = frames as f32 / quick as f32;
                assert!((longer - slow).abs() < 0.1, "level {level}: {longer}");
            }
        }
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

    #[test]
    fn the_built_in_rules_laid_over_themselves_change_nothing() {
        let rules = Rules::layered(&[("themselves", BUILT_IN)]).unwrap();
        assert_eq!(rules, Rules::default());
    }

    /// A whole number of the rules that a layer might name: what a file
    /// calls it, and where the rules keep it.
    type Whole = (&'static str, fn(&mut Rules) -> &mut u32);
    /// The same, for a number that need not be whole.
    type Part = (&'static str, fn(&mut Rules) -> &mut f32);

    /// Some of each, from all over the file: in tables of their own, in
    /// tables within tables, and in the tables that are written on one line.
    /// None is a number the game holds within bounds, since a layer here
    /// may give it any value at all.
    const WHOLES: [Whole; 10] = [
        ("match.outs", |rules| &mut rules.game.outs),
        ("match.runs_down.hard", |rules| {
            &mut rules.game.runs_down.hard
        }),
        ("count.strikes", |rules| &mut rules.count.strikes),
        ("full_match.innings", |rules| &mut rules.full_match.innings),
        ("throw.settle", |rules| &mut rules.throw.settle),
        ("hit.watch", |rules| &mut rules.hit.watch),
        ("zinger.runs_down.medium", |rules| {
            &mut rules.zinger.runs_down.medium
        }),
        ("heat.most", |rules| &mut rules.heat.most),
        ("golden.every", |rules| &mut rules.golden.every),
        ("arcade.multiplier.easy", |rules| {
            &mut rules.arcade.multiplier.easy
        }),
    ];
    const PARTS: [Part; 9] = [
        ("pitch.medium.band.top", |rules| {
            &mut rules.pitch.medium.band.top
        }),
        ("throw.growth", |rules| &mut rules.throw.growth),
        ("hit.pull", |rules| &mut rules.hit.pull),
        ("field.catch_height", |rules| &mut rules.field.catch_height),
        ("field.fielder_speed.easy", |rules| {
            &mut rules.field.fielder_speed.easy
        }),
        ("zinger.hang.best", |rules| &mut rules.zinger.hang.best),
        ("zinger.sky.peak", |rules| &mut rules.zinger.sky.peak),
        ("shift.follow", |rules| &mut rules.shift.follow),
        ("steal.their_safe", |rules| &mut rules.steal.their_safe),
    ];

    /// What a layer names: for each of those numbers, in the order they are
    /// listed, the value it gives it, if it names it at all.
    type Named = (Vec<Option<u32>>, Vec<Option<f32>>);

    fn named() -> impl Strategy<Value = Named> {
        let whole = prop::option::of(0u32..1_000_000);
        // Eighths, which are written out and read back to the last place.
        let part = prop::option::of((-800i16..=800).prop_map(|eighths| f32::from(eighths) / 8.0));
        (
            prop::collection::vec(whole, WHOLES.len()),
            prop::collection::vec(part, PARTS.len()),
        )
    }

    /// Writes out the layer that names these numbers, and makes the same
    /// changes to `rules` by hand.
    fn lay(named: &Named, rules: &mut Rules) -> String {
        let mut text = String::new();
        for ((name, place), value) in WHOLES.iter().zip(&named.0) {
            if let Some(value) = *value {
                writeln!(text, "{name} = {value}").unwrap();
                *place(rules) = value;
            }
        }
        for ((name, place), value) in PARTS.iter().zip(&named.1) {
            if let Some(value) = *value {
                writeln!(text, "{name} = {value:?}").unwrap();
                *place(rules) = value;
            }
        }
        text
    }

    proptest! {
        // Each case reads the whole of the built-in rules several times.
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn a_layer_gives_the_built_in_rules_with_the_numbers_it_names_changed_and_no_others(
            named in named(),
        ) {
            let mut wanted = Rules::default();
            let text = lay(&named, &mut wanted);
            let rules = Rules::layered(&[("a mod", &text)]).unwrap();
            prop_assert_eq!(rules, wanted, "from {:?}", text);
        }

        #[test]
        fn a_layer_laid_on_twice_is_the_layer_laid_on_once(named in named()) {
            let text = lay(&named, &mut Rules::default());
            let once = Rules::layered(&[("once", &text)]).unwrap();
            let twice = Rules::layered(&[("once", &text), ("again", &text)]).unwrap();
            prop_assert_eq!(twice, once, "from {:?}", text);
        }

        #[test]
        fn of_two_layers_each_changes_what_it_names_and_the_later_has_the_last_word(
            first in named(),
            second in named(),
        ) {
            // Whatever the first names and the second does not is still as
            // the first left it.
            let mut wanted = Rules::default();
            let first = lay(&first, &mut wanted);
            let second = lay(&second, &mut wanted);
            let rules = Rules::layered(&[("first", &first), ("second", &second)]).unwrap();
            prop_assert_eq!(rules, wanted, "from {:?} and then {:?}", first, second);
        }
    }

    proptest! {
        #[test]
        fn a_span_many_times_over_keeps_its_ends_in_order_and_neither_is_less_than_one(
            low in 0u32..=10_000,
            more in 0u32..=10_000,
            by in -4.0f32..40.0,
        ) {
            let span = Span {
                low,
                high: low + more,
            };
            let times = span.times(by);
            prop_assert!(1 <= times.low && times.low <= times.high, "{:?}", times);
            // No times over, or fewer than none, is as short as a span
            // gets.
            if by <= 0.0 {
                prop_assert_eq!(times, Span { low: 1, high: 1 });
            }
            // Once over is the span itself, if it began at one or more.
            if low >= 1 {
                prop_assert_eq!(span.times(1.0), span);
            }
        }
    }

    #[test]
    fn a_number_the_game_cannot_be_played_by_is_refused_and_named() {
        let wrong = |layer: &str| {
            let error = Rules::layered(&[("a mod", layer)]).expect_err("rules to be refused");
            format!("{error:#}")
        };
        assert_eq!(
            wrong("[pitch.hard.speed]\nlow = 80\nhigh = 40\n"),
            "in a mod: `pitch.hard.speed` has its low above its high"
        );
        assert_eq!(
            wrong("[pitch.easy.speed]\nlow = 0\n"),
            "in a mod: `pitch.easy.speed` has to be at least 1"
        );
        assert_eq!(
            wrong("[field]\nwall = 0.0\n"),
            "in a mod: `field.wall` has to be more than nought"
        );
        assert_eq!(
            wrong("[shift]\nmost = -0.1\n"),
            "in a mod: `shift.most` cannot be less than nought"
        );
        assert_eq!(
            wrong("[field]\ngravity = nan\n"),
            "in a mod: `field.gravity` is not a number the game can do sums with"
        );
        assert_eq!(
            wrong("[moon]\nslow = [1.5, inf]\n"),
            "in a mod: `moon.slow` is not a number the game can do sums with"
        );
    }
}
