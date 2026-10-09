//! What the mods say about the ball in the field: who fields it, what
//! it bounces off, a zinger, a sign struck, and the lights.

use bb_engine::math::ColorTransform;

use super::ModsInPlay;
use crate::play::mods::bullet_time;
use crate::play::mods::butterfingers::Butterfingers;
use crate::play::mods::hit_the_sign;
use crate::play::mods::hit_the_sign::HitTheSign;
use crate::play::mods::the_shift::TheShift;
use crate::play::mods::zinger_hit::ZingerHit;
use crate::play::night_game;
use crate::rng::Rng;

impl ModsInPlay {
    /// Whether the pitcher is left to field every ball by himself: nobody
    /// else goes after it, goes back to watch it, or throws it on.
    pub fn the_pitcher_fields_alone(&self) -> bool {
        self.lone_pitcher.is_some()
    }

    /// Whether a fielder having a go at the ball lets it go. A number is
    /// drawn for the go only if the mod that makes them slip is in play.
    pub fn a_fielder_lets_go(&mut self, rng: &mut Rng) -> bool {
        self.butterfingers
            .as_mut()
            .is_some_and(|butter| butter.lets_go(rng))
    }

    /// How often the fielders have let the ball go.
    pub fn let_go(&self) -> u32 {
        self.butterfingers.as_ref().map_or(0, Butterfingers::slips)
    }

    /// Whether the ball keeps its speed when it bounces and cannot get out
    /// of the field except over the wall on the fly.
    pub fn the_park_is_a_pinball_table(&self) -> bool {
        self.pinball_park.is_some()
    }

    /// How far the fielders have shifted for the pitch in hand.
    pub fn shifted(&self) -> f32 {
        self.the_shift.as_ref().map_or(0.0, TheShift::by)
    }

    /// Whether whatever the bat meets goes out of the ground.
    pub fn every_hit_is_a_home_run(&self) -> bool {
        self.zinger_hit.is_some()
    }

    /// The longest zinger of this game, in feet. Nought if there was none.
    pub fn longest_zinger(&self) -> u32 {
        self.zinger_hit.as_ref().map_or(0, ZingerHit::longest)
    }

    /// The longest zinger there has ever been, as far as this game knows.
    pub fn zinger_record(&self) -> u32 {
        self.zinger_hit.as_ref().map_or(0, ZingerHit::record)
    }

    /// Tells the game the record its zingers have to beat.
    pub fn set_zinger_record(&mut self, feet: u32) {
        if let Some(zinger) = &mut self.zinger_hit {
            zinger.set_record(feet);
        }
    }

    /// A zinger has gone `feet`. Returns whether it is a new record.
    pub fn a_zinger_went(&mut self, feet: u32) -> bool {
        self.zinger_hit
            .as_mut()
            .is_some_and(|zinger| zinger.count(feet))
    }

    /// A ball that was hit fair has come down this far across the field,
    /// from 0 at one foul line to 1 at the other.
    pub fn a_fair_ball_came_down(&mut self, across: f32) {
        if let Some(shift) = &mut self.the_shift {
            shift.remember(across);
        }
    }

    /// A home run has been hit, or a zinger has come down.
    pub fn a_home_run_was_hit(&mut self) {
        if let Some(night) = &mut self.night_game {
            night.a_home_run_was_hit();
        }
    }

    /// How the stadium is to be lit this frame, if any mod has a say in
    /// it: dark by night, flashing for a home run, and cooler while the
    /// ball is being held back, which `held_back` says it is.
    pub fn lighting(&mut self, held_back: bool) -> Option<ColorTransform> {
        if self.night_game.is_none() && self.bullet_time.is_none() {
            return None;
        }
        let lighting = match &mut self.night_game {
            Some(night) => night.lighting(),
            None => night_game::DAY,
        };
        Some(if held_back {
            bullet_time::cool(lighting)
        } else {
            lighting
        })
    }

    /// The sign that is lit for this innings, counting from 0. `None` when
    /// the mod is off.
    pub fn light_a_sign(&mut self, innings: u32, signs: &hit_the_sign::Signs) -> Option<usize> {
        let sign = self.hit_the_sign.as_mut()?;
        Some(sign.light(innings, signs))
    }

    /// A ball has struck this sign, counting from 0, for this many runs.
    /// It is kept to be told.
    pub fn a_sign_was_struck(&mut self, sign: usize, runs: u32) {
        if let Some(signs) = &mut self.hit_the_sign {
            signs.news = Some((sign, runs));
        }
    }

    /// The runs a sign that was struck paid, the first time it is asked
    /// for.
    pub fn news_of_a_sign(&mut self) -> Option<u32> {
        let signs = self.hit_the_sign.as_mut()?;
        let (sign, runs) = signs.news.take()?;
        signs.struck = Some((sign, runs));
        Some(runs)
    }

    /// Which sign on the wall is lit, counting from 0, once one has been.
    pub fn sign_lit(&self) -> Option<usize> {
        self.hit_the_sign.as_ref().and_then(HitTheSign::lit)
    }

    /// The sign a ball has struck on the pitch in hand, counting from 0,
    /// and the runs it paid.
    pub fn sign_struck(&self) -> Option<(usize, u32)> {
        self.hit_the_sign.as_ref().and_then(HitTheSign::struck)
    }
}
