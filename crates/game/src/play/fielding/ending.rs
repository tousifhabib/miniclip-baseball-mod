//! The end of a play: when it is over, what it comes to, and how it goes
//! in the book.

use bb_engine::stage::Stage;

use super::play::{Fair, Fielding, Job, Play};
use crate::game::Game;
use crate::play::book::{End, Hit, Thrown};
use crate::play::field::reach;
use crate::play::{AtBat, Match, Parts, Place, show};

/// How many frames the picture of a foul plays for before the next pitch,
/// and the picture of a home run, and how far into the home run's the
/// board in the field joins in.
const FOUL_PLAYS_FOR: u32 = 91;
const HOME_RUN_PLAYS_FOR: u32 = 116;
const BOARD_JOINS_IN: u32 = 60;

impl Match {
    /// Whether the play has come to its end: a foul or a home run once its
    /// picture has played out, a walk once everyone has walked, and any
    /// other once the ball is dead or has been in play too long.
    pub(super) fn the_play_is_over(
        &self,
        state: &mut Fielding,
        parts: &Parts,
        game: &Game,
        stage: &mut Stage,
    ) -> bool {
        match &mut state.play {
            Play::Walk => !self.runners.anyone_running(),
            // The foul's picture plays itself out first.
            Play::Foul { called } => {
                *called += 1;
                *called >= FOUL_PLAYS_FOR
            }
            // And the home run's, which the board joins in part of the way
            // through.
            Play::Fair(Fair::HomeRun { called }) => {
                *called += 1;
                if *called == BOARD_JOINS_IN
                    && let Some(board) = &parts.field_scoreboard
                {
                    stage.goto_label(board, "homeRun", true);
                }
                *called >= HOME_RUN_PLAYS_FOR
            }
            Play::Steal { held: true } | Play::Fair(Fair::Held) => true,
            // A ball still in play is called dead if it goes on too long.
            Play::Steal { held: false } | Play::Fair(Fair::Live | Fair::Gone) => {
                state.frames > game.rules.field.longest
            }
        }
    }

    /// Calls the play dead: runners between bases are given the base they
    /// were making for, the mods are told how it went, it goes in the book,
    /// and the next pitch is put on offer.
    pub(super) fn end_the_play(
        &mut self,
        at_bat: &mut AtBat,
        state: &Fielding,
        parts: &Parts,
        game: &Game,
        stage: &mut Stage,
    ) {
        let rules = &game.rules.field;
        // Anyone still between bases when a play is called dead is given
        // the base he was making for.
        for runner in 0..self.runners.len() {
            if self.runners[runner].running_to.is_some() {
                self.arrive(runner, parts, stage);
            }
        }
        if state.is_fair() {
            // Where it went is remembered, for the shift to go by.
            let ground = parts.ground(rules);
            self.mods.a_fair_ball_came_down(ground.across(state.land));
        }
        // A hit puts some of bullet time's meter back, and a home run
        // all of it.
        let batter = state.batter.and_then(|batter| self.runners.get(batter));
        let hit = state.is_fair();
        match batter.map(|batter| batter.place) {
            Some(Place::Home) if hit => self.mods.a_hit_came_off(true),
            Some(Place::Base(_)) if hit => self.mods.a_hit_came_off(false),
            _ => {}
        }
        self.book_play(at_bat, state);
        self.ready(parts, stage);
    }

    /// In a full match, writes a play that is over into the book: a foul,
    /// a walk, or what came of a ball that was put in play.
    fn book_play(&mut self, at_bat: &AtBat, state: &Fielding) {
        let Some(ground) = self.mode.full().map(|full| *full.ground()) else {
            return;
        };
        match state.play {
            // The pitch is in the book already: nobody hit it.
            Play::Steal { .. } => return,
            Play::Foul { .. } => return self.book_pitch(at_bat, Thrown::Foul),
            Play::Walk => return self.book_end(End::Walk, None),
            Play::Fair(_) => self.book_pitch(at_bat, Thrown::InPlay),
        }
        let (score, outs) = self.thrown_at;
        let place = state
            .batter
            .and_then(|batter| self.runners.get(batter))
            .map(|runner| runner.place);
        let safe = matches!(place, Some(Place::Base(_) | Place::Home));
        let end = match place {
            // He would have been out, had the catch been held.
            _ if safe && state.was_dropped() => End::Error,
            Some(Place::Home) => End::HomeRun,
            Some(Place::Base(2)) => End::Double,
            Some(Place::Base(3)) => End::Triple,
            Some(Place::Base(_)) => End::Single,
            // A run that came in on a catch that was not the last out.
            _ if state.was_caught() && self.score > score && outs < 2 => End::SacrificeFly,
            _ if state.was_caught() => End::FlyOut,
            _ if self.outs >= outs + 2 => End::DoublePlay,
            _ => End::GroundOut,
        };
        // It was in the air if it was caught, went out of the park, or
        // first came down beyond the infield.
        let deep = reach(ground.home, state.land) >= ground.infield;
        let fly = state.catch.is_some() || state.is_home_run() || deep;
        let feet = at_bat.zinger.map(|zinger| zinger.feet);
        let hit = Hit::at(&ground, state.land, fly, feet);
        if end == End::Error
            && let Some(full) = self.mode.full_mut()
        {
            full.book.theirs.errors += 1;
        }
        self.book_end(end, Some(hit));
    }

    /// The ball has cleared the wall: everybody scores.
    pub(super) fn home_run(&mut self, state: &mut Fielding, parts: &Parts, stage: &mut Stage) {
        state.play = Play::Fair(Fair::HomeRun { called: 0 });
        state.job = Job::Rest;
        self.mods.a_home_run_was_hit();
        let worth = self.run_worth;
        // The batter has reached every base there is.
        if self.runners.batter().is_some() {
            self.mods.the_batter_reached_base();
        }
        for runner in &mut self.runners {
            if matches!(runner.place, Place::AtBat | Place::Base(_)) {
                runner.place = Place::Home;
                runner.running_to = None;
                runner.runs += worth;
                self.score += worth;
                if let Some(path) = &runner.path {
                    stage.goto_label(path, "empty", false);
                }
            }
        }
        self.clear_count();
        self.announce = true;
        let fielder = parts.fielders[state.fielder].clone();
        stage.goto_label(&fielder, "waiting", false);
        show(stage, &parts.field_ball, false);
        let transitions = parts.transitions.clone();
        self.play_section(&transitions, "homeRun", 117, stage);
        self.show_numbers(stage);
    }
}
