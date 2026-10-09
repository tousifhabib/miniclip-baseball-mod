//! The ball over the field: how it flies, bounces and meets the wall.
//!
//! Nothing here touches the art. Positions are in the overhead field's own
//! pixels, and height is in the same pixels, up from the grass.

use crate::play::pitch::Point;
use crate::rules::{FieldRules, HitRules};

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

/// The size the ball is drawn at over a point of the field, its own being
/// 1: smaller the further up the field it is.
pub fn seen_size(home: Point, at: Point) -> f32 {
    (0.6 + (at.1 - home.1) / 1000.0).max(0.1)
}

pub fn distance(a: Point, b: Point) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// The fixed points of the field that a hit is placed by.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ground {
    pub home: Point,
    /// How far down the field's picture the row is that hits are aimed
    /// along, and where on it the foul lines cross it, left and right.
    pub mark_y: f32,
    pub foul: (f32, f32),
    /// How far off the wall is, and second base, as [`reach`] measures.
    pub wall: f32,
    pub infield: f32,
    /// How many feet one of [`reach`]'s units is taken to be.
    pub feet_each: f32,
}

impl Default for Ground {
    /// The field as the art draws it.
    fn default() -> Ground {
        Ground {
            home: (240.8, 336.85),
            mark_y: 168.7,
            foul: (-54.65, 638.3),
            wall: 820.0,
            infield: 440.0,
            feet_each: 400.0 / 820.0,
        }
    }
}

impl Ground {
    /// How far across the field from home a point is: nought on the left
    /// foul line, one on the right, and outside those in foul ground.
    pub fn across(&self, at: Point) -> f32 {
        let up = self.home.1 - at.1;
        if up <= 0.0 {
            return 0.5;
        }
        let along = self.home.0 + (at.0 - self.home.0) * (self.home.1 - self.mark_y) / up;
        (along - self.foul.0) / (self.foul.1 - self.foul.0)
    }

    /// The point that far across the field and that far from home, as
    /// [`reach`] measures.
    pub fn point(&self, across: f32, far: f32) -> Point {
        let mark = (
            self.foul.0 + across * (self.foul.1 - self.foul.0),
            self.mark_y,
        );
        let way = distance(self.home, mark).max(0.001);
        let towards = ((mark.0 - self.home.0) / way, (mark.1 - self.home.1) / way);
        // Along a straight line reach grows evenly.
        let from = reach(self.home, self.home);
        let each = reach(
            self.home,
            (self.home.0 + towards.0, self.home.1 + towards.1),
        ) - from;
        let pixels = (far - from) / each;
        (
            self.home.0 + towards.0 * pixels,
            self.home.1 + towards.1 * pixels,
        )
    }

    /// How far from home a point is, in feet.
    pub fn feet(&self, at: Point) -> u32 {
        (reach(self.home, at) * self.feet_each).max(0.0).round() as u32
    }
}

