//! Before the ball is thrown: the pitcher stands, a runner may be sent,
//! and the wind-up ends in the throw.

use bb_engine::stage::Stage;

use crate::game::Game;
use crate::play::pitch::Point;
use crate::play::{AtBat, MYSTERY_TOP, Match, Phase, called, frame_of, put, show, steal};

/// The pitcher's frame label for his wind-up.
const PITCH: &str = "pitch";

impl Match {
    /// The pitcher stands and waits, with `left` frames to go before he
    /// winds up. While he does, a click on the outfield calls the shot.
    pub(in crate::play) fn wait_for_the_wind_up(
        &mut self,
        at_bat: &mut AtBat,
        left: u32,
        pressed: Option<Point>,
        game: &Game,
        stage: &mut Stage,
    ) {
        if let Some((x, y)) = pressed
            && self.mods.shots_are_called()
            && let Some(pointer) = stage.from_stage(&at_bat.parts.main, x, y)
        {
            let called = &mut at_bat.called;
            called::Called::call(called, pointer, &at_bat.parts, &game.rules, stage);
        }
        if left == 0 {
            stage.goto_label(&at_bat.parts.pitcher, PITCH, true);
            self.phase = Phase::WindUp;
            // If a runner may be sent to steal, the corner of the view
            // says so.
            if let Some(leads) = &at_bat.leads
                && leads.anyone_may_go(&self.runners)
            {
                let (notices, parts) = (&mut at_bat.notices, &at_bat.parts);
                notices.put(steal::asks(leads.hint_at()), parts, stage);
            }
        } else {
            self.phase = Phase::Settling { left: left - 1 };
        }
    }

    /// A click during the wind-up, at `pointer` in the batting view: with
    /// the stolen bases mod on, one on the little field sends a runner for
    /// the next base.
    fn send_a_stealer(&mut self, at_bat: &mut AtBat, pointer: Point, stage: &mut Stage) {
        let Some(leads) = &at_bat.leads else {
            return;
        };
        let Some(sent) = leads.sent_by(pointer, &self.runners) else {
            return;
        };
        self.runners[sent.runner].stole_from = Some(sent.from);
        self.send(sent.runner, sent.to, stage);
        let (notices, parts) = (&mut at_bat.notices, &at_bat.parts);
        let says = steal::says_one_is_going(leads.hint_at());
        notices.put(says, parts, stage);
    }

    /// The pitcher winds up, shows where the pitch is going, and lets the
    /// ball go. While he winds up, a click on the little field sends a
    /// runner.
    pub(in crate::play) fn wind_up_and_throw(
        &mut self,
        at_bat: &mut AtBat,
        pressed: Option<Point>,
        game: &Game,
        stage: &mut Stage,
    ) {
        if let Some((x, y)) = pressed
            && let Some(pointer) = stage.from_stage(&at_bat.parts.main, x, y)
        {
            self.send_a_stealer(at_bat, pointer, stage);
        }
        let frame = frame_of(stage, &at_bat.parts.pitcher);
        if !at_bat.marker_shown && frame >= at_bat.table.marker_frame {
            at_bat.marker_shown = true;
            put(stage, &at_bat.parts.marker, at_bat.marker_at, 1.0);
        }
        if frame < game.rules.throw.release_frame {
            return;
        }
        show(stage, &at_bat.parts.ball, true);
        show(stage, &at_bat.parts.shadow, true);
        self.pitched += 1;
        self.mods.the_ball_was_thrown();
        // Nobody can be sent to steal now.
        if !self.runners.anyone_stealing() {
            at_bat.notices.take_down(steal::HINT, stage);
        }
        self.book_thrown();
        if let Some(arcade) = self.mode.arcade_mut() {
            arcade.left = arcade.left.saturating_sub(1);
        }
        self.show_numbers(stage);
        if let (Some(kind), Some(mystery)) = (at_bat.kind, &self.mods.mystery_pitch) {
            // Now it can be told what he threw.
            let top = (at_bat.parts.centre_x, MYSTERY_TOP);
            at_bat
                .notices
                .put(mystery.news(kind).at(top), &at_bat.parts, stage);
        }
        self.phase = Phase::Flight { step: 0 };
    }
}
