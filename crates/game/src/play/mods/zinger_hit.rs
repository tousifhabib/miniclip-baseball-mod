//! The zinger hit: a mod that makes a home run of every ball the bat meets.
//!
//! A swing still has to be timed to meet the ball at all. What its timing
//! no longer decides is whether the ball gets out of the ground, only by how
//! much: the nearer the swing came to the best frame of its window, the
//! further beyond the wall the ball comes down. Holding the ring on the ball
//! adds a little more. Holding it below the ball skies the hit, which then
//! hangs in the air a long time before it drops, and holding it above
//! drives the hit low and fast. To one side or the other still sends the
//! ball that way, but a hit that would have gone foul is kept inside the
//! line.
//!
//! The home run is called when the ball comes down, not when it crosses the
//! wall. Until then it is drawn large with a trail behind it, a marker
//! shows where it will land, and the distance it has gone is counted up.

use std::collections::VecDeque;

use bb_engine::display::{Path, child_bounds};
use bb_engine::library::Library;
use bb_engine::math::Matrix;
use bb_engine::stage::Stage;

use crate::art;
use crate::look::{self, Rgb};
use crate::play::field::{Ball, Contact, Happened, distance, reach, seen_size};
use crate::play::overlay::{self, Words};
use crate::play::pitch::{Point, nearness};
use crate::play::{Parts, put};
use crate::rules::{FieldRules, HitRules, PitchRules, Rules, ZingerRules};
use crate::settings::Difficulty;

/// The mod, in play. What a zinger is made of is below; the match keeps the
/// longest of the game and the record it has to beat.
pub(crate) struct ZingerHit;

/// How far inside a foul line a zinger is kept, in pixels of the field
/// where the lines are marked.
const INSIDE: f32 = 12.0;
/// Frames of the view of the field for which a zinger is still inside the
/// wall, at the least, so that it is seen to go over.
const SEEN: f32 = 20.0;
/// How many times the height of the wall a zinger is over it by, at the
/// least, however low it was driven.
const CLEAR_BY: f32 = 1.6;

/// How far down the view of the field the distance is written, which is
/// just under where the home-run banner comes, with the size of its
/// lettering, its own being 1. Under it are the name of where the ball
/// went, and word of a record.
const FEET_TOP: f32 = 230.0;
const FEET_SIZE: f32 = 1.5;
const PLACE_TOP: f32 = 259.0;
const RECORD_TOP: f32 = 276.0;
const SMALL_SIZE: f32 = 0.8;
const COUNTING: Rgb = [0xff, 0xff, 0xff];
const GOLD: Rgb = [0xff, 0xe2, 0x4a];

/// The dots that trail behind the ball: how many, and how many frames
/// apart.
const TRAIL: usize = 8;
const TRAIL_EVERY: usize = 3;
/// How many times the size of the ball's own shadow the mark of where it
/// will land is, between its beats.
const MARKER_SIZE: f32 = 6.0;

/// A ball hit for a zinger.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zinger {
    /// How near the best the swing was timed, from 0 to 1.
    pub timed: f32,
    /// How near the ball the ring was held, from 0 to 1.
    pub aimed: f32,
    /// How far the ring's height made the hit a skied one, up to 1, or one
    /// driven low, down to -1. A ball hit level is 0.
    pub shape: f32,
    /// How far from home the ball comes down, as the field measures it, and
    /// the same with the distance to the wall as 1.
    pub carry: f32,
    pub walls: f32,
    /// The same in feet, which is what the player is told.
    pub feet: u32,
    /// Frames the ball is in the air, and how high it goes.
    pub hang: f32,
    pub peak: f32,
    /// The power it leaves the bat with in the batting view, and how far
    /// off the ball's height the ring counts as having been there.
    pub power: f32,
    tilt: f32,
}

