//! The full match: every innings, with the other side's made up.

mod common;

use bb_game::art;
use bb_game::mods::Mod;
use bb_game::script::Script;
use bb_game::settings::Ground;
use common::{full_match, game, match_to_order, next, pitch, said, state, state_after, text};

/// The row of the menu's first page that leads to the full match, and the
/// buttons on the pages after it.
const FULL_MATCH: &str = "click 200 216";
const AWAY: &str = "click 445 312";
const NEXT: &str = "click 490 362";
const BACK: &str = "click 290 362";
const PLAY_BALL: &str = "click 480 362";
/// The button on the board between innings.
const NEXT_INNINGS: &str = "click 542 357";

/// Lets every pitch go by until the side is out or the match is over, and
/// returns how things stand then.
fn sit_out(script: &mut Script) -> String {
    // Far more frames than an innings needs: one that never ends fails
    // here instead of hanging the test.
    for _ in 0..20_000 {
        let now = state(script);
        if !now.starts_with("FullMatch,") && !now.starts_with("Loading") {
            return now;
        }
        if now.contains(": Ready") {
            next(script);
        } else {
            script.run("wait 5").unwrap();
        }
    }
    panic!("the side was never out: {}", state(script));
}

#[test]
fn away_the_side_bats_first_and_the_board_follows_its_innings() {
    let Some(mut script) = full_match(1, Ground::Away, &[], Some(match_to_order(9, 2))) else {
        return;
    };
    let begun = state_after(&mut script, "wait 60; state");
    assert!(begun.starts_with("FullMatch,"), "{begun}");
    assert!(begun.contains("top of innings 1, score 0 to 0"), "{begun}");
    assert!(begun.ends_with("away, visitors -, home side -"), "{begun}");
    // The corner of the view says which half it is, and the scoreboard
    // shows the other side's score under a word of its own.
    assert_eq!(said(&script, "innings"), ["TOP 1ST"]);
    assert_eq!(said(&script, "themLabel"), ["THEM"]);
    assert_eq!(text(&script, "scoreTarget"), "0");

    let out = sit_out(&mut script);
    assert!(out.starts_with("Interval,"), "{out}");
    assert!(out.contains("top of innings 2, score 0 to 2"), "{out}");
    assert!(out.ends_with("away, visitors 0, home side 2"), "{out}");
    // The board arrives, with the full match's words on it and none of the
    // art's own.
    script.run("wait 90").unwrap();
    assert_eq!(said(&script, "boardHeading"), ["END OF THE 1ST"]);
    let lines = said(&script, "boardLine");
    assert!(
        lines[0].starts_with("THE HOME SIDE MADE 2 RUNS ON "),
        "{lines:?}"
    );
    assert_eq!(
        lines[1..],
        ["YOU TRAIL 0 - 2", "YOU BAT IN THE TOP OF THE 2ND"]
    );
    assert_eq!(said(&script, "boardSide"), ["YOU", "THEM"]);
    assert_eq!(said(&script, "boardRuns"), ["0", "2"]);
    let stage = &script.runner.stage;
    let board = art::in_shell(stage, art::BOARD).expect("the board");
    let theirs = stage
        .clip(&board)
        .unwrap()
        .children
        .values()
        .filter(|child| art::BOARD_WORDS.contains(&child.symbol))
        .collect::<Vec<_>>();
    assert!(!theirs.is_empty());
    assert!(theirs.iter().all(|child| !child.visible));

    // The button on it brings the side in again, with nobody out.
    let again = state_after(&mut script, &format!("{NEXT_INNINGS}; wait 60; state"));
    assert!(again.starts_with("FullMatch,"), "{again}");
    assert!(
        again.contains("top of innings 2, score 0 to 2, outs 0, count 0-0"),
        "{again}"
    );
    assert_eq!(said(&script, "innings"), ["TOP 2ND"]);
    assert_eq!(text(&script, "scoreTarget"), "2");
}

