//! The fixtures of a tournament, played: each a full match against the
//! side the menu named, whose result goes into the tables.

use bb_game::mods::Mod;
use bb_game::rules::Rules;
use bb_game::script::Script;
use bb_game::tournament::Format;

use super::CARRY_ON;
use super::common::{
    NEXT_INNINGS, full_match_played, next, number, pitch, said, short_tournament, state,
    state_after, written,
};

/// Where things are on the way into a fixture and out of it, in stage
/// pixels: the menu's button, the game's own and its prompt's, and the
/// badge on the screen a match ends on.
const PLAY_BALL: &str = "click 480 362";
const QUIT: &str = "click 30 386";
const YES: &str = "click 355 232";
const ON_FROM_THE_RESULT: &str = "click 545 355";

/// From the tables, on to what the menu says is next. Returns how things
/// stand there.
pub(super) fn to_the_summary(script: &mut Script) -> String {
    state_after(script, &format!("wait 120; {CARRY_ON}; wait 70; state"))
}

/// From the menu's summary into the fixture, as far as the pitcher
/// standing ready for its first pitch. Returns how things stand then.
pub(super) fn to_the_first_pitch(script: &mut Script) -> String {
    script.run(PLAY_BALL).unwrap();
    for _ in 0..400 {
        let now = state(script);
        if now.starts_with("FullMatch,") && now.contains("Settling") {
            return now;
        }
        // At home the other side has batted already, and the board says
        // how.
        let step = if now.starts_with("Interval,") {
            format!("wait 90; {NEXT_INNINGS}; wait 30")
        } else {
            "wait 5".to_owned()
        };
        script.run(&step).unwrap();
    }
    panic!("the fixture never began: {}", state(script));
}

/// On from the screen a fixture ended on to the tables. Returns how
/// things stand there.
pub(super) fn on_to_the_tables(script: &mut Script) -> String {
    state_after(script, &format!("{ON_FROM_THE_RESULT}; wait 130; state"))
}

/// The short name of the side with this name.
fn short_of(name: &str) -> String {
    let rules = Rules::default().tournament;
    let side = rules.sides.values().find(|side| side.name == name);
    side.unwrap_or_else(|| panic!("no side is called {name}"))
        .short
        .clone()
}

/// Plays a fixture to its end as a batter who hits everything until the
/// side has eight runs, and then lets every pitch go by. The timing bar
/// has to be up, and every hit a home run.
fn won(script: &mut Script) -> String {
    for _ in 0..4000 {
        let now = state(script);
        let batting = now.starts_with("FullMatch,") || now.starts_with("Loading");
        if now.starts_with("Interval,") {
            script
                .run(&format!("wait 90; {NEXT_INNINGS}; wait 30"))
                .unwrap();
        } else if !batting {
            script.run("wait 340").unwrap();
            return state(script);
        } else if number(&now, "score ").is_some_and(|score| score < 8.0) {
            if pitch(script, 0, (0.0, 0.0)).contains(": Ready") {
                next(script);
            }
        } else if now.contains(": Ready") {
            next(script);
        } else {
            script.run("wait 5").unwrap();
        }
    }
    panic!("the fixture never ended: {}", state(script));
}

#[test]
fn a_fixture_is_a_full_match_against_the_side_the_menu_named() {
    let Some(mut script) = short_tournament(3, Format::League, &[]) else {
        return;
    };
    let summary = to_the_summary(&mut script);
    let begins =
        "Menu, TournamentSummary, Medium: league of 6, 1 innings, played 0 of 15, ROUND 1, ";
    assert!(summary.starts_with(begins), "{summary}");
    let lines = written(&script, "tournamentLine");
    let against = lines[1]
        .strip_prefix("You play ")
        .and_then(|rest| rest.strip_suffix(','))
        .expect("who the player meets");
    let short = short_of(against);
    // The match is at home or away as the menu said, and the scoreboard
    // has the other side by its short name where it had THEM.
    let begun = to_the_first_pitch(&mut script);
    assert_eq!(
        begun.contains(", at home, "),
        summary.contains("next at home "),
        "{begun}"
    );
    assert_eq!(said(&script, "themLabel"), std::slice::from_ref(&short));
    // With nobody swinging it is lost by the one run they make.
    let over = full_match_played(&mut script, false);
    assert!(
        over.starts_with("MatchLost, Medium: YOU LOST 0 - 1, "),
        "{over}"
    );
    let mut sides = written(&script, "boardSide");
    sides.sort();
    let mut both = vec![short, "YOU".to_owned()];
    both.sort();
    assert_eq!(sides, both);
    // On from there are the tables, with the match in them and the rest
    // of its round played.
    let tables = on_to_the_tables(&mut script);
    let begins = "Tournament, Medium: league of 6, 1 innings, played 3 of 15, ROUND 2, next ";
    assert!(tables.starts_with(begins), "{tables}");
    assert!(tables.ends_with("TABLE, page 1 of 1"), "{tables}");
    let ours = written(&script, "tableOurs");
    assert_eq!(ours[1..5], ["YOU", "1", "0", "1"]);
    let played: Vec<String> = written(&script, "tableCell")
        .chunks(8)
        .map(|row| row[2].clone())
        .collect();
    assert_eq!(played, ["1"; 5]);
}

