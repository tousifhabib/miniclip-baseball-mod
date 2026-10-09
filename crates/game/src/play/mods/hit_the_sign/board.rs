//! The signs as they are drawn on the outfield wall, in the view of the
//! field and behind the pitcher.

use bb_engine::display::Path;
use bb_engine::library::Library;
use bb_engine::math::Matrix;
use bb_engine::stage::Stage;

use super::{BEAT, LIT, Signs, UNLIT};
use crate::art;
use crate::look::{self};
use crate::play::Parts;
use crate::play::overlay::{self, DARK, Words};
use crate::play::pitch::Point;
use crate::rules::{Rules, SignRules};

/// The size of what is written on a sign, the lettering's own being 1.
const WORDS_SIZE: f32 = 0.6;

/// How many frames a lit sign is one colour before it is the other, and
/// the same just after a ball has struck it.
const BEAT_FRAMES: u32 = 14;
const STRUCK_FRAMES: u32 = 4;

/// How much wider and taller than a sign its dark edge is, all round.
const EDGE: f32 = 1.0;

/// Where the wall is in the batting view: how far down the top of a sign
/// is, and how tall it is there.
const VIEW_TOP: f32 = 169.5;
const VIEW_TALL: f32 = 11.0;

/// How tall a sign is drawn on the wall over the field, in the field's
/// pixels, and how far up the wall its foot is.
const TALL: f32 = 11.0;
const FOOT: f32 = 1.0;

/// One sign as it is drawn in one of the views: its face, and what is
/// written on it.
struct Panel {
    sign: usize,
    face: Path,
    words: Words,
}

/// The signs on the wall, in the batting view and over the field, for as
/// long as a pitch's view lasts.
pub(crate) struct Board {
    panels: Vec<Panel>,
    lit: usize,
    /// What each sign is worth: the lit one, and any other.
    worth: (u32, u32),
    frames: u32,
    /// The sign a ball has struck.
    struck: Option<usize>,
}

/// A four-sided shape standing up from the ground: its two bottom corners,
/// and how tall it stands from them.
#[derive(Clone, Copy)]
struct Upright {
    left: Point,
    right: Point,
    tall: f32,
}

impl Upright {
    /// The same shape with this much more of it all round.
    fn edged(self, by: f32) -> Upright {
        Upright {
            left: (self.left.0 - by, self.left.1 + by),
            right: (self.right.0 + by, self.right.1 + by),
            tall: self.tall + by * 2.0,
        }
    }

    /// The middle of its top edge, a little way down.
    fn top(self) -> Point {
        let middle = (
            (self.left.0 + self.right.0) / 2.0,
            (self.left.1 + self.right.1) / 2.0,
        );
        (middle.0, middle.1 - self.tall + 1.0)
    }
}

/// Puts a block of the art's on the stage as this shape.
fn stand(
    holder: &[u16],
    depth: u16,
    name: &str,
    shape: Upright,
    stage: &mut Stage,
    library: &Library,
) -> Option<Path> {
    let Upright {
        left: from,
        right: to,
        tall,
    } = shape;
    let path = stage.attach(holder, art::BLOCK, depth, name, library)?;
    stage.child_mut(&path)?.set_matrix(Matrix {
        a: (to.0 - from.0) / art::BLOCK_SIDE,
        b: (to.1 - from.1) / art::BLOCK_SIDE,
        c: 0.0,
        d: tall / art::BLOCK_SIDE,
        tx: from.0,
        ty: from.1 - tall,
    });
    Some(path)
}

