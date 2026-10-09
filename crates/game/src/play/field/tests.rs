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

/// A hit by this contact, frame by frame until it comes down or comes to
/// the wall: how much of the field it has covered, how high it is, and
/// what happened to it.
fn flight(contact: &Contact) -> Vec<(f32, f32, Happened)> {
    let rules = Rules::default();
    let mut ball = Ball::hit(HOME, STRAIGHT, contact, &rules.hit, &rules.field);
    let mut frames = Vec::new();
    for _ in 0..900 {
        let happened = ball.step(HOME, contact.miss(), &rules.field);
        frames.push((covered(HOME, ball.at), ball.height, happened));
        if happened != Happened::Nothing {
            break;
        }
    }
    frames
}

#[test]
fn the_same_swing_goes_as_far_whichever_way_it_is_sent() {
    let (rules, ground) = (Rules::default().field, Ground::default());
    // Timed well and badly, with the ring on the ball, under it and over.
    for power in [14.0, 17.0, 25.0] {
        for under in [0.0, 5.0, 20.0, -12.0] {
            let swing = |aside: f32| Contact {
                power,
                under,
                aside,
            };
            let straight = flight(&swing(0.0));
            // From the left foul line to the right.
            for across in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let mark = ground.foul.0 + across * (ground.foul.1 - ground.foul.0);
                let sent = flight(&swing((mark - STRAIGHT.0) * rules.aim_share));
                let told = format!("power {power}, {under} under, {across} across");
                // It is in the air as long, and the same comes of it.
                assert_eq!(sent.len(), straight.len(), "{told}");
                for (frame, (sent, straight)) in sent.iter().zip(&straight).enumerate() {
                    let (far, height, happened) = *sent;
                    assert!((far - straight.0).abs() < 0.5, "{told}, frame {frame}");
                    assert!((height - straight.1).abs() < 0.001, "{told}, frame {frame}");
                    assert_eq!(happened, straight.2, "{told}, frame {frame}");
                }
            }
        }
    }
}

#[test]
fn a_hit_that_goes_straight_flies_as_it_did_when_the_screen_was_the_measure() {
    // The original had the ball cross the screen to its mark in its frames,
    // and took from its speed by how far across the screen it was from
    // home, squared, over 1500. Straight up the middle that is the flight
    // there is now, which is how the number the rules have was come by.
    let rules = Rules::default();
    for power in [14.0, 17.0, 25.0] {
        for under in [0.0, 5.0, 20.0, -12.0, 60.0] {
            let contact = Contact {
                power,
                under,
                aside: 0.0,
            };
            let mut ball = Ball::hit(HOME, STRAIGHT, &contact, &rules.hit, &rules.field);
            let frames = power * rules.field.pace;
            let mut was_at = HOME;
            let mut speed = (
                (STRAIGHT.0 - HOME.0) / frames,
                (STRAIGHT.1 - HOME.1) / frames,
            );
            for frame in 0..900 {
                was_at = (was_at.0 + speed.0, was_at.1 + speed.1);
                let far = distance(HOME, was_at);
                let lost = (0.002 * (contact.miss() / 40.0) * (far * far / 1500.0)).min(1.0);
                speed = (speed.0 - speed.0 * lost, speed.1 - speed.1 * lost);
                let happened = ball.step(HOME, contact.miss(), &rules.field);
                let off = distance(ball.at, was_at);
                assert!(
                    off < 0.02,
                    "power {power}, {under} under, frame {frame}: {off}"
                );
                if happened != Happened::Nothing {
                    break;
                }
            }
        }
    }
}

#[test]
fn a_hit_sent_to_one_side_crosses_more_of_the_screen_and_no_more_of_the_field() {
    let (rules, ground) = (Rules::default(), Ground::default());
    let to = |mark: f32| Contact {
        aside: (mark - STRAIGHT.0) * rules.field.aim_share,
        ..well_hit()
    };
    let sent = |mark: f32| Ball::hit(HOME, STRAIGHT, &to(mark), &rules.hit, &rules.field);
    let (left, middle, right) = (sent(ground.foul.0), sent(STRAIGHT.0), sent(ground.foul.1));
    let pace = |ball: &Ball| distance((0.0, 0.0), ball.speed);
    // Right field is drawn across more of the screen than left, and
    // either across more than the way up the middle.
    assert!(pace(&right) > pace(&left) && pace(&left) > pace(&middle));
    for ball in [left, middle, right] {
        let next = (HOME.0 + ball.speed.0, HOME.1 + ball.speed.1);
        let each = covered(HOME, next);
        assert!((each - covered(HOME, STRAIGHT) / (14.0 * rules.field.pace)).abs() < 0.001);
    }
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
