//! The game's own rules: which screen is showing and what each button does.
//!
//! The art only knows how to play its animations. Moving between screens is
//! decided here, by jumping the art's clips to their labelled frames.

use bb_engine::app::Logic;
use bb_engine::display::{ButtonEvent, Event, Path};
use bb_engine::input::Key;
use bb_engine::library::Library;
use bb_engine::math::Matrix;
use bb_engine::stage::Stage;
use bb_format::SymbolId;

use crate::art::{self, ButtonLabels};
use crate::board;
use crate::look::{self, Look, Rgb, Swatch};
use crate::menu::{Game, Leave, Menu, MenuPage};
use crate::mods::{Asked, Mod, Mods, ModsPage};
use crate::play::full::FullMatch;
use crate::play::overlay::Words;
use crate::play::{Match, Outcome, bullet};
use crate::rng::Rng;
use crate::rules::Rules;
use crate::scores::Scores;
use crate::settings::{Difficulty, Ground};

/// What the player is looking at. Each is a labelled frame of the shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    /// The art has not reached the shell yet.
    Loading,
    Intro,
    Menu,
    Match,
    Arcade,
    MatchLost,
    MatchWon,
    InningsTied,
    ArcadeFinish,
    Instructions,
    /// A full match, while the player's side is batting. It is played on
    /// the match's own frame of the shell.
    FullMatch,
    /// The board a full match shows between innings, which is the one the
    /// art has for an innings that was tied.
    Interval,
}

impl Screen {
    /// Whether a game is being played on this screen.
    fn is_game(self) -> bool {
        matches!(self, Screen::Match | Screen::FullMatch | Screen::Arcade)
    }

    const ALL: [Screen; 9] = [
        Screen::Intro,
        Screen::Menu,
        Screen::Match,
        Screen::Arcade,
        Screen::MatchLost,
        Screen::MatchWon,
        Screen::InningsTied,
        Screen::ArcadeFinish,
        Screen::Instructions,
    ];

    /// What a full match is called where a screen is asked for by its
    /// label, the art having none for it.
    pub const FULL_MATCH: &str = "fullMatch";

    /// The screen the shell shows on the frame with this label.
    pub fn from_label(label: &str) -> Option<Screen> {
        if label == Screen::FULL_MATCH {
            return Some(Screen::FullMatch);
        }
        Screen::ALL
            .into_iter()
            .find(|screen| screen.label() == Some(label))
    }

    /// The shell's label for this screen.
    fn label(self) -> Option<&'static str> {
        Some(match self {
            Screen::Loading => return None,
            Screen::Intro => "intro",
            Screen::Menu => "menu",
            Screen::Match | Screen::FullMatch => "match",
            Screen::Arcade => "arcade",
            Screen::MatchLost => "matchLost",
            Screen::MatchWon => "matchWon",
            Screen::InningsTied | Screen::Interval => "inningsTied",
            Screen::ArcadeFinish => "arcadeFinish",
            Screen::Instructions => "instructionsAll",
        })
    }
}

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

