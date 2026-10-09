use super::*;
use crate::play::field::Ground;

pub(super) fn pitch(thrown: Thrown, in_zone: bool) -> Pitch {
    Pitch {
        in_zone,
        thrown,
        off: None,
        quality: None,
    }
}

#[expect(clippy::unnecessary_wraps, reason = "it is handed straight to `close`")]
pub(super) fn ball(across: f32, far: f32, fly: bool) -> Option<Hit> {
    let ground = Ground::default();
    Some(Hit::at(&ground, ground.point(across, far), fly, None))
}

/// A side that has had six turns: a strikeout looking, a walk, a
/// double with the walk aboard, a home run, a sacrifice fly and a
/// ground ball for two outs.
fn side() -> Side {
    let mut side = Side::default();
    side.come_up(1, 0, 0, [false; 3]);
    side.pitch(pitch(Thrown::Swinging, true));
    side.pitch(pitch(Thrown::Foul, false));
    side.pitch(pitch(Thrown::Called, true));
    side.close(End::Strikeout, None, 0, 1);
    side.come_up(1, 1, 1, [false; 3]);
    for _ in 0..4 {
        side.pitch(pitch(Thrown::Ball, false));
    }
    side.close(End::Walk, None, 0, 0);
    side.come_up(1, 2, 1, [true, false, false]);
    side.pitch(pitch(Thrown::InPlay, true));
    side.close(End::Double, ball(0.1, 700.0, true), 0, 0);
    side.come_up(1, 3, 1, [false, true, true]);
    side.pitch(pitch(Thrown::Ball, false));
    side.pitch(pitch(Thrown::InPlay, true));
    side.close(End::HomeRun, ball(0.5, 900.0, true), 3, 0);
    side.come_up(1, 4, 1, [false, false, true]);
    side.pitch(pitch(Thrown::InPlay, false));
    side.close(End::SacrificeFly, ball(0.9, 650.0, true), 1, 1);
    side.come_up(1, 5, 2, [true, false, false]);
    side.pitch(pitch(Thrown::InPlay, true));
    side.close(End::GroundOut, ball(0.3, 300.0, false), 0, 1);
    side.runs = [0, 1, 1, 1, 0, 0, 0, 0, 0];
    side.left = vec![1];
    side
}

#[test]
fn a_sides_turns_add_up_as_a_scorer_would_add_them() {
    let figures = side().figures();
    assert_eq!(figures.turns, 6);
    // A walk and a sacrifice are not at-bats.
    assert_eq!(figures.at_bats, 4);
    assert_eq!(
        (figures.hits, figures.doubles, figures.home_runs),
        (2, 1, 1)
    );
    assert_eq!(figures.total_bases, 6);
    assert_eq!((figures.runs, figures.runs_in, figures.left), (3, 4, 1));
    assert_eq!(
        (figures.walks, figures.strikeouts, figures.sacrifices),
        (1, 1, 1)
    );
    assert_eq!(average(figures.average()), ".500");
    // Two hits and a walk in four at-bats, a walk and a sacrifice.
    assert_eq!(average(figures.on_base()), ".500");
    assert_eq!(average(figures.slugging()), "1.500");
    assert_eq!(average(figures.on_base_plus_slugging()), "2.000");
    // The double is the one hit that stayed in the park, of three
    // balls that did.
    assert_eq!(average(figures.in_play_average()), ".333");
    // With a runner on second or third: the home run, in one at-bat.
    assert_eq!((figures.chances, figures.chances_taken), (1, 1));
    assert_eq!(figures.two_out_runs, 0);
}

#[test]
fn the_pitches_are_counted_by_what_came_of_them() {
    let figures = side().figures();
    assert_eq!(figures.pitches, 12);
    // Five balls, and seven strikes of one kind or another.
    assert_eq!(figures.strikes, 7);
    assert_eq!(percent(figures.strike_rate()), "58%");
    assert_eq!((figures.called, figures.swinging, figures.fouls), (1, 1, 1));
    // Six swings, of which one missed.
    assert_eq!(figures.swings, 6);
    assert_eq!(percent(figures.contact_rate()), "83%");
    assert_eq!(percent(figures.miss_rate()), "17%");
    // Seven pitches outside the zone, and two of them swung at.
    assert_eq!((figures.outside, figures.chases), (7, 2));
    assert_eq!(percent(figures.chase_rate()), "29%");
    assert_eq!(tenths(figures.pitches_a_turn()), "2.0");
    assert_eq!(percent(figures.strikeout_rate()), "17%");
}

