//! The shapes a tournament comes in: how many sides, in how many groups,
//! over how many rounds, and what each round is called.

/// The shape of a tournament.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Two groups of four, each side meeting the other three of its
    /// group. The first two of each go on to semi-finals and a final.
    Groups,
    /// Six sides, each meeting the other five. The table decides it.
    League,
    /// Eight sides, and the loser of every tie goes out.
    Cup,
}

impl Format {
    pub const ALL: [Format; 3] = [Format::Groups, Format::League, Format::Cup];

    /// What it is kept as, and called on the command line.
    pub fn key(self) -> &'static str {
        match self {
            Format::Groups => "groups",
            Format::League => "league",
            Format::Cup => "cup",
        }
    }

    /// The shape with this key, however it is written.
    pub fn from_key(key: &str) -> Option<Format> {
        Format::ALL
            .into_iter()
            .find(|format| format.key().eq_ignore_ascii_case(key))
    }

    /// The word for it on the setup page.
    pub fn word(self) -> &'static str {
        match self {
            Format::Groups => "Groups",
            Format::League => "League",
            Format::Cup => "Cup",
        }
    }

    /// How many sides are in it, the player's among them.
    pub fn sides(self) -> usize {
        match self {
            Format::Groups | Format::Cup => 8,
            Format::League => 6,
        }
    }

    /// How many groups it has in which everyone meets everyone. A league
    /// is one such group, and a cup has none.
    pub fn groups(self) -> usize {
        match self {
            Format::Groups => 2,
            Format::League => 1,
            Format::Cup => 0,
        }
    }

    /// How many sides are in each of those groups.
    pub fn in_a_group(self) -> usize {
        match self {
            Format::Groups => 4,
            Format::League => 6,
            Format::Cup => 0,
        }
    }

    /// How many of each group go on to the knockout rounds.
    pub fn go_through(self) -> usize {
        match self {
            Format::Groups => 2,
            Format::League | Format::Cup => 0,
        }
    }

    /// How many rounds it has.
    pub fn rounds(self) -> usize {
        match self {
            Format::Groups | Format::League => 5,
            Format::Cup => 3,
        }
    }

    /// What the round with this number is, the first being nought. A
    /// number past the last is the last.
    pub fn round(self, number: usize) -> Round {
        let from_the_last = self.rounds().saturating_sub(number + 1);
        match (self, from_the_last) {
            (Format::League, _) => Round::Of(number.min(self.rounds() - 1) as u32 + 1),
            (_, 0) => Round::Final,
            (_, 1) => Round::SemiFinals,
            (Format::Cup, _) => Round::QuarterFinals,
            (Format::Groups, _) => Round::Of(number as u32 + 1),
        }
    }
}

/// What a round of a tournament is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Round {
    /// One of the rounds in which everyone meets everyone: which of them,
    /// the first being 1.
    Of(u32),
    QuarterFinals,
    SemiFinals,
    Final,
}

impl Round {
    /// What the round is called, in capitals.
    pub fn words(self) -> String {
        match self {
            Round::Of(number) => format!("ROUND {number}"),
            Round::QuarterFinals => "THE QUARTER-FINALS".to_owned(),
            Round::SemiFinals => "THE SEMI-FINALS".to_owned(),
            Round::Final => "THE FINAL".to_owned(),
        }
    }

    /// Whether the loser of a tie in this round goes out.
    pub fn is_knockout(self) -> bool {
        !matches!(self, Round::Of(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shape_is_known_by_its_key_however_it_is_written() {
        for format in Format::ALL {
            assert_eq!(Format::from_key(format.key()), Some(format));
            assert_eq!(Format::from_key(&format.key().to_uppercase()), Some(format));
        }
        assert_eq!(Format::from_key("ladder"), None);
    }

    #[test]
    fn the_groups_of_a_shape_hold_all_its_sides_or_none_of_them() {
        assert_eq!(Format::Groups.groups() * Format::Groups.in_a_group(), 8);
        assert_eq!(Format::League.groups() * Format::League.in_a_group(), 6);
        assert_eq!(Format::Cup.groups() * Format::Cup.in_a_group(), 0);
        assert_eq!(Format::Cup.sides(), 8);
    }

    #[test]
    fn each_round_has_its_name() {
        let names = |format: Format| -> Vec<String> {
            (0..format.rounds())
                .map(|round| format.round(round).words())
                .collect()
        };
        assert_eq!(
            names(Format::Groups),
            [
                "ROUND 1",
                "ROUND 2",
                "ROUND 3",
                "THE SEMI-FINALS",
                "THE FINAL"
            ]
        );
        assert_eq!(
            names(Format::League),
            ["ROUND 1", "ROUND 2", "ROUND 3", "ROUND 4", "ROUND 5"]
        );
        assert_eq!(
            names(Format::Cup),
            ["THE QUARTER-FINALS", "THE SEMI-FINALS", "THE FINAL"]
        );
        // A round past the last is the last.
        assert_eq!(Format::Cup.round(9), Round::Final);
        assert_eq!(Format::League.round(9), Round::Of(5));
        assert!(Round::SemiFinals.is_knockout() && !Round::Of(2).is_knockout());
    }
}
