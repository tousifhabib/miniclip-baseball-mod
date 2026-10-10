//! The fixtures of a tournament as far as they are known: who is to meet
//! whom and where, and which of them is next.

use super::Tournament;
use super::format::Round;
use super::schedule::{self, Slot, Tie};
use super::seeds;

/// A fixture: a tie of the tournament, with the two sides that are to
/// play it once it is known who they are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fixture {
    /// Its number among all the fixtures, the first being nought.
    pub number: usize,
    /// The round it is in, the first being nought, and what that round
    /// is.
    pub round: usize,
    pub kind: Round,
    /// The group it is a fixture of, if it is one of a group's.
    pub group: Option<usize>,
    /// The side at home and the side away, each by its place in the draw.
    pub home: Option<usize>,
    pub away: Option<usize>,
    /// Where each of them comes from, which is all there is to call it by
    /// until it is known who it is.
    pub home_from: Slot,
    pub away_from: Slot,
}

impl Fixture {
    /// Whether this side is one of the two, as far as they are known.
    pub fn has(&self, side: usize) -> bool {
        self.home == Some(side) || self.away == Some(side)
    }

    /// The side at home and the side away, once both are known.
    pub fn sides(&self) -> Option<(usize, usize)> {
        Some((self.home?, self.away?))
    }

    /// Who this side is to play, if it is one of the two and the other is
    /// known.
    pub fn against(&self, side: usize) -> Option<usize> {
        match self.sides()? {
            (home, away) if home == side => Some(away),
            (home, away) if away == side => Some(home),
            _ => None,
        }
    }
}

impl Tournament {
    /// Which side fills a place in a tie, once that is settled: when the
    /// group it is to finish in has played all its ties, or the tie it is
    /// to win has been played.
    fn settled(&self, slot: Slot, ties: &[Tie]) -> Option<usize> {
        match slot {
            Slot::Place(place) => Some(place),
            Slot::WinnerOf(tie) => self.card(tie).map(super::Card::winner),
            Slot::Finisher { group, place } => {
                let of_the_group = |tie: &&Tie| tie.group == Some(group);
                let all = ties.iter().filter(of_the_group).count();
                let table = self.table(group);
                let played: u32 = table.iter().map(|row| row.played).sum();
                // Every tie is two sides' match.
                (played as usize == all * 2).then(|| table.get(place).map(|row| row.side))?
            }
        }
    }

    /// Every fixture of the tournament, in the order of their numbers.
    pub fn fixtures(&self) -> Vec<Fixture> {
        let format = self.setup.format;
        let ties = schedule::ties(format);
        ties.iter()
            .enumerate()
            .map(|(number, tie)| {
                let first = self.settled(tie.first, &ties);
                let second = self.settled(tie.second, &ties);
                // The first-named is at home, unless a coin says not.
                let swapped = tie.tossed && !seeds::first_named_is_at_home(self.seed, number);
                let ((home, home_from), (away, away_from)) = if swapped {
                    ((second, tie.second), (first, tie.first))
                } else {
                    ((first, tie.first), (second, tie.second))
                };
                Fixture {
                    number,
                    round: tie.round,
                    kind: format.round(tie.round),
                    group: tie.group,
                    home,
                    away,
                    home_from,
                    away_from,
                }
            })
            .collect()
    }

    /// The fixture to be played next. The rounds are played in turn, and
    /// in each the player's own fixture comes first, so that whenever the
    /// player comes to bat every other side has played as many matches.
    /// `None` when every fixture has been played.
    pub fn next(&self) -> Option<Fixture> {
        let fixtures = self.fixtures();
        let to_come = |fixture: &&Fixture| self.card(fixture.number).is_none();
        let round = fixtures.iter().find(to_come)?.round;
        let mut of_the_round = fixtures
            .iter()
            .filter(|fixture| fixture.round == round)
            .filter(to_come);
        let players = of_the_round
            .clone()
            .find(|fixture| fixture.has(self.player));
        players.or_else(|| of_the_round.next()).copied()
    }

    /// Whether every fixture has been played.
    pub fn is_over(&self) -> bool {
        self.next().is_none()
    }
}
