//! What the mods say about the runners: when they may go, how fast, and
//! a base being stolen.

use super::ModsInPlay;
use crate::look::Rgb;
use crate::play::mods::turbo_runners::TurboRunners;

impl ModsInPlay {
    /// Whether a runner on a base may be sent on at any time the ball is
    /// in play, and need not wait for it to come down or be caught.
    pub fn runners_may_go_at_any_time(&self) -> bool {
        self.turbo_runners.is_some()
    }

    /// How many frames more than the usual one the runners are moved on by
    /// this frame.
    pub fn hurry_the_runners(&mut self) -> u16 {
        self.turbo_runners.as_mut().map_or(0, TurboRunners::hurry)
    }

    /// Whether runners may be sent to steal a base.
    pub fn runners_steal(&self) -> bool {
        self.stolen_bases.is_some()
    }

    /// Says whether the play in the field is one on which a base can be
    /// stolen.
    pub fn a_steal_is_in_play(&mut self, is: bool) {
        if let Some(steals) = &mut self.stolen_bases {
            steals.in_play = is;
        }
    }

    /// Whether the play in the field is one on which a base can be stolen.
    pub fn is_a_steal_in_play(&self) -> bool {
        self.stolen_bases
            .as_ref()
            .is_some_and(|steals| steals.in_play)
    }

    /// A runner who was stealing got there, or was put out on his way.
    pub fn a_steal_came_out(&mut self, safe: bool) {
        if let Some(steals) = &mut self.stolen_bases {
            steals.came_out(safe);
        }
    }

    /// How the last steal came out, the first time it is asked for: the
    /// words to say, and their colour.
    pub fn news_of_a_steal(&mut self) -> Option<(&'static str, Rgb)> {
        self.stolen_bases.as_mut()?.to_tell.take()
    }

    /// How many bases have been stolen in this game, and how many runners
    /// caught at it.
    pub fn steals(&self) -> (u32, u32) {
        self.stolen_bases
            .as_ref()
            .map_or((0, 0), |steals| (steals.stolen, steals.caught))
    }
}
