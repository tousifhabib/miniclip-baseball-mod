//! What a full match's book has to say when the match is over: the pages
//! on the board it ends on.

mod common;

use bb_game::art::{self, all_named};
use bb_game::mods::Mod;
use bb_game::script::Script;
use bb_game::settings::Ground;
use common::{
    full_match, full_match_played as played, game, match_to_order, number, said, state, text,
    written,
};

/// The arrows that turn the pages.
const PAGE_ON: &str = "click 362 323";
const PAGE_BACK: &str = "click 228 323";

/// Turns to a page, counting from 1, and says how many there are.
fn turn_to(script: &mut Script, page: usize) -> usize {
    for _ in 0..60 {
        let now = state(script);
        let at = number(&now, ", page ").expect("a page") as usize;
        let of = number(now.split(", page ").nth(1).unwrap(), " of ").unwrap() as usize;
        if at == page {
            return of;
        }
        script.run(&format!("{PAGE_ON}; wait 2")).unwrap();
    }
    panic!("there is no page {page}: {}", state(script));
}

/// Whether the figures the art puts on the board are to be seen.
fn arts_figures_seen(script: &Script) -> bool {
    let stage = &script.runner.stage;
    let shell = art::shell(stage).expect("the shell");
    art::RESULT_FIGURES
        .into_iter()
        .filter_map(|figures| stage.find_symbol(&shell, figures))
        .any(|path| stage.child(&path).is_some_and(|figures| figures.visible))
}

#[test]
fn with_nobody_swinging_the_book_is_all_strikeouts() {
    let Some(mut script) = full_match(1, Ground::Away, &[], Some(match_to_order(2, 1))) else {
        return;
    };
    let over = played(&mut script, false);
    assert!(over.starts_with("MatchLost,"), "{over}");
    // Seven pages of figures, and one for each innings.
    assert!(over.ends_with(", page 1 of 9"), "{over}");
    assert!(arts_figures_seen(&script));
    let pitched = text(&script, "ballsPitched");
    let their_hits = written(&script, "boardHits")[1].clone();
    assert_eq!(written(&script, "boardHits")[0], "0");
    assert_eq!(written(&script, "boardErrors"), ["0", "0"]);

    // The player's batting: six came up and six struck out, three an
    // innings, and the pitcher who did it threw every ball there was.
    assert_eq!(turn_to(&mut script, 2), 9);
    assert!(!arts_figures_seen(&script));
    assert_eq!(said(&script, "pageHeading"), ["YOUR BATTING"]);
    assert_eq!(
        written(&script, "battingAll"),
        ["ALL", "6", "0", "0", "0", "0", "0", "0", "0", "6", ".000"]
    );
    let cells = written(&script, "battingCell");
    assert_eq!(cells.len(), 9 * 11);
    // The seventh in the order never came up.
    assert_eq!(
        cells[66..77],
        ["7", "0", "0", "0", "0", "0", "0", "0", "0", "0", "---"]
    );
    let pitcher = &said(&script, "pitcherLine")[0];
    let line = format!("THEIR PITCHER: 2.0 INNINGS, {pitched} PITCHES, ");
    assert!(pitcher.starts_with(&line), "{pitcher}");
    assert!(pitcher.ends_with("6 STRIKEOUTS, 0 WALKS"), "{pitcher}");

    // Theirs: one run in the one innings they needed, on the hits the
    // first page said.
    turn_to(&mut script, 3);
    assert_eq!(said(&script, "pageHeading"), ["THEIR BATTING"]);
    let all = written(&script, "battingAll");
    assert_eq!(
        (all[2].as_str(), all[3].as_str()),
        ("1", their_hits.as_str())
    );
    let pitcher = &said(&script, "pitcherLine")[0];
    assert!(
        pitcher.starts_with("YOUR PITCHER: 1.0 INNINGS, "),
        "{pitcher}"
    );

    // The figures of the two sides, side by side.
    turn_to(&mut script, 4);
    let ours = written(&script, "figuresOurs");
    let names = written(&script, "figuresName");
    assert_eq!((ours.len(), names.len()), (24, 24));
    let of = |name: &str| ours[names.iter().position(|each| each == name).unwrap()].clone();
    assert_eq!(of("AVERAGE"), ".000");
    assert_eq!(of("PITCHES SEEN"), pitched);
    assert_eq!(of("SWUNG AT"), "0%");
    assert_eq!(of("CALLED STRIKES"), "18");
    assert_eq!(of("SWINGING STRIKES"), "0");
    assert_eq!(of("STRUCK OUT"), "100%");
    assert_eq!(of("LONGEST HIT"), "0 FT");

    // Nothing was hit, and nothing swung at.
    turn_to(&mut script, 5);
    assert_eq!(said(&script, "pageHeading"), ["WHERE YOU HIT IT"]);
    assert_eq!(written(&script, "fieldLine")[0], "HOME RUNS 0");
    assert!(all_named(&script.runner.stage, &[], "fieldMark").is_empty());
    turn_to(&mut script, 7);
    assert_eq!(said(&script, "timingLine"), ["NOT A SWING ALL MATCH"]);

    // Each innings turn by turn: the visitors on the left.
    turn_to(&mut script, 8);
    assert_eq!(said(&script, "pageHeading"), ["THE 1ST INNINGS"]);
    let heads = written(&script, "turnsHead");
    assert_eq!(heads[0], "TOP: YOU, NO RUNS ON NO HITS");
    assert!(heads[1].starts_with("BOTTOM: THEM, 1 RUN ON "), "{heads:?}");
    let turns = written(&script, "turnLine");
    for (order, turn) in turns.iter().take(3).enumerate() {
        let begins = format!("{} STRUCK OUT LOOKING (", order + 1);
        assert!(turn.starts_with(&begins), "{turn}");
    }
    // Their half is there too, from their first batter on.
    assert!(turns.len() > 6, "{turns:?}");
    assert!(turns[3].starts_with("1 "), "{turns:?}");
    turn_to(&mut script, 9);
    assert_eq!(
        written(&script, "turnsHead"),
        ["TOP: YOU, NO RUNS ON NO HITS", "BOTTOM: THEM, NOT BATTED"]
    );
    assert_eq!(written(&script, "turnLine").len(), 3);

    // The arrows go round, and the art's figures are back with the first
    // page.
    script.run(&format!("{PAGE_ON}; wait 2")).unwrap();
    assert!(state(&mut script).ends_with(", page 1 of 9"));
    assert!(arts_figures_seen(&script));
    assert_eq!(said(&script, "boardVerdict"), ["YOU LOST 0 - 1"]);
    script.run(&format!("{PAGE_BACK}; wait 2")).unwrap();
    assert!(state(&mut script).ends_with(", page 9 of 9"));
}