/// How many frames into a result screen the line about zingers is written,
/// which is when the screen has got to its figures, and how far down the
/// screen: on the ones a match ends on, and on the arcade game's.
const ZINGER_LINE_AFTER: u32 = 260;
const ZINGER_LINE_TOP: (f32, f32) = (262.0, 325.0);
const ZINGER_LINE_SIZE: f32 = 0.8;
const ZINGER_LINE_COLOUR: Rgb = look::CREAM;

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
        self.mods_file = Some(file);
    }

    /// Switches a mod on or off for this run, without writing that down.
    pub fn switch_mod(&mut self, which: Mod, on: bool) {
        self.game.mods.set(which, on);
    }

    /// Sets a mod's setting to a level for this run, without writing that
    /// down.
    pub fn set_mod_level(&mut self, which: Mod, level: u8) {
        self.game.mods.set_level(which, level);
    }

    /// Acts on a click on one of the boxes on the mods' page.
    fn choose_mod(&mut self, path: &[u16]) {
        match self.mods_page.clicked(path) {
            Some(Asked::Switch(which)) => {
                self.game.mods.toggle(which);
            }
            Some(Asked::Level(which, level)) => self.game.mods.set_level(which, level),
            None => return,
        }
        if let Some(file) = &self.mods_file
            && let Err(error) = self.game.mods.save(file)
        {
            eprintln!("The choice of mods could not be saved: {error:#}");
        }
    }

    /// Writes the score table over the panel on the high-score page, for as
    /// long as that page is up.
    fn show_scores(&mut self, stage: &mut Stage, library: &Library) {
        // The lines go when the panel does, as the page is left.
        if self
            .table
            .first()
            .is_some_and(|line| stage.child(line).is_none())
        {
            self.table.clear();
        }
        let on_page = self.screen == Screen::Menu && self.menu.page() == MenuPage::HighScores;
        if !on_page || !self.table.is_empty() {
            return;
        }
        let Some(panel) =
            art::shell(stage).and_then(|shell| stage.find_symbol(&shell, art::SCORE_PANEL))
        else {
            return;
        };
        // The panel says the scores are kept on a web site. Here they are
        // not, so that goes and the table takes its place.
        let notice: Vec<Path> = stage.clip(&panel).map_or(Vec::new(), |clip| {
            clip.children
                .iter()
                .filter(|(_, child)| art::SCORE_PANEL_NOTICE.contains(&child.symbol))
                .map(|(&depth, _)| {
                    let mut path = panel.clone();
                    path.push(depth);
                    path
                })
                .collect()
        });
        if notice.is_empty() {
            // The panel has not finished arriving.
            return;
        }
        for path in notice {
            if let Some(child) = stage.child_mut(&path) {
                child.set_visible(false);
            }
        }
        let Some(field) = library.edit_texts.get(&art::TABLE_FIELD) else {
            return;
        };
        // The field centres what it says, so a line is placed by its middle.
        let middle = ((field.bounds.x_min + field.bounds.x_max) / 2.0) as f32;
        // What to write, where its middle goes, and in what colour: dark
        // on the panel's white, and white for the heading on its bar. The
        // heading was drawn in one piece with the notice, so it is written
        // back in.
        const DARK: Rgb = look::NAVY;
        const WHITE: Rgb = look::WHITE;
        let mut lines = vec![("HIGHSCORES".to_owned(), -104.0, -123.0, WHITE)];
        if self.scores.entries.is_empty() {
            lines.push(("NO SCORES YET".to_owned(), 0.0, -10.0, DARK));
        }
        for (place, entry) in self.scores.entries.iter().enumerate() {
            let down = -88.0 + place as f32 * 19.0;
            lines.push((format!("{}", place + 1), -150.0, down, DARK));
            lines.push((entry.name.to_uppercase(), -35.0, down, DARK));
            lines.push((entry.points.to_string(), 125.0, down, DARK));
        }
        const SIZE: f32 = 0.8;
        for (index, (text, across, down, colour)) in lines.into_iter().enumerate() {
            let depth = Stage::RULES_DEPTH + 200 + index as u16;
            let Some(path) = stage.attach(&panel, art::TABLE_FIELD, depth, "scoreLine", library)
            else {
                continue;
            };
            if let Some(child) = stage.child_mut(&path) {
                child.said = Some(text);
                child.set_matrix(Matrix {
                    a: SIZE,
                    d: SIZE,
                    tx: across - middle * SIZE,
                    ty: down,
                    ..Matrix::IDENTITY
                });
                child.set_color(look::tint(colour));
            }
            self.table.push(path);
        }
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

    /// The colour under the pointer on one of the setup pages' strips.
    fn picked(stage: &Stage, strip: Option<&Swatch>, name: &str) -> Option<Rgb> {
        let shell = art::shell(stage)?;
        let path = stage.find_named(&shell, name)?;
        let (x, y) = stage.from_stage(&path, stage.pointer.x, stage.pointer.y)?;
        strip?.at(x, y)
    }

    /// Acts on a click on one of the setup pages' colour and logo choices.
    fn choose_look(&mut self, button: SymbolId, stage: &Stage) {
        let settings = &mut self.game.settings;
        if art::CLOTHES_STRIP_BUTTONS.contains(&button) {
            if let Some(colour) =
                Baseball::picked(stage, self.clothes_strip.as_ref(), "clothesPicker")
            {
                settings.clothes = Some(colour);
            }
        } else if art::SKIN_STRIP_BUTTONS.contains(&button) {
            if let Some(colour) = Baseball::picked(stage, self.skin_strip.as_ref(), "skinPicker") {
                settings.skin = Some(colour);
            }
        } else if button == art::CLOTHES_BUTTON.0 {
            settings.clothes = Some(art::CLOTHES_BUTTON.1);
        } else if button == art::SKIN_BUTTON.0 {
            settings.skin = Some(art::SKIN_BUTTON.1);
        } else if let Some((_, logo)) = art::LOGO_BUTTONS.iter().find(|(id, _)| *id == button) {
            settings.logo = Some((*logo).to_owned());
        }
    }

    /// How the batting side should look on the screen that is showing.
    fn look(&self) -> Look {
        let settings = &self.game.settings;
        match (&self.play, self.screen) {
            (Some(play), Screen::Match | Screen::FullMatch) => play.look(settings.clothes),
            // The arcade game and the setup pages show what was chosen.
            _ => Look {
                clothes: settings.clothes,
                skin: settings.skin,
                logo: settings.logo.clone(),
                second_skin: None,
            },
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
    }

    /// Opens on `screen` instead of the intro.
    pub fn start_on(&mut self, screen: Screen) {
        self.first = Some(screen);
    }

    /// Has the shell show another screen, with what is heard on it. Returns
    /// whether the shell was there to do it.
    fn switch(&mut self, screen: Screen, stage: &mut Stage, library: &Library) -> bool {
        let (Some(shell), Some(label)) = (art::shell(stage), screen.label()) else {
            return false;
        };
        self.holds.clear();
        if let Some(lines) = self.result_lines.take() {
            stage.remove(&lines);
        }
        self.pages = None;
        // What was written on the board went with the board.
        self.board = None;
        // A game with the southpaw mod on has the stage draw what is
        // written the right way round, for the number on the batter's
        // shirt. Nothing on any other screen is mirrored.
        stage.upright_text = false;
        self.result_frames = 0;
        stage.goto_label(&shell, label, false, library);
        self.screen = screen;
        self.sound_for(screen, stage, library);
        true
    }

    /// Switches to another screen, and starts the game that is played on
    /// it if one is.
    fn show(&mut self, screen: Screen, stage: &mut Stage, library: &Library) {
        let from_intro = self.screen == Screen::Intro;
        if !self.switch(screen, stage, library) {
            return;
        }
        self.finished = None;
        let seed = self.seed.unwrap_or_else(Rng::seed_from_clock);
        self.play = match screen {
            Screen::Match => {
                self.playing = self.game.as_played(true);
                Some(Match::new(&self.playing, seed, library))
            }
            Screen::FullMatch => {
                self.playing = self.game.as_played(true);
                let home = self.menu.take_home(&self.game.settings);
                Some(Match::new_full(&self.playing, home, seed, library))
            }
            Screen::Arcade => {
                self.playing = self.game.as_played(false);
                Some(Match::new_arcade(&self.playing, seed, library))
            }
            _ => None,
        };
        if let Some(play) = &mut self.play {
            play.set_zinger_record(self.scores.longest_zinger);
            self.last_zinger = 0;
        }
        if screen == Screen::Menu {
            self.menu.shown(from_intro, &self.game, stage, library);
        }
        // At home in a full match the other side has batted already, and
        // the board says how before a ball is thrown.
        let at_home = self.play.as_ref().and_then(Match::full);
        if at_home.is_some_and(FullMatch::at_home) {
            self.switch(Screen::Interval, stage, library);
        }
    }

    /// The board between innings has been read: back to the batting.
    fn bat_again(&mut self, stage: &mut Stage, library: &Library) {
        let Some(play) = &mut self.play else {
            return;
        };
        play.bat_again();
        self.switch(Screen::FullMatch, stage, library);
    }

    /// While the board between innings is up, keeps the art's own words
    /// off it, and writes the full match's once the board has arrived.
    fn show_interval(&mut self, stage: &mut Stage, library: &Library) {
        if self.screen != Screen::Interval {
            return;
        }
        let Some(path) = art::in_shell(stage, art::BOARD) else {
            return;
        };
        let Some(clip) = stage.clip(&path) else {
            return;
        };
        let arrived = clip.frame >= art::BOARD_WORDS_FRAME;
        let theirs: Vec<u16> = clip
            .children
            .iter()
            .filter(|(_, child)| art::BOARD_WORDS.contains(&child.symbol))
            .map(|(&depth, _)| depth)
            .collect();
        for depth in theirs {
            let mut child = path.clone();
            child.push(depth);
            if let Some(child) = stage.child_mut(&child) {
                child.set_visible(false);
            }
        }
        if !arrived || self.board.is_some() {
            return;
        }
        if let Some(full) = self.play.as_ref().and_then(Match::full) {
            self.board = board::interval(full, &path, stage, library);
        }
    }

    /// Keeps a zinger that has beaten the longest there had been, as soon
    /// as it has, so that leaving the game early does not lose it.
    fn keep_zinger_record(&mut self) {
        let Some(play) = &self.play else {
            return;
        };
        if play.zinger_record() <= self.scores.longest_zinger {
            return;
        }
        self.scores.longest_zinger = play.zinger_record();
        if let Some(file) = &self.scores_file
            && let Err(error) = self.scores.save(file)
        {
            eprintln!("The longest zinger could not be saved: {error:#}");
        }
    }

    /// Writes on a result screen what the art has no place for, once the
    /// screen has got to its figures: the pages of a full match, and the
    /// longest zinger of the game just finished with the longest there has
    /// ever been. A game with no zinger in it has no such line.
    fn show_result_lines(&mut self, stage: &mut Stage, library: &Library) {
        if let Some(pages) = &self.pages {
            pages.keep(stage);
        }
        let top = match self.screen {
            Screen::MatchWon | Screen::MatchLost | Screen::InningsTied => ZINGER_LINE_TOP.0,
            Screen::ArcadeFinish => ZINGER_LINE_TOP.1,
            _ => return,
        };
        let nothing = self.last_zinger == 0 && self.finished.is_none();
        if nothing || self.result_lines.is_some() {
            return;
        }
        self.result_frames += 1;
        if self.result_frames < ZINGER_LINE_AFTER {
            return;
        }
        let Some(shell) = art::shell(stage) else {
            return;
        };
        let depth = Stage::RULES_DEPTH + 400;
        let Some(holder) = stage.attach(&shell, art::HOLDER, depth, "resultLines", library) else {
            return;
        };
        self.result_lines = Some(holder.clone());
        let zingers = (self.last_zinger > 0).then(|| {
            format!(
                "LONGEST ZINGER {} FT   BEST EVER {} FT",
                self.last_zinger, self.scores.longest_zinger
            )
        });
        if let Some(full) = &self.finished {
            // A full match has pages, and what there is to say of zingers
            // is on the first of them.
            self.pages = board::Pages::new(full, zingers, &holder, stage, library);
            return;
        }
        let Some(zingers) = zingers else {
            return;
        };
        let middle = library.manifest.stage.width as f32 / 2.0;
        if let Some(words) = Words::new(
            &holder,
            1,
            "zingerLine",
            (middle, top),
            ZINGER_LINE_SIZE,
            stage,
            library,
        ) {
            words.say(&zingers, ZINGER_LINE_COLOUR, stage);
        }
    }

    /// Takes in a click on the button at `path`, which may be one of the
    /// arrows that turn the pages of a finished full match.
    fn turn_page(&mut self, path: &[u16], stage: &mut Stage, library: &Library) {
        if let Some(pages) = &mut self.pages {
            pages.clicked(path, stage, library);
        }
    }

    /// Goes where the menu has asked to go.
    fn leave_menu(&mut self, leave: Leave, stage: &mut Stage, library: &Library) {
        let screen = match leave {
            Leave::Instructions => Screen::Instructions,
            Leave::Match => Screen::Match,
            Leave::Arcade => Screen::Arcade,
            Leave::FullMatch => Screen::FullMatch,
        };
        self.show(screen, stage, library);
    }

    /// Acts on a button the player has clicked.
    fn clicked(&mut self, button: SymbolId, stage: &mut Stage, library: &Library) {
        let label = self.labels.get(button).unwrap_or_default().to_owned();
        let label = label.as_str();
        match self.screen {
            Screen::Intro if label == "SKIP" => self.show(Screen::Menu, stage, library),
            Screen::Menu => {
                let leave = self.menu.clicked(label, &mut self.game, stage, library);
                if let Some(leave) = leave {
                    self.leave_menu(leave, stage, library);
                }
            }
            Screen::Match | Screen::FullMatch | Screen::Arcade => match label {
                "QUIT" => self.quit_prompt(true, stage, library),
                "NO" => self.quit_prompt(false, stage, library),
                "YES" => self.show(Screen::Menu, stage, library),
                _ => {}
            },
            Screen::MatchLost | Screen::MatchWon | Screen::InningsTied | Screen::ArcadeFinish => {
                if label.contains("HIGH SCORES") {
                    self.show(Screen::Menu, stage, library);
                    self.menu
                        .open(MenuPage::HighScores, &self.game, stage, library);
                } else if label == "MAIN MENU" || art::CONTINUE_BUTTONS.contains(&button) {
                    self.show(Screen::Menu, stage, library);
                }
            }
            Screen::Interval => {
                if art::CONTINUE_BUTTONS.contains(&button) {
                    self.bat_again(stage, library);
                }
            }
            Screen::Instructions => self.instructions_clicked(label, stage, library),
            Screen::Loading | Screen::Intro => {}
        }
    }

    /// Slides the "are you sure?" panel on, or puts it away.
    fn quit_prompt(&mut self, on: bool, stage: &mut Stage, library: &Library) {
        let Some(prompt) = stage.find_symbol(&[], art::QUIT_PROMPT) else {
            return;
        };
        if on {
            // The slide runs to the end of the clip, which would loop back
            // to its hidden first frame if left to play on.
            let last = stage
                .clip(&prompt)
                .map_or(1, |clip| clip.frame_count(library));
            stage.goto_label(&prompt, "onScreen", true, library);
            self.holds.push((prompt, last));
        } else {
            self.holds.retain(|(path, _)| *path != prompt);
            stage.goto_label(&prompt, "offScreen", false, library);
        }
    }

    fn instructions_clicked(&mut self, label: &str, stage: &mut Stage, library: &Library) {
        let Some(path) = art::INSTRUCTIONS
            .iter()
            .find_map(|&clip| art::in_shell(stage, clip))
        else {
            return;
        };
        match label {
            "MENU" | "SKIP" => self.show(Screen::Menu, stage, library),
            // Each page stops at its end, so going on is just playing again.
            "NEXT" => {
                if let Some(clip) = stage.clip_mut(&path) {
                    clip.playing = true;
                }
            }
            "BACK" => {
                let Some(clip) = stage.clip(&path) else {
                    return;
                };
                let Some(timeline) = library.timeline(clip.symbol) else {
                    return;
                };
                // The pages start at the labelled frames. The one being
                // shown is the last to start at or before this frame, and
                // the one to go back to is the one before that.
                let mut starts: Vec<u16> = timeline.labels.values().copied().collect();
                starts.sort_unstable();
                let before: Vec<u16> = starts
                    .into_iter()
                    .filter(|&start| start <= clip.frame)
                    .collect();
                if let [.., previous, _] = before[..] {
                    stage.goto_clip(&path, previous, library);
                    if let Some(clip) = stage.clip_mut(&path) {
                        clip.playing = true;
                    }
                }
            }
            _ => {}
        }
    }

    /// Stops each clip that was left playing to a frame, once it is there.
    fn stop_held_clips(&mut self, stage: &mut Stage) {
        self.holds
            .retain(|(path, frame)| match stage.clip_mut(path) {
                Some(clip) if clip.frame >= *frame => {
                    clip.playing = false;
                    false
                }
                Some(_) => true,
                // The clip has gone, and its hold with it.
                None => false,
            });
    }

    /// Plays a frame of the game in hand, if one is being played, and
    /// moves on when it has come to the board between innings or to its
    /// end.
    fn play_a_frame(&mut self, stage: &mut Stage, library: &Library) {
        // A game that has stopped for the board between innings waits.
        let outcome = match (&mut self.play, self.screen.is_game()) {
            (Some(play), true) => play.tick(&self.playing, stage, library),
            _ => None,
        };
        self.keep_zinger_record();
        match outcome {
            None => {}
            Some(Outcome::Interval) => {
                self.switch(Screen::Interval, stage, library);
            }
            Some(outcome) => self.end_the_game(outcome, stage, library),
        }
    }

    /// The game is over: its result is shown on the screen for how it
    /// ended, and an arcade score goes in the table.
    fn end_the_game(&mut self, outcome: Outcome, stage: &mut Stage, library: &Library) {
        let Some(play) = &mut self.play else {
            return;
        };
        let longest = play.longest_zinger();
        let finished = play.full().cloned();
        play.show_result(stage);
        play.show_arcade_result(&self.playing, stage);
        if let Some(points) = play.arcade_score(&self.playing) {
            // Under the name typed on the setup page, if one was.
            let name = stage
                .text("playerName")
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .unwrap_or("PLAYER")
                .to_owned();
            if self.scores.add(&name, points)
                && let Some(file) = &self.scores_file
                && let Err(error) = self.scores.save(file)
            {
                eprintln!("The score could not be saved: {error:#}");
            }
        }
        self.show(screen_after(outcome), stage, library);
        self.last_zinger = longest;
        self.finished = finished;
    }

    /// The arcade game's finish screen names the skill level played, on a
    /// clip with a frame for each. It appears part of the way through the
    /// screen's arrival, so it is set whenever it is there.
    fn name_the_skill_played(&self, stage: &mut Stage, library: &Library) {
        if self.screen != Screen::ArcadeFinish {
            return;
        }
        let Some(shell) = art::shell(stage) else {
            return;
        };
        let frame = match self.game.settings.difficulty {
            Difficulty::Easy => 1,
            Difficulty::Medium => 2,
            Difficulty::Hard => 3,
        };
        for path in art::all_named(stage, &shell, "skillLevelText") {
            stage.goto_clip(&path, frame, library);
            if let Some(clip) = stage.clip_mut(&path) {
                clip.playing = false;
            }
        }
    }

    /// A pointer hidden for aiming comes back for the quit prompt, and
    /// whenever no game is being played.
    fn give_the_pointer_back(&self, stage: &mut Stage) {
        let prompt_up = stage
            .find_symbol(&[], art::QUIT_PROMPT)
            .and_then(|path| stage.clip(&path))
            .is_some_and(|prompt| prompt.frame > 1);
        if self.play.is_none() || prompt_up || !self.screen.is_game() {
            stage.hide_pointer = false;
        }
    }

    /// Goes from a screen that leads to another by itself, once it is time.
    fn move_on(&mut self, stage: &mut Stage, library: &Library) {
        match self.screen {
            Screen::Loading => {
                if art::shell(stage).is_some() {
                    match self.first.take() {
                        Some(first) => self.show(first, stage, library),
                        None => self.screen = Screen::Intro,
                    }
                }
            }
            // The intro leads into the menu when it has played out.
            Screen::Intro => {
                let finished = art::in_shell(stage, art::INTRO)
                    .and_then(|path| stage.clip(&path))
                    .is_some_and(|intro| intro.frame >= intro.frame_count(library));
                if finished {
                    self.show(Screen::Menu, stage, library);
                }
            }
            Screen::Menu => {
                if let Some(leave) = self.menu.tick(&self.game, stage, library) {
                    self.leave_menu(leave, stage, library);
                }
            }
            _ => {}
        }
    }
}

/// The screen that shows how a game came out.
fn screen_after(outcome: Outcome) -> Screen {
    match outcome {
        Outcome::Won => Screen::MatchWon,
        Outcome::Lost => Screen::MatchLost,
        Outcome::Tied => Screen::InningsTied,
        Outcome::ArcadeOver => Screen::ArcadeFinish,
        Outcome::Interval => Screen::Interval,
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
            && self.game.mods.is_on(Mod::BulletTime)
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