impl Ball {
    /// The ball as it leaves the bat, heading for `mark`.
    pub fn hit(
        home: Point,
        mark: Point,
        contact: &Contact,
        hit: &HitRules,
        rules: &FieldRules,
    ) -> Ball {
        let frames = contact.power * rules.pace;
        Ball {
            at: home,
            speed: ((mark.0 - home.0) / frames, (mark.1 - home.1) / frames),
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
        let far = distance(home, self.at);
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

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::rules::Rules;

    const HOME: Point = (240.8, 336.85);
    const STRAIGHT: Point = (303.8, 168.7);

    fn well_hit() -> Contact {
        Contact {
            power: 14.0,
            under: 0.0,
            aside: 0.0,
        }
    }

    fn fly(contact: &Contact, mark: Point) -> (Vec<Happened>, Ball) {
        let rules = Rules::default();
        let mut ball = Ball::hit(HOME, mark, contact, &rules.hit, &rules.field);
        let mut seen = Vec::new();
        for _ in 0..900 {
            let happened = ball.step(HOME, contact.miss(), &rules.field);
            if happened != Happened::Nothing {
                seen.push(happened);
            }
        }
        (seen, ball)
    }

    #[test]
    fn a_ball_met_squarely_clears_the_wall() {
        let (seen, _) = fly(&well_hit(), STRAIGHT);
        assert_eq!(seen.first(), Some(&Happened::Cleared), "{seen:?}");
    }

    #[test]
    fn a_weak_hit_lands_in_the_field_and_stays_there() {
        let weak = Contact {
            power: 30.0,
            under: -30.0,
            aside: 0.0,
        };
        let (seen, ball) = fly(&weak, STRAIGHT);
        assert_eq!(seen.first(), Some(&Happened::Landed), "{seen:?}");
        assert!(!seen.contains(&Happened::Cleared));
        assert!(ball.bounced);
        let rules = Rules::default();
        assert!(reach(HOME, ball.at) < rules.field.wall);
    }

    #[test]
    fn the_landing_is_where_the_ball_first_comes_down() {
        let rules = Rules::default();
        let weak = Contact {
            power: 25.0,
            under: -20.0,
            aside: 0.0,
        };
        let start = Ball::hit(HOME, STRAIGHT, &weak, &rules.hit, &rules.field);
        let landing = start.landing(HOME, weak.miss(), &rules.field);
        let mut ball = start;
        while ball.step(HOME, weak.miss(), &rules.field) == Happened::Nothing {}
        assert_eq!(ball.at, landing);
        // It went up the field, away from home.
        assert!(landing.1 < HOME.1);
    }

    #[test]
    fn a_ball_sent_a_distance_comes_down_that_far_off_whichever_way_it_goes() {
        let rules = Rules::default();
        for across in [-40.0, 120.0, STRAIGHT.0, 480.0, 620.0] {
            for (frames, peak) in [(150, 100.0), (90, 30.0), (420, 300.0)] {
                let mark = (across, STRAIGHT.1);
                let mut ball = Ball::sent(HOME, mark, 600.0, frames as f32, peak);
                let (mut taken, mut highest) = (1, 0.0f32);
                while ball.step(HOME, 0.0, &rules.field) != Happened::Landed {
                    taken += 1;
                    highest = highest.max(ball.height);
                    assert!(taken < 900, "towards {across}: it never came down");
                }
                assert_eq!(taken, frames, "towards {across}");
                assert!((highest - peak).abs() < peak * 0.05, "{highest} for {peak}");
                let far = reach(HOME, ball.at);
                assert!((far - 600.0).abs() < 1.0, "towards {across}: {far}");
            }
        }
    }

    #[test]
    fn swinging_under_the_ball_lifts_it_more() {
        let rules = Rules::default();
        let level = well_hit();
        let under = Contact {
            under: 40.0,
            ..level
        };
        assert!(under.lift(&rules.hit) > level.lift(&rules.hit));
    }

    #[test]
    fn a_ball_never_turns_round_in_the_air() {
        // Far off-centre, the air takes all the ball's speed but no more.
        let mishit = Contact {
            power: 14.0,
            under: 200.0,
            aside: 0.0,
        };
        let rules = Rules::default();
        let mut ball = Ball::hit(HOME, STRAIGHT, &mishit, &rules.hit, &rules.field);
        for _ in 0..300 {
            ball.step(HOME, mishit.miss(), &rules.field);
            if !ball.walled {
                assert!(ball.speed.1 <= 0.0, "{ball:?}");
            }
        }
    }

    #[test]
    fn a_point_of_the_ground_is_found_from_how_far_across_and_how_far_off_it_is() {
        let ground = Ground::default();
        for across in [0.0, 0.2, 0.5, 0.9, 1.0] {
            for far in [150.0, 440.0, 700.0, 820.0, 950.0] {
                let at = ground.point(across, far);
                assert!((reach(ground.home, at) - far).abs() < 0.05, "{at:?}");
                assert!((ground.across(at) - across).abs() < 0.001, "{at:?}");
            }
        }
        // The wall is four hundred feet off, and second base about half
        // that.
        assert_eq!(ground.feet(ground.point(0.5, ground.wall)), 400);
        assert_eq!(ground.feet(ground.point(0.5, ground.infield)), 215);
        // Second base as the art places it is where the infield ends.
        assert!((reach(ground.home, (277.9, 215.75)) - ground.infield).abs() < 2.0);
        assert_eq!(ground.feet(ground.home), 0);
    }

    #[test]
    fn a_fielder_faces_the_way_he_is_going() {
        let from = (100.0, 100.0);
        assert_eq!(Facing::towards(from, (200.0, 100.0)), Facing::Right);
        assert_eq!(Facing::towards(from, (0.0, 100.0)), Facing::Left);
        assert_eq!(Facing::towards(from, (100.0, 0.0)), Facing::Up);
        assert_eq!(Facing::towards(from, (100.0, 200.0)), Facing::Down);
        assert_eq!(Facing::towards(from, (200.0, 0.0)), Facing::UpRight);
        assert_eq!(Facing::towards(from, (0.0, 200.0)), Facing::DownLeft);
        assert_eq!(Facing::UpLeft.pick_label(), "pickLeft");
        assert_eq!(Facing::DownRight.throw_label(), "throwRight");
    }

    proptest! {
        #[test]
        fn a_ball_sent_to_come_down_so_far_off_after_so_many_frames_does_just_that(
            // Towards anywhere between the foul lines, to come down short of
            // the wall, which would get in its way.
            across in 0.0f32..=1.0,
            carry in 100.0f32..800.0,
            frames in 4u16..=600,
            peak in 5.0f32..400.0,
        ) {
            let (rules, ground) = (Rules::default().field, Ground::default());
            let (left, right) = ground.foul;
            let mark = (left + (right - left) * across, ground.mark_y);
            let mut ball = Ball::sent(ground.home, mark, carry, f32::from(frames), peak);
            let (mut taken, mut highest) = (0, 0.0f32);
            loop {
                taken += 1;
                // Nothing happens to it on the way but coming down.
                match ball.step(ground.home, 0.0, &rules) {
                    Happened::Landed => break,
                    happened => prop_assert_eq!(happened, Happened::Nothing),
                }
                highest = highest.max(ball.height);
                prop_assert!(taken < frames, "still up after its {} frames", frames);
            }
            prop_assert_eq!(taken, frames);
            let far = reach(ground.home, ball.at);
            prop_assert!((far - carry).abs() < 1.0, "it came down {} off", far);
            // The top of its flight comes between two frames, so the
            // highest it is seen is one part short of the height it was
            // sent to, in as many parts as the frames it is up for.
            let seen = peak * (1.0 - 1.0 / f32::from(frames));
            prop_assert!((highest - seen).abs() < peak * 0.001, "it went {} high", highest);
        }

        #[test]
        fn any_point_of_the_ground_is_as_far_across_and_as_far_off_as_was_asked_for(
            across in 0.0f32..=1.0,
            far in 50.0f32..1200.0,
        ) {
            let ground = Ground::default();
            let at = ground.point(across, far);
            let (is_across, is_far) = (ground.across(at), reach(ground.home, at));
            prop_assert!((is_across - across).abs() < 0.001, "{} across", is_across);
            prop_assert!((is_far - far).abs() < 0.1, "{} off", is_far);
        }
    }
}
