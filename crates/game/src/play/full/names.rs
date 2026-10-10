//! What a full match calls the other side: "them", unless it has been
//! told who they are.
//!
//! The player's side is "you" whatever it is called anywhere else. That is
//! how the boards speak to whoever is reading them.

use super::FullMatch;

/// What the other side is called, when it has a name: in full, and in the
/// few letters there is room for on a scoreboard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Named {
    name: String,
    short: String,
}

impl FullMatch {
    /// Says who the other side is: its name in full, and in a few letters.
    /// Left unsaid, it is only "them".
    pub fn call_them(&mut self, name: &str, short: &str) {
        self.them = Some(Named {
            name: name.to_owned(),
            short: short.to_owned(),
        });
    }

    /// What the other side is called where there is room for a few
    /// letters.
    pub fn them(&self) -> &str {
        self.them.as_ref().map_or("THEM", |them| &them.short)
    }

    /// The other side's name in full, if it has one.
    pub fn their_name(&self) -> Option<&str> {
        self.them.as_ref().map(|them| them.name.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::field::Ground;
    use crate::rules::Rules;
    use crate::settings::Difficulty;

    /// A match at home, in which the visitors have batted already.
    fn at_home() -> FullMatch {
        let rules = Rules::default().full_match;
        let ground = Ground::default();
        FullMatch::new(true, &rules, Difficulty::Medium, false, None, ground, 7)
    }

    #[test]
    fn a_side_nobody_has_named_is_them() {
        let full = at_home();
        assert_eq!((full.them(), full.their_name()), ("THEM", None));
        let [visitors, home] = full.lines();
        assert_eq!(
            (visitors.name.as_str(), home.name.as_str()),
            ("THEM", "YOU")
        );
        assert!(full.report().lines[0].starts_with("THE VISITORS MADE "));
    }

    #[test]
    fn a_side_that_has_a_name_goes_by_it_and_the_players_is_still_you() {
        let mut full = at_home();
        full.call_them("ASHCOMBE ROOKS", "ROOK");
        assert_eq!(full.them(), "ROOK");
        assert_eq!(full.their_name(), Some("ASHCOMBE ROOKS"));
        let [visitors, home] = full.lines();
        assert_eq!(
            (visitors.name.as_str(), home.name.as_str()),
            ("ROOK", "YOU")
        );
        let report = full.report();
        assert!(report.lines[0].starts_with("ASHCOMBE ROOKS MADE "));
        assert!(report.lines[1].starts_with("YOU ") || report.lines[1].starts_with("IT IS "));
    }
}
