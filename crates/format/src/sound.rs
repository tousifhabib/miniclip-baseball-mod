//! The sounds a frame of a timeline starts and stops.

use serde::{Deserialize, Serialize};

use crate::SymbolId;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundStart {
    pub sound: SymbolId,
    pub event: SoundEvent,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub loops: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_sample: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out_sample: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub envelope: Vec<EnvelopePoint>,
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde hands the field over by reference"
)]
fn is_zero(n: &u16) -> bool {
    *n == 0
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundEvent {
    /// Play, even if this sound is already playing.
    Event,
    /// Play only if this sound is not already playing.
    Start,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnvelopePoint {
    pub sample: u32,
    pub left: f32,
    pub right: f32,
}
