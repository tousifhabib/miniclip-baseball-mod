//! Where the fielders stand when they have shifted, and how the little
//! field in the batting view draws them there.

use bb_engine::display::{Path, child_bounds};
use bb_engine::math::{ColorTransform, Matrix};
use bb_engine::stage::Stage;

use super::Shift;
use crate::art;
use crate::play::field::reach;
use crate::play::pitch::Point;
use crate::play::{Parts, at, overlay};
use crate::rules::FieldRules;

/// How far a mark on the little field in the corner moves for each pixel
/// its fielder moves on the field itself, across and down.
const LITTLE: Point = (0.1525, 0.353);

/// Those of them who stand in the outfield, and what each is called in the
/// batting view once the shift has moved him there. The art has no drawing
/// of the left fielder, who is given the centre fielder's.
const OUTFIELD: [(usize, &str); 3] = [
    (0, "leftFielder"),
    (CENTRE, "centreFielder"),
    (4, "rightFielder"),
];
const CENTRE: usize = 3;

/// The fielders who move: the three in the outfield and the shortstop,
/// counting from 0. The pitcher stays on his mound.
const ROAMERS: [usize; 4] = [0, 1, 3, 4];

impl Shift {
    /// Moves the fielders who roam to where the shift has them stand: on
    /// the field, behind the pitcher in the batting view, and on the little
    /// field in its corner.
    pub(crate) fn place(self, parts: &Parts, field: &FieldRules, stage: &mut Stage) {
        if self.by() == 0.0 {
            return;
        }
        let ground = parts.ground(field);
        let little = stage.find_symbol(&parts.main, art::LITTLE_FIELD);
        // The centre fielder as the batting view draws him before anyone
        // has moved, and the lowest of the men it draws behind the pitcher.
        let centre = Drawn::of(CENTRE, parts, stage).map(|(_, drawn)| drawn);
        let lowest = stage.clip(&parts.main).and_then(|view| {
            let men = [art::VIEW_FIELDER, art::VIEW_UMPIRE];
            let lowest = view
                .children
                .iter()
                .find(|(_, child)| men.contains(&child.symbol));
            lowest.map(|(&depth, _)| depth)
        });
        for index in ROAMERS {
            let Some(fielder) = parts.fielders.get(index) else {
                continue;
            };
            let was = at(stage, fielder);
            let (stood, stands) = (ground.across(was), self.across(ground.across(was)));
            let to = ground.point(stands, reach(ground.home, was));
            if let Some(fielder) = stage.child_mut(fielder) {
                fielder.move_to(to.0, to.1);
            }
            // In the batting view he goes as far across as the art's
            // pointer does for a ball hit that much further over. He is as
            // far off as he was, so no higher up the view and no smaller.
            let over = parts.across_view(stands, field);
            let slide = over - parts.across_view(stood, field);
            let drawn = Drawn::of(index, parts, stage);
            match OUTFIELD.iter().find(|(who, _)| *who == index) {
                None => {
                    if let Some((path, drawn)) = drawn
                        && let Some(figure) = stage.child_mut(&path)
                    {
                        figure.move_to(drawn.matrix.tx + slide, drawn.matrix.ty);
                    }
                }
                // An outfielder may now stand behind a man of the infield,
                // so he is drawn again under them all. The left fielder,
                // whom the art leaves just out of the picture, is drawn as
                // the centre fielder is, where the pointer has him stand.
                Some(&(_, name)) => {
                    let (like, middle) = match &drawn {
                        Some((_, drawn)) => (Some(*drawn), drawn.middle + slide),
                        None => (centre, over),
                    };
                    let under = lowest
                        .zip(stage.clip(&parts.main))
                        .and_then(|(lowest, view)| overlay::free_below(view, lowest));
                    if let (Some(like), Some(under)) = (like, under) {
                        if let Some((path, _)) = &drawn {
                            stage.remove(path);
                        }
                        let symbol = art::VIEW_FIELDER;
                        let again = stage.attach(&parts.main, symbol, under, name);
                        if let Some(figure) = again.and_then(|path| stage.child_mut(&path)) {
                            figure.set_matrix(Matrix {
                                tx: like.matrix.tx + middle - like.middle,
                                ..like.matrix
                            });
                            figure.set_color(like.color);
                        }
                    }
                }
            }
            // His mark on the little field goes the same way, by as much as
            // that field is smaller.
            let mark = art::LITTLE_FIELDERS
                .iter()
                .find(|(fielder, _)| *fielder == index)
                .zip(little.as_ref())
                .map(|(&(_, depth), little)| {
                    let mut path: Path = little.clone();
                    path.push(depth);
                    path
                });
            if let Some(mark) = mark.and_then(|mark| stage.child_mut(&mark)) {
                let (x, y) = (mark.matrix.tx, mark.matrix.ty);
                mark.move_to(x + (to.0 - was.0) * LITTLE.0, y + (to.1 - was.1) * LITTLE.1);
            }
        }
    }
}

/// A fielder as the batting view draws him behind the pitcher.
#[derive(Clone, Copy)]
struct Drawn {
    matrix: Matrix,
    color: ColorTransform,
    /// Where the middle of him comes, across the view.
    middle: f32,
}

impl Drawn {
    /// The art's own drawing of a fielder, counting from 0, and where it
    /// is on the stage. `None` for one it does not draw.
    pub(super) fn of(index: usize, parts: &Parts, stage: &Stage) -> Option<(Path, Drawn)> {
        let (_, depth) = art::VIEW_FIELDERS.iter().find(|(who, _)| *who == index)?;
        let mut path: Path = parts.main.clone();
        path.push(*depth);
        let child = stage.child(&path)?;
        if child.symbol != art::VIEW_FIELDER {
            return None;
        }
        let [left, _, right, _] = child_bounds(child, Matrix::IDENTITY, stage.library())?;
        let drawn = Drawn {
            matrix: child.matrix,
            color: child.color,
            middle: (left + right) / 2.0,
        };
        Some((path, drawn))
    }
}
