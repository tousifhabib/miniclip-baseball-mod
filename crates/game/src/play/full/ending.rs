//! How a match ends, whichever side is whose: when the side at home has
//! no need of its half, how many runs it stops at, and when the match is
//! decided.
//!
//! `last` is whether the innings in hand is the last there has to be, or
//! one added to it. The runs are each side's in the whole match so far.

/// Whether the home side, about to bat in the bottom of an innings, has
/// won already: ahead, with every innings there has to be all but played.
pub(crate) fn home_has_no_need_to_bat(last: bool, visitors: u32, home: u32) -> bool {
    last && home > visitors
}

/// What the home side makes of a half that left to itself would come to
/// `drawn` runs, and whether that wins it the match. In the last innings
/// it stops as soon as it is ahead.
pub(crate) fn home_makes(last: bool, visitors: u32, home: u32, drawn: u32) -> (u32, bool) {
    if !last {
        return (drawn, false);
    }
    let made = drawn.min((visitors + 1).saturating_sub(home));
    (made, home + made > visitors)
}

/// Whether the match is over once the bottom of an innings has been
/// played: it is when that was the last and the sides are not level.
pub(crate) fn decided(last: bool, visitors: u32, home: u32) -> bool {
    last && visitors != home
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_in_the_last_innings_is_being_ahead_enough() {
        assert!(home_has_no_need_to_bat(true, 3, 4));
        assert!(!home_has_no_need_to_bat(true, 4, 4));
        assert!(!home_has_no_need_to_bat(true, 5, 4));
        assert!(!home_has_no_need_to_bat(false, 0, 9));
    }

    #[test]
    fn the_home_side_stops_as_soon_as_it_is_ahead_in_the_last_innings() {
        // Before the last, an innings runs its course and wins nothing.
        assert_eq!(home_makes(false, 2, 0, 6), (6, false));
        // Two behind, three wins it, however many there might have been.
        assert_eq!(home_makes(true, 5, 3, 6), (3, true));
        assert_eq!(home_makes(true, 5, 3, 3), (3, true));
        // Fewer than that and the side is out, level or still behind.
        assert_eq!(home_makes(true, 5, 3, 2), (2, false));
        assert_eq!(home_makes(true, 5, 3, 0), (0, false));
        // Level, one run is the match.
        assert_eq!(home_makes(true, 4, 4, 2), (1, true));
    }

    #[test]
    fn a_match_is_decided_when_the_last_innings_leaves_one_side_ahead() {
        assert!(decided(true, 3, 2));
        assert!(decided(true, 2, 3));
        assert!(!decided(true, 3, 3));
        assert!(!decided(false, 7, 0));
    }
}
