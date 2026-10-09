//! Deciding the pitch: what kind it is, where it is thrown from and to,
//! and the points of the view it is drawn between.

use bb_engine::display::child_bounds;
use bb_engine::math::Matrix;
use bb_engine::stage::Stage;

use super::{Coming, Decided};
use crate::game::Game;
use crate::play::mods::southpaw;
use crate::play::pitch::{Choice, Mound, Pitch};
use crate::play::{Match, Parts, at};

impl Match {
    /// Decides the pitch: what kind it is, where it is aimed, how it flies.
    /// Nothing here touches the stage.
    pub(super) fn decide_the_pitch(
        &mut self,
        coming: &mut Coming,
        mound: &Mound,
        game: &Game,
    ) -> Decided {
        let rules = &game.rules;
        let table = &mut coming.table;
        // A pitcher waits longer before a slower pitch. A mystery pitch
        // would be no mystery if he did, so before one he waits as long as
        // for a pitch of the usual pace, and the marker is not shown until
        // the ball has left his hand.
        let usual_pace = table.speed.high as f32;
        let release = rules.throw.release_frame;
        let kind = self
            .mods
            .mystery_pitch
            .as_ref()
            .map(|mystery| mystery.pick(table, release, &mut self.rng));
        let mut choice = Choice::pick(table, &rules.throw, &mut self.rng);
        if self.mods.southpaw.is_some() {
            // A left-hander is pitched to as a right-hander was.
            southpaw::turn(&mut choice, coming.parts.centre_x);
        }
        let mut pitch = Pitch::throw(&choice, mound, &rules.throw);
        let wait = match kind {
            Some(_) => {
                let usual = Choice {
                    speed: usual_pace,
                    ..choice
                };
                Pitch::throw(&usual, mound, &rules.throw).samples.len()
            }
            None => pitch.samples.len(),
        };
        // The marker shows where the pitch was going before a knuckleball
        // began to sway.
        let marker_at = pitch.crosses;
        if let Some(knuckleball) = &self.mods.knuckleball {
            let start = self.rng.unit();
            knuckleball.sway(&mut pitch, start, mound);
        }
        Decided {
            kind,
            pitch,
            marker_at,
            wait,
        }
    }

    /// The fixed points a pitch is drawn between.
    pub(super) fn mound(parts: &Parts, stage: &Stage) -> Option<Mound> {
        let test = stage.find(&parts.main, &["test"])?;
        let point = |name: &str| stage.find(&test, &[name]).map(|path| at(stage, &path));
        // With no strike zone to miss, as in the arcade game, no pitch is
        // ever outside it.
        let zone = stage
            .find(&parts.main, &["strikeZone"])
            .and_then(|path| stage.child(&path))
            .and_then(|child| child_bounds(child, Matrix::IDENTITY, stage.library()))
            .unwrap_or([f32::MIN, f32::MIN, f32::MAX, f32::MAX]);
        Some(Mound {
            ball: point("ballAll")?,
            shadow: point("ballShadow")?,
            ball_from: point("startpointMarker")?,
            shadow_from: point("startpointShadowMarker")?,
            plate: point("shadowMarker")?.1,
            zone,
        })
    }
}
