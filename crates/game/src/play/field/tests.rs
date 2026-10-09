use proptest::prelude::*;

use super::*;
use crate::rules::Rules;

const HOME: Point = (240.8, 336.85);
const STRAIGHT: Point = (303.8, 168.7);

fn well_hit() -> Contact {
    Contact {
        power: 14.0,
        under: 0.0,
        aside: 0.0,
    }
}

fn fly(contact: &Contact, mark: Point) -> (Vec<Happened>, Ball) {
    let rules = Rules::default();
    let mut ball = Ball::hit(HOME, mark, contact, &rules.hit, &rules.field);
    let mut seen = Vec::new();
    for _ in 0..900 {
        let happened = ball.step(HOME, contact.miss(), &rules.field);
        if happened != Happened::Nothing {
            seen.push(happened);
        }
    }
    (seen, ball)
}

#[test]
fn a_ball_met_squarely_clears_the_wall() {
    let (seen, _) = fly(&well_hit(), STRAIGHT);
    assert_eq!(seen.first(), Some(&Happened::Cleared), "{seen:?}");
}

#[test]
fn a_weak_hit_lands_in_the_field_and_stays_there() {
    let weak = Contact {
        power: 30.0,
        under: -30.0,
        aside: 0.0,
    };
    let (seen, ball) = fly(&weak, STRAIGHT);
    assert_eq!(seen.first(), Some(&Happened::Landed), "{seen:?}");
    assert!(!seen.contains(&Happened::Cleared));
    assert!(ball.bounced);
    let rules = Rules::default();
    assert!(reach(HOME, ball.at) < rules.field.wall);
}

#[test]
fn the_landing_is_where_the_ball_first_comes_down() {
    let rules = Rules::default();
    let weak = Contact {
        power: 25.0,
        under: -20.0,
        aside: 0.0,
    };
    let start = Ball::hit(HOME, STRAIGHT, &weak, &rules.hit, &rules.field);
    let landing = start.landing(HOME, weak.miss(), &rules.field);
    let mut ball = start;
    while ball.step(HOME, weak.miss(), &rules.field) == Happened::Nothing {}
    assert_eq!(ball.at, landing);
    // It went up the field, away from home.
    assert!(landing.1 < HOME.1);
}

#[test]
fn a_ball_sent_a_distance_comes_down_that_far_off_whichever_way_it_goes() {
    let rules = Rules::default();
    for across in [-40.0, 120.0, STRAIGHT.0, 480.0, 620.0] {
        for (frames, peak) in [(150, 100.0), (90, 30.0), (420, 300.0)] {
            let mark = (across, STRAIGHT.1);
            let mut ball = Ball::sent(HOME, mark, 600.0, frames as f32, peak);
            let (mut taken, mut highest) = (1, 0.0f32);
            while ball.step(HOME, 0.0, &rules.field) != Happened::Landed {
                taken += 1;
                highest = highest.max(ball.height);
                assert!(taken < 900, "towards {across}: it never came down");
            }
            assert_eq!(taken, frames, "towards {across}");
            assert!((highest - peak).abs() < peak * 0.05, "{highest} for {peak}");
            let far = reach(HOME, ball.at);
            assert!((far - 600.0).abs() < 1.0, "towards {across}: {far}");
        }
    }
}

#[test]
fn swinging_under_the_ball_lifts_it_more() {
    let rules = Rules::default();
    let level = well_hit();
    let under = Contact {
        under: 40.0,
        ..level
    };
    assert!(under.lift(&rules.hit) > level.lift(&rules.hit));
}

#[test]
fn a_ball_never_turns_round_in_the_air() {
    // Far off-centre, the air takes all the ball's speed but no more.
    let mishit = Contact {
        power: 14.0,
        under: 200.0,
        aside: 0.0,
    };
    let rules = Rules::default();
    let mut ball = Ball::hit(HOME, STRAIGHT, &mishit, &rules.hit, &rules.field);
    for _ in 0..300 {
        ball.step(HOME, mishit.miss(), &rules.field);
        if !ball.walled {
            assert!(ball.speed.1 <= 0.0, "{ball:?}");
        }
    }
}

