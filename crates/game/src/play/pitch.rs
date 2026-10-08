//! The pitch: where it goes, and what a swing does to it.
//!
//! Nothing here touches the art. A pitch is worked out in full before it is
//! thrown, as a list of where the ball and its shadow are on each frame, so
//! that the player can be shown where it will cross before it leaves the
//! pitcher's hand.

use serde::Deserialize;

use crate::rng::Rng;
use crate::rules::{Band, MysteryRules, PitchRules, ThrowRules};

/// How well the bat met the ball, from the worst to the best.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    Poor,
    MediumPoor,
    Medium,
    Good,
}

/// A point of the batting view, in pixels.
pub type Point = (f32, f32);

/// What a mystery pitch turns out to be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Fastball,
    ChangeUp,
    Curve,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Fastball, Kind::ChangeUp, Kind::Curve];

    /// What the player is told it was.
    pub fn words(self) -> &'static str {
        match self {
            Kind::Fastball => "FASTBALL",
            Kind::ChangeUp => "CHANGE-UP",
            Kind::Curve => "CURVE",
        }
    }

    /// Changes the table a pitch is picked from so that it is one of this
    /// kind. A curve goes to the left or to the right.
    pub fn shape(self, table: &mut PitchRules, rules: &MysteryRules, to_left: bool) {
        match self {
            Kind::Fastball => table.speed = table.speed.times(rules.fast),
            Kind::ChangeUp => table.speed = table.speed.times(rules.slow),
            Kind::Curve => {
                let way = if to_left { -1.0 } else { 1.0 };
                table.swing.base += way * rules.curve_swing;
                table.dip.base += rules.curve_dip;
            }
        }
    }
}

/// The fixed points a pitch is drawn between, taken from the art.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mound {
    /// Where the ball and its shadow are as they leave the hand.
    pub ball: Point,
    pub shadow: Point,
    /// The points the art measures the ball's journey from.
    pub ball_from: Point,
    pub shadow_from: Point,
    /// The height on screen at which the shadow passes the batter.
    pub plate: f32,
    /// The strike zone: left, top, right, bottom.
    pub zone: [f32; 4],
}

/// Where the ball and its shadow are on one frame of a pitch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub ball: Point,
    pub shadow: Point,
    /// The size to draw both at, 1 being the art's own.
    pub size: f32,
    /// How solid both are, from 1 down to 0 as they fade past the batter.
    pub alpha: f32,
}

impl Sample {
    /// Whether the ball can be hit on this frame: its shadow is in the band.
    pub fn in_band(&self, band: Band) -> bool {
        self.shadow.1 > band.top && self.shadow.1 < band.bottom
    }
}

/// One pitch, worked out from the hand to past the batter.
#[derive(Clone, Debug, PartialEq)]
pub struct Pitch {
    pub samples: Vec<Sample>,
    /// Where the ball is as its shadow passes the batter.
    pub crosses: Point,
    /// Whether that is inside the strike zone.
    pub in_zone: bool,
}

/// What the pitcher has decided to throw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Choice {
    pub speed: f32,
    pub swing: f32,
    pub dip: f32,
    pub aim: Point,
}

impl Choice {
    /// Picks a pitch for this skill level.
    pub fn pick(rules: &PitchRules, throw: &ThrowRules, rng: &mut Rng) -> Choice {
        let speed = rules.speed.low + rng.below(rules.speed.high - rules.speed.low + 1);
        let mut curve = |curve: &crate::rules::Curve| {
            if curve.over == 0.0 {
                curve.base
            } else {
                curve.base + curve.over / (rng.below(curve.parts.max(1)) + 1) as f32
            }
        };
        let (swing, dip) = (curve(&rules.swing), curve(&rules.dip));
        let target = &rules.target;
        // He aims off to allow for the curve.
        let aim = (
            target.x + rng.below(target.width as u32) as f32 - swing * throw.swing_lead,
            target.y + rng.below(target.height as u32) as f32 + dip * throw.dip_lead,
        );
        Choice {
            speed: speed as f32,
            swing,
            dip,
            aim,
        }
    }
}

