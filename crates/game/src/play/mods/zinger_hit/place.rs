//! Where a zinger came down, by how far it went.

use crate::play::pitch::Point;
use crate::rules::ZingerRules;

/// Where a zinger came down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Stands,
    Scoreboard,
    OutOfThePark,
}

impl Place {
    /// Where a ball comes down that lands at `landing`, this many times as
    /// far off as the wall. `scoreboard` is where the board behind the wall
    /// is drawn: left, top, right, bottom.
    pub fn of(
        landing: Point,
        walls: f32,
        scoreboard: Option<[f32; 4]>,
        rules: &ZingerRules,
    ) -> Place {
        let on_board = scoreboard.is_some_and(|[left, top, right, bottom]| {
            (left..=right).contains(&landing.0) && (top..=bottom).contains(&landing.1)
        });
        if on_board {
            Place::Scoreboard
        } else if walls <= rules.stands {
            Place::Stands
        } else {
            Place::OutOfThePark
        }
    }

    pub fn words(self) -> &'static str {
        match self {
            Place::Stands => "INTO THE STANDS",
            Place::Scoreboard => "OFF THE SCOREBOARD",
            Place::OutOfThePark => "OUT OF THE PARK",
        }
    }

    /// What the crowd makes of it.
    pub fn cheers(self) -> &'static [&'static str] {
        match self {
            Place::Stands => &["crowd_smallCheer"],
            Place::Scoreboard => &["crowd_bigClap"],
            Place::OutOfThePark => &["crowd_bigClap", "baseball_organ_FX"],
        }
    }
}
