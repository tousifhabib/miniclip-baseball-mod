//! The best of a tournament so far: its batters, each of whom is a side
//! and a place in its order, there being no names, and its sides.

use crate::play::book::{Figures, ORDER, average, tenths};
use crate::rules::TournamentRules;
use crate::tournament::Tournament;
use crate::tournament::stats::Summed;

/// One of the best at something: who, and how much of it they have.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Leader {
    pub who: String,
    pub has: String,
    /// Whether it is the player's own side, or one of its batters.
    pub ours: bool,
}

/// The best at one thing, the best of them first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct List {
    pub of: &'static str,
    pub best: Vec<Leader>,
}

/// Something to be best at: what it is called, how much of it a batter or
/// a side has, if anything, and how that is written. The more the better,
/// unless `fewest`.
struct Count<T> {
    of: &'static str,
    has: fn(&T) -> Option<f32>,
    written: fn(f32) -> String,
    fewest: bool,
}

/// A whole number of something, where there is any of it.
fn some(count: u32) -> Option<f32> {
    (count > 0).then_some(count as f32)
}

fn whole(count: f32) -> String {
    format!("{count:.0}")
}

fn feet(count: f32) -> String {
    format!("{count:.0} FT")
}

/// The `most` best of `all` at one thing. Those level at it stay in the
/// order they came in, which is the order of the draw.
fn best<T>(all: &[(String, bool, T)], count: &Count<T>, most: usize) -> List {
    let mut has: Vec<(&String, bool, f32)> = all
        .iter()
        .filter_map(|(who, ours, each)| Some((who, *ours, (count.has)(each)?)))
        .collect();
    has.sort_by(|one, other| {
        let order = one.2.total_cmp(&other.2);
        if count.fewest { order } else { order.reverse() }
    });
    let best = has.into_iter().take(most).map(|(who, ours, has)| Leader {
        who: who.clone(),
        has: (count.written)(has),
        ours,
    });
    List {
        of: count.of,
        best: best.collect(),
    }
}

/// The best batters at each of several things, `most` of them for each. A
/// batter is listed by his averages only once he has had a turn for every
/// so many innings his side has batted in, as the rules say.
pub fn batters(tournament: &Tournament, rules: &TournamentRules, most: usize) -> Vec<List> {
    /// A batter's figures, with whether he has batted enough for his
    /// averages to count.
    type Batter = (Figures, bool);
    let mut all: Vec<(String, bool, Batter)> = Vec::new();
    for side in 0..tournament.sides().len() {
        let summed = tournament.summed(side);
        for order in 0..ORDER {
            let figures = summed.own.places[order];
            let enough =
                figures.turns > 0 && figures.turns * rules.innings_a_turn >= summed.own.halves;
            let who = format!("{} {}", tournament.short_of(side), order + 1);
            all.push((who, side == tournament.player(), (figures, enough)));
        }
    }
    let counts: [Count<Batter>; 6] = [
        Count {
            of: "AVERAGE",
            has: |(figures, enough)| figures.average().filter(|_| *enough),
            written: |value| average(Some(value)),
            fewest: false,
        },
        Count {
            of: "ON BASE + SLUGGING",
            has: |(figures, enough)| figures.on_base_plus_slugging().filter(|_| *enough),
            written: |value| average(Some(value)),
            fewest: false,
        },
        Count {
            of: "HOME RUNS",
            has: |(figures, _)| some(figures.home_runs),
            written: whole,
            fewest: false,
        },
        Count {
            of: "RUNS BATTED IN",
            has: |(figures, _)| some(figures.runs_in),
            written: whole,
            fewest: false,
        },
        Count {
            of: "HITS",
            has: |(figures, _)| some(figures.hits),
            written: whole,
            fewest: false,
        },
        Count {
            of: "LONGEST HIT",
            has: |(figures, _)| some(figures.longest),
            written: feet,
            fewest: false,
        },
    ];
    counts.iter().map(|count| best(&all, count, most)).collect()
}

