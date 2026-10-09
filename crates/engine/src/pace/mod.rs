//! Decides when the game's frames are played, redraw by redraw.
//!
//! The game plays a fixed number of frames a second and the screen redraws
//! at a rate of its own. Going by the clock alone, a frame is played each
//! time a frame's worth of time has gone by. On a screen that redraws a
//! whole number of times to each frame of the game, that puts every redraw
//! right on the line between "not yet" and "now", and the moment a redraw
//! begins wobbles by a millisecond or two. So some frames are held for a
//! redraw too many and the next for one too few, and what should move
//! evenly judders: all the time if the game happened to start at the wrong
//! moment, and not at all if it did not.
//!
//! So while the screen keeps such a rate the redraws are counted instead,
//! and a frame is played every so many of them however the clock wobbles.
//! The clock is still read, to see whether the screen is keeping that rate
//! and to catch up after a stall.

#[cfg(test)]
mod properties;

use std::collections::VecDeque;
use std::time::Duration;

/// How many redraws the screen's rate is judged from.
const RECENT: usize = 64;
/// How many redraws there must have been before it is judged at all.
const ENOUGH: usize = 20;
/// How close the screen's rate must be to a whole number of redraws for
/// each frame of the game to be taken as in step with it, and how far it
/// may then stray before it is taken to have left: shares of that rate.
const NEAR: f64 = 0.03;
const FAR: f64 = 0.1;
/// The most redraws to a frame that are counted. Past this the screen is
/// not drawing at all, or is not being waited for.
const MOST_REDRAWS: u32 = 8;
/// How much of the time the count is behind the clock is kept from one
/// redraw to the next. The rest is let go, so that a clock which runs a
/// touch faster or slower than the screen never adds up to a redraw.
const KEEP: f64 = 0.98;
/// How many redraws the count may be behind the clock, or ahead of it,
/// before it is put right. A redraw that comes late is nearly always made
/// up by the next one coming early, and where it is not, a frame shown a
/// moment late is better than one hurried off the screen to make it up.
const SLACK: f64 = 2.5;
/// The most frames played for one redraw. After a long stall the game
/// skips ahead instead of replaying it all.
const MOST_AT_ONCE: u32 = 5;
/// The longest a redraw is taken to have been in coming.
const LONGEST: Duration = Duration::from_secs(1);

pub struct Pace {
    /// How long a frame of the game lasts.
    frame: Duration,
    /// How long each of the last redraws was in coming, the latest last.
    recent: VecDeque<Duration>,
    /// How many redraws go to a frame of the game, while the screen is in
    /// step with it.
    every: Option<u32>,
    /// Redraws counted towards the next frame, while in step.
    counted: u32,
    /// How far the count is behind the clock, in redraws.
    behind: f64,
    /// Time that has passed but not yet been played, while going by the
    /// clock.
    owed: Duration,
}

impl Pace {
    /// The pace for a game whose frames each last this long.
    pub fn new(frame: Duration) -> Pace {
        Pace {
            frame,
            recent: VecDeque::with_capacity(RECENT),
            every: None,
            counted: 0,
            behind: 0.0,
            owed: Duration::ZERO,
        }
    }

    /// How many redraws go to each frame of the game, if the screen is in
    /// step with it. `None` while frames are played by the clock.
    pub fn in_step(&self) -> Option<u32> {
        self.every
    }

    /// How many frames to play for a redraw that has begun `elapsed` after
    /// the one before.
    pub fn frames(&mut self, elapsed: Duration) -> u32 {
        if self.frame.is_zero() {
            return 0;
        }
        let elapsed = elapsed.min(LONGEST);
        if self.recent.len() == RECENT {
            self.recent.pop_front();
        }
        self.recent.push_back(elapsed);
        let every = self.keeping();
        if every != self.every {
            self.carry_over(every);
        }

        let frames = match every {
            Some(every) => {
                let redraw = self.frame.as_secs_f64() / f64::from(every);
                self.behind = self.behind * KEEP + elapsed.as_secs_f64() / redraw - 1.0;
                // This redraw counts for one, unless the clock says the
                // count has come adrift.
                let redraws = if self.behind >= SLACK {
                    let whole = self.behind.floor();
                    self.behind -= whole;
                    1 + whole as u32
                } else if self.behind <= -SLACK {
                    self.behind += 1.0;
                    0
                } else {
                    1
                };
                self.counted += redraws;
                let frames = self.counted / every;
                self.counted %= every;
                frames
            }
            None => {
                self.owed += elapsed;
                let frames = (self.owed.as_nanos() / self.frame.as_nanos()) as u32;
                self.owed -= self.frame * frames;
                frames
            }
        };
        if frames > MOST_AT_ONCE {
            self.owed = Duration::ZERO;
            self.counted = 0;
            self.behind = 0.0;
            return MOST_AT_ONCE;
        }
        frames
    }

    /// The redraw last asked about came to nothing: the window is hidden,
    /// say. Such redraws come as fast as they are asked for and tell
    /// nothing of the screen, so frames are played by the clock until it
    /// has been drawn to for a while again.
    pub fn undrawn(&mut self) {
        self.recent.clear();
        if self.every.is_some() {
            self.carry_over(None);
        }
    }

    /// How many redraws the screen is making to each frame of the game, if
    /// it is a whole number of them.
    fn keeping(&self) -> Option<u32> {
        let ratio = self.frame.as_secs_f64() / self.usual()?;
        let within = |every: u32, share: f64| (ratio / f64::from(every) - 1.0).abs() <= share;
        // A screen that has been in step is left so through a rough patch.
        if let Some(every) = self.every
            && within(every, FAR)
        {
            return Some(every);
        }
        let every = ratio.round();
        if !(1.0..=f64::from(MOST_REDRAWS)).contains(&every) {
            return None;
        }
        within(every as u32, NEAR).then_some(every as u32)
    }

    /// How long a redraw usually is in coming, in seconds: the mean of the
    /// last of them, leaving out the longest and shortest. A stall and the
    /// hurried redraws after it are left out that way, and redraws that
    /// take turns to be long and short still come out right.
    fn usual(&self) -> Option<f64> {
        if self.recent.len() < ENOUGH {
            return None;
        }
        let mut sorted = [0.0; RECENT];
        let sorted = &mut sorted[..self.recent.len()];
        for (slot, took) in sorted.iter_mut().zip(&self.recent) {
            *slot = took.as_secs_f64();
        }
        sorted.sort_by(f64::total_cmp);
        let cut = sorted.len() / 8;
        let middle = &sorted[cut..sorted.len() - cut];
        let usual = middle.iter().sum::<f64>() / middle.len() as f64;
        (usual > 0.0).then_some(usual)
    }

    /// Goes over to counting `every` redraws to a frame, or to the clock
    /// for `None`, keeping what has been counted or is owed so far.
    fn carry_over(&mut self, every: Option<u32>) {
        match (self.every, every) {
            (None, Some(now)) => {
                let share = self.owed.as_secs_f64() / self.frame.as_secs_f64();
                self.counted = ((share * f64::from(now)) as u32).min(now - 1);
            }
            (Some(was), None) => self.owed = self.frame * self.counted / was,
            (Some(was), Some(now)) => self.counted = self.counted * now / was,
            (None, None) => {}
        }
        self.behind = 0.0;
        self.every = every;
    }
}

#[cfg(test)]
mod tests;
