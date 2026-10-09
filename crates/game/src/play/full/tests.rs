use super::*;
use crate::play::book::ORDER;
use crate::rules::Rules;

/// A match in which the other side makes `their` runs in every innings.
fn against(home: bool, their: u32, innings: u32) -> FullMatch {
    let mut rules = Rules::default().full_match;
    rules.innings = innings;
    let mut chances = vec![0; their as usize + 1];
    chances[their as usize] = 1;
    rules.runs.easy = chances.clone();
    rules.runs.medium = chances.clone();
    rules.runs.hard = chances;
    FullMatch::new(
        home,
        &rules,
        Difficulty::Medium,
        false,
        None,
        Ground::default(),
        7,
    )
}

#[test]
fn the_other_sides_runs_follow_the_chances_given() {
    let rules = Rules::default().full_match;
    // The first share of the draw is for no runs, and the last for the
    // most there can be.
    assert_eq!(rules.runs_for(Difficulty::Easy, 0.0), 0);
    assert_eq!(rules.runs_for(Difficulty::Easy, 0.999), 5);
    assert_eq!(rules.runs_for(Difficulty::Hard, 0.999), 8);
    // On easy, forty-four innings in a hundred are noughts, and the
    // next thirty are ones.
    assert_eq!(rules.runs_for(Difficulty::Easy, 0.43), 0);
    assert_eq!(rules.runs_for(Difficulty::Easy, 0.45), 1);
    assert_eq!(rules.runs_for(Difficulty::Easy, 0.75), 2);
    // The harder the level, the more runs on the whole.
    let mean = |difficulty| {
        (0..1000)
            .map(|step| rules.runs_for(difficulty, step as f32 / 1000.0))
            .sum::<u32>()
    };
    assert!(mean(Difficulty::Easy) < mean(Difficulty::Medium));
    assert!(mean(Difficulty::Medium) < mean(Difficulty::Hard));
}

#[test]
fn away_the_match_is_won_by_being_ahead_when_the_home_side_is_out() {
    let mut full = against(false, 1, 3);
    assert!(!full.at_home() && !full.sudden());
    assert_eq!(full.batting_in(), "top of innings 1");
    assert_eq!(full.side_out(2), Next::Bat);
    assert_eq!((full.ours(), full.theirs()), (2, 1));
    assert_eq!(full.side_out(0), Next::Bat);
    // Three to two up going into their last half, in which they make
    // only the one: level, so on it goes.
    assert_eq!(full.side_out(1), Next::Bat);
    assert_eq!((full.ours(), full.theirs()), (3, 3));
    assert_eq!(full.batting_in(), "top of innings 4");
    assert!(
        full.report()
            .lines
            .contains(&"THE MATCH GOES ON UNTIL IT IS WON".to_owned())
    );
    // Two more is one more than they can answer with.
    assert_eq!(full.side_out(2), Next::Won);
    assert_eq!(full.describe(), "away, visitors 2 0 1 2, home side 1 1 1 1");
    assert_eq!(full.verdict(), "YOU WON 5 - 4 IN 4 INNINGS");
}

#[test]
fn away_the_home_side_stops_as_soon_as_it_is_ahead() {
    let mut full = against(false, 4, 2);
    assert_eq!(full.side_out(6), Next::Bat);
    // Six to four behind in the bottom of the last, they would make
    // four, and stop at the three that win it.
    assert_eq!(full.side_out(0), Next::Lost);
    assert_eq!(full.describe(), "away, visitors 6 0, home side 4 3");
    assert_eq!(full.verdict(), "YOU LOST 6 - 7");
}

#[test]
fn away_the_home_side_does_not_bat_last_when_it_is_ahead() {
    let mut full = against(false, 3, 2);
    assert_eq!(full.side_out(1), Next::Bat);
    assert_eq!(full.side_out(1), Next::Lost);
    assert_eq!(full.describe(), "away, visitors 1 1, home side 3 x");
    let [visitors, home] = full.lines();
    assert_eq!(visitors.cells, [Cell::Runs(1), Cell::Runs(1)]);
    assert_eq!(home.cells, [Cell::Runs(3), Cell::NotNeeded]);
    assert_eq!((visitors.runs, home.runs), (2, 3));
}

