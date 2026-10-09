//! The numbers the ball goes by: the pitch, the throw, the hit, and its
//! flight over the field.

use serde::Deserialize;

use super::shapes::{Area, Band, BySkill, Curve, Span};
use crate::play::pitch::Quality;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PitchRules {
    pub speed: Span,
    pub swing: Curve,
    pub dip: Curve,
    pub target: Area,
    pub marker_frame: u16,
    pub aim_ease: f32,
    pub show_zone: bool,
    pub band: Band,
    /// Frames after the swing, how well the ball is met, and the power.
    pub window: Vec<(u32, Quality, f32)>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThrowRules {
    pub settle: u32,
    pub release_frame: u16,
    pub swing_lead: f32,
    pub dip_lead: f32,
    pub approach: f32,
    pub size: f32,
    pub growth: f32,
    pub fade: f32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HitRules {
    pub pull: f32,
    pub pointer_before_pitch: bool,
    pub watch: u32,
    pub walk_wait: u32,
    pub lift: f32,
    pub lift_aim: f32,
    pub power_drag: f32,
    pub gravity: f32,
    pub bounce: f32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldRules {
    pub x: f32,
    pub pace: f32,
    pub aim_share: f32,
    pub lift_share: f32,
    pub gravity: f32,
    pub drag: f32,
    pub drag_aim: f32,
    pub drag_reach: f32,
    pub bounce_run: f32,
    pub bounce_lift: f32,
    pub bounce_cap: f32,
    pub bounce_loss: f32,
    pub wall: f32,
    pub clear: f32,
    pub wall_bounce: f32,
    pub fielder_reach: f32,
    pub catch_height: f32,
    pub throw_speed: f32,
    pub throw_near: f32,
    pub pick_time: u32,
    pub throw_time: u32,
    pub longest: u32,
    pub fielder_speed: BySkill<f32>,
}
