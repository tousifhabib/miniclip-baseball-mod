//! The runners round the bases: sent on, arriving, put out, and the
//! buttons that send them.

use bb_engine::library::Library;
use bb_engine::stage::Stage;
use bb_format::SymbolId;

use super::play::Fielding;
use crate::game::Game;
use crate::play::{Match, Parts, Phase, Place, frame_of, play_from, show};

/// The buttons that send a runner on from first, second and third.
const RUN_BUTTONS: [SymbolId; 3] = [1501, 1503, 1520];
const SLIDE_BUTTONS: [SymbolId; 5] = [1494, 1495, 1502, 1519, 1521];

/// Frames from here on are slides, not the run round the bases.
const FIRST_SLIDE_FRAME: u16 = 850;

/// The frame on which a runner gets to each base, the frame his slide ends
/// on, and the frame a slide rejoins the run at.
pub(crate) const ARRIVES: [u16; 4] = [211, 421, 630, 840];
const SLIDE_ENDS: [u16; 4] = [873, 890, 906, 922];
const SLIDE_JOINS: [u16; 4] = [210, 420, 629, 839];

/// The runner clip's frame labels for running and sliding to each base.
const RUN: [&str; 4] = ["runToFirst", "runToSecond", "runToThird", "runToFourth"];
const SLIDE: [&str; 4] = [
    "slideToFirst",
    "slideToSecond",
    "slideToThird",
    "slideToFourth",
];

impl Match {
    /// Sets a runner off for a base.
    pub(crate) fn send(&mut self, runner: usize, to: u8, stage: &mut Stage, library: &Library) {
        self.runners[runner].running_to = Some(to);
        self.runners[runner].sliding = false;
        if let Some(path) = &self.runners[runner].path {
            stage.goto_label(path, RUN[usize::from(to) - 1], true, library);
        }
    }

    /// The batter runs to first, and pushes on anyone in his way.
    pub(super) fn start_runners(&mut self, stage: &mut Stage, library: &Library) {
        for (runner, to) in self.runners.forced_on() {
            self.send(runner, to, stage, library);
        }
    }

    /// Puts a runner out.
    pub(super) fn put_out(&mut self, runner: usize, stage: &mut Stage, library: &Library) {
        let stealing = self.runners[runner].stole_from.take();
        let making_for = self.runners[runner].running_to.take();
        self.runners[runner].place = Place::Out;
        self.outs += 1;
        self.mods.somebody_is_out();
        match (stealing, making_for) {
            // A runner caught stealing is out, and the batter's count is
            // as it was.
            (Some(_), Some(base)) if self.mods.is_a_steal_in_play() => {
                self.a_steal_came_out(runner, base, false);
            }
            _ => self.clear_count(),
        }
        self.announce = true;
        let Some(path) = self.runners[runner].path.clone() else {
            return;
        };
        if let Some(clip) = stage.clip_mut(&path) {
            clip.playing = false;
        }
        for name in ["runner", "btn_slide", "runBtn"] {
            if let Some(part) = stage.find(&path, &[name]) {
                show(stage, &part, false);
            }
        }
        // "OUT" comes up over him, and he walks off.
        for name in ["outText", "outWalk"] {
            if let Some(part) = stage.find(&path, &[name]) {
                play_from(stage, &part, 2, library);
            }
        }
    }

    /// A runner has got to the base he was running to.
    pub(super) fn arrive(
        &mut self,
        runner: usize,
        parts: &Parts,
        stage: &mut Stage,
        library: &Library,
    ) {
        let Some(base) = self.runners[runner].running_to.take() else {
            return;
        };
        let was_batting = self.runners[runner].place == Place::AtBat;
        let path = self.runners[runner].path.clone();
        self.runners[runner].sliding = false;
        if self.runners[runner].stole_from.take().is_some() && self.mods.is_a_steal_in_play() {
            self.a_steal_came_out(runner, base, true);
        }
        if base == 4 {
            self.runners[runner].place = Place::Home;
            self.runners[runner].runs += self.run_worth;
            self.score += self.run_worth;
            if let Some(path) = &path {
                stage.goto_label(path, "addRun", false, library);
                if let Some(walk) = stage.find(path, &["outWalk"]) {
                    play_from(stage, &walk, 2, library);
                }
            }
        } else {
            self.runners[runner].place = Place::Base(base);
            if let Some(path) = &path {
                stage.goto_label(path, &format!("base{base}"), false, library);
            }
            let umpire = parts.umpires[usize::from(base) - 1].clone();
            self.play_section(&umpire, "safe", 49, stage, library);
        }
        if was_batting {
            self.mods.the_batter_reached_base();
            self.clear_count();
        }
        self.show_numbers(stage);
    }

