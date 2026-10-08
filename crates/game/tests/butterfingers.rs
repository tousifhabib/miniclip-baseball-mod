//! The butterfingers mod: fielders who let the ball go, as often as the mod
//! is set to.

mod common;

use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{
    game_levelled, game_modded, next, number, pitch, pitch_seen, playing, said, sounds, state,
};

/// The highest level the mod can be set to, at which a fielder lets go of
/// the ball every time, and the lowest.
const ALWAYS: u8 = 5;
const SELDOM: u8 = 1;
/// The ring on the ball.
const ON: (f32, f32) = (0.0, 0.0);
/// Games whose first pitch can be hit for a fly ball that is caught as the
/// game was, each with where to hold the ring for it.
const FLIES: [(u64, (f32, f32)); 3] = [(1, (0.0, 35.0)), (2, (0.0, 20.0)), (3, (0.0, 20.0))];
/// Where to hold the ring for a ball along the ground that gets the batter
/// to first, in the first game.
const GROUNDER: (f32, f32) = (0.0, -18.0);

/// A match as the game was, with the timing bar up to say when to swing.
fn usual(seed: u64) -> Option<Script> {
    game_modded("match", seed, &[Mod::TimingIndicator])
}

/// A match with fielders who let the ball go as often as `level` says.
fn clumsy(seed: u64, level: u8) -> Option<Script> {
    game_levelled("match", seed, Mod::Butterfingers, level)
}

/// How many times the state says a fielder has let the ball go.
fn let_go(state: &str) -> u32 {
    number(state, "let go ").map_or(0, |times| times as u32)
}

/// Plays one pitch, swung at as well as can be with the ring held `off`.
/// Returns the state after, each word that went up over a fielder, in the
/// order they went up, and how many frames the ball was in the field.
fn play(script: &mut Script, late: i32, off: (f32, f32)) -> (String, Vec<String>, u32) {
    let (mut words, mut last, mut frames) = (Vec::new(), None, 0);
    let after = pitch_seen(script, late, off, |script, now| {
        frames += u32::from(now.contains(": Fielding"));
        let word = said(script, "butterWord").pop();
        if word != last {
            words.extend(word.clone());
            last = word;
        }
    });
    (after, words, frames)
}

/// Plays a match to its end, swinging at every pitch on a step the bar
/// calls best with the ring held a different way each time. Returns the
/// screen it ended on, the most outs there were, and how often the ball
/// was let go.
fn play_out(script: &mut Script) -> (String, u32, u32) {
    const OFF: [(f32, f32); 6] = [
        (0.0, -18.0),
        (-25.0, 6.0),
        (25.0, -8.0),
        (-12.0, 14.0),
        (14.0, -24.0),
        (0.0, 4.0),
    ];
    let (mut outs, mut slips) = (0, 0);
    for pitched in 0..500 {
        let now = pitch(script, 0, OFF[pitched % OFF.len()]);
        if !playing(&now) {
            return (now, outs, slips);
        }
        outs = outs.max(number(&now, "outs ").unwrap_or(0.0) as u32);
        slips = let_go(&now);
        next(script);
    }
    panic!("the match never ended: {}", state(script));
}

#[test]
fn a_fly_that_would_be_caught_is_dropped_and_the_batter_is_safe() {
    for (seed, off) in FLIES {
        let Some(mut script) = usual(seed) else {
            return;
        };
        // As the game was, the fielder under it takes it and the batter
        // is out.
        let (after, words, _) = play(&mut script, 0, off);
        assert!(after.contains("outs 1, count 0-0, bases ---"), "{after}");
        assert!(sounds(&mut script).contains(&"umpire_out_1".to_owned()));
        assert!(words.is_empty(), "{words:?}");

        let mut script = clumsy(seed, ALWAYS).unwrap();
        let (after, words, _) = play(&mut script, 0, off);
        assert!(after.contains("outs 0, count 0-0, bases x--"), "{after}");
        assert_eq!(words.first().map(String::as_str), Some("DROPPED!"));
        let heard = sounds(&mut script);
        assert!(!heard.contains(&"umpire_out_1".to_owned()), "{heard:?}");
        assert!(heard.contains(&"crowd_smallCheer".to_owned()), "{heard:?}");
        assert!(let_go(&after) >= 1, "{after}");
    }
}

