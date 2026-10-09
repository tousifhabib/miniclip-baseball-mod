//! The overhead view: the ball in play, the fielder going after it, the
//! throws to the bases, and the runners.
//!
//! One frame of a play is here, and whose job each part of it is. What kind
//! of play it is and how it stands is in `play`. After that there is a file
//! for each thing that goes on in one: the change of view in `view`, the
//! ball while nobody has hold of it in `loose_ball`, the fielder going
//! after it in `chase`, the throws in `throw`, the runners in `running`,
//! the end of the play in `ending`, and what the mods are told of it in
//! `news`.

mod chase;
mod ending;
mod loose_ball;
mod news;
mod play;
mod running;
mod throw;
mod view;

pub(crate) use play::Fielding;
use play::Job;
pub(super) use running::ARRIVES;

use bb_engine::library::Library;
use bb_engine::stage::Stage;

use super::{AtBat, Match, Parts, at, show};
use crate::game::Game;

/// What a fielder does his job on: the parts of the view, the numbers the
/// game is played by, the stage and the art. They are handed on together
/// to whichever job is his this frame.
struct Scene<'a> {
    parts: &'a Parts,
    game: &'a Game,
    stage: &'a mut Stage,
    library: &'a Library,
}

/// The fielder behind the plate, counting from 0 as [`Fielding::fielder`]
/// does: the art's `fielder9`.
const CATCHER: usize = 8;
/// How far short of the wall an outfielder stops to watch a zinger go over
/// it, as the field measures distance.
const WARNING_TRACK: f32 = 40.0;

impl Match {
    /// One frame of the play in the field.
    pub(crate) fn field(
        &mut self,
        at_bat: &mut AtBat,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        let parts = at_bat.parts.clone();
        let Some(mut state) = at_bat.fielding.take() else {
            return;
        };
        state.frames += 1;
        self.move_the_loose_ball(at_bat, &mut state, &parts, game, stage, library);
        Match::watch_zinger(&mut state, &parts, game, stage, library);
        self.do_the_fielders_job(at_bat, &mut state, &parts, game, stage, library);

        let down = at_bat.ball.is_some_and(|ball| ball.bounced);
        self.move_runners(&state, down, &parts, stage, library);
        self.tell_steal(at_bat, game.rules.steal.told_time, stage, library);
        self.tell_sign(at_bat, game.rules.sign.told_time, stage, library);

        if self.the_play_is_over(&mut state, &parts, game, stage, library) {
            self.end_the_play(at_bat, &state, &parts, game, stage, library);
            // A runner still between bases was given his base as the play
            // was called dead. If he was stealing it, that is told now,
            // and not left to be told on the play after.
            self.tell_steal(at_bat, game.rules.steal.told_time, stage, library);
        }
        at_bat.fielding = Some(state);
    }

    /// Has the fielder whose ball it is do what he is doing for a frame:
    /// run for it, wait under it, pick it up, draw back, throw it, or go
    /// after it again when he has let it go.
    fn do_the_fielders_job(
        &mut self,
        at_bat: &mut AtBat,
        state: &mut Fielding,
        parts: &Parts,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        let fielder = parts.fielders[state.fielder].clone();
        let here = at(stage, &fielder);
        let scene = Scene {
            parts,
            game,
            stage,
            library,
        };
        match state.job {
            Job::Chase => self.go_after_the_ball(at_bat, state, &fielder, here, scene),
            Job::WaitCatch => self.wait_under_the_ball(at_bat, state, &fielder, here, scene),
            // He has it up off the ground, and draws back to throw.
            Job::PickUp { left: 0 } => {
                state.job = Match::wind_up(state, here, &fielder, parts, game, stage, library);
            }
            Job::PickUp { left } => state.job = Job::PickUp { left: left - 1 },
            Job::WindUp { left: 0 } => {
                state.job = Match::let_the_throw_go(&mut at_bat.ball, state, here, parts, game);
                show(stage, &parts.field_ball, true);
            }
            Job::WindUp { left } => state.job = Job::WindUp { left: left - 1 },
            Job::Throwing { step } => self.carry_the_throw(at_bat, state, step, scene),
            // He has his wits back, and goes after it.
            Job::Fumbling { left: 0 } => state.job = Job::Chase,
            Job::Fumbling { left } => state.job = Job::Fumbling { left: left - 1 },
            Job::Gather { left: 0 } => {
                show(stage, &parts.field_ball, false);
                stage.goto_label(&fielder, "baseWaiting", false, library);
                self.hold_or_throw_on(state, parts, game, stage, library);
            }
            Job::Gather { left } => state.job = Job::Gather { left: left - 1 },
            Job::Rest => {}
        }
    }
}
