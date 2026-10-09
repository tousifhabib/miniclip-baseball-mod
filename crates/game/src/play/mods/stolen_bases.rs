//! Stolen bases: a mod that lets a runner be sent for the next base while
//! the pitcher winds up.
//!
//! The batting view shows nothing of the runners but the one on second, so
//! each is marked on the little field in its corner. A click on that field
//! during the wind-up sends the runner whose mark is nearest, if the base in
//! front of him is free. If the pitch is hit fair he is a runner like any
//! other, with a start. If it is fouled off he goes back. And if it is not
//! hit at all the catcher throws, and the view goes to the field to see
//! which of them is there first.

use bb_engine::display::{Path, child_bounds};
use bb_engine::library::Library;
use bb_engine::math::Matrix;
use bb_engine::stage::Stage;

use crate::art;
use crate::look::{self, Rgb};
use crate::mods::About;
use crate::play::book::ORDER;
use crate::play::fielding::ARRIVES;
use crate::play::overlay::{DARK, Says};
use crate::play::pitch::Point;
use crate::play::{AtBat, Match, Parts, Place, Runner, frame_of};

/// What the menu and the files know this mod by.
pub(crate) const ABOUT: About = About {
    key: "stolen_bases",
    name: "STOLEN BASES",
    does: "CLICK THE LITTLE FIELD IN THE WIND-UP TO SEND A RUNNER",
    setting: None,
};