#[test]
fn hits_are_written_up_with_where_they_went_and_how_they_were_timed() {
    // With every hit a home run, the four that win it are the whole of the
    // player's batting.
    let mods = [Mod::TimingIndicator, Mod::ZingerHit];
    let Some(mut script) = full_match(1, Ground::Home, &mods, Some(match_to_order(1, 1))) else {
        return;
    };
    let over = played(&mut script, true);
    assert!(over.starts_with("MatchWon,"), "{over}");
    assert!(over.contains("YOU WON 4 - 3"), "{over}");
    assert_eq!(written(&script, "boardHits")[1], "4");
    // The longest of the zingers is on the first page, under who won.
    assert!(said(&script, "zingerLine")[0].starts_with("LONGEST ZINGER "));

    turn_to(&mut script, 2);
    assert_eq!(
        written(&script, "battingAll"),
        ["ALL", "4", "4", "4", "0", "0", "4", "4", "0", "0", "1.000"]
    );
    turn_to(&mut script, 4);
    let ours = written(&script, "figuresOurs");
    let names = written(&script, "figuresName");
    let of = |name: &str| ours[names.iter().position(|each| each == name).unwrap()].clone();
    assert_eq!(of("SLUGGING"), "4.000");
    assert_eq!(of("TOTAL BASES"), "16");
    assert_eq!(of("PITCHES SEEN"), "4");
    assert_eq!(of("MET WHEN SWUNG AT"), "100%");
    let longest = of("LONGEST HIT");

    // Four marks on the field, all of them home runs, and the longest as
    // long as the figures said.
    turn_to(&mut script, 5);
    assert_eq!(all_named(&script.runner.stage, &[], "fieldMark").len(), 4);
    let lines = written(&script, "fieldLine");
    assert_eq!(lines[0], "HOME RUNS 4");
    assert_eq!(lines[9], format!("LONGEST {longest}"));
    assert_eq!(lines[7], "IN THE AIR 4");

    turn_to(&mut script, 7);
    let timing = said(&script, "timingLine");
    assert_eq!(timing[0], "SWINGS 4   EARLY 0   ON TIME 4   LATE 0");
    assert_eq!(timing[2], "ON THE WHOLE: ON TIME");

    // The innings: their three runs first, then the four home runs.
    turn_to(&mut script, 8);
    let turns = written(&script, "turnLine");
    let ours: Vec<&String> = turns.iter().rev().take(4).collect();
    for turn in ours {
        assert!(turn.contains(" HOME RUN TO "), "{turn}");
        assert!(turn.ends_with(", 1 RUN (1)"), "{turn}");
    }
    assert_eq!(
        written(&script, "turnsHead")[1],
        "BOTTOM: YOU, 4 RUNS ON 4 HITS"
    );
}

#[test]
fn the_last_innings_alone_has_no_pages() {
    let Some(mut script) = game("matchWon") else {
        return;
    };
    script.run("wait 340").unwrap();
    assert!(said(&script, "pageCount").is_empty());
    assert!(arts_figures_seen(&script));
}
