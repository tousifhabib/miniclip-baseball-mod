//! Southpaw: a mod that has the batter bat left-handed.
//!
//! He stands on the other side of the plate, turned round, and nothing else
//! in the view moves: the stadium, the scoreboard, the little field and
//! whatever the mods draw are where they always are. The pitches are thrown
//! to him as they were to a right-hander. Where each is aimed is turned
//! over about the middle of the plate, and its curve with it, so that a
//! pitch that came in on a right-hander's hands comes in on his.

use bb_engine::library::Library;
use bb_engine::math::Matrix;
use bb_engine::stage::Stage;

use super::pitch::Choice;
use super::{Parts, frame_of};

/// The art has one way of running to first, which is to the right, as a
/// right-hander goes after turning round from his swing. A left-hander is
/// facing that way already, so when he sets off he is drawn as the
/// right-hander is this many frames into it, with the turn behind him, and
/// this much further across the view, which is where he stands.
const RUN_JOINS: u16 = 10;
const RUN_ACROSS: f32 = 210.0;

/// A turning over, left for right, about the line this far across.
fn over(line: f32) -> Matrix {
    Matrix {
        a: -1.0,
        tx: line * 2.0,
        ..Matrix::IDENTITY
    }
}

/// Stands the batter on the other side of the plate, in a view that has
/// just been built, with the aiming ring on the side away from him and the
/// box it is kept to as much to his side as it was to a right-hander's.
pub(crate) fn stand(parts: &mut Parts, stage: &mut Stage) {
    let centre = parts.centre_x;
    let Some(hitter) = stage.child_mut(&parts.hitter) else {
        return;
    };
    // The art builds the view afresh for a pitch, with him where it has
    // him. A view that has him turned round already is left alone.
    if hitter.matrix.a < 0.0 {
        return;
    }
    hitter.set_matrix(over(centre).then_inner(hitter.matrix));
    for ring in [&parts.aim, &parts.aim_shadow] {
        if let Some(ring) = stage.child_mut(ring) {
            let (x, y) = (ring.matrix.tx, ring.matrix.ty);
            ring.move_to(centre * 2.0 - x, y);
        }
    }
    let [left, top, right, bottom] = parts.aim_box;
    parts.aim_box = [centre * 2.0 - right, top, centre * 2.0 - left, bottom];
}

/// Turns a pitch over about the middle of the plate, `centre` across the
/// view: it is aimed as far to the other side, and curves the other way.
pub fn turn(choice: &mut Choice, centre: f32) {
    choice.aim.0 = centre * 2.0 - choice.aim.0;
    choice.swing = -choice.swing;
}

/// The batter has hit the ball and has just been set off for first, by
/// the art's way of running there. He goes to the right as anyone does,
/// from where he stands.
pub(crate) fn run(parts: &Parts, stage: &mut Stage, library: &Library) {
    let Some(hitter) = stage.child_mut(&parts.hitter) else {
        return;
    };
    let mut faced = over(parts.centre_x).then_inner(hitter.matrix);
    faced.tx += RUN_ACROSS;
    hitter.set_matrix(faced);
    let frame = frame_of(stage, &parts.hitter) + RUN_JOINS;
    stage.goto_clip(&parts.hitter, frame, library);
    if let Some(clip) = stage.clip_mut(&parts.hitter) {
        clip.playing = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pitch_turned_over_is_aimed_as_far_the_other_side_and_curves_the_other_way() {
        let mut choice = Choice {
            speed: 60.0,
            swing: 0.4,
            dip: 0.2,
            aim: (270.0, 240.0),
        };
        turn(&mut choice, 295.0);
        assert_eq!(choice.aim, (320.0, 240.0));
        assert_eq!((choice.swing, choice.dip, choice.speed), (-0.4, 0.2, 60.0));
        // Turned over again it is the pitch it was.
        turn(&mut choice, 295.0);
        assert_eq!((choice.aim, choice.swing), ((270.0, 240.0), 0.4));
    }
}
