//! Getting around: the menu, its pages, and the way into a game.

mod common;

use common::{game, state_after};

/// Where things are on the menu's pages, in stage pixels.
const BOTTOM_OF_THE_NINTH: &str = "click 200 192";
const ARCADE: &str = "click 200 237";
const MODS: &str = "click 330 360";
/// The mods listed on the mods' page, from the top.
const FIRST_MOD: &str = "click 300 140";
const SECOND_MOD: &str = "click 300 175";
const THIRD_MOD: &str = "click 300 215";
const FOURTH_MOD: &str = "click 300 255";
/// The first, third and last of the boxes that set how often the fourth
/// mod's fielders let the ball go.
const SELDOM: &str = "click 329 276";
const MIDDLING: &str = "click 353 276";
const ALWAYS: &str = "click 379 276";
const NEXT: &str = "click 490 362";
const BACK: &str = "click 290 362";
const PLAY_BALL: &str = "click 480 362";

#[test]
fn the_menu_leads_through_setup_to_a_match() {
    let Some(mut script) = game("menu") else {
        return;
    };
    assert_eq!(
        state_after(&mut script, "wait 60; state"),
        "Menu, Main, Medium"
    );
    let setup = format!("{BOTTOM_OF_THE_NINTH}; wait 60; state");
    assert_eq!(state_after(&mut script, &setup), "Menu, MatchSetup, Medium");
    let summary = format!("{NEXT}; wait 60; state");
    assert_eq!(
        state_after(&mut script, &summary),
        "Menu, MatchSummary, Medium"
    );
    let play = format!("{PLAY_BALL}; wait 120; state");
    assert!(state_after(&mut script, &play).starts_with("Match"));
}

#[test]
fn the_menu_leads_through_setup_to_the_arcade_game() {
    let Some(mut script) = game("menu") else {
        return;
    };
    let steps = format!("wait 60; {ARCADE}; wait 60; {NEXT}; wait 60; state");
    assert_eq!(
        state_after(&mut script, &steps),
        "Menu, ArcadeSummary, Medium"
    );
    let play = format!("{PLAY_BALL}; wait 120; state");
    assert!(state_after(&mut script, &play).starts_with("Arcade"));
}

#[test]
fn back_returns_to_the_page_before() {
    let Some(mut script) = game("menu") else {
        return;
    };
    let steps =
        format!("wait 60; {BOTTOM_OF_THE_NINTH}; wait 60; {NEXT}; wait 60; {BACK}; wait 60; state");
    assert_eq!(state_after(&mut script, &steps), "Menu, MatchSetup, Medium");
    let steps = format!("{BACK}; wait 60; state");
    assert_eq!(state_after(&mut script, &steps), "Menu, Main, Medium");
}

#[test]
fn a_page_that_is_still_arriving_ignores_clicks() {
    let Some(mut script) = game("menu") else {
        return;
    };
    // No wait first: the menu has only begun to fade in.
    let steps = format!("{BOTTOM_OF_THE_NINTH}; wait 60; state");
    assert_eq!(state_after(&mut script, &steps), "Menu, Main, Medium");
}

#[test]
fn the_team_name_can_be_typed() {
    let Some(mut script) = game("menu") else {
        return;
    };
    let steps = format!(
        "wait 60; {BOTTOM_OF_THE_NINTH}; wait 60; click 450 134; type Red Sox 9; \
         key backspace; key backspace"
    );
    script.run(&steps).unwrap();
    assert_eq!(script.runner.stage.text("teamName"), Some("Red Sox"));
    // A click off the field ends the typing, so later keys change nothing.
    script.run("click 300 300; type xyz").unwrap();
    assert_eq!(script.runner.stage.text("teamName"), Some("Red Sox"));
}