#[test]
fn at_home_the_board_comes_before_the_first_ball() {
    let Some(mut script) = full_match(1, Ground::Home, &[], Some(match_to_order(9, 1))) else {
        return;
    };
    let begun = state_after(&mut script, "wait 90; state");
    assert!(begun.starts_with("Interval,"), "{begun}");
    assert!(
        begun.ends_with("at home, visitors 1, home side -"),
        "{begun}"
    );
    assert_eq!(said(&script, "boardHeading"), ["TOP OF THE 1ST"]);
    assert_eq!(said(&script, "boardSide"), ["THEM", "YOU"]);
    let batting = state_after(&mut script, &format!("{NEXT_INNINGS}; wait 60; state"));
    assert!(batting.starts_with("FullMatch,"), "{batting}");
    assert!(
        batting.contains("bottom of innings 1, score 0 to 1, outs 0"),
        "{batting}"
    );
    assert_eq!(said(&script, "innings"), ["BOT 1ST"]);
}

#[test]
fn a_match_lost_says_what_each_side_made() {
    let Some(mut script) = full_match(1, Ground::Away, &[], Some(match_to_order(2, 1))) else {
        return;
    };
    assert!(sit_out(&mut script).starts_with("Interval,"));
    script
        .run(&format!("wait 90; {NEXT_INNINGS}; wait 30"))
        .unwrap();
    // One behind with the home side's last half still to come, which they
    // have no need of.
    let lost = sit_out(&mut script);
    assert_eq!(
        lost,
        "MatchLost, Medium: YOU LOST 0 - 1, away, visitors 0 0, home side 1 x"
    );
    // The board the match ends on has the outs of both innings, and once
    // it has got to its figures, every innings.
    script.run("wait 330").unwrap();
    assert_eq!(text(&script, "out"), "6");
    assert_eq!(text(&script, "score"), "0");
    assert_eq!(said(&script, "boardVerdict"), ["YOU LOST 0 - 1"]);
    assert_eq!(said(&script, "boardInnings"), ["1", "2", "R", "H", "E"]);
    // The visitors' two noughts read as one: what is said twice running
    // is given once.
    assert_eq!(said(&script, "boardCell"), ["0", "1", "X"]);
    assert_eq!(said(&script, "boardRuns"), ["0", "1"]);
    // And from there it is back to the menu.
    let menu = state_after(&mut script, "click 545 355; wait 30; state");
    assert!(menu.starts_with("Menu,"), "{menu}");
    assert!(said(&script, "boardVerdict").is_empty());
}

#[test]
fn at_home_going_ahead_in_the_last_innings_wins_there_and_then() {
    // With every hit a home run, the other side's one run an innings is
    // worth three.
    let mods = [Mod::TimingIndicator, Mod::ZingerHit];
    let Some(mut script) = full_match(1, Ground::Home, &mods, Some(match_to_order(1, 1))) else {
        return;
    };
    script.run("wait 90").unwrap();
    assert_eq!(
        said(&script, "boardLine").last().unwrap(),
        "4 RUNS WILL WIN THE MATCH"
    );
    script.run(&format!("{NEXT_INNINGS}; wait 30")).unwrap();
    let mut pitches = 0;
    let won = loop {
        let now = pitch(&mut script, 0, (0.0, 0.0));
        if !now.starts_with("FullMatch,") {
            break now;
        }
        pitches += 1;
        assert!(pitches < 30, "the match was never won: {now}");
        next(&mut script);
    };
    assert_eq!(
        won,
        "MatchWon, Medium: YOU WON 4 - 3, at home, visitors 3, home side 4"
    );
}

#[test]
fn the_batting_order_comes_round_again() {
    let Some(mut script) = full_match(1, Ground::Away, &[], Some(match_to_order(9, 0))) else {
        return;
    };
    // Nobody swings and nobody walks: three up and three down, three
    // times.
    script.run("wait 60").unwrap();
    assert_eq!(text(&script, "batsmanOnStrike"), "1");
    for first_up in ["4", "7", "1"] {
        let out = sit_out(&mut script);
        assert!(out.starts_with("Interval,"), "{out}");
        script
            .run(&format!("wait 90; {NEXT_INNINGS}; wait 60"))
            .unwrap();
        assert_eq!(
            text(&script, "batsmanOnStrike"),
            first_up,
            "{}",
            state(&mut script)
        );
    }
}

