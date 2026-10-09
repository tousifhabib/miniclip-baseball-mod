//! A game in progress: a frame of it, the end of it, and what comes
//! after.

use bb_engine::stage::Stage;

use super::Baseball;
use super::screen::{Screen, screen_after};
use crate::art;
use crate::play::Outcome;
use crate::settings::Difficulty;

impl Baseball {
    /// The board between innings has been read: back to the batting.
    pub(super) fn bat_again(&mut self, stage: &mut Stage) {
        let Some(play) = &mut self.play else {
            return;
        };
        play.bat_again();
        self.switch(Screen::FullMatch, stage);
    }

    /// Stops each clip that was left playing to a frame, once it is there.
    pub(super) fn stop_held_clips(&mut self, stage: &mut Stage) {
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
    pub(super) fn play_a_frame(&mut self, stage: &mut Stage) {
        // A game that has stopped for the board between innings waits.
        let outcome = match (&mut self.play, self.screen.is_game()) {
            (Some(play), true) => play.tick(&self.playing, stage),
            _ => None,
        };
        self.keep_zinger_record();
        match outcome {
            None => {}
            Some(Outcome::Interval) => {
                self.switch(Screen::Interval, stage);
            }
            Some(outcome) => self.end_the_game(outcome, stage),
        }
    }

    /// The game is over: its result is shown on the screen for how it
    /// ended, and an arcade score goes in the table.
    fn end_the_game(&mut self, outcome: Outcome, stage: &mut Stage) {
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
        self.show(screen_after(outcome), stage);
        self.last_zinger = longest;
        self.finished = finished;
    }

    /// The arcade game's finish screen names the skill level played, on a
    /// clip with a frame for each. It appears part of the way through the
    /// screen's arrival, so it is set whenever it is there.
    pub(super) fn name_the_skill_played(&self, stage: &mut Stage) {
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
            stage.goto_clip(&path, frame);
            if let Some(clip) = stage.clip_mut(&path) {
                clip.playing = false;
            }
        }
    }

    /// A pointer hidden for aiming comes back for the quit prompt, and
    /// whenever no game is being played.
    pub(super) fn give_the_pointer_back(&self, stage: &mut Stage) {
        let prompt_up = stage
            .find_symbol(&[], art::QUIT_PROMPT)
            .and_then(|path| stage.clip(&path))
            .is_some_and(|prompt| prompt.frame > 1);
        if self.play.is_none() || prompt_up || !self.screen.is_game() {
            stage.hide_pointer = false;
        }
    }

    /// Goes from a screen that leads to another by itself, once it is time.
    pub(super) fn move_on(&mut self, stage: &mut Stage) {
        match self.screen {
            Screen::Loading => {
                if art::shell(stage).is_some() {
                    match self.first.take() {
                        Some(first) => self.show(first, stage),
                        None => self.screen = Screen::Intro,
                    }
                }
            }
            // The intro leads into the menu when it has played out.
            Screen::Intro => {
                let finished = art::in_shell(stage, art::INTRO)
                    .and_then(|path| stage.clip(&path))
                    .is_some_and(|intro| intro.frame >= intro.frame_count(stage.library()));
                if finished {
                    self.show(Screen::Menu, stage);
                }
            }
            Screen::Menu => {
                if let Some(leave) = self.menu.tick(&self.game, stage) {
                    self.leave_menu(leave, stage);
                }
            }
            _ => {}
        }
    }
}
