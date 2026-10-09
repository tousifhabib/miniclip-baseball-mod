use proptest::prelude::*;

use super::place::Place;
use super::*;
use crate::play::field::Happened;
use crate::play::pitch::meets;

/// The fixed points of the field as they are in the game's art: home,
/// how far up the field a hit is aimed, and the two foul lines there.
const HOME: Point = (240.8, 336.85);
const MARK: f32 = 168.7;
const FOUL: (f32, f32) = (-54.65, 638.3);

const LEVELS: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard];
/// Where the ring might be held: on the ball, well above it, well
/// below it, off to one side, and a little out every way.
const RINGS: [Point; 6] = [
    (0.0, 0.0),
    (0.0, -90.0),
    (0.0, 90.0),
    (70.0, 0.0),
    (-20.0, 25.0),
    (15.0, -30.0),
];

fn zinger(difficulty: Difficulty, frames: u32, ring: Point, rules: &Rules) -> Zinger {
    let table = rules.pitch.at(difficulty);
    Zinger::of(table, frames, ring, HOME, difficulty, rules).unwrap()
}

/// Every kind of zinger the rules allow: one for each frame of each
/// level's window, with the ring held each of those ways.
fn zingers(rules: &Rules) -> Vec<Zinger> {
    let mut all = Vec::new();
    for difficulty in LEVELS {
        for &(frames, ..) in &rules.pitch.at(difficulty).window {
            for ring in RINGS {
                all.push(zinger(difficulty, frames, ring, rules));
            }
        }
    }
    all
}

/// Places across the field to send a ball, from one foul line to the
/// other.
fn across() -> impl Iterator<Item = f32> {
    (0..=20).map(|step| FOUL.0 + (FOUL.1 - FOUL.0) * step as f32 / 20.0)
}

/// Flies a zinger towards `across`. Returns the frame on which it went
/// over the wall, the frame on which it came down, and where that was.
fn fly(zinger: &Zinger, across: f32, rules: &Rules) -> (u32, u32, Point) {
    let mut ball = zinger.ball(HOME, (across, MARK));
    let (mut frames, mut over) = (0, None);
    loop {
        frames += 1;
        assert!(frames < 5000, "{zinger:?} never came down");
        match ball.step(HOME, 0.0, &rules.field) {
            Happened::Nothing => {}
            Happened::Cleared => over = Some(frames),
            Happened::HitWall => panic!("{zinger:?} towards {across} hit the wall"),
            Happened::Landed => {
                let over = over.unwrap_or_else(|| panic!("{zinger:?} fell short"));
                return (over, frames, ball.at);
            }
        }
    }
}

#[test]
fn a_miss_is_no_zinger() {
    let rules = Rules::default();
    let easy = rules.pitch.at(Difficulty::Easy);
    let of = |frames| Zinger::of(easy, frames, (0.0, 0.0), HOME, Difficulty::Easy, &rules);
    assert_eq!(of(6), None);
    assert!(of(7).is_some());
}

#[test]
fn every_zinger_clears_the_wall_wherever_it_is_sent_and_however_it_was_hit() {
    let rules = Rules::default();
    for zinger in zingers(&rules) {
        for across in across() {
            let (over, down, at) = fly(&zinger, across, &rules);
            // The view has changed to the field by the time it goes
            // over, so that it is seen to go.
            assert!(over > rules.hit.watch, "{zinger:?} towards {across}");
            assert!(down > over);
            // It comes down where it was sent, and well inside the
            // longest a play may last.
            let far = reach(HOME, at);
            assert!((far - zinger.carry).abs() < 1.0, "{zinger:?}: {far}");
            assert!(down < rules.field.longest / 2, "{zinger:?}: {down}");
        }
    }
}

