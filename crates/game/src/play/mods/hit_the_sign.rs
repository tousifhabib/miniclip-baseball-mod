//! Hit the sign: a mod that puts signs along the outfield wall and pays in
//! runs for a ball that strikes one.
//!
//! The signs stand side by side on the wall, and one of them is lit, a
//! different one each innings. A ball that comes to the wall at a sign, no
//! higher than the sign is tall, has struck it: the lit one is worth the
//! most. The signs are drawn on the wall in both views. In the batting
//! view they are where the art's pointer shows a hit going, so the pointer
//! on a sign is a hit at it.

use bb_engine::display::Path;
use bb_engine::library::Library;
use bb_engine::math::Matrix;
use bb_engine::stage::Stage;

use crate::art;
use crate::look::{self, Rgb};
use crate::mods::About;
use crate::play::overlay::{self, DARK, Says, Words};
use crate::play::pitch::Point;
use crate::play::{AtBat, Match, Parts};
use crate::rng::Rng;
use crate::rules::{Rules, SignRules};

/// What the menu and the files know this mod by.
pub(crate) const ABOUT: About = About {
    key: "hit_the_sign",
    name: "HIT THE SIGN",
    does: "SIGNS ON THE WALL PAY RUNS, THE LIT ONE MOST OF ALL",
    setting: None,
};

/// What makes the choice of the lit sign come out differently from the
/// pitches, which are drawn from the seed itself: the same pitches come
/// whether the mod is on or not.
const SIGN_SEED: u64 = 0xbb67_ae85_84ca_a73b;

/// The mod, in play: which sign is lit, and what a ball that struck one
/// was worth.
pub(crate) struct HitTheSign {
    /// The innings a sign was last lit for, and which it was, counting
    /// from 0.
    lit: Option<(u32, usize)>,
    /// What the next sign to be lit is drawn by.
    rng: Rng,
    /// The sign a ball has just struck and the runs that was worth, until
    /// that has been told.
    pub news: Option<(usize, u32)>,
    /// The same for the pitch in hand, once it has been told.
    pub struck: Option<(usize, u32)>,
}

impl HitTheSign {
    /// `seed` is the game's own, from which the signs' is made.
    pub fn new(seed: u64) -> HitTheSign {
        HitTheSign {
            lit: None,
            rng: Rng::new(seed ^ SIGN_SEED),
            news: None,
            struck: None,
        }
    }

    /// Which sign is lit, counting from 0, once one has been.
    pub fn lit(&self) -> Option<usize> {
        self.lit.map(|(_, lit)| lit)
    }

    /// The sign a ball has struck on the pitch in hand and what it paid,
    /// told yet or not.
    pub fn struck(&self) -> Option<(usize, u32)> {
        self.struck.or(self.news)
    }

    /// The sign that is lit for this innings, counting from 0. A new
    /// innings lights another.
    pub fn light(&mut self, innings: u32, signs: &Signs) -> usize {
        let count = signs.count() as u32;
        match self.lit {
            Some((lit_in, lit)) if lit_in == innings && lit < signs.count() => lit,
            // Never the one that was lit the innings before.
            Some((_, was)) if count > 1 => {
                let pick = self.rng.below(count - 1) as usize;
                let lit = if pick >= was { pick + 1 } else { pick };
                self.lit = Some((innings, lit));
                lit
            }
            _ => {
                let lit = self.rng.below(count) as usize;
                self.lit = Some((innings, lit));
                lit
            }
        }
    }
}

/// How tall a sign is drawn on the wall over the field, in the field's
/// pixels, and how far up the wall its foot is.
const TALL: f32 = 11.0;
const FOOT: f32 = 1.0;
/// Where the wall is in the batting view: how far down the top of a sign
/// is, and how tall it is there.
const VIEW_TOP: f32 = 169.5;
const VIEW_TALL: f32 = 11.0;
/// How much wider and taller than a sign its dark edge is, all round.
const EDGE: f32 = 1.0;
/// The colours of a sign and of what is written on it: unlit, lit, and lit
/// on the beat.
const UNLIT: (Rgb, Rgb) = ([0x16, 0x34, 0x58], [0x9c, 0xc4, 0xe4]);
const LIT: (Rgb, Rgb) = ([0xff, 0xd2, 0x2a], [0x7a, 0x1a, 0x08]);
const BEAT: (Rgb, Rgb) = ([0xff, 0xf4, 0xb0], [0x7a, 0x1a, 0x08]);
/// How many frames a lit sign is one colour before it is the other, and
/// the same just after a ball has struck it.
const BEAT_FRAMES: u32 = 14;
const STRUCK_FRAMES: u32 = 4;
/// The size of what is written on a sign, the lettering's own being 1.
const WORDS_SIZE: f32 = 0.6;
/// Where the view of the field says that a sign was struck: how far down,
/// and in what colour.
const NEWS_TOP: f32 = 250.0;
const NEWS_COLOUR: Rgb = [0xff, 0xe2, 0x4a];

