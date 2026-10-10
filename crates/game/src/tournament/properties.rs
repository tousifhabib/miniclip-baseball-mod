use proptest::prelude::*;

use super::testing::{drawn, played_out};
use super::*;
use crate::rules::Rules;

proptest! {
    // Each case plays a whole tournament on paper, twice.
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn any_tournament_is_played_to_one_champion_and_goes_the_same_way_twice(
        seed: u64,
        shape in 0usize..3,
        innings in 1u32..=3,
    ) {
        let format = Format::ALL[shape];
        let rules = Rules::default();
        let played = played_out(drawn(format, innings, seed), &rules, |_| {});
        prop_assert!(played.is_over());
        prop_assert_eq!(played.cards().len(), schedule::ties(format).len());
        prop_assert!(played.cards().iter().all(Card::is_sound));
        prop_assert!(played.champion().is_some());
        prop_assert!(played.players_end().is_some());
        // In a table everyone has met everyone, and every match was one
        // side's win and another's loss.
        for group in 0..format.groups() {
            let table = played.table(group);
            let sides = table.len() as u32;
            let won: u32 = table.iter().map(|row| row.won).sum();
            let lost: u32 = table.iter().map(|row| row.lost).sum();
            prop_assert_eq!((won, lost), (sides * (sides - 1) / 2, sides * (sides - 1) / 2));
            prop_assert!(table.iter().all(|row| row.played == sides - 1));
        }
        let again = played_out(drawn(format, innings, seed), &rules, |_| {});
        prop_assert_eq!(again, played);
    }
}
