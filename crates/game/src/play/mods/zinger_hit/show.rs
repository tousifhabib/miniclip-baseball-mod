//! A zinger as it is shown over the field: the feet counting up as it
//! flies, its trail, and where it came down.

use std::collections::VecDeque;

use bb_engine::display::{Path, child_bounds};
use bb_engine::library::Library;
use bb_engine::math::Matrix;
use bb_engine::stage::Stage;

use super::Zinger;
use super::place::Place;
use crate::art;
use crate::look::{self, Rgb};
use crate::play::field::{Ball, Happened, reach, seen_size};
use crate::play::overlay::{self, Words};
use crate::play::pitch::Point;
use crate::play::{Parts, put};
use crate::rules::Rules;

/// How many times the size of the ball's own shadow the mark of where it
/// will land is, between its beats.
const MARKER_SIZE: f32 = 6.0;

/// The dots that trail behind the ball: how many, and how many frames
/// apart.
const TRAIL: usize = 8;
const TRAIL_EVERY: usize = 3;

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
        let depth = overlay::free_below(field, ball)?;
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
