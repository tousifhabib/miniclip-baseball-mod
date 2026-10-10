//! A tournament in a line: what a script or a test reads to know where it
//! has got to.

use super::progress::End;
use super::{Tournament, schedule};

impl Tournament {
    /// The tournament in a line: its shape and its length of match, how
    /// much of it has been played, and what the player has next or how it
    /// all ended. The tests read it, so what it says is not to change.
    pub fn describe(&self) -> String {
        let setup = self.setup;
        let all = schedule::ties(setup.format).len();
        let so_far = format!(
            "{} of {}, {} innings, played {} of {all}",
            setup.format.key(),
            setup.format.sides(),
            setup.innings,
            self.played.len()
        );
        let Some(next) = self.next() else {
            let champion = self.champion().map_or("", |side| self.name_of(side));
            let end = match self.players_end() {
                Some(End::Champion) => "you won it".to_owned(),
                Some(End::LostIn(round)) => format!("you went out in {}", round.words()),
                Some(End::Placed(place)) => format!("you were placed {place}"),
                None => String::new(),
            };
            return format!("{so_far}, over, won by {champion}, {end}");
        };
        let round = next.kind.words();
        match next.against(self.player) {
            Some(against) => {
                let ground = if next.home == Some(self.player) {
                    "at home"
                } else {
                    "away"
                };
                let against = self.name_of(against);
                format!("{so_far}, {round}, next {ground} against {against}")
            }
            // A fixture of other sides, waiting to be played on paper.
            None => format!("{so_far}, {round}, others to play"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::Rules;
    use crate::tournament::Format;
    use crate::tournament::testing::{drawn, played_out};

    #[test]
    fn a_tournament_says_how_far_it_has_got_and_who_is_next() {
        let fresh = drawn(Format::League, 3, 2);
        let next = fresh.next().expect("a fixture");
        let player = fresh.player();
        let ground = if next.home == Some(player) {
            "at home"
        } else {
            "away"
        };
        let against = fresh.name_of(next.against(player).expect("the other side"));
        assert_eq!(
            fresh.describe(),
            format!(
                "league of 6, 3 innings, played 0 of 15, ROUND 1, next {ground} against {against}"
            )
        );
        let mut said = Vec::new();
        let done = played_out(fresh, &Rules::default(), |tournament| {
            said.push(tournament.describe());
        });
        assert!(
            said[1].contains("played 3 of 15, ROUND 2, next "),
            "{}",
            said[1]
        );
        assert!(
            said[4].contains("played 12 of 15, ROUND 5, next "),
            "{}",
            said[4]
        );
        let champion = done.name_of(done.champion().expect("a champion"));
        let over =
            format!("league of 6, 3 innings, played 15 of 15, over, won by {champion}, you ");
        assert!(done.describe().starts_with(&over), "{}", done.describe());
    }

    #[test]
    fn a_cup_says_which_round_and_how_the_player_went_out_of_it() {
        let rules = Rules::default();
        let mut ends = std::collections::BTreeSet::new();
        for seed in 0..40 {
            let done = played_out(drawn(Format::Cup, 5, seed), &rules, |tournament| {
                let said = tournament.describe();
                assert!(said.starts_with("cup of 8, 5 innings, played "), "{said}");
                let round = ["THE QUARTER-FINALS", "THE SEMI-FINALS", "THE FINAL"]
                    .into_iter()
                    .any(|round| said.contains(&format!(", {round}, next ")));
                assert!(round, "{said}");
            });
            let said = done.describe();
            let end = said.rsplit(", ").next().expect("how it ended").to_owned();
            ends.insert(end);
        }
        let all = [
            "you went out in THE FINAL",
            "you went out in THE QUARTER-FINALS",
            "you went out in THE SEMI-FINALS",
            "you won it",
        ];
        assert_eq!(ends.into_iter().collect::<Vec<_>>(), all);
    }
}