impl Zinger {
    /// The zinger made by a swing that meets the ball this many frames
    /// after it began. `None` if it does not meet it. `ring` is how far to
    /// the right of the ball and how far below it the ring was held, and
    /// `home` is where the ball sets off from over the field.
    pub fn of(
        table: &PitchRules,
        frames_since_swing: u32,
        ring: Point,
        home: Point,
        difficulty: Difficulty,
        rules: &Rules,
    ) -> Option<Zinger> {
        let timed = nearness(table, frames_since_swing)?;
        let zinger = &rules.zinger;
        let aimed = 1.0 - (distance((0.0, 0.0), ring) / zinger.aim_reach.max(0.001)).min(1.0);
        let (worst, best) = (zinger.carry_worst, zinger.carry_best.at(difficulty));
        let walls = worst + (best - worst - zinger.aim) * timed + zinger.aim * aimed;
        let carry = rules.field.wall * walls;

        let tilt = ring.1.clamp(-zinger.shape_reach, zinger.shape_reach);
        let shape = tilt / zinger.shape_reach.max(0.001);
        let (full, by) = if shape >= 0.0 {
            (zinger.sky, shape)
        } else {
            (zinger.drive, -shape)
        };
        let blend = |level: f32, full: f32| level + (full - level) * by;
        // How much of its flight is behind it as it comes to the wall.
        let from = reach(home, home);
        let inside = ((rules.field.wall - from) / (carry - from)).clamp(0.05, 0.95);
        Some(Zinger {
            timed,
            aimed,
            shape,
            carry,
            walls,
            feet: (zinger.wall_feet * walls).round() as u32,
            // A whole number of frames, so that it comes down where it was
            // sent and not a part of a frame further on, and enough of
            // them that the view has changed to the field before it is
            // over the wall.
            hang: (zinger.hang.at(timed) * blend(1.0, full.hang))
                .max((rules.hit.watch as f32 + SEEN) / inside)
                .round(),
            peak: blend(zinger.level_peak, full.peak)
                .max(CLEAR_BY * rules.field.clear / (4.0 * inside * (1.0 - inside))),
            power: zinger.power.at(timed),
            tilt,
        })
    }

    /// What the bat did to the ball: met it level as far as the air's drag
    /// on it goes, wherever the ring was.
    pub fn contact(&self, aside: f32) -> Contact {
        Contact {
            power: self.power,
            under: 0.0,
            aside,
        }
    }

    /// How hard the ball leaves the bat upwards in the batting view, where
    /// a skied hit is seen to go up steeply.
    pub fn lift(&self, rules: &HitRules) -> f32 {
        Contact {
            power: self.power,
            under: self.tilt,
            aside: 0.0,
        }
        .lift(rules)
    }

    /// The ball over the field, on its way towards `mark`.
    pub fn ball(&self, home: Point, mark: Point) -> Ball {
        Ball::sent(home, mark, self.carry, self.hang, self.peak)
    }

