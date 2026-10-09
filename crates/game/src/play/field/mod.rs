//! The ball over the field: how it flies, bounces and meets the wall.
//!
//! Nothing here touches the art. Positions are in the overhead field's own
//! pixels, and height is in the same pixels, up from the grass.
//!
//! The field is drawn at a slant, with right field across more of the
//! screen than left and either across more than the way up the middle. So
//! how far the ball has gone is never counted in pixels of the screen but
//! as [`reach`] has it, which is what puts the wall as far off one way as
//! another. A hit is given its speed and held back by the air by that
//! measure, and so goes as far whichever way it is sent. The original
//! counted both by the screen: a ball went up the screen at one pace
//! whichever way it was sent, which carried one sent to either side
//! further over the field, and was held back by how much of the screen it
//! had crossed, which to right field cost it more than it had gained.

mod facing;
mod ground;
#[cfg(test)]
mod properties;

use crate::play::pitch::Point;
use crate::rules::{FieldRules, HitRules};
pub use facing::Facing;
pub use ground::Ground;

/// What the bat did to the ball.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact {
    /// From the swing's timing. A lower power sends the ball further.
    pub power: f32,
    /// How far below the ball the ring was, in pixels. Above is negative.
    pub under: f32,
    /// How far to the side of straight the ball is sent, in pixels of the
    /// batting view.
    pub aside: f32,
}

impl Contact {
    /// How far off the ball's height the ring was, either way.
    pub fn miss(&self) -> f32 {
        self.under.abs()
    }

    /// The mark over the field that the ball heads for, `straight` being
    /// the one for a hit sent to neither side.
    pub fn heads_for(&self, straight: Point, rules: &FieldRules) -> Point {
        (straight.0 + self.aside / rules.aim_share, straight.1)
    }

    /// How hard the ball leaves the bat upwards, in the batting view.
    pub fn lift(&self, rules: &HitRules) -> f32 {
        rules.lift + self.under / rules.lift_aim - self.power / rules.power_drag
    }
}

/// What happened to the ball on one frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Happened {
    Nothing,
    /// It came down and bounced.
    Landed,
    /// It reached the wall too low, and came back off it.
    HitWall,
    /// It cleared the wall: a home run, if the hit was fair.
    Cleared,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ball {
    pub at: Point,
    pub speed: Point,
    /// How far above the grass.
    pub height: f32,
    /// How fast it is going up. Negative coming down.
    pub lift: f32,
    /// How much of that it loses each frame.
    pub fall: f32,
    pub bounced: bool,
    /// It has been to the wall, one way or the other.
    pub walled: bool,
}

/// How far a point is from home plate as the game measures a hit: the plain
/// distance, allowing for the field being drawn at a slant.
pub fn reach(home: Point, at: Point) -> f32 {
    let plain = ((at.0 - home.0).powi(2) + (at.1 - home.1).powi(2)).sqrt();
    plain - ((at.1 - home.1) * 3.0 + at.0 / 5.6)
}

/// How much of the field lies between home and a point: [`reach`] counted
/// from home, which is not where `reach` itself is nought. Along a straight
/// line from home it grows evenly.
pub fn covered(home: Point, at: Point) -> f32 {
    reach(home, at) - reach(home, home)
}

/// The size the ball is drawn at over a point of the field, its own being
/// 1: smaller the further up the field it is.
pub fn seen_size(home: Point, at: Point) -> f32 {
    (0.6 + (at.1 - home.1) / 1000.0).max(0.1)
}

pub fn distance(a: Point, b: Point) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

