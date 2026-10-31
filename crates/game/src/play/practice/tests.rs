use super::*;

/// A ball that came down this far off, this far across the field.
fn sent(feet: u32, across: f32) -> Sent {
    Sent {
        hit: Hit {
            at: (0.0, 0.0),
            across,
            feet,
            fly: feet > 150,
            deep: feet > 150,
        },
        foul: false,
        over_wall: feet >= 400,
    }
}

/// A swing that began `off` frames from the best moment and sent the ball
/// off as `sent` says, met this well.
fn hit(off: i32, quality: Quality, sent: Sent) -> Seen {
    Seen {
        in_zone: true,
        swung: true,
        off: Some(off),
        quality: Some(quality),
        sent: Some(sent),
    }
}

/// A swing that began `off` frames from the best moment and missed.
fn miss(off: i32, in_zone: bool) -> Seen {
    Seen {
        in_zone,
        swung: true,
        off: Some(off),
        quality: None,
        sent: None,
    }
}

/// A pitch let go by.
fn taken(in_zone: bool) -> Seen {
    Seen {
        in_zone,
        swung: false,
        off: None,
        quality: None,
        sent: None,
    }
}

fn session(seen: &[Seen]) -> Session {
    let mut session = Session::new(true);
    for &one in seen {
        session.add(one);
    }
    session
}

#[test]
fn a_session_with_nothing_in_it_has_nothing_to_say_of_the_last_pitch() {
    let empty = session(&[]);
    assert_eq!(empty.pitches(), 0);
    assert_eq!(empty.longest(), None);
    assert_eq!(empty.last(), None);
    assert!(empty.timed().is_empty());
    assert_eq!(empty.tally(), "PITCHES 0  MET 0  SWEET 0");
}

#[test]
fn every_pitch_counts_and_only_a_swing_can_meet_the_ball() {
    let all = session(&[
        hit(0, Quality::Good, sent(300, 0.5)),
        hit(2, Quality::Medium, sent(120, 0.1)),
        miss(-7, true),
        taken(true),
        taken(false),
    ]);
    assert_eq!(all.pitches(), 5);
    assert_eq!(all.swings(), 3);
    assert_eq!(all.met(), 2);
    assert_eq!(all.sweet(), 1);
    assert_eq!(all.tally(), "PITCHES 5  MET 2  SWEET 1  BEST 300 FT");
}

#[test]
fn a_foul_is_no_ones_longest_ball() {
    let mut far = sent(390, 1.2);
    far.foul = true;
    let all = session(&[
        hit(0, Quality::Good, far),
        hit(0, Quality::Good, sent(200, 0.5)),
    ]);
    assert_eq!(all.longest(), Some(200));
    assert_eq!(all.fair().count(), 1);
}

#[test]
fn the_last_pitch_is_told_by_what_came_of_it_and_when_the_swing_was() {
    let told = |seen: Seen| session(&[seen]).last().unwrap();
    assert_eq!(
        told(hit(-2, Quality::Medium, sent(312, 0.5))),
        "LAST 312 FT TO CENTRE, 2 FRAMES EARLY"
    );
    assert_eq!(
        told(hit(1, Quality::Good, sent(404, 0.1))),
        "LAST 404 FT OVER THE WALL, 1 FRAME LATE"
    );
    let mut foul = sent(250, 1.2);
    foul.foul = true;
    assert_eq!(told(hit(0, Quality::Good, foul)), "LAST FOUL, ON TIME");
    assert_eq!(told(miss(5, true)), "LAST MISSED, 5 FRAMES LATE");
    assert_eq!(told(taken(true)), "LAST LET GO BY, A STRIKE");
    assert_eq!(told(taken(false)), "LAST LET GO BY, A BALL");
}

#[test]
fn a_view_with_no_strike_zone_says_nothing_of_balls_and_strikes() {
    let mut session = Session::new(false);
    session.add(taken(true));
    assert_eq!(session.last().unwrap(), "LAST LET GO BY");
    assert_eq!(session.taking_line(), None);
}

#[test]
fn a_swing_with_no_moment_to_be_early_or_late_for_is_not_timed() {
    let no_moment = Seen {
        off: None,
        ..miss(0, true)
    };
    let all = session(&[no_moment, miss(3, true), taken(true)]);
    assert_eq!(all.swings(), 2);
    assert_eq!(all.timed(), vec![(3, None, false)]);
    assert_eq!(all.last(), session(&[taken(true)]).last());
}

#[test]
fn the_report_says_where_the_balls_went() {
    let mut foul = sent(200, 1.3);
    foul.foul = true;
    let all = session(&[
        hit(0, Quality::Good, sent(400, 0.1)),
        hit(0, Quality::Good, sent(300, 0.5)),
        hit(0, Quality::Medium, sent(100, 0.9)),
        hit(0, Quality::Medium, sent(200, 0.9)),
        hit(0, Quality::Poor, foul),
        taken(true),
    ]);
    assert_eq!(
        all.sent_lines(),
        [
            "OVER THE WALL 1",
            "IN PLAY 3",
            "FOUL 1",
            "TO LEFT 25%",
            "TO CENTRE 25%",
            "TO RIGHT 50%",
            "IN THE AIR 3",
            "ON THE GROUND 1",
            "LONGEST 400 FT",
            "USUALLY 250 FT",
        ]
    );
}

#[test]
fn the_report_says_dashes_where_nothing_was_hit() {
    let lines = session(&[taken(true)]).sent_lines();
    assert!(lines.contains(&"TO LEFT -".to_owned()), "{lines:?}");
    assert!(lines.contains(&"LONGEST -".to_owned()), "{lines:?}");
    assert!(lines.contains(&"USUALLY -".to_owned()), "{lines:?}");
}

#[test]
fn the_report_says_what_was_let_go_by_and_what_was_chased() {
    let all = session(&[
        taken(true),
        taken(false),
        taken(false),
        miss(0, false),
        miss(0, true),
    ]);
    assert_eq!(
        all.taking_line().unwrap(),
        "LET GO BY 3: BALLS 2, STRIKES 1   SWUNG AT BALLS 1"
    );
}
