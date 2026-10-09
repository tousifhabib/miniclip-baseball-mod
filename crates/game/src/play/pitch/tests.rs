use super::*;
use crate::rng::Rng;
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
