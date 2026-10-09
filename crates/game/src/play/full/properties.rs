use proptest::prelude::*;

use super::tests::book_agrees;
use super::*;
use crate::rules::Rules;

/// The player's side bats and makes these runs. At home with the innings
/// all but played, runs that put it ahead are the match there and then.
/// Otherwise it is out, and this says how things stand once the other
/// side has had its turn.
fn bat(full: &mut FullMatch, runs: u32) -> Next {
    if full.sudden() && full.ours() + runs > full.theirs() {
        full.walked_off(runs);
        return Next::Won;
    }
    full.side_out(runs)
}

proptest! {
    // Each case plays a whole match, the other side's part of it on
    // paper.
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn a_full_match_ends_only_with_one_side_ahead_and_no_sooner_than_its_innings(
        (home, zinger, stealing) in any::<(bool, bool, bool)>(),
        innings in 1u32..=9,
        level in prop::sample::select(
            &[Difficulty::Easy, Difficulty::Medium, Difficulty::Hard][..],
        ),
        seed: u64,
        // The runs the player's side makes in each of its innings, for
        // more innings than a match should ever need.
        ours in prop::collection::vec(0u32..4, 40),
    ) {
        let mut rules = Rules::default().full_match;
        rules.innings = innings;
        let steals = stealing.then(|| Rules::default().steal);
        let begin = || {
            let steals = steals.clone();
            FullMatch::new(home, &rules, level, zinger, steals, Ground::default(), seed)
        };
        let mut full = begin();
        let mut ended = None;
        for &runs in &ours {
            // Between innings the book says what the board says, and
            // the board has a line for each side.
            book_agrees(&full);
            let (_, columns) = full.shown();
            for line in full.lines() {
                let made = if line.ours { full.ours() } else { full.theirs() };
                prop_assert_eq!((line.runs, line.cells.len() as u32), (made, columns));
            }
            prop_assert!(!full.report().lines.is_empty());
            let next = bat(&mut full, runs);
            if next != Next::Bat {
                ended = Some(next);
                break;
            }
            // It goes on past its innings only while it is level, as of
            // the last innings both sides have had.
            let both = full.ours.len().min(full.theirs.len());
            if both as u32 >= innings {
                let by_then = |made: &[u32]| made[..both].iter().sum::<u32>();
                let (us, them) = (by_then(&full.ours), by_then(&full.theirs));
                prop_assert_eq!(us, them, "{}", full.describe());
            }
        }
        book_agrees(&full);
        if let Some(ended) = ended {
            let (us, them) = (full.ours(), full.theirs());
            prop_assert!(full.over && !full.sudden());
            prop_assert_ne!(us, them, "{}", full.describe());
            prop_assert_eq!(ended == Next::Won, us > them, "{}", full.describe());
            let said = if us > them { "YOU WON" } else { "YOU LOST" };
            prop_assert!(full.verdict().starts_with(said), "{}", full.verdict());
            // Each side has batted in every innings there had to be,
            // but that the side batting last had no need of a last
            // half it was ahead without.
            let (batted, fielded) = (full.ours.len() as u32, full.theirs.len() as u32);
            let (first, last) = if home { (fielded, batted) } else { (batted, fielded) };
            prop_assert!(first >= innings, "{}", full.describe());
            prop_assert_eq!(first, last + u32::from(full.unneeded), "{}", full.describe());
            if full.unneeded {
                prop_assert_eq!(home, us > them, "{}", full.describe());
            }
        }
        // The same seed, and the same runs from the player's side, are
        // the same match over again.
        let mut again = begin();
        for &runs in &ours {
            if bat(&mut again, runs) != Next::Bat {
                break;
            }
        }
        prop_assert_eq!(again.describe(), full.describe());
        prop_assert_eq!(&again.book, &full.book);
    }
}
