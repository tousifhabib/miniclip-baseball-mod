//! The pinball park mod: a ball that keeps bouncing, and a wall and foul
//! lines that send it back.

mod common;

use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{game_levelled, game_modded, number, pitch, pitch_seen};

/// Ways to swing at the first pitch for balls that get to the outfield on
/// the ground: how late, and where to hold the ring.
const HITS: [(i32, (f32, f32)); 4] = [
    (2, (0.0, 8.0)),
    (2, (0.0, -8.0)),
    (2, (0.0, 0.0)),
    (2, (20.0, 5.0)),
];

fn rebounds(state: &str) -> u32 {
    number(state, "rebounds ").unwrap_or(0.0) as u32
}

/// Plays the first pitch of a game. Returns how many times the ball was
/// sent back, how many frames it was in the field, and the state after.
fn played(script: &mut Script, hit: (i32, (f32, f32))) -> (u32, u32, String) {
    let mut frames = 0;
    let after = pitch_seen(script, hit.0, hit.1, |_, now| {
        frames += u32::from(now.contains(": Fielding"));
    });
    (rebounds(&after), frames, after)
}

#[test]
fn the_ball_comes_back_off_the_wall_again_and_again_and_the_play_still_ends() {
    let (mut most, mut longer) = (0, 0);
    for seed in [1, 4, 6] {
        for hit in HITS {
            let Some(mut usual) = game_modded("match", seed, &[Mod::TimingIndicator]) else {
                return;
            };
            let (none, quick, _) = played(&mut usual, hit);
            assert_eq!(none, 0);

            let mut park = game_levelled("match", seed, Mod::PinballPark, 5).unwrap();
            let (back, slow, after) = played(&mut park, hit);
            // It ends with the next pitch on offer, as any play does.
            assert!(after.contains(": Ready"), "{after}");
            most = most.max(back);
            longer += u32::from(slow > quick);
        }
    }
    assert!(most >= 3, "{most}");
    assert!(longer >= 6, "{longer}");
}

#[test]
fn it_bounces_more_the_higher_the_mod_is_set() {
    let total = |level: u8| {
        let mut all = 0;
        for seed in [1, 4, 6] {
            for hit in HITS {
                let mut park = game_levelled("match", seed, Mod::PinballPark, level)?;
                all += played(&mut park, hit).0;
            }
        }
        Some(all)
    };
    let Some(low) = total(1) else {
        return;
    };
    let high = total(5).unwrap();
    assert!(low < high, "{low} against {high}");
}

#[test]
fn a_ball_hit_over_the_wall_on_the_fly_is_still_a_home_run() {
    // The first pitch of the first game, timed as well as can be with the
    // ring on the ball, is one as the game was.
    let Some(mut park) = game_levelled("match", 1, Mod::PinballPark, 5) else {
        return;
    };
    let after = pitch(&mut park, 0, (0.0, 0.0));
    assert!(after.contains("score 1 of 3"), "{after}");
    assert_eq!(rebounds(&after), 0);
}

#[test]
fn the_arcade_game_is_left_as_it_was() {
    let mut ends = Vec::new();
    for park in [false, true] {
        let script = if park {
            game_levelled("arcade", 1, Mod::PinballPark, 5)
        } else {
            game_modded("arcade", 1, &[Mod::TimingIndicator])
        };
        let Some(mut script) = script else {
            return;
        };
        pitch(&mut script, 2, (0.0, 8.0));
        script.run("wait 500").unwrap();
        ends.push(common::state(&mut script));
    }
    assert_eq!(ends[0], ends[1]);
}
