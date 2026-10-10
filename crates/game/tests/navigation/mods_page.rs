//! The page of the menu that lists the mods: its boxes, its pages, and
//! the settings of the mods that have one.

use super::{BACK, LAST_PAGE, MODS, NEXT_PAGE};
use crate::common::{game, state_after};

/// The first, third and last of the boxes that set how often the fourth
/// mod's fielders let the ball go.
const SELDOM: &str = "click 329 271";
const MIDDLING: &str = "click 353 271";
const ALWAYS: &str = "click 379 271";

/// The mods listed on the mods' page, from the top.
const FIRST_MOD: &str = "click 300 140";
const SECOND_MOD: &str = "click 300 175";
const THIRD_MOD: &str = "click 300 210";
const FOURTH_MOD: &str = "click 300 245";

#[test]
fn the_menu_lists_the_mods_each_with_a_box_to_tick() {
    let Some(mut script) = game("menu") else {
        return;
    };
    let steps = format!("wait 60; {MODS}; wait 90; state");
    assert_eq!(state_after(&mut script, &steps), "Menu, Mods, Medium");
    let said = |script: &bb_game::script::Script| {
        let stage = &script.runner.stage;
        let mut written = Vec::new();
        for line in bb_game::art::all_named(stage, &[], "modsWords") {
            written.extend(stage.child(&line).unwrap().said.clone());
        }
        written
    };
    let ticked = |script: &bb_game::script::Script| {
        let stage = &script.runner.stage;
        bb_game::art::all_named(stage, &[], "modTick")
            .iter()
            .map(|tick| stage.child(tick).unwrap().visible)
            .collect::<Vec<bool>>()
    };
    let written = said(&script);
    for wanted in [
        "MODS",
        "TIMING INDICATOR",
        "LONE PITCHER",
        "ZINGER HIT",
        "BUTTERFINGERS",
        "HOW OFTEN",
        "60%",
    ] {
        assert!(written.iter().any(|text| text == wanted), "{written:?}");
    }
    // The page is the high-score page put to another use: none of the
    // table is written on it.
    assert!(bb_game::art::all_named(&script.runner.stage, &[], "scoreLine").is_empty());
    assert_eq!(ticked(&script), [false, false, false, false]);

    // A click anywhere on a mod's line switches it and no other, and
    // another switches it back.
    let tick = format!("{FIRST_MOD}; wait 2; state");
    assert_eq!(
        state_after(&mut script, &tick),
        "Menu, Mods, Medium, with timing_indicator"
    );
    assert_eq!(ticked(&script), [true, false, false, false]);
    assert_eq!(state_after(&mut script, &tick), "Menu, Mods, Medium");
    assert_eq!(ticked(&script), [false, false, false, false]);
    let second = format!("{SECOND_MOD}; wait 2; state");
    assert_eq!(
        state_after(&mut script, &second),
        "Menu, Mods, Medium, with lone_pitcher"
    );
    assert_eq!(ticked(&script), [false, true, false, false]);
    assert_eq!(state_after(&mut script, &second), "Menu, Mods, Medium");
    let third = format!("{THIRD_MOD}; wait 2; state");
    assert_eq!(
        state_after(&mut script, &third),
        "Menu, Mods, Medium, with zinger_hit"
    );
    assert_eq!(ticked(&script), [false, false, true, false]);
    assert_eq!(state_after(&mut script, &third), "Menu, Mods, Medium");

    // What was chosen lasts through the rest of the menu, and the page
    // shows it on coming back.
    let steps = format!("{FIRST_MOD}; wait 2; {BACK}; wait 90; state");
    assert_eq!(
        state_after(&mut script, &steps),
        "Menu, Main, Medium, with timing_indicator"
    );
    let steps = format!("{MODS}; wait 90; state");
    assert_eq!(
        state_after(&mut script, &steps),
        "Menu, Mods, Medium, with timing_indicator"
    );
    assert_eq!(ticked(&script), [true, false, false, false]);
}