impl Pitch {
    /// Works out the whole of a pitch.
    pub fn throw(choice: &Choice, mound: &Mound, rules: &ThrowRules) -> Pitch {
        let (mut ball, mut shadow) = (mound.ball, mound.shadow);
        // The whole way each has to go, to the aim and to the ground under
        // it at the batter.
        let way = (
            choice.aim.0 - mound.ball_from.0,
            choice.aim.1 - mound.ball_from.1,
        );
        let shadow_way = (
            choice.aim.0 - mound.shadow_from.0,
            mound.plate - mound.shadow_from.1,
        );
        let mut pitch = Pitch {
            samples: Vec::new(),
            crosses: choice.aim,
            in_zone: false,
        };
        let mut alpha = 1.0f32;
        let mut crossed = false;
        // No pitch takes anything like this long: it is a guard against
        // numbers in a data file that would never bring the ball in.
        for _ in 0..3000 {
            // The nearer the ball, the faster it comes on.
            let come = (shadow.1 - mound.shadow_from.1).max(0.0);
            let step = come / rules.approach / choice.speed;
            ball.0 += way.0 * step + choice.swing;
            ball.1 += way.1 * step - choice.dip;
            shadow.0 += shadow_way.0 * step + choice.swing;
            shadow.1 += shadow_way.1 * step;
            if shadow.1 >= mound.plate {
                alpha -= rules.fade;
                if !crossed {
                    crossed = true;
                    pitch.crosses = ball;
                    let [left, top, right, bottom] = mound.zone;
                    pitch.in_zone =
                        (left..=right).contains(&ball.0) && (top..=bottom).contains(&ball.1);
                }
            }
            pitch.samples.push(Sample {
                ball,
                shadow,
                size: rules.size + come * rules.growth,
                alpha: alpha.max(0.0),
            });
            if alpha <= 0.0 {
                break;
            }
        }
        pitch
    }

    /// Makes a knuckleball of the pitch: the ball sways from side to side
    /// on its way in, `sway` pixels either way as it comes by the batter
    /// and less while it is far off and small, `turns` times there and
    /// back. `start` is where in a turn it sets off, from 0 to 1. Where
    /// the pitch crosses is worked out again.
    pub fn knuckle(&mut self, sway: f32, turns: f32, start: f32, mound: &Mound) {
        let Some(crossing) = self
            .samples
            .iter()
            .position(|sample| sample.shadow.1 >= mound.plate)
            .filter(|&step| step > 0)
        else {
            return;
        };
        let full_size = self.samples[crossing].size.max(0.001);
        for (step, sample) in self.samples.iter_mut().enumerate() {
            let gone = step as f32 / crossing as f32;
            // It leaves the hand straight, and takes a moment to start.
            let eased = (gone / 0.2).min(1.0);
            let turn = (start + turns * gone) * std::f32::consts::TAU;
            let aside = sway * (sample.size / full_size) * eased * turn.sin();
            sample.ball.0 += aside;
            sample.shadow.0 += aside;
        }
        let ball = self.samples[crossing].ball;
        let [left, top, right, bottom] = mound.zone;
        self.crosses = ball;
        self.in_zone = (left..=right).contains(&ball.0) && (top..=bottom).contains(&ball.1);
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rules;
    use crate::settings::Difficulty;

    /// The fixed points as they are in the game's art.
    pub(crate) fn mound() -> Mound {
        Mound {
            ball: (286.1, 137.85),
            shadow: (286.1, 233.35),
            ball_from: (285.85, 137.8),
            shadow_from: (285.85, 230.35),
            plate: 351.0,
            zone: [246.2, 190.6, 341.1, 305.7],
        }
    }

    fn straight(aim: Point, speed: f32) -> Choice {
        Choice {
            speed,
            swing: 0.0,
            dip: 0.0,
            aim,
        }
    }

    #[test]
    fn a_straight_pitch_crosses_where_it_was_aimed() {
        let rules = Rules::default();
        let pitch = Pitch::throw(&straight((300.0, 250.0), 70.0), &mound(), &rules.throw);
        assert!((pitch.crosses.0 - 300.0).abs() < 4.0, "{:?}", pitch.crosses);
        assert!((pitch.crosses.1 - 250.0).abs() < 8.0, "{:?}", pitch.crosses);
        assert!(pitch.in_zone);
    }

    #[test]
    fn a_pitch_aimed_wide_is_outside_the_zone() {
        let rules = Rules::default();
        let pitch = Pitch::throw(&straight((400.0, 250.0), 70.0), &mound(), &rules.throw);
        assert!(!pitch.in_zone);
    }

    #[test]
    fn the_ball_comes_on_faster_and_larger_and_then_fades() {
        let rules = Rules::default();
        let pitch = Pitch::throw(&straight((300.0, 250.0), 70.0), &mound(), &rules.throw);
        let samples = &pitch.samples;
        let early = samples[10].shadow.1 - samples[9].shadow.1;
        let late = samples[samples.len() - 12].shadow.1 - samples[samples.len() - 13].shadow.1;
        assert!(late > early * 5.0, "{early} then {late}");
        assert!(samples.last().unwrap().size > samples[0].size * 2.0);
        assert_eq!(samples[0].alpha, 1.0);
        assert_eq!(samples.last().unwrap().alpha, 0.0);
    }

    #[test]
    fn a_slower_setting_takes_more_frames() {
        let rules = Rules::default();
        let fast = Pitch::throw(&straight((300.0, 250.0), 35.0), &mound(), &rules.throw);
        let slow = Pitch::throw(&straight((300.0, 250.0), 70.0), &mound(), &rules.throw);
        assert!(slow.samples.len() > fast.samples.len());
        // Sanity: a pitch is a second or three, not an age.
        assert!(
            (40..400).contains(&slow.samples.len()),
            "{}",
            slow.samples.len()
        );
    }

    #[test]
    fn swing_carries_the_ball_sideways() {
        let rules = Rules::default();
        let mut curving = straight((300.0, 250.0), 60.0);
        curving.swing = 0.5;
        let straight = Pitch::throw(&straight((300.0, 250.0), 60.0), &mound(), &rules.throw);
        let curved = Pitch::throw(&curving, &mound(), &rules.throw);
        assert!(curved.crosses.0 > straight.crosses.0 + 10.0);
    }

    #[test]
    fn the_pitcher_keeps_to_what_the_rules_allow() {
        let rules = Rules::default();
        let mut rng = Rng::new(9);
        for difficulty in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard] {
            let table = rules.pitch.at(difficulty);
            for _ in 0..200 {
                let choice = Choice::pick(table, &rules.throw, &mut rng);
                assert!((table.speed.low as f32..=table.speed.high as f32).contains(&choice.speed));
                let pitch = Pitch::throw(&choice, &mound(), &rules.throw);
                assert!(pitch.samples.len() < 1000, "{choice:?}");
            }
        }
        // On easy the ball is thrown straight.
        let easy = Choice::pick(rules.pitch.at(Difficulty::Easy), &rules.throw, &mut rng);
        assert_eq!((easy.swing, easy.dip), (0.0, 0.0));
    }

