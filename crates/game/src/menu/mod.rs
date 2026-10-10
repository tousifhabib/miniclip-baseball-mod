//! The menu: what the buttons on each of its pages do. The pages
//! themselves, and the ways out of the menu, are named in `page`.

mod full_match;
mod page;
mod summary;
mod tournament;

use bb_engine::display::{ClipState, Path};
use bb_engine::stage::Stage;

use crate::art;
use full_match::Grounds;
pub use page::{Leave, MenuPage};
// The game was this file's once, and is still found here.
pub use crate::game::Game;
use crate::look::{self, Rgb};
use crate::rng::{Rng, mixed_with};
use crate::settings::{Difficulty, Settings};
use crate::tournament::Brief;
pub use tournament::innings_chosen;

/// The lettering the pages a full match and a tournament add are written
/// in is drawn 18 high. These are the sizes of its lines, that being 1.
const HEADING_SIZE: f32 = 17.0 / 18.0;
const WORDS_SIZE: f32 = 14.0 / 18.0;
const CHOICE_SIZE: f32 = 12.5 / 18.0;
const WHITE: Rgb = look::WHITE;

pub struct Menu {
    page: MenuPage,
    /// A page is on its way in, and has things to be set once it is there.
    arriving: bool,
    /// What the coin for a full match is tossed with, if not the clock.
    coin: Option<Rng>,
    /// Whether the side is at home in the full match about to begin, once
    /// that has been settled.
    home: Option<bool>,
    /// What a full match has put on its setup page.
    grounds: Option<Grounds>,
    /// What a full match or a tournament has written on its summary page.
    summary: Option<Path>,
    /// The tournament in hand as the game has told the menu of it, if
    /// there is one, and what it has put on its setup page and its summary.
    told: Option<Brief>,
    choices: Option<tournament::Choices>,
    boxes: Option<tournament::Boxes>,
    /// Something a box on one of those pages has asked for.
    asked: Option<Leave>,
}

impl Default for Menu {
    fn default() -> Menu {
        Menu {
            page: MenuPage::Opening,
            arriving: false,
            coin: None,
            home: None,
            grounds: None,
            summary: None,
            told: None,
            choices: None,
            boxes: None,
            asked: None,
        }
    }
}

impl Menu {
    pub fn page(&self) -> MenuPage {
        self.page
    }

    /// Makes the toss for a full match come out the same way every time.
    pub fn seed(&mut self, seed: u64) {
        self.coin = Some(Rng::new(seed ^ mixed_with::THE_COIN));
    }

    fn clip(stage: &Stage) -> Option<&ClipState> {
        stage.clip(&art::in_shell(stage, art::MENU)?)
    }

    /// The menu has just been put on screen. Coming from the intro it plays
    /// its whole arrival. Coming back from anywhere else it goes straight to
    /// fading in.
    pub fn shown(&mut self, from_intro: bool, game: &Game, stage: &mut Stage) {
        self.page = MenuPage::Opening;
        if !from_intro {
            self.open(MenuPage::Main, game, stage);
        }
    }

    /// Plays in another page.
    pub fn open(&mut self, page: MenuPage, game: &Game, stage: &mut Stage) {
        let Some(menu) = art::in_shell(stage, art::MENU) else {
            return;
        };
        if stage.goto_label(&menu, page.label(), true) {
            self.page = page;
            // What the summary pages say about the game to come.
            let rules = &game.rules.game;
            let behind = *rules.runs_down.at(game.settings.difficulty);
            stage.set_text("oppositionScore", behind.to_string());
            // Drawing level is not enough: the target is one run more.
            stage.set_text("scoreTarget", (behind + 1).to_string());
            stage.set_text("maximumOuts", rules.outs.to_string());
            // The page's own clips only exist once it has played in.
            self.arriving = true;
            match page {
                // What the summary says fades away with it.
                MenuPage::ToFull | MenuPage::ToTournament => {}
                // The summary says where the side is to play.
                MenuPage::FullSummary => {
                    self.clear_full(stage);
                    self.at_home(&game.settings);
                }
                // What a full match put on the page that was up goes with
                // it, and where the side was to play is forgotten.
                _ => {
                    self.clear_full(stage);
                    self.home = None;
                }
            }
        }
    }

