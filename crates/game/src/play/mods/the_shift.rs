//! The shift: a mod that has the fielders stand where the batting side has
//! been hitting the ball.
//!
//! Every fair ball is remembered by how far across the field it came down.
//! Before each pitch the middle of the field is taken to be where the last
//! few went on the whole, and the fielders who roam stand as far to either
//! side of that as they stood of the real middle. They are moved wherever
//! they are drawn: on the field, behind the pitcher in the batting view,
//! and on the little field in its corner.

use bb_engine::display::{Path, child_bounds};
use bb_engine::library::Library;
use bb_engine::math::{ColorTransform, Matrix};
use bb_engine::stage::Stage;

use super::Line;
use crate::art;
use crate::play::field::reach;
use crate::play::pitch::Point;
use crate::play::{Parts, at, overlay};
use crate::rules::{FieldRules, ShiftRules};

/// The mod, in play: where the balls have been going, and where that has
/// the fielders standing.
pub(crate) struct TheShift {
    rules: ShiftRules,
    /// How far across the field each fair ball of this game came down,
    /// from 0 at one foul line to 1 at the other, in the order they were
    /// hit.
    spray: Vec<f32>,
    /// How far the middle has moved for the pitch in hand: to the left
    /// below nought, to the right above it.
    by: f32,
}

impl TheShift {
    pub fn new(rules: &ShiftRules) -> TheShift {
        TheShift {
            rules: rules.clone(),
            spray: Vec::new(),
            by: 0.0,
        }
    }

    /// A fair ball has come down this far across the field. It is
    /// remembered, caught or not.
    pub fn remember(&mut self, across: f32) {
        self.spray.push(across);
    }

    pub fn by(&self) -> f32 {
        self.by
    }

    /// Works out where the fielders stand for the coming pitch, by where
    /// the last few balls went.
    pub fn stand(&mut self) -> Shift {
        let shift = Shift::of(&self.spray, &self.rules);
        self.by = shift.by();
        shift
    }

    /// What the corner of the batting view says of where they stand, when
    /// they have moved.
    pub fn line(&self, shift: Shift) -> Option<Line> {
        shift.words(&self.rules).map(|words| Line {
            name: "shift",
            words: words.to_owned(),
            colour: [0xc8, 0xf0, 0xff],
        })
    }
}

/// The fielders who move: the three in the outfield and the shortstop,
/// counting from 0. The pitcher stays on his mound.
const ROAMERS: [usize; 4] = [0, 1, 3, 4];
/// Those of them who stand in the outfield, and what each is called in the
/// batting view once the shift has moved him there. The art has no drawing
/// of the left fielder, who is given the centre fielder's.
const OUTFIELD: [(usize, &str); 3] = [
    (0, "leftFielder"),
    (CENTRE, "centreFielder"),
    (4, "rightFielder"),
];
const CENTRE: usize = 3;
/// How near a foul line a fielder may be moved, the width of the field
/// between the lines being 1.
const MARGIN: f32 = 0.04;
/// How far a mark on the little field in the corner moves for each pixel
/// its fielder moves on the field itself, across and down.
const LITTLE: Point = (0.1525, 0.353);

/// Where the fielders take the middle of the field to be.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shift {
    /// How far across the field, from 0 on the left foul line to 1 on the
    /// right. With no shift on it is a half.
    pub middle: f32,
}

impl Shift {
    /// The shift for a side whose fair balls came down this far across the
    /// field, the latest last.
    pub fn of(spray: &[f32], rules: &ShiftRules) -> Shift {
        let from = spray.len().saturating_sub(rules.memory as usize);
        let last = &spray[from..];
        if last.is_empty() || last.len() < rules.least as usize {
            return Shift { middle: 0.5 };
        }
        let usual = last
            .iter()
            .map(|across| across.clamp(0.0, 1.0))
            .sum::<f32>()
            / last.len() as f32;
        let by = ((usual - 0.5) * rules.follow).clamp(-rules.most, rules.most);
        Shift { middle: 0.5 + by }
    }

    /// How far the middle has moved: to the left if less than nought.
    pub fn by(self) -> f32 {
        self.middle - 0.5
    }