/// Plays a tournament through with nobody swinging, so that every fixture
/// is lost. Returns how many fixtures the player had, and what the menu
/// says when it is over.
fn lost_throughout(format: Format) -> Option<(usize, String, Vec<String>)> {
    let mut script = short_tournament(5, format, &[])?;
    for fixtures in 0..=5 {
        let summary = to_the_summary(&mut script);
        if summary.contains(", over, ") {
            return Some((fixtures, summary, written(&script, "tournamentLine")));
        }
        to_the_first_pitch(&mut script);
        let over = full_match_played(&mut script, false);
        assert!(over.starts_with("MatchLost,"), "{over}");
        on_to_the_tables(&mut script);
    }
    panic!("the tournament never ended: {}", state(&mut script));
}

#[test]
fn a_league_lost_throughout_is_five_matches_and_the_bottom_of_the_table() {
    let Some((fixtures, summary, lines)) = lost_throughout(Format::League) else {
        return;
    };
    assert_eq!(fixtures, 5);
    assert!(
        summary.contains("played 15 of 15, over, won by "),
        "{summary}"
    );
    assert!(summary.ends_with(", you were placed 6"), "{summary}");
    assert_eq!(lines[0], "THE LEAGUE IS OVER!");
    assert_eq!(lines[2], "You were placed 6th.");
}

#[test]
fn a_cup_lost_at_once_is_played_out_to_its_winner_by_the_others() {
    let Some((fixtures, summary, lines)) = lost_throughout(Format::Cup) else {
        return;
    };
    assert_eq!(fixtures, 1);
    assert!(
        summary.contains("played 7 of 7, over, won by "),
        "{summary}"
    );
    assert!(
        summary.ends_with(", you went out in THE QUARTER-FINALS"),
        "{summary}"
    );
    assert_eq!(lines[2], "You went out in the quarter-finals.");
}

#[test]
fn a_side_that_loses_all_three_of_its_group_does_not_go_on() {
    let Some((fixtures, summary, lines)) = lost_throughout(Format::Groups) else {
        return;
    };
    assert_eq!(fixtures, 3);
    assert!(
        summary.contains("played 15 of 15, over, won by "),
        "{summary}"
    );
    assert!(summary.ends_with(", you were placed 4"), "{summary}");
    assert_eq!(lines[2], "You were placed 4th.");
}

#[test]
fn a_cup_is_won_by_winning_its_three_ties() {
    let mods = [Mod::TimingIndicator, Mod::ZingerHit];
    let Some(mut script) = short_tournament(2, Format::Cup, &mods) else {
        return;
    };
    let rounds = ["THE QUARTER-FINALS", "THE SEMI-FINALS", "THE FINAL"];
    for (played, round) in rounds.into_iter().enumerate() {
        let summary = to_the_summary(&mut script);
        assert!(summary.contains(&format!(", {round}, next ")), "{summary}");
        let lines = written(&script, "tournamentLine");
        assert_eq!(lines[0], format!("{round} OF THE CUP!"));
        if played > 0 {
            assert_eq!(lines[3], format!("You have won {played} and lost 0,"));
            assert_eq!(lines[4], "and are still in it.");
        }
        to_the_first_pitch(&mut script);
        let over = won(&mut script);
        assert!(over.starts_with("MatchWon,"), "{over}");
        on_to_the_tables(&mut script);
    }
    let summary = to_the_summary(&mut script);
    assert!(
        summary.ends_with("played 7 of 7, over, won by YOU, you won it"),
        "{summary}"
    );
    assert_eq!(written(&script, "tournamentLine")[1], "You have won it!");
}

#[test]
fn a_fixture_given_up_half_way_is_still_to_play_and_is_another_game_then() {
    let Some(mut script) = short_tournament(3, Format::League, &[]) else {
        return;
    };
    let before = to_the_summary(&mut script);
    let pitch_of = |state: &str| state.split("crossing ").nth(1).map(str::to_owned);
    let first = pitch_of(&to_the_first_pitch(&mut script)).expect("a pitch");
    let after = state_after(
        &mut script,
        &format!("{QUIT}; wait 40; {YES}; wait 70; state"),
    );
    // Nothing has been played, and it is the same side still to meet.
    assert_eq!(after, before);
    assert_eq!(
        written(&script, "tournamentLine")[3],
        "It is your first match."
    );
    let again = pitch_of(&to_the_first_pitch(&mut script)).expect("a pitch");
    assert_ne!(again, first, "the same game over again");
}
