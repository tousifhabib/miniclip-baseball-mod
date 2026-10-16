//! The menu's pages, and the ways out of it.

/// Where the player is in the menu. Each is a section of the menu clip that
/// plays in and then waits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuPage {
    /// The menu arriving for the first time, before it can be used.
    Opening,
    Main,
    MatchSetup,
    ArcadeSetup,
    HighScores,
    /// The mods, each with a box to tick. It is shown on the high-score
    /// page's section of the menu.
    Mods,
    MatchSummary,
    ArcadeSummary,
    /// The full match's setup and summary. They are shown on the match's
    /// sections of the menu, with what a full match needs put there.
    FullSetup,
    FullSummary,
    /// A tournament's setup and its summary, which are shown on the
    /// match's sections of the menu as well.
    TournamentSetup,
    TournamentSummary,
    /// Fading out on the way to a match.
    ToMatch,
    /// Fading out on the way to the arcade game.
    ToArcade,
    /// Fading out on the way to a full match.
    ToFull,
    /// Fading out on the way to a fixture of a tournament.
    ToTournament,
}

impl MenuPage {
    /// The label of the section that shows this page.
    pub(super) fn label(self) -> &'static str {
        match self {
            MenuPage::Opening | MenuPage::Main => "menuFadeIn",
            MenuPage::MatchSetup | MenuPage::FullSetup | MenuPage::TournamentSetup => "matchIn",
            MenuPage::ArcadeSetup => "arcadeIn",
            MenuPage::HighScores | MenuPage::Mods => "highScoresIn",
            MenuPage::MatchSummary | MenuPage::FullSummary | MenuPage::TournamentSummary => {
                "matchSummary"
            }
            MenuPage::ArcadeSummary => "arcadeSummary",
            MenuPage::ToMatch | MenuPage::ToFull | MenuPage::ToTournament => "fadeOutMatch",
            MenuPage::ToArcade => "fadeOutArcade",
        }
    }
}

/// Something the menu wants that is not its own to do: another screen, or
/// something done about the tournament in hand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leave {
    Instructions,
    Match,
    Arcade,
    FullMatch,
    /// The next fixture of the tournament in hand.
    Fixture,
    /// A tournament is to be drawn as the setup page has it.
    Draw,
    /// The tables of the tournament in hand are to be shown.
    Tables,
    /// The tournament in hand is to be given up.
    GiveUp,
}