#[test]
fn mods_are_played_by_in_a_full_match() {
    // With sudden death on, the one strike is the batter out.
    let Some(mut script) = full_match(1, Ground::Away, &[Mod::SuddenDeath], None) else {
        return;
    };
    let mut now = String::new();
    for _ in 0..200 {
        now = state_after(&mut script, "wait 5; state");
        if now.contains(": Ready") {
            break;
        }
    }
    assert!(now.contains("outs 1, count 0-0"), "{now}");
}

#[test]
fn the_menu_leads_to_a_full_match_at_the_ground_chosen() {
    let Some(mut script) = game("menu") else {
        return;
    };
    let setup = state_after(
        &mut script,
        &format!("wait 60; {FULL_MATCH}; wait 90; state"),
    );
    assert_eq!(setup, "Menu, FullSetup, Medium");
    assert_eq!(said(&script, "groundWord"), ["Home", "Away", "Toss"]);
    let summary = state_after(
        &mut script,
        &format!("{AWAY}; wait 5; {NEXT}; wait 90; state"),
    );
    assert_eq!(summary, "Menu, FullSummary, Medium");
    let lines = said(&script, "fullLine");
    assert_eq!(lines[0], "IT'S A FULL MATCH OF 9 INNINGS!");
    assert_eq!(
        lines[1..3],
        ["As you asked,", "you are away and bat first."]
    );
    // The art's own words about the last innings are out of sight.
    let stage = &script.runner.stage;
    let menu = art::in_shell(stage, art::MENU).expect("the menu");
    let theirs = stage
        .clip(&menu)
        .unwrap()
        .children
        .values()
        .filter(|child| art::SUMMARY_WORDS.contains(&child.symbol))
        .collect::<Vec<_>>();
    assert!(!theirs.is_empty());
    assert!(theirs.iter().all(|child| !child.visible));
    let playing = state_after(&mut script, &format!("{PLAY_BALL}; wait 120; state"));
    assert!(playing.starts_with("FullMatch,"), "{playing}");
    assert!(playing.contains("top of innings 1"), "{playing}");
}

#[test]
fn left_to_the_toss_the_match_is_played_where_the_summary_says() {
    let mut seen = [false, false];
    for seed in 1..=8 {
        let Some(mut script) = common::game_with("menu", Some(seed)) else {
            return;
        };
        let steps = format!("wait 60; {FULL_MATCH}; wait 90; {NEXT}; wait 90; state");
        assert_eq!(
            state_after(&mut script, &steps),
            "Menu, FullSummary, Medium"
        );
        let lines = said(&script, "fullLine");
        assert_eq!(lines[1], "The coin is tossed, and");
        let home = lines[2] == "you are at home and bat second.";
        assert!(
            home || lines[2] == "you are away and bat first.",
            "{lines:?}"
        );
        seen[usize::from(home)] = true;
        let playing = state_after(&mut script, &format!("{PLAY_BALL}; wait 120; state"));
        let (screen, ground) = if home {
            ("Interval,", "at home")
        } else {
            ("FullMatch,", "away")
        };
        assert!(playing.starts_with(screen), "{playing}");
        assert!(playing.contains(ground), "{playing}");
    }
    // A coin that only ever came down one way would be no coin.
    assert_eq!(seen, [true, true]);
}

#[test]
fn the_choice_of_ground_is_only_on_the_full_matchs_setup() {
    let Some(mut script) = game("menu") else {
        return;
    };
    script
        .run(&format!("wait 60; {FULL_MATCH}; wait 90"))
        .unwrap();
    assert!(!said(&script, "groundWord").is_empty());
    let main = state_after(&mut script, &format!("{BACK}; wait 90; state"));
    assert_eq!(main, "Menu, Main, Medium");
    assert!(said(&script, "groundWord").is_empty());
    // The last innings on its own has nothing to choose.
    let setup = state_after(&mut script, "click 200 181; wait 90; state");
    assert_eq!(setup, "Menu, MatchSetup, Medium");
    assert!(said(&script, "groundWord").is_empty());
}