    #[test]
    fn a_knuckleball_sways_and_crosses_somewhere_near_where_it_was_going() {
        let rules = Rules::default();
        let straight = Pitch::throw(&straight((300.0, 250.0), 60.0), &mound(), &rules.throw);
        let mut far_off = 0.0f32;
        for start in [0.0, 0.2, 0.45, 0.7, 0.9] {
            let mut pitch = straight.clone();
            pitch.knuckle(16.0, 2.5, start, &mound());
            // It leaves the hand where it would have, and no frame of it is
            // further out than the sway.
            assert_eq!(pitch.samples[0].ball, straight.samples[0].ball);
            let aside: Vec<f32> = pitch
                .samples
                .iter()
                .zip(&straight.samples)
                .map(|(swayed, plain)| swayed.ball.0 - plain.ball.0)
                .collect();
            assert!(
                aside.iter().all(|aside| aside.abs() <= 16.0 * 1.6),
                "{aside:?}"
            );
            // It goes to both sides on the way.
            assert!(aside.iter().any(|&aside| aside > 3.0), "{start}: {aside:?}");
            assert!(
                aside.iter().any(|&aside| aside < -3.0),
                "{start}: {aside:?}"
            );
            // Only side to side: its height is as it was.
            assert_eq!(pitch.crosses.1, straight.crosses.1);
            let off = pitch.crosses.0 - straight.crosses.0;
            assert!(off.abs() <= 16.0, "{off}");
            far_off = far_off.max(off.abs());
            // The shadow goes with the ball.
            let shadow = pitch.samples[20].shadow.0 - straight.samples[20].shadow.0;
            assert_eq!(shadow, aside[20]);
        }
        assert!(far_off > 8.0, "{far_off}");
    }

    #[test]
    fn a_knuckleball_that_sways_out_of_the_zone_is_a_ball() {
        let rules = Rules::default();
        // Aimed just inside the zone's right edge.
        let mut pitch = Pitch::throw(&straight((338.0, 250.0), 60.0), &mound(), &rules.throw);
        assert!(pitch.in_zone);
        // A quarter of a turn on at the batter, which is as far right as
        // it goes.
        pitch.knuckle(16.0, 2.0, 0.25, &mound());
        assert!(pitch.crosses.0 > 341.1, "{:?}", pitch.crosses);
        assert!(!pitch.in_zone);
    }

