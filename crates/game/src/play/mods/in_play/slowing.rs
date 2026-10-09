//! Bullet time's say in a pitch: whether the ball is held back this
//! frame, and what is left in the meter.

use super::ModsInPlay;
use crate::play::mods::bullet_time::BulletTime;
use crate::play::pitch::Point;

impl ModsInPlay {
    /// Whether the ball was held back on the frame just played. It is asked
    /// once a frame.
    pub fn the_ball_was_held_back(&mut self) -> bool {
        self.bullet_time
            .as_mut()
            .is_some_and(BulletTime::was_slowed)
    }

    /// A click that was kept from a frame the ball was held back on.
    pub fn late_press(&mut self) -> Option<Point> {
        self.bullet_time.as_mut().and_then(BulletTime::late_press)
    }

    /// Keeps a click made while the ball is held back.
    pub fn keep_press(&mut self, pressed: Option<Point>) {
        if let Some(bullet) = &mut self.bullet_time {
            bullet.keep_press(pressed);
        }
    }

    /// How much of bullet time's meter is left, from 0 to 1, once it has
    /// been filled.
    pub fn meter_left(&self) -> Option<f32> {
        self.bullet_time.as_ref().and_then(BulletTime::share_left)
    }

    /// What is left in bullet time's meter, and whether the ball is being
    /// held back.
    pub fn bullet_time(&self) -> Option<(u32, bool)> {
        let bullet = self.bullet_time.as_ref()?;
        Some((bullet.left()?, bullet.slowed()))
    }

    /// With bullet time on, its meter is full when the game starts. Says
    /// whether the mod is on.
    pub fn fill_the_meter_at_the_start(&mut self) -> bool {
        let Some(bullet) = &mut self.bullet_time else {
            return false;
        };
        bullet.fill_at_the_start();
        true
    }

    /// Whether the frame in hand is one that bullet time holds the ball
    /// back for. `step` is the step of its flight the pitch has come to,
    /// of `steps`, `swung` whether the batter has swung at it, and
    /// `key_down` whether bullet time's key is held.
    pub fn holds_the_ball_back(
        &mut self,
        step: usize,
        steps: usize,
        swung: bool,
        key_down: bool,
    ) -> bool {
        let Some(bullet) = &mut self.bullet_time else {
            return false;
        };
        let near = bullet.is_near(step, steps);
        bullet.holds_back(near, swung, key_down)
    }
}
