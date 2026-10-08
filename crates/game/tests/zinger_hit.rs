//! The zinger hit mod: every ball the bat meets is a home run, and the
//! better the swing was timed the further it goes.

mod common;

use bb_game::art::all_named;
use bb_game::mods::Mod;
use bb_game::scores::Scores;
use bb_game::script::Script;
use common::{
    game_keeping, game_modded, next, number, pitch, pitch_seen, playing, said, sounds, state,
};

/// The mods a game is played with here: the timing bar always, which says
/// when to swing, and the zinger or not.
fn mods(zinger: bool) -> Vec<Mod> {
    let mut mods = vec![Mod::TimingIndicator];
    if zinger {
        mods.push(Mod::ZingerHit);
    }
    mods
}

fn game(screen: &str, seed: u64, zinger: bool) -> Option<Script> {
    game_modded(screen, seed, &mods(zinger))
}

/// How far the state says the ball went, if it was hit for a zinger.
fn feet(state: &str) -> Option<u32> {
    number(state, "a zinger of ").map(|feet| feet as u32)
}

/// The feet a line such as "850 FT" gives.
fn feet_said(script: &Script) -> Option<u32> {
    let said = said(script, "zingerFeet");
    number(&format!("={}", said.first()?), "=").map(|feet| feet as u32)
}

/// Where each of the nine fielders is on the field.
fn fielders(script: &Script) -> Vec<(f32, f32)> {
    let stage = &script.runner.stage;
    let Some(main) = stage.find_named(&[], "gameMain") else {
        return Vec::new();
    };
    (1..=9)
        .filter_map(|number| {
            let path = stage.find(&main, &["field", &format!("fielder{number}")])?;
            let fielder = stage.child(&path)?;
            Some((fielder.matrix.tx, fielder.matrix.ty))
        })
        .collect()
}

/// Ways to hold the ring that spoil a hit as the game was, each with a
/// game whose first pitch it spoils: far above the ball, which tops it into
/// the ground, far below, which skies it, and far to either side, which
/// sends it foul past one line or the other.
const SPOILT: [(u64, (f32, f32)); 4] = [
    (1, (0.0, -80.0)),
    (1, (0.0, 80.0)),
    (1, (-140.0, 0.0)),
    (4, (170.0, 0.0)),
];
/// The ring on the ball.
const ON: (f32, f32) = (0.0, 0.0);
/// The furthest a zinger goes at the skill level these games are played
/// at, and the least any goes.
const MOST: u32 = 850;
const LEAST: u32 = 440;

#[test]
fn every_ball_the_bat_meets_is_a_home_run() {
    for seed in [1, 2, 3] {
        // From the earliest swing that meets the ball to one of the latest.
        for late in -1..=4 {
            let Some(mut script) = game("match", seed, true) else {
                return;
            };
            let after = pitch(&mut script, late, ON);
            assert!(after.contains("score 1 of"), "seed {seed}, {late}: {after}");
            assert!(after.contains("outs 0, count 0-0, bases ---"), "{after}");
            assert!(feet(&after).is_some(), "seed {seed}, {late}: {after}");
        }
    }
}

#[test]
fn it_is_a_home_run_wherever_the_ring_was_held() {
    for (seed, off) in SPOILT {
        let Some(mut usual) = game("match", seed, false) else {
            return;
        };
        // As the game was, none of these gets a run.
        let after = pitch(&mut usual, 0, off);
        assert!(after.contains("score 0 of"), "{off:?}: {after}");
        assert_eq!(feet(&after), None);

        let mut zinger = game("match", seed, true).unwrap();
        let after = pitch(&mut zinger, 0, off);
        assert!(after.contains("score 1 of"), "{off:?}: {after}");
        assert!(after.contains("outs 0, count 0-0, bases ---"), "{after}");
    }
}

#[test]
fn the_better_the_swing_is_timed_the_further_the_ball_goes() {
    for seed in [1, 2, 3] {
        // Later and later after the best moment, as far as the last swing
        // that still meets the ball.
        let mut went = Vec::new();
        for late in 0..=4 {
            let Some(mut script) = game("match", seed, true) else {
                return;
            };
            went.push(feet(&pitch(&mut script, late, ON)).unwrap());
        }
        assert!(went.is_sorted_by(|a, b| a >= b), "seed {seed}: {went:?}");
        // From the furthest there is to the least a ring on the ball gets.
        assert!(
            went[0] > MOST - 5 && went[0] <= MOST,
            "seed {seed}: {went:?}"
        );
        assert!(went[4] > LEAST && went[4] <= LEAST + 60, "{went:?}");
        assert!(
            went[0] > went[2] && went[2] > went[4],
            "seed {seed}: {went:?}"
        );

        // Too early by the same amount is no better than too late.
        let mut script = game("match", seed, true).unwrap();
        let early = feet(&pitch(&mut script, -1, ON)).unwrap();
        assert!(early < went[0], "seed {seed}: {early}");
    }
}

