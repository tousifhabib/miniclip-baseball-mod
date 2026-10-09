//! Which way a fielder faces, and the frames of his clip for each way.

use crate::play::pitch::Point;

/// The eight ways a fielder can face, by where he is headed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Facing {
    Left,
    Right,
    Up,
    Down,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

impl Facing {
    /// The way from `from` to `to`. Up the screen is away from home plate.
    pub fn towards(from: Point, to: Point) -> Facing {
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        // Eighths of a turn, starting from due right and going clockwise on
        // screen.
        let eighth = (dy.atan2(dx) / std::f32::consts::FRAC_PI_4).round() as i32;
        match eighth.rem_euclid(8) {
            0 => Facing::Right,
            1 => Facing::DownRight,
            2 => Facing::Down,
            3 => Facing::DownLeft,
            4 => Facing::Left,
            5 => Facing::UpLeft,
            6 => Facing::Up,
            _ => Facing::UpRight,
        }
    }

    /// The fielder's frame label for running this way.
    pub fn run_label(self) -> &'static str {
        match self {
            Facing::Left => "left",
            Facing::Right => "right",
            Facing::Up => "up",
            Facing::Down => "down",
            Facing::UpLeft => "upLeft",
            Facing::UpRight => "upRight",
            Facing::DownLeft => "downLeft",
            Facing::DownRight => "downRight",
        }
    }

    /// The label for picking the ball up after running this way. The art
    /// has four of these.
    pub fn pick_label(self) -> &'static str {
        match self {
            Facing::Left | Facing::UpLeft | Facing::DownLeft => "pickLeft",
            Facing::Right | Facing::UpRight | Facing::DownRight => "pickRight",
            Facing::Up => "pickUp",
            Facing::Down => "pickDown",
        }
    }

    /// The label for throwing this way. The art throws in four directions.
    pub fn throw_label(self) -> &'static str {
        match self {
            Facing::Left | Facing::UpLeft | Facing::DownLeft => "throwLeft",
            Facing::Right | Facing::UpRight | Facing::DownRight => "throwRight",
            Facing::Up => "throwUp",
            Facing::Down => "throwDown",
        }
    }
}
