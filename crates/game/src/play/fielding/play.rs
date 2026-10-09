//! What kind of play is being made in the field, and how far it has got.

use crate::play::field::Facing;
use crate::play::pitch::Point;

/// What the fielder with the ball, or going for it, is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Job {
    /// Running to where the ball will come down, or after it once it has.
    Chase,
    /// Standing under a ball still in the air.
    WaitCatch,
    PickUp {
        left: u32,
    },
    /// Drawing back to throw.
    WindUp {
        left: u32,
    },
    /// The ball is in the air between fielders.
    Throwing {
        step: Point,
    },
    /// He has let the ball go, and is at a loss for a moment before he
    /// goes after it.
    Fumbling {
        left: u32,
    },
    /// At a base, gathering a throw he dropped.
    Gather {
        left: u32,
    },
    Rest,
}

/// What kind of play is being made in the field, and how it stands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Play {
    /// Nobody hit it: the batter walks to first on four balls.
    Walk,
    /// The bat sent it outside the lines. `called` is how many frames ago
    /// that was settled.
    Foul { called: u32 },
    /// Nobody hit it: the catcher is throwing to a base that a runner is
    /// stealing. `held` once the fielder there has the ball and the play
    /// is done.
    Steal { held: bool },
    /// The bat sent it fair.
    Fair(Fair),
}

/// How a ball that was hit fair stands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Fair {
    /// In play: runners may be put out, and may go on.
    Live,
    /// A zinger is over the wall, and nothing is called until it comes
    /// down.
    Gone,
    /// Over the wall, and called a home run this many frames ago.
    HomeRun { called: u32 },
    /// A fielder at a base has it, with nobody left to throw out.
    Held,
}

/// What came of a fielder getting his glove to the ball before it came
/// down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Catch {
    Made,
    /// It was in his glove, and he let it go.
    Dropped,
}

impl Play {
    /// Whether the ball is in play: runners may be put out, and may go on.
    pub(super) fn is_live(self) -> bool {
        matches!(
            self,
            Play::Steal { held: false } | Play::Fair(Fair::Live | Fair::Gone)
        )
    }
}

pub(crate) struct Fielding {
    /// What kind of play it is, and how it stands.
    pub(super) play: Play,
    /// What came of a fielder getting his glove to the ball in the air, if
    /// one did.
    pub(super) catch: Option<Catch>,
    /// The outfielders on their way back to the wall to watch a zinger go
    /// over it, counting from 0.
    pub(super) watchers: Vec<usize>,
    /// Which fielder has the job, counting from 0.
    pub(super) fielder: usize,
    pub(super) job: Job,
    /// Where the ball will first come down.
    pub(super) land: Point,
    pub(super) facing: Facing,
    /// The base the ball is being thrown to, home being 4.
    pub(super) throw_to: u8,
    pub(super) frames: u32,
    /// The ball on the ground has been fumbled once, and will not be
    /// again before somebody has hold of it.
    pub(super) fumbled: bool,
    /// Who hit the ball: his place among the runners.
    pub(super) batter: Option<usize>,
}

impl Fielding {
    /// A play that has just begun, of this kind, with the catcher's part
    /// in it still to be said: nobody has the ball, and nothing has come
    /// of it yet.
    pub(super) fn begun(play: Play, home: Point, batter: Option<usize>) -> Fielding {
        Fielding {
            play,
            catch: None,
            watchers: Vec::new(),
            fielder: 0,
            job: Job::Rest,
            land: home,
            facing: Facing::Down,
            throw_to: 1,
            frames: 0,
            fumbled: false,
            batter,
        }
    }

    pub(super) fn is_home_run(&self) -> bool {
        matches!(self.play, Play::Fair(Fair::HomeRun { .. }))
    }

    /// Whether the bat sent the ball fair: not a foul, a walk or a steal.
    pub(super) fn is_fair(&self) -> bool {
        matches!(self.play, Play::Fair(_))
    }

    pub(super) fn was_caught(&self) -> bool {
        self.catch == Some(Catch::Made)
    }

    pub(super) fn was_dropped(&self) -> bool {
        self.catch == Some(Catch::Dropped)
    }
}