#[test]
fn holding_the_ring_on_the_ball_adds_a_little_and_never_as_much_as_timing() {
    let went = |late, off| {
        let mut script = game("match", 1, true)?;
        feet(&pitch(&mut script, late, off))
    };
    let Some(on) = went(0, ON) else {
        return;
    };
    let off = went(0, (-40.0, 0.0)).unwrap();
    let far_off = went(0, (-140.0, 0.0)).unwrap();
    assert!(on > off && off > far_off, "{on} {off} {far_off}");
    assert_eq!(on - far_off, 60);
    // The best-timed swing with the ring nowhere near still beats one a
    // little late with the ring on the ball.
    assert!(far_off > went(2, ON).unwrap());
    // And the least there is comes of the worst of both.
    assert_eq!(went(3, (-140.0, 0.0)), Some(LEAST));
}

/// Plays the first pitch of a match with the ring held `off`, timed as
/// well as can be. Returns how many frames after the swing the run was
/// scored, the furthest the count had got while the score stood at
/// nothing, and the state after.
fn timed_to_the_run(off: (f32, f32)) -> Option<(u32, u32, String)> {
    let mut script = game("match", 1, true)?;
    let (mut since, mut scored, mut counted) = (0, None, 0);
    let after = pitch_seen(&mut script, 0, off, |script, now| {
        if feet(now).is_none() {
            return;
        }
        since += 1;
        if now.contains("score 0 of") {
            counted = counted.max(feet_said(script).unwrap_or(0));
        } else if scored.is_none() {
            scored = Some(since);
        }
    });
    Some((scored.expect("a run to be scored"), counted, after))
}

#[test]
fn a_skied_ball_hangs_and_a_driven_one_is_soon_gone() {
    let Some((level, ..)) = timed_to_the_run(ON) else {
        return;
    };
    let (skied, _, after) = timed_to_the_run((0.0, 50.0)).unwrap();
    let (driven, ..) = timed_to_the_run((0.0, -50.0)).unwrap();
    assert!(skied > level * 2, "{skied} against {level}");
    assert!(driven < level, "{driven} against {level}");
    // However long it hung, it was a home run in the end.
    assert!(after.contains("score 1 of"), "{after}");
    assert!(after.contains("bases ---"), "{after}");
}

#[test]
fn the_home_run_is_called_when_the_ball_comes_down_not_when_it_goes_over() {
    for off in [ON, (0.0, 50.0)] {
        let Some((_, counted, after)) = timed_to_the_run(off) else {
            return;
        };
        // The wall is 400 feet off. The count had gone far past that, all
        // but the whole way, before there was a run.
        let whole = feet(&after).unwrap();
        assert!(
            counted > 600 && counted > whole - 20,
            "{counted} of {whole}"
        );
        assert!(counted <= whole);
    }
}

#[test]
fn the_distance_is_counted_up_and_the_place_named_when_the_ball_is_down() {
    let Some(mut script) = game("match", 1, true) else {
        return;
    };
    assert!(said(&script, "zingerFeet").is_empty());
    let mut counts = Vec::new();
    let after = pitch_seen(&mut script, 0, ON, |script, now| {
        if let (Some(count), true) = (feet_said(script), now.contains("score 0 of")) {
            // Nothing is said of where it went while it is still going.
            assert!(said(script, "zingerPlace").is_empty());
            counts.push(count);
        }
    });
    assert!(counts.is_sorted(), "{counts:?}");
    counts.dedup();
    assert!(counts.len() > 50, "{counts:?}");
    // It stops on how far the ball went.
    let whole = feet(&after).unwrap();
    assert_eq!(said(&script, "zingerFeet"), [format!("{whole} FT")]);
    assert_eq!(said(&script, "zingerPlace"), ["OUT OF THE PARK"]);
    // The ball's trail and the mark of where it would land have gone.
    let stage = &script.runner.stage;
    assert!(all_named(stage, &[], "zingerTrail").is_empty());

    // All of it goes with the next pitch.
    next(&mut script);
    script.run("wait 60").unwrap();
    assert!(said(&script, "zingerFeet").is_empty());
    assert!(said(&script, "zingerPlace").is_empty());
}

