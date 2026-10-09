//! A pitch that was not hit: what the umpire calls it and what the call
//! comes to.

use bb_engine::library::Library;
use bb_engine::stage::Stage;

use crate::game::Game;
use crate::play::book::{End, Thrown};
use crate::play::{AtBat, Match, Parts, Phase, Place, show, steal};

impl Match {
    /// The ball has gone by: a strike, or a ball.
    pub(super) fn call(
        &mut self,
        at_bat: &mut AtBat,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        let rules = &game.rules;
        let parts = at_bat.parts.clone();
        show(stage, &parts.ball, false);
        show(stage, &parts.shadow, false);
        if self.mode.is_arcade() {
            // No count in the arcade game: a miss is just a pitch gone. It
            // is a miss all the same to the mods that mind one, and a bat
            // that was hot goes cold on it.
            self.mods.a_strike_was_called();
            return self.ready(&parts, stage, library);
        }
        Match::sound(stage, library, "ballCatch_1");
        if !at_bat.pitch.in_zone && at_bat.swing.is_none() {
            self.book_pitch(at_bat, Thrown::Ball);
            self.count.balls += 1;
            if let Some(board) = &parts.scoreboard {
                self.play_section(board, "noBall", 261, stage, library);
            }
            self.show_numbers(stage);
            if self.count.is_a_walk(rules.count.balls) {
                self.phase = Phase::Walking {
                    left: rules.hit.walk_wait,
                };
            } else if self.runners.anyone_stealing() {
                // The catcher has the ball, and a runner to throw out.
                self.show_steal(at_bat, game, stage, library);
            } else {
                self.ready(&parts, stage, library);
            }
            return;
        }
        let thrown = match at_bat.swing {
            Some(_) => Thrown::Swinging,
            None => Thrown::Called,
        };
        self.book_pitch(at_bat, thrown);
        let allowed = self.strikes_allowed(game);
        self.count.strike(at_bat.golden, allowed);
        self.mods.a_strike_was_called();
        if let Some(anim) = &parts.strike_anim {
            let label = format!("strike{}", self.count.strikes.min(3));
            stage.goto_label(anim, &label, false, library);
            // The badge plays for 69 frames and is then taken down. Left
            // up, it would play again and again over the scoreboard.
            self.put_away.push((anim.clone(), 68));
        }
        if let Some(board) = &parts.scoreboard {
            self.play_section(board, "strike", 136, stage, library);
        }
        if self.count.is_out(allowed) {
            let call = ["1", "2", "3"][self.rng.below(3) as usize];
            Match::sound(stage, library, &format!("umpire_yourOuttaHere_{call}"));
            Match::sound(stage, library, "crowd_unhappy");
            if let Some(batter) = self.runners.batter() {
                self.runners[batter].place = Place::Out;
            }
            self.outs += 1;
            self.mods.somebody_is_out();
            self.clear_count();
            self.announce = true;
            self.book_end(End::Strikeout, None);
        } else {
            Match::sound(stage, library, "umpire_Strike_grunt");
            if self.count.is_one_strike_from_out(allowed) {
                let organ = ["baseball_organ_FX", "baseball_organ_tense_FX"];
                Match::sound(stage, library, organ[self.rng.below(2) as usize]);
            }
        }
        self.show_numbers(stage);
        if self.runners.anyone_stealing() && self.outs < self.max_outs {
            return self.show_steal(at_bat, game, stage, library);
        }
        // With the side out, nobody has anywhere to steal to.
        steal::send_back(&mut self.runners, stage, library);
        // The call is left up for a moment before the next pitch is offered.
        self.phase = Phase::Called { left: 58 };
    }

    /// The play is over: offers the next pitch.
    pub(crate) fn ready(&mut self, parts: &Parts, stage: &mut Stage, library: &Library) {
        if self.phase == Phase::Ready {
            return;
        }
        self.phase = Phase::Ready;
        stage.goto_clip(&parts.next, 2, library);
        self.show_numbers(stage);
    }
}