    /// The sounds of the hit: the bat, and what the crowd makes of it. The
    /// better it was timed the more they make of it.
    pub fn hit_sounds(&self) -> (&'static str, &'static [&'static str]) {
        if self.timed >= 1.0 {
            ("batHit_good", &["crowd_bigClap", "crowd_smallCheer"])
        } else if self.timed >= 0.5 {
            ("batHit_good", &["crowd_bigClap"])
        } else {
            ("batHit_medium", &["crowd_smallCheer"])
        }
    }
}

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

/// A zinger on its way over the field, as the player is shown it.
pub(crate) struct Show {
    pub zinger: Zinger,
    /// Where it will come down, and what that place is.
    pub landing: Point,
    pub place: Place,
    /// The distance, the name of the place, and word of a record.
    feet: Words,
    named: Words,
    record: Words,
    /// The clip under the ball that the marker and the trail are in.
    under: Option<Path>,
    marker: Option<Path>,
    trail: Vec<Path>,
    /// Where the ball has been drawn, the latest first.
    past: VecDeque<Point>,
    frames: u32,
    /// How many times its usual size the ball is drawn.
    large: f32,
    /// Feet for each unit the field measures distance in.
    feet_each: f32,
    home: Point,
}

impl Show {
    /// Gets ready to show `ball`, a zinger, as the view changes to the
    /// field.
    pub fn new(
        zinger: Zinger,
        ball: &Ball,
        parts: &Parts,
        rules: &Rules,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<Show> {
        // It is followed past the wall to where it comes down.
        let mut landing = *ball;
        for _ in 0..10_000 {
            if landing.bounced || landing.step(parts.home, 0.0, &rules.field) == Happened::Landed {
                break;
            }
        }
        let scoreboard = parts
            .field_scoreboard
            .as_ref()
            .and_then(|board| stage.child(board))
            .and_then(|board| child_bounds(board, Matrix::IDENTITY, library));
        let place = Place::of(landing.at, zinger.walls, scoreboard, &rules.zinger);

        let holder = overlay::holder(parts, "zinger", stage, library)?;
        let mut words = |depth: u16, name: &str, top: f32, size: f32| {
            let top = (parts.centre_x, top);
            Words::new(&holder, depth, name, top, size, stage, library)
        };
        let feet = words(1, "zingerFeet", FEET_TOP, FEET_SIZE)?;
        let named = words(3, "zingerPlace", PLACE_TOP, SMALL_SIZE)?;
        let record = words(5, "zingerRecord", RECORD_TOP, SMALL_SIZE)?;

        let mut show = Show {
            zinger,
            landing: landing.at,
            place,
            feet,
            named,
            record,
            under: None,
            marker: None,
            trail: Vec::new(),
            past: VecDeque::new(),
            frames: 0,
            large: rules.zinger.ball_size.max(1.0),
            feet_each: rules.zinger.wall_feet / rules.field.wall,
            home: parts.home,
        };
        show.lay_under(parts, stage, library);
        Some(show)
    }

    /// Puts the marker and the dots of the trail under the ball. They are
    /// the ball's own shadow and the ball's own picture.
    fn lay_under(&mut self, parts: &Parts, stage: &mut Stage, library: &Library) -> Option<()> {
        let (&ball, _) = parts.field_ball.split_last()?;
        let field = stage.clip(&parts.field)?;
        let depth = (1..ball)
            .rev()
            .find(|depth| !field.children.contains_key(depth))?;
        let shadow = stage
            .find(&parts.field_ball, &["ballShadow"])
            .and_then(|shadow| stage.child(&shadow))
            .map(|shadow| shadow.symbol);
        let picture = stage.child(&parts.field_ball_inner)?.symbol;
        let under = stage.attach(&parts.field, art::HOLDER, depth, "zingerTrail", library)?;
        if let Some(shadow) = shadow {
            self.marker = stage.attach(&under, shadow, 1, "zingerMarker", library);
        }
        for dot in 0..TRAIL {
            let depth = 2 + dot as u16;
            let Some(path) = stage.attach(&under, picture, depth, "zingerDot", library) else {
                break;
            };
            if let Some(dot) = stage.child_mut(&path) {
                dot.set_visible(false);
            }
            self.trail.push(path);
        }
        self.under = Some(under);
        Some(())
    }

    /// Draws one frame of the ball in the air: the ball itself, large, the
    /// trail behind it, the marker where it will land, and how far it has
    /// gone.
    pub fn follow(&mut self, ball: &Ball, parts: &Parts, stage: &mut Stage) {
        self.frames += 1;
        let size = seen_size(parts.home, ball.at);
        // The ball and its shadow are drawn large, but no higher off the
        // ground for it.
        put(stage, &parts.field_ball, ball.at, size * self.large);
        let mut across = 0.0;
        if let Some(inner) = stage.child_mut(&parts.field_ball_inner) {
            across = inner.matrix.tx;
            inner.move_to(across, -ball.height / self.large);
        }
        let drawn = (
            ball.at.0 + across * size * self.large,
            ball.at.1 - ball.height * size,
        );
        self.past.push_front(drawn);
        self.past.truncate(TRAIL * TRAIL_EVERY + 1);
        for (index, path) in self.trail.iter().enumerate() {
            let Some(dot) = stage.child_mut(path) else {
                continue;
            };
            let Some(&at) = self.past.get((index + 1) * TRAIL_EVERY) else {
                dot.set_visible(false);
                continue;
            };
            // Each dot is smaller and fainter than the one before it.
            let fade = 1.0 - (index + 1) as f32 / (TRAIL + 1) as f32;
            let scale = size * self.large * (0.4 + 0.6 * fade);
            dot.set_matrix(Matrix {
                a: scale,
                d: scale,
                tx: at.0,
                ty: at.1,
                ..Matrix::IDENTITY
            });
            dot.set_alpha(0.6 * fade);
            dot.set_visible(true);
        }
        if let Some(marker) = self.marker.as_ref().and_then(|path| stage.child_mut(path)) {
            // It beats, to be seen against the crowd.
            let beat = 1.0 + 0.25 * (self.frames as f32 * 0.2).sin();
            let scale = seen_size(parts.home, self.landing) * MARKER_SIZE * beat;
            marker.set_matrix(Matrix {
                a: scale,
                d: scale,
                tx: self.landing.0,
                ty: self.landing.1,
                ..Matrix::IDENTITY
            });
            marker.set_color(look::tint(GOLD));
        }
        let gone = (reach(self.home, ball.at) * self.feet_each).max(0.0) as u32;
        let text = format!("{} FT", gone.min(self.zinger.feet));
        self.feet.say(&text, COUNTING, stage);
    }

    /// The ball has come down: the count stops on how far it went, and the
    /// place it went to is named.
    pub fn landed(&mut self, record: bool, stage: &mut Stage) {
        if let Some(under) = self.under.take() {
            stage.remove(&under);
        }
        self.marker = None;
        self.trail.clear();
        let text = format!("{} FT", self.zinger.feet);
        self.feet.say(&text, GOLD, stage);
        self.named.say(self.place.words(), COUNTING, stage);
        if record {
            self.record.say("NEW RECORD", GOLD, stage);
        }
    }
}

/// The nearest place to `aim`, where the art's pointer shows a hit going,
/// that sends the ball fair with room to spare.
pub(crate) fn fair(aim: f32, parts: &Parts, rules: &FieldRules) -> f32 {
    // Where the pointer is for a hit that goes this far across the field.
    let pointer = |across: f32| parts.centre_x + (across - parts.field_mark.0) * rules.aim_share;
    aim.max(pointer(parts.foul.0 + INSIDE))
        .min(pointer(parts.foul.1 - INSIDE))
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::play::field::Happened;
    use crate::play::pitch::meets;

    /// The fixed points of the field as they are in the game's art: home,
    /// how far up the field a hit is aimed, and the two foul lines there.
    const HOME: Point = (240.8, 336.85);
    const MARK: f32 = 168.7;
    const FOUL: (f32, f32) = (-54.65, 638.3);

    const LEVELS: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard];
    /// Where the ring might be held: on the ball, well above it, well
    /// below it, off to one side, and a little out every way.
    const RINGS: [Point; 6] = [
        (0.0, 0.0),
        (0.0, -90.0),
        (0.0, 90.0),
        (70.0, 0.0),
        (-20.0, 25.0),
        (15.0, -30.0),
    ];

