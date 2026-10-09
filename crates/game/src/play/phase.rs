//! How far a pitch has got, and how a match ended.

/// How a match ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Won,
    Lost,
    Tied,
    /// The arcade game's pitches are used up.
    ArcadeOver,
    /// In a full match, the player's side is out and the other side has
    /// batted: there is a board to read, and then more to play.
    Interval,
}

/// What stage a pitch has reached.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Phase {
    /// The batting view is being put up.
    Arriving,
    /// The pitcher stands and waits.
    Settling {
        left: u32,
    },
    WindUp,
    /// The ball is on its way.
    Flight {
        step: usize,
    },
    /// A strike or a ball has been called, and is being shown.
    Called {
        left: u32,
    },
    /// The ball is seen leaving the bat.
    Watching {
        left: u32,
    },
    /// Four balls: a moment, then the batter walks.
    Walking {
        left: u32,
    },
    /// The overhead view: the ball, the fielders and the runners.
    Fielding,
    /// The play is over and the next-ball button is up.
    Ready,
    /// The button has been pressed and the view is about to be rebuilt.
    Leaving {
        left: u32,
    },
    Over,
}
