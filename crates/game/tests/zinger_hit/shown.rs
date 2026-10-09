//! What is seen and heard of a zinger: the call, the feet counted up,
//! the place named, the ball and its trail, the crowd, and the fielders.

use bb_game::art::all_named;
use bb_game::mods::Mod;

use super::{feet, feet_said, fielders, game, mods, timed_to_the_run};
use crate::common::{ON, game_modded, next, pitch, pitch_seen, said, sounds};

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
