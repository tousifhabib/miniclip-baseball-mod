use proptest::prelude::*;

use super::*;
use crate::rules::Rules;

#[test]
fn nobody_moves_until_enough_balls_have_been_put_in_play() {
    let rules = Rules::default().shift;
    assert_eq!(Shift::of(&[], &rules).middle, 0.5);
    let few = vec![0.1; rules.least as usize - 1];
    assert_eq!(Shift::of(&few, &rules).by(), 0.0);
    assert_eq!(Shift::of(&few, &rules).words(&rules), None);
    let enough = vec![0.1; rules.least as usize];
    assert!(Shift::of(&enough, &rules).by() < 0.0);
}

#[test]
fn the_middle_goes_where_the_last_balls_went_and_no_further_than_it_may() {
    let rules = Rules::default().shift;
    let left = Shift::of(&[0.3, 0.2, 0.25, 0.3], &rules);
    assert!((left.middle - 0.2625).abs() < 1e-5, "{left:?}");
    assert_eq!(left.words(&rules), Some("SHIFT LEFT"));
    let right = Shift::of(&[0.9, 0.95, 1.0], &rules);
    assert_eq!(right.middle, 0.5 + rules.most);
    assert_eq!(right.words(&rules), Some("SHIFT RIGHT"));
    // Balls hit all round the field leave them where they were.
    let even = Shift::of(&[0.2, 0.8, 0.5, 0.45, 0.55], &rules);
    assert!(even.by().abs() < 0.01);
    assert_eq!(even.words(&rules), None);
    // Only the last few count, so going the other way brings them
    // back.
    let mut spray = vec![0.1; 20];
    spray.extend(vec![0.9; rules.memory as usize]);
    assert!(Shift::of(&spray, &rules).by() > 0.0);
}

#[test]
fn the_fielders_keep_their_order_and_stay_between_the_lines() {
    for middle in [0.2, 0.35, 0.5, 0.7, 0.8] {
        let shift = Shift { middle };
        let stood = [0.0, 0.17, 0.27, 0.48, 0.5, 0.71, 1.0];
        let stand: Vec<f32> = stood.iter().map(|&across| shift.across(across)).collect();
        assert!(stand.windows(2).all(|pair| pair[0] <= pair[1]), "{stand:?}");
        assert!(stand.iter().all(|across| (0.0..=1.0).contains(across)));
        // The one in the middle is where the middle is now.
        assert!((stand[4] - middle).abs() < 1e-6, "{stand:?}");
        // Everyone has moved the way the middle did.
        for (was, now) in stood[1..6].iter().zip(&stand[1..6]) {
            assert_eq!((now - was).signum(), (middle - 0.5).signum());
        }
    }
    let none = Shift { middle: 0.5 };
    assert_eq!(none.across(0.3), 0.3);
}

/// Where balls might have come down across the field, the latest last,
/// some of them in foul ground.
fn any_spray() -> impl Strategy<Value = Vec<f32>> {
    prop::collection::vec(-0.3f32..1.3, 0..30)
}

/// Rules the fielders might shift by: how many balls they remember and
/// how many they wait for, how far they follow them and how far they
/// may go. That last is never less than nought: a file that had it so
/// would stop the game at the sum that keeps the move within it.
fn any_rules() -> impl Strategy<Value = ShiftRules> {
    (1u32..12, 0u32..6, 0.0f32..=2.0, 0.0f32..=0.5).prop_map(|(memory, least, follow, most)| {
        ShiftRules {
            memory,
            least,
            follow,
            most,
            told: 0.05,
        }
    })
}

proptest! {
    #[test]
    fn the_middle_stays_put_for_too_few_balls_and_never_goes_further_than_it_may(
        spray in any_spray(),
        rules in any_rules(),
    ) {
        let shift = Shift::of(&spray, &rules);
        let counted = spray.len().min(rules.memory as usize);
        if counted == 0 || counted < rules.least as usize {
            prop_assert_eq!(shift.middle, 0.5);
        }
        // A hair is allowed for the sum that puts the move on the half.
        prop_assert!(shift.by().abs() <= rules.most + 1e-6, "moved by {}", shift.by());
        // And it never goes away from where the balls went on the whole,
        // a ball in foul ground counting as one on the line. A hair is
        // allowed here too, for balls that went to neither side.
        let last = &spray[spray.len() - counted..];
        let lean: f32 = last.iter().map(|across| across.clamp(0.0, 1.0) - 0.5).sum();
        prop_assert!(shift.by() * lean >= -1e-9, "{} for balls {} over", shift.by(), lean);
    }

    #[test]
    fn only_the_last_balls_the_fielders_remember_count(
        spray in any_spray(),
        older in any_spray(),
        rules in any_rules(),
    ) {
        let remembered = &spray[spray.len().saturating_sub(rules.memory as usize)..];
        let shift = Shift::of(&spray, &rules);
        prop_assert_eq!(shift, Shift::of(remembered, &rules));
        // With as many balls as they remember, none from before those
        // makes any difference.
        if remembered.len() == rules.memory as usize {
            let longer = [&older[..], remembered].concat();
            prop_assert_eq!(shift, Shift::of(&longer, &rules));
        }
    }

    #[test]
    fn wherever_the_middle_goes_the_fielders_keep_their_order_and_stay_clear_of_the_lines(
        middle in 0.0f32..=1.0,
        one in -0.2f32..1.2,
        other in -0.2f32..1.2,
    ) {
        let shift = Shift { middle };
        let (left, right) = (one.min(other), one.max(other));
        let (stands_left, stands_right) = (shift.across(left), shift.across(right));
        prop_assert!(stands_left <= stands_right, "{} and {}", stands_left, stands_right);
        let clear = MARGIN..=1.0 - MARGIN;
        prop_assert!(clear.contains(&stands_left) && clear.contains(&stands_right));
        // The one in the middle is where the middle is now, if that is
        // clear of the lines.
        prop_assert_eq!(shift.across(0.5), middle.clamp(MARGIN, 1.0 - MARGIN));
    }

    #[test]
    fn with_no_shift_on_a_fielder_clear_of_the_lines_stands_just_where_he_stood(
        across in MARGIN..=1.0 - MARGIN,
    ) {
        prop_assert_eq!(Shift { middle: 0.5 }.across(across), across);
    }
}
