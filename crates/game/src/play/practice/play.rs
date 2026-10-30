//! A session as the match plays it: what it starts each pitch with, what it
//! puts up in the view, and what it is told of each pitch.

use bb_engine::stage::Stage;

use super::{Seen, Sent, Session};
use crate::game::Game;
use crate::play::book::Hit;
use crate::play::field::{Contact, Happened, reach};
use crate::play::overlay::{Notices, Says};
use crate::play::runners::{Count, Runners};
use crate::play::{AtBat, Match, Parts, show};

/// What the lines a session writes across the top of the view are called on
/// the stage, how far down each is, and their colour, which is the one the
/// innings is said in. They are too long for the corner the mods write in.
const TALLY: (&str, f32) = ("practiceTally", 8.0);
const LAST: (&str, f32) = ("practiceLast", 24.0);
const COLOUR: [u8; 3] = [0xfd, 0xf6, 0xc0];

impl Match {
    /// The session being played, if this is practice.
    pub fn session(&self) -> Option<&Session> {
        self.practice.as_ref()
    }

    /// The session being played, taken away, to be reported on.
    pub fn take_session(&mut self) -> Option<Session> {
        self.practice.take()
    }

    /// Every pitch of a session is to a batter just come up, with nobody on
    /// and nobody out, whatever came of the last.
    pub(crate) fn begin_the_pitch_afresh(&mut self) {
        if self.practice.is_none() {
            return;
        }
        self.runners = Runners::default();
        self.outs = 0;
        self.score = 0;
        self.count = Count::default();
        self.announce = false;
        self.came_up = 0;
    }

    /// Writes across the top of the view what the session has come to, and
    /// what came of the last pitch.
    pub(crate) fn say_how_the_session_stands(
        &self,
        notices: &mut Notices,
        parts: &Parts,
        stage: &mut Stage,
    ) {
        let Some(session) = &self.practice else {
            return;
        };
        let lines = [(TALLY, Some(session.tally())), (LAST, session.last())];
        for ((name, down), words) in lines {
            if let Some(words) = words {
                let says = Says::line(name, &words, COLOUR).at((parts.centre_x, down));
                notices.put(says, parts, stage);
            }
        }
    }

    /// Takes off the view what would be counting something a session does
    /// not count: the boards in a match's view, and the score, the balls
    /// left and the target in the arcade game's.
    pub(crate) fn clear_the_boards(&self, parts: &Parts, stage: &mut Stage) {
        if self.practice.is_none() {
            return;
        }
        for board in [&parts.scoreboard, &parts.field_scoreboard]
            .into_iter()
            .flatten()
        {
            show(stage, board, false);
        }
        for name in ["onScreenScore", "onScreen_ballsLeft"] {
            if let Some(path) = stage.find(&parts.main, &[name]) {
                show(stage, &path, false);
            }
        }
    }

    /// A pitch has gone by unhit, swung at or not.
    pub(crate) fn tell_the_session_it_went_by(&mut self, at_bat: &mut AtBat, stage: &mut Stage) {
        let seen = Match::seen_of(at_bat, None);
        self.add_to_the_session(seen, at_bat, stage);
    }

    /// The bat has met the ball. Where it will go is worked out now, so that
    /// it counts whatever becomes of the view, and in either view alike:
    /// the arcade game offers the next pitch while the ball is still up.
    pub(crate) fn tell_the_session_it_was_hit(
        &mut self,
        at_bat: &mut AtBat,
        contact: &Contact,
        game: &Game,
        stage: &mut Stage,
    ) {
        let Some(ball) = at_bat.ball else {
            return;
        };
        let (rules, parts) = (&game.rules.field, &at_bat.parts);
        let ground = parts.ground(rules);
        let (at, came_to) = ball.first_comes_to(parts.home, contact.miss(), rules);
        let over_wall = came_to == Happened::Cleared;
        // It was in the air if it got to the wall or came down beyond the
        // infield, as a full match's book has it.
        let fly = over_wall || reach(ground.home, at) >= ground.infield;
        let feet = at_bat.zinger.map(|zinger| zinger.feet);
        let sent = Sent {
            hit: Hit::at(&ground, at, fly, feet),
            foul: parts.is_foul(contact, rules),
            over_wall,
        };
        let seen = Match::seen_of(at_bat, Some(sent));
        self.add_to_the_session(seen, at_bat, stage);
    }

    /// Writes a pitch into the session, if there is one, and says at once
    /// what it comes to.
    fn add_to_the_session(&mut self, seen: Seen, at_bat: &mut AtBat, stage: &mut Stage) {
        if let Some(session) = &mut self.practice {
            session.add(seen);
            self.say_how_the_session_stands(&mut at_bat.notices, &at_bat.parts, stage);
        }
    }

    fn seen_of(at_bat: &AtBat, sent: Option<Sent>) -> Seen {
        Seen {
            in_zone: at_bat.pitch.in_zone,
            swung: at_bat.swing.is_some(),
            off: at_bat.swing_off,
            quality: at_bat.met,
            sent,
        }
    }
}