    fn zinger(difficulty: Difficulty, frames: u32, ring: Point, rules: &Rules) -> Zinger {
        let table = rules.pitch.at(difficulty);
        Zinger::of(table, frames, ring, HOME, difficulty, rules).unwrap()
    }

    /// Every kind of zinger the rules allow: one for each frame of each
    /// level's window, with the ring held each of those ways.
    fn zingers(rules: &Rules) -> Vec<Zinger> {
        let mut all = Vec::new();
        for difficulty in LEVELS {
            for &(frames, ..) in &rules.pitch.at(difficulty).window {
                for ring in RINGS {
                    all.push(zinger(difficulty, frames, ring, rules));
                }
            }
        }
        all
    }

    /// Places across the field to send a ball, from one foul line to the
    /// other.
    fn across() -> impl Iterator<Item = f32> {
        (0..=20).map(|step| FOUL.0 + (FOUL.1 - FOUL.0) * step as f32 / 20.0)
    }

    /// Flies a zinger towards `across`. Returns the frame on which it went
    /// over the wall, the frame on which it came down, and where that was.
    fn fly(zinger: &Zinger, across: f32, rules: &Rules) -> (u32, u32, Point) {
        let mut ball = zinger.ball(HOME, (across, MARK));
        let (mut frames, mut over) = (0, None);
        loop {
            frames += 1;
            assert!(frames < 5000, "{zinger:?} never came down");
            match ball.step(HOME, 0.0, &rules.field) {
                Happened::Nothing => {}
                Happened::Cleared => over = Some(frames),
                Happened::HitWall => panic!("{zinger:?} towards {across} hit the wall"),
                Happened::Landed => {
                    let over = over.unwrap_or_else(|| panic!("{zinger:?} fell short"));
                    return (over, frames, ball.at);
                }
            }
        }
    }

    #[test]
    fn a_miss_is_no_zinger() {
        let rules = Rules::default();
        let easy = rules.pitch.at(Difficulty::Easy);
        let of = |frames| Zinger::of(easy, frames, (0.0, 0.0), HOME, Difficulty::Easy, &rules);
        assert_eq!(of(6), None);
        assert!(of(7).is_some());
    }

