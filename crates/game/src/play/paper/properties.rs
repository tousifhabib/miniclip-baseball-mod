use proptest::prelude::*;

use super::tests::{played_by, sound};
use super::*;
use crate::rules::Rules;

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