/// The best sides at each of several things, `most` of them for each. A
/// side that has yet to play is in none of them.
pub fn sides(tournament: &Tournament, most: usize) -> Vec<List> {
    let all: Vec<(String, bool, Summed)> = (0..tournament.sides().len())
        .map(|side| {
            let who = tournament.name_of(side).to_owned();
            (who, side == tournament.player(), tournament.summed(side))
        })
        .filter(|(_, _, summed)| summed.matches > 0)
        .collect();
    let counts: [Count<Summed>; 6] = [
        Count {
            of: "RUNS A MATCH",
            has: |summed| Some(summed.own.runs as f32 / summed.matches as f32),
            written: |value| tenths(Some(value)),
            fewest: false,
        },
        Count {
            of: "FEWEST AGAINST A MATCH",
            has: |summed| Some(summed.against.runs as f32 / summed.matches as f32),
            written: |value| tenths(Some(value)),
            fewest: true,
        },
        Count {
            of: "AVERAGE",
            has: |summed| summed.own.all.average(),
            written: |value| average(Some(value)),
            fewest: false,
        },
        Count {
            of: "HOME RUNS",
            has: |summed| some(summed.own.all.home_runs),
            written: whole,
            fewest: false,
        },
        Count {
            of: "STRIKEOUTS THROWN",
            has: |summed| some(summed.against.all.strikeouts),
            written: whole,
            fewest: false,
        },
        Count {
            of: "MATCHES WON",
            has: |summed| some(summed.won),
            written: whole,
            fewest: false,
        },
    ];
    counts.iter().map(|count| best(&all, count, most)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rules;
    use crate::tournament::Format;
    use crate::tournament::testing::{drawn, played_out};

    #[test]
    fn the_best_batters_are_listed_from_the_best_and_only_with_turns_enough() {
        let rules = Rules::default();
        let done = played_out(drawn(Format::League, 3, 4), &rules, |_| {});
        let lists = batters(&done, &rules.tournament, 5);
        let of: Vec<&str> = lists.iter().map(|list| list.of).collect();
        assert_eq!(
            of,
            [
                "AVERAGE",
                "ON BASE + SLUGGING",
                "HOME RUNS",
                "RUNS BATTED IN",
                "HITS",
                "LONGEST HIT"
            ]
        );
        for list in &lists {
            assert_eq!(list.best.len(), 5, "{}", list.of);
            // A batter is his side's short name and his place in its
            // order.
            for leader in &list.best {
                let (side, place) = leader.who.split_once(' ').expect("a side and a place");
                assert!(done.sides().iter().any(|entrant| entrant.short == side));
                assert!((1..=9).contains(&place.parse::<u32>().expect("a place")));
                assert_eq!(leader.ours, side == done.short_of(done.player()));
            }
        }
        // From the most down.
        let hits: Vec<u32> = lists[4]
            .best
            .iter()
            .map(|leader| leader.has.parse().unwrap())
            .collect();
        assert!(hits.windows(2).all(|pair| pair[0] >= pair[1]), "{hits:?}");
        assert!(lists[5].best[0].has.ends_with(" FT"));
        // With turns asked for that nobody has had, nobody has an
        // average to be listed by, and the counts are as they were.
        let mut strict = rules.tournament.clone();
        strict.innings_a_turn = 1;
        let lists = batters(&done, &strict, 5);
        assert!(lists[0].best.is_empty() && lists[1].best.is_empty());
        assert_eq!(lists[4].best.len(), 5);
    }

    #[test]
    fn before_a_ball_is_thrown_nobody_is_the_best_at_anything() {
        let rules = Rules::default();
        let fresh = drawn(Format::Cup, 3, 4);
        assert!(
            batters(&fresh, &rules.tournament, 5)
                .iter()
                .all(|list| list.best.is_empty())
        );
        assert!(sides(&fresh, 5).iter().all(|list| list.best.is_empty()));
    }

    #[test]
    fn the_best_sides_are_listed_by_what_they_made_and_what_they_let_in() {
        let rules = Rules::default();
        let done = played_out(drawn(Format::League, 3, 4), &rules, |_| {});
        let lists = sides(&done, 3);
        assert_eq!(lists.len(), 6);
        assert!(lists.iter().all(|list| list.best.len() == 3), "{lists:?}");
        let numbers = |list: &List| -> Vec<f32> {
            list.best
                .iter()
                .map(|leader| leader.has.parse().unwrap())
                .collect()
        };
        let (made, let_in) = (numbers(&lists[0]), numbers(&lists[1]));
        assert!(made.windows(2).all(|pair| pair[0] >= pair[1]), "{made:?}");
        assert!(
            let_in.windows(2).all(|pair| pair[0] <= pair[1]),
            "{let_in:?}"
        );
        // The side that won most won the league, or was level with it.
        let champion = done.name_of(done.champion().expect("a champion"));
        let most = &lists[5].best[0];
        let champions = lists[5].best.iter().find(|leader| leader.who == champion);
        assert_eq!(champions.map(|leader| &leader.has), Some(&most.has));
        let ours: usize = lists
            .iter()
            .flat_map(|list| &list.best)
            .filter(|leader| leader.ours)
            .count();
        let players = done.name_of(done.player());
        let named: usize = lists
            .iter()
            .flat_map(|list| &list.best)
            .filter(|leader| leader.who == players)
            .count();
        assert_eq!(ours, named);
    }
}
