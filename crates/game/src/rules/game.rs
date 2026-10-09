//! The numbers of the game itself: a match, a full match, the count, the
//! arcade game, the teams and the sounds.

use serde::Deserialize;

use super::shapes::{Area, BySkill, Ring};
use crate::settings::Difficulty;

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
pub struct CountRules {
    pub strikes: u32,
    pub balls: u32,
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
    pub runs: BySkill<Vec<u32>>,
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
pub struct SoundRules {
    /// How loud each sound is, by its name in the art, from 0 to 1.
    pub levels: std::collections::BTreeMap<String, f32>,
    pub music: String,
    pub crowd: String,
}
