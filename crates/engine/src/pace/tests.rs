use super::*;

const FRAME: Duration = Duration::from_nanos(1_000_000_000 / 60);

/// The times between 240 redraws of a screen that redraws 120 times a
/// second, in microseconds, as they were measured on one: two seconds
/// in which hardly any came when it should.
const MEASURED: [u16; 240] = [
    7028, 7383, 9488, 6136, 9724, 7031, 10816, 6956, 7115, 9713, 9850, 6760, 6945, 11382, 5300,
    12163, 6626, 7496, 9458, 9882, 4428, 10871, 7181, 6899, 12249, 5922, 6759, 12313, 4490, 11180,
    6967, 8883, 7251, 9257, 6708, 10822, 7199, 7939, 8384, 9328, 7917, 7199, 8632, 10033, 7919,
    6575, 9697, 6504, 8851, 10233, 7046, 7926, 9610, 8720, 5842, 11314, 6696, 7692, 10042, 7676,
    7733, 9424, 6980, 7314, 11498, 8040, 7700, 7662, 6787, 12125, 4573, 10636, 5993, 10757, 5925,
    12005, 4638, 13024, 3732, 9202, 8816, 9813, 6806, 6858, 11168, 6889, 7010, 11890, 7188, 5976,
    12310, 4361, 12487, 4170, 8293, 12238, 4527, 12001, 4558, 12272, 4570, 11815, 6698, 7886, 6846,
    11631, 5091, 10949, 6931, 7972, 9851, 6349, 10247, 6397, 9738, 8982, 7755, 6787, 10758, 7643,
    6655, 10618, 6218, 8178, 11078, 5913, 9512, 6880, 10711, 7734, 6108, 11544, 6568, 7848, 9183,
    8883, 5873, 10686, 8307, 5998, 11662, 6528, 9214, 6127, 10097, 8932, 6341, 10297, 5697, 10964,
    7756, 8868, 7315, 9206, 8597, 8082, 6867, 9348, 8050, 13842, 2453, 6985, 9973, 7708, 8343,
    8579, 8348, 8345, 8615, 8286, 8648, 7571, 8367, 6971, 9539, 7015, 10000, 7998, 8615, 8174,
    8741, 7937, 9353, 7667, 8200, 8614, 8896, 8639, 8870, 7647, 8228, 8257, 8508, 9045, 8603, 8226,
    7723, 6623, 10490, 7795, 7004, 9400, 8275, 7116, 10662, 7236, 6449, 11486, 6324, 8092, 9299,
    9504, 5270, 9181, 10175, 6654, 7185, 10956, 6952, 9132, 10282, 4342, 8230, 12195, 4624, 12070,
    4583, 11187, 6690, 6972, 11805, 4945, 11766, 7279, 7900, 6386, 12110, 4609, 12066, 4532,
];

/// Numbers that look random and are the same every time.
struct Dice(u64);

impl Dice {
    /// A number from -1 to 1.
    fn roll(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0
    }
}

/// The times between redraws on a screen that redraws `rate` times a
/// second, each redraw beginning up to `wobble` early or late.
fn redraws(rate: f64, count: usize, wobble: Duration, dice: &mut Dice) -> Vec<Duration> {
    let mut times: Vec<f64> = (0..=count)
        .map(|redraw| redraw as f64 / rate + dice.roll() * wobble.as_secs_f64())
        .collect();
    times.sort_by(f64::total_cmp);
    times
        .windows(2)
        .map(|pair| Duration::from_secs_f64(pair[1] - pair[0]))
        .collect()
}

/// The frames played for each redraw, the game having begun `late` into
/// a frame.
fn played(pace: &mut Pace, late: Duration, redraws: &[Duration]) -> Vec<u32> {
    pace.frames(late);
    redraws.iter().map(|&took| pace.frames(took)).collect()
}

/// How many redraws each frame was on the screen for.
fn held(played: &[u32]) -> Vec<u32> {
    let mut held = Vec::new();
    for &frames in played {
        match (frames, held.last_mut()) {
            (0, Some(last)) => *last += 1,
            (0, None) => {}
            // A frame played with another is never seen.
            _ => held.extend((1..frames).map(|_| 0).chain([1])),
        }
    }
    // The first and last may have been cut short.
    held[1..held.len() - 1].to_vec()
}

