//! The menu: its pages, and what the buttons on each one do.

use bb_engine::display::{ClipState, Path};
use bb_engine::library::Library;
use bb_engine::stage::Stage;

use crate::art;
use crate::look::{self, Rgb};
// The game was this file's once, and is still found here.
pub use crate::game::Game;
use crate::rng::Rng;
use crate::settings::{Difficulty, Ground, Settings};
use crate::sheet::Sheet;

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
    /// Fading out on the way to a match.
    ToMatch,
    /// Fading out on the way to the arcade game.
    ToArcade,
    /// Fading out on the way to a full match.
    ToFull,
}

impl MenuPage {
    /// The label of the section that shows this page.
    fn label(self) -> &'static str {
        match self {
            MenuPage::Opening | MenuPage::Main => "menuFadeIn",
            MenuPage::MatchSetup | MenuPage::FullSetup => "matchIn",
            MenuPage::ArcadeSetup => "arcadeIn",
            MenuPage::HighScores | MenuPage::Mods => "highScoresIn",
            MenuPage::MatchSummary | MenuPage::FullSummary => "matchSummary",
            MenuPage::ArcadeSummary => "arcadeSummary",
            MenuPage::ToMatch | MenuPage::ToFull => "fadeOutMatch",
            MenuPage::ToArcade => "fadeOutArcade",
        }
    }
}

/// Something the menu wants that is not its own to do: another screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leave {
    Instructions,
    Match,
    Arcade,
    FullMatch,
}

/// The choice of ground on the full match's setup page: the clip it is all
/// in, and each ground's box.
struct Grounds {
    holder: Path,
    boxes: Vec<GroundBox>,
}

/// One ground to choose: its box, and what fills the box when it is the one
/// chosen.
struct GroundBox {
    ground: Ground,
    button: Path,
    fill: Path,
}

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
    /// What a full match has written on its summary page.
    summary: Option<Path>,
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
        }
    }
}

/// What makes the toss come out differently from everything else that is
/// worked out from the same seed.
const COIN_SEED: u64 = 0xbb67_ae85_84ca_a73b;
/// The lettering the full match's pages are written in is drawn 18 high.
/// These are the sizes of its lines, that being 1.
const HEADING_SIZE: f32 = 17.0 / 18.0;
const WORDS_SIZE: f32 = 14.0 / 18.0;
const CHOICE_SIZE: f32 = 12.5 / 18.0;
const WHITE: Rgb = look::WHITE;
/// On the setup page, where the choice of ground is: the middle of the top
/// of its heading, the corner of the first of its boxes, how far apart the
/// boxes are, and where each one's word is from its box.
const GROUND_HEADING: (f32, f32) = (452.5, 280.0);
const GROUND_FIRST: (f32, f32) = (357.0, 309.0);
const GROUND_PITCH: f32 = 66.0;
const GROUND_WORD: (f32, f32) = (37.0, -5.0);
/// What fills the box of the ground chosen: how far into the box it sits,
/// its size, the art's block being 1, and its colour.
const FILL_IN: f32 = 2.0;
const FILL_SIZE: f32 = 0.58;
const FILL_COLOUR: Rgb = look::NAVY;
/// On the summary page: where the badge at the start of the heading is, the
/// middle of the top of the heading's words, the middle of the lines under
/// it, and how far down each of those is.
const SUMMARY_BADGE: (f32, f32) = (333.0, 75.1);
const SUMMARY_HEADING: (f32, f32) = (421.0, 75.0);
const SUMMARY_MIDDLE: f32 = 400.0;
const SUMMARY_DOWN: [f32; 5] = [136.0, 172.0, 191.0, 227.0, 246.0];

impl Menu {
    pub fn page(&self) -> MenuPage {
        self.page
    }

    /// Makes the toss for a full match come out the same way every time.
    pub fn seed(&mut self, seed: u64) {
        self.coin = Some(Rng::new(seed ^ COIN_SEED));
    }

