//! One turn at bat as the book has it: each pitch, how the turn ended,
//! where a hit went, and a base stolen during it.

use crate::play::field::Ground;
use crate::play::pitch::{Point, Quality};

/// How a pitch ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Thrown {
    Ball,
    /// A strike the batter let go by.
    Called,
    /// A strike he swung at and missed.
    Swinging,
    Foul,
    InPlay,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pitch {
    pub in_zone: bool,
    pub thrown: Thrown,
    /// How many frames after the best moment for it the swing began, or
    /// before it if less than nought. Only a batter who is really played
    /// has one.
    pub off: Option<i32>,
    /// How well the bat met the ball, if it did.
    pub quality: Option<Quality>,
}

impl Pitch {
    pub fn swung(&self) -> bool {
        matches!(
            self.thrown,
            Thrown::Swinging | Thrown::Foul | Thrown::InPlay
        )
    }

    pub fn strike(&self) -> bool {
        self.thrown != Thrown::Ball
    }

    /// Whether the bat met the ball.
    pub fn met(&self) -> bool {
        matches!(self.thrown, Thrown::Foul | Thrown::InPlay)
    }
}

/// How a batter's turn ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    Strikeout,
    Walk,
    Single,
    Double,
    Triple,
    HomeRun,
    FlyOut,
    GroundOut,
    /// A fly that was caught, on which a runner came home.
    SacrificeFly,
    /// A ground ball that put two out.
    DoublePlay,
    /// Safe because a fielder dropped a catch.
    Error,
}

impl End {
    /// How many bases a hit is worth. Nought if it was not a hit.
    pub fn bases(self) -> u32 {
        match self {
            End::Single => 1,
            End::Double => 2,
            End::Triple => 3,
            End::HomeRun => 4,
            _ => 0,
        }
    }

    pub fn hit(self) -> bool {
        self.bases() > 0
    }

    /// Whether the turn counts as an at-bat: all do but a walk and a
    /// sacrifice.
    pub fn at_bat(self) -> bool {
        !matches!(self, End::Walk | End::SacrificeFly)
    }

    /// Whether the ball was put in play.
    pub fn in_play(self) -> bool {
        !matches!(self, End::Strikeout | End::Walk)
    }
}

/// Where a ball that was put in play went.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// Where it first came down, was caught, or would have come down had
    /// the wall not been in the way, in the field's own pixels.
    pub at: Point,
    /// How far across the field that is: nought on the left foul line and
    /// one on the right.
    pub across: f32,
    pub feet: u32,
    /// It was in the air: a fly or a line drive, not a ground ball.
    pub fly: bool,
    /// It went beyond the infield.
    pub deep: bool,
}

impl Hit {
    /// A ball that came down at `at`. `feet` is how far it went if that is
    /// known better than by where it came down.
    pub fn at(ground: &Ground, at: Point, fly: bool, feet: Option<u32>) -> Hit {
        Hit {
            at,
            across: ground.across(at),
            feet: feet.unwrap_or_else(|| ground.feet(at)),
            fly,
            deep: crate::play::field::reach(ground.home, at) >= ground.infield,
        }
    }

    /// The part of the field it went to, as a scorer would say it.
    pub fn place(&self) -> &'static str {
        let fifth = (self.across.clamp(0.0, 0.999) * 5.0) as usize;
        if self.deep {
            ["LEFT", "LEFT CENTRE", "CENTRE", "RIGHT CENTRE", "RIGHT"][fifth]
        } else {
            ["THIRD", "SHORT", "THE MOUND", "SECOND", "FIRST"][fifth]
        }
    }

    /// Which third of the field it went to: left, centre or right.
    pub fn third(&self) -> usize {
        (self.across.clamp(0.0, 0.999) * 3.0) as usize
    }
}

/// One batter's turn at the plate.
#[derive(Clone, Debug, PartialEq)]
pub struct Turn {
    /// The innings it was in, counting from 1.
    pub innings: u32,
    /// His place in the order, counting from nought.
    pub order: usize,
    /// How many were out when he came up, and which bases had runners on.
    pub outs: u32,
    pub on: [bool; 3],
    pub pitches: Vec<Pitch>,
    pub end: End,
    pub ball: Option<Hit>,
    /// The runs that came in on it.
    pub runs_in: u32,
    /// The outs made on it, his own and any runner's.
    pub outs_made: u32,
}

impl Turn {
    /// Whether a runner was on second or third when he came up.
    pub fn in_scoring_position(&self) -> bool {
        self.on[1] || self.on[2]
    }

    /// The turn in a line, for the list of an innings: who, what he did,
    /// what came of it, and how many pitches he saw.
    pub fn words(&self) -> String {
        let place = self.ball.map_or("", |ball| ball.place());
        let feet = self.ball.map_or(0, |ball| ball.feet);
        let looking = self.pitches.last().map(|pitch| pitch.thrown) == Some(Thrown::Called);
        let mut what = match self.end {
            End::Strikeout if looking => "STRUCK OUT LOOKING".to_owned(),
            End::Strikeout => "STRUCK OUT SWINGING".to_owned(),
            End::Walk => "WALKED".to_owned(),
            End::Single => format!("SINGLE TO {place}"),
            End::Double => format!("DOUBLE TO {place}"),
            End::Triple => format!("TRIPLE TO {place}"),
            End::HomeRun => format!("HOME RUN TO {place}, {feet} FT"),
            End::FlyOut => format!("FLEW OUT TO {place}"),
            End::GroundOut => format!("GROUNDED OUT TO {place}"),
            End::SacrificeFly => format!("SACRIFICE FLY TO {place}"),
            End::DoublePlay => format!("DOUBLE PLAY TO {place}"),
            End::Error => format!("SAFE ON AN ERROR AT {place}"),
        };
        // An out that was not the batter's own, or not his alone.
        let own = match self.end {
            End::DoublePlay => 2,
            End::Strikeout | End::FlyOut | End::GroundOut | End::SacrificeFly => 1,
            _ => 0,
        };
        if self.outs_made > own {
            what += ", RUNNER OUT";
        }
        let runs = match self.runs_in {
            0 => String::new(),
            1 => ", 1 RUN".to_owned(),
            runs => format!(", {runs} RUNS"),
        };
        format!("{} {what}{runs} ({})", self.order + 1, self.pitches.len())
    }
}

/// A runner's try at stealing a base.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Steal {
    /// The innings it was in, counting from 1.
    pub innings: u32,
    /// The runner's place in the order, counting from nought.
    pub order: usize,
    /// The base he went for: 2 for second, 3 for third.
    pub base: u8,
    /// Whether he got there. If not he was thrown out.
    pub safe: bool,
    /// How many of his side's turns were over when he went, which is where
    /// among them it is told.
    pub at: usize,
}

impl Steal {
    /// The try in a line, for the list of an innings.
    pub fn words(&self) -> String {
        let base = match self.base {
            2 => "SECOND",
            3 => "THIRD",
            _ => "HOME",
        };
        let what = if self.safe {
            "STOLE"
        } else {
            "CAUGHT STEALING"
        };
        format!("{} {what} {base}", self.order + 1)
    }
}
