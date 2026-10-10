//! Going from one screen to another, and what is clicked on the ones
//! that are not a game.

use bb_engine::stage::Stage;
use bb_format::SymbolId;

use super::Baseball;
use super::screen::Screen;
use crate::art;
use crate::menu::{Leave, MenuPage};
use crate::play::full::FullMatch;
use crate::play::{Match, play_from};
use crate::rng::Rng;

impl Baseball {
    /// Has the shell show another screen, with what is heard on it. Returns
    /// whether the shell was there to do it.
    pub(super) fn switch(&mut self, screen: Screen, stage: &mut Stage) -> bool {
        let (Some(shell), Some(label)) = (art::shell(stage), screen.label()) else {
            return false;
        };
        self.holds.clear();
        if let Some(lines) = self.result_lines.take() {
            stage.remove(&lines);
        }
        self.pages = None;
        if let Some(tables) = self.tables.take() {
            tables.take_down(stage);
        }
        // What was written on the board went with the board.
        self.board = None;
        // A game with the southpaw mod on has the stage draw what is
        // written the right way round, for the number on the batter's
        // shirt. Nothing on any other screen is mirrored.
        stage.upright_text = false;
        self.result_frames = 0;
        stage.goto_label(&shell, label, false);
        self.screen = screen;
        self.sound_for(screen, stage);
        true
    }

    /// Switches to another screen, and starts the game that is played on
    /// it if one is.
    pub(super) fn show(&mut self, screen: Screen, stage: &mut Stage) {
        let from_intro = self.screen == Screen::Intro;
        if !self.switch(screen, stage) {
            return;
        }
        self.finished = None;
        // The tables are of a tournament, which is drawn if there is none.
        if screen == Screen::Tournament && self.tournament.is_none() {
            self.draw_tournament(stage);
            self.tell_the_menu();
        }
        let seed = self.seed.unwrap_or_else(Rng::seed_from_clock);
        self.play = match screen {
            Screen::Match => {
                self.playing = self.game.as_played(true);
                Some(Match::new(&self.playing, seed, stage.library()))
            }
            Screen::FullMatch => {
                self.playing = self.game.as_played(true);
                let home = self.menu.take_home(&self.game.settings);
                Some(Match::new_full(&self.playing, home, seed, stage.library()))
            }
            Screen::Arcade => {
                self.playing = self.game.as_played(false);
                Some(Match::new_arcade(&self.playing, seed, stage.library()))
            }
            _ => None,
        };
        if let Some(play) = &mut self.play {
            play.set_zinger_record(self.scores.longest_zinger);
            self.last_zinger = 0;
        }
        if screen == Screen::Menu {
            self.menu.shown(from_intro, &self.game, stage);
        }
        // At home in a full match the other side has batted already, and
        // the board says how before a ball is thrown.
        let at_home = self.play.as_ref().and_then(Match::full);
        if at_home.is_some_and(FullMatch::at_home) {
            self.switch(Screen::Interval, stage);
        }
    }

    /// Goes where the menu has asked to go.
    pub(super) fn leave_menu(&mut self, leave: Leave, stage: &mut Stage) {
        let screen = match leave {
            Leave::Instructions => Screen::Instructions,
            Leave::Match => Screen::Match,
            Leave::Arcade => Screen::Arcade,
            Leave::FullMatch => Screen::FullMatch,
            Leave::Tables => Screen::Tournament,
            Leave::Draw | Leave::GiveUp => return self.about_the_tournament(leave, stage),
        };
        self.show(screen, stage);
    }

    /// Acts on a button the player has clicked.
    pub(super) fn clicked(&mut self, button: SymbolId, stage: &mut Stage) {
        let label = self.labels.get(button).unwrap_or_default().to_owned();
        let label = label.as_str();
        match self.screen {
            Screen::Intro if label == "SKIP" => self.show(Screen::Menu, stage),
            Screen::Menu => {
                let leave = self.menu.clicked(label, &mut self.game, stage);
                if let Some(leave) = leave {
                    self.leave_menu(leave, stage);
                }
            }
            Screen::Match | Screen::FullMatch | Screen::Arcade => match label {
                "QUIT" => self.quit_prompt(true, stage),
                "NO" => self.quit_prompt(false, stage),
                "YES" => self.show(Screen::Menu, stage),
                _ => {}
            },
            Screen::MatchLost | Screen::MatchWon | Screen::InningsTied | Screen::ArcadeFinish => {
                if label.contains("HIGH SCORES") {
                    self.show(Screen::Menu, stage);
                    self.menu.open(MenuPage::HighScores, &self.game, stage);
                } else if label == "MAIN MENU" || art::CONTINUE_BUTTONS.contains(&button) {
                    self.show(Screen::Menu, stage);
                }
            }
            Screen::Interval => {
                if art::CONTINUE_BUTTONS.contains(&button) {
                    self.bat_again(stage);
                }
            }
            Screen::Tournament => {
                // On from the tables is the menu's page of what is next.
                if art::CONTINUE_BUTTONS.contains(&button) {
                    self.show(Screen::Menu, stage);
                    self.menu
                        .open(MenuPage::TournamentSummary, &self.game, stage);
                }
            }
            Screen::Instructions => self.instructions_clicked(label, stage),
            Screen::Loading | Screen::Intro => {}
        }
    }

    /// Slides the "are you sure?" panel on, or puts it away.
    fn quit_prompt(&mut self, on: bool, stage: &mut Stage) {
        let Some(prompt) = stage.find_symbol(&[], art::QUIT_PROMPT) else {
            return;
        };
        if on {
            // The slide runs to the end of the clip, which would loop back
            // to its hidden first frame if left to play on.
            let last = stage
                .clip(&prompt)
                .map_or(1, |clip| clip.frame_count(stage.library()));
            stage.goto_label(&prompt, "onScreen", true);
            self.holds.push((prompt, last));
        } else {
            self.holds.retain(|(path, _)| *path != prompt);
            stage.goto_label(&prompt, "offScreen", false);
        }
    }

    fn instructions_clicked(&mut self, label: &str, stage: &mut Stage) {
        let Some(path) = art::INSTRUCTIONS
            .iter()
            .find_map(|&clip| art::in_shell(stage, clip))
        else {
            return;
        };
        match label {
            "MENU" | "SKIP" => self.show(Screen::Menu, stage),
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
                let Some(timeline) = stage.library().timeline(clip.symbol) else {
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
                    play_from(stage, &path, previous);
                }
            }
            _ => {}
        }
    }
}
