//! The bullet time mod: holding the space bar slows the pitch as it comes
//! to the plate, for as long as a meter lasts that hits fill up again.

mod common;

use bb_engine::input::Key;
use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{game_modded, long_match, long_match_ruled, next, number, pitch_seen, said, state};

/// The ring on the ball, and where to hold it for a ball topped along the
/// ground that gets the batter to first, on the second pitch of the first
/// game. And a swing that never comes.
const ON: (f32, f32) = (0.0, 0.0);
const TOPPED: (f32, f32) = (0.0, -30.0);
const LEAVE: i32 = 1000;

fn game(bullet_time: bool) -> Option<Script> {
    let mut mods = vec![Mod::TimingIndicator];
    if bullet_time {
        mods.push(Mod::BulletTime);
    }
    long_match(1, &mods)
}

/// Waits for the pitcher to stand ready for the next pitch, and returns how
/// things stand then.
fn ready(script: &mut Script) -> String {
    for _ in 0..600 {
        let now = state(script);
        if now.contains("Settling") {
            return now;
        }
        script.run("wait 1").unwrap();
    }
    panic!("the next pitch never came: {}", state(script));
}

/// What is left in the meter.
fn meter(now: &str) -> u32 {
    number(now, ", bullet time ").expect("a meter") as u32
}

/// Plays the next pitch, swinging `late` steps after the best step for it
/// with the ring on the ball, and with the space bar held or not. Returns how many frames the ball was in
/// flight before anything came of it, whether it was ever held back, and
/// how things stood when the play was over.
fn play(script: &mut Script, late: i32, held: bool) -> (u32, bool, String) {
    play_with(script, late, ON, held)
}

/// The same, with the ring held `off` from the ball.
fn play_with(script: &mut Script, late: i32, off: (f32, f32), held: bool) -> (u32, bool, String) {
    ready(script);
    if held {
        script.run("hold space").unwrap();
    }
    let (mut frames, mut slowed) = (0, false);
    let after = pitch_seen(script, late, off, |_, now| {
        frames += u32::from(now.contains(": Flight"));
        slowed |= now.contains(" slowed");
    });
    script.run("lift space").unwrap();
    next(script);
    (frames, slowed, after)
}

#[test]
fn holding_the_key_holds_the_ball_back_near_the_plate_until_the_meter_is_empty() {
    let Some(mut script) = game(true) else {
        return;
    };
    assert_eq!(meter(&ready(&mut script)), 120);
    assert_eq!(said(&script, "bulletWords"), ["SLOW: SPACE"]);
    // A pitch with the key left alone, to measure the next by.
    let (usual, slowed, after) = play(&mut script, LEAVE, false);
    assert!(!slowed);
    assert_eq!(meter(&after), 120);
    // Held, the last forty frames of the flight take three times as long,
    // which is all the meter has.
    let (frames, slowed, after) = play(&mut script, LEAVE, true);
    assert!(slowed);
    assert_eq!(frames, usual + 80);
    assert_eq!(meter(&after), 0);
    // With nothing in the meter the key does nothing.
    let (frames, slowed, after) = play(&mut script, LEAVE, true);
    assert!(!slowed);
    assert_eq!(frames, usual);
    assert_eq!(meter(&after), 0);
}

#[test]
fn a_swing_made_in_bullet_time_meets_the_ball_and_ends_it() {
    let Some(mut script) = game(true) else {
        return;
    };
    let before = ready(&mut script);
    let score = number(&before, "score ").unwrap();
    // The best step to swing on is one the ball is held back on.
    let (_, slowed, after) = play(&mut script, 0, true);
    assert!(slowed);
    // It was timed as well as a swing can be: over the wall, which fills
    // the meter again.
    assert_eq!(number(&after, "score ").unwrap(), score + 1.0, "{after}");
    assert_eq!(meter(&after), 120);
}

#[test]
fn a_hit_puts_half_the_meter_back() {
    let Some(mut script) = game(true) else {
        return;
    };
    let (_, _, after) = play(&mut script, LEAVE, true);
    assert_eq!(meter(&after), 0);
    let (_, _, after) = play_with(&mut script, 0, TOPPED, false);
    assert!(after.contains("bases x--"), "{after}");
    assert_eq!(meter(&after), 60);
    // A walk is no hit, and puts nothing back.
    let walks = "[count]\nballs = 1\n[pitch.medium]\n\
                 target = { x = 420.0, y = 250.0, width = 2.0, height = 2.0 }\n";
    let mods = [Mod::TimingIndicator, Mod::BulletTime];
    let mut script = long_match_ruled(1, &mods, walks).unwrap();
    let (_, _, after) = play(&mut script, LEAVE, true);
    assert!(after.contains("bases x--"), "{after}");
    assert_eq!(meter(&after), 0);
}

#[test]
fn without_the_mod_the_key_is_not_the_games() {
    let Some(mut script) = game(false) else {
        return;
    };
    let now = ready(&mut script);
    assert!(!now.contains("bullet time"), "{now}");
    assert!(said(&script, "bulletWords").is_empty());
    let (usual, ..) = play(&mut script, LEAVE, false);
    let (frames, slowed, _) = play(&mut script, LEAVE, true);
    assert!(!slowed);
    assert_eq!(frames, usual);
    // The window keeps the space bar for itself.
    assert!(!script.runner.key(Key::Char(' ')));
    // With the mod on the game takes it, while a game is being played.
    let mut with = game(true).unwrap();
    ready(&mut with);
    assert!(with.runner.key(Key::Char(' ')));
    assert!(!with.runner.key(Key::Char('x')));
}

#[test]
fn the_arcade_game_has_it_too() {
    let mods = [Mod::TimingIndicator, Mod::BulletTime];
    let Some(mut script) = game_modded("arcade", 1, &mods) else {
        return;
    };
    assert_eq!(meter(&ready(&mut script)), 120);
    let (_, slowed, after) = play(&mut script, LEAVE, true);
    assert!(slowed);
    assert_eq!(meter(&after), 0);
}
