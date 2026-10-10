//! What a side's strength does to the runs it is likely to make, and the
//! rules a fixture against it is played by.
//!
//! The rules say how likely the other side of a full match is to make each
//! number of runs in an innings. A side of a tournament makes its runs by
//! the same table, leant: the chance of each number of runs is multiplied
//! by the side's strength once for every run, so a strong side's chances
//! are moved towards the high scores and a weak side's towards the low.

use crate::rules::FullMatchRules;

/// What the chances of a leant table add up to. Every chance is a whole
/// number of these, and there are few enough of them for the sums done
/// with the table to come out exactly.
pub const SHARES: u32 = 1_000_000;

/// The chances of making each number of runs in an innings, from none up,
/// for a side of this strength, given a middling side's. A number of runs
/// that had no chance has none, and the rest add up to [`SHARES`].
pub fn leant(chances: &[u32], strength: f32) -> Vec<u32> {
    // Each run multiplies by the strength again. It is multiplied out a
    // run at a time, which every machine does alike.
    let mut by = 1.0_f64;
    let weights: Vec<f64> = chances
        .iter()
        .map(|&chance| {
            let weight = f64::from(chance) * by;
            by *= f64::from(strength);
            weight
        })
        .collect();
    let all: f64 = weights.iter().sum();
    let Some(last) = weights.iter().rposition(|&weight| weight > 0.0) else {
        // A side that never scores is no stronger or weaker for it.
        return chances.to_vec();
    };
    // What is kept is how many shares there are up to and including each
    // number of runs, rounded down. A stronger side has no more of them up
    // to any number than a weaker one, before the rounding and so after
    // it, which is what makes it never score fewer for the same draw.
    let mut so_far = 0.0;
    let mut kept = 0;
    weights
        .iter()
        .enumerate()
        .map(|(runs, weight)| {
            so_far += weight;
            let up_to = if runs >= last {
                SHARES
            } else {
                (so_far / all * f64::from(SHARES)).floor() as u32
            };
            let share = up_to.saturating_sub(kept);
            kept = kept.max(up_to);
            share
        })
        .collect()
}

/// The rules a fixture is played by: a full match's, with this many
/// innings, against a side of this strength at every skill level.
pub fn match_rules(base: &FullMatchRules, strength: f32, innings: u32) -> FullMatchRules {
    let mut rules = base.clone();
    rules.innings = innings;
    for chances in [
        &mut rules.runs.easy,
        &mut rules.runs.medium,
        &mut rules.runs.hard,
    ] {
        *chances = leant(chances, strength);
    }
    rules
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::rules::Rules;
    use crate::settings::Difficulty;

    /// The runs an innings comes to on the whole, by these chances.
    fn usual(chances: &[u32]) -> f64 {
        let all: u32 = chances.iter().sum();
        let runs: f64 = (0u32..)
            .zip(chances)
            .map(|(runs, &chance)| f64::from(runs) * f64::from(chance))
            .sum();
        runs / f64::from(all)
    }

    #[test]
    fn a_middling_side_makes_what_the_rules_have_the_other_side_make() {
        let chances = Rules::default().full_match.runs.medium;
        let all: u32 = chances.iter().sum();
        let leant = leant(&chances, 1.0);
        assert_eq!(leant.iter().sum::<u32>(), SHARES);
        for (was, is) in chances.iter().zip(&leant) {
            let share = f64::from(*was) / f64::from(all) * f64::from(SHARES);
            assert!((share - f64::from(*is)).abs() <= 1.0, "{was} became {is}");
        }
    }

    #[test]
    fn the_stronger_the_side_the_more_runs_on_the_whole() {
        let chances = Rules::default().full_match.runs.medium;
        let usual_at = |strength| usual(&leant(&chances, strength));
        // As data/rules.toml says of them.
        assert!((usual_at(0.8) - 1.25).abs() < 0.01, "{}", usual_at(0.8));
        assert!((usual_at(1.0) - 1.78).abs() < 0.01, "{}", usual_at(1.0));
        assert!((usual_at(1.25) - 2.53).abs() < 0.01, "{}", usual_at(1.25));
    }

    #[test]
    fn runs_that_had_no_chance_have_none_and_a_side_that_never_scores_never_does() {
        assert_eq!(leant(&[3, 0, 1, 0], 2.0), [428_571, 0, 571_429, 0]);
        assert_eq!(leant(&[0, 0, 0], 1.5), [0, 0, 0]);
        assert_eq!(leant(&[7], 0.5), [SHARES]);
        assert!(leant(&[], 1.5).is_empty());
    }

    #[test]
    fn a_fixtures_rules_are_a_full_matchs_but_for_its_innings_and_their_runs() {
        let base = Rules::default().full_match;
        let rules = match_rules(&base, 1.25, 5);
        assert_eq!(rules.innings, 5);
        assert_eq!(rules.runs.hard, leant(&base.runs.hard, 1.25));
        assert_eq!(rules.their_batting, base.their_batting);
        assert_eq!(rules.zinger_innings, base.zinger_innings);
    }

    proptest! {
        #[test]
        fn a_stronger_side_never_makes_fewer_runs_for_the_same_draw(
            chances in prop::collection::vec(0u32..=100, 1..10),
            // Strengths by the hundredth, as the rules have them.
            weaker in 25u32..=400,
            more in 0u32..=375,
            drawn in 0.0f32..1.0,
        ) {
            let stronger = (weaker + more).min(400);
            let mut rules = Rules::default().full_match;
            let runs = |rules: &mut FullMatchRules, hundredths: u32| {
                rules.runs.medium = leant(&chances, hundredths as f32 / 100.0);
                rules.runs_for(Difficulty::Medium, drawn)
            };
            let (fewer, as_many) = (runs(&mut rules, weaker), runs(&mut rules, stronger));
            prop_assert!(fewer <= as_many, "{} then {}", fewer, as_many);
        }

        #[test]
        fn a_leant_table_adds_up_and_gives_no_chance_where_there_was_none(
            chances in prop::collection::vec(0u32..=100, 1..10),
            hundredths in 25u32..=400,
        ) {
            let leant = leant(&chances, hundredths as f32 / 100.0);
            prop_assert_eq!(leant.len(), chances.len());
            if chances.iter().any(|&chance| chance > 0) {
                prop_assert_eq!(leant.iter().sum::<u32>(), SHARES);
            }
            for (was, is) in chances.iter().zip(&leant) {
                prop_assert!(*was > 0 || *is == 0);
            }
        }
    }
}