/// The mod, in play: the bases stolen, the runners caught, and how the last
/// try came out.
#[derive(Default)]
pub(crate) struct StolenBases {
    /// How many bases have been stolen in this game, and how many runners
    /// caught at it.
    pub stolen: u32,
    pub caught: u32,
    /// Whether the play in the field is one on which a base can be stolen:
    /// the pitch was not hit, and a runner had gone.
    pub in_play: bool,
    /// How the last try came out, and in what colour to say so, until that
    /// has been told.
    pub to_tell: Option<(&'static str, Rgb)>,
}

/// The sizes of a runner's mark on the little field and of the dark edge
/// under it, the art's dot being 1, and how many times its size a mark
/// swells to while its runner may be sent.
const FACE: f32 = 1.5;
const EDGE: f32 = 2.1;
const SWELL: f32 = 1.5;
/// How many frames a mark is swollen for, and then not.
const BEAT: u32 = 8;
/// The colours of a runner's mark: standing on his base, and going.
const STANDING: Rgb = [0xff, 0xff, 0xff];
const GOING: Rgb = [0xff, 0x8a, 0x2a];
/// What the corner of the batting view says while a runner may be sent,
/// and once one has been, and what the view of the field says when it is
/// known how that came out: the words, and their colours.
const ASK: &str = "CLICK TO STEAL";
const SENT: &str = "RUNNER GOING";
const HINT_COLOUR: Rgb = [0xc8, 0xf0, 0xff];
const STOLEN: (&str, Rgb) = ("STOLEN BASE!", [0xff, 0xe2, 0x4a]);
const CAUGHT: (&str, Rgb) = ("CAUGHT STEALING!", [0xff, 0x8a, 0x6a]);
/// How far down the view of the field that is said.
const NEWS_TOP: f32 = 232.0;

/// One runner's mark on the little field: which runner, the base he stood
/// on when the pitch began, and the mark, with its edge under it.
struct Lead {
    runner: usize,
    base: u8,
    marks: [Path; 2],
}

/// The runners' marks on the little field, for as long as a pitch's view
/// lasts.
pub(crate) struct Leads {
    /// Where the little field is in the view, and the box it covers there:
    /// left, top, right, bottom.
    at: Point,
    area: [f32; 4],
    leads: Vec<Lead>,
    /// Where in the corner of the view the mod has its say.
    hint_at: Point,
    frames: u32,
}

/// The point this far of the way from one point to another.
fn between(from: Point, to: Point, share: f32) -> Point {
    (
        from.0 + (to.0 - from.0) * share,
        from.1 + (to.1 - from.1) * share,
    )
}

impl Leads {
    /// Marks every runner who is on a base. `None` if nobody is, or the
    /// view has no little field. `hint_at` is where in the corner of the
    /// view the mod may write.
    pub fn put(
        runners: &[Runner],
        hint_at: Point,
        parts: &Parts,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<Leads> {
        let little = stage.find_symbol(&parts.main, art::LITTLE_FIELD)?;
        let child = stage.child(&little)?;
        let at = (child.matrix.tx, child.matrix.ty);
        let area = child_bounds(child, Matrix::IDENTITY, library)?;
        let mut leads = Vec::new();
        for (index, runner) in runners.iter().enumerate() {
            let Place::Base(base) = runner.place else {
                continue;
            };
            let depth = Stage::RULES_DEPTH + 100 + index as u16 * 2;
            let edge = stage.attach(&little, art::DOT, depth, "leadEdge", library)?;
            let face = stage.attach(&little, art::DOT, depth + 1, "lead", library)?;
            if let Some(edge) = stage.child_mut(&edge) {
                edge.set_color(look::tint(DARK));
            }
            leads.push(Lead {
                runner: index,
                base,
                marks: [edge, face],
            });
        }
        if leads.is_empty() {
            return None;
        }
        Some(Leads {
            at,
            area,
            leads,
            hint_at,
            frames: 0,
        })
    }

    /// Called every frame the batting view is up: keeps each mark where
    /// its runner is, and has the marks of those who may be sent beat.
    /// `winding` is whether the pitcher is in his wind-up.
    pub fn keep(&mut self, play: &Match, winding: bool, stage: &mut Stage) {
        self.frames += 1;
        let beat = (self.frames / BEAT).is_multiple_of(2);
        for lead in &self.leads {
            let Some(runner) = play.runners.get(lead.runner) else {
                continue;
            };
            let stood = art::LITTLE_BASES[usize::from(lead.base) - 1];
            let going = runner.stole_from.zip(runner.running_to);
            let at = match going {
                Some((from, to)) if (2..=3).contains(&to) => {
                    // As far along as his run has got.
                    let (start, end) =
                        (ARRIVES[usize::from(from) - 1], ARRIVES[usize::from(to) - 1]);
                    let frame = runner
                        .path
                        .as_ref()
                        .map_or(start, |path| frame_of(stage, path));
                    let share = f32::from(frame.clamp(start, end) - start) / f32::from(end - start);
                    between(stood, art::LITTLE_BASES[usize::from(to) - 1], share)
                }
                _ => stood,
            };
            let asked = winding && play.runners.may_steal(lead.runner).is_some();
            let swell = if asked && beat { SWELL } else { 1.0 };
            let colour = if going.is_some() { GOING } else { STANDING };
            for (mark, size) in lead.marks.iter().zip([EDGE, FACE]) {
                let Some(mark) = stage.child_mut(mark) else {
                    continue;
                };
                mark.set_matrix(Matrix {
                    a: size * swell,
                    d: size * swell,
                    tx: at.0,
                    ty: at.1,
                    ..Matrix::IDENTITY
                });
            }
            if let Some(face) = stage.child_mut(&lead.marks[1]) {
                face.set_color(look::tint(colour));
            }
        }
    }
}

impl Match {
    /// The wind-up has begun: if a runner may be sent, the corner of the
    /// view says so.
    pub(crate) fn ask_for_steals(&self, at_bat: &mut AtBat, stage: &mut Stage, library: &Library) {
        let Some(leads) = &at_bat.leads else {
            return;
        };
        let anyone = leads
            .leads
            .iter()
            .any(|lead| self.runners.may_steal(lead.runner).is_some());
        if anyone {
            let (notices, parts, top) = (&mut at_bat.notices, &at_bat.parts, leads.hint_at);
            notices.put(
                Says::line("steal", ASK, HINT_COLOUR).at(top),
                parts,
                stage,
                library,
            );
        }
    }

    /// The ball has left the pitcher's hand: nobody can be sent now.
    pub(crate) fn stop_asking_for_steals(&self, at_bat: &mut AtBat, stage: &mut Stage) {
        if !self.runners.anyone_stealing() {
            at_bat.notices.take_down("steal", stage);
        }
    }

