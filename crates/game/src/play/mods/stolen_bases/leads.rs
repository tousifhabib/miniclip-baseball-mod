//! The runners marked on the little field in the batting view, and the
//! click there that sends one.

use bb_engine::display::{Path, child_bounds};
use bb_engine::library::Library;
use bb_engine::math::Matrix;
use bb_engine::stage::Stage;

use super::{GOING, STANDING};
use crate::art;
use crate::look::{self};
use crate::play::fielding::ARRIVES;
use crate::play::overlay::DARK;
use crate::play::pitch::Point;
use crate::play::{Parts, Place, Runner, Runners, frame_of};

/// How many frames a mark is swollen for, and then not.
const BEAT: u32 = 8;

/// The sizes of a runner's mark on the little field and of the dark edge
/// under it, the art's dot being 1, and how many times its size a mark
/// swells to while its runner may be sent.
const FACE: f32 = 1.5;
const EDGE: f32 = 2.1;
const SWELL: f32 = 1.5;

/// One runner's mark on the little field: which runner, the base he stood
/// on when the pitch began, and the mark, with its edge under it.
pub(super) struct Lead {
    pub(super) runner: usize,
    pub(super) base: u8,
    pub(super) marks: [Path; 2],
}

/// A runner a click on the little field sends: who he is, the base he
/// leaves, and the one he goes for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Sent {
    pub runner: usize,
    pub from: u8,
    pub to: u8,
}

/// The runners' marks on the little field, for as long as a pitch's view
/// lasts.
pub(crate) struct Leads {
    /// Where the little field is in the view, and the box it covers there:
    /// left, top, right, bottom.
    pub(super) at: Point,
    pub(super) area: [f32; 4],
    pub(super) leads: Vec<Lead>,
    /// Where in the corner of the view the mod has its say.
    pub(super) hint_at: Point,
    pub(super) frames: u32,
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
    pub fn keep(&mut self, runners: &Runners, winding: bool, stage: &mut Stage) {
        self.frames += 1;
        let beat = (self.frames / BEAT).is_multiple_of(2);
        for lead in &self.leads {
            let Some(runner) = runners.get(lead.runner) else {
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
            let asked = winding && runners.may_steal(lead.runner).is_some();
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

impl Leads {
    /// Where in the corner of the view the mod has its say.
    pub fn hint_at(&self) -> Point {
        self.hint_at
    }

    /// Whether any runner marked here may be sent.
    pub fn anyone_may_go(&self, runners: &Runners) -> bool {
        self.leads
            .iter()
            .any(|lead| runners.may_steal(lead.runner).is_some())
    }

    /// The runner a click at `pointer` in the batting view sends, if it
    /// sends one: the click is on the little field, and he is the one who
    /// may go whose mark is nearest it.
    pub fn sent_by(&self, pointer: Point, runners: &Runners) -> Option<Sent> {
        let [left, top, right, bottom] = self.area;
        if !(left..=right).contains(&pointer.0) || !(top..=bottom).contains(&pointer.1) {
            return None;
        }
        let on_little = (pointer.0 - self.at.0, pointer.1 - self.at.1);
        let far = |base: u8| {
            let mark = art::LITTLE_BASES[usize::from(base) - 1];
            (mark.0 - on_little.0).hypot(mark.1 - on_little.1)
        };
        self.leads
            .iter()
            .filter_map(|lead| {
                Some(Sent {
                    runner: lead.runner,
                    from: lead.base,
                    to: runners.may_steal(lead.runner)?,
                })
            })
            .min_by(|a, b| far(a.from).total_cmp(&far(b.from)))
    }
}
