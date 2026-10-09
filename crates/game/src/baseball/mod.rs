//! The game's own rules: which screen is showing and what each button does.
//!
//! The art only knows how to play its animations. Moving between screens is
//! decided here, by jumping the art's clips to their labelled frames.

mod after;
mod choices;
mod playing;
mod scores;
mod screen;
mod screens;

use bb_engine::app::Logic;
use bb_engine::display::{ButtonEvent, Event, Path};
use bb_engine::input::Key;
use bb_engine::library::Library;
use bb_engine::stage::Stage;

use crate::art::{self, ButtonLabels};
use crate::board;
use crate::game::Game;
use crate::look::{self, Swatch};
use crate::menu::{Menu, MenuPage};
use crate::mods::{Mod, Mods, ModsPage};
use crate::play::full::FullMatch;
use crate::play::{Match, bullet};
use crate::rules::Rules;
use crate::scores::Scores;
use crate::settings::Ground;
pub use screen::Screen;

pub struct Baseball {
    screen: Screen,
    menu: Menu,
    game: Game,
    /// The game as the one in hand is being played: with what the mods
    /// change of its numbers laid over them.
    playing: Game,
    labels: ButtonLabels,
    /// The screen to open on, if not the intro.
    first: Option<Screen>,
    /// The match being played, while the match screen is showing.
    play: Option<Match>,
    /// What the game's chances are worked out from, if not the clock.
    seed: Option<u64>,
    scores: Scores,
    /// Where the scores are kept. `None` keeps them only for this run.
    scores_file: Option<std::path::PathBuf>,
    /// The lines of the score table on the stage, while its page is up.
    table: Vec<Path>,
    /// Where the choice of mods is kept. `None` keeps it only for this run.
    mods_file: Option<std::path::PathBuf>,
    /// The list of mods on the stage, while its page is up.
    mods_page: ModsPage,
    /// Whether the menu's music and the game's crowd are being heard.
    music_on: bool,
    crowd_on: bool,
    /// The strips on the setup pages that colours are picked from.
    clothes_strip: Option<Swatch>,
    skin_strip: Option<Swatch>,
    /// Clips that are playing an animation and must stop when they reach
    /// this frame, where the art has no stop of its own.
    holds: Vec<(Path, u16)>,
    /// The longest zinger of the game just finished, in feet, how many
    /// frames its result has been showing, and what has been written on
    /// that screen about it, once that is up.
    last_zinger: u32,
    result_frames: u32,
    result_lines: Option<Path>,
    /// The full match just finished, which its result screen tells of.
    finished: Option<FullMatch>,
    /// What a full match has written on the board between innings, while
    /// that is up.
    board: Option<Path>,
    /// The pages of the full match just finished, once they are up on its
    /// result screen.
    pages: Option<board::Pages>,
}

impl Baseball {
    pub fn new(library: &Library) -> Baseball {
        Baseball {
            screen: Screen::Loading,
            menu: Menu::default(),
            game: Game::default(),
            playing: Game::default(),
            labels: ButtonLabels::read(library),
            first: None,
            play: None,
            seed: None,
            scores: Scores::default(),
            scores_file: None,
            table: Vec::new(),
            mods_file: None,
            mods_page: ModsPage::default(),
            music_on: false,
            crowd_on: false,
            // A game whose art has no such strip is played in the art's own
            // colours.
            clothes_strip: Swatch::of_clip(library, art::CLOTHES_STRIP).ok(),
            skin_strip: Swatch::of_clip(library, art::SKIN_STRIP).ok(),
            holds: Vec::new(),
            last_zinger: 0,
            result_frames: 0,
            result_lines: None,
            finished: None,
            board: None,
            pages: None,
        }
    }

    /// Where the side plays a full match, for this run.
    pub fn play_on(&mut self, ground: Ground) {
        self.game.settings.ground = ground;
    }

    /// Keeps the high scores in this file, starting from what it holds.
    pub fn keep_scores_in(&mut self, file: std::path::PathBuf) {
        self.scores = Scores::load(&file);
        self.scores_file = Some(file);
    }

    /// Keeps the choice of mods in this file, starting from what it holds.
    pub fn keep_mods_in(&mut self, file: std::path::PathBuf) {
        self.game.mods = Mods::load(&file);
        self.game.mods.keep_within(&self.game.rules);
        self.mods_file = Some(file);
    }

