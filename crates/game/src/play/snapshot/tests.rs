use super::*;

fn a_match(phase: Phase) -> Snapshot {
    Snapshot {
        phase,
        standing: Standing::Match {
            score: Score::Of {
                score: 0,
                target: 3,
            },
            outs: 0,
            balls: 0,
            strikes: 0,
            bases: [false; 3],
            pitched: 1,
            innings: None,
        },
        pitch: Some(a_pitch()),
        mods: ModsSeen::default(),
    }
}

fn a_pitch() -> PitchSeen {
    PitchSeen {
        crosses: (309.4, 196.6),
        frames: 66,
        in_zone: true,
        best: None,
        zinger_feet: None,
        mystery: None,
        golden: false,
        rebounds: 0,
        called: None,
        came_down: None,
    }
}

#[test]
fn a_match_with_no_mods_says_the_score_the_count_and_the_pitch() {
    assert_eq!(
        a_match(Phase::Flight { step: 25 }).to_string(),
        "Flight { step: 25 }, score 0 of 3, outs 0, count 0-0, bases ---, pitched 1, \
         crossing 309,197 after 66 frames"
    );
}

#[test]
fn a_game_between_pitches_says_nothing_of_a_pitch() {
    let between = Snapshot {
        pitch: None,
        ..a_match(Phase::Arriving)
    };
    assert_eq!(
        between.to_string(),
        "Arriving, score 0 of 3, outs 0, count 0-0, bases ---, pitched 1"
    );
}

#[test]
fn the_bases_are_marked_from_first_to_third() {
    let mut snapshot = a_match(Phase::Ready);
    let Standing::Match {
        bases,
        outs,
        balls,
        strikes,
        ..
    } = &mut snapshot.standing
    else {
        unreachable!("a match was made");
    };
    (*bases, *outs, *balls, *strikes) = ([true, false, true], 2, 3, 1);
    snapshot.pitch = None;
    assert_eq!(
        snapshot.to_string(),
        "Ready, score 0 of 3, outs 2, count 3-1, bases x-x, pitched 1"
    );
}

#[test]
fn everything_about_a_pitch_is_said_in_its_order() {
    let pitch = PitchSeen {
        in_zone: false,
        best: Some((42, 43)),
        zinger_feet: Some(497),
        mystery: Some(Kind::ChangeUp),
        golden: true,
        rebounds: 2,
        called: Some((200.0, 160.4)),
        came_down: Some((310.5, 88.0)),
        ..a_pitch()
    };
    assert_eq!(
        pitch.to_string(),
        ", crossing 309,197 after 66 frames outside the zone, best swung on steps 42 to 43, \
         a zinger of 497 feet, mystery change-up, golden, rebounds 2, called 200,160, \
         came down at 310,88"
    );
}

#[test]
fn every_mod_with_something_to_tell_tells_it_in_its_order() {
    let mods = ModsSeen {
        let_go: 3,
        heat: 2,
        hits_in_a_row: 4,
        rally: 1,
        clutch: true,
        southpaw: true,
        bullet_time: Some((60, true)),
        sign_lit: Some(1),
        sign_struck: Some((0, 3)),
        stealing: vec![2, 3],
        stolen: 1,
        caught: 2,
        arm: Some(ArmSeen {
            thrown: 12,
            tired: 0.1,
            relieved: 1,
        }),
        shifted: -0.125,
    };
    assert_eq!(
        mods.to_string(),
        ", let go 3, heat 2, hits in a row 4, rally 1, clutch, southpaw, \
         bullet time 60 slowed, sign 2 lit, struck sign 1 for 3, stealing 2, stealing 3, \
         stolen 1, caught 2, arm 12 tired 0.10, pitcher 2, shifted left 0.12"
    );
}

#[test]
fn mods_with_nothing_to_tell_say_nothing() {
    assert_eq!(ModsSeen::default().to_string(), "");
    // A fresh arm is told of, and a first pitcher is not numbered.
    let fresh = ModsSeen {
        arm: Some(ArmSeen {
            thrown: 0,
            tired: 0.0,
            relieved: 0,
        }),
        shifted: 0.25,
        bullet_time: Some((120, false)),
        ..ModsSeen::default()
    };
    assert_eq!(
        fresh.to_string(),
        ", bullet time 120, arm 0 tired 0.00, shifted right 0.25"
    );
}

#[test]
fn a_full_match_says_the_innings_both_scores_and_how_it_has_gone() {
    let snapshot = Snapshot {
        standing: Standing::Match {
            score: Score::Against {
                batting_in: "top of innings 1".to_owned(),
                score: 0,
                theirs: 0,
            },
            outs: 0,
            balls: 0,
            strikes: 0,
            bases: [false; 3],
            pitched: 1,
            innings: Some("away".to_owned()),
        },
        mods: ModsSeen {
            heat: 1,
            ..ModsSeen::default()
        },
        ..a_match(Phase::Flight { step: 2 })
    };
    assert_eq!(
        snapshot.to_string(),
        "Flight { step: 2 }, top of innings 1, score 0 to 0, outs 0, count 0-0, bases ---, \
         pitched 1, crossing 309,197 after 66 frames, heat 1, away"
    );
}

#[test]
fn the_arcade_game_says_its_points_its_pitches_and_only_bullet_time() {
    let snapshot = Snapshot {
        phase: Phase::Settling { left: 92 },
        standing: Standing::Arcade {
            points: 50,
            pitches_left: 10,
        },
        pitch: Some(PitchSeen {
            best: Some((42, 43)),
            ..a_pitch()
        }),
        // Anything else is left out, whatever it holds.
        mods: ModsSeen {
            heat: 3,
            southpaw: true,
            bullet_time: Some((120, false)),
            ..ModsSeen::default()
        },
    };
    assert_eq!(
        snapshot.to_string(),
        "Settling { left: 92 }, 50 points, 10 pitches left, crossing 309,197 after 66 frames, \
         best swung on steps 42 to 43, bullet time 120"
    );
}
