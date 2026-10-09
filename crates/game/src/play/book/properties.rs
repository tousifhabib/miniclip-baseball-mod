use proptest::prelude::*;

use super::tests::{ball, pitch};
use super::*;

/// A turn at the plate such as no game need ever have had: its pitches,
/// how it ended and where the ball went go together no better than
/// chance has them. The sums are to come out all the same.
fn any_turn() -> impl Strategy<Value = Turn> {
    let thrown = prop::sample::select(
        &[
            Thrown::Ball,
            Thrown::Called,
            Thrown::Swinging,
            Thrown::Foul,
            Thrown::InPlay,
        ][..],
    );
    let end = prop::sample::select(
        &[
            End::Strikeout,
            End::Walk,
            End::Single,
            End::Double,
            End::Triple,
            End::HomeRun,
            End::FlyOut,
            End::GroundOut,
            End::SacrificeFly,
            End::DoublePlay,
            End::Error,
        ][..],
    );
    let came_up = (1u32..=9, 0..ORDER, 0u32..3, any::<[bool; 3]>());
    let pitches = prop::collection::vec((thrown, any::<bool>()), 0..8);
    let hit = prop::option::of((0.0f32..=1.0, 100.0f32..1000.0, any::<bool>()));
    (came_up, pitches, end, hit, 0u32..=4, 0u32..=2).prop_map(
        |((innings, order, outs, on), pitches, end, hit, runs_in, outs_made)| {
            // Written up as a turn is: he comes up, sees his pitches
            // and is done.
            let mut side = Side::default();
            side.come_up(innings, order, outs, on);
            for (thrown, in_zone) in pitches {
                side.pitch(pitch(thrown, in_zone));
            }
            let hit = hit.and_then(|(across, far, fly)| ball(across, far, fly));
            side.close(end, hit, runs_in, outs_made);
            side.turns.remove(0)
        },
    )
}

fn any_turns() -> impl Strategy<Value = Vec<Turn>> {
    prop::collection::vec(any_turn(), 0..12)
}

/// One side's part of a book, made up the same way: its turns, the
/// tries at a base its runners made between them, the runs each batter
/// made and the runners it left on.
fn any_side() -> impl Strategy<Value = Side> {
    let steal = prop::option::of((0..ORDER, 2u8..=3, any::<bool>()));
    let turns = prop::collection::vec((steal, any_turn()), 0..12);
    let runs = prop::array::uniform9(0u32..5);
    let left = prop::collection::vec(0u32..=3, 0..9);
    (turns, runs, left).prop_map(|(turns, runs, left)| {
        let mut side = Side {
            runs,
            left,
            ..Side::default()
        };
        for (steal, turn) in turns {
            if let Some((order, base, safe)) = steal {
                side.stole(turn.innings, order, base, safe);
            }
            side.turns.push(turn);
        }
        side
    })
}

/// Every count there is in a set of figures. The longest ball is not
/// one: it is the longest, not how many.
fn counts(figures: &Figures) -> Vec<u32> {
    // Taken apart by name, so that a count added to the figures has to
    // be added here before this will build.
    let Figures {
        turns,
        at_bats,
        runs,
        hits,
        singles,
        doubles,
        triples,
        home_runs,
        total_bases,
        runs_in,
        walks,
        strikeouts,
        sacrifices,
        double_plays,
        left,
        chances,
        chances_taken,
        two_out_runs,
        pitches,
        strikes,
        called,
        swinging,
        fouls,
        swings,
        outside,
        chases,
        in_play,
        flies,
        grounders,
        thirds: [to_left, to_centre, to_right],
        feet,
        longest: _,
        stolen,
        caught,
    } = *figures;
    vec![
        turns,
        at_bats,
        runs,
        hits,
        singles,
        doubles,
        triples,
        home_runs,
        total_bases,
        runs_in,
        walks,
        strikeouts,
        sacrifices,
        double_plays,
        left,
        chances,
        chances_taken,
        two_out_runs,
        pitches,
        strikes,
        called,
        swinging,
        fouls,
        swings,
        outside,
        chases,
        in_play,
        flies,
        grounders,
        to_left,
        to_centre,
        to_right,
        feet,
        stolen,
        caught,
    ]
}

