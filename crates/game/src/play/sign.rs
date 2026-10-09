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

use super::field::Ground;
use super::overlay::{DARK, Says, Words};
use super::{AtBat, Match, Parts};
use crate::art;
use crate::look::{self, Rgb};
use crate::rules::{FieldRules, SignRules};

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

/// Puts a block of the art's on the stage as the four-sided shape with
/// these two bottom corners, standing `tall` up from them.
#[allow(
    clippy::too_many_arguments,
    reason = "it takes each thing it needs on its own, until they are gathered up"
)]
fn stand(
    holder: &[u16],
    depth: u16,
    name: &str,
    from: (f32, f32),
    to: (f32, f32),
    tall: f32,
    stage: &mut Stage,
    library: &Library,
) -> Option<Path> {
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

impl Board {
    /// Draws the signs on the wall in both views, with `lit` the one that
    /// is lit.
    #[allow(
        clippy::too_many_arguments,
        reason = "it takes each thing it needs on its own, until they are gathered up"
    )]
    pub fn put(
        signs: &Signs,
        lit: usize,
        rules: &SignRules,
        field: &FieldRules,
        parts: &Parts,
        ground: &Ground,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<Board> {
        let mut board = Board {
            panels: Vec::new(),
            lit,
            worth: (rules.lit, rules.unlit),
            frames: 0,
            struck: None,
        };
        // Each view has a clip for them, just over its picture of the
        // stadium and under everything else.
        let wall_in_view = |across: f32| parts.across_view(across, field);
        for over_field in [false, true] {
            let view = if over_field {
                &parts.field
            } else {
                &parts.main
            };
            let clip = stage.clip(view)?;
            let backdrop = clip
                .children
                .iter()
                .find(|(_, child)| art::BACKDROPS.contains(&child.symbol))
                .map(|(&depth, _)| depth)?;
            let depth = (backdrop + 1..).find(|depth| !clip.children.contains_key(depth))?;
            let holder = stage.attach(view, art::HOLDER, depth, "signs", library)?;
            for sign in 0..signs.count() {
                let (from, to) = signs.span(sign);
                // The two feet of the sign, and how tall it stands.
                let (left, right, tall) = if over_field {
                    let foot = |across: f32| {
                        let at = ground.point(across, ground.wall);
                        (at.0, at.1 - FOOT)
                    };
                    (foot(from), foot(to), TALL)
                } else {
                    let foot = VIEW_TOP + VIEW_TALL;
                    (
                        (wall_in_view(from), foot),
                        (wall_in_view(to), foot),
                        VIEW_TALL,
                    )
                };
                let base = sign as u16 * 4 + 1;
                let edge = stand(
                    &holder,
                    base,
                    "signEdge",
                    (left.0 - EDGE, left.1 + EDGE),
                    (right.0 + EDGE, right.1 + EDGE),
                    tall + EDGE * 2.0,
                    stage,
                    library,
                )?;
                if let Some(edge) = stage.child_mut(&edge) {
                    edge.set_color(look::tint(DARK));
                }
                let face = stand(&holder, base + 1, "sign", left, right, tall, stage, library)?;
                let middle = ((left.0 + right.0) / 2.0, (left.1 + right.1) / 2.0);
                let top = (middle.0, middle.1 - tall + 1.0);
                let name = if over_field {
                    "signWords"
                } else {
                    "signWordsSeen"
                };
                let words = Words::new(&holder, base + 2, name, top, WORDS_SIZE, stage, library)?;
                board.panels.push(Panel { sign, face, words });
            }
        }
        board.keep(stage);
        Some(board)
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
    /// The sign that is lit for the innings in hand, counting from 0. A
    /// new innings lights another.
    pub(crate) fn lit_sign(&mut self, signs: &Signs) -> usize {
        let innings = self.mode.full().map_or(1, |full| full.innings());
        let count = signs.count() as u32;
        match self.sign {
            Some((lit_in, lit)) if lit_in == innings && lit < signs.count() => lit,
            // Never the one that was lit the innings before.
            Some((_, was)) if count > 1 => {
                let pick = self.sign_rng.below(count - 1) as usize;
                let lit = if pick >= was { pick + 1 } else { pick };
                self.sign = Some((innings, lit));
                lit
            }
            _ => {
                let lit = self.sign_rng.below(count) as usize;
                self.sign = Some((innings, lit));
                lit
            }
        }
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
        self.sign_news = Some((sign, runs));
    }

    /// Says over the field that a sign was struck, once one has been.
    pub(crate) fn tell_sign(
        &mut self,
        at_bat: &mut AtBat,
        frames: u32,
        stage: &mut Stage,
        library: &Library,
    ) {
        let Some((sign, runs)) = self.sign_news.take() else {
            return;
        };
        self.sign_struck = Some((sign, runs));
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
