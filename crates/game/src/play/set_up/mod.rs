//! Getting the batting view ready for a pitch.
//!
//! The art builds the view afresh for every pitch, so before each one
//! everything is put back where the last left it and the pitch is decided.
//! It is done a step at a time, in an order that matters: the steps take
//! the lines in the corner of the view from the top down, numbers by chance
//! are drawn in the order the steps come, and one step's sums are done on
//! what the step before left.
//!
//! The order is here. The steps are in `side`, for the side at bat, in
//! `mods`, for what each mod does and puts up, and in `pitch`, for the
//! deciding of the pitch.

mod mods;
mod pitch;
mod side;

use bb_engine::library::Library;
use bb_engine::stage::Stage;

use super::mods::Line;
use super::overlay::{Corner, Notices, Says};
use super::pitch::{Kind, Pitch, Point};
use super::{AtBat, Match, Outcome, Parts, Phase, at, full};
use crate::game::Game;
use crate::rules::PitchRules;

/// What is being got ready for the pitch that is coming, as each step adds
/// what it has to add.
struct Coming {
    parts: Parts,
    /// The numbers the pitch is picked by, which the mods change.
    table: PitchRules,
    notices: Notices,
    /// Where the next line in the corner of the view goes.
    corner: Corner,
    /// Whether the pitch is a golden ball.
    golden: bool,
}

impl Coming {
    /// Writes a mod's line in the corner of the view, under the lines
    /// already there.
    fn write(&mut self, line: &Line, stage: &mut Stage, library: &Library) {
        let top = self.corner.line();
        self.notices.put(
            Says::line(line.name, &line.words, line.colour).at(top),
            &self.parts,
            stage,
            library,
        );
    }
}

/// The pitch that has been decided on.
struct Decided {
    /// What a mystery pitch turned out to be.
    kind: Option<Kind>,
    pitch: Pitch,
    /// Where the marker of where it will cross is shown.
    marker_at: Point,
    /// How many frames the pitcher waits for it, on top of what he always
    /// waits.
    wait: usize,
}

impl Match {
    /// Gets a freshly built batting view ready for a pitch. Returns how the
    /// match ended if it has.
    pub(super) fn set_up(
        &mut self,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<Outcome> {
        let mut parts = Parts::find(stage, library)?;
        self.cues.clear();
        self.put_away.clear();
        self.stand_the_batter(&mut parts, stage);
        if let Some(outcome) = self.outcome() {
            self.phase = Phase::Over;
            return Some(self.close_half(outcome));
        }
        self.bring_up_a_batter(game);

        let mut coming = Coming {
            parts,
            table: game.rules.pitch.at(game.settings.difficulty).clone(),
            notices: Notices::default(),
            corner: Corner::default(),
            golden: false,
        };
        self.say_the_innings(&mut coming, stage, library);
        self.gild_the_ball(&mut coming, stage);
        self.heat_the_pitch(&mut coming, stage, library);
        self.tire_the_arm(&mut coming, stage, library);
        self.widen_for_a_hot_bat(&mut coming, stage, library);
        Match::say_the_ball_is_golden(&mut coming, stage, library);
        self.say_it_is_the_clutch(&mut coming, stage, library);
        self.say_what_a_rally_is_worth(&mut coming, stage, library);
        let meter = self.put_up_the_meter(&mut coming, stage, library);
        self.shift_the_fielders(&mut coming, game, stage, library);
        Match::clear_the_plate(&coming, stage);
        self.stand_the_runners(&coming, stage, library);
        let leads = self.mark_the_leads(&mut coming, stage, library);
        let signs = self.put_up_the_signs(&coming, game, stage, library);
        self.mark_the_field(&coming, stage, library);
        let them = match self.mode.full() {
            Some(_) => full::Them::put(&coming.parts, stage, library),
            None => Vec::new(),
        };
        self.set_up_arcade(&coming.parts, game, stage, library);
        self.show_numbers(stage);

        let mound = Match::mound(&coming.parts, stage, library)?;
        let decided = self.decide_the_pitch(&mut coming, &mound, game);
        let timing = self.put_up_the_timing_bar(&coming, &decided.pitch, game, stage, library);
        self.phase = Phase::Settling {
            left: game.rules.throw.settle + decided.wait as u32,
        };
        let Coming {
            parts,
            table,
            notices,
            golden,
            ..
        } = coming;
        self.at = Some(AtBat {
            aim: at(stage, &parts.aim),
            aim_area_x: at(stage, &parts.aim_area).0,
            parts,
            table,
            pitch: decided.pitch,
            marker_shown: false,
            marker_at: decided.marker_at,
            kind: decided.kind,
            golden,
            called: None,
            swing: None,
            under: 0.0,
            across: 0.0,
            contact: None,
            fly: ((0.0, 0.0), 0.0, 0.0),
            fly_size: 1.0,
            fly_target: (0.0, 0.0),
            run_in: None,
            ball: None,
            fielding: None,
            timing,
            zinger: None,
            zinger_show: None,
            over_wall: false,
            notices,
            came_down: None,
            rebounds: 0,
            them,
            swing_off: None,
            met: None,
            leads,
            signs,
            meter,
        });
        None
    }
}