#[test]
fn balls_in_play_are_counted_by_where_they_went() {
    let figures = side().figures();
    assert_eq!(
        (figures.in_play, figures.flies, figures.grounders),
        (4, 3, 1)
    );
    assert_eq!(figures.thirds, [2, 1, 1]);
    assert_eq!(figures.longest, 439);
    assert_eq!(figures.feet, 341 + 439 + 317 + 146);
}

#[test]
fn one_batters_figures_are_his_own() {
    let side = side();
    let fourth = side.figures_of(3);
    assert_eq!((fourth.at_bats, fourth.hits, fourth.home_runs), (1, 1, 1));
    assert_eq!((fourth.runs, fourth.runs_in), (1, 3));
    assert_eq!(average(fourth.average()), "1.000");
    // A batter who has not been up has no average.
    assert_eq!(average(side.figures_of(8).average()), "---");
    assert_eq!(percent(side.figures_of(8).strike_rate()), "-");
    assert_eq!(side.hits_in(1), 2);
    assert_eq!(side.hits_in(2), 0);
}

#[test]
fn a_base_stolen_is_the_runners_and_is_told_where_it_happened() {
    let mut side = Side::default();
    side.come_up(1, 0, 0, [false; 3]);
    for _ in 0..4 {
        side.pitch(pitch(Thrown::Ball, false));
    }
    side.close(End::Walk, None, 0, 0);
    // He steals second while the next man is up, who strikes out.
    side.come_up(1, 1, 0, [true, false, false]);
    side.pitch(pitch(Thrown::Called, true));
    side.stole(1, 0, 2, true);
    side.pitch(pitch(Thrown::Swinging, true));
    side.pitch(pitch(Thrown::Swinging, true));
    side.close(End::Strikeout, None, 0, 1);
    // And is thrown out going for third before the one after grounds
    // out.
    side.come_up(1, 2, 1, [false, true, false]);
    side.stole(1, 0, 3, false);
    side.pitch(pitch(Thrown::InPlay, true));
    side.close(End::GroundOut, ball(0.3, 300.0, false), 0, 1);
    let figures = side.figures();
    assert_eq!((figures.stolen, figures.caught), (1, 1));
    assert_eq!(figures.stolen_of(), "1 OF 2");
    assert_eq!(side.figures_of(0).stolen_of(), "1 OF 2");
    assert_eq!(side.figures_of(1).stolen_of(), "0 OF 0");
    // Three are out, one of them on the bases.
    assert_eq!(side.outs(), 3);
    let told: Vec<String> = side.told(1).into_iter().map(|(line, _)| line).collect();
    assert_eq!(
        told,
        [
            "1 WALKED (4)",
            "1 STOLE SECOND",
            "2 STRUCK OUT SWINGING (3)",
            "1 CAUGHT STEALING THIRD",
            "3 GROUNDED OUT TO SHORT (1)",
        ]
    );
    assert!(side.told(2).is_empty());
}

#[test]
fn a_turn_is_told_in_a_line() {
    let side = side();
    let lines: Vec<String> = side.turns.iter().map(Turn::words).collect();
    assert_eq!(
        lines,
        [
            "1 STRUCK OUT LOOKING (3)",
            "2 WALKED (4)",
            "3 DOUBLE TO LEFT (1)",
            "4 HOME RUN TO CENTRE, 439 FT, 3 RUNS (2)",
            "5 SACRIFICE FLY TO RIGHT, 1 RUN (1)",
            "6 GROUNDED OUT TO SHORT (1)",
        ]
    );
}

#[test]
fn a_turn_that_is_not_begun_takes_no_pitches_and_one_begun_can_be_dropped() {
    let mut side = Side::default();
    side.pitch(pitch(Thrown::Ball, false));
    side.close(End::Walk, None, 0, 0);
    assert!(side.turns.is_empty());
    side.come_up(2, 0, 0, [false; 3]);
    side.pitch(pitch(Thrown::Ball, false));
    side.abandon();
    side.close(End::Walk, None, 0, 0);
    assert!(side.turns.is_empty());
}