    /// Moves the marker on the setup pages to the chosen difficulty.
    fn show_difficulty(settings: &Settings, stage: &mut Stage) {
        let marker =
            art::in_shell(stage, art::MENU).and_then(|menu| stage.find_named(&menu, "skillSelect"));
        if let Some(marker) = marker {
            stage.goto_label(&marker, settings.difficulty.label(), false);
        }
    }

    /// Acts on a button the player has clicked, known by the words on it.
    pub fn clicked(&mut self, label: &str, game: &mut Game, stage: &mut Stage) -> Option<Leave> {
        use MenuPage::{
            ArcadeSetup, ArcadeSummary, FullSetup, FullSummary, HighScores, Main, MatchSetup,
            MatchSummary, Mods, ToArcade, ToFull, ToMatch, TournamentSetup, TournamentSummary,
        };
        // A page that is still arriving cannot be used yet.
        if Menu::clip(stage).is_none_or(|menu| menu.playing) {
            return None;
        }
        let page = match (self.page, label) {
            (Main, "BOTTOM OF THE NINTH") => MatchSetup,
            (Main, "FULL MATCH") => FullSetup,
            // A tournament that has a result in it is gone on with, and
            // one that has none can still be set up.
            (Main, "TOURNAMENT") if self.underway() => TournamentSummary,
            (Main, "TOURNAMENT") => TournamentSetup,
            (Main, "ARCADE") => ArcadeSetup,
            (Main, "INSTRUCTIONS") => return Some(Leave::Instructions),
            (Main, label) if label.starts_with("HIGH SCORES") => HighScores,
            (Main, "MODS") => Mods,
            (MatchSetup | ArcadeSetup | FullSetup | TournamentSetup, "BACK") => Main,
            (HighScores | Mods, "BACK") => Main,
            (TournamentSummary, "BACK") if self.underway() => Main,
            (TournamentSummary, "BACK") => TournamentSetup,
            (MatchSummary, "BACK") => MatchSetup,
            (ArcadeSummary, "BACK") => ArcadeSetup,
            (FullSummary, "BACK") => FullSetup,
            (MatchSetup, "NEXT") => MatchSummary,
            (ArcadeSetup, "NEXT") => ArcadeSummary,
            (FullSetup, "NEXT") => FullSummary,
            (TournamentSetup, "NEXT") => return Some(Leave::Draw),
            (MatchSummary, "PLAY BALL") => ToMatch,
            (ArcadeSummary, "PLAY BALL") => ToArcade,
            (FullSummary, "PLAY BALL") => ToFull,
            // When it is over, there is another to be set up.
            (TournamentSummary, "PLAY BALL") if self.over() => TournamentSetup,
            (
                MatchSetup | ArcadeSetup | FullSetup | TournamentSetup,
                "EASY" | "MEDIUM" | "HARD",
            ) => {
                game.settings.difficulty = match label {
                    "EASY" => Difficulty::Easy,
                    "MEDIUM" => Difficulty::Medium,
                    _ => Difficulty::Hard,
                };
                Menu::show_difficulty(&game.settings, stage);
                return None;
            }
            _ => return None,
        };
        self.open(page, game, stage);
        None
    }

    /// Called once a frame while the menu is showing.
    pub fn tick(&mut self, game: &Game, stage: &mut Stage) -> Option<Leave> {
        let menu = Menu::clip(stage)?;
        let (frame, playing, last) = (menu.frame, menu.playing, menu.frame_count(stage.library()));
        // The frame the fade-out into a match begins on, which is where the
        // arcade game's own ends.
        let to_match = stage
            .library()
            .timeline(menu.symbol)
            .and_then(|timeline| timeline.labels.get(MenuPage::ToMatch.label()))
            .copied();
        if self.arriving && !playing {
            self.arriving = false;
            Menu::show_difficulty(&game.settings, stage);
        }
        self.show_grounds(&game.settings, stage);
        self.show_summary(game, stage);
        self.show_choices(game, stage);
        self.show_told(stage);
        if let Some(asked) = self.asked.take() {
            return Some(asked);
        }
        match self.page {
            MenuPage::Opening if !playing => self.page = MenuPage::Main,
            MenuPage::ToMatch if frame >= last => return Some(Leave::Match),
            MenuPage::ToFull if frame >= last => return Some(Leave::FullMatch),
            // The arcade's fade-out is followed directly by the match's, so
            // it has ended when that is about to begin.
            MenuPage::ToArcade if to_match.is_some_and(|next| frame + 1 >= next) => {
                return Some(Leave::Arcade);
            }
            _ => {}
        }
        None
    }
}
