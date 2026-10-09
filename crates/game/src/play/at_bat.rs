//! Everything that lasts for one pitch and is built again for the next.

use crate::play::fielding;
use crate::play::full;
use bb_engine::stage::Stage;
use overlay::Notices;
use zinger::Zinger;

use crate::play::field::{Ball, Contact};
use crate::play::mods::bullet_time as bullet;
use crate::play::mods::{
    called_shot as called, hit_the_sign as sign, stolen_bases as steal, timing_indicator as timing,
    zinger_hit as zinger,
};
use crate::play::pitch::{Kind, Pitch, Point, Quality};
use crate::play::view::{Parts, overlay};
use crate::rules::PitchRules;

/// The pitch being played.
pub(crate) struct AtBat {
    pub parts: Parts,
    pub table: PitchRules,
    pub pitch: Pitch,
    pub marker_shown: bool,
    /// Where the marker shows the pitch crossing, which the knuckleball
    /// mod makes only roughly right.
    pub marker_at: Point,
    /// What the pitch is, with the mystery pitch mod on.
    pub kind: Option<Kind>,
    /// The pitch is a golden ball.
    pub golden: bool,
    /// Where the batter has said his hit will come down, with the called
    /// shot mod on.
    pub called: Option<called::Called>,
    pub aim: Point,
    /// Where the hit would go sideways, as the art's indicator shows it.
    pub aim_area_x: f32,
    /// Frames since the swing began.
    pub swing: Option<u32>,
    /// How far below the ball the ring was when the swing began, and how
    /// far to the right of it.
    pub under: f32,
    pub across: f32,
    pub contact: Option<Contact>,
    /// The ball leaving the bat, in the batting view: where it is, how high,
    /// and how fast it is rising.
    pub fly: (Point, f32, f32),
    /// The size the ball had grown to when the bat met it.
    pub fly_size: f32,
    pub fly_target: Point,
    /// Frames until the batter drops his bat and runs.
    pub run_in: Option<u32>,
    pub ball: Option<Ball>,
    pub fielding: Option<fielding::Fielding>,
    /// The timing bar, while that mod is on and the batting view is up.
    pub timing: Option<timing::Indicator>,
    /// The hit, if that mod made a zinger of it, and what the player is
    /// shown of it over the field.
    pub zinger: Option<Zinger>,
    pub zinger_show: Option<zinger::Show>,
    /// The ball went over the wall while it was still being watched leaving
    /// the bat, which the view of the field has yet to be told.
    pub over_wall: bool,
    /// What the mods have written up in the view.
    pub notices: Notices,
    /// Where the hit first came down, if it has and the called shot mod
    /// wants to know.
    pub came_down: Option<Point>,
    /// How many times the wall or a foul line has sent the ball back, in
    /// a pinball park.
    pub rebounds: u32,
    /// In a full match, the word on each scoreboard over the other side's
    /// score.
    pub them: Vec<full::Them>,
    /// For a full match's book: how many frames after the best moment for
    /// it the swing began, and how well the bat met the ball.
    pub swing_off: Option<i32>,
    pub met: Option<Quality>,
    /// The runners' marks on the little field, with the stolen bases mod
    /// on and anyone on base.
    pub leads: Option<steal::Leads>,
    /// The signs on the wall, with the hit the sign mod on.
    pub signs: Option<sign::Board>,
    /// The meter in the corner of the view, with the bullet time mod on.
    pub meter: Option<bullet::Meter>,
}

impl AtBat {
    /// The view is changing to the field, where the timing bar has no
    /// place.
    pub(crate) fn leave_batting_view(&mut self, stage: &mut Stage) {
        if let Some(bar) = self.timing.take() {
            bar.put_away(stage);
        }
    }
}