    /// Brings in the runners who have got to their bases, and keeps the
    /// buttons that send them on for when they can be used.
    pub(super) fn move_runners(
        &mut self,
        state: &Fielding,
        ball_down: bool,
        parts: &Parts,
        stage: &mut Stage,
        library: &Library,
    ) {
        // Once the ball has been caught or has come down, a runner on a
        // base may try for the next. Turbo runners may at any time.
        let turbo = self.mods.runners_may_go_at_any_time();
        let may_go_on = state.play.is_live() && (turbo || state.was_caught() || ball_down);
        // A runner's run is a clip that plays a frame at a time. Turbo
        // runners are hurried on through it by more frames than that.
        let hurried = self.mods.hurry_the_runners();
        for runner in 0..self.runners.len() {
            let Some(path) = self.runners[runner].path.clone() else {
                continue;
            };
            let mut frame = frame_of(stage, &path);
            if let (Some(base), true) = (self.runners[runner].running_to, hurried > 0) {
                // As far as the base, or the end of his slide, and no
                // further: what comes after belongs to the next base.
                let index = usize::from(base) - 1;
                let end = if self.runners[runner].sliding {
                    SLIDE_ENDS[index]
                } else {
                    ARRIVES[index]
                };
                if frame < end {
                    frame = (frame + hurried).min(end);
                    play_from(stage, &path, frame, library);
                }
            }
            if let Some(base) = self.runners[runner].running_to {
                let index = usize::from(base) - 1;
                if self.runners[runner].sliding && frame >= SLIDE_ENDS[index] {
                    self.runners[runner].sliding = false;
                    play_from(stage, &path, SLIDE_JOINS[index], library);
                } else if frame >= ARRIVES[index] && frame < FIRST_SLIDE_FRAME {
                    self.arrive(runner, parts, stage, library);
                }
            } else if let Place::Base(base) = self.runners[runner].place {
                // The way on must be clear: nobody on the next base and
                // nobody making for it.
                let next = base + 1;
                let clear = next == 4
                    || (self.runners.on_base(next).is_none()
                        && !self
                            .runners
                            .iter()
                            .any(|other| other.running_to == Some(next)));
                if let Some(button) = stage.find(&path, &["runBtn"]) {
                    show(stage, &button, may_go_on && clear);
                }
            }
        }
    }

    /// A press on one of a runner's own buttons: slide, or go on.
    pub(crate) fn runner_button(
        &mut self,
        symbol: SymbolId,
        path: &[u16],
        _game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        if self.phase != Phase::Fielding {
            return;
        }
        let Some(runner) = self.runners.iter().position(|runner| {
            runner
                .path
                .as_ref()
                .is_some_and(|own| path.starts_with(own))
        }) else {
            return;
        };
        let Some(own) = self.runners[runner].path.clone() else {
            return;
        };
        if SLIDE_BUTTONS.contains(&symbol) {
            if let (Some(base), false) = (
                self.runners[runner].running_to,
                self.runners[runner].sliding,
            ) {
                self.runners[runner].sliding = true;
                stage.goto_label(&own, SLIDE[usize::from(base) - 1], true, library);
            }
        } else if RUN_BUTTONS.contains(&symbol)
            && self.runners[runner].running_to.is_none()
            && let Place::Base(base) = self.runners[runner].place
        {
            // The button is only showing when he may go, so going is all
            // there is to do.
            self.send(runner, base + 1, stage, library);
        }
    }
}
