//! The shift: a mod that has the fielders stand where the batting side has
//! been hitting the ball.
//!
//! Every fair ball is remembered by how far across the field it came down.
//! Before each pitch the middle of the field is taken to be where the last
//! few went on the whole, and the fielders who roam stand as far to either
//! side of that as they stood of the real middle. The little field in the
//! corner of the batting view shows them where they are.

use bb_engine::display::Path;
use bb_engine::stage::Stage;

use super::field::{Ground, reach};
use super::pitch::Point;
use super::{Parts, at};
use crate::art;
use crate::rules::ShiftRules;

/// The fielders who move: the three in the outfield and the shortstop,
/// counting from 0. The pitcher stays on his mound.
const ROAMERS: [usize; 4] = [0, 1, 3, 4];
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
    pub fn by(&self) -> f32 {
        self.middle - 0.5
    }

    /// Where a fielder who stands this far across the field stands now.
    /// The foul lines stay where they are, and the field between them is
    /// squeezed on the side the middle has moved to and stretched on the
    /// other.
    pub fn across(&self, across: f32) -> f32 {
        let middle = self.middle.clamp(MARGIN, 1.0 - MARGIN);
        let moved = if across <= 0.5 {
            across * middle / 0.5
        } else {
            1.0 - (1.0 - across) * (1.0 - middle) / 0.5
        };
        moved.clamp(MARGIN, 1.0 - MARGIN)
    }

    /// What the player is told of it, if it is enough to tell.
    pub fn words(&self, rules: &ShiftRules) -> Option<&'static str> {
        match self.by() {
            by if by <= -rules.told => Some("SHIFT LEFT"),
            by if by >= rules.told => Some("SHIFT RIGHT"),
            _ => None,
        }
    }

    /// Moves the fielders who roam to where the shift has them stand, on
    /// the field and on the little field in the corner of the batting view.
    pub(crate) fn place(&self, parts: &Parts, ground: &Ground, stage: &mut Stage) {
        if self.by() == 0.0 {
            return;
        }
        let little = stage.find_symbol(&parts.main, art::LITTLE_FIELD);
        for index in ROAMERS {
            let Some(fielder) = parts.fielders.get(index) else {
                continue;
            };
            let was = at(stage, fielder);
            let to = ground.point(self.across(ground.across(was)), reach(ground.home, was));
            if let Some(fielder) = stage.child_mut(fielder) {
                fielder.move_to(to.0, to.1);
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

#[cfg(test)]
mod tests {
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
}
