//! The count on the batter at the plate: his strikes and his balls.

/// The count on the batter at the plate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Count {
    pub strikes: u32,
    pub balls: u32,
}

impl Count {
    /// Whether nothing has been called on this batter yet.
    pub fn is_clean(self) -> bool {
        self.strikes + self.balls == 0
    }

    /// A strike is called or swung at. On a golden ball one strike is all
    /// the strikes there are, however many are `allowed`.
    pub fn strike(&mut self, golden: bool, allowed: u32) {
        self.strikes += 1;
        if golden {
            self.strikes = self.strikes.max(allowed);
        }
    }

    /// A foul is a strike, but never the last one. Says whether it was
    /// counted as one.
    pub fn foul(&mut self, allowed: u32) -> bool {
        let counted = self.strikes + 1 < allowed;
        if counted {
            self.strikes += 1;
        }
        counted
    }

    /// Whether the batter is out on strikes, with this many allowed.
    pub fn is_out(self, allowed: u32) -> bool {
        self.strikes >= allowed
    }

    /// Whether one more strike puts the batter out.
    pub fn is_one_strike_from_out(self, allowed: u32) -> bool {
        self.strikes + 1 == allowed
    }

    /// Whether the batter walks, with this many balls for a walk.
    pub fn is_a_walk(self, balls_for_a_walk: u32) -> bool {
        self.balls >= balls_for_a_walk
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn a_foul_is_a_strike_but_never_the_last_one() {
        let mut count = Count::default();
        assert!(count.is_clean());
        assert!(count.foul(3));
        assert!(count.foul(3));
        assert!(!count.foul(3));
        assert_eq!(count.strikes, 2);
        assert!(count.is_one_strike_from_out(3));
        assert!(!count.is_out(3));
        // With one strike and out, a foul is no strike at all.
        let mut sudden = Count::default();
        assert!(!sudden.foul(1));
        assert!(sudden.is_clean());
    }

    #[test]
    fn a_strike_on_a_golden_ball_is_all_the_strikes_there_are() {
        let mut count = Count::default();
        count.strike(false, 3);
        assert_eq!(count.strikes, 1);
        assert!(!count.is_out(3));
        count.strike(true, 3);
        assert!(count.is_out(3));
        assert_eq!(count.strikes, 3);
    }

    #[test]
    fn four_balls_are_a_walk() {
        let mut count = Count::default();
        for _ in 0..3 {
            count.balls += 1;
            assert!(!count.is_a_walk(4));
        }
        count.balls += 1;
        assert!(count.is_a_walk(4));
    }
}