    /// Starts and stops the music and the crowd for the screen being shown.
    /// The music belongs to the menu and the screens a game ends on. The
    /// crowd is heard under a game.
    fn sound_for(&mut self, screen: Screen, stage: &mut Stage, library: &Library) {
        let sound = &self.game.rules.sound;
        for (name, level) in &sound.levels {
            stage.set_sound_level(name, *level, library);
        }
        let in_game = screen.is_game();
        let wants_music = screen == Screen::Menu;
        let stops_music = in_game || screen == Screen::Instructions;
        if wants_music && !self.music_on {
            self.music_on = stage.play_sound(&sound.music, 999, library);
        } else if stops_music && self.music_on {
            stage.stop_sound(&sound.music, library);
            self.music_on = false;
        }
        if in_game && !self.crowd_on {
            self.crowd_on = stage.play_sound(&sound.crowd, 999, library);
        } else if screen == Screen::Menu && self.crowd_on {
            stage.stop_sound(&sound.crowd, library);
            self.crowd_on = false;
        }
    }

    /// Makes every game go the same way, for a test or for chasing a fault.
    pub fn seed(&mut self, seed: u64) {
        self.seed = Some(seed);
        self.menu.seed(seed);
    }

    /// Plays by `rules` instead of the ones built in.
    pub fn play_by(&mut self, rules: Rules) {
        self.game.rules = rules;
        // These rules may give a mod's setting fewer levels than it is at.
        self.game.mods.keep_within(&self.game.rules);
    }

    /// Opens on `screen` instead of the intro.
    pub fn start_on(&mut self, screen: Screen) {
        self.first = Some(screen);
    }
}

impl Logic for Baseball {
    fn event(&mut self, event: &Event, stage: &mut Stage, library: &Library) {
        if let (Some(play), true) = (&mut self.play, self.screen.is_game()) {
            play.event(event, &self.playing, stage, library);
        }
        if let Event::Button {
            symbol,
            path,
            event: ButtonEvent::Release,
        } = event
        {
            self.choose_look(*symbol, stage);
            self.choose_mod(path);
            self.menu.chose(path, &mut self.game.settings);
            self.turn_page(path, stage, library);
            self.clicked(*symbol, stage, library);
        }
    }

    fn tick(&mut self, stage: &mut Stage, library: &Library) {
        self.stop_held_clips(stage);
        self.play_a_frame(stage, library);
        self.show_interval(stage, library);
        self.show_result_lines(stage, library);
        self.name_the_skill_played(stage, library);
        if let Some(shell) = art::shell(stage) {
            look::dress(stage, &shell, &self.look(), library);
        }
        self.show_scores(stage, library);
        let on_mods = self.screen == Screen::Menu && self.menu.page() == MenuPage::Mods;
        self.mods_page
            .show(on_mods, &self.game.mods, &self.game.rules, stage, library);
        self.give_the_pointer_back(stage);
        self.move_on(stage, library);
    }

    fn key(&mut self, key: &Key, _stage: &mut Stage, _library: &Library) -> bool {
        // With bullet time on, the space bar is the game's while a game
        // is being played.
        *key == bullet::KEY
            && self.play.is_some()
            && self.screen.is_game()
            && self.playing.mods.is_on(Mod::BulletTime)
    }

    fn describe(&self) -> String {
        match self.screen {
            Screen::Menu => {
                // The mods that are on are named, when any are, each with
                // the level its setting is at if it has one.
                let mods: Vec<String> = self
                    .game
                    .mods
                    .all_on()
                    .map(|which| match which.setting() {
                        Some(_) => format!("{}={}", which.key(), self.game.mods.level(which)),
                        None => which.key().to_owned(),
                    })
                    .collect();
                let mods = if mods.is_empty() {
                    String::new()
                } else {
                    format!(", with {}", mods.join(" and "))
                };
                format!(
                    "Menu, {:?}, {:?}{mods}",
                    self.menu.page(),
                    self.game.settings.difficulty
                )
            }
            screen => {
                // A game says how it stands, and the screen a full match
                // ended on says how it went.
                let play = match (&self.play, &self.finished) {
                    (Some(play), _) => format!(": {}", play.describe()),
                    (None, Some(full)) => {
                        let page = self.pages.as_ref().map_or(String::new(), |pages| {
                            let (page, of) = pages.at();
                            format!(", page {page} of {of}")
                        });
                        format!(": {}, {}{page}", full.verdict(), full.describe())
                    }
                    (None, None) => String::new(),
                };
                format!("{screen:?}, {:?}{play}", self.game.settings.difficulty)
            }
        }
    }
}