#[test]
fn at_home_the_visitors_have_batted_before_the_first_ball() {
    let full = against(true, 2, 9);
    assert!(full.at_home());
    assert_eq!((full.ours(), full.theirs()), (0, 2));
    assert_eq!(full.batting_in(), "bottom of innings 1");
    assert_eq!(full.half_words(), "BOT 1ST");
    let report = full.report();
    assert_eq!(report.heading, "TOP OF THE 1ST");
    // How many hits the two runs came on is as the book has it.
    let hits = hits_words(full.book.theirs.hits_in(1));
    assert_eq!(
        report.lines,
        [
            format!("THE VISITORS MADE 2 RUNS ON {hits}"),
            "YOU TRAIL 0 - 2".to_owned(),
            "YOU BAT IN THE BOTTOM OF THE 1ST".to_owned()
        ]
    );
    let [visitors, home] = full.lines();
    assert_eq!((visitors.name, home.name), ("THEM", "YOU"));
    assert_eq!(visitors.cells[0], Cell::Runs(2));
    assert!(home.cells.iter().all(|cell| *cell == Cell::Blank));
}

#[test]
fn at_home_the_last_half_is_not_needed_when_ahead() {
    let mut full = against(true, 1, 2);
    // Three to one up after the first, and three to two after the top
    // of the second: there is nothing left to bat for.
    assert_eq!(full.side_out(3), Next::Won);
    assert_eq!(full.describe(), "at home, visitors 1 1, home side 3 x");
}

#[test]
fn at_home_getting_ahead_in_the_last_innings_wins_there_and_then() {
    let mut full = against(true, 1, 2);
    assert!(!full.sudden());
    assert_eq!(full.side_out(1), Next::Bat);
    // One behind after the top of the last.
    assert!(full.sudden());
    let report = full.report();
    assert_eq!(report.heading, "TOP OF THE 2ND");
    assert_eq!(report.lines[1], "YOU TRAIL 1 - 2");
    assert_eq!(report.lines[3], "2 RUNS WILL WIN THE MATCH");
    full.walked_off(2);
    assert!(!full.sudden());
    assert_eq!(full.describe(), "at home, visitors 1 1, home side 1 2");
}

#[test]
fn at_home_a_level_match_goes_on_and_one_behind_is_lost() {
    let mut full = against(true, 1, 1);
    assert!(full.sudden());
    assert_eq!(full.side_out(1), Next::Bat);
    assert_eq!(full.batting_in(), "bottom of innings 2");
    assert_eq!(full.side_out(0), Next::Lost);
    assert_eq!(full.describe(), "at home, visitors 1 1, home side 1 0");
}

#[test]
fn a_long_match_shows_its_last_nine_innings() {
    let mut full = against(false, 0, 9);
    assert_eq!(full.shown(), (1, 9));
    for _ in 0..11 {
        assert_eq!(full.side_out(0), Next::Bat);
    }
    assert_eq!(full.shown(), (3, 9));
    let [visitors, _] = full.lines();
    assert_eq!(visitors.cells.len(), 9);
    // A short one has no more columns than innings.
    assert_eq!(against(true, 0, 3).shown(), (1, 3));
}

#[test]
fn with_every_hit_a_home_run_the_other_side_makes_more() {
    let rules = Rules::default().full_match;
    let ground = Ground::default();
    let total = |zinger: bool| {
        (0..40)
            .map(|seed| {
                FullMatch::new(true, &rules, Difficulty::Hard, zinger, None, ground, seed).theirs()
            })
            .sum::<u32>()
    };
    assert!(total(true) > total(false) * 3 / 2);
}

