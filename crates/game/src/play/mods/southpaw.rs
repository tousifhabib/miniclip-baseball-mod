//! Southpaw: a mod that has the batter bat left-handed.
//!
//! He stands on the other side of the plate, turned round, and nothing else
//! in the view moves: the stadium, the scoreboard, the little field and
//! whatever the mods draw are where they always are. The pitches are thrown
//! to him as they were to a right-hander. Where each is aimed is turned
//! over about the middle of the plate, and its curve with it, so that a
//! pitch that came in on a right-hander's hands comes in on his.

use bb_engine::math::Matrix;
use bb_engine::stage::Stage;

use crate::mods::About;
use crate::play::pitch::Choice;
use crate::play::{Parts, frame_of, play_from};

/// What the menu and the files know this mod by.
pub(crate) const ABOUT: About = About {
    key: "southpaw",
    name: "SOUTHPAW",
    does: "BAT LEFT-HANDED, FROM THE OTHER SIDE OF THE PLATE",
    setting: None,
};

/// The mod, in play: that the batter bats left-handed.
#[derive(Default)]
pub(crate) struct Southpaw {
    /// Whether he has taken his stand yet. He does so when the first view
    /// is got ready, and nothing is said of him before.
    stood: bool,
}

impl Southpaw {
    pub fn has_stood(&self) -> bool {
        self.stood
    }

    /// Stands the batter on the other side of the plate, turned round.
    pub fn stand(&mut self, parts: &mut Parts, stage: &mut Stage) {
        self.stood = true;
        stand(parts, stage);
    }
}

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
fn stand(parts: &mut Parts, stage: &mut Stage) {
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
pub(crate) fn run(parts: &Parts, stage: &mut Stage) {
    let Some(hitter) = stage.child_mut(&parts.hitter) else {
        return;
    };
    let mut faced = over(parts.centre_x).then_inner(hitter.matrix);
    faced.tx += RUN_ACROSS;
    hitter.set_matrix(faced);
    let frame = frame_of(stage, &parts.hitter) + RUN_JOINS;
    play_from(stage, &parts.hitter, frame);
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::play::pitch::properties::any_choice;

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

    proptest! {
        #[test]
        fn any_pitch_turned_over_twice_is_the_pitch_it_was(
            choice in any_choice(),
            centre in 200.0f32..400.0,
        ) {
            let mut turned = choice;
            turn(&mut turned, centre);
            // Once over, it is aimed as far to the other side of the
            // middle, and curves the other way.
            let (was, now) = (choice.aim.0 - centre, turned.aim.0 - centre);
            prop_assert!((was + now).abs() < 0.001, "{} off, and then {}", was, now);
            prop_assert_eq!(turned.swing, -choice.swing);
            turn(&mut turned, centre);
            // How far across it is aimed has been worked out twice by now,
            // and so is only within a hair of what it was. Nothing else of
            // it has been touched.
            let across = turned.aim.0;
            prop_assert!((across - choice.aim.0).abs() < 0.001, "aimed {} across", across);
            turned.aim.0 = choice.aim.0;
            prop_assert_eq!(turned, choice);
        }

        #[test]
        fn a_pitch_aimed_at_the_middle_of_the_plate_is_aimed_there_still_when_turned_over(
            choice in any_choice(),
        ) {
            let mut turned = choice;
            turn(&mut turned, choice.aim.0);
            prop_assert_eq!(turned.aim, choice.aim);
        }
    }
}
