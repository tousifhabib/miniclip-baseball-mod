use super::*;
use crate::play::full::Cell;
use crate::rules::Rules;
use crate::tournament::testing::{drawn, played_out};
use crate::tournament::{Format, schedule};

fn finished(format: Format, seed: u64) -> Tournament {
    played_out(drawn(format, 3, seed), &Rules::default(), |_| {})
}

#[test]
fn a_table_is_written_from_the_top_with_the_players_side_marked() {
    let league = finished(Format::League, 3);
    let rows = standing::standing(&league, 0);
    assert_eq!(rows.len(), 6);
    assert_eq!(rows.iter().filter(|row| row.ours).count(), 1);
    let champion = league.champion().expect("a champion");
    for (place, row) in rows.iter().enumerate() {
        assert_eq!(row.cells.len(), standing::HEADS.len());
        assert_eq!(row.cells[0], (place + 1).to_string());
        // Five played, won or lost, and the difference has its sign.
        let number = |column: usize| row.cells[column].parse::<i64>().expect("a number");
        assert_eq!((number(2), number(3) + number(4)), (5, 5));
        assert_eq!(number(7), number(5) - number(6));
        assert_eq!(row.cells[7].starts_with('+'), number(7) > 0);
    }
    assert_eq!(rows[0].cells[1], league.name_of(champion));
    assert_eq!(standing::title(Format::League, 0), "THE TABLE");
    assert_eq!(standing::title(Format::Groups, 1), "GROUP B");
    // Before a ball is thrown the table is there, with nothing in it.
    let fresh = standing::standing(&drawn(Format::Groups, 3, 3), 1);
    assert!(
        fresh
            .iter()
            .all(|row| row.cells[2] == "0" && row.cells[7] == "0")
    );
    assert_eq!(fresh.len(), 4);
}

#[test]
fn a_round_lists_its_fixtures_by_who_is_to_come_until_it_is_known_who() {
    let fresh = drawn(Format::Groups, 3, 3);
    let first = rounds::round(&fresh, 0);
    assert_eq!(first.len(), 4);
    assert_eq!(first.iter().filter(|listed| listed.ours).count(), 1);
    assert!(first.iter().all(|listed| listed.score.is_none()));
    assert!(first[0].words().contains(" v "));
    let names = |round: usize| -> Vec<(String, String)> {
        rounds::round(&fresh, round)
            .into_iter()
            .map(|listed| (listed.home.name, listed.away.name))
            .collect()
    };
    let of = |home: &str, away: &str| (home.to_owned(), away.to_owned());
    assert_eq!(
        names(3),
        [
            of("WINNER OF GROUP A", "SECOND OF GROUP B"),
            of("WINNER OF GROUP B", "SECOND OF GROUP A")
        ]
    );
    // A final is tossed for, so either of them may be named first.
    let last = names(4).remove(0);
    let either = [
        of("WINNER OF SEMI-FINAL 1", "WINNER OF SEMI-FINAL 2"),
        of("WINNER OF SEMI-FINAL 2", "WINNER OF SEMI-FINAL 1"),
    ];
    assert!(either.contains(&last), "{last:?}");
    let cup = rounds::round(&drawn(Format::Cup, 3, 3), 1);
    assert!(cup[0].home.name.starts_with("WINNER OF QUARTER-FINAL "));
    assert!(cup[0].home.side.is_none() && !cup[0].ours);
}

#[test]
fn a_round_that_has_been_played_lists_who_made_what() {
    let done = finished(Format::Cup, 3);
    for round in 0..3 {
        for listed in rounds::round(&done, round) {
            let card = done.card(listed.number).expect("its card");
            assert_eq!(listed.score, Some((card.home.total(), card.away.total())));
            assert_eq!(listed.home.side, Some(card.home.side));
            assert_eq!(listed.home.short, done.short_of(card.home.side));
            let words = format!(
                "{} {} - {} {}",
                done.name_of(card.home.side),
                card.home.total(),
                card.away.total(),
                done.name_of(card.away.side)
            );
            assert_eq!(listed.words(), words);
        }
    }
    assert_eq!(rounds::round(&done, 2).len(), 1);
    assert!(rounds::round(&done, 3).is_empty());
}