    #[test]
    fn every_zinger_clears_the_wall_wherever_it_is_sent_and_however_it_was_hit() {
        let rules = Rules::default();
        for zinger in zingers(&rules) {
            for across in across() {
                let (over, down, at) = fly(&zinger, across, &rules);
                // The view has changed to the field by the time it goes
                // over, so that it is seen to go.
                assert!(over > rules.hit.watch, "{zinger:?} towards {across}");
                assert!(down > over);
                // It comes down where it was sent, and well inside the
                // longest a play may last.
                let far = reach(HOME, at);
                assert!((far - zinger.carry).abs() < 1.0, "{zinger:?}: {far}");
                assert!(down < rules.field.longest / 2, "{zinger:?}: {down}");
            }
        }
    }

    #[test]
    fn the_nearer_the_best_the_swing_the_further_the_ball_goes() {
        let rules = Rules::default();
        for difficulty in LEVELS {
            let table = rules.pitch.at(difficulty);
            // The worst the ring can do for a better-timed swing still
            // beats the best it can do for a worse one.
            let mut by_timing: Vec<(f32, u32, u32)> = table
                .window
                .iter()
                .map(|&(frames, ..)| {
                    let on = zinger(difficulty, frames, (0.0, 0.0), &rules);
                    let off = zinger(difficulty, frames, (200.0, 0.0), &rules);
                    (on.timed, off.feet, on.feet)
                })
                .collect();
            by_timing.sort_by(|a, b| a.0.total_cmp(&b.0));
            for pair in by_timing.windows(2) {
                if pair[0].0 < pair[1].0 {
                    assert!(pair[0].2 < pair[1].1, "{difficulty:?}: {pair:?}");
                }
            }
            let (worst, best) = (by_timing[0], by_timing[by_timing.len() - 1]);
            assert_eq!((worst.0, best.0), (0.0, 1.0));
            assert_eq!(worst.1, 440, "{difficulty:?}");
        }
    }

    #[test]
    fn the_harder_the_level_the_further_the_best_zinger_goes() {
        let rules = Rules::default();
        let best = |difficulty, frames| zinger(difficulty, frames, (0.0, 0.0), &rules).feet;
        assert_eq!(best(Difficulty::Easy, 11), 800);
        assert_eq!(best(Difficulty::Medium, 10), 850);
        assert_eq!(best(Difficulty::Hard, 10), 900);
    }

    #[test]
    fn holding_the_ring_on_the_ball_adds_a_little() {
        let rules = Rules::default();
        let feet = |ring| zinger(Difficulty::Easy, 11, ring, &rules).feet;
        assert_eq!(feet((0.0, 0.0)), 800);
        assert_eq!(feet((40.0, 0.0)), 770);
        assert_eq!(feet((0.0, 80.0)), 740);
        // Further off than that takes no more away.
        assert_eq!(feet((300.0, 0.0)), 740);
    }

    #[test]
    fn the_height_of_the_ring_shapes_the_flight() {
        let rules = Rules::default();
        let of = |under| zinger(Difficulty::Easy, 11, (0.0, under), &rules);
        let (driven, level, skied) = (of(-50.0), of(0.0), of(50.0));
        assert_eq!((driven.shape, level.shape, skied.shape), (-1.0, 0.0, 1.0));
        // A skied ball hangs two and a half times as long and goes far
        // higher. One driven low is the other way about.
        assert_eq!((level.hang, skied.hang), (200.0, 500.0));
        assert!(skied.peak > level.peak * 2.5 && driven.peak < level.peak / 2.0);
        assert!(driven.hang < level.hang);
        // Half the way there has half the effect, and further than all the
        // way has no more.
        assert_eq!(of(25.0).shape, 0.5);
        assert_eq!(of(25.0).hang, 350.0);
        assert_eq!(of(120.0).hang, skied.hang);
        // In the batting view the skied one is seen to leave more steeply.
        assert!(skied.lift(&rules.hit) > level.lift(&rules.hit));
        assert!(driven.lift(&rules.hit) < level.lift(&rules.hit));
    }

    #[test]
    fn a_ball_driven_low_is_not_kept_under_the_top_of_the_wall() {
        let rules = Rules::default();
        // The worst-timed swing drops the ball just behind the wall, which
        // a low flight would not get over.
        let low = zinger(Difficulty::Easy, 7, (0.0, -90.0), &rules);
        assert!(low.peak > rules.zinger.drive.peak);
        let mut ball = low.ball(HOME, (303.8, MARK));
        while !ball.walled {
            ball.step(HOME, 0.0, &rules.field);
        }
        assert!(ball.height > rules.field.clear * 1.3, "{}", ball.height);
    }

