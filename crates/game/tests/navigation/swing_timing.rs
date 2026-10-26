//! The page after the mods' on which how late the player's clicks reach
//! the game is measured, and set by hand.

use bb_game::art::all_named;
use bb_game::script::Script;

use super::{MODS, NEXT_PAGE};
use crate::common::game_keeping_delay;

/// The arrows that take the delay down and up.
const LESS: &str = "click 338 226";
const MORE: &str = "click 418 226";
/// Somewhere with no button, to click on as the marker crosses the line.
const ANYWHERE: &str = "click 100 100";
/// Across the panel, where the marker's white is when it is on the line.
const ON_THE_LINE: f32 = 32.0;

/// Everything the page has written.
fn written(script: &Script) -> Vec<String> {
    let stage = &script.runner.stage;
    all_named(stage, &[], "modsWords")
        .iter()
        .filter_map(|words| stage.child(words).unwrap().said.clone())
        .collect()
}

/// Opens the mods' page and turns its pages to the last, the delay's.
fn to_the_page(script: &mut Script) {
    script.run(&format!("wait 60; {MODS}; wait 90")).unwrap();
    for _ in 0..20 {
        if written(script).iter().any(|said| said == "SWING TIMING") {
            return;
        }
        script.run(&format!("{NEXT_PAGE}; wait 3")).unwrap();
    }
    panic!("no page for the delay: {:?}", written(script));
}

/// Waits for the marker to be shown on the line.
fn to_the_line(script: &mut Script) {
    for _ in 0..200 {
        let stage = &script.runner.stage;
        let white = all_named(stage, &[], "delayMarker").pop().unwrap();
        let marker = stage.child(&white).unwrap();
        if marker.visible && marker.matrix.tx == ON_THE_LINE {
            return;
        }
        script.run("wait 1").unwrap();
    }
    panic!("the marker never reached the line");
}

#[test]
fn ten_clicks_measure_the_delay_and_the_arrows_set_it_and_it_is_kept() {
    let file = std::env::temp_dir().join(format!("bb-delay-page-{}.toml", std::process::id()));
    let _ = std::fs::remove_file(&file);
    let Some(mut script) = game_keeping_delay("menu", &file) else {
        return;
    };
    to_the_page(&mut script);
    assert!(written(&script).contains(&"0 MS".to_owned()));
    // Each click comes two frames after the marker is seen on the line.
    for click in 1..=10 {
        to_the_line(&mut script);
        script.run(&format!("wait 2; {ANYWHERE}")).unwrap();
        if click < 10 {
            let said = format!("CLICK {click} OF 10: 33 MS LATE");
            assert!(written(&script).contains(&said), "{:?}", written(&script));
        }
    }
    let said = written(&script);
    assert!(
        said.contains(&"MEASURED: THE DELAY IS NOW 25 MS".to_owned()),
        "{said:?}"
    );
    assert!(said.contains(&"25 MS".to_owned()), "{said:?}");

    // Five more, and ten less.
    script.run(&format!("{MORE}; wait 3")).unwrap();
    assert!(written(&script).contains(&"30 MS".to_owned()));
    script
        .run(&format!("{LESS}; wait 3; {LESS}; wait 3"))
        .unwrap();
    assert!(written(&script).contains(&"20 MS".to_owned()));

    // The next run starts where this one left it.
    let mut again = game_keeping_delay("menu", &file).unwrap();
    to_the_page(&mut again);
    assert!(
        written(&again).contains(&"20 MS".to_owned()),
        "{:?}",
        written(&again)
    );
    let _ = std::fs::remove_file(&file);
}

#[test]
fn a_click_far_from_the_line_is_not_counted() {
    let file = std::env::temp_dir().join(format!("bb-delay-far-{}.toml", std::process::id()));
    let _ = std::fs::remove_file(&file);
    let Some(mut script) = game_keeping_delay("menu", &file) else {
        return;
    };
    to_the_page(&mut script);
    to_the_line(&mut script);
    script.run(&format!("wait 25; {ANYWHERE}")).unwrap();
    assert!(written(&script).contains(&"TOO FAR FROM THE LINE TO COUNT".to_owned()));
    // Nothing was measured, so nothing was written down.
    assert!(!file.exists());
}
