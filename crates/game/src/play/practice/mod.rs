//! A session of batting practice: the pitches seen, how each swing was timed
//! and where each ball went, and what they all come to.
//!
//! Practice adds no kind of game. A session is an arcade game or a last
//! innings with one of these beside it, so that the batting is the game's
//! own. Nothing here touches the stage. What is done on it is in `play`.

mod play;

use crate::play::book::{Hit, percent};
use crate::play::pitch::Quality;

/// How many frames off the best moment a swing began, how well it met the
/// ball and whether it did: what the page of timing is drawn from.
pub(crate) type TimedSwing = (i32, Option<Quality>, bool);

/// Where a ball the bat met went.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Sent {
    /// Where it first came down or came to the wall, and how far that is.
    pub hit: Hit,
    pub foul: bool,
    pub over_wall: bool,
}

/// One pitch of a session, and what came of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Seen {
    pub in_zone: bool,
    pub swung: bool,
    /// How many frames after the best moment the swing began, or before it
    /// if less than nought. A pitch no swing could meet has no best moment.
    pub off: Option<i32>,
    /// How well the bat met the ball, if it did.
    pub quality: Option<Quality>,
    pub sent: Option<Sent>,
}

/// A session of batting practice.
#[derive(Clone, Debug, PartialEq)]
pub struct Session {
    /// Whether the view the session is played on has a strike zone, without
    /// which nothing can be said of what was let go by.
    has_a_zone: bool,
    seen: Vec<Seen>,
}

impl Session {
    pub(crate) fn new(has_a_zone: bool) -> Session {
        Session {
            has_a_zone,
            seen: Vec::new(),
        }
    }

    pub(crate) fn add(&mut self, seen: Seen) {
        self.seen.push(seen);
    }

    /// How many pitches have been seen through to what came of them.
    pub fn pitches(&self) -> u32 {
        self.seen.len() as u32
    }

    pub fn swings(&self) -> u32 {
        self.count(|seen| seen.swung)
    }

    /// How many swings met the ball.
    pub fn met(&self) -> u32 {
        self.count(|seen| seen.quality.is_some())
    }

    /// How many swings met the ball as well as a swing can.
    pub fn sweet(&self) -> u32 {
        self.count(|seen| seen.quality == Some(Quality::Good))
    }

    /// The longest fair ball of the session, in feet.
    pub fn longest(&self) -> Option<u32> {
        self.fair().map(|sent| sent.hit.feet).max()
    }

    fn count(&self, wanted: impl Fn(&Seen) -> bool) -> u32 {
        self.seen.iter().filter(|seen| wanted(seen)).count() as u32
    }

    /// The balls that were hit fair.
    pub(crate) fn fair(&self) -> impl Iterator<Item = Sent> + '_ {
        self.sent().filter(|sent| !sent.foul)
    }

    /// The balls the bat met, fair or not.
    pub(crate) fn sent(&self) -> impl Iterator<Item = Sent> + '_ {
        self.seen.iter().filter_map(|seen| seen.sent)
    }

    /// The swings there is a moment to be early or late for.
    pub(crate) fn timed(&self) -> Vec<TimedSwing> {
        self.seen
            .iter()
            .filter(|seen| seen.swung)
            .filter_map(|seen| Some((seen.off?, seen.quality, seen.quality.is_some())))
            .collect()
    }

    /// The first line in the corner of the view: what the session has come
    /// to.
    pub(crate) fn tally(&self) -> String {
        let longest = self
            .longest()
            .map_or(String::new(), |feet| format!("  BEST {feet} FT"));
        format!(
            "PITCHES {}  MET {}  SWEET {}{longest}",
            self.pitches(),
            self.met(),
            self.sweet()
        )
    }

    /// The second line there: what came of the last pitch. There is none
    /// before the first.
    pub(crate) fn last(&self) -> Option<String> {
        let seen = self.seen.last()?;
        let what = match seen.sent {
            Some(sent) if sent.foul => "FOUL".to_owned(),
            Some(sent) if sent.over_wall => format!("{} FT OVER THE WALL", sent.hit.feet),
            Some(sent) => format!("{} FT TO {}", sent.hit.feet, sent.hit.place()),
            None if seen.swung => "MISSED".to_owned(),
            None if !self.has_a_zone => "LET GO BY".to_owned(),
            None if seen.in_zone => "LET GO BY, A STRIKE".to_owned(),
            None => "LET GO BY, A BALL".to_owned(),
        };
        Some(match seen.off.filter(|_| seen.swung) {
            Some(off) => format!("LAST {what}, {}", when(off)),
            None => format!("LAST {what}"),
        })
    }

    /// What the report says of where the balls went, from the top down.
    pub(crate) fn sent_lines(&self) -> Vec<String> {
        let fair: Vec<Sent> = self.fair().collect();
        let fouls = self.sent().filter(|sent| sent.foul).count();
        let over = fair.iter().filter(|sent| sent.over_wall).count();
        let of_the_fair =
            |part: usize| percent((!fair.is_empty()).then(|| part as f32 / fair.len() as f32));
        let to = |third: usize| {
            of_the_fair(fair.iter().filter(|sent| sent.hit.third() == third).count())
        };
        let up = fair.iter().filter(|sent| sent.hit.fly).count();
        let feet: u32 = fair.iter().map(|sent| sent.hit.feet).sum();
        let usual = match fair.len() as u32 {
            0 => "-".to_owned(),
            balls => format!("{} FT", feet / balls),
        };
        let longest = self
            .longest()
            .map_or("-".to_owned(), |feet| format!("{feet} FT"));
        vec![
            format!("OVER THE WALL {over}"),
            format!("IN PLAY {}", fair.len() - over),
            format!("FOUL {fouls}"),
            format!("TO LEFT {}", to(0)),
            format!("TO CENTRE {}", to(1)),
            format!("TO RIGHT {}", to(2)),
            format!("IN THE AIR {up}"),
            format!("ON THE GROUND {}", fair.len() - up),
            format!("LONGEST {longest}"),
            format!("USUALLY {usual}"),
        ]
    }

    /// What the report says of what was let go by and what was swung at:
    /// the pitches let go by, how many of them were balls and how many
    /// strikes, and the balls that were swung at. A view with no strike
    /// zone has nothing to say of it.
    pub(crate) fn taking_line(&self) -> Option<String> {
        if !self.has_a_zone {
            return None;
        }
        let taken = self.pitches() - self.swings();
        let balls = self.count(|seen| !seen.swung && !seen.in_zone);
        let chased = self.count(|seen| seen.swung && !seen.in_zone);
        Some(format!(
            "LET GO BY {taken}: BALLS {balls}, STRIKES {}   SWUNG AT BALLS {chased}",
            taken - balls
        ))
    }
}

/// When a swing was, by how many frames off the best moment.
fn when(off: i32) -> String {
    let frames = off.unsigned_abs();
    let plural = if frames == 1 { "" } else { "S" };
    match off {
        0 => "ON TIME".to_owned(),
        ..0 => format!("{frames} FRAME{plural} EARLY"),
        _ => format!("{frames} FRAME{plural} LATE"),
    }
}

#[cfg(test)]
mod tests;