impl Ball {
    /// The ball as it leaves the bat. `straight` is the mark that a hit sent
    /// to neither side heads for, which it would come to in as many frames
    /// as the rules give its power. Sent to one side it heads for a mark of
    /// its own, and covers as much of the field in a frame as it would have
    /// going straight, however much of the screen that is.
    pub fn hit(
        home: Point,
        straight: Point,
        contact: &Contact,
        hit: &HitRules,
        rules: &FieldRules,
    ) -> Ball {
        let frames = contact.power * rules.pace;
        let mark = contact.heads_for(straight, rules);
        let way = distance(home, mark).max(0.001);
        let towards = ((mark.0 - home.0) / way, (mark.1 - home.1) / way);
        // What a pixel that way is worth, which is never nothing: the mark
        // is up the field from home.
        let each = covered(home, (home.0 + towards.0, home.1 + towards.1));
        let pace = covered(home, straight) / each / frames;
        Ball {
            at: home,
            speed: (towards.0 * pace, towards.1 * pace),
            height: 0.0,
            lift: contact.lift(hit) * rules.lift_share,
            fall: rules.gravity,
            bounced: false,
            walled: false,
        }
    }

    /// A ball sent off towards `mark` to come down `carry` from home, as
    /// [`reach`] measures it, after `frames` in the air, going `peak` high
    /// on the way.
    pub fn sent(home: Point, mark: Point, carry: f32, frames: f32, peak: f32) -> Ball {
        // It takes a few frames to go up and come down at all.
        let frames = frames.max(4.0);
        let way = distance(home, mark).max(0.001);
        let towards = ((mark.0 - home.0) / way, (mark.1 - home.1) / way);
        // Along a straight line reach grows evenly, so its first pixel
        // tells how many pixels the whole carry is.
        let from = reach(home, home);
        let each = reach(home, (home.0 + towards.0, home.1 + towards.1)) - from;
        let pace = (carry - from) / each / frames;
        // What takes a ball that high and back in that many frames.
        let fall = 8.0 * peak / (frames * frames);
        Ball {
            at: home,
            speed: (towards.0 * pace, towards.1 * pace),
            height: 0.0,
            // All of this has been lost between the last frame but one and
            // the last, which is the frame it comes down on.
            lift: fall * (frames - 1.5) / 2.0,
            fall,
            bounced: false,
            walled: false,
        }
    }

    /// Moves the ball on by one frame. `miss` is how far off the ball's
    /// height the ring was: a ball hit off-centre is slowed by the air more.
    pub fn step(&mut self, home: Point, miss: f32, rules: &FieldRules) -> Happened {
        self.at.0 += self.speed.0;
        self.at.1 += self.speed.1;
        // How much of the field it has covered, and not how much of the
        // screen it has crossed, so that the air holds it back alike
        // whichever way it has gone.
        let far = covered(home, self.at);
        // Never more than all of it, or a mishit far from home would turn
        // round in the air.
        let lost = (rules.drag * (miss / rules.drag_aim) * (far * far / rules.drag_reach)).min(1.0);
        self.speed.0 -= self.speed.0 * lost;
        self.speed.1 -= self.speed.1 * lost;
        self.height += self.lift;
        self.lift -= self.fall;

        let mut happened = Happened::Nothing;
        if self.height < 0.0 {
            self.height = -self.height;
            self.lift = (-self.lift * rules.bounce_lift).min(rules.bounce_cap) - rules.bounce_loss;
            self.speed.0 *= rules.bounce_run;
            self.speed.1 *= rules.bounce_run;
            self.bounced = true;
            happened = Happened::Landed;
        }
        if !self.walled && reach(home, self.at) >= rules.wall {
            self.walled = true;
            if self.height > rules.clear {
                return Happened::Cleared;
            }
            self.speed.0 *= -rules.wall_bounce;
            self.speed.1 *= -rules.wall_bounce;
            return Happened::HitWall;
        }
        happened
    }

    /// Where the ball will first come down, if it is left alone.
    pub fn landing(&self, home: Point, miss: f32, rules: &FieldRules) -> Point {
        let mut ball = *self;
        for _ in 0..600 {
            match ball.step(home, miss, rules) {
                Happened::Nothing => {}
                // Wherever it stops being in the air over the field.
                _ => break,
            }
        }
        ball.at
    }
}

#[cfg(test)]
mod tests;
