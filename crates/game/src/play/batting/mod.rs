//! At the plate: the frames of a pitch, from the pitcher standing and
//! waiting to the bat meeting the ball or the umpire's call.
//!
//! What lasts from pitch to pitch is the match's. What happens on each
//! frame of one pitch is here, a function for each part of it: the wait, the
//! wind-up, the ball's flight, the call, and watching a hit go.

mod aim;
mod call;
mod swing;
mod watch;
mod wind_up;

use bb_engine::display::Content;
use bb_engine::library::Library;
use bb_engine::stage::Stage;

use super::steal;
use super::{AtBat, Match, Phase, play_from};
use crate::art;
use crate::game::Game;

impl Match {
    /// Keeps the batting view as it should be for this frame: the pointer
    /// out of sight while the ring stands for it, the batter still once he
    /// has swung, and whatever the mods have drawn kept up.
    pub(super) fn keep_the_view(
        &mut self,
        at_bat: &mut AtBat,
        slowed: bool,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        let rules = &game.rules;
        // While the ring is being aimed it stands for the pointer, which
        // would only get in its way.
        let aiming = at_bat.contact.is_none()
            && matches!(
                self.phase,
                Phase::Settling { .. } | Phase::WindUp | Phase::Flight { .. }
            );
        stage.hide_pointer = aiming
            && stage
                .from_stage(&at_bat.parts.main, stage.pointer.x, stage.pointer.y)
                .is_some_and(|(x, y)| {
                    let [left, top, right, bottom] = at_bat.parts.aim_box;
                    (left..=right).contains(&x) && (top..=bottom).contains(&y)
                });
        Match::still_batter(stage, &at_bat.parts.hitter, library);
        if let Some(glow) = self.mods.glow_of_the_bat() {
            // The mark on the bat glows, hotter the longer the run of hits.
            for mark in art::all_named(stage, &at_bat.parts.hitter, "batLogo") {
                if let Some(mark) = stage.child_mut(&mark) {
                    mark.set_color(glow);
                }
            }
        }
        Match::settle_fielders(&at_bat.parts, stage, library);
        for them in &at_bat.them {
            them.keep(stage);
        }
        at_bat.notices.fade(stage);
        if let Some(leads) = &mut at_bat.leads {
            leads.keep(&self.runners, self.phase == Phase::WindUp, stage);
            steal::hold(&self.runners, stage);
        }
        if let Some(signs) = &mut at_bat.signs {
            signs.keep(stage);
        }
        if let (Some(meter), Some(left)) = (&at_bat.meter, self.mods.meter_left()) {
            meter.keep(left, slowed, stage);
        }
        if at_bat.contact.is_none() {
            Match::aim(at_bat, stage);
            Match::point_hit(at_bat, &rules.hit, stage, library);
        }
    }

    /// The next pitch has been asked for and the last has been cleared
    /// away: the art builds the view again, which starts the next pitch.
    pub(super) fn ask_for_a_new_view(
        &mut self,
        at_bat: &AtBat,
        stage: &mut Stage,
        library: &Library,
    ) {
        self.zinger_unseen();
        let mut holder = at_bat.parts.main.clone();
        holder.pop();
        play_from(stage, &holder, 1, library);
        self.phase = Phase::Arriving;
    }
}

impl Match {
    /// Stills the batter once his swing is done.
    ///
    /// The swing is a clip that stops on its last frame, with his skin,
    /// shirt and helmet as clips of their own inside it, moving in step.
    /// The art stopped those from a script. Left alone they go round again
    /// over a body that has stopped, and he swings on for ever.
    fn still_batter(stage: &mut Stage, hitter: &[u16], library: &Library) {
        let Some(clip) = stage.clip(hitter) else {
            return;
        };
        let mut moving = Vec::new();
        for (&depth, child) in &clip.children {
            let Content::Clip(swing) = &child.content else {
                continue;
            };
            let last = swing.frame_count(library);
            if swing.playing || last <= 1 || swing.frame != last {
                continue;
            }
            for (&inner_depth, inner) in &swing.children {
                if let Content::Clip(part) = &inner.content
                    && part.playing
                    && part.frame_count(library) > 1
                {
                    let mut path = hitter.to_vec();
                    path.extend([depth, inner_depth]);
                    moving.push((path, part.frame_count(library)));
                }
            }
        }
        for (path, last) in moving {
            stage.goto_clip(&path, last, library);
            if let Some(part) = stage.clip_mut(&path) {
                part.playing = false;
            }
        }
    }
}