/// Checks that the book of the other side says what the board says.
pub(super) fn book_agrees(full: &FullMatch) {
    let theirs = &full.book.theirs;
    for (index, &made) in full.theirs.iter().enumerate() {
        let innings = index as u32 + 1;
        let in_it: u32 = theirs.innings(innings).map(|turn| turn.runs_in).sum();
        assert_eq!(in_it, made, "the runs of innings {innings}");
    }
    assert_eq!(theirs.runs.iter().sum::<u32>(), full.theirs());
    assert_eq!(theirs.left.len(), full.theirs.len());
    // They bat in order from one innings to the next.
    for (index, turn) in theirs.turns.iter().enumerate() {
        assert_eq!(turn.order, index % ORDER);
    }
    // Nobody batted in a half that was not played.
    let played = full.theirs.len() as u32;
    assert!(theirs.turns.iter().all(|turn| turn.innings <= played));
    let figures = theirs.figures();
    assert_eq!(figures.runs, full.theirs());
    assert_eq!(figures.runs_in, full.theirs());
    let line = full.lines().into_iter().find(|line| !line.ours).unwrap();
    assert_eq!((line.runs, line.hits), (figures.runs, figures.hits));
    // Everyone who came up was put out, at the plate or on the bases,
    // came home or was left on.
    assert_eq!(figures.turns, theirs.outs() + figures.runs + figures.left);
    // A steal is told among the turns of the innings it was in.
    for steal in &theirs.steals {
        assert!(steal.at <= theirs.turns.len());
        let before = theirs.turns[..steal.at].last().map(|turn| turn.innings);
        let after = theirs.turns.get(steal.at).map(|turn| turn.innings);
        assert!(
            before == Some(steal.innings) || after == Some(steal.innings),
            "{steal:?}"
        );
    }
}

#[test]
fn the_other_sides_book_says_what_the_board_says() {
    let rules = Rules::default().full_match;
    let ground = Ground::default();
    for seed in 0..60 {
        let home = seed % 2 == 0;
        let difficulty = [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard][seed % 3];
        let zinger = seed % 5 == 0;
        // In some of them their runners steal, and in some of those
        // every chance they get.
        let steals = match seed % 4 {
            0 => Some(Rules::default().steal),
            1 => Some(StealRules {
                their_chance: 1.0,
                their_safe: 0.5,
                ..Rules::default().steal
            }),
            _ => None,
        };
        let stealing = steals.is_some();
        let seed_of = seed as u64;
        let mut full = FullMatch::new(home, &rules, difficulty, zinger, steals, ground, seed_of);
        book_agrees(&full);
        for innings in 0..30 {
            // Sometimes ahead and sometimes behind, so that matches
            // end every way they can.
            let next = full.side_out((seed as u32 + innings) % 4);
            book_agrees(&full);
            if next != Next::Bat {
                break;
            }
        }
        // Three were out in every half of theirs that ran its course.
        let theirs = &full.book.theirs;
        let won_at_bat = !home && full.theirs() > full.ours() && !full.unneeded;
        assert!(stealing || theirs.steals.is_empty());
        for innings in 1..=full.theirs.len() as u32 {
            let caught = |steal: &&Steal| steal.innings == innings && !steal.safe;
            let caught = theirs.steals.iter().filter(caught).count() as u32;
            let outs = theirs
                .innings(innings)
                .map(|turn| turn.outs_made)
                .sum::<u32>()
                + caught;
            let last = innings as usize == full.theirs.len();
            if last && won_at_bat && full.over {
                assert!(outs < 3, "a winning half that went on");
            } else {
                assert_eq!(outs, 3, "innings {innings} of seed {seed}");
            }
        }
    }
}

#[test]
fn the_same_seed_gives_the_same_match() {
    let rules = Rules::default().full_match;
    let ground = Ground::default();
    let play = |seed| {
        let mut full = FullMatch::new(false, &rules, Difficulty::Medium, false, None, ground, seed);
        for _ in 0..8 {
            full.side_out(1);
        }
        full.describe()
    };
    assert_eq!(play(3), play(3));
    assert_ne!(play(3), play(4));
}