#[test]
fn the_nearer_the_best_the_swing_the_further_the_ball_goes() {
    let rules = Rules::default();
    for difficulty in LEVELS {
        let table = rules.pitch.at(difficulty);
        // The worst the ring can do for a better-timed swing still
        // beats the best it can do for a worse one.
        let mut by_timing: Vec<(f32, u32, u32)> = table
            .window
            .iter()
            .map(|&(frames, ..)| {
                let on = zinger(difficulty, frames, (0.0, 0.0), &rules);
                let off = zinger(difficulty, frames, (200.0, 0.0), &rules);
                (on.timed, off.feet, on.feet)
            })
            .collect();
        by_timing.sort_by(|a, b| a.0.total_cmp(&b.0));
        for pair in by_timing.windows(2) {
            if pair[0].0 < pair[1].0 {
                assert!(pair[0].2 < pair[1].1, "{difficulty:?}: {pair:?}");
            }
        }
        let (worst, best) = (by_timing[0], by_timing[by_timing.len() - 1]);
        assert_eq!((worst.0, best.0), (0.0, 1.0));
        assert_eq!(worst.1, 440, "{difficulty:?}");
    }
}

#[test]
fn the_harder_the_level_the_further_the_best_zinger_goes() {
    let rules = Rules::default();
    let best = |difficulty, frames| zinger(difficulty, frames, (0.0, 0.0), &rules).feet;
    assert_eq!(best(Difficulty::Easy, 11), 800);
    assert_eq!(best(Difficulty::Medium, 10), 850);
    assert_eq!(best(Difficulty::Hard, 10), 900);
}

#[test]
fn holding_the_ring_on_the_ball_adds_a_little() {
    let rules = Rules::default();
    let feet = |ring| zinger(Difficulty::Easy, 11, ring, &rules).feet;
    assert_eq!(feet((0.0, 0.0)), 800);
    assert_eq!(feet((40.0, 0.0)), 770);
    assert_eq!(feet((0.0, 80.0)), 740);
    // Further off than that takes no more away.
    assert_eq!(feet((300.0, 0.0)), 740);
}

#[test]
fn the_height_of_the_ring_shapes_the_flight() {
    let rules = Rules::default();
    let of = |under| zinger(Difficulty::Easy, 11, (0.0, under), &rules);
    let (driven, level, skied) = (of(-50.0), of(0.0), of(50.0));
    assert_eq!((driven.shape, level.shape, skied.shape), (-1.0, 0.0, 1.0));
    // A skied ball hangs two and a half times as long and goes far
    // higher. One driven low is the other way about.
    assert_eq!((level.hang, skied.hang), (200.0, 500.0));
    assert!(skied.peak > level.peak * 2.5 && driven.peak < level.peak / 2.0);
    assert!(driven.hang < level.hang);
    // Half the way there has half the effect, and further than all the
    // way has no more.
    assert_eq!(of(25.0).shape, 0.5);
    assert_eq!(of(25.0).hang, 350.0);
    assert_eq!(of(120.0).hang, skied.hang);
    // In the batting view the skied one is seen to leave more steeply.
    assert!(skied.lift(&rules.hit) > level.lift(&rules.hit));
    assert!(driven.lift(&rules.hit) < level.lift(&rules.hit));
}

#[test]
fn a_ball_driven_low_is_not_kept_under_the_top_of_the_wall() {
    let rules = Rules::default();
    // The worst-timed swing drops the ball just behind the wall, which
    // a low flight would not get over.
    let low = zinger(Difficulty::Easy, 7, (0.0, -90.0), &rules);
    assert!(low.peak > rules.zinger.drive.peak);
    let mut ball = low.ball(HOME, (303.8, MARK));
    while !ball.walled {
        ball.step(HOME, 0.0, &rules.field);
    }
    assert!(ball.height > rules.field.clear * 1.3, "{}", ball.height);
}

#[test]
fn the_ring_neither_drags_on_the_ball_nor_slows_it() {
    let rules = Rules::default();
    let contact = zinger(Difficulty::Easy, 11, (30.0, 60.0), &rules).contact(30.0);
    assert_eq!(
        (contact.under, contact.miss(), contact.aside),
        (0.0, 0.0, 30.0)
    );
}

