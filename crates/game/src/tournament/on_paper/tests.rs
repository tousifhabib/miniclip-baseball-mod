use proptest::prelude::*;

use super::*;
use crate::play::book::Figures;

fn by<'a>(rules: &'a Rules, ground: &'a Ground, innings: u32) -> Paper<'a> {
    Paper {
        rules,
        skill: Difficulty::Medium,
        innings,
        mods: OnPaper::default(),
        ground,
    }
}

/// A match between two middling sides, at places 0 and 1 of the draw.
fn level(by: &Paper<'_>, seed: u64) -> Card {
    played(7, (0, 1.0), (1, 1.0), by, seed)
}

/// Checks everything that has to be so of the card of a match on paper,
/// however it went.
fn sound(card: &Card, innings: u32) {
    assert!(card.is_sound(), "{card:?}");
    assert!(card.innings() >= innings, "over before it had to be");
    for side in [&card.home, &card.away] {
        // Every run was somebody's, and came in on somebody's turn.
        let figures = side.figures();
        assert_eq!(figures.runs, side.total());
        assert_eq!(figures.runs_in, side.total());
        assert_eq!(side.outs_an_innings, paper::OUTS);
    }
    let (home, away) = (card.home.total(), card.away.total());
    let every_half = paper::OUTS * card.innings();
    // The visitors were out in every innings there was.
    assert_eq!(card.away.outs, every_half);
    if card.unneeded {
        assert!(home > away);
        assert_eq!(card.home.outs, every_half - paper::OUTS);
    } else if home > away {
        // Level or behind going into its last half, the side at home
        // stopped the moment it was ahead.
        assert_eq!(home, away + 1);
        assert!(card.home.outs < every_half);
    } else {
        assert_eq!(card.home.outs, every_half);
    }
}

#[test]
fn a_match_on_paper_ends_as_a_match_does() {
    let (rules, ground) = (Rules::default(), Ground::default());
    for innings in [3, 5, 9] {
        for seed in 0..30 {
            let card = level(&by(&rules, &ground, innings), seed);
            assert_eq!((card.fixture, card.home.side, card.away.side), (7, 0, 1));
            sound(&card, innings);
        }
    }
}

#[test]
fn the_same_seed_plays_the_same_match_and_another_seed_another() {
    let (rules, ground) = (Rules::default(), Ground::default());
    let by = by(&rules, &ground, 5);
    assert_eq!(level(&by, 3), level(&by, 3));
    assert_ne!(level(&by, 3), level(&by, 4));
    // Which side is which is part of it: the two do not bat from the
    // same numbers.
    let card = level(&by, 3);
    assert_ne!(card.home.runs, card.away.runs);
}

#[test]
fn sides_that_make_the_same_in_every_innings_are_parted_by_a_coin_in_the_end() {
    let always_two = "[full_match.runs]\neasy = [0, 0, 1]\nmedium = [0, 0, 1]\nhard = [0, 0, 1]\n";
    let rules = Rules::layered(&[("two an innings", always_two)]).expect("rules that read");
    let ground = Ground::default();
    let mut won_at_home = 0;
    for seed in 0..24 {
        let card = level(&by(&rules, &ground, 3), seed);
        sound(&card, 3);
        assert_eq!(card.innings(), 3 + MOST_MORE + 1);
        let (home, away) = (card.home.total(), card.away.total());
        assert_eq!(home.abs_diff(away), 1);
        won_at_home += u32::from(home > away);
    }
    assert!((4..=20).contains(&won_at_home), "{won_at_home} of 24");
}

#[test]
fn the_stronger_side_wins_more_often_and_more_often_still_over_a_long_match() {
    let (rules, ground) = (Rules::default(), Ground::default());
    let wins = |innings| {
        let by = by(&rules, &ground, innings);
        (0..200)
            .filter(|&seed| {
                // At home for half of them and away for the other half.
                let card = if seed % 2 == 0 {
                    played(0, (0, 1.25), (1, 0.8), &by, seed)
                } else {
                    played(0, (1, 0.8), (0, 1.25), &by, seed)
                };
                card.winner() == 0
            })
            .count()
    };
    let (short, long) = (wins(3), wins(9));
    assert!(short > 140, "{short} of 200 over three innings");
    assert!(long > 170, "{long} of 200 over nine innings");
    assert!(long > short);
}

#[test]
fn the_mods_that_change_the_other_side_of_a_full_match_change_both_sides_here() {
    let (rules, ground) = (Rules::default(), Ground::default());
    let all = |mods: OnPaper| -> Figures {
        let by = Paper {
            mods,
            ..by(&rules, &ground, 3)
        };
        (0..20)
            .map(|seed| level(&by, seed))
            .flat_map(|card| [card.home.figures(), card.away.figures()])
            .sum()
    };
    let plain = all(OnPaper::default());
    assert_eq!(plain.stolen + plain.caught, 0);
    let zingers = all(OnPaper {
        every_hit_is_a_home_run: true,
        runners_steal: false,
    });
    assert!(
        zingers.runs > plain.runs * 2,
        "{} and {}",
        zingers.runs,
        plain.runs
    );
    let stealing = all(OnPaper {
        every_hit_is_a_home_run: false,
        runners_steal: true,
    });
    assert!(stealing.stolen + stealing.caught > 0);
}

proptest! {
    // Each case plays a whole match, every half of it over and over until
    // it comes out right.
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn any_match_played_on_paper_adds_up(
        seed: u64,
        innings in 1u32..=9,
        // The two sides' strengths, by the hundredth.
        home in 25u32..=400,
        away in 25u32..=400,
        zingers: bool,
        steals: bool,
    ) {
        let (rules, ground) = (Rules::default(), Ground::default());
        let by = Paper {
            mods: OnPaper { every_hit_is_a_home_run: zingers, runners_steal: steals },
            ..by(&rules, &ground, innings)
        };
        let strength = |hundredths: u32| hundredths as f32 / 100.0;
        let card = played(2, (5, strength(home)), (3, strength(away)), &by, seed);
        sound(&card, innings);
        prop_assert_eq!(played(2, (5, strength(home)), (3, strength(away)), &by, seed), card);
    }
}