#[test]
fn where_the_ball_went_is_named_by_how_far_and_which_way() {
    // The first pitch of each game, how late the swing and where the ring:
    // one sent a long way, one just over the wall in the middle, where the
    // board is, and one just over it down the line.
    let cases = [
        (1, 0, ON, "OUT OF THE PARK"),
        (3, 3, (0.0, 0.0), "OFF THE SCOREBOARD"),
        (1, 3, (-140.0, 0.0), "INTO THE STANDS"),
    ];
    for (seed, late, off, place) in cases {
        let Some(mut script) = game("match", seed, true) else {
            return;
        };
        pitch(&mut script, late, off);
        assert_eq!(said(&script, "zingerPlace"), [place], "seed {seed}");
    }
}

#[test]
fn the_ball_is_drawn_large_with_a_trail_and_a_mark_where_it_will_land() {
    let Some(mut script) = game("match", 1, true) else {
        return;
    };
    let (mut largest, mut dots, mut marked) = (0.0f32, 0, false);
    pitch_seen(&mut script, 3, (-140.0, 0.0), |script, now| {
        if !now.contains(": Fielding") || !now.contains("score 0 of") {
            return;
        }
        let stage = &script.runner.stage;
        let main = stage.find_named(&[], "gameMain").unwrap();
        let ball = stage.find(&main, &["field", "ballFly"]).unwrap();
        largest = largest.max(stage.child(&ball).unwrap().matrix.a);
        let seen = |name: &str| {
            all_named(stage, &[], name)
                .iter()
                .filter(|path| stage.child(path).unwrap().visible)
                .count()
        };
        dots = dots.max(seen("zingerDot"));
        marked |= seen("zingerMarker") == 1;
    });
    // As the game was, the ball is never drawn larger than 0.6 of its own
    // size. It is larger than that now, but not by much.
    assert!(largest > 0.6 && largest < 1.0, "{largest}");
    assert_eq!(dots, 8);
    assert!(marked);
}

#[test]
fn the_crowd_makes_more_of_a_better_hit_and_of_a_longer_one() {
    let heard = |late, off| {
        let mut script = game("match", 1, true)?;
        let mut all = Vec::new();
        pitch_seen(&mut script, late, off, |_, _| {});
        all.extend(sounds(&mut script));
        Some(all)
    };
    let Some(best) = heard(0, ON) else {
        return;
    };
    let worst = heard(3, (-140.0, 0.0)).unwrap();
    let count = |heard: &[String], name: &str| heard.iter().filter(|sound| *sound == name).count();
    // The bat, and the crowd as the ball is hit and as it comes down.
    assert_eq!(count(&best, "batHit_good"), 1, "{best:?}");
    assert_eq!(count(&worst, "batHit_medium"), 1, "{worst:?}");
    assert!(count(&best, "crowd_bigClap") > count(&worst, "crowd_bigClap"));
    assert_eq!(count(&best, "baseball_organ_FX"), 1, "{best:?}");
    assert!(count(&worst, "crowd_smallCheer") >= 1, "{worst:?}");
}

#[test]
fn the_timing_bar_says_how_far_each_of_its_colours_sends_the_ball() {
    let Some(mut script) = game("match", 1, true) else {
        return;
    };
    script.run("move 300 250; wait 150").unwrap();
    // In the order the colours come: early, best, late, and later still.
    assert_eq!(said(&script, "zoneFeet"), ["675", "850", "675", "500"]);
    // The verdict takes their place.
    pitch_seen(&mut script, 0, ON, |script, now| {
        if now.contains("Watching") {
            assert!(said(script, "zoneFeet").is_empty());
            assert_eq!(said(script, "verdict"), ["PERFECT"]);
        }
    });

    // Without the zinger there is nothing to say.
    let mut script = game("match", 1, false).unwrap();
    script.run("move 300 250; wait 150").unwrap();
    assert!(said(&script, "zoneFeet").is_empty());
}

#[test]
fn the_outfielders_go_back_to_the_wall_to_watch_it_over() {
    for (alone, moved) in [(false, vec![1, 4, 5]), (true, vec![3])] {
        let mut mods = mods(true);
        if alone {
            mods.push(Mod::LonePitcher);
        }
        let Some(mut script) = game_modded("match", 1, &mods) else {
            return;
        };
        let (mut first, mut last) = (Vec::new(), Vec::new());
        pitch_seen(&mut script, 0, ON, |script, now| {
            if now.contains(": Fielding") {
                last = fielders(script);
                if first.is_empty() {
                    first = last.clone();
                }
            }
        });
        let stirred: Vec<usize> = (1..=9)
            .filter(|number| first[number - 1] != last[number - 1])
            .collect();
        // Left, centre and right field, and nobody else: or, with the
        // pitcher left to do it all, the pitcher and nobody else.
        assert_eq!(stirred, moved, "alone {alone}");
        if !alone {
            // Each went up the field, away from home.
            for number in moved {
                assert!(last[number - 1].1 < first[number - 1].1, "fielder {number}");
            }
        }
    }
}

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
