//! What each mod does to a pitch before it is thrown, and what it puts
//! up in the view. Each step is one mod's, and is called in its place.

use bb_engine::library::Library;
use bb_engine::stage::Stage;

use super::Coming;
use crate::game::Game;
use crate::play::mods::{GoldenBall, TiredArm};
use crate::play::pitch::Pitch;
use crate::play::zinger::Zinger;
use crate::play::{MYSTERY_TOP, Match, bullet, sign, steal, timing};

impl Match {
    /// Settles whether the pitch is a golden ball, and with that what a run
    /// is worth on it, and colours a golden ball gold.
    ///
    /// The pitch about to be thrown is one more than have been. The arcade
    /// game has no runs and no outs for a golden ball to change.
    pub(super) fn gild_the_ball(&mut self, coming: &mut Coming, stage: &mut Stage) {
        let golden = self.mods.is_golden(self.pitched + 1);
        coming.golden = golden;
        self.run_worth = self.worth_of_a_run(golden);
        if golden {
            GoldenBall::gild(&coming.parts, stage);
        }
    }

    /// With the heat check mod on, every run since the last pitch makes
    /// this one faster.
    pub(super) fn heat_the_pitch(
        &mut self,
        coming: &mut Coming,
        stage: &mut Stage,
        library: &Library,
    ) {
        if let Some(line) = self.mods.heat_the_pitch(self.score, &mut coming.table) {
            coming.write(&line, stage, library);
        }
    }

    /// With the tired arm mod on, the pitcher is slower and wilder the more
    /// he has thrown, and one who has thrown his last gives way to a fresh
    /// one.
    pub(super) fn tire_the_arm(
        &mut self,
        coming: &mut Coming,
        stage: &mut Stage,
        library: &Library,
    ) {
        let Some(arm) = &mut self.mods.tired_arm else {
            return;
        };
        if arm.relieve() {
            Match::sound(stage, library, "baseball_organ_FX");
            coming.notices.put(
                arm.news().at((coming.parts.centre_x, MYSTERY_TOP)),
                &coming.parts,
                stage,
                library,
            );
        }
        let tired = arm.tire(&mut coming.table);
        if let Some(pitcher) = stage.child_mut(&coming.parts.pitcher) {
            pitcher.set_color(TiredArm::flush(tired));
        }
        coming.write(&arm.line(tired), stage, library);
    }

    /// With the hot bat mod on, every hit in a row has widened the window
    /// by a frame at each end.
    pub(super) fn widen_for_a_hot_bat(
        &self,
        coming: &mut Coming,
        stage: &mut Stage,
        library: &Library,
    ) {
        if let Some(line) = self.mods.widen_for_a_hot_bat(&mut coming.table) {
            coming.write(&line, stage, library);
        }
    }

    pub(super) fn say_the_ball_is_golden(
        coming: &mut Coming,
        stage: &mut Stage,
        library: &Library,
    ) {
        if coming.golden {
            coming.write(&GoldenBall::line(), stage, library);
        }
    }

    /// Settles whether this pitch is thrown in the clutch, and says so.
    pub(super) fn say_it_is_the_clutch(
        &mut self,
        coming: &mut Coming,
        stage: &mut Stage,
        library: &Library,
    ) {
        let in_it = self.in_the_clutch();
        let Some(line) = self.mods.settle_the_clutch(in_it) else {
            return;
        };
        // The organ plays as the batter comes up to it, and not again for
        // every pitch to him.
        if self.count.is_clean() {
            Match::sound(stage, library, "baseball_organ_tense_FX");
        }
        coming.write(&line, stage, library);
    }

    /// Says what runs are worth while a rally is on.
    pub(super) fn say_what_a_rally_is_worth(
        &self,
        coming: &mut Coming,
        stage: &mut Stage,
        library: &Library,
    ) {
        if let Some(line) = self.mods.rally_line() {
            coming.write(&line, stage, library);
        }
    }

