//! The throws: from the fielder who has the ball to a base, and on from
//! there while anybody is still running.

use bb_engine::display::Path;
use bb_engine::stage::Stage;

use super::play::{Fair, Fielding, Job, Play};
use super::{CATCHER, Scene};
use crate::game::Game;
use crate::play::field::{Ball, Facing, distance};
use crate::play::overlay::Notices;
use crate::play::pitch::Point;
use crate::play::{AtBat, Match, Parts, at, put, show};

impl Match {
    /// The fielder's arm is back: the ball leaves his hand for the base he
    /// is throwing to, and is given how far it goes each frame.
    pub(super) fn let_the_throw_go(
        ball: &mut Option<Ball>,
        state: &Fielding,
        here: Point,
        parts: &Parts,
        game: &Game,
    ) -> Job {
        let rules = &game.rules;
        let to = parts.bases[usize::from(state.throw_to) - 1];
        let gap = distance(here, to).max(0.001);
        // A catcher's throw to a base being stolen has a speed of its own.
        let stealing = matches!(state.play, Play::Steal { .. });
        let speed = if stealing && state.fielder == CATCHER {
            rules.steal.throw_speed
        } else {
            rules.field.throw_speed
        };
        let step = ((to.0 - here.0) / gap * speed, (to.1 - here.1) / gap * speed);
        if let Some(ball) = ball {
            ball.at = here;
            ball.height = rules.field.catch_height;
        }
        Job::Throwing { step }
    }

    /// Moves a thrown ball on a frame towards its base, and has the fielder
    /// there take it when it is near enough.
    pub(super) fn carry_the_throw(
        &mut self,
        at_bat: &mut AtBat,
        state: &mut Fielding,
        step: Point,
        scene: Scene<'_>,
    ) {
        let Scene { parts, game, stage } = scene;
        let to = parts.bases[usize::from(state.throw_to) - 1];
        let Some(ball) = &mut at_bat.ball else {
            return;
        };
        ball.at = (ball.at.0 + step.0, ball.at.1 + step.1);
        let size = (0.6 + (ball.at.1 - parts.home.1) / 1000.0).max(0.1);
        put(stage, &parts.field_ball, ball.at, size);
        if let Some(inner) = stage.child_mut(&parts.field_ball_inner) {
            inner.move_to(inner.matrix.tx, -ball.height);
        }
        if distance(ball.at, to) < game.rules.field.throw_near {
            let told = &mut at_bat.notices;
            self.ball_at_base(state, told, parts, game, stage);
        }
    }

    /// Turns the fielder to face the base and starts his throw.
    pub(super) fn wind_up(
        state: &Fielding,
        here: Point,
        fielder: &Path,
        parts: &Parts,
        game: &Game,
        stage: &mut Stage,
    ) -> Job {
        let to = parts.bases[usize::from(state.throw_to) - 1];
        let label = Facing::towards(here, to).throw_label();
        stage.goto_label(fielder, label, false);
        Job::WindUp {
            left: game.rules.field.throw_time,
        }
    }

    /// A throw has reached its base.
    fn ball_at_base(
        &mut self,
        state: &mut Fielding,
        told: &mut Notices,
        parts: &Parts,
        game: &Game,
        stage: &mut Stage,
    ) {
        let base = state.throw_to;
        Match::sound(stage, "ballCatch_1");
        // The fielder minding that base has the ball now, or should have.
        state.fielder = 4 + usize::from(base);
        if self.lets_go() {
            // It is through his hands. Nobody is out, and he has it to
            // gather from the ground beside him.
            let fielder = parts.fielders[state.fielder].clone();
            let here = at(stage, &fielder);
            stage.goto_label(&fielder, Facing::Down.pick_label(), false);
            Match::sound(stage, "crowd_smallCheer");
            Match::tell(told, "DROPPED!", here, parts, game, stage);
            state.job = Job::Gather {
                left: game.rules.butterfingers.gather_time,
            };
            return;
        }
        show(stage, &parts.field_ball, false);
        let late: Vec<usize> = (0..self.runners.len())
            .filter(|&runner| self.runners[runner].running_to == Some(base))
            .collect();
        for runner in late {
            self.put_out(runner, stage);
            Match::sound(stage, "umpire_out_2");
            Match::sound(stage, "crowd_unhappy");
            if base <= 3 {
                let umpire = parts.umpires[usize::from(base) - 1].clone();
                self.play_section(&umpire, "out", 99, stage);
            }
        }
        self.show_numbers(stage);
        self.hold_or_throw_on(state, parts, game, stage);
    }

    /// The fielder at a base has the ball in his hands. He throws it on if
    /// anybody is still between bases, and the play is over if nobody is.
    pub(super) fn hold_or_throw_on(
        &mut self,
        state: &mut Fielding,
        parts: &Parts,
        game: &Game,
        stage: &mut Stage,
    ) {
        // With the pitcher fielding alone nobody throws the ball on: the
        // play ends where his throw does, and anyone still running is given
        // his base.
        let thrown_on = self.runners.anyone_running() && !self.mods.the_pitcher_fields_alone();
        if thrown_on {
            // Somebody is still between bases: on it goes.
            let fielder = parts.fielders[state.fielder].clone();
            let here = at(stage, &fielder);
            state.throw_to = self.pick_base(here, parts);
            state.job = Match::wind_up(state, here, &fielder, parts, game, stage);
        } else {
            state.play = match state.play {
                Play::Steal { .. } => Play::Steal { held: true },
                Play::Fair(Fair::Live) => Play::Fair(Fair::Held),
                // In no other play has a fielder at a base the ball.
                other => other,
            };
            state.job = Job::Rest;
        }
    }
}