    #[test]
    fn the_ring_neither_drags_on_the_ball_nor_slows_it() {
        let rules = Rules::default();
        let contact = zinger(Difficulty::Easy, 11, (30.0, 60.0), &rules).contact(30.0);
        assert_eq!(
            (contact.under, contact.miss(), contact.aside),
            (0.0, 0.0, 30.0)
        );
    }

    #[test]
    fn the_crowd_makes_more_of_a_better_timed_hit() {
        let rules = Rules::default();
        let sounds = |frames| zinger(Difficulty::Easy, frames, (0.0, 0.0), &rules).hit_sounds();
        let (worst, middling, best) = (sounds(7), sounds(13), sounds(11));
        assert!(worst.1.len() <= middling.1.len() && middling.1.len() < best.1.len());
        assert_ne!(worst.0, best.0);
    }

    #[test]
    fn where_it_comes_down_is_named_by_how_far_and_which_way() {
        let rules = Rules::default().zinger;
        // The board behind the wall in the middle of the field.
        let board = Some([230.0, 20.0, 380.0, 110.0]);
        assert_eq!(
            Place::of((300.0, 90.0), 1.2, board, &rules),
            Place::Scoreboard
        );
        assert_eq!(Place::of((40.0, 150.0), 1.2, board, &rules), Place::Stands);
        assert_eq!(
            Place::of((-90.0, 100.0), 1.6, board, &rules),
            Place::OutOfThePark
        );
        // Over the board altogether.
        assert_eq!(
            Place::of((330.0, -60.0), 1.9, board, &rules),
            Place::OutOfThePark
        );
        // The arcade game's field has no board.
        assert_eq!(Place::of((300.0, 90.0), 1.2, None, &rules), Place::Stands);
        assert_ne!(Place::Stands.words(), Place::OutOfThePark.words());
    }

    /// Anywhere the ring might be held about the ball: up to sixty pixels
    /// to either side of it, and as far above or below.
    fn any_ring() -> impl Strategy<Value = Point> {
        (-60.0f32..=60.0, -60.0f32..=60.0)
    }

    fn any_level() -> impl Strategy<Value = Difficulty> {
        prop::sample::select(&LEVELS[..])
    }

    proptest! {
        #[test]
        fn there_is_a_zinger_exactly_when_the_swing_meets_the_ball(
            frames in 0u32..25,
            ring in any_ring(),
            level in any_level(),
        ) {
            let rules = Rules::default();
            let table = rules.pitch.at(level);
            let zinger = Zinger::of(table, frames, ring, HOME, level, &rules);
            prop_assert_eq!(zinger.is_some(), meets(table, frames).is_some());
        }

        #[test]
        fn with_the_ring_held_the_same_a_better_timed_swing_never_carries_less(
            // Two frames of the level's window, whichever they are.
            (one, other) in any::<(prop::sample::Index, prop::sample::Index)>(),
            ring in any_ring(),
            level in any_level(),
        ) {
            let rules = Rules::default();
            let window = &rules.pitch.at(level).window;
            let of = |frame: prop::sample::Index| zinger(level, frame.get(window).0, ring, &rules);
            let (one, other) = (of(one), of(other));
            let (worse, better) = if one.timed <= other.timed {
                (one, other)
            } else {
                (other, one)
            };
            prop_assert!(worse.carry <= better.carry, "{:?} and {:?}", worse, better);
            prop_assert!(worse.feet <= better.feet, "{:?} and {:?}", worse, better);
        }

        #[test]
        fn any_zinger_goes_over_the_wall_and_comes_down_where_it_was_sent(
            frame: prop::sample::Index,
            ring in any_ring(),
            level in any_level(),
            // Sent anywhere from the one foul line to the other.
            across in 0.0f32..=1.0,
        ) {
            let rules = Rules::default();
            let (frames, ..) = *frame.get(&rules.pitch.at(level).window);
            let zinger = zinger(level, frames, ring, &rules);
            // Flying it fails if it comes to the wall too low, or comes
            // down inside it.
            let towards = FOUL.0 + (FOUL.1 - FOUL.0) * across;
            let (over, down, at) = fly(&zinger, towards, &rules);
            // It goes over once the view has changed to the field, so that
            // it is seen to go, and comes down in good time.
            prop_assert!(rules.hit.watch < over && over < down, "over on {}", over);
            prop_assert!(down < rules.field.longest / 2, "down on {}", down);
            let far = reach(HOME, at);
            prop_assert!((far - zinger.carry).abs() < 1.0, "{} off for {:?}", far, zinger);
        }
    }
}