#[test]
fn the_crowd_makes_more_of_a_better_timed_hit() {
    let rules = Rules::default();
    let sounds = |frames| zinger(Difficulty::Easy, frames, (0.0, 0.0), &rules).hit_sounds();
    let (worst, middling, best) = (sounds(7), sounds(13), sounds(11));
    assert!(worst.1.len() <= middling.1.len() && middling.1.len() < best.1.len());
    assert_ne!(worst.0, best.0);
}

#[test]
fn where_it_comes_down_is_named_by_how_far_and_which_way() {
    let rules = Rules::default().zinger;
    // The board behind the wall in the middle of the field.
    let board = Some([230.0, 20.0, 380.0, 110.0]);
    assert_eq!(
        Place::of((300.0, 90.0), 1.2, board, &rules),
        Place::Scoreboard
    );
    assert_eq!(Place::of((40.0, 150.0), 1.2, board, &rules), Place::Stands);
    assert_eq!(
        Place::of((-90.0, 100.0), 1.6, board, &rules),
        Place::OutOfThePark
    );
    // Over the board altogether.
    assert_eq!(
        Place::of((330.0, -60.0), 1.9, board, &rules),
        Place::OutOfThePark
    );
    // The arcade game's field has no board.
    assert_eq!(Place::of((300.0, 90.0), 1.2, None, &rules), Place::Stands);
    assert_ne!(Place::Stands.words(), Place::OutOfThePark.words());
}

/// Anywhere the ring might be held about the ball: up to sixty pixels
/// to either side of it, and as far above or below.
fn any_ring() -> impl Strategy<Value = Point> {
    (-60.0f32..=60.0, -60.0f32..=60.0)
}

fn any_level() -> impl Strategy<Value = Difficulty> {
    prop::sample::select(&LEVELS[..])
}

proptest! {
    #[test]
    fn there_is_a_zinger_exactly_when_the_swing_meets_the_ball(
        frames in 0u32..25,
        ring in any_ring(),
        level in any_level(),
    ) {
        let rules = Rules::default();
        let table = rules.pitch.at(level);
        let zinger = Zinger::of(table, frames, ring, HOME, level, &rules);
        prop_assert_eq!(zinger.is_some(), meets(table, frames).is_some());
    }

    #[test]
    fn with_the_ring_held_the_same_a_better_timed_swing_never_carries_less(
        // Two frames of the level's window, whichever they are.
        (one, other) in any::<(prop::sample::Index, prop::sample::Index)>(),
        ring in any_ring(),
        level in any_level(),
    ) {
        let rules = Rules::default();
        let window = &rules.pitch.at(level).window;
        let of = |frame: prop::sample::Index| zinger(level, frame.get(window).0, ring, &rules);
        let (one, other) = (of(one), of(other));
        let (worse, better) = if one.timed <= other.timed {
            (one, other)
        } else {
            (other, one)
        };
        prop_assert!(worse.carry <= better.carry, "{:?} and {:?}", worse, better);
        prop_assert!(worse.feet <= better.feet, "{:?} and {:?}", worse, better);
    }

    #[test]
    fn any_zinger_goes_over_the_wall_and_comes_down_where_it_was_sent(
        frame: prop::sample::Index,
        ring in any_ring(),
        level in any_level(),
        // Sent anywhere from the one foul line to the other.
        across in 0.0f32..=1.0,
    ) {
        let rules = Rules::default();
        let (frames, ..) = *frame.get(&rules.pitch.at(level).window);
        let zinger = zinger(level, frames, ring, &rules);
        // Flying it fails if it comes to the wall too low, or comes
        // down inside it.
        let towards = FOUL.0 + (FOUL.1 - FOUL.0) * across;
        let (over, down, at) = fly(&zinger, towards, &rules);
        // It goes over once the view has changed to the field, so that
        // it is seen to go, and comes down in good time.
        prop_assert!(rules.hit.watch < over && over < down, "over on {}", over);
        prop_assert!(down < rules.field.longest / 2, "down on {}", down);
        let far = reach(HOME, at);
        prop_assert!((far - zinger.carry).abs() < 1.0, "{} off for {:?}", far, zinger);
    }
}