#[test]
fn a_colour_picked_from_the_strip_dresses_the_batter_and_goes_into_the_match() {
    let Some(mut script) = game("menu") else {
        return;
    };
    // The green part of the Team Colours strip on the match setup page.
    let steps = format!("wait 60; {BOTTOM_OF_THE_NINTH}; wait 60; click 440 194; wait 5");
    script.run(&steps).unwrap();
    let colour_of = |script: &bb_game::script::Script, name: &str| {
        let stage = &script.runner.stage;
        let path = stage.find_named(&[], name).expect("the part to be there");
        stage.child(&path).unwrap().color
    };
    let helmet = colour_of(&script, "helmetMovie");
    // A flat tint: the art's own colours are thrown away for the one picked.
    assert_eq!(helmet.mult, [0.0, 0.0, 0.0, 1.0]);
    let [red, green, blue, _] = helmet.add;
    assert!(green > red && green > blue, "{:?}", helmet.add);
    assert_eq!(colour_of(&script, "tShirtMovie"), helmet);

    // In the match the batter wears it too, and has a skin of his own.
    let play = format!("{NEXT}; wait 60; {PLAY_BALL}; wait 150");
    script.run(&play).unwrap();
    assert!(state_after(&mut script, "state").starts_with("Match"));
    assert_eq!(colour_of(&script, "helmetMovie"), helmet);
    assert_eq!(colour_of(&script, "skinMovie").mult, [0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn the_menu_has_music_and_a_game_has_a_crowd() {
    let Some(mut script) = game("menu") else {
        return;
    };
    let note = |script: &bb_game::script::Script, name: &str| {
        format!("sound {}", script.runner.library.manifest.exports[name])
    };
    let (music, crowd) = (
        note(&script, "introMusic_loop"),
        note(&script, "crowd_loop_1"),
    );
    let heard = script.run("wait 60; events").unwrap();
    assert!(heard.iter().any(|line| line.trim() == music), "{heard:?}");
    assert!(!heard.iter().any(|line| line.trim() == crowd), "{heard:?}");

    let steps =
        format!("{BOTTOM_OF_THE_NINTH}; wait 60; {NEXT}; wait 60; {PLAY_BALL}; wait 200; events");
    let heard = script.run(&steps).unwrap();
    assert!(heard.iter().any(|line| line.trim() == crowd), "{heard:?}");
}

#[test]
fn an_arcade_score_is_shown_on_the_high_score_page() {
    let Some(mut script) = game("arcade") else {
        return;
    };
    // Ten pitches let go by: a score of nothing, but a score.
    for _ in 0..10 {
        script.run("wait 320; click 545 355").unwrap();
    }
    assert!(state_after(&mut script, "wait 420; state").starts_with("ArcadeFinish"));
    // The finish screen's HIGH SCORES button.
    assert_eq!(
        state_after(&mut script, "click 300 373; wait 120; state"),
        "Menu, HighScores, Medium"
    );
    let stage = &script.runner.stage;
    let mut written = Vec::new();
    for line in bb_game::art::all_named(stage, &[], "scoreLine") {
        written.extend(stage.child(&line).unwrap().said.clone());
    }
    for wanted in ["HIGHSCORES", "1", "PLAYER", "0"] {
        assert!(written.iter().any(|text| text == wanted), "{written:?}");
    }
}

#[test]
fn the_pointer_is_hidden_only_while_the_ring_is_being_aimed() {
    let Some(mut script) = game("match") else {
        return;
    };
    // Over the batting area before the pitch: the ring stands for it.
    script.run("wait 30; move 300 250; wait 5").unwrap();
    assert!(script.runner.stage.hide_pointer);
    // Off to the side, over the quit button's corner: an ordinary pointer.
    script.run("move 20 390; wait 5").unwrap();
    assert!(!script.runner.stage.hide_pointer);
    // And once the pitch has gone by, wherever it is.
    script.run("move 300 250; wait 400").unwrap();
    assert!(state_after(&mut script, "state").contains("Ready"));
    assert!(!script.runner.stage.hide_pointer);
}

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
    let steps = format!("wait 60; {MODS}; wait 90; {BACK}; wait 90; click 250 275; wait 90; state");
    assert_eq!(state_after(&mut script, &steps), "Menu, HighScores, Medium");
    let stage = &script.runner.stage;
    assert!(bb_game::art::all_named(stage, &[], "modsWords").is_empty());
    assert!(bb_game::art::all_named(stage, &[], "modsPanel").is_empty());
    assert!(!bb_game::art::all_named(stage, &[], "scoreLine").is_empty());
}

#[test]
fn the_start_menu_no_longer_names_its_author() {
    let Some(mut script) = game("menu") else {
        return;
    };
    script.run("wait 60").unwrap();
    let stage = &script.runner.stage;
    let menu = bb_game::art::in_shell(stage, bb_game::art::MENU).unwrap();
    assert!(
        stage
            .find_symbol(&menu, bb_game::art::MENU_CREDIT)
            .is_none()
    );
}