    #[test]
    fn a_mystery_pitch_is_quick_or_slow_or_curves_more() {
        let rules = Rules::default();
        let usual = rules.pitch.at(Difficulty::Medium);
        let shaped = |kind: Kind, to_left: bool| {
            let mut table = usual.clone();
            kind.shape(&mut table, &rules.mystery, to_left);
            table
        };
        let frames = |table: &PitchRules| {
            let choice = straight((300.0, 250.0), table.speed.high as f32);
            Pitch::throw(&choice, &mound(), &rules.throw).samples.len()
        };
        let (fast, slow) = (shaped(Kind::Fastball, false), shaped(Kind::ChangeUp, false));
        assert!(frames(&fast) * 10 < frames(usual) * 8);
        assert!(frames(&slow) * 10 > frames(usual) * 13);
        // A curve takes as long as any pitch, and swings and drops more.
        for to_left in [false, true] {
            let curve = shaped(Kind::Curve, to_left);
            assert_eq!(curve.speed, usual.speed);
            assert_eq!((curve.swing.base - usual.swing.base).abs(), 1.5);
            assert_eq!(curve.swing.base < usual.swing.base, to_left);
            assert!(curve.dip.base > usual.dip.base);
        }
        let names: Vec<&str> = Kind::ALL.iter().map(|kind| kind.words()).collect();
        assert_eq!(names, ["FASTBALL", "CHANGE-UP", "CURVE"]);
    }

    #[test]
    fn a_widened_window_has_frames_at_each_end_as_good_as_the_end_was() {
        let rules = Rules::default();
        let medium = &rules.pitch.at(Difficulty::Medium).window;
        assert_eq!(widened(medium, 0), *medium);
        let wider = widened(medium, 2);
        let frames: Vec<u32> = wider.iter().map(|&(frames, ..)| frames).collect();
        assert_eq!(frames, [6, 7, 8, 9, 10, 11, 12, 13, 14]);
        assert_eq!(wider[0], (6, Quality::Poor, 25.0));
        assert_eq!(wider[8], (14, Quality::Medium, 17.0));
        // The frames that were there are as they were.
        assert_eq!(wider[2..7], medium[..]);
        // It cannot begin before the swing does.
        assert_eq!(widened(&[(1, Quality::Good, 14.0)], 3).len(), 5);
        assert!(widened(&[], 3).is_empty());
    }

    #[test]
    fn a_swing_only_meets_the_ball_inside_its_window() {
        let rules = Rules::default();
        let easy = rules.pitch.at(Difficulty::Easy);
        assert_eq!(meets(easy, 6), None);
        assert_eq!(meets(easy, 7), Some((Quality::MediumPoor, 18.0)));
        assert_eq!(meets(easy, 11), Some((Quality::Good, 14.0)));
        assert_eq!(meets(easy, 14), None);
        // The harder the level, the fewer frames there are to hit in.
        let frames = |d| rules.pitch.at(d).window.len();
        assert!(frames(Difficulty::Easy) > frames(Difficulty::Medium));
        assert!(frames(Difficulty::Medium) > frames(Difficulty::Hard));
    }

    #[test]
    fn a_swing_is_nearest_the_best_on_the_frame_that_sends_the_ball_furthest() {
        let rules = Rules::default();
        let easy = rules.pitch.at(Difficulty::Easy);
        // Three frames of the easy window are good. The middle one has the
        // least power, and the others are judged by how far off it they
        // are.
        assert_eq!(nearness(easy, 11), Some(1.0));
        assert_eq!(nearness(easy, 10), Some(0.75));
        assert_eq!(nearness(easy, 12), Some(0.75));
        assert_eq!(nearness(easy, 13), Some(0.5));
        assert_eq!(nearness(easy, 7), Some(0.0));
        // A miss is not near anything.
        assert_eq!(nearness(easy, 6), None);
        assert_eq!(nearness(easy, 14), None);
        // Two frames of the medium window are as good as each other.
        let medium = rules.pitch.at(Difficulty::Medium);
        assert_eq!(nearness(medium, 10), Some(1.0));
        assert_eq!(nearness(medium, 11), Some(1.0));
        assert_eq!(nearness(medium, 12), Some(0.5));
        assert_eq!(nearness(medium, 8), Some(0.0));
    }

    #[test]
    fn at_every_level_one_frame_is_the_best_and_one_the_worst() {
        let rules = Rules::default();
        for difficulty in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard] {
            let table = rules.pitch.at(difficulty);
            let near: Vec<f32> = table
                .window
                .iter()
                .map(|&(frames, ..)| nearness(table, frames).unwrap())
                .collect();
            assert!(near.contains(&1.0), "{difficulty:?}: {near:?}");
            assert!(near.contains(&0.0), "{difficulty:?}: {near:?}");
            assert!(near.iter().all(|near| (0.0..=1.0).contains(near)));
        }
    }

    #[test]
    fn a_window_of_one_frame_is_all_best() {
        let rules = Rules::default();
        let mut table = rules.pitch.at(Difficulty::Hard).clone();
        table.window.truncate(1);
        let (frames, ..) = table.window[0];
        assert_eq!(nearness(&table, frames), Some(1.0));
    }
}