proptest! {
    #[test]
    fn the_figures_of_two_lists_of_turns_add_up_to_the_figures_of_both_together(
        first in any_turns(),
        second in any_turns(),
    ) {
        let (one, other) = (Figures::of(first.iter()), Figures::of(second.iter()));
        let both = Figures::of(first.iter().chain(&second));
        let added: Vec<u32> = counts(&one)
            .iter()
            .zip(counts(&other))
            .map(|(one, other)| one + other)
            .collect();
        prop_assert_eq!(counts(&both), added);
        // The longest ball of them all is the longer of the two longest.
        prop_assert_eq!(both.longest, one.longest.max(other.longest));
    }

    #[test]
    fn the_figures_of_any_turns_agree_with_one_another(turns in any_turns()) {
        let figures = Figures::of(turns.iter());
        // Every turn is an at-bat, a walk or a sacrifice.
        let Figures { at_bats, walks, sacrifices, .. } = figures;
        prop_assert_eq!(figures.turns, at_bats + walks + sacrifices);
        // A hit is a single, a double, a triple or a home run, and is
        // worth that many bases.
        let Figures { singles, doubles, triples, home_runs, .. } = figures;
        prop_assert_eq!(figures.hits, singles + doubles + triples + home_runs);
        let bases = singles + 2 * doubles + 3 * triples + 4 * home_runs;
        prop_assert_eq!(figures.total_bases, bases);
        // A strike was let go by or was swung at.
        prop_assert_eq!(figures.strikes, figures.called + figures.swings);
        // A ball in play went in the air or along the ground, and to
        // one third of the field or another.
        prop_assert_eq!(figures.in_play, figures.flies + figures.grounders);
        prop_assert_eq!(figures.in_play, figures.thirds.iter().sum::<u32>());
        // And no part is more than what it is a part of.
        let parts = [
            (figures.hits, at_bats),
            (figures.chances_taken, figures.chances),
            (figures.chances, at_bats),
            (figures.two_out_runs, figures.runs_in),
            (figures.swinging + figures.fouls, figures.swings),
            (figures.chases, figures.outside),
            (figures.outside, figures.pitches),
            (figures.longest, figures.feet),
        ];
        for (number, (part, whole)) in parts.into_iter().enumerate() {
            prop_assert!(part <= whole, "{} of {}, in pair {}", part, whole, number);
        }
    }

    #[test]
    fn every_average_is_there_when_there_is_something_to_divide_by_and_is_one_that_can_be(
        turns in any_turns(),
    ) {
        let figures = Figures::of(turns.iter());
        // Each average with what it is so many for each of, and the
        // most it can come to: a base at a time, or four.
        let averages = [
            (figures.average(), figures.at_bats, 1.0),
            (figures.on_base(), figures.turns, 1.0),
            (figures.slugging(), figures.at_bats, 4.0),
            (figures.chance_average(), figures.chances, 1.0),
            (figures.strikeout_rate(), figures.turns, 1.0),
            (figures.walk_rate(), figures.turns, 1.0),
            (figures.strike_rate(), figures.pitches, 1.0),
            (figures.swing_rate(), figures.pitches, 1.0),
            (figures.contact_rate(), figures.swings, 1.0),
            (figures.miss_rate(), figures.swings, 1.0),
            (figures.chase_rate(), figures.outside, 1.0),
        ];
        for (number, (average, of, most)) in averages.into_iter().enumerate() {
            prop_assert_eq!(average.is_some(), of > 0, "average {}", number);
            let within = average.is_none_or(|average| (0.0..=most).contains(&average));
            prop_assert!(within, "average {} is {:?}", number, average);
        }
        // Hits for each ball that stayed in the park are no more than
        // one for one, whatever there is to divide by.
        let in_play = figures.in_play_average();
        prop_assert!(in_play.is_none_or(|average| (0.0..=1.0).contains(&average)));
        // A swing met the ball or missed it.
        if let (Some(met), Some(missed)) = (figures.contact_rate(), figures.miss_rate()) {
            prop_assert!((met + missed - 1.0).abs() < 1e-6, "{} and {}", met, missed);
        }
        // No ball went further than the longest, so nor did they on
        // the whole.
        let usual = figures.usual_feet();
        prop_assert!(usual.is_none_or(|feet| feet <= figures.longest as f32));
    }

    #[test]
    fn what_the_nine_batters_did_adds_up_to_what_their_side_did(side in any_side()) {
        let whole = side.figures();
        let each: Vec<Figures> = (0..ORDER).map(|order| side.figures_of(order)).collect();
        let mut added = vec![0; counts(&whole).len()];
        for figures in &each {
            for (sum, count) in added.iter_mut().zip(counts(figures)) {
                *sum += count;
            }
        }
        // But for the runners left on base, who are the side's and no
        // one batter's.
        let but_left = Figures { left: 0, ..whole };
        prop_assert_eq!(added, counts(&but_left));
        let longest = each.iter().map(|figures| figures.longest).max();
        prop_assert_eq!(longest, Some(whole.longest));
    }
}
