//! What the menu says of a tournament before a fixture is played: where
//! it has got to, who the player meets next and where, and how the
//! player's side stands, or how it all ended.

use super::progress::End;
use super::{Format, Round, Tournament};
use crate::play::full::ordinal;

/// A tournament as the menu tells it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Brief {
    /// Whether a result is in, after which the tournament is not to be
    /// drawn afresh by going back.
    pub begun: bool,
    /// Whether every fixture has been played.
    pub over: bool,
    /// What there is to say: a line in capitals, and up to four under it.
    pub lines: Vec<String>,
}

/// What a tournament of this shape is called in the middle of a line.
fn called(format: Format) -> &'static str {
    match format {
        Format::Groups => "the groups",
        Format::League => "the league",
        Format::Cup => "the cup",
    }
}

/// A place in an order as it is written in the middle of a line: 3rd.
fn place(number: usize) -> String {
    ordinal(number as u32).to_lowercase()
}

impl Tournament {
    /// How the player's side stands in the tournament so far: what it has
    /// won and lost, and where it is in its table if it has one.
    fn standing(&self) -> [String; 2] {
        let summed = self.summed(self.player);
        if summed.matches == 0 {
            return ["It is your first match.".to_owned(), String::new()];
        }
        let lost = summed.matches - summed.won;
        let so_far = format!("You have won {} and lost {lost},", summed.won);
        let table = self.group_of(self.player).map(|group| self.table(group));
        let stands = table.and_then(|table| {
            let at = table.iter().position(|row| row.side == self.player)?;
            Some(format!("and are {} of {}.", place(at + 1), table.len()))
        });
        // In a cup there is no table to be anywhere in.
        [so_far, stands.unwrap_or("and are still in it.".to_owned())]
    }

    /// How the tournament ended, for the player's side and for whoever won
    /// it.
    fn ending(&self) -> Vec<String> {
        let format = self.setup.format;
        let champion = self.champion().map_or("", |side| self.name_of(side));
        let won = match self.players_end() {
            Some(End::Champion) => "You have won it!".to_owned(),
            _ => format!("{champion} won it."),
        };
        let came = match self.players_end() {
            Some(End::LostIn(Round::Final)) => "You lost the final.".to_owned(),
            Some(End::LostIn(round)) => {
                format!("You went out in {}.", round.words().to_lowercase())
            }
            Some(End::Placed(at)) => format!("You were placed {}.", place(at)),
            Some(End::Champion) | None => String::new(),
        };
        vec![
            format!("{} IS OVER!", called(format).to_uppercase()),
            won,
            came,
            "Play ball, and there is".to_owned(),
            "another to be drawn.".to_owned(),
        ]
    }

    /// The tournament as the menu tells it.
    pub fn brief(&self) -> Brief {
        let begun = !self.played.is_empty();
        let next = self.next().and_then(|next| {
            let against = next.against(self.player)?;
            Some((next, against))
        });
        let Some((next, against)) = next else {
            return Brief {
                begun,
                over: self.is_over(),
                lines: self.ending(),
            };
        };
        let format = self.setup.format;
        let ground = if next.home == Some(self.player) {
            "at home"
        } else {
            "away"
        };
        let [so_far, stands] = self.standing();
        let lines = vec![
            format!(
                "{} OF {}!",
                next.kind.words(),
                called(format).to_uppercase()
            ),
            format!("You play {},", self.name_of(against)),
            format!("{ground}, over {} innings.", self.setup.innings),
            so_far,
            stands,
        ];
        Brief {
            begun,
            over: false,
            lines,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rules;
    use crate::tournament::testing::{drawn, played_out};

    #[test]
    fn before_the_first_match_the_menu_says_who_it_is_against_and_where() {
        let fresh = drawn(Format::League, 5, 3);
        let brief = fresh.brief();
        assert!(!brief.begun && !brief.over);
        let next = fresh.next().expect("a fixture");
        let against = fresh.name_of(next.against(fresh.player()).expect("the other side"));
        let ground = if next.home == Some(fresh.player()) {
            "at home"
        } else {
            "away"
        };
        assert_eq!(
            brief.lines,
            [
                "ROUND 1 OF THE LEAGUE!".to_owned(),
                format!("You play {against},"),
                format!("{ground}, over 5 innings."),
                "It is your first match.".to_owned(),
                String::new(),
            ]
        );
    }

    #[test]
    fn between_matches_it_says_how_the_players_side_stands() {
        let rules = Rules::default();
        let mut said = Vec::new();
        played_out(drawn(Format::League, 3, 3), &rules, |tournament| {
            said.push(tournament.brief());
        });
        assert!(!said[0].begun && said[1..].iter().all(|brief| brief.begun));
        for (round, brief) in said.iter().enumerate().skip(1) {
            assert_eq!(
                brief.lines[0],
                format!("ROUND {} OF THE LEAGUE!", round + 1)
            );
            let so_far = &brief.lines[3];
            assert!(so_far.starts_with("You have won "), "{so_far}");
            let stands = &brief.lines[4];
            assert!(
                stands.starts_with("and are ") && stands.ends_with(" of 6."),
                "{stands}"
            );
        }
        // In a cup there is no table, and being there at all is the
        // standing.
        let mut cup = Vec::new();
        played_out(drawn(Format::Cup, 3, 1), &rules, |tournament| {
            cup.push(tournament.brief().lines);
        });
        assert_eq!(cup[0][0], "THE QUARTER-FINALS OF THE CUP!");
        for lines in cup.iter().skip(1) {
            assert_eq!(lines[4], "and are still in it.");
        }
    }

    #[test]
    fn when_it_is_over_it_says_who_won_it_and_how_the_player_did() {
        let rules = Rules::default();
        let mut ends = std::collections::BTreeSet::new();
        for seed in 0..40 {
            let done = played_out(drawn(Format::Cup, 3, seed), &rules, |_| {});
            let brief = done.brief();
            assert!(brief.begun && brief.over);
            assert_eq!(brief.lines[0], "THE CUP IS OVER!");
            assert_eq!(brief.lines.len(), 5);
            let champion = done.name_of(done.champion().expect("a champion"));
            let won = [format!("{champion} won it."), "You have won it!".to_owned()];
            assert!(won.contains(&brief.lines[1]), "{}", brief.lines[1]);
            ends.insert(brief.lines[2].clone());
        }
        let all = [
            "",
            "You lost the final.",
            "You went out in the quarter-finals.",
            "You went out in the semi-finals.",
        ];
        assert_eq!(ends.into_iter().collect::<Vec<_>>(), all);
        let league = played_out(drawn(Format::League, 3, 3), &rules, |_| {}).brief();
        let placed = &league.lines[2];
        assert!(
            placed.is_empty() || placed.starts_with("You were placed "),
            "{placed}"
        );
    }
}
