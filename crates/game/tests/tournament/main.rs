//! A tournament: its tables on the board, and the pages they are read
//! through.

mod fixtures;
mod menu;
mod sections;

#[path = "../common/mod.rs"]
mod common;

use bb_game::script::Script;
use bb_game::tournament::Format;
use common::{said, state, state_after, tournament, written};

/// The boxes that choose a section of the tables, the arrows that turn
/// its pages, the arrow beside the first fixture of a round, and the
/// board's own button.
const TABLE: &str = "click 47 63";
const MATCHES: &str = "click 151 63";
const PAGE_ON: &str = "click 362 323";
const PAGE_BACK: &str = "click 228 323";
const FIRST_MATCH: &str = "click 52 122";
const CARRY_ON: &str = "click 542 357";

/// A tournament on its tables, once the board has arrived and they are
/// up.
fn tables(format: Format, played: usize) -> Option<Script> {
    let mut script = tournament(7, format, 3, played)?;
    script.run("wait 120").unwrap();
    Some(script)
}

/// The sections the tables are in, by the words beside their boxes.
const SECTIONS: [&str; 5] = ["TABLE", "MATCHES", "SIDES", "LEADERS", "RECORDS"];

/// What comes after the tournament's own account of itself in the state:
/// which section of the tables is up, what is open in it, and which page.
fn page(script: &mut Script) -> String {
    let now = state(script);
    let from = SECTIONS
        .iter()
        .filter_map(|section| now.rfind(&format!(", {section}")))
        .max()
        .expect("a section of the tables");
    now[from + 2..].to_owned()
}

#[test]
fn the_tables_open_on_how_the_sides_of_a_league_stand() {
    let Some(mut script) = tables(Format::League, 9) else {
        return;
    };
    let now = state(&mut script);
    let begins = "Tournament, Medium: league of 6, 3 innings, played 9 of 15, ROUND 4, next ";
    assert!(now.starts_with(begins), "{now}");
    assert_eq!(page(&mut script), "TABLE, page 1 of 1");
    assert_eq!(said(&script, "pageHeading"), ["THE TABLE"]);
    // A row for each of the six, the player's own marked out, under the
    // heads of the columns.
    let heads = written(&script, "tableHead");
    assert_eq!(heads, ["", "", "P", "W", "L", "FOR", "AGST", "DIFF"]);
    let ours = written(&script, "tableOurs");
    assert_eq!(ours.len(), 8);
    assert_eq!(ours[1], "YOU");
    let others = written(&script, "tableCell");
    assert_eq!(others.len(), 5 * 8);
    // Nine matches have been won and nine lost, and everyone has played
    // three.
    let rows: Vec<&[String]> = others.chunks(8).chain([&ours[..]]).collect();
    let sum = |column: usize| -> u32 {
        rows.iter()
            .map(|row| row[column].parse::<u32>().unwrap())
            .sum()
    };
    assert_eq!((sum(2), sum(3), sum(4)), (18, 9, 9));
    assert_eq!(sum(5), sum(6));
    // The board's button says what it does here, and not what it does
    // between innings.
    assert_eq!(said(&script, "buttonWords"), ["CARRY", "ON"]);
}

#[test]
fn two_groups_have_a_table_each_and_the_knockout_rounds_after_them() {
    let Some(mut script) = tables(Format::Groups, 13) else {
        return;
    };
    assert_eq!(page(&mut script), "TABLE, page 1 of 2");
    assert_eq!(said(&script, "pageHeading"), ["THE GROUPS"]);
    assert_eq!(said(&script, "tableTitle"), ["GROUP A", "GROUP B"]);
    assert_eq!(written(&script, "tableHead").len(), 16);
    assert_eq!(
        written(&script, "tableOurs").len() + written(&script, "tableCell").len(),
        64
    );
    script.run(&format!("{PAGE_ON}; wait 2")).unwrap();
    assert_eq!(page(&mut script), "TABLE, page 2 of 2");
    assert_eq!(said(&script, "pageHeading"), ["THE KNOCKOUT ROUNDS"]);
    assert_eq!(
        said(&script, "knockoutRound"),
        ["THE SEMI-FINALS", "THE FINAL"]
    );
    // One semi-final has been played, so one finalist is known and the
    // other is whoever wins the second.
    let sides = written(&script, "knockoutSide");
    assert_eq!(sides.len(), 6);
    assert!(
        sides.contains(&"WINNER OF SEMI-FINAL 2".to_owned()),
        "{sides:?}"
    );
    assert_eq!(written(&script, "knockoutRuns").len(), 2);
    // The arrows go round.
    script.run(&format!("{PAGE_ON}; wait 2")).unwrap();
    assert_eq!(page(&mut script), "TABLE, page 1 of 2");
}

