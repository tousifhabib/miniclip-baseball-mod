//! The numbers of the mods that change the ball in the field, the
//! fielders, the runners or the ground.

use serde::Deserialize;

use super::shapes::{Area, BySkill, ByTiming, Levels, Shape, Span};

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
    pub speed: Levels<f32>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoonRules {
    /// How many times as long the ball takes over its flight, for each
    /// level the mod can be set to, the lowest first.
    pub slow: Levels<f32>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinballRules {
    /// The share of its speed the ball keeps when it bounces, for each
    /// level the mod can be set to, the lowest first.
    pub keeps: Levels<f32>,
    /// The most a bounce can send the ball up by.
    pub hop: f32,
    /// How low a ball has to be for a fielder to get hold of it.
    pub low: f32,
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
pub struct ButterfingersRules {
    /// How often a fielder lets the ball go, out of every hundred goes at
    /// it, for each level the mod can be set to, the lowest first.
    pub chance: Levels<u32>,
    pub roll: f32,
    pub pop: f32,
    pub fumble_time: u32,
    pub gather_time: u32,
    pub told_time: u32,
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
