use proptest::prelude::*;

use super::*;
use crate::play::book::Figures;
// By name, because all of what proptest offers includes an `Rng` of its
// own.
use crate::rng::Rng;
use crate::rules::Rules;

fn played(made: u32, winning: bool, first_up: usize, seed: u64) -> Half {
    played_by(made, winning, first_up, seed, None)
}

fn played_by(
    made: u32,
    winning: bool,
    first_up: usize,
    seed: u64,
    steals: Option<&StealRules>,
) -> Half {
    let rules = Rules::default().full_match.their_batting;
    let mut rng = Rng::new(seed);
    let wanted = Wanted {
        made,
        winning,
        innings: 3,
        first_up,
    };
    half(wanted, &rules, steals, &Ground::default(), &mut rng)
}

/// Checks everything that has to be so of a half however it went.
fn sound(half: &Half, made: u32, winning: bool, first_up: usize) {
    let runs_in: u32 = half.turns.iter().map(|turn| turn.runs_in).sum();
    assert_eq!(runs_in, made, "the runs that came in");
    assert_eq!(
        half.runs.iter().sum::<u32>(),
        made,
        "the runs each batter made"
    );
    let caught = half.steals.iter().filter(|steal| !steal.safe).count() as u32;
    let outs = half.turns.iter().map(|turn| turn.outs_made).sum::<u32>() + caught;
    if winning {
        assert!(outs < 3, "a match that was won before the side was out");
        assert!(half.turns.last().is_some_and(|turn| turn.runs_in > 0));
    } else {
        assert_eq!(outs, 3, "the outs");
    }
    // Everyone who came up is out, home or left on base.
    let reached = half.turns.len() as u32;
    assert_eq!(reached, outs + made + half.left, "where the batters went");
    let mut out_so_far = 0;
    let mut on_base = 0;
    for steal in &half.steals {
        assert!(steal.at <= half.turns.len() && steal.order < ORDER);
        assert!(steal.innings == 3 && (steal.base == 2 || steal.base == 3));
    }
    for (index, turn) in half.turns.iter().enumerate() {
        // A runner thrown out stealing before this turn is out, and
        // off the bases.
        for steal in half.steals.iter().filter(|steal| steal.at == index) {
            if !steal.safe {
                out_so_far += 1;
                on_base -= 1;
            }
        }
        // They come up in order, with the outs there have been.
        assert_eq!(turn.order, (first_up + index) % ORDER);
        assert_eq!(turn.outs, out_so_far);
        assert_eq!(turn.on.iter().filter(|on| **on).count() as u32, on_base);
        assert_eq!(turn.innings, 3);
        out_so_far += turn.outs_made;
        on_base = on_base + 1 - turn.outs_made - turn.runs_in;
        // The count is one that can be: the turn ends on its last
        // pitch, and not before.
        let (mut balls, mut strikes) = (0, 0);
        for (number, pitch) in turn.pitches.iter().enumerate() {
            assert!(balls < 4 && strikes < 3, "a pitch after the turn was over");
            let last = number + 1 == turn.pitches.len();
            match pitch.thrown {
                Thrown::Ball => balls += 1,
                Thrown::Called | Thrown::Swinging => strikes += 1,
                Thrown::Foul => strikes = (strikes + 1).min(2),
                Thrown::InPlay => assert!(last, "a ball in play that was not the last pitch"),
            }
            assert!(pitch.off.is_none() && pitch.quality.is_none());
        }
        let last = turn.pitches.last().expect("a turn has a pitch").thrown;
        match turn.end {
            End::Strikeout => assert_eq!(strikes, 3),
            End::Walk => assert_eq!(balls, 4),
            _ => assert_eq!(last, Thrown::InPlay),
        }
        assert_eq!(turn.ball.is_some(), turn.end.in_play());
        if let Some(ball) = turn.ball {
            assert!((0.0..=1.0).contains(&ball.across), "a fair ball");
            // A home run is over the wall, and nothing else is.
            assert_eq!(ball.feet > 400, turn.end == End::HomeRun, "{ball:?}");
        }
        match turn.end {
            End::DoublePlay => assert_eq!(turn.outs_made, 2),
            End::Strikeout | End::FlyOut | End::GroundOut | End::SacrificeFly => {
                assert_eq!(turn.outs_made, 1);
            }
            _ => assert_eq!(turn.outs_made, 0),
        }
        if turn.end == End::SacrificeFly {
            assert!(turn.on[2] && turn.runs_in == 1 && turn.outs < 2);
        }
        if turn.end == End::DoublePlay {
            assert!(turn.on[0] && turn.outs < 2);
        }
    }
    assert_eq!(half.next, (first_up + half.turns.len()) % ORDER);
}

#[test]
fn a_half_comes_to_the_runs_it_was_to_and_adds_up() {
    for seed in 0..400 {
        let made = (seed % 9) as u32;
        let first_up = (seed % 7) as usize;
        let half = played(made, false, first_up, seed);
        sound(&half, made, false, first_up);
    }
}

