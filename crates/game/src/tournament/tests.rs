use std::collections::BTreeSet;

use super::testing::{drawn, played_out, result};
use super::*;
use crate::rules::Rules;

/// Plays a tournament out with nothing looked at on the way.
fn to_the_end(format: Format, seed: u64) -> Tournament {
    played_out(drawn(format, 3, seed), &Rules::default(), |_| {})
}

#[test]
fn a_tournament_of_any_shape_is_played_to_a_champion() {
    for format in Format::ALL {
        for seed in 0..10 {
            let tournament = to_the_end(format, seed);
            assert!(tournament.is_over() && tournament.next().is_none());
            // Every fixture was played, once.
            let fixtures = tournament.fixtures();
            let numbers: BTreeSet<usize> =
                tournament.cards().iter().map(|card| card.fixture).collect();
            assert_eq!(numbers.len(), fixtures.len());
            assert_eq!(tournament.cards().len(), fixtures.len());
            for fixture in fixtures {
                let card = tournament.card(fixture.number).expect("its card");
                assert!(card.is_sound());
                assert_eq!(fixture.sides(), Some((card.home.side, card.away.side)));
            }
            let champion = tournament.champion().expect("a champion");
            assert!(champion < format.sides());
            assert!(tournament.players_end().is_some());
        }
    }
}

#[test]
fn nothing_is_won_until_everything_is_played() {
    let rules = Rules::default();
    for format in Format::ALL {
        let mut turns = 0;
        played_out(drawn(format, 3, 4), &rules, |tournament| {
            turns += 1;
            assert!(!tournament.is_over());
            assert_eq!(tournament.champion(), None);
            assert_eq!(tournament.players_end(), None);
        });
        // The player has a fixture in the first round whatever the shape.
        assert!(turns >= 1, "{format:?}");
    }
}

#[test]
fn whenever_the_player_comes_to_play_every_side_of_a_table_has_played_as_many() {
    let rules = Rules::default();
    for format in [Format::Groups, Format::League] {
        for seed in 0..6 {
            let mut round = 0;
            played_out(drawn(format, 3, seed), &rules, |tournament| {
                let next = tournament.next().expect("a fixture to play");
                assert!(next.has(tournament.player()));
                assert_eq!(next.round, round, "a round was missed");
                // Rounds of a table that are done are done for everybody.
                let done = round.min(format.in_a_group() - 1) as u32;
                for group in 0..format.groups() {
                    for row in tournament.table(group) {
                        assert_eq!(row.played, done, "{format:?}, round {round}");
                    }
                }
                round += 1;
            });
        }
    }
}

#[test]
fn the_first_two_of_each_group_go_on_and_the_winners_of_those_meet_for_it() {
    for seed in 0..10 {
        let tournament = to_the_end(Format::Groups, seed);
        let top = |group: usize, place: usize| tournament.table(group)[place].side;
        let fixtures = tournament.fixtures();
        // The winner of a group is at home to the runner-up of the other.
        assert_eq!(fixtures[12].sides(), Some((top(0, 0), top(1, 1))));
        assert_eq!(fixtures[13].sides(), Some((top(1, 0), top(0, 1))));
        let winner = |fixture: usize| tournament.card(fixture).expect("its card").winner();
        let last: BTreeSet<usize> = fixtures[14]
            .sides()
            .into_iter()
            .flat_map(|(a, b)| [a, b])
            .collect();
        assert_eq!(last, BTreeSet::from([winner(12), winner(13)]));
        assert_eq!(tournament.champion(), Some(winner(14)));
        // The groups' tables are of the groups' own matches and no
        // others.
        for group in 0..2 {
            assert!(tournament.table(group).iter().all(|row| row.played == 3));
            assert_eq!(tournament.group_of(top(group, 0)), Some(group));
        }
    }
}

#[test]
fn a_league_is_won_by_the_top_of_its_table() {
    for seed in 0..10 {
        let tournament = to_the_end(Format::League, seed);
        let table = tournament.table(0);
        assert_eq!(tournament.champion(), Some(table[0].side));
        let place = table.iter().position(|row| row.side == tournament.player());
        let end = match place {
            Some(0) => End::Champion,
            Some(place) => End::Placed(place + 1),
            None => panic!("the player is not in the table"),
        };
        assert_eq!(tournament.players_end(), Some(end));
    }
}