    /// Whether the side is at home in the full match about to begin: as
    /// chosen, or as a coin comes down. Once asked, the answer stands until
    /// it is taken.
    fn at_home(&mut self, settings: &Settings) -> bool {
        if let Some(home) = self.home {
            return home;
        }
        let home = match settings.ground {
            Ground::Home => true,
            Ground::Away => false,
            Ground::Toss => {
                let coin = self
                    .coin
                    .get_or_insert_with(|| Rng::new(Rng::seed_from_clock() ^ COIN_SEED));
                coin.below(2) == 0
            }
        };
        self.home = Some(home);
        home
    }

    /// The same, for the match that is now beginning. The next one is
    /// settled afresh.
    pub fn take_home(&mut self, settings: &Settings) -> bool {
        let home = self.at_home(settings);
        self.home = None;
        home
    }

    fn clip(stage: &Stage) -> Option<&ClipState> {
        stage.clip(&art::in_shell(stage, art::MENU)?)
    }

    /// The menu has just been put on screen. Coming from the intro it plays
    /// its whole arrival. Coming back from anywhere else it goes straight to
    /// fading in.
    pub fn shown(&mut self, from_intro: bool, game: &Game, stage: &mut Stage, library: &Library) {
        self.page = MenuPage::Opening;
        if !from_intro {
            self.open(MenuPage::Main, game, stage, library);
        }
    }

