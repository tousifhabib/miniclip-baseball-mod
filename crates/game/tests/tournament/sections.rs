//! The sections of the tables that tell of the sides, of the best of
//! them, and of the records.

use bb_game::tournament::Format;

use super::common::{said, written};
use super::{PAGE_ON, page, tables};

/// The boxes that choose these sections, and the arrow beside the first
/// side of the list.
const SIDES: &str = "click 255 63";
const LEADERS: &str = "click 359 63";
const RECORDS: &str = "click 463 63";
const FIRST_SIDE: &str = "click 52 120";

#[test]
fn every_side_is_listed_and_one_that_is_opened_shows_what_it_has_done() {
    let Some(mut script) = tables(Format::League, 9) else {
        return;
    };
    script.run(&format!("{SIDES}; wait 2")).unwrap();
    assert_eq!(page(&mut script), "SIDES, page 1 of 1");
    assert_eq!(said(&script, "sidesHead"), ["WON", "LOST", "RUNS"]);
    let names = written(&script, "sideName");
    assert_eq!(names.len(), 6);
    assert!(names.contains(&"YOU".to_owned()), "{names:?}");
    // Three matches each: won and lost add up to them.
    let done = written(&script, "sideDone");
    for side in done.chunks(3) {
        let number = |cell: &String| cell.parse::<u32>().unwrap();
        assert_eq!(number(&side[0]) + number(&side[1]), 3, "{side:?}");
        assert!(side[2].contains(" - "));
    }
    // The first of them, opened: its fixtures, and how each came out.
    script.run(&format!("{FIRST_SIDE}; wait 2")).unwrap();
    let first = &names[0];
    assert_eq!(page(&mut script), format!("SIDES, {first}, page 1 of 3"));
    let heading = said(&script, "pageHeading").remove(0);
    assert!(heading.starts_with(&format!("{first}: WON ")), "{heading}");
    let results = written(&script, "resultsCell");
    assert_eq!(results.len(), 5 * 4);
    let came_out: Vec<&String> = results.chunks(4).map(|row| &row[3]).collect();
    let played = came_out.iter().filter(|cell| cell.as_str() != "TO PLAY");
    assert_eq!(played.count(), 3, "{came_out:?}");
    // Its batting in every match, which adds up.
    script.run(&format!("{PAGE_ON}; wait 2")).unwrap();
    assert_eq!(
        said(&script, "pageHeading"),
        [format!("{first}: BATTING IN EVERY MATCH")]
    );
    let (all, each) = (
        written(&script, "battingAll"),
        written(&script, "battingCell"),
    );
    let at_bats: u32 = each
        .chunks(11)
        .map(|row| row[1].parse::<u32>().unwrap())
        .sum();
    assert_eq!(all[1], at_bats.to_string());
    assert!(said(&script, "pitcherLine")[0].starts_with("PITCHED TO: "));
    // And its figures beside those of the sides it has played.
    script.run(&format!("{PAGE_ON}; wait 2")).unwrap();
    assert_eq!(page(&mut script), format!("SIDES, {first}, page 3 of 3"));
    let heads = written(&script, "figuresHead");
    assert_eq!(heads.len(), 4);
    assert_eq!((heads[1].as_str(), heads[3].as_str()), ("AGST", "AGST"));
    // Choosing the section again is the list again.
    script.run(&format!("{SIDES}; wait 2")).unwrap();
    assert_eq!(page(&mut script), "SIDES, page 1 of 1");
}

#[test]
fn the_best_batters_and_the_best_sides_are_each_six_lists() {
    let Some(mut script) = tables(Format::Groups, 13) else {
        return;
    };
    script.run(&format!("{LEADERS}; wait 2")).unwrap();
    assert_eq!(page(&mut script), "LEADERS, page 1 of 2");
    assert_eq!(said(&script, "pageHeading"), ["THE BEST BATTERS"]);
    let batters = said(&script, "bestOf");
    assert_eq!(batters.len(), 6);
    assert_eq!(
        (batters[0].as_str(), batters[5].as_str()),
        ("AVERAGE", "LONGEST HIT")
    );
    // Five of the best in each, a batter being his side and his place.
    let (who, has) = (written(&script, "bestWho"), written(&script, "bestHas"));
    assert_eq!((who.len(), has.len()), (30, 30));
    for batter in &who {
        let (side, place) = batter.split_once(' ').expect("a side and a place");
        assert!(side.len() <= 4 && place.parse::<u32>().is_ok(), "{batter}");
    }
    script.run(&format!("{PAGE_ON}; wait 2")).unwrap();
    assert_eq!(said(&script, "pageHeading"), ["THE BEST SIDES"]);
    let sides = said(&script, "bestOf");
    assert_eq!(
        (sides[0].as_str(), sides[5].as_str()),
        ("RUNS A MATCH", "MATCHES WON")
    );
    assert_eq!(written(&script, "bestWho").len(), 30);
}

#[test]
fn the_records_and_the_totals_are_of_the_matches_there_have_been() {
    let Some(mut script) = tables(Format::Groups, 13) else {
        return;
    };
    script.run(&format!("{RECORDS}; wait 2")).unwrap();
    assert_eq!(page(&mut script), "RECORDS, page 1 of 2");
    let what = written(&script, "recordWhat");
    assert_eq!(what[0], "HIGHEST SCORE");
    assert_eq!(what.len(), written(&script, "recordStands").len());
    let by = written(&script, "recordBy");
    assert!(
        by.iter().all(|by| by.contains(" v ") || by.contains(" - ")),
        "{by:?}"
    );
    script.run(&format!("{PAGE_ON}; wait 2")).unwrap();
    assert_eq!(said(&script, "pageHeading"), ["IN ALL"]);
    assert_eq!(written(&script, "totalWhat")[0], "MATCHES PLAYED");
    let totals = written(&script, "totalHas");
    assert_eq!(totals[0], "13 OF 15");
    assert_eq!(totals.len(), 13);
}

#[test]
fn before_a_ball_is_thrown_there_is_nobody_and_nothing_to_list() {
    let Some(mut script) = tables(Format::Cup, 0) else {
        return;
    };
    script.run(&format!("{LEADERS}; wait 2")).unwrap();
    assert_eq!(written(&script, "bestNobody").len(), 6);
    assert!(written(&script, "bestWho").is_empty());
    script.run(&format!("{RECORDS}; wait 2")).unwrap();
    assert_eq!(
        said(&script, "recordsNone"),
        ["NO MATCH HAS BEEN PLAYED YET"]
    );
    script.run(&format!("{PAGE_ON}; wait 2")).unwrap();
    assert_eq!(written(&script, "totalHas")[0], "0 OF 7");
    // Every side is there to be opened all the same, with its fixtures
    // still to play.
    script
        .run(&format!("{SIDES}; wait 2; {FIRST_SIDE}; wait 2"))
        .unwrap();
    let results = written(&script, "resultsCell");
    assert_eq!(results.len(), 4);
    assert_eq!(results[3], "TO PLAY");
}
