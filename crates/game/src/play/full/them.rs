//! The line that says what the other side made of the half it has just
//! batted.

use bb_engine::display::Path;
use bb_engine::stage::Stage;

use crate::art;
use crate::look::Rgb;
use crate::play::Parts;
use crate::play::overlay::Words;
use crate::play::pitch::Point;

/// Where the word that names the other side's score goes on a scoreboard,
/// which is over that score: the middle of its top edge, its size, the
/// lettering's own being 1, and its colour.
const THEM_AT: Point = (21.6, 2.4);
const THEM_SIZE: f32 = 10.0 / 18.0;
const THEM_COLOUR: Rgb = [0xfc, 0xf2, 0xa5];

/// The word a full match writes on a scoreboard over the other side's
/// score, in place of the art's own word for the score to beat.
pub(crate) struct Them {
    board: Path,
    words: Words,
    /// The board is the one over the field, which draws its home runs
    /// over its figures without taking them off.
    field: bool,
    /// What the other side is called, in the few letters there is room
    /// for.
    says: String,
}

impl Them {
    /// Puts the word on both of a view's scoreboards: `says`, which is
    /// what the other side is called. It is not seen until it is kept.
    pub fn put(parts: &Parts, says: &str, stage: &mut Stage) -> Vec<Them> {
        let boards = [(&parts.scoreboard, false), (&parts.field_scoreboard, true)];
        let mut put = Vec::new();
        for (board, field) in boards {
            let Some(board) = board else {
                continue;
            };
            let depth = Stage::RULES_DEPTH + 1;
            let words = Words::new(board, depth, "themLabel", THEM_AT, THEM_SIZE, stage);
            if let Some(words) = words {
                put.push(Them {
                    board: board.clone(),
                    words,
                    field,
                    says: says.to_owned(),
                });
            }
        }
        put
    }

    /// Takes the art's word out of sight, and shows this one in its place
    /// for as long as the board is showing its figures.
    pub fn keep(&self, stage: &mut Stage) {
        let Some(board) = stage.clip(&self.board) else {
            return;
        };
        let labels: Vec<u16> = board
            .children
            .iter()
            .filter(|(_, child)| art::TARGET_LABEL.contains(&child.symbol))
            .map(|(&depth, _)| depth)
            .collect();
        let showing = !labels.is_empty() && (!self.field || board.frame == 1);
        for depth in labels {
            let mut path = self.board.clone();
            path.push(depth);
            if let Some(label) = stage.child_mut(&path) {
                label.set_visible(false);
            }
        }
        if showing {
            self.words.say(&self.says, THEM_COLOUR, stage);
        } else {
            self.words.hide(stage);
        }
    }
}