#[test]
fn a_match_is_told_from_its_card_as_its_own_board_told_it() {
    let done = finished(Format::League, 8);
    for card in done.cards() {
        let verdict = a_match::verdict(card, &done);
        let (won, lost) = (card.winner(), card.loser());
        let begins = format!("{} BEAT {} ", done.name_of(won), done.name_of(lost));
        assert!(verdict.starts_with(&begins), "{verdict}");
        assert_eq!(
            verdict.contains(" INNINGS"),
            card.innings() > 3,
            "{verdict}"
        );
        let (shown, [visitors, home]) = a_match::line_score(card, &done);
        assert_eq!(shown, (1, card.innings().max(3)));
        assert_eq!(
            (visitors.runs, home.runs),
            (card.away.total(), card.home.total())
        );
        assert_eq!(visitors.name, done.short_of(card.away.side));
        assert_eq!(home.ours, card.home.side == done.player());
        // The side at home has an X for a half it had no need of.
        let needless = home.cells.contains(&Cell::NotNeeded);
        assert_eq!(needless, card.unneeded);
        assert!(!visitors.cells.contains(&Cell::NotNeeded));
        // Both battings add up, and are told with the other side's
        // pitcher.
        for at_home in [true, false] {
            let (rows, lines) = a_match::batting(card, at_home, &done);
            assert_eq!(rows.len(), 10);
            let (side, fielding) = if at_home {
                (&card.home, &card.away)
            } else {
                (&card.away, &card.home)
            };
            let all = side.figures();
            assert_eq!(rows[9][0], "ALL");
            assert_eq!(rows[9][3], all.hits.to_string());
            let hits: u32 = rows[..9]
                .iter()
                .map(|row| row[3].parse::<u32>().unwrap())
                .sum();
            assert_eq!(hits, all.hits);
            let pitcher = format!("{} PITCHER: ", done.short_of(fielding.side));
            assert!(lines[1].starts_with(&pitcher), "{}", lines[1]);
        }
        let (hitting, pitching) = a_match::figures(card);
        assert_eq!(hitting[7].0, "HOME RUNS");
        let home_runs = (card.away.figures().home_runs, card.home.figures().home_runs);
        assert_eq!(hitting[7].1, home_runs.0.to_string());
        assert_eq!(hitting[7].2, home_runs.1.to_string());
        assert_eq!(pitching[0].0, "PITCHES SEEN");
    }
}

#[test]
fn what_the_sides_did_adds_up_to_what_was_done_against_them() {
    for format in Format::ALL {
        let done = finished(format, 5);
        let sides = format.sides();
        let all: Vec<Summed> = (0..sides).map(|side| done.summed(side)).collect();
        let sum = |of: fn(&Summed) -> u32| all.iter().map(of).sum::<u32>();
        let cards = schedule::ties(format).len() as u32;
        assert_eq!(sum(|summed| summed.matches), 2 * cards);
        assert_eq!(sum(|summed| summed.won), cards);
        assert_eq!(
            sum(|summed| summed.own.runs),
            sum(|summed| summed.against.runs)
        );
        assert_eq!(
            sum(|summed| summed.own.all.hits),
            sum(|summed| summed.against.all.hits)
        );
        assert_eq!(
            sum(|summed| summed.own.outs),
            sum(|summed| summed.against.outs)
        );
        for (side, summed) in all.iter().enumerate() {
            // The whole side is its nine places, but for the runners it
            // left on, who are nobody's.
            let places: Figures = summed.own.places.iter().copied().sum();
            let but_left = Figures {
                left: 0,
                ..summed.own.all
            };
            assert_eq!(places, but_left, "{format:?}, side {side}");
            assert_eq!(done.cards_of(side).count() as u32, summed.matches);
        }
    }
    // In a league every side's runs are the table's.
    let league = finished(Format::League, 5);
    for row in league.table(0) {
        let summed = league.summed(row.side);
        assert_eq!(
            (summed.own.runs, summed.against.runs),
            (row.runs_for, row.runs_against)
        );
        assert_eq!((summed.matches, summed.won), (row.played, row.won));
    }
}

#[test]
fn a_side_is_told_by_its_results_its_batting_and_its_figures() {
    let done = finished(Format::Groups, 5);
    for side in 0..8 {
        let summed = done.summed(side);
        let heading = a_side::heading(&done, side);
        let begins = format!("{}: WON {}, LOST ", done.name_of(side), summed.won);
        assert!(heading.starts_with(&begins), "{heading}");
        let results = a_side::results(&done, side);
        // Every match it played is there, and nothing is left to play.
        assert_eq!(results.len() as u32, summed.matches);
        assert!(results.len() >= 3);
        let won = results
            .iter()
            .filter(|row| row.cells[3].starts_with("WON "));
        assert_eq!(won.count() as u32, summed.won);
        assert!(
            results
                .iter()
                .all(|row| row.cells.len() == a_side::RESULT_HEADS.len())
        );
        // The ones against the player's own side are marked.
        let players = done.name_of(done.player());
        assert!(
            results
                .iter()
                .all(|row| row.ours == (row.cells[2] == players))
        );
        let (rows, lines) = a_side::batting(&done, side);
        assert_eq!(rows[9][3], summed.own.all.hits.to_string());
        assert!(lines[1].starts_with("PITCHED TO: "), "{}", lines[1]);
        let (hitting, _) = a_side::figures(&done, side);
        assert_eq!(hitting[7].1, summed.own.all.home_runs.to_string());
        assert_eq!(hitting[7].2, summed.against.all.home_runs.to_string());
    }
    // A side that has yet to play has its fixtures still, to play.
    let fresh = drawn(Format::League, 3, 5);
    let results = a_side::results(&fresh, 0);
    assert_eq!(results.len(), 5);
    assert!(results.iter().all(|row| row.cells[3] == "TO PLAY"));
    assert_eq!(results[0].cells[0], "ROUND 1");
}