#[test]
fn in_a_cup_the_loser_of_a_tie_plays_no_more() {
    let mut ends = BTreeSet::new();
    for seed in 0..40 {
        let tournament = to_the_end(Format::Cup, seed);
        let mut out = BTreeSet::new();
        for fixture in tournament.fixtures() {
            let (home, away) = fixture.sides().expect("both its sides");
            assert!(!out.contains(&home) && !out.contains(&away));
            out.insert(tournament.card(fixture.number).expect("its card").loser());
        }
        // Seven of the eight have lost once, and the other has won it.
        assert_eq!(out.len(), 7);
        assert!(!out.contains(&tournament.champion().expect("a champion")));
        let end = tournament.players_end().expect("an end");
        assert!(!matches!(end, End::Placed(_)));
        ends.insert(format!("{end:?}"));
    }
    // Over forty cups the player's side goes out everywhere and wins one.
    assert_eq!(ends.len(), 4, "{ends:?}");
}

#[test]
fn a_side_that_does_not_come_through_its_group_is_placed_in_it() {
    let placed = (0..40).find_map(|seed| {
        let tournament = to_the_end(Format::Groups, seed);
        match tournament.players_end() {
            Some(End::Placed(place)) => Some((tournament, place)),
            _ => None,
        }
    });
    let (tournament, place) = placed.expect("a group the player did not come through");
    assert!(place == 3 || place == 4);
    let group = tournament.group_of(tournament.player()).expect("its group");
    assert_eq!(tournament.table(group)[place - 1].side, tournament.player());
}

#[test]
fn the_players_fixture_is_a_full_match_of_the_tournaments_length_against_a_leant_side() {
    let rules = Rules::default();
    let tournament = drawn(Format::League, 5, 2);
    let to_play = tournament
        .to_play(&rules)
        .expect("the player's first fixture");
    let next = tournament.next().expect("a fixture");
    assert_eq!(
        (to_play.fixture, to_play.round),
        (next.number, Round::Of(1))
    );
    assert_eq!(next.against(tournament.player()), Some(to_play.against));
    assert_eq!(to_play.at_home, next.home == Some(tournament.player()));
    assert_eq!(to_play.skill, tournament.setup().skill);
    let against = &tournament.sides()[to_play.against];
    let strength = rules.tournament.strength_of(&against.key);
    assert_eq!(
        to_play.rules,
        strength::match_rules(&rules.full_match, strength, 5)
    );
}

#[test]
fn a_fixture_begun_again_is_another_game() {
    let rules = Rules::default();
    let mut tournament = drawn(Format::Cup, 3, 2);
    let first = tournament.to_play(&rules).expect("a fixture");
    assert_eq!(tournament.to_play(&rules).as_ref(), Some(&first));
    tournament.begin();
    let again = tournament.to_play(&rules).expect("the same fixture");
    assert_ne!(again.seed, first.seed);
    // It is the same fixture in everything else.
    let but_for_the_seed = ToPlay {
        seed: first.seed,
        ..again
    };
    assert_eq!(but_for_the_seed, first);
    assert!(tournament.cards().is_empty());
}

#[test]
fn only_the_card_of_the_next_fixture_is_taken() {
    let mut tournament = drawn(Format::League, 3, 2);
    let next = tournament.next().expect("a fixture");
    let (home, away) = next.sides().expect("both its sides");
    let sound = result(next.number, (home, 3), (away, 1));
    let other = (0..6)
        .find(|side| *side != home && *side != away)
        .expect("a third side");
    let wrong = [
        (
            result(next.number + 1, (home, 3), (away, 1)),
            Misfit::OutOfTurn,
        ),
        (
            result(next.number, (away, 3), (home, 1)),
            Misfit::OtherSides,
        ),
        (
            result(next.number, (home, 3), (other, 1)),
            Misfit::OtherSides,
        ),
        (result(next.number, (home, 2), (away, 2)), Misfit::Unsound),
    ];
    for (card, misfit) in wrong {
        assert_eq!(tournament.take(card), Err(misfit));
        assert!(tournament.cards().is_empty());
    }
    assert_eq!(tournament.take(sound.clone()), Ok(()));
    assert_eq!(tournament.cards(), std::slice::from_ref(&sound));
    // And not a second time.
    assert_eq!(tournament.take(sound), Err(Misfit::OutOfTurn));
}

#[test]
fn the_same_seed_gives_the_same_tournament_and_another_seed_another() {
    for format in Format::ALL {
        assert_eq!(to_the_end(format, 6), to_the_end(format, 6));
        assert_ne!(to_the_end(format, 6), to_the_end(format, 7));
    }
    let champions: BTreeSet<String> = (0..40)
        .map(|seed| {
            let tournament = to_the_end(Format::Cup, seed);
            let champion = tournament.champion().expect("a champion");
            tournament.sides()[champion].key.clone()
        })
        .collect();
    assert!(champions.len() >= 5, "only {champions:?} ever win it");
}
