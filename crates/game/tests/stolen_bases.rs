//! The stolen bases mod: a runner sent on while the pitcher winds up, and
//! the catcher's throw to put him out.

mod common;

use bb_game::art::all_named;
use bb_game::mods::Mod;
use bb_game::rules::Rules;
use bb_game::script::Script;
use bb_game::settings::Ground;
use common::{full_match, long_match_ruled, next, number, pitch, said, state, written};

/// A swing this many steps before the best misses the ball, and one this
/// many after never comes: the pitch is let go by.
const MISS: i32 = -6;
const LEAVE: i32 = 1000;
/// A click on the little field in the corner of the batting view.
const LITTLE_FIELD: &str = "click 60 45";
/// One ball walks the batter, so that there is soon a runner on first, and
/// the catcher always takes as long over his throw.
const QUICK: &str = "[count]\nballs = 1\n[steal]\npop = { low = 30, high = 30 }\n";

fn game(steals: bool) -> Option<Script> {
    let mut mods = vec![Mod::TimingIndicator];
    if steals {
        mods.push(Mod::StolenBases);
    }
    long_match_ruled(1, &mods, QUICK)
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

/// Lets pitches go by until a batter has walked to first with nobody else
/// on, and the pitcher stands ready for the next.
fn runner_on_first(script: &mut Script) -> String {
    for _ in 0..60 {
        let now = ready(script);
        if now.contains("bases x--") {
            return now;
        }
        pitch(script, LEAVE, (0.0, 0.0));
        next(script);
    }
    panic!("nobody walked: {}", state(script));
}

/// Plays the pitch that is coming. The little field is clicked on the
/// `send`th frame of the wind-up, if one is given, and the bat is swung
/// `late` steps after the best step for it. Returns how things stand when
/// the play is over.
fn play(script: &mut Script, send: Option<u32>, late: i32) -> String {
    let mut wound = 0;
    for _ in 0..20_000 {
        let now = state(script);
        if !now.starts_with("Match,") && !now.starts_with("FullMatch,") || now.contains(": Ready") {
            return now;
        }
        let ring =
            number(&now, "crossing ").zip(number(now.split("crossing ").nth(1).unwrap_or(""), ","));
        let step = number(&now, "Flight { step: ").map(|step| step as i32);
        let best = number(&now, "best swung on steps ").map(|best| best as i32);
        wound += u32::from(now.contains("WindUp"));
        let steps = match (ring, step, best) {
            (Some((x, y)), _, _) if now.contains("WindUp") && send == Some(wound) => {
                // And back to the ball.
                format!("{LITTLE_FIELD}; move {x} {y}")
            }
            (Some((x, y)), Some(step), Some(best)) if step == best + late => {
                format!("click {x} {y}")
            }
            (Some((x, y)), None, _) if now.contains("Settling") => {
                format!("move {x} {y}; wait 1")
            }
            _ => "wait 1".to_owned(),
        };
        script.run(&steps).unwrap();
    }
    panic!("the pitch never ended: {}", state(script));
}

/// How many runners are marked on the little field.
fn marked(script: &Script) -> usize {
    all_named(&script.runner.stage, &[], "lead").len()
}

#[test]
fn a_runner_sent_early_steals_the_base_and_the_count_goes_on() {
    let Some(mut script) = game(true) else {
        return;
    };
    let before = runner_on_first(&mut script);
    let outs = number(&before, "outs ").unwrap();
    let after = play(&mut script, Some(10), MISS);
    // He is on second, nobody is out, and the swing that missed is a
    // strike on the batter.
    assert!(after.contains("bases -x-"), "{after}");
    assert!(after.contains(", stolen 1, caught 0"), "{after}");
    assert!(
        after.contains(&format!("outs {outs}, count 0-1")),
        "{after}"
    );
    assert_eq!(said(&script, "stealNews"), ["STOLEN BASE!"]);
    // From second he can be sent for third.
    next(&mut script);
    ready(&mut script);
    let third = play(&mut script, Some(5), LEAVE);
    assert!(third.contains(", stolen 2, caught 0"), "{third}");
    // And from third there is nowhere to steal to.
    next(&mut script);
    if ready(&mut script).contains("bases --x") {
        let home = play(&mut script, Some(5), MISS);
        assert!(home.contains(", stolen 2, caught 0"), "{home}");
    }
}

#[test]
fn a_runner_sent_late_is_thrown_out_and_the_batter_bats_on() {
    let Some(mut script) = game(true) else {
        return;
    };
    let before = runner_on_first(&mut script);
    let outs = number(&before, "outs ").unwrap();
    let after = play(&mut script, Some(65), MISS);
    assert!(after.contains("bases ---"), "{after}");
    assert!(after.contains(", stolen 0, caught 1"), "{after}");
    assert!(
        after.contains(&format!("outs {}, count 0-1", outs + 1.0)),
        "{after}"
    );
    assert_eq!(said(&script, "stealNews"), ["CAUGHT STEALING!"]);
}

#[test]
fn the_little_field_marks_the_runner_and_asks_for_the_click_in_the_wind_up() {
    let Some(mut script) = game(true) else {
        return;
    };
    ready(&mut script);
    // Nobody on, nothing marked.
    assert_eq!(marked(&script), 0);
    runner_on_first(&mut script);
    assert_eq!(marked(&script), 1);
    assert!(said(&script, "steal").is_empty());
    // A click before the wind-up sends nobody.
    script.run(&format!("{LITTLE_FIELD}; wait 2")).unwrap();
    assert!(!state(&mut script).contains("stealing"));
    for _ in 0..600 {
        if state(&mut script).contains("WindUp") {
            break;
        }
        script.run("wait 1").unwrap();
    }
    script.run("wait 1").unwrap();
    assert_eq!(said(&script, "steal"), ["CLICK TO STEAL"]);
    // Nor does a click anywhere else.
    script.run("click 300 300; wait 1").unwrap();
    assert!(!state(&mut script).contains("stealing"));
    script.run(&format!("{LITTLE_FIELD}; wait 1")).unwrap();
    assert!(state(&mut script).contains(", stealing 2"));
    assert_eq!(said(&script, "steal"), ["RUNNER GOING"]);
}

#[test]
fn a_pitch_that_is_hit_is_no_steal() {
    let Some(mut script) = game(true) else {
        return;
    };
    runner_on_first(&mut script);
    let after = play(&mut script, Some(10), 0);
    assert!(!after.contains("stolen"), "{after}");
    assert!(!after.contains("stealing"), "{after}");
    assert!(said(&script, "stealNews").is_empty());
}

#[test]
fn without_the_mod_nobody_is_marked_and_nobody_goes() {
    let Some(mut script) = game(false) else {
        return;
    };
    runner_on_first(&mut script);
    assert_eq!(marked(&script), 0);
    let after = play(&mut script, Some(10), MISS);
    assert!(after.contains("bases x--"), "{after}");
    assert!(!after.contains("stolen"), "{after}");
    assert!(said(&script, "steal").is_empty());
}

/// Turns the pages of a finished match until one has this heading.
fn turn_to(script: &mut Script, heading: &str) {
    for _ in 0..60 {
        if said(script, "pageHeading") == [heading] {
            return;
        }
        script.run("click 362 323; wait 2").unwrap();
    }
    panic!("there is no page headed {heading}");
}

#[test]
fn a_full_match_writes_both_sides_steals_in_its_book() {
    // One innings, in which the other side makes two runs and its runners
    // go every chance they get.
    let text = format!(
        "{QUICK}their_chance = 1.0\n[full_match]\ninnings = 1\n[full_match.runs]\n\
         easy = [0, 0, 1]\nmedium = [0, 0, 1]\nhard = [0, 0, 1]\n"
    );
    let rules = Rules::layered(&[("a short match", &text)]).unwrap();
    let mods = [Mod::TimingIndicator, Mod::StolenBases];
    let Some(mut script) = full_match(3, Ground::Away, &mods, Some(rules)) else {
        return;
    };
    runner_on_first(&mut script);
    let after = play(&mut script, Some(10), MISS);
    assert!(after.contains(", stolen 1, caught 0"), "{after}");
    // The rest of the match, with nobody swinging.
    for _ in 0..4000 {
        let now = state(&mut script);
        if now.starts_with("Interval,") {
            script.run("wait 90; click 542 357; wait 30").unwrap();
        } else if !now.starts_with("FullMatch,") {
            break;
        } else if now.contains(": Ready") {
            next(&mut script);
        } else {
            script.run("wait 5").unwrap();
        }
    }
    script.run("wait 340").unwrap();
    assert!(state(&mut script).contains(", page 1 of "));
    turn_to(&mut script, "THE FIGURES");
    let names = written(&script, "figuresName");
    let row = names
        .iter()
        .position(|name| name == "BASES STOLEN")
        .expect("a line for the bases stolen");
    assert_eq!(written(&script, "figuresOurs")[row], "1 OF 1");
    // So many of so many, for them.
    let theirs = written(&script, "figuresTheirs")[row].clone();
    let (safe, tries) = theirs.split_once(" OF ").expect("so many of so many");
    let (safe, tries): (usize, usize) = (safe.parse().unwrap(), tries.parse().unwrap());
    assert!(tries >= 1 && safe <= tries, "{theirs}");
    // Each is told in its innings, among the turns.
    turn_to(&mut script, "THE 1ST INNINGS");
    let lines = written(&script, "turnLine");
    let told = |what: &str| lines.iter().filter(|line| line.contains(what)).count();
    assert_eq!(told(" STOLE SECOND") + told(" STOLE THIRD"), 1 + safe);
    assert_eq!(told(" CAUGHT STEALING "), tries - safe);
}