/// What going by the clock alone makes of the same redraws.
fn by_the_clock(late: Duration, redraws: &[Duration]) -> Vec<u32> {
    let mut owed = late;
    redraws
        .iter()
        .map(|&took| {
            owed += took;
            let frames = (owed.as_nanos() / FRAME.as_nanos()) as u32;
            owed -= FRAME * frames;
            frames
        })
        .collect()
}

/// Every sixteenth of a frame the game might have begun at.
fn starts() -> impl Iterator<Item = Duration> {
    (0..16).map(|sixteenth| FRAME * sixteenth / 16)
}

#[test]
fn a_screen_in_step_shows_every_frame_for_as_long_however_it_wobbles() {
    let wobble = Duration::from_micros(900);
    for (rate, every) in [(60.0, 1), (59.94, 1), (120.0, 2), (119.88, 2), (240.0, 4)] {
        for late in starts() {
            let redraws = redraws(rate, 2000, wobble, &mut Dice(7));
            let mut pace = Pace::new(FRAME);
            let played = played(&mut pace, late, &redraws);
            assert_eq!(pace.in_step(), Some(every), "at {rate}");
            // Once it has seen enough redraws to know the rate.
            let held = held(&played[RECENT..]);
            assert!(
                held.iter().all(|&redraws| redraws == every),
                "at {rate}, {late:?} late: {held:?}"
            );
        }
    }
}

#[test]
fn redraws_measured_on_a_real_screen_are_shown_evenly_too() {
    let measured: Vec<Duration> = MEASURED
        .iter()
        .map(|&micros| Duration::from_micros(u64::from(micros)))
        .collect();
    // The same two seconds, six times over.
    let redraws: Vec<Duration> = measured.iter().copied().cycle().take(1440).collect();
    let mut uneven_by_the_clock = 0;
    for late in starts() {
        let mut pace = Pace::new(FRAME);
        let played = played(&mut pace, late, &redraws);
        assert_eq!(pace.in_step(), Some(2));
        let held = held(&played[RECENT..]);
        assert!(
            held.iter().all(|&redraws| redraws == 2),
            "{late:?}: {held:?}"
        );
        let clock = self::held(&by_the_clock(late, &redraws)[RECENT..]);
        uneven_by_the_clock += clock.iter().filter(|&&redraws| redraws != 2).count();
    }
    // Which is the trouble this is here to end.
    assert!(uneven_by_the_clock > 1000, "{uneven_by_the_clock}");
}

#[test]
fn a_screen_out_of_step_is_played_by_the_clock() {
    let wobble = Duration::from_micros(900);
    for rate in [50.0, 75.0, 90.0, 144.0, 165.0] {
        let redraws = redraws(rate, (rate * 10.0) as usize, wobble, &mut Dice(3));
        let mut pace = Pace::new(FRAME);
        let played = played(&mut pace, Duration::ZERO, &redraws);
        assert_eq!(pace.in_step(), None, "at {rate}");
        // Ten seconds of redraws is six hundred frames of the game.
        let frames: u32 = played.iter().sum();
        assert!((599..=601).contains(&frames), "at {rate}: {frames}");
    }
}

#[test]
fn a_late_redraw_and_the_early_one_after_it_change_nothing() {
    let steady = vec![FRAME / 2; 200];
    let mut pace = Pace::new(FRAME);
    played(&mut pace, Duration::ZERO, &steady);
    let (late, early) = (Duration::from_millis(14), Duration::from_micros(2_667));
    let mut bumpy = Vec::new();
    for _ in 0..20 {
        bumpy.extend([FRAME / 2, late, early, FRAME / 2, FRAME / 2]);
    }
    let played: Vec<u32> = bumpy.iter().map(|&took| pace.frames(took)).collect();
    assert!(
        held(&played).iter().all(|&redraws| redraws == 2),
        "{played:?}"
    );
}

