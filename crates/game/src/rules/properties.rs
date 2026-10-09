use std::fmt::Write;

use proptest::prelude::*;

use super::shapes::{Levels, Span};
use super::*;

/// A whole number of the rules that a layer might name: what a file
/// calls it, and where the rules keep it.
type Whole = (&'static str, fn(&mut Rules) -> &mut u32);

/// The same, for a number that need not be whole.
type Part = (&'static str, fn(&mut Rules) -> &mut f32);

/// Some of each, from all over the file: in tables of their own, in
/// tables within tables, and in the tables that are written on one line.
/// None is a number the game holds within bounds, since a layer here
/// may give it any value at all.
const WHOLES: [Whole; 10] = [
    ("match.outs", |rules| &mut rules.game.outs),
    ("match.runs_down.hard", |rules| {
        &mut rules.game.runs_down.hard
    }),
    ("count.strikes", |rules| &mut rules.count.strikes),
    ("full_match.innings", |rules| &mut rules.full_match.innings),
    ("throw.settle", |rules| &mut rules.throw.settle),
    ("hit.watch", |rules| &mut rules.hit.watch),
    ("zinger.runs_down.medium", |rules| {
        &mut rules.zinger.runs_down.medium
    }),
    ("heat.most", |rules| &mut rules.heat.most),
    ("golden.every", |rules| &mut rules.golden.every),
    ("arcade.multiplier.easy", |rules| {
        &mut rules.arcade.multiplier.easy
    }),
];

const PARTS: [Part; 9] = [
    ("pitch.medium.band.top", |rules| {
        &mut rules.pitch.medium.band.top
    }),
    ("throw.growth", |rules| &mut rules.throw.growth),
    ("hit.pull", |rules| &mut rules.hit.pull),
    ("field.catch_height", |rules| &mut rules.field.catch_height),
    ("field.fielder_speed.easy", |rules| {
        &mut rules.field.fielder_speed.easy
    }),
    ("zinger.hang.best", |rules| &mut rules.zinger.hang.best),
    ("zinger.sky.peak", |rules| &mut rules.zinger.sky.peak),
    ("shift.follow", |rules| &mut rules.shift.follow),
    ("steal.their_safe", |rules| &mut rules.steal.their_safe),
];

/// What a layer names: for each of those numbers, in the order they are
/// listed, the value it gives it, if it names it at all.
type Named = (Vec<Option<u32>>, Vec<Option<f32>>);

fn named() -> impl Strategy<Value = Named> {
    let whole = prop::option::of(0u32..1_000_000);
    // Eighths, which are written out and read back to the last place.
    let part = prop::option::of((-800i16..=800).prop_map(|eighths| f32::from(eighths) / 8.0));
    (
        prop::collection::vec(whole, WHOLES.len()),
        prop::collection::vec(part, PARTS.len()),
    )
}

/// Writes out the layer that names these numbers, and makes the same
/// changes to `rules` by hand.
fn lay(named: &Named, rules: &mut Rules) -> String {
    let mut text = String::new();
    for ((name, place), value) in WHOLES.iter().zip(&named.0) {
        if let Some(value) = *value {
            writeln!(text, "{name} = {value}").unwrap();
            *place(rules) = value;
        }
    }
    for ((name, place), value) in PARTS.iter().zip(&named.1) {
        if let Some(value) = *value {
            writeln!(text, "{name} = {value:?}").unwrap();
            *place(rules) = value;
        }
    }
    text
}

proptest! {
    // Each case reads the whole of the built-in rules several times.
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn a_layer_gives_the_built_in_rules_with_the_numbers_it_names_changed_and_no_others(
        named in named(),
    ) {
        let mut wanted = Rules::default();
        let text = lay(&named, &mut wanted);
        let rules = Rules::layered(&[("a mod", &text)]).unwrap();
        prop_assert_eq!(rules, wanted, "from {:?}", text);
    }

    #[test]
    fn a_layer_laid_on_twice_is_the_layer_laid_on_once(named in named()) {
        let text = lay(&named, &mut Rules::default());
        let once = Rules::layered(&[("once", &text)]).unwrap();
        let twice = Rules::layered(&[("once", &text), ("again", &text)]).unwrap();
        prop_assert_eq!(twice, once, "from {:?}", text);
    }

    #[test]
    fn of_two_layers_each_changes_what_it_names_and_the_later_has_the_last_word(
        first in named(),
        second in named(),
    ) {
        // Whatever the first names and the second does not is still as
        // the first left it.
        let mut wanted = Rules::default();
        let first = lay(&first, &mut wanted);
        let second = lay(&second, &mut wanted);
        let rules = Rules::layered(&[("first", &first), ("second", &second)]).unwrap();
        prop_assert_eq!(rules, wanted, "from {:?} and then {:?}", first, second);
    }
}

proptest! {
    #[test]
    fn a_level_there_is_none_of_is_taken_as_the_nearest_there_is(
        numbers in proptest::collection::vec(0u32..1000, 0..12),
        level in any::<u8>(),
    ) {
        let levels = Levels::from(numbers.clone());
        prop_assert_eq!(usize::from(levels.count()), numbers.len());
        let Some(last) = numbers.len().checked_sub(1) else {
            prop_assert_eq!(levels.at(level), None);
            return Ok(());
        };
        // Levels are counted from 1, so the first is at place 0.
        let place = usize::from(level).saturating_sub(1).min(last);
        prop_assert_eq!(levels.at(level), Some(numbers[place]));
    }

    #[test]
    fn a_span_many_times_over_keeps_its_ends_in_order_and_neither_is_less_than_one(
        low in 0u32..=10_000,
        more in 0u32..=10_000,
        by in -4.0f32..40.0,
    ) {
        let span = Span {
            low,
            high: low + more,
        };
        let times = span.times(by);
        prop_assert!(1 <= times.low && times.low <= times.high, "{:?}", times);
        // No times over, or fewer than none, is as short as a span
        // gets.
        if by <= 0.0 {
            prop_assert_eq!(times, Span { low: 1, high: 1 });
        }
        // Once over is the span itself, if it began at one or more.
        if low >= 1 {
            prop_assert_eq!(span.times(1.0), span);
        }
    }
}
