//! The timing window: what a swing does to a ball it meets, and how near
//! the best it was timed.

use serde::Deserialize;

use super::Pitch;
use crate::rules::PitchRules;

/// How well the bat met the ball, from the worst to the best.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    Poor,
    MediumPoor,
    Medium,
    Good,
}

impl Pitch {
    /// What a swing begun on `step` of this pitch comes to: the step on
    /// which the bat meets the ball, how well, and with what power. `None`
    /// is a miss. The bat meets the ball on the first frame the ball is in
    /// the band with the swing at a point in its window.
    pub fn swing_from(&self, rules: &PitchRules, step: usize) -> Option<(usize, Quality, f32)> {
        let rest = self.samples.get(step..)?;
        rest.iter().enumerate().find_map(|(frames, sample)| {
            let (quality, power) = sample
                .in_band(rules.band)
                .then(|| meets(rules, frames as u32))
                .flatten()?;
            Some((step + frames, quality, power))
        })
    }
}

/// What a swing made this many frames ago does to a ball in the band now:
/// how well it is met and with what power, or `None` for a miss.
pub fn meets(rules: &PitchRules, frames_since_swing: u32) -> Option<(Quality, f32)> {
    rules
        .window
        .iter()
        .find(|(frames, _, _)| *frames == frames_since_swing)
        .map(|&(_, quality, power)| (quality, power))
}

/// A timing window with `more` frames added to each end, each as good as
/// the frame that was the end and with its power. A window cannot begin
/// before the swing does.
pub fn widened(window: &[(u32, Quality, f32)], more: u32) -> Vec<(u32, Quality, f32)> {
    let first = window.iter().min_by_key(|&&(frames, ..)| frames).copied();
    let last = window.iter().max_by_key(|&&(frames, ..)| frames).copied();
    let (Some(first), Some(last)) = (first, last) else {
        return Vec::new();
    };
    let mut wider: Vec<(u32, Quality, f32)> = (1..=more.min(first.0))
        .rev()
        .map(|by| (first.0 - by, first.1, first.2))
        .collect();
    wider.extend_from_slice(window);
    wider.extend((1..=more).map(|by| (last.0 + by, last.1, last.2)));
    wider
}

/// How near the best a swing that meets the ball this many frames after it
/// began was timed: 1 on the best frame of the window, falling evenly to 0
/// on the frame of the window furthest from it. `None` for a miss.
///
/// The best frames are the ones that meet the ball best and, of those, the
/// ones with the least power, which send it furthest.
pub fn nearness(rules: &PitchRules, frames_since_swing: u32) -> Option<f32> {
    meets(rules, frames_since_swing)?;
    let top = rules.window.iter().map(|&(_, quality, _)| quality).max()?;
    let least = rules
        .window
        .iter()
        .filter(|&&(_, quality, _)| quality == top)
        .map(|&(.., power)| power)
        .fold(f32::INFINITY, f32::min);
    // How many frames off the nearest of the best frames.
    let off = |frames: u32| {
        rules
            .window
            .iter()
            .filter(|&&(_, quality, power)| quality == top && power == least)
            .map(|&(best, ..)| best.abs_diff(frames))
            .min()
    };
    let here = off(frames_since_swing)?;
    let furthest = rules.window.iter().filter_map(|&(frames, ..)| off(frames));
    Some(match furthest.max() {
        Some(furthest) if furthest > 0 => 1.0 - here as f32 / furthest as f32,
        // Every frame of the window is as good as the next.
        _ => 1.0,
    })
}
