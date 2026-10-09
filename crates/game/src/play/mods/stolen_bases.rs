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
use crate::play::fielding::ARRIVES;
use crate::play::overlay::{DARK, Says};
use crate::play::pitch::Point;
use crate::play::{Parts, Place, Runner, Runners, frame_of};

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
/// What the mod's line in the corner of the batting view is named, what it
/// says while a runner may be sent, and once one has been, and what the
/// view of the field says when it is known how that came out: the words,
/// and their colours.
pub(crate) const HINT: &str = "steal";
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

impl StolenBases {
    /// A try has come out: it is counted, and kept to be told.
    pub fn came_out(&mut self, safe: bool) {
        if safe {
            self.stolen += 1;
        } else {
            self.caught += 1;
        }
        self.to_tell = Some(if safe { STOLEN } else { CAUGHT });
    }
}

/// What the corner of the view says while a runner may be sent. `top` is
/// where the mod has its say there.
pub(crate) fn asks(top: Point) -> Says<'static> {
    Says::line(HINT, ASK, HINT_COLOUR).at(top)
}

/// What it says once one has been.
pub(crate) fn says_one_is_going(top: Point) -> Says<'static> {
    Says::line(HINT, SENT, GOING).at(top)
}

/// What the view of the field says of how a try came out, for this many
/// frames. `centre_x` is the middle of that view.
pub(crate) fn news(told: (&'static str, Rgb), frames: u32, centre_x: f32) -> Says<'static> {
    Says::news("stealNews", told.0, told.1, frames)
        .at((centre_x, NEWS_TOP))
        .sized(1.2)
}

/// Keeps a runner who is stealing from running on past the base he is
/// making for while the batting view is still up, where nobody is watching
/// for him to get there.
pub(crate) fn hold(runners: &Runners, stage: &mut Stage) {
    for runner in runners {
        let (Some(_), Some(to), Some(path)) = (runner.stole_from, runner.running_to, &runner.path)
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

/// The pitch was fouled off, or the side is out: whoever was stealing goes
/// back to the base he left.
pub(crate) fn send_back(runners: &mut Runners, stage: &mut Stage, library: &Library) {
    for runner in runners {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::runners::tests::{at, runners};

    /// Where the little field is in the view these tests click on.
    const LITTLE_AT: Point = (10.0, 20.0);

    /// The marks of runners standing on these bases, in the order they
    /// came up.
    fn leads(bases: &[u8]) -> Leads {
        let leads = bases.iter().enumerate().map(|(runner, &base)| Lead {
            runner,
            base,
            marks: [Path::new(), Path::new()],
        });
        Leads {
            at: LITTLE_AT,
            area: [10.0, 20.0, 110.0, 100.0],
            leads: leads.collect(),
            hint_at: (0.0, 0.0),
            frames: 0,
        }
    }

    /// The point of the view where the little field marks a base.
    fn mark_of(base: u8) -> Point {
        let mark = art::LITTLE_BASES[usize::from(base) - 1];
        (LITTLE_AT.0 + mark.0, LITTLE_AT.1 + mark.1)
    }

    #[test]
    fn a_click_off_the_little_field_sends_nobody() {
        let half = runners(vec![at(Place::Base(1))]);
        let marks = leads(&[1]);
        assert!(marks.anyone_may_go(&half));
        assert_eq!(marks.sent_by((300.0, 200.0), &half), None);
    }

    #[test]
    fn a_click_on_the_little_field_sends_the_runner_who_may_go() {
        // Second is free, and nobody steals home.
        let half = runners(vec![at(Place::Base(1)), at(Place::Base(3))]);
        let sent = leads(&[1, 3]).sent_by(mark_of(3), &half);
        let first_to_second = Sent {
            runner: 0,
            from: 1,
            to: 2,
        };
        assert_eq!(sent, Some(first_to_second));
        // The runner on first has one standing in his way. The one on
        // second has not.
        let half = runners(vec![at(Place::Base(1)), at(Place::Base(2))]);
        let sent = leads(&[1, 2]).sent_by(mark_of(1), &half);
        let second_to_third = Sent {
            runner: 1,
            from: 2,
            to: 3,
        };
        assert_eq!(sent, Some(second_to_third));
    }

    #[test]
    fn a_click_sends_nobody_when_nobody_may_go() {
        let half = runners(vec![at(Place::Base(3))]);
        let marks = leads(&[3]);
        assert!(!marks.anyone_may_go(&half));
        assert_eq!(marks.sent_by(mark_of(3), &half), None);
    }

    #[test]
    fn a_try_is_counted_and_kept_to_be_told() {
        let mut steals = StolenBases::default();
        steals.came_out(true);
        steals.came_out(false);
        assert_eq!((steals.stolen, steals.caught), (1, 1));
        assert_eq!(steals.to_tell, Some(CAUGHT));
    }
}