#[test]
fn a_ball_on_the_ground_is_fumbled_once_and_then_picked_up() {
    let Some(mut script) = usual(1) else {
        return;
    };
    let (after, words, quick) = play(&mut script, 0, GROUNDER);
    assert!(after.contains("outs 0, count 0-0, bases x--"), "{after}");
    assert!(words.is_empty());

    let mut script = clumsy(1, ALWAYS).unwrap();
    let (after, words, slow) = play(&mut script, 0, GROUNDER);
    // He fumbles it, has it at the second go, and his throw to first is
    // dropped there. The batter is where he would have been, later.
    assert_eq!(words, ["FUMBLED!", "DROPPED!"]);
    assert!(after.contains("outs 0, count 0-0, bases x--"), "{after}");
    assert_eq!(let_go(&after), 2);
    assert!(slow > quick + 40, "{slow} against {quick}");
}

/// A hit that puts a runner on first, and then a weak one back towards the
/// mound, which as the game was gets the runner thrown out at second.
/// Returns the state after the second play, and the words that went up in
/// it.
fn runner_on_first_and_a_weak_hit(script: &mut Script) -> (String, Vec<String>) {
    let (first, ..) = play(script, 0, GROUNDER);
    assert!(first.contains("bases x--"), "{first}");
    next(script);
    let (after, words, _) = play(script, 3, (0.0, -45.0));
    (after, words)
}

#[test]
fn a_throw_that_would_put_a_runner_out_is_dropped_at_the_base() {
    let Some(mut script) = usual(1) else {
        return;
    };
    let (after, _) = runner_on_first_and_a_weak_hit(&mut script);
    assert!(after.contains("outs 1,"), "{after}");

    let mut script = clumsy(1, ALWAYS).unwrap();
    let (after, words) = runner_on_first_and_a_weak_hit(&mut script);
    // Nobody is out, and both are on base.
    assert!(after.contains("outs 0, count 0-0, bases xx-"), "{after}");
    assert!(words.contains(&"DROPPED!".to_owned()), "{words:?}");
}

#[test]
fn with_fielders_who_always_let_go_nobody_is_out_in_the_field() {
    let mut outs_as_it_was = 0;
    for seed in [1, 2, 3] {
        let Some(mut script) = usual(seed) else {
            return;
        };
        let (_, outs, slips) = play_out(&mut script);
        outs_as_it_was += outs;
        assert_eq!(slips, 0);

        // The batter meets every pitch, so there are no strike-outs, and
        // nothing the fielders do gets anybody out: the match is won.
        let mut script = clumsy(seed, ALWAYS).unwrap();
        let (end, outs, slips) = play_out(&mut script);
        assert_eq!(outs, 0, "seed {seed}");
        assert!(end.starts_with("MatchWon"), "seed {seed}: {end}");
        assert!(slips > 5, "seed {seed}: {slips}");
    }
    assert!(outs_as_it_was > 0);
}

#[test]
fn how_often_the_ball_is_let_go_is_by_the_level_the_mod_is_set_to() {
    let slips = |level: u8| {
        let mut all = 0;
        for seed in [1, 2, 3, 4] {
            let mut script = clumsy(seed, level)?;
            all += play_out(&mut script).2;
        }
        Some(all)
    };
    let Some(seldom) = slips(SELDOM) else {
        return;
    };
    let (middling, always) = (slips(3).unwrap(), slips(ALWAYS).unwrap());
    assert!(seldom > 0, "{seldom}");
    assert!(
        seldom < middling && middling < always,
        "{seldom} {middling} {always}"
    );
}

#[test]
fn the_word_comes_down_again_even_when_the_play_is_over() {
    let (seed, off) = FLIES[0];
    let Some(mut script) = clumsy(seed, ALWAYS) else {
        return;
    };
    let (after, words, _) = play(&mut script, 0, off);
    assert!(after.contains(": Ready"), "{after}");
    assert!(!words.is_empty());
    // The last of them may still be up as the next pitch is offered.
    script.run("wait 100").unwrap();
    assert!(said(&script, "butterWord").is_empty());
}

#[test]
fn the_arcade_game_has_no_fielders_to_let_anything_go() {
    let mut ends = Vec::new();
    for clumsy in [false, true] {
        let script = if clumsy {
            game_levelled("arcade", 1, Mod::Butterfingers, ALWAYS)
        } else {
            game_modded("arcade", 1, &[Mod::TimingIndicator])
        };
        let Some(mut script) = script else {
            return;
        };
        let mut states = Vec::new();
        for late in [0, 1, 2] {
            pitch(&mut script, late, ON);
            script.run("wait 300").unwrap();
            states.push(state(&mut script));
            assert!(said(&script, "butterWord").is_empty());
            next(&mut script);
        }
        ends.push(states);
    }
    assert_eq!(ends[0], ends[1]);
}
