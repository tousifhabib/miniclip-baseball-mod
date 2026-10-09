//! The shapes a number comes in: one for each skill level, one for each
//! level of a setting, a span to draw from, an area of the view.

use serde::Deserialize;

use crate::settings::Difficulty;

/// A number for each level a mod's setting can be put at, the lowest
/// first. Levels are counted from 1.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct Levels<T>(Vec<T>);

impl<T: Copy> Levels<T> {
    /// How many levels there are.
    pub fn count(&self) -> u8 {
        self.0.len().min(usize::from(u8::MAX)) as u8
    }

    /// The number for this level. A level there is none of is taken as the
    /// nearest there is, and there is no number only if there are no
    /// levels at all.
    pub fn at(&self, level: u8) -> Option<T> {
        let last = self.0.len().checked_sub(1)?;
        self.0.get(usize::from(level.max(1) - 1).min(last)).copied()
    }
}

impl<T> From<Vec<T>> for Levels<T> {
    fn from(lowest_first: Vec<T>) -> Levels<T> {
        Levels(lowest_first)
    }
}

/// A number or a table that differs with the skill level chosen.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BySkill<T> {
    pub easy: T,
    pub medium: T,
    pub hard: T,
}

impl<T> BySkill<T> {
    pub fn at(&self, difficulty: Difficulty) -> &T {
        match difficulty {
            Difficulty::Easy => &self.easy,
            Difficulty::Medium => &self.medium,
            Difficulty::Hard => &self.hard,
        }
    }
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

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ring {
    pub over: f32,
    pub within: f32,
    pub points: u32,
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
