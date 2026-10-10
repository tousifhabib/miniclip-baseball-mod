//! Everyone meets everyone once: the rounds of a group or of a league.

/// The rounds in which `count` sides, an even number of them, each meet
/// every other once. A side is its place among them, the first being
/// nought, and a tie is the side at home and then the side away. An odd
/// number of sides, or fewer than two, have no rounds.
pub fn rounds(count: usize) -> Vec<Vec<(usize, usize)>> {
    if count < 2 || !count.is_multiple_of(2) {
        return Vec::new();
    }
    // The last place stays where it is, and the rest go round it: in each
    // round it meets the next of them, and the others pair off to either
    // side of that one. Who is at home goes turn and turn about, so that
    // nobody is at home or away much more than anybody else.
    let last = count - 1;
    (0..last)
        .map(|round| {
            let mut ties = vec![if round.is_multiple_of(2) {
                (last, round)
            } else {
                (round, last)
            }];
            for step in 1..count / 2 {
                let one = (round + step) % last;
                let other = (round + last - step) % last;
                ties.push(if step.is_multiple_of(2) {
                    (other, one)
                } else {
                    (one, other)
                });
            }
            ties
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_sides_meet_over_three_rounds() {
        assert_eq!(
            rounds(4),
            [[(3, 0), (1, 2)], [(1, 3), (2, 0)], [(3, 2), (0, 1)]]
        );
    }

    #[test]
    fn two_sides_meet_once_and_an_odd_number_not_at_all() {
        assert_eq!(rounds(2), [[(1, 0)]]);
        assert!(rounds(5).is_empty() && rounds(1).is_empty() && rounds(0).is_empty());
    }
}
