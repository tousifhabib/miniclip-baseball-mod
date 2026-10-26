//! How late the player's clicks reach the game, and so which frame a click
//! was aimed at.
//!
//! A click is aimed at what the player sees, and what the player sees is a
//! frame the game played a little while before. The window knows when each
//! frame was handed to the screen, and times every click by the frame that
//! was showing then (`Pointer::went_down_at`). What it cannot know is how
//! long the screen itself takes to light up what it is handed, how long
//! the mouse takes to say it was pressed, and how far ahead of what they
//! see a player's hand goes. Together that is the delay. It is measured on
//! the page for it in the menu, or set by hand there, and is kept from one
//! run to the next.
//!
//! The sums of measuring it are in `taps`, and the page is in `page`.

pub(crate) mod page;
pub mod taps;

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::kept;

/// How much later than the window can see for itself a click reaches the
/// game, in thousandths of a second.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Delay {
    ms: f32,
}

impl Delay {
    /// The least and the most it can be. A delay below nought is a player
    /// who clicks ahead of what they see.
    pub const LEAST: f32 = -100.0;
    pub const MOST: f32 = 250.0;

    /// A delay of `ms`, to the nearest whole thousandth and kept within
    /// what it can be.
    pub fn of(ms: f32) -> Delay {
        let ms = if ms.is_finite() { ms.round() } else { 0.0 };
        Delay {
            ms: ms.clamp(Delay::LEAST, Delay::MOST),
        }
    }

    pub fn ms(self) -> f32 {
        self.ms
    }

    /// The delay in frames of a game played at this many frames a second.
    pub fn frames(self, frame_rate: f64) -> f64 {
        f64::from(self.ms) * frame_rate / 1000.0
    }

    /// Where the delay is kept.
    pub fn usual_file() -> Option<PathBuf> {
        kept::usual_file("delay.toml")
    }

    /// Reads the delay kept in `file`. One that is missing or cannot be
    /// read is no delay.
    pub fn load(file: &Path) -> Delay {
        kept::read::<Delay>(file).map_or(Delay::default(), |kept| Delay::of(kept.ms))
    }

    pub fn save(self, file: &Path) -> Result<()> {
        kept::write(file, &self, "the delay")
    }
}

/// The frame a click was aimed at: the one the player was looking at when
/// they clicked, the click being made at `at` on the window's clock (see
/// `Pointer::went_down_at`) and reaching the game `delay` frames after
/// the player meant it.
pub fn aimed_at(at: f64, delay: f64) -> i64 {
    (at - delay).floor() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_delay_is_in_whole_thousandths_and_within_bounds() {
        assert_eq!(Delay::of(41.6).ms(), 42.0);
        assert_eq!(Delay::of(-400.0).ms(), Delay::LEAST);
        assert_eq!(Delay::of(9000.0).ms(), Delay::MOST);
        assert_eq!(Delay::of(f32::NAN), Delay::default());
        assert!((Delay::of(50.0).frames(60.0) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn a_click_is_aimed_at_the_frame_showing_when_it_was_meant() {
        // With no delay, the frame on the screen.
        assert_eq!(aimed_at(10.0, 0.0), 10);
        assert_eq!(aimed_at(10.9, 0.0), 10);
        assert_eq!(aimed_at(9.5, 0.0), 9);
        // Two and a half frames late, it was meant for frame 8.
        assert_eq!(aimed_at(10.9, 2.5), 8);
        assert_eq!(aimed_at(10.4, 2.5), 7);
    }

    #[test]
    fn a_delay_reads_back_as_it_was_kept_and_as_none_if_it_cannot_be_read() {
        let folder = std::env::temp_dir().join(format!("bb-delay-{}", std::process::id()));
        let file = folder.join("delay.toml");
        assert_eq!(Delay::load(&file), Delay::default());
        Delay::of(37.0)
            .save(&file)
            .expect("a delay that is written");
        assert_eq!(Delay::load(&file), Delay::of(37.0));
        std::fs::write(&file, "ms = 7000.0").expect("a file");
        assert_eq!(Delay::load(&file).ms(), Delay::MOST);
        let _ = std::fs::remove_dir_all(folder);
    }
}