#[test]
fn a_point_of_the_ground_is_found_from_how_far_across_and_how_far_off_it_is() {
    let ground = Ground::default();
    for across in [0.0, 0.2, 0.5, 0.9, 1.0] {
        for far in [150.0, 440.0, 700.0, 820.0, 950.0] {
            let at = ground.point(across, far);
            assert!((reach(ground.home, at) - far).abs() < 0.05, "{at:?}");
            assert!((ground.across(at) - across).abs() < 0.001, "{at:?}");
        }
    }
    // The wall is four hundred feet off, and second base about half
    // that.
    assert_eq!(ground.feet(ground.point(0.5, ground.wall)), 400);
    assert_eq!(ground.feet(ground.point(0.5, ground.infield)), 215);
    // Second base as the art places it is where the infield ends.
    assert!((reach(ground.home, (277.9, 215.75)) - ground.infield).abs() < 2.0);
    assert_eq!(ground.feet(ground.home), 0);
}

#[test]
fn a_fielder_faces_the_way_he_is_going() {
    let from = (100.0, 100.0);
    assert_eq!(Facing::towards(from, (200.0, 100.0)), Facing::Right);
    assert_eq!(Facing::towards(from, (0.0, 100.0)), Facing::Left);
    assert_eq!(Facing::towards(from, (100.0, 0.0)), Facing::Up);
    assert_eq!(Facing::towards(from, (100.0, 200.0)), Facing::Down);
    assert_eq!(Facing::towards(from, (200.0, 0.0)), Facing::UpRight);
    assert_eq!(Facing::towards(from, (0.0, 200.0)), Facing::DownLeft);
    assert_eq!(Facing::UpLeft.pick_label(), "pickLeft");
    assert_eq!(Facing::DownRight.throw_label(), "throwRight");
}

proptest! {
    #[test]
    fn a_ball_sent_to_come_down_so_far_off_after_so_many_frames_does_just_that(
        // Towards anywhere between the foul lines, to come down short of
        // the wall, which would get in its way.
        across in 0.0f32..=1.0,
        carry in 100.0f32..800.0,
        frames in 4u16..=600,
        peak in 5.0f32..400.0,
    ) {
        let (rules, ground) = (Rules::default().field, Ground::default());
        let (left, right) = ground.foul;
        let mark = (left + (right - left) * across, ground.mark_y);
        let mut ball = Ball::sent(ground.home, mark, carry, f32::from(frames), peak);
        let (mut taken, mut highest) = (0, 0.0f32);
        loop {
            taken += 1;
            // Nothing happens to it on the way but coming down.
            match ball.step(ground.home, 0.0, &rules) {
                Happened::Landed => break,
                happened => prop_assert_eq!(happened, Happened::Nothing),
            }
            highest = highest.max(ball.height);
            prop_assert!(taken < frames, "still up after its {} frames", frames);
        }
        prop_assert_eq!(taken, frames);
        let far = reach(ground.home, ball.at);
        prop_assert!((far - carry).abs() < 1.0, "it came down {} off", far);
        // The top of its flight comes between two frames, so the
        // highest it is seen is one part short of the height it was
        // sent to, in as many parts as the frames it is up for.
        let seen = peak * (1.0 - 1.0 / f32::from(frames));
        prop_assert!((highest - seen).abs() < peak * 0.001, "it went {} high", highest);
    }

    #[test]
    fn any_point_of_the_ground_is_as_far_across_and_as_far_off_as_was_asked_for(
        across in 0.0f32..=1.0,
        far in 50.0f32..1200.0,
    ) {
        let ground = Ground::default();
        let at = ground.point(across, far);
        let (is_across, is_far) = (ground.across(at), reach(ground.home, at));
        prop_assert!((is_across - across).abs() < 0.001, "{} across", is_across);
        prop_assert!((is_far - far).abs() < 0.1, "{} off", is_far);
    }
}
