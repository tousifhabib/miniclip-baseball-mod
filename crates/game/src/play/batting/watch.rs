//! A hit, watched from behind the batter until the view changes to the
//! field.

use bb_engine::library::Library;
use bb_engine::stage::Stage;

use crate::game::Game;
use crate::play::field::Happened;
use crate::play::{AtBat, Cue, Match, at, put, southpaw};

impl Match {
    /// One frame of the ball leaving the bat, seen from behind the batter.
    pub(in crate::play) fn watch(
        &mut self,
        at_bat: &mut AtBat,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        let rules = &game.rules;
        let Some(contact) = at_bat.contact else {
            return;
        };
        let parts = &at_bat.parts;
        let (at_point, height, lift) = &mut at_bat.fly;
        at_point.0 -= (at_point.0 - at_bat.fly_target.0) / contact.power;
        at_point.1 -= (at_point.1 - at_bat.fly_target.1) / contact.power;
        *height += *lift;
        if *lift >= -5.0 {
            *lift -= rules.hit.gravity;
        }
        if *height < 0.0 {
            *height = 0.0;
            *lift = -*lift * rules.hit.bounce;
        }
        let size = (1.0 + (at_point.1 - parts.ground_y) / 190.0).max(0.05);
        put(stage, &parts.fly, *at_point, size);
        let across = at(stage, &parts.fly_ball).0;
        put(stage, &parts.fly_ball, (across, -*height), at_bat.fly_size);
        if let Some(left) = at_bat.run_in {
            if left == 0 {
                at_bat.run_in = None;
                stage.goto_label(&parts.hitter, "run", true, library);
                if self.mods.southpaw.is_some() {
                    southpaw::run(parts, stage, library);
                }
                let last = stage
                    .clip(&parts.hitter)
                    .map_or(1, |clip| clip.frame_count(library));
                self.cues.push(Cue {
                    path: parts.hitter.clone(),
                    frame: last,
                    rewind: false,
                });
            } else {
                at_bat.run_in = Some(left - 1);
            }
        }
        // The ball is already on its way over the field, out of sight.
        let ground = parts.ground(&rules.field);
        let mut at_wall = None;
        if let Some(ball) = &mut at_bat.ball {
            let happened = ball.step(parts.home, contact.miss(), &rules.field);
            if happened == Happened::Cleared {
                at_bat.over_wall = true;
            }
            if matches!(happened, Happened::Cleared | Happened::HitWall) {
                at_wall = Some((ground.across(ball.at), ball.height));
            }
        }
        if let Some((across, height)) = at_wall {
            self.strike_sign(at_bat, across, height, &rules.sign);
        }
    }
}
