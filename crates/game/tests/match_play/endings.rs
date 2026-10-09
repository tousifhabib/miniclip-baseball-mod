//! Games played through: each comes to an end, however it is played.

use super::{in_match, play_out, play_screen};
use crate::common::game;

#[test]
fn a_match_left_alone_is_lost_on_strikes() {
    let Some(mut script) = game("match") else {
        return;
    };
    // Nobody swings. Every pitch is offered again as soon as it is over.
    let mut last = String::new();
    for _ in 0..4000 {
        last = script
            .run("click 545 355; wait 30; state")
            .unwrap()
            .pop()
            .unwrap();
        if !in_match(&last) {
            break;
        }
    }
    assert!(
        last.starts_with("MatchLost")
            || last.starts_with("InningsTied")
            || last.starts_with("MatchWon"),
        "{last}"
    );
}

#[test]
fn matches_played_with_a_bat_come_to_an_end() {
    let mut ends = Vec::new();
    for seed in 1..=6 {
        // A swing 24 frames before the ball is gone meets it well.
        let (end, pitches) = play_out(seed, 24, (0.0, 0.0));
        if end.is_empty() {
            return;
        }
        assert!(pitches > 0, "seed {seed} threw nothing");
        eprintln!("seed {seed}: {pitches} pitches, then {end}");
        ends.push(end.split(',').next().unwrap_or_default().to_owned());
    }
    for end in &ends {
        assert!(
            ["MatchWon", "MatchLost", "InningsTied"].contains(&end.as_str()),
            "{ends:?}"
        );
    }
}

#[test]
fn matches_played_badly_come_to_an_end_too() {
    // Off-centre rings and mistimed swings keep the ball in the field,
    // where it has to be chased, caught and thrown in.
    let batters = [
        (22, (0.0, -25.0)),
        (26, (0.0, 30.0)),
        (24, (12.0, -40.0)),
        (21, (-10.0, 15.0)),
        (27, (6.0, -15.0)),
        (24, (-25.0, 0.0)),
        (24, (30.0, 10.0)),
    ];
    let mut seen_ends = std::collections::BTreeSet::new();
    for (index, (early, off)) in batters.into_iter().enumerate() {
        for seed in [11, 12, 13] {
            let (end, pitches) = play_out(seed + index as u64 * 10, early, off);
            if end.is_empty() {
                return;
            }
            let end = end.split(',').next().unwrap_or_default().to_owned();
            eprintln!("early {early} off {off:?} seed {seed}: {pitches} pitches, then {end}");
            assert!(
                ["MatchWon", "MatchLost", "InningsTied"].contains(&end.as_str()),
                "{end}"
            );
            seen_ends.insert(end);
        }
    }
    // Between them they should not all go the same way.
    assert!(seen_ends.len() > 1, "{seen_ends:?}");
}

#[test]
fn an_arcade_game_is_ten_pitches_and_the_target_can_be_hit() {
    // Swings aimed under the ball by different amounts drop it at different
    // depths, so between them some come down on the target.
    let mut best = 0;
    for (index, under) in [0.0, 10.0, 18.0, 26.0, 34.0, 42.0, -15.0, -30.0]
        .into_iter()
        .enumerate()
    {
        let (end, pitches, points) = play_screen("arcade", 40 + index as u64, 24, (0.0, under));
        if end.is_empty() {
            return;
        }
        eprintln!("ring {under} under: {pitches} pitches, {points} points, then {end}");
        assert!(end.starts_with("ArcadeFinish"), "{end}");
        assert_eq!(pitches, 10);
        best = best.max(points);
    }
    assert!(best > 0, "nobody hit the target");
}
