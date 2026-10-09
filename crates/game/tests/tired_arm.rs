//! The tired arm mod: the pitcher slows and loses the strike zone as his
//! pitches mount up, until a fresh pitcher comes in for him.

mod common;

use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{
    LEAVE, ON, game_modded, long_match, long_match_ruled, next, number, pitch, ready, said,
};

/// A pitcher who tires from his third pitch, is spent by his seventh, and
/// gives way after his eighth.
const QUICK: &str = "[tired_arm]\nfresh = 2\nspent = 6\nrelief = 8\nslow = 1.5\nwild = 3.0\n";

/// Lets the pitch that is coming go by, and asks for the next.
fn leave(script: &mut Script) {
    pitch(script, LEAVE, ON);
    next(script);
}

/// How many frames the pitch that is coming takes.
fn frames(now: &str) -> f32 {
    number(now, "after ").unwrap()
}

#[test]
fn his_pitches_slow_as_they_mount_up_until_a_new_pitcher_comes_in() {
    let mods = [Mod::TimingIndicator, Mod::TiredArm];
    let (Some(mut script), Some(mut plain)) = (
        long_match_ruled(1, &mods, QUICK),
        long_match(1, &[Mod::TimingIndicator]),
    ) else {
        return;
    };
    let usual = frames(&ready(&mut plain));
    let mut last = usual;
    for thrown in 0..8 {
        let now = ready(&mut script);
        assert!(now.contains(&format!(", arm {thrown} tired ")), "{now}");
        assert!(!now.contains("pitcher"), "{now}");
        assert_eq!(said(&script, "pitches"), [format!("PITCHES {thrown}")]);
        assert!(said(&script, "newPitcher").is_empty());
        let tired = number(&now, "tired ").unwrap();
        match thrown {
            // Fresh, he pitches as he always did.
            0..=2 => assert_eq!((tired, frames(&now)), (0.0, usual), "{now}"),
            // Then each pitch is slower than the last, until he is spent.
            3..=6 => assert!(frames(&now) > last, "{now}"),
            _ => assert_eq!((tired, frames(&now)), (1.0, last), "{now}"),
        }
        last = frames(&now);
        leave(&mut script);
    }
    assert!(last > usual * 1.3, "{last} from {usual}");
    // Eight thrown: he gives way to a fresh arm.
    let now = ready(&mut script);
    assert!(now.contains(", arm 0 tired 0.00, pitcher 2"), "{now}");
    assert_eq!(frames(&now), usual);
    assert_eq!(said(&script, "newPitcher"), ["NEW PITCHER"]);
    assert_eq!(said(&script, "pitches"), ["PITCHES 0"]);
    // The word of it does not stay.
    leave(&mut script);
    let now = ready(&mut script);
    assert!(now.contains(", arm 1 tired 0.00, pitcher 2"), "{now}");
    assert!(said(&script, "newPitcher").is_empty());
}

/// How many of the next `pitches` are outside the strike zone.
fn outside(script: &mut Script, pitches: u32) -> u32 {
    let mut outside = 0;
    for _ in 0..pitches {
        outside += u32::from(ready(script).contains("outside the zone"));
        leave(script);
    }
    outside
}

#[test]
fn a_spent_pitcher_misses_the_strike_zone_far_more_often() {
    // One who is spent after a pitch, and never taken off.
    let spent = "[tired_arm]\nfresh = 0\nspent = 1\nrelief = 1000\nwild = 3.0\n";
    let mods = [Mod::TimingIndicator, Mod::TiredArm];
    let (Some(mut script), Some(mut plain)) = (
        long_match_ruled(1, &mods, spent),
        long_match(1, &[Mod::TimingIndicator]),
    ) else {
        return;
    };
    let (tired, fresh) = (outside(&mut script, 40), outside(&mut plain, 40));
    assert!(fresh < 15, "{fresh}");
    assert!(tired > fresh + 15, "{tired} against {fresh}");
}

#[test]
fn without_the_mod_or_in_the_arcade_game_no_arm_tires() {
    for (screen, mods) in [
        ("match", vec![Mod::TimingIndicator]),
        ("arcade", vec![Mod::TimingIndicator, Mod::TiredArm]),
    ] {
        let Some(mut script) = game_modded(screen, 1, &mods) else {
            return;
        };
        let first = frames(&ready(&mut script));
        for _ in 0..6 {
            let now = ready(&mut script);
            assert!(!now.contains("arm"), "{screen}: {now}");
            assert_eq!(frames(&now), first, "{screen}: {now}");
            leave(&mut script);
        }
        assert!(said(&script, "pitches").is_empty());
    }
}