    /// A click during the wind-up, at `pointer` in the batting view. If it
    /// is on the little field, the runner whose mark is nearest it who may
    /// go is sent for the next base.
    pub(crate) fn steal_click(
        &mut self,
        at_bat: &mut AtBat,
        pointer: Point,
        stage: &mut Stage,
        library: &Library,
    ) {
        let Some(leads) = &at_bat.leads else {
            return;
        };
        let [left, top, right, bottom] = leads.area;
        if !(left..=right).contains(&pointer.0) || !(top..=bottom).contains(&pointer.1) {
            return;
        }
        let on_little = (pointer.0 - leads.at.0, pointer.1 - leads.at.1);
        let far = |base: u8| {
            let mark = art::LITTLE_BASES[usize::from(base) - 1];
            (mark.0 - on_little.0).hypot(mark.1 - on_little.1)
        };
        let nearest = leads
            .leads
            .iter()
            .filter_map(|lead| Some((lead.runner, lead.base, self.runners.may_steal(lead.runner)?)))
            .min_by(|a, b| far(a.1).total_cmp(&far(b.1)));
        let Some((runner, from, to)) = nearest else {
            return;
        };
        self.runners[runner].stole_from = Some(from);
        self.send(runner, to, stage, library);
        let (notices, parts, top) = (&mut at_bat.notices, &at_bat.parts, leads.hint_at);
        notices.put(
            Says::line("steal", SENT, GOING).at(top),
            parts,
            stage,
            library,
        );
    }

    /// Keeps a runner who is stealing from running on past the base he is
    /// making for while the batting view is still up, where nobody is
    /// watching for him to get there.
    pub(crate) fn hold_stealers(&self, stage: &mut Stage) {
        for runner in &self.runners {
            let (Some(_), Some(to), Some(path)) =
                (runner.stole_from, runner.running_to, &runner.path)
            else {
                continue;
            };
            if frame_of(stage, path) >= ARRIVES[usize::from(to) - 1]
                && let Some(clip) = stage.clip_mut(path)
            {
                clip.playing = false;
            }
        }
    }

    /// The pitch was hit fair: whoever was stealing is a runner like any
    /// other now, and has stolen nothing.
    pub(crate) fn steals_are_runs(&mut self) {
        for runner in &mut self.runners {
            runner.stole_from = None;
        }
    }

    /// The pitch was fouled off, or the side is out: whoever was stealing
    /// goes back to the base he left.
    pub(crate) fn steals_go_back(&mut self, stage: &mut Stage, library: &Library) {
        for runner in &mut self.runners {
            let Some(from) = runner.stole_from.take() else {
                continue;
            };
            runner.running_to = None;
            runner.sliding = false;
            if let Some(path) = &runner.path {
                stage.goto_label(path, &format!("base{from}"), false, library);
            }
        }
    }

    /// Four balls: a runner the walk pushes on was going there anyway, and
    /// has stolen nothing. One it does not push has a base to steal, with
    /// nobody throwing.
    pub(crate) fn steals_on_a_walk(&mut self) {
        // The walk pushes the runner on first, and whoever is on the bases
        // behind him without a gap.
        let pushed = self.runners.pushed_by_a_walk();
        for runner in &mut self.runners {
            if let Some(from) = runner.stole_from
                && pushed[usize::from(from) - 1]
            {
                runner.stole_from = None;
            }
        }
        let stealing = self.runners.anyone_stealing();
        self.mods.a_steal_is_in_play(stealing);
    }

    /// A runner who was stealing has got to `base`, or has been put out on
    /// his way there: it is counted, written in a full match's book, and
    /// told.
    pub(crate) fn stole(&mut self, runner: usize, base: u8, safe: bool) {
        if let Some(steals) = &mut self.mods.stolen_bases {
            if safe {
                steals.stolen += 1;
            } else {
                steals.caught += 1;
            }
            steals.to_tell = Some(if safe { STOLEN } else { CAUGHT });
        }
        let order = self.runners[runner].order % ORDER;
        if let Some(full) = self.mode.full_mut() {
            let innings = full.innings();
            full.book.ours.stole(innings, order, base, safe);
        }
    }

    /// Says over the field how a steal came out, once it has.
    pub(crate) fn tell_steal(
        &mut self,
        at_bat: &mut AtBat,
        frames: u32,
        stage: &mut Stage,
        library: &Library,
    ) {
        let news = self
            .mods
            .stolen_bases
            .as_mut()
            .and_then(|steals| steals.to_tell.take());
        let Some((says, colour)) = news else {
            return;
        };
        at_bat.notices.take_down("steal", stage);
        let top = (at_bat.parts.centre_x, NEWS_TOP);
        at_bat.notices.put(
            Says::news("stealNews", says, colour, frames)
                .at(top)
                .sized(1.2),
            &at_bat.parts,
            stage,
            library,
        );
    }
}