    /// With bullet time on there is a meter, full when the game starts.
    pub(super) fn put_up_the_meter(
        &mut self,
        coming: &mut Coming,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<bullet::Meter> {
        if !self.mods.fill_the_meter_at_the_start() {
            return None;
        }
        let (top, under) = (coming.corner.line(), coming.corner.line());
        let under = (under.0, under.1 + 2.0);
        let meter = bullet::Meter::put(&coming.parts, top, under, stage, library);
        if let (Some(meter), Some(left)) = (&meter, self.mods.meter_left()) {
            meter.keep(left, false, stage);
        }
        meter
    }

    /// With the shift on, the fielders stand where the last few balls went.
    pub(super) fn shift_the_fielders(
        &mut self,
        coming: &mut Coming,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        let Some(the_shift) = &mut self.mods.the_shift else {
            return;
        };
        let shift = the_shift.stand();
        shift.place(&coming.parts, &game.rules.field, stage, library);
        if let Some(line) = the_shift.line(shift) {
            coming.write(&line, stage, library);
        }
    }

    /// With the stolen bases mod on, each runner is marked on the little
    /// field in the corner, and the mod has a line under it to write on.
    pub(super) fn mark_the_leads(
        &self,
        coming: &mut Coming,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<steal::Leads> {
        if !self.mods.runners_steal() || self.runners.on_the_bases() == 0 {
            return None;
        }
        let under = coming.corner.line();
        steal::Leads::put(&self.runners, under, &coming.parts, stage, library)
    }

    /// With the hit the sign mod on, the wall has its signs, one lit for
    /// the innings. The arcade game has no runs for a sign to be worth.
    pub(super) fn put_up_the_signs(
        &mut self,
        coming: &Coming,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<sign::Board> {
        self.mods.a_new_pitch_is_coming();
        let rules = &game.rules;
        let parts = &coming.parts;
        let signs = sign::Signs::of(&rules.sign);
        let innings = self.mode.full().map_or(1, |full| full.innings());
        let lit = self.mods.light_a_sign(innings, &signs)?;
        sign::Board::put(&signs, lit, rules, parts, stage, library)
    }

    /// The marks the art keeps on the field: whether a runner is on second,
    /// the fielders who mind the bases standing ready at them, and the
    /// runs to get called out when a side comes in.
    pub(super) fn mark_the_field(&mut self, coming: &Coming, stage: &mut Stage, library: &Library) {
        let parts = &coming.parts;
        if let Some(mark) = stage.find(&parts.main, &["runnerOnSecond"]) {
            let label = if self.runners.on_base(2).is_some() {
                "full"
            } else {
                "none"
            };
            stage.goto_label(&mark, label, false, library);
        }
        for fielder in parts.fielders.iter().skip(5) {
            stage.goto_label(fielder, "baseWaiting", false, library);
        }
        if self.announce {
            self.announce = false;
            // In a full match there is a number of runs that wins it only
            // when getting ahead ends it.
            let to_win = self.mode.full().is_none_or(|full| full.sudden());
            if let (true, Some(board)) = (to_win, parts.scoreboard.clone()) {
                self.play_section(&board, "runsToGet", 361, stage, library);
            }
        }
    }

    /// With the timing indicator on, the bar that shows when to swing. With
    /// the zinger mod on as well, it says how far a swing on each of its
    /// colours sends the ball at the most.
    pub(super) fn put_up_the_timing_bar(
        &self,
        coming: &Coming,
        pitch: &Pitch,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<timing::Indicator> {
        if !self.mods.the_timing_bar_is_shown() {
            return None;
        }
        let (table, parts) = (&coming.table, &coming.parts);
        let difficulty = game.settings.difficulty;
        let feet = |frames: u32| {
            Zinger::of(
                table,
                frames,
                (0.0, 0.0),
                parts.home,
                difficulty,
                &game.rules,
            )
            .map(|zinger| zinger.feet)
        };
        let feet: Option<&dyn Fn(u32) -> Option<u32>> =
            self.mods.every_hit_is_a_home_run().then_some(&feet);
        timing::Indicator::new(pitch, table, parts, feet, stage, library)
    }
}