    /// Where a fielder who stands this far across the field stands now.
    /// The foul lines stay where they are, and the field between them is
    /// squeezed on the side the middle has moved to and stretched on the
    /// other.
    pub fn across(self, across: f32) -> f32 {
        let middle = self.middle.clamp(MARGIN, 1.0 - MARGIN);
        let moved = if across <= 0.5 {
            across * middle / 0.5
        } else {
            1.0 - (1.0 - across) * (1.0 - middle) / 0.5
        };
        moved.clamp(MARGIN, 1.0 - MARGIN)
    }

    /// What the player is told of it, if it is enough to tell.
    pub fn words(self, rules: &ShiftRules) -> Option<&'static str> {
        match self.by() {
            by if by <= -rules.told => Some("SHIFT LEFT"),
            by if by >= rules.told => Some("SHIFT RIGHT"),
            _ => None,
        }
    }

    /// Moves the fielders who roam to where the shift has them stand: on
    /// the field, behind the pitcher in the batting view, and on the little
    /// field in its corner.
    pub(crate) fn place(
        self,
        parts: &Parts,
        field: &FieldRules,
        stage: &mut Stage,
        library: &Library,
    ) {
        if self.by() == 0.0 {
            return;
        }
        let ground = parts.ground(field);
        let little = stage.find_symbol(&parts.main, art::LITTLE_FIELD);
        // The centre fielder as the batting view draws him before anyone
        // has moved, and the lowest of the men it draws behind the pitcher.
        let centre = Drawn::of(CENTRE, parts, stage, library).map(|(_, drawn)| drawn);
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
            let drawn = Drawn::of(index, parts, stage, library);
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
                        let again = stage.attach(&parts.main, symbol, under, name, library);
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
    fn of(index: usize, parts: &Parts, stage: &Stage, library: &Library) -> Option<(Path, Drawn)> {
        let (_, depth) = art::VIEW_FIELDERS.iter().find(|(who, _)| *who == index)?;
        let mut path: Path = parts.main.clone();
        path.push(*depth);
        let child = stage.child(&path)?;
        if child.symbol != art::VIEW_FIELDER {
            return None;
        }
        let [left, _, right, _] = child_bounds(child, Matrix::IDENTITY, library)?;
        let drawn = Drawn {
            matrix: child.matrix,
            color: child.color,
            middle: (left + right) / 2.0,
        };
        Some((path, drawn))
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::rules::Rules;

    #[test]
    fn nobody_moves_until_enough_balls_have_been_put_in_play() {
        let rules = Rules::default().shift;
        assert_eq!(Shift::of(&[], &rules).middle, 0.5);
        let few = vec![0.1; rules.least as usize - 1];
        assert_eq!(Shift::of(&few, &rules).by(), 0.0);
        assert_eq!(Shift::of(&few, &rules).words(&rules), None);
        let enough = vec![0.1; rules.least as usize];
        assert!(Shift::of(&enough, &rules).by() < 0.0);
    }

    #[test]
    fn the_middle_goes_where_the_last_balls_went_and_no_further_than_it_may() {
        let rules = Rules::default().shift;
        let left = Shift::of(&[0.3, 0.2, 0.25, 0.3], &rules);
        assert!((left.middle - 0.2625).abs() < 1e-5, "{left:?}");
        assert_eq!(left.words(&rules), Some("SHIFT LEFT"));
        let right = Shift::of(&[0.9, 0.95, 1.0], &rules);
        assert_eq!(right.middle, 0.5 + rules.most);
        assert_eq!(right.words(&rules), Some("SHIFT RIGHT"));
        // Balls hit all round the field leave them where they were.
        let even = Shift::of(&[0.2, 0.8, 0.5, 0.45, 0.55], &rules);
        assert!(even.by().abs() < 0.01);
        assert_eq!(even.words(&rules), None);
        // Only the last few count, so going the other way brings them
        // back.
        let mut spray = vec![0.1; 20];
        spray.extend(vec![0.9; rules.memory as usize]);
        assert!(Shift::of(&spray, &rules).by() > 0.0);
    }

    #[test]
    fn the_fielders_keep_their_order_and_stay_between_the_lines() {
        for middle in [0.2, 0.35, 0.5, 0.7, 0.8] {
            let shift = Shift { middle };
            let stood = [0.0, 0.17, 0.27, 0.48, 0.5, 0.71, 1.0];
            let stand: Vec<f32> = stood.iter().map(|&across| shift.across(across)).collect();
            assert!(stand.windows(2).all(|pair| pair[0] <= pair[1]), "{stand:?}");
            assert!(stand.iter().all(|across| (0.0..=1.0).contains(across)));
            // The one in the middle is where the middle is now.
            assert!((stand[4] - middle).abs() < 1e-6, "{stand:?}");
            // Everyone has moved the way the middle did.
            for (was, now) in stood[1..6].iter().zip(&stand[1..6]) {
                assert_eq!((now - was).signum(), (middle - 0.5).signum());
            }
        }
        let none = Shift { middle: 0.5 };
        assert_eq!(none.across(0.3), 0.3);
    }

    /// Where balls might have come down across the field, the latest last,
    /// some of them in foul ground.
    fn any_spray() -> impl Strategy<Value = Vec<f32>> {
        prop::collection::vec(-0.3f32..1.3, 0..30)
    }

    /// Rules the fielders might shift by: how many balls they remember and
    /// how many they wait for, how far they follow them and how far they
    /// may go. That last is never less than nought: a file that had it so
    /// would stop the game at the sum that keeps the move within it.
    fn any_rules() -> impl Strategy<Value = ShiftRules> {
        (1u32..12, 0u32..6, 0.0f32..=2.0, 0.0f32..=0.5).prop_map(|(memory, least, follow, most)| {
            ShiftRules {
                memory,
                least,
                follow,
                most,
                told: 0.05,
            }
        })
    }

    proptest! {
        #[test]
        fn the_middle_stays_put_for_too_few_balls_and_never_goes_further_than_it_may(
            spray in any_spray(),
            rules in any_rules(),
        ) {
            let shift = Shift::of(&spray, &rules);
            let counted = spray.len().min(rules.memory as usize);
            if counted == 0 || counted < rules.least as usize {
                prop_assert_eq!(shift.middle, 0.5);
            }
            // A hair is allowed for the sum that puts the move on the half.
            prop_assert!(shift.by().abs() <= rules.most + 1e-6, "moved by {}", shift.by());
            // And it never goes away from where the balls went on the whole,
            // a ball in foul ground counting as one on the line. A hair is
            // allowed here too, for balls that went to neither side.
            let last = &spray[spray.len() - counted..];
            let lean: f32 = last.iter().map(|across| across.clamp(0.0, 1.0) - 0.5).sum();
            prop_assert!(shift.by() * lean >= -1e-9, "{} for balls {} over", shift.by(), lean);
        }

        #[test]
        fn only_the_last_balls_the_fielders_remember_count(
            spray in any_spray(),
            older in any_spray(),
            rules in any_rules(),
        ) {
            let remembered = &spray[spray.len().saturating_sub(rules.memory as usize)..];
            let shift = Shift::of(&spray, &rules);
            prop_assert_eq!(shift, Shift::of(remembered, &rules));
            // With as many balls as they remember, none from before those
            // makes any difference.
            if remembered.len() == rules.memory as usize {
                let longer = [&older[..], remembered].concat();
                prop_assert_eq!(shift, Shift::of(&longer, &rules));
            }
        }

        #[test]
        fn wherever_the_middle_goes_the_fielders_keep_their_order_and_stay_clear_of_the_lines(
            middle in 0.0f32..=1.0,
            one in -0.2f32..1.2,
            other in -0.2f32..1.2,
        ) {
            let shift = Shift { middle };
            let (left, right) = (one.min(other), one.max(other));
            let (stands_left, stands_right) = (shift.across(left), shift.across(right));
            prop_assert!(stands_left <= stands_right, "{} and {}", stands_left, stands_right);
            let clear = MARGIN..=1.0 - MARGIN;
            prop_assert!(clear.contains(&stands_left) && clear.contains(&stands_right));
            // The one in the middle is where the middle is now, if that is
            // clear of the lines.
            prop_assert_eq!(shift.across(0.5), middle.clamp(MARGIN, 1.0 - MARGIN));
        }

        #[test]
        fn with_no_shift_on_a_fielder_clear_of_the_lines_stands_just_where_he_stood(
            across in MARGIN..=1.0 - MARGIN,
        ) {
            prop_assert_eq!(Shift { middle: 0.5 }.across(across), across);
        }
    }
}