#[test]
fn a_stall_is_caught_up_and_a_long_one_only_in_part() {
    let steady = vec![FRAME / 2; 200];
    let mut pace = Pace::new(FRAME);
    played(&mut pace, Duration::ZERO, &steady);
    // Fifty milliseconds is six redraws, and so three frames.
    assert_eq!(pace.frames(Duration::from_millis(50)), 3);
    let after: Vec<u32> = (0..40).map(|_| pace.frames(FRAME / 2)).collect();
    assert!(
        held(&after).iter().all(|&redraws| redraws == 2),
        "{after:?}"
    );
    assert_eq!(pace.in_step(), Some(2));
    // A stall of seconds is not replayed.
    assert_eq!(pace.frames(Duration::from_secs(3)), MOST_AT_ONCE);
    let after: Vec<u32> = (0..40).map(|_| pace.frames(FRAME / 2)).collect();
    assert_eq!(after.iter().sum::<u32>(), 20);
    assert!(
        held(&after).iter().all(|&redraws| redraws == 2),
        "{after:?}"
    );
}

#[test]
fn a_game_too_slow_for_the_screen_is_not_slowed_down() {
    // Each redraw takes half as long again as the screen allows it.
    let slow = vec![FRAME * 3 / 4; 400];
    let mut pace = Pace::new(FRAME);
    let steady = vec![FRAME / 2; 200];
    played(&mut pace, Duration::ZERO, &steady);
    let frames: u32 = slow.iter().map(|&took| pace.frames(took)).sum();
    // Five seconds of them is three hundred frames.
    assert!((290..=302).contains(&frames), "{frames}");
}

#[test]
fn it_follows_the_screen_from_one_rate_to_another() {
    let wobble = Duration::from_micros(600);
    let mut dice = Dice(11);
    let mut pace = Pace::new(FRAME);
    let fast = redraws(120.0, 600, wobble, &mut dice);
    let slow = redraws(60.0, 600, wobble, &mut dice);
    played(&mut pace, Duration::ZERO, &fast);
    assert_eq!(pace.in_step(), Some(2));
    let through: Vec<u32> = slow.iter().map(|&took| pace.frames(took)).collect();
    assert_eq!(pace.in_step(), Some(1));
    // Ten seconds at the new rate, with no time lost or made on the way.
    let frames: u32 = through.iter().sum();
    assert!((597..=603).contains(&frames), "{frames}");
    assert!(through[2 * RECENT..].iter().all(|&frames| frames == 1));
    let back: Vec<u32> = fast.iter().map(|&took| pace.frames(took)).collect();
    assert_eq!(pace.in_step(), Some(2));
    let frames: u32 = back.iter().sum();
    assert!((297..=303).contains(&frames), "{frames}");
}

#[test]
fn redraws_that_come_to_nothing_tell_it_nothing_of_the_screen() {
    let mut pace = Pace::new(FRAME);
    played(&mut pace, Duration::ZERO, &vec![FRAME / 2; 200]);
    assert_eq!(pace.in_step(), Some(2));
    // The window is covered for a tenth of a second, and is asked to
    // redraw two thousand times in it.
    let mut frames = 0;
    for _ in 0..2000 {
        frames += pace.frames(Duration::from_micros(50));
        pace.undrawn();
        assert_eq!(pace.in_step(), None);
    }
    assert_eq!(frames, 6);
    // Back in sight, it is in step again once it has seen enough, and
    // never takes the screen for any other.
    let mut seen = Vec::new();
    for _ in 0..200 {
        pace.frames(FRAME / 2);
        seen.push(pace.in_step());
    }
    assert!(seen[..ENOUGH - 1].iter().all(Option::is_none), "{seen:?}");
    assert!(
        seen[ENOUGH..].iter().all(|&step| step == Some(2)),
        "{seen:?}"
    );
}

#[test]
fn nothing_is_played_while_no_time_passes() {
    // A window that is not being drawn is asked again at once.
    let mut pace = Pace::new(FRAME);
    let frames: u32 = (0..10_000)
        .map(|_| pace.frames(Duration::from_micros(2)))
        .sum();
    assert_eq!(pace.in_step(), None);
    assert_eq!(frames, 1);
}