#[test]
fn a_half_of_a_great_many_runs_still_adds_up() {
    for seed in 0..40 {
        let made = 10 + (seed % 14) as u32;
        let half = played(made, false, 0, seed);
        sound(&half, made, false, 0);
    }
}

#[test]
fn a_half_that_wins_the_match_ends_on_the_run_that_wins_it() {
    for seed in 0..300 {
        let made = 1 + (seed % 5) as u32;
        let half = played(made, true, 4, seed);
        sound(&half, made, true, 4);
    }
}

#[test]
fn with_runners_who_steal_a_half_still_adds_up() {
    // Runners who go every chance they get, and are out half the time.
    let keen = StealRules {
        their_chance: 1.0,
        their_third: 1.0,
        their_safe: 0.5,
        ..Rules::default().steal
    };
    let (mut stolen, mut caught, mut thirds) = (0, 0, 0);
    for seed in 0..400 {
        let made = (seed % 7) as u32;
        let winning = seed % 5 == 0 && made > 0;
        let half = played_by(made, winning, 2, seed, Some(&keen));
        sound(&half, made, winning, 2);
        stolen += half.steals.iter().filter(|steal| steal.safe).count();
        caught += half.steals.iter().filter(|steal| !steal.safe).count();
        thirds += half.steals.iter().filter(|steal| steal.base == 3).count();
    }
    assert!(stolen > 100 && caught > 100 && thirds > 20);
    // As the rules have them they go now and then, and mostly get
    // there.
    let usual = Rules::default().steal;
    let (mut tries, mut safe, mut turns) = (0, 0, 0);
    for seed in 0..600 {
        let made = [0, 0, 0, 1, 1, 2, 2, 3, 4, 6][(seed % 10) as usize];
        let half = played_by(made, false, 0, seed, Some(&usual));
        sound(&half, made, false, 0);
        tries += half.steals.len();
        safe += half.steals.iter().filter(|steal| steal.safe).count();
        turns += half.turns.len();
    }
    assert!(tries > 20 && tries * 20 < turns, "{tries} in {turns}");
    assert!(safe * 2 > tries, "{safe} of {tries}");
}

#[test]
fn without_runners_who_steal_nobody_does() {
    for seed in 0..100 {
        assert!(played(3, false, 0, seed).steals.is_empty());
    }
}

#[test]
fn the_same_numbers_play_the_same_half() {
    assert_eq!(played(3, false, 2, 11), played(3, false, 2, 11));
    assert_ne!(played(3, false, 2, 11), played(3, false, 2, 12));
}

#[test]
fn over_many_innings_the_figures_are_ones_a_side_might_have() {
    // Innings of the runs the middle skill level gives, near enough.
    let mut turns = Vec::new();
    for seed in 0..600 {
        let made = [0, 0, 0, 1, 1, 2, 2, 3, 4, 6][(seed % 10) as usize];
        turns.extend(played(made, false, (seed % 9) as usize, seed).turns);
    }
    let figures = Figures::of(turns.iter());
    let between = |value: Option<f32>, low: f32, high: f32| {
        let value = value.expect("something to divide by");
        assert!(
            (low..=high).contains(&value),
            "{value} is not in {low}..{high}"
        );
    };
    // Two runs an innings is a great many, and takes a great many
    // hits: the average is high as the scores are.
    between(figures.average(), 0.36, 0.5);
    between(figures.strikeout_rate(), 0.12, 0.25);
    between(figures.walk_rate(), 0.05, 0.14);
    between(figures.strike_rate(), 0.57, 0.69);
    between(figures.contact_rate(), 0.7, 0.86);
    between(figures.chase_rate(), 0.2, 0.34);
    between(figures.pitches_a_turn(), 2.8, 4.0);
    assert!(figures.home_runs > 0 && figures.doubles > figures.triples);
}

#[test]
fn an_innings_that_will_not_come_out_is_written_plainly() {
    let ground = Ground::default();
    let wanted = |made, winning, first_up| Wanted {
        made,
        winning,
        innings: 3,
        first_up,
    };
    let half = plainly(wanted(4, false, 7), &ground);
    sound(&half, 4, false, 7);
    let winning = plainly(wanted(2, true, 0), &ground);
    sound(&winning, 2, true, 0);
}

proptest! {
    // Each case plays a half over and over until it comes out right.
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn any_half_played_on_paper_adds_up(
        made in 0u32..=12,
        winning: bool,
        first_up in 0..ORDER,
        seed: u64,
        // Whether their runners steal, and if they do how often one
        // goes for second, how much of that often for third, and how
        // often either gets there.
        steals in prop::option::of((0.0f32..=1.0, 0.0f32..=1.0, 0.0f32..=1.0)),
    ) {
        // A half that wins the match has at least the run that wins it.
        let winning = winning && made > 0;
        let steals = steals.map(|(their_chance, their_third, their_safe)| StealRules {
            their_chance,
            their_third,
            their_safe,
            ..Rules::default().steal
        });
        let half = played_by(made, winning, first_up, seed, steals.as_ref());
        sound(&half, made, winning, first_up);
    }
}
