//! What a full match's book says of the bases stolen in it.

use bb_game::mods::Mod;
use bb_game::rules::Rules;
use bb_game::script::Script;
use bb_game::settings::Ground;

use super::{QUICK, play, runner_on_first};
use crate::common::{MISS, full_match, next, said, state, written};

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