#[test]
fn a_cup_has_its_rounds_side_by_side_and_no_table() {
    let Some(mut script) = tables(Format::Cup, 5) else {
        return;
    };
    assert_eq!(page(&mut script), "TABLE, page 1 of 1");
    let rounds = ["THE QUARTER-FINALS", "THE SEMI-FINALS", "THE FINAL"];
    assert_eq!(said(&script, "knockoutRound"), rounds);
    assert_eq!(written(&script, "knockoutSide").len(), 14);
    // Five ties played: ten scores.
    assert_eq!(written(&script, "knockoutRuns").len(), 10);
    assert!(written(&script, "tableHead").is_empty());
}

#[test]
fn the_matches_are_listed_round_by_round_and_one_that_has_been_played_opens() {
    let Some(mut script) = tables(Format::League, 9) else {
        return;
    };
    // The matches open on the round that is being played, which has
    // nothing to open yet.
    script.run(&format!("{MATCHES}; wait 2")).unwrap();
    assert_eq!(page(&mut script), "MATCHES, page 4 of 5");
    assert_eq!(said(&script, "pageHeading"), ["ROUND 4"]);
    let to_play = written(&script, "roundFixture");
    assert_eq!(to_play.len(), 3);
    assert!(
        to_play.iter().all(|fixture| fixture.contains(" v ")),
        "{to_play:?}"
    );
    assert!(said(&script, "roundHint").is_empty());
    // The round before it has been played.
    script.run(&format!("{PAGE_BACK}; wait 2")).unwrap();
    assert_eq!(page(&mut script), "MATCHES, page 3 of 5");
    let played = written(&script, "roundFixture");
    assert!(
        played.iter().all(|fixture| fixture.contains(" - ")),
        "{played:?}"
    );
    assert_eq!(said(&script, "roundHint").len(), 1);
    // Its first match, opened: who won, and the innings of both sides.
    script.run(&format!("{FIRST_MATCH}; wait 2")).unwrap();
    assert_eq!(page(&mut script), "MATCHES, match 7, page 1 of 4");
    let verdict = said(&script, "matchVerdict").remove(0);
    assert!(verdict.contains(" BEAT "), "{verdict}");
    let runs: Vec<f32> = written(&script, "boardRuns")
        .iter()
        .map(|runs| runs.parse().unwrap())
        .collect();
    // The verdict gives the winner's runs and then the loser's.
    assert_eq!(runs.len(), 2);
    assert_ne!(runs[0], runs[1]);
    let line = format!("{} - {}", runs[0].max(runs[1]), runs[0].min(runs[1]));
    assert!(verdict.contains(&line), "{verdict} has not {line}");
    // Its figures, and the batting of each side, which adds up.
    script.run(&format!("{PAGE_ON}; wait 2")).unwrap();
    assert_eq!(written(&script, "figuresHead").len(), 4);
    for side in [3, 4] {
        script.run(&format!("{PAGE_ON}; wait 2")).unwrap();
        assert_eq!(
            page(&mut script),
            format!("MATCHES, match 7, page {side} of 4")
        );
        let heading = said(&script, "pageHeading").remove(0);
        assert!(heading.contains(" BATTING AGAINST "), "{heading}");
        let all = written(&script, "battingAll");
        let each = written(&script, "battingCell");
        assert_eq!((all.len(), each.len()), (11, 99));
        let hits: u32 = each
            .chunks(11)
            .map(|row| row[3].parse::<u32>().unwrap())
            .sum();
        assert_eq!(all[3], hits.to_string());
        assert!(said(&script, "pitcherLine")[0].contains(" PITCHER: "));
    }
    // Choosing a section begins it again, with nothing open.
    script.run(&format!("{MATCHES}; wait 2")).unwrap();
    assert_eq!(page(&mut script), "MATCHES, page 4 of 5");
    script.run(&format!("{TABLE}; wait 2")).unwrap();
    assert_eq!(page(&mut script), "TABLE, page 1 of 1");
}

#[test]
fn the_boards_button_leaves_the_tables() {
    let Some(mut script) = tables(Format::League, 3) else {
        return;
    };
    let after = state_after(&mut script, &format!("{CARRY_ON}; wait 60; state"));
    assert!(after.starts_with("Menu, "), "{after}");
    // Nothing of the tables is left behind them.
    assert!(said(&script, "pageHeading").is_empty());
    assert!(said(&script, "buttonWords").is_empty());
}
