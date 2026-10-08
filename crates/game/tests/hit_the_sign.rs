//! The hit the sign mod: signs on the outfield wall, one of them lit, that
//! pay runs to a ball that strikes them.

mod common;

use bb_game::art::all_named;
use bb_game::mods::Mod;
use bb_game::rules::Rules;
use bb_game::script::Script;
use bb_game::settings::Ground;
use common::{
    full_match, game_modded, long_match, long_match_ruled, next, number, pitch, pitch_seen, said,
    state, written,
};

/// The middle of the batting view, which a hit straight up the field goes
/// to.
const MIDDLE: f32 = 295.0;
/// A wall so near and so high that every ball that is hit comes to it and
/// none clears it, with five signs that between them cover all of it.
const NEAR: &str = "[field]\nwall = 300.0\nclear = 1000.0\n\
                    [sign]\nfirst = 0.1\nlast = 0.9\nwidth = 0.2\nhigh = 1000.0\n";
const LEAVE: i32 = 1000;

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

/// Hits the next pitch at the middle of a sign, counting from 1, with the
/// signs as [`NEAR`] has them. Returns how things stand when the play is
/// over, and what was said over the field of a sign being struck while it
/// went on.
fn hit_at(script: &mut Script, sign: u32) -> (String, Vec<String>) {
    let crosses = number(&ready(script), "crossing ").unwrap();
    // Where the sign is across the field, and so across the batting view.
    let across = 0.1 + 0.2 * (sign - 1) as f32;
    let aside = (-54.65 + across * 692.95 - 303.8) * 1.3;
    // A hit goes four pixels aside for each the ball is off the middle,
    // and three the other way for each the ring is off the ball.
    let off = (4.0 * (crosses - MIDDLE) - aside) / 3.0;
    let mut news = Vec::new();
    let after = pitch_seen(script, 0, (off, 0.0), |script, _| {
        for line in said(script, "signNews") {
            if !news.contains(&line) {
                news.push(line);
            }
        }
    });
    (after, news)
}

fn lit(now: &str) -> u32 {
    number(now, ", sign ").expect("a lit sign") as u32
}

#[test]
fn a_ball_that_strikes_a_sign_is_worth_runs_and_the_lit_sign_most() {
    let mods = [Mod::TimingIndicator, Mod::HitTheSign];
    let Some(mut script) = long_match_ruled(1, &mods, NEAR) else {
        return;
    };
    let before = ready(&mut script);
    let lit = lit(&before);
    assert!(!before.contains("struck"), "{before}");
    // The sign next to the lit one, which is not in the middle, where the
    // pitcher is.
    let other = if lit == 2 { 4 } else { 2 };
    let (after, news) = hit_at(&mut script, other);
    assert!(
        after.contains(&format!(", struck sign {other} for 1")),
        "{after}"
    );
    assert_eq!(news, ["OFF THE SIGN! +1"]);
    let score = number(&after, "score ").unwrap();
    assert!(score >= 1.0, "{after}");
    // And then the lit one.
    next(&mut script);
    let before = ready(&mut script);
    assert!(!before.contains("struck"), "{before}");
    let (after, news) = hit_at(&mut script, lit);
    assert!(
        after.contains(&format!(", struck sign {lit} for 3")),
        "{after}"
    );
    assert_eq!(news, ["OFF THE SIGN! +3"]);
    assert!(number(&after, "score ").unwrap() >= score + 3.0, "{after}");
}

#[test]
fn the_signs_are_on_the_wall_in_both_views_with_what_each_is_worth() {
    let Some(mut script) = long_match(1, &[Mod::TimingIndicator, Mod::HitTheSign]) else {
        return;
    };
    let lit = lit(&ready(&mut script)) as usize;
    assert_eq!(all_named(&script.runner.stage, &[], "sign").len(), 10);
    for name in ["signWordsSeen", "signWords"] {
        let worth = written(&script, name);
        assert_eq!(worth.len(), 5);
        for (sign, worth) in worth.iter().enumerate() {
            let should = if sign + 1 == lit { "+3" } else { "+1" };
            assert_eq!(worth, should, "sign {} of {name}", sign + 1);
        }
    }
    // A pitch that is let go by strikes nothing.
    let after = pitch(&mut script, LEAVE, (0.0, 0.0));
    assert!(!after.contains("struck"), "{after}");
    assert!(said(&script, "signNews").is_empty());
}

#[test]
fn a_full_match_lights_another_sign_each_innings() {
    // Three innings, in which the other side makes nothing.
    let text = "[full_match]\ninnings = 3\n[full_match.runs]\n\
                easy = [1]\nmedium = [1]\nhard = [1]\n";
    let rules = Rules::layered(&[("a short match", text)]).unwrap();
    let mods = [Mod::TimingIndicator, Mod::HitTheSign];
    let Some(mut script) = full_match(2, Ground::Away, &mods, Some(rules)) else {
        return;
    };
    let mut lit_in = Vec::new();
    for _ in 0..3 {
        let first = lit(&ready(&mut script));
        // It is the same sign for as long as the innings lasts: nine
        // pitches let go by are three strikeouts.
        for _ in 0..40 {
            let now = state(&mut script);
            if !now.starts_with("FullMatch,") {
                break;
            }
            if now.contains("Settling") {
                assert_eq!(lit(&now), first, "{now}");
            }
            if pitch(&mut script, LEAVE, (0.0, 0.0)).contains(": Ready") {
                next(&mut script);
                script.run("wait 40").unwrap();
            }
        }
        lit_in.push(first);
        if state(&mut script).starts_with("Interval,") {
            script.run("wait 90; click 542 357; wait 30").unwrap();
        }
    }
    assert!(
        lit_in[0] != lit_in[1] && lit_in[1] != lit_in[2],
        "{lit_in:?}"
    );
}

#[test]
fn without_the_mod_or_in_the_arcade_game_the_wall_is_bare() {
    for (screen, mods) in [
        ("match", vec![Mod::TimingIndicator]),
        ("arcade", vec![Mod::TimingIndicator, Mod::HitTheSign]),
    ] {
        let Some(mut script) = game_modded(screen, 1, &mods) else {
            return;
        };
        let now = ready(&mut script);
        assert!(!now.contains("sign"), "{screen}: {now}");
        assert!(all_named(&script.runner.stage, &[], "sign").is_empty());
        let after = pitch(&mut script, 0, (0.0, 0.0));
        assert!(!after.contains("struck"), "{screen}: {after}");
    }
}