#[test]
fn every_mod_is_listed_on_one_of_the_pages_of_the_list() {
    use bb_game::mods::Mod;
    let Some(mut script) = game("menu") else {
        return;
    };
    script.run(&format!("wait 60; {MODS}; wait 90")).unwrap();
    // What a page of the list says, and whether it has a page after it.
    let page = |script: &bb_game::script::Script| {
        let stage = &script.runner.stage;
        let said: Vec<String> = bb_game::art::all_named(stage, &[], "modsWords")
            .iter()
            .filter_map(|words| stage.child(words).unwrap().said.clone())
            .collect();
        let boxes = bb_game::art::all_named(stage, &[], "modBox").len();
        let more = bb_game::art::all_named(stage, &[], "modsOn")
            .iter()
            .any(|arrow| stage.child(arrow).unwrap().visible);
        (said, boxes, more)
    };
    let (first, ..) = page(&script);
    let (mut listed, mut boxes, mut pages) = (Vec::new(), 0, 0);
    loop {
        let (said, on_page, more) = page(&script);
        listed.extend(said);
        boxes += on_page;
        pages += 1;
        if !more {
            break;
        }
        assert!(pages < 20);
        script.run(&format!("{NEXT_PAGE}; wait 3")).unwrap();
    }
    for which in Mod::ALL {
        assert!(listed.iter().any(|said| said == which.name()), "{which:?}");
        assert!(listed.iter().any(|said| said == which.about()), "{which:?}");
    }
    assert_eq!(boxes, Mod::ALL.len());
    // The arrow back leads to the first page again.
    for _ in 1..pages {
        script.run(&format!("{LAST_PAGE}; wait 3")).unwrap();
    }
    assert_eq!(page(&script).0, first);
    if pages > 1 {
        assert!(
            first
                .iter()
                .any(|said| said == &format!("PAGE 1 OF {pages}"))
        );
    }
}

#[test]
fn a_mod_with_a_setting_has_its_level_set_on_the_mods_page() {
    let Some(mut script) = game("menu") else {
        return;
    };
    let steps = format!("wait 60; {MODS}; wait 90; {FOURTH_MOD}; wait 2; state");
    // It starts at the middle of its five levels.
    assert_eq!(
        state_after(&mut script, &steps),
        "Menu, Mods, Medium, with butterfingers=3"
    );
    // How many of the setting's boxes are filled, and what the page says
    // that level comes to.
    let shown = |script: &bb_game::script::Script| {
        let stage = &script.runner.stage;
        let filled = bb_game::art::all_named(stage, &[], "modPipFill")
            .iter()
            .filter(|fill| stage.child(fill).unwrap().visible)
            .count();
        let says = bb_game::art::all_named(stage, &[], "modsWords")
            .iter()
            .filter_map(|words| stage.child(words).unwrap().said.clone())
            .find(|said| said.ends_with('%'));
        (filled, says.unwrap_or_default())
    };
    assert_eq!(shown(&script), (3, "60%".to_owned()));

    // A click on a box sets the level to it, and does not switch the mod.
    for (click, level, says) in [
        (ALWAYS, 5, "100%"),
        (SELDOM, 1, "20%"),
        (MIDDLING, 3, "60%"),
    ] {
        let steps = format!("{click}; wait 2; state");
        assert_eq!(
            state_after(&mut script, &steps),
            format!("Menu, Mods, Medium, with butterfingers={level}")
        );
        assert_eq!(shown(&script), (level, says.to_owned()));
    }

    // The level is kept while the mod is off, and is there when it is
    // switched on again.
    let steps = format!("{ALWAYS}; wait 2; {FOURTH_MOD}; wait 2; state");
    assert_eq!(state_after(&mut script, &steps), "Menu, Mods, Medium");
    assert_eq!(shown(&script), (5, "100%".to_owned()));
    let steps = format!("{SELDOM}; wait 2; state");
    assert_eq!(state_after(&mut script, &steps), "Menu, Mods, Medium");
    let steps = format!("{FOURTH_MOD}; wait 2; {BACK}; wait 90; state");
    assert_eq!(
        state_after(&mut script, &steps),
        "Menu, Main, Medium, with butterfingers=1"
    );
}

#[test]
fn the_high_score_page_is_itself_again_after_the_mods_page() {
    let Some(mut script) = game("menu") else {
        return;
    };
    let steps = format!("wait 60; {MODS}; wait 90; {BACK}; wait 90; click 250 303; wait 90; state");
    assert_eq!(state_after(&mut script, &steps), "Menu, HighScores, Medium");
    let stage = &script.runner.stage;
    assert!(bb_game::art::all_named(stage, &[], "modsWords").is_empty());
    assert!(bb_game::art::all_named(stage, &[], "modsPanel").is_empty());
    assert!(!bb_game::art::all_named(stage, &[], "scoreLine").is_empty());
}
