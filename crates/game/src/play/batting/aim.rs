//! Where the player is pointing: the ring over the plate, the pointer
//! that shows where a hit would go, and the timing bar's marker.

use bb_engine::library::Library;
use bb_engine::stage::Stage;

use crate::game::Game;
use crate::play::{AtBat, Match, Phase, at, frame_of, show};
use crate::rules::HitRules;

impl Match {
    /// Moves the timing bar's marker on, where the bar is up.
    pub(in crate::play) fn point_the_timing_bar(
        &self,
        at_bat: &mut AtBat,
        game: &Game,
        stage: &mut Stage,
    ) {
        let Some(bar) = &mut at_bat.timing else {
            return;
        };
        match self.phase {
            // A swing made now begins on the step the flight has come to.
            Phase::Flight { step } => bar.point(step as i32, stage),
            // Until the ball is thrown, the steps to go are the frames left
            // of the wind-up.
            Phase::WindUp => {
                let frame = frame_of(stage, &at_bat.parts.pitcher);
                let to_go = i32::from(game.rules.throw.release_frame) - i32::from(frame);
                bar.point(-to_go, stage);
            }
            _ => {}
        }
    }

    /// Moves the aiming ring a step towards the pointer, and with it the
    /// art's pointer to where a hit would go.
    pub(super) fn aim(at_bat: &mut AtBat, stage: &mut Stage) {
        let parts = &at_bat.parts;
        let pointer = (stage.pointer.x, stage.pointer.y);
        // A pointer nobody has moved yet is nowhere.
        if pointer.0 < -1.0e5 {
            return;
        }
        let Some(pointer) = stage.from_stage(&parts.main, pointer.0, pointer.1) else {
            return;
        };
        let [left, top, right, bottom] = parts.aim_box;
        let ease = at_bat.table.aim_ease.max(1.0);
        at_bat.aim.0 += (pointer.0 - at_bat.aim.0) / ease;
        at_bat.aim.1 += (pointer.1 - at_bat.aim.1) / ease;
        at_bat.aim.0 = at_bat.aim.0.clamp(left + 1.0, right - 1.0);
        at_bat.aim.1 = at_bat.aim.1.clamp(top + 1.0, bottom - 1.0);
        if let Some(ring) = stage.child_mut(&parts.aim) {
            ring.move_to(at_bat.aim.0, at_bat.aim.1);
        }
        let shadow_y = at(stage, &parts.aim_shadow).1;
        if let Some(shadow) = stage.child_mut(&parts.aim_shadow) {
            shadow.move_to(at_bat.aim.0, shadow_y);
        }
    }

    /// Works out where a hit made now would go sideways, and shows it.
    pub(super) fn point_hit(
        at_bat: &mut AtBat,
        rules: &HitRules,
        stage: &mut Stage,
        library: &Library,
    ) {
        if at_bat.contact.is_some() {
            return;
        }
        let parts = &at_bat.parts;
        let pull = rules.pull;
        // Unless the rules say otherwise, the pointer is kept out of sight
        // until there is a crossing point for it to answer to.
        show(
            stage,
            &parts.aim_area,
            at_bat.marker_shown || rules.pointer_before_pitch,
        );
        // Until the player has been shown where this pitch will cross, the
        // pointer answers the ring as if it were coming down the middle.
        // It answers to where the player has been shown the ball crossing,
        // which is not always quite where it will.
        let crosses = if at_bat.marker_shown {
            at_bat.marker_at.0
        } else {
            parts.centre_x
        };
        at_bat.aim_area_x = hit_towards(crosses, at_bat.aim.0, parts.centre_x, pull);
        let y = at(stage, &parts.aim_area).1;
        if let Some(area) = stage.child_mut(&parts.aim_area) {
            area.move_to(at_bat.aim_area_x, y);
        }
        // The pointer's look is drawn for every position, one a frame.
        let frame = at_bat.aim_area_x.clamp(1.0, 550.0) as u16;
        stage.goto_clip(&parts.aim_area, frame, library);
        if let Some(clip) = stage.clip_mut(&parts.aim_area) {
            clip.playing = false;
        }
    }
}

/// Where across the batting view the art's pointer shows a hit going, for a
/// ball that crosses at `crosses` with the ring held at `aim`. Aiming to
/// one side sends the ball the other way, and a ball that comes in
/// off-centre goes off further still.
pub(super) fn hit_towards(crosses: f32, aim: f32, centre: f32, pull: f32) -> f32 {
    let off = (crosses - aim) + (crosses - centre);
    (crosses + off * pull).ceil()
}
