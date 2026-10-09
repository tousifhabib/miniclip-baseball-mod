//! What a zinger does to the game it is hit in: the match, the arcade
//! game, and the longest that is kept.

use bb_game::art::all_named;
use bb_game::scores::Scores;

use super::{SPOILT, feet, game, mods};
use crate::common::{ON, game_keeping, next, pitch, playing, said, state};

#[test]
fn a_swing_that_misses_is_still_a_strike() {
    let Some(mut script) = game("match", 1, true) else {
        return;
    };
    let after = pitch(&mut script, -6, ON);
    assert!(after.contains("score 0 of"), "{after}");
    assert!(after.contains("count 0-1"), "{after}");
    assert_eq!(feet(&after), None);
    assert!(said(&script, "zingerFeet").is_empty());
}

#[test]
fn there_are_more_runs_to_get_and_a_hit_for_each_wins_the_match() {
    let Some(mut usual) = game("match", 1, false) else {
        return;
    };
    usual.run("wait 60").unwrap();
    assert!(state(&mut usual).contains("score 0 of 3"));

    let mut script = game("match", 1, true).unwrap();
    let mut pitches = 0;
    let end = loop {
        let (_, off) = SPOILT[pitches % SPOILT.len()];
        let now = pitch(&mut script, 0, off);
        if !playing(&now) {
            break now;
        }
        pitches += 1;
        assert!(pitches <= 6, "{now}");
        assert!(now.contains(&format!("score {pitches} of 6")), "{now}");
        next(&mut script);
    };
    assert!(end.starts_with("MatchWon"), "{end}");
    assert_eq!(pitches, 6);
}

#[test]
fn without_the_mod_a_poorly_timed_swing_is_no_home_run() {
    let Some(mut script) = game("match", 1, false) else {
        return;
    };
    let after = pitch(&mut script, 3, ON);
    assert!(after.contains("score 0 of"), "{after}");
    assert_eq!(feet(&after), None);
    assert!(said(&script, "zingerFeet").is_empty());
}

#[test]
fn in_the_arcade_game_a_zinger_scores_the_feet_it_goes() {
    let Some(mut script) = game("arcade", 1, true) else {
        return;
    };
    script.run("wait 60").unwrap();
    // There is no target to drop the ball on.
    let stage = &script.runner.stage;
    let targets = all_named(stage, &[], "landMarker");
    assert!(!targets.is_empty());
    assert!(
        targets
            .iter()
            .all(|target| !stage.child(target).unwrap().visible)
    );

    let mut total = 0;
    for (late, off) in [(0, ON), (3, ON), (1, (0.0, 50.0))] {
        let after = pitch(&mut script, late, off);
        let went = feet(&after).unwrap();
        // The next pitch is on offer while the ball is still in the air,
        // and nothing is scored until it comes down.
        assert!(after.contains(&format!("{total} points")), "{after}");
        script.run("wait 700").unwrap();
        total += went;
        let landed = state(&mut script);
        assert!(landed.contains(&format!("{total} points")), "{landed}");
        assert_eq!(said(&script, "zingerFeet"), [format!("{went} FT")]);
        next(&mut script);
    }

    // A zinger left in the air for the next pitch still scores.
    let after = pitch(&mut script, 0, ON);
    total += feet(&after).unwrap();
    next(&mut script);
    script.run("wait 30").unwrap();
    let gone = state(&mut script);
    assert!(gone.contains(&format!("{total} points")), "{gone}");

    // Without the mod the target is there.
    let usual = game("arcade", 1, false).unwrap();
    let stage = &usual.runner.stage;
    assert!(
        all_named(stage, &[], "landMarker")
            .iter()
            .all(|target| stage.child(target).unwrap().visible)
    );
}

#[test]
fn the_longest_zinger_is_kept_and_told_on_the_result_screen() {
    let file = std::env::temp_dir().join(format!("bb-zinger-{}.toml", std::process::id()));
    let _ = std::fs::remove_file(&file);
    let Some(mut script) = game_keeping("match", 1, &mods(true), &file) else {
        return;
    };
    // The first there has ever been is a record, and is kept at once.
    let first = feet(&pitch(&mut script, 3, ON)).unwrap();
    assert_eq!(said(&script, "zingerRecord"), ["NEW RECORD"]);
    assert_eq!(Scores::load(&file).longest_zinger, first);
    // A longer one beats it, and one no longer does not.
    next(&mut script);
    let second = feet(&pitch(&mut script, 0, ON)).unwrap();
    assert!(second > first);
    assert_eq!(said(&script, "zingerRecord"), ["NEW RECORD"]);
    next(&mut script);
    pitch(&mut script, 0, ON);
    assert!(said(&script, "zingerRecord").is_empty());
    assert_eq!(Scores::load(&file).longest_zinger, second);

    // The match is played out, and its result screen tells of both.
    let end = loop {
        next(&mut script);
        let now = pitch(&mut script, 2, ON);
        if !playing(&now) {
            break now;
        }
    };
    assert!(end.starts_with("MatchWon"), "{end}");
    assert!(said(&script, "zingerLine").is_empty());
    script.run("wait 300").unwrap();
    let line = format!("LONGEST ZINGER {second} FT   BEST EVER {second} FT");
    assert_eq!(said(&script, "zingerLine"), [line]);

    // Another game starts from the record that was kept, and its own
    // longest may be shorter.
    let mut script = game_keeping("arcade", 1, &mods(true), &file).unwrap();
    let short = feet(&pitch(&mut script, 3, ON)).unwrap();
    script.run("wait 700").unwrap();
    assert!(said(&script, "zingerRecord").is_empty());
    for _ in 0..9 {
        next(&mut script);
        pitch(&mut script, -6, ON);
    }
    next(&mut script);
    script.run("wait 400").unwrap();
    assert!(state(&mut script).starts_with("ArcadeFinish"));
    let line = format!("LONGEST ZINGER {short} FT   BEST EVER {second} FT");
    assert_eq!(said(&script, "zingerLine"), [line]);
    std::fs::remove_file(&file).unwrap();

    // A game with no zinger in it has no such line.
    let mut script = game("matchWon", 1, true).unwrap();
    script.run("wait 400").unwrap();
    assert!(said(&script, "zingerLine").is_empty());
}