    /// Plays in another page.
    pub fn open(&mut self, page: MenuPage, game: &Game, stage: &mut Stage, library: &Library) {
        let Some(menu) = art::in_shell(stage, art::MENU) else {
            return;
        };
        if stage.goto_label(&menu, page.label(), true, library) {
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
                MenuPage::ToFull => {}
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

    /// Takes down what a full match has put on the menu's pages.
    fn clear_full(&mut self, stage: &mut Stage) {
        if let Some(grounds) = self.grounds.take() {
            stage.remove(&grounds.holder);
        }
        if let Some(holder) = self.summary.take() {
            stage.remove(&holder);
        }
    }

    /// On the full match's setup page, puts the choice of ground under the
    /// skill levels once the page is there, and keeps the box of the one
    /// chosen filled.
    fn show_grounds(&mut self, settings: &Settings, stage: &mut Stage, library: &Library) {
        // The menu may have been left and come back to since.
        if let Some(grounds) = &self.grounds
            && stage.child(&grounds.holder).is_none()
        {
            self.grounds = None;
        }
        if self.page != MenuPage::FullSetup || self.arriving {
            return;
        }
        if self.grounds.is_none() {
            let Some(menu) = art::in_shell(stage, art::MENU) else {
                return;
            };
            let depth = Stage::RULES_DEPTH + 500;
            let Some(holder) = stage.attach(&menu, art::HOLDER, depth, "grounds", library) else {
                return;
            };
            let mut sheet = Sheet::on(holder.clone(), 1, library).lettered(art::MENU_FIELD);
            let (heading, size) = (GROUND_HEADING, WORDS_SIZE);
            sheet.write(
                stage,
                "groundHeading",
                "Home or Away:",
                heading,
                size,
                WHITE,
            );
            let mut boxes = Vec::new();
            for (index, ground) in Ground::ALL.into_iter().enumerate() {
                // Each ground's things have ten depths to themselves.
                sheet.depth = 10 + index as u16 * 10;
                let at = (GROUND_FIRST.0 + GROUND_PITCH * index as f32, GROUND_FIRST.1);
                let word = (at.0 + GROUND_WORD.0, at.1 + GROUND_WORD.1);
                let size = CHOICE_SIZE;
                sheet.write(stage, "groundWord", ground.word(), word, size, WHITE);
                // The box goes on after its word, so that a click on the
                // word is a click on the box.
                let button = sheet.add(stage, art::CHOICE, "ground", at, (1.0, 1.0));
                let inside = (at.0 + FILL_IN, at.1 + FILL_IN);
                let size = (FILL_SIZE, FILL_SIZE);
                let fill = sheet.add(stage, art::BLOCK, "groundFill", inside, size);
                if let (Some(button), Some(fill)) = (button, fill) {
                    if let Some(fill) = stage.child_mut(&fill) {
                        fill.set_color(look::tint(FILL_COLOUR));
                    }
                    boxes.push(GroundBox {
                        ground,
                        button,
                        fill,
                    });
                }
            }
            self.grounds = Some(Grounds { holder, boxes });
        }
        for each in self.grounds.iter().flat_map(|grounds| &grounds.boxes) {
            if let Some(fill) = stage.child_mut(&each.fill) {
                fill.set_visible(each.ground == settings.ground);
            }
        }
    }

    /// Takes in a click on the button at `path`, which may be one of the
    /// choices of ground on the full match's setup page.
    pub fn chose(&mut self, path: &[u16], settings: &mut Settings) {
        let chosen = self
            .grounds
            .iter()
            .flat_map(|grounds| &grounds.boxes)
            .find(|each| each.button == path);
        if let Some(chosen) = chosen {
            settings.ground = chosen.ground;
        }
    }

    /// The lines of the full match's summary page.
    pub fn summary_lines(game: &Game, home: bool) -> [String; 5] {
        let innings = game.rules.full_match.innings;
        let how = match game.settings.ground {
            Ground::Toss => "The coin is tossed, and",
            Ground::Home | Ground::Away => "As you asked,",
        };
        let side = if home {
            "you are at home and bat second."
        } else {
            "you are away and bat first."
        };
        [
            format!("IT'S A FULL MATCH OF {innings} INNINGS!"),
            how.to_owned(),
            side.to_owned(),
            "The other side's innings are".to_owned(),
            "played for you. You just bat.".to_owned(),
        ]
    }

    /// On the full match's summary page, takes the art's words about the
    /// last innings out of sight and writes what a full match is to be.
    fn show_summary(&mut self, game: &Game, stage: &mut Stage, library: &Library) {
        if let Some(holder) = &self.summary
            && stage.child(holder).is_none()
        {
            self.summary = None;
        }
        if !matches!(self.page, MenuPage::FullSummary | MenuPage::ToFull) {
            return;
        }
        let Some(menu) = art::in_shell(stage, art::MENU) else {
            return;
        };
        let Some(clip) = stage.clip(&menu) else {
            return;
        };
        let (frame, last) = (clip.frame, clip.frame_count(library));
        let theirs: Vec<u16> = clip
            .children
            .iter()
            .filter(|(_, child)| art::SUMMARY_WORDS.contains(&child.symbol))
            .map(|(&depth, _)| depth)
            .collect();
        for depth in theirs {
            let mut path = menu.clone();
            path.push(depth);
            if let Some(child) = stage.child_mut(&path) {
                child.set_visible(false);
            }
        }
        if self.page == MenuPage::ToFull {
            // The page fades away over the frames that are left, and what
            // was written on it with it.
            let labels = library
                .timeline(Some(art::MENU))
                .map(|timeline| &timeline.labels);
            let first = labels
                .and_then(|labels| labels.get(MenuPage::ToFull.label()))
                .copied()
                .unwrap_or(frame);
            let over = f32::from(last.saturating_sub(first).max(1));
            let left = 1.0 - f32::from(frame.saturating_sub(first)) / over;
            if let Some(holder) = self.summary.as_ref().and_then(|path| stage.child_mut(path)) {
                holder.set_alpha(left.clamp(0.0, 1.0));
            }
            return;
        }
        if self.summary.is_some() || self.arriving {
            return;
        }
        let depth = Stage::RULES_DEPTH + 510;
        let Some(holder) = stage.attach(&menu, art::HOLDER, depth, "fullSummary", library) else {
            return;
        };
        let mut sheet = Sheet::on(holder.clone(), 1, library).lettered(art::MENU_FIELD);
        let badge = SUMMARY_BADGE;
        for symbol in art::BADGE {
            sheet.add(stage, symbol, "badge", badge, (1.0, 1.0));
        }
        let nine = (
            badge.0 + art::NINE_FROM_BADGE.0,
            badge.1 + art::NINE_FROM_BADGE.1,
        );
        sheet.add(stage, art::NINE, "nine", nine, (1.0, 1.0));
        let (heading, size) = (SUMMARY_HEADING, HEADING_SIZE);
        sheet.write(stage, "fullHeading", "FULL MATCH", heading, size, WHITE);
        let home = self.at_home(&game.settings);
        let lines = Menu::summary_lines(game, home);
        // The lines go on over all of that, from a depth of their own.
        sheet.depth = 10;
        for (line, down) in lines.iter().zip(SUMMARY_DOWN) {
            let top = (SUMMARY_MIDDLE, down);
            sheet.write(stage, "fullLine", line, top, WORDS_SIZE, WHITE);
        }
        self.summary = Some(holder);
    }

    /// Moves the marker on the setup pages to the chosen difficulty.
    fn show_difficulty(settings: &Settings, stage: &mut Stage, library: &Library) {
        let marker =
            art::in_shell(stage, art::MENU).and_then(|menu| stage.find_named(&menu, "skillSelect"));
        if let Some(marker) = marker {
            stage.goto_label(&marker, settings.difficulty.label(), false, library);
        }
    }

    /// Acts on a button the player has clicked, known by the words on it.
    pub fn clicked(
        &mut self,
        label: &str,
        game: &mut Game,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<Leave> {
        use MenuPage::{
            ArcadeSetup, ArcadeSummary, FullSetup, FullSummary, HighScores, Main, MatchSetup,
            MatchSummary, Mods, ToArcade, ToFull, ToMatch,
        };
        // A page that is still arriving cannot be used yet.
        if Menu::clip(stage).is_none_or(|menu| menu.playing) {
            return None;
        }
        let page = match (self.page, label) {
            (Main, "BOTTOM OF THE NINTH") => MatchSetup,
            (Main, "FULL MATCH") => FullSetup,
            (Main, "ARCADE") => ArcadeSetup,
            (Main, "INSTRUCTIONS") => return Some(Leave::Instructions),
            (Main, label) if label.starts_with("HIGH SCORES") => HighScores,
            (Main, "MODS") => Mods,
            (MatchSetup | ArcadeSetup | FullSetup | HighScores | Mods, "BACK") => Main,
            (MatchSummary, "BACK") => MatchSetup,
            (ArcadeSummary, "BACK") => ArcadeSetup,
            (FullSummary, "BACK") => FullSetup,
            (MatchSetup, "NEXT") => MatchSummary,
            (ArcadeSetup, "NEXT") => ArcadeSummary,
            (FullSetup, "NEXT") => FullSummary,
            (MatchSummary, "PLAY BALL") => ToMatch,
            (ArcadeSummary, "PLAY BALL") => ToArcade,
            (FullSummary, "PLAY BALL") => ToFull,
            (MatchSetup | ArcadeSetup | FullSetup, "EASY" | "MEDIUM" | "HARD") => {
                game.settings.difficulty = match label {
                    "EASY" => Difficulty::Easy,
                    "MEDIUM" => Difficulty::Medium,
                    _ => Difficulty::Hard,
                };
                Menu::show_difficulty(&game.settings, stage, library);
                return None;
            }
            _ => return None,
        };
        self.open(page, game, stage, library);
        None
    }

    /// Called once a frame while the menu is showing.
    pub fn tick(&mut self, game: &Game, stage: &mut Stage, library: &Library) -> Option<Leave> {
        let menu = Menu::clip(stage)?;
        let (frame, playing, last) = (menu.frame, menu.playing, menu.frame_count(library));
        let labels = library
            .timeline(menu.symbol)
            .map(|timeline| &timeline.labels);
        if self.arriving && !playing {
            self.arriving = false;
            Menu::show_difficulty(&game.settings, stage, library);
        }
        self.show_grounds(&game.settings, stage, library);
        self.show_summary(game, stage, library);
        match self.page {
            MenuPage::Opening if !playing => self.page = MenuPage::Main,
            MenuPage::ToMatch if frame >= last => return Some(Leave::Match),
            MenuPage::ToFull if frame >= last => return Some(Leave::FullMatch),
            // The arcade's fade-out is followed directly by the match's, so
            // it has ended when that is about to begin.
            MenuPage::ToArcade => {
                let next = labels.and_then(|labels| labels.get(MenuPage::ToMatch.label()));
                if next.is_some_and(|&next| frame + 1 >= next) {
                    return Some(Leave::Arcade);
                }
            }
            _ => {}
        }
        None
    }
}