impl Panel {
    /// Stands a sign up in a view as this shape: its dark edge, its face
    /// over that, and the words on it, named `words`.
    fn stand(
        sign: usize,
        holder: &[u16],
        shape: Upright,
        words: &str,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<Panel> {
        let base = sign as u16 * 4 + 1;
        let edge = stand(holder, base, "signEdge", shape.edged(EDGE), stage, library)?;
        if let Some(edge) = stage.child_mut(&edge) {
            edge.set_color(look::tint(DARK));
        }
        let face = stand(holder, base + 1, "sign", shape, stage, library)?;
        let top = shape.top();
        let words = Words::new(holder, base + 2, words, top, WORDS_SIZE, stage, library)?;
        Some(Panel { sign, face, words })
    }
}

impl Board {
    /// Draws the signs on the wall in both views, with `lit` the one that
    /// is lit.
    pub fn put(
        signs: &Signs,
        lit: usize,
        rules: &Rules,
        parts: &Parts,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<Board> {
        let mut board = Board {
            panels: Vec::new(),
            lit,
            worth: (rules.sign.lit, rules.sign.unlit),
            frames: 0,
            struck: None,
        };
        let ground = parts.ground(&rules.field);
        let wall_in_view = |across: f32| parts.across_view(across, &rules.field);
        for over_field in [false, true] {
            let (view, words) = if over_field {
                (&parts.field, "signWords")
            } else {
                (&parts.main, "signWordsSeen")
            };
            let holder = Board::holder_in(view, stage, library)?;
            for sign in 0..signs.count() {
                let (from, to) = signs.span(sign);
                // The two feet of the sign, and how tall it stands.
                let shape = if over_field {
                    let foot = |across: f32| {
                        let at = ground.point(across, ground.wall);
                        (at.0, at.1 - FOOT)
                    };
                    Upright {
                        left: foot(from),
                        right: foot(to),
                        tall: TALL,
                    }
                } else {
                    let foot = VIEW_TOP + VIEW_TALL;
                    Upright {
                        left: (wall_in_view(from), foot),
                        right: (wall_in_view(to), foot),
                        tall: VIEW_TALL,
                    }
                };
                let panel = Panel::stand(sign, &holder, shape, words, stage, library)?;
                board.panels.push(panel);
            }
        }
        board.keep(stage);
        Some(board)
    }

    /// A clip for the signs in one of the views, just over its picture of
    /// the stadium and under everything else.
    fn holder_in(view: &[u16], stage: &mut Stage, library: &Library) -> Option<Path> {
        let clip = stage.clip(view)?;
        let backdrop = clip
            .children
            .iter()
            .find(|(_, child)| art::BACKDROPS.contains(&child.symbol))
            .map(|(&depth, _)| depth)?;
        let depth = overlay::free_above(clip, backdrop)?;
        stage.attach(view, art::HOLDER, depth, "signs", library)
    }

    /// Called every frame: colours each sign, and has the lit one beat.
    pub fn keep(&mut self, stage: &mut Stage) {
        self.frames += 1;
        let every = match self.struck {
            Some(_) => STRUCK_FRAMES,
            None => BEAT_FRAMES,
        };
        let beat = (self.frames / every).is_multiple_of(2);
        for panel in &self.panels {
            let lit = panel.sign == self.lit;
            // A sign that has been struck beats as the lit one does.
            let beats = self.struck.map_or(lit, |struck| panel.sign == struck);
            let (face, words) = match (lit, beats && beat) {
                (_, true) => BEAT,
                (true, false) => LIT,
                (false, false) => UNLIT,
            };
            if let Some(sign) = stage.child_mut(&panel.face) {
                sign.set_color(look::tint(face));
            }
            let worth = if lit { self.worth.0 } else { self.worth.1 };
            panel.words.say(&format!("+{worth}"), words, stage);
        }
    }
}

impl Board {
    /// The ball is at the wall, `across` the field and this high: the sign
    /// it has struck and the runs that is worth, if a sign is there and
    /// stands that tall. Only the first sign a hit strikes counts.
    pub fn strike(&mut self, across: f32, height: f32, rules: &SignRules) -> Option<(usize, u32)> {
        if self.struck.is_some() || height > rules.high {
            return None;
        }
        let sign = Signs::of(rules).at(across)?;
        self.struck = Some(sign);
        let runs = if sign == self.lit {
            rules.lit
        } else {
            rules.unlit
        };
        Some((sign, runs))
    }
}