/// Where the signs are along the wall.
#[derive(Clone, Debug, PartialEq)]
pub struct Signs {
    /// How far across the field the middle of each is, from 0 on the left
    /// foul line to 1 on the right, and how wide each is, the same way.
    middles: Vec<f32>,
    width: f32,
}

impl Signs {
    pub fn of(rules: &SignRules) -> Signs {
        let count = rules.count.max(1);
        let apart = match count {
            1 => 0.0,
            count => (rules.last - rules.first) / (count - 1) as f32,
        };
        Signs {
            middles: (0..count)
                .map(|sign| rules.first + apart * sign as f32)
                .collect(),
            width: rules.width,
        }
    }

    pub fn count(&self) -> usize {
        self.middles.len()
    }

    /// Where a sign begins and ends, across the field.
    pub fn span(&self, sign: usize) -> (f32, f32) {
        let middle = self.middles[sign];
        (middle - self.width / 2.0, middle + self.width / 2.0)
    }

    /// The sign at this place across the field, counting from 0, if there
    /// is one there.
    pub fn at(&self, across: f32) -> Option<usize> {
        (0..self.count()).find(|&sign| {
            let (from, to) = self.span(sign);
            (from..=to).contains(&across)
        })
    }
}

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

impl Match {
    /// The sign that is lit for the innings in hand, counting from 0.
    /// `None` when the wall has no signs.
    pub(crate) fn lit_sign(&mut self, signs: &Signs) -> Option<usize> {
        let innings = self.mode.full().map_or(1, |full| full.innings());
        let sign = self.mods.hit_the_sign.as_mut()?;
        Some(sign.light(innings, signs))
    }

    /// The ball is at the wall, `across` the field and this high: if a
    /// sign is there and stands that tall, the ball has struck it, and the
    /// runs that is worth are the batter's. Only the first sign a hit
    /// strikes counts.
    pub(crate) fn strike_sign(
        &mut self,
        at_bat: &mut AtBat,
        across: f32,
        height: f32,
        rules: &SignRules,
    ) {
        let Some(board) = &mut at_bat.signs else {
            return;
        };
        if board.struck.is_some() || height > rules.high {
            return;
        }
        let Some(sign) = Signs::of(rules).at(across) else {
            return;
        };
        board.struck = Some(sign);
        let runs = if sign == board.lit {
            rules.lit
        } else {
            rules.unlit
        };
        self.score += runs;
        // The batter is the last to have come up.
        if let Some(batter) = self.runners.last_mut() {
            batter.runs += runs;
        }
        if let Some(signs) = &mut self.mods.hit_the_sign {
            signs.news = Some((sign, runs));
        }
    }

    /// Says over the field that a sign was struck, once one has been.
    pub(crate) fn tell_sign(
        &mut self,
        at_bat: &mut AtBat,
        frames: u32,
        stage: &mut Stage,
        library: &Library,
    ) {
        let Some(signs) = &mut self.mods.hit_the_sign else {
            return;
        };
        let Some((sign, runs)) = signs.news.take() else {
            return;
        };
        signs.struck = Some((sign, runs));
        self.show_numbers(stage);
        Match::sound(stage, library, "crowd_bigClap");
        Match::sound(stage, library, "baseball_organ_FX");
        at_bat.notices.put(
            Says::news(
                "signNews",
                &format!("OFF THE SIGN! +{runs}"),
                NEWS_COLOUR,
                frames,
            )
            .at((at_bat.parts.centre_x, NEWS_TOP))
            .sized(1.2),
            &at_bat.parts,
            stage,
            library,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rules;

    #[test]
    fn the_signs_stand_side_by_side_along_the_wall() {
        let rules = Rules::default().sign;
        let signs = Signs::of(&rules);
        assert_eq!(signs.count(), 5);
        // The first and the last are where the rules put them, and none
        // overlaps the next.
        let spans: Vec<(f32, f32)> = (0..5).map(|sign| signs.span(sign)).collect();
        assert!(((spans[0].0 + spans[0].1) / 2.0 - rules.first).abs() < 1e-5);
        assert!(((spans[4].0 + spans[4].1) / 2.0 - rules.last).abs() < 1e-5);
        assert!(spans.windows(2).all(|pair| pair[0].1 < pair[1].0));
        assert!(spans.iter().all(|span| span.0 > 0.0 && span.1 < 1.0));
        // A ball at the middle of one strikes it, and one in the gap
        // between two strikes nothing.
        assert_eq!(signs.at(rules.first), Some(0));
        assert_eq!(signs.at(0.52), Some(2));
        assert_eq!(signs.at((spans[1].1 + spans[2].0) / 2.0), None);
        assert_eq!(signs.at(0.02), None);
        // One sign alone is where the first would be.
        let one = Signs::of(&SignRules { count: 1, ..rules });
        assert_eq!(one.count(), 1);
        assert_eq!(one.at(0.24), Some(0));
    }
}
