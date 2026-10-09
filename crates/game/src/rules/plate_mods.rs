//! The numbers of the mods that change the pitch, the swing or what a
//! run is worth.

use serde::Deserialize;

use super::ball::PitchRules;
use super::shapes::Area;

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
