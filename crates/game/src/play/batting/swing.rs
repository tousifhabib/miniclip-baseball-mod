//! The ball's flight to the plate and the swing at it, up to the bat
//! meeting the ball and sending it off.

use bb_engine::stage::Stage;

use super::aim::hit_towards;
use crate::game::Game;
use crate::play::field::{Ball, Contact};
use crate::play::pitch::{self, Point, Quality, Sample};
use crate::play::zinger::Zinger;
use crate::play::{AtBat, Match, Phase, at, put, show, timing, zinger};
use crate::rules::Rules;

/// How the bat met the ball: how many frames into the swing, how well, and
/// how hard.
#[derive(Clone, Copy)]
struct Met {
    frames: u32,
    quality: Quality,
    power: f32,
}

impl Match {
    /// One frame of the ball on its way to the batter.
    pub(in crate::play) fn flight(
        &mut self,
        at_bat: &mut AtBat,
        step: usize,
        pressed: Option<Point>,
        game: &Game,
        stage: &mut Stage,
    ) {
        let Some(&sample) = at_bat.pitch.samples.get(step) else {
            return self.call(at_bat, game, stage);
        };
        for (path, point) in [
            (&at_bat.parts.ball, sample.ball),
            (&at_bat.parts.shadow, sample.shadow),
        ] {
            put(stage, path, point, sample.size);
            if let Some(child) = stage.child_mut(path) {
                child.set_alpha(sample.alpha);
            }
        }

        if let Some(clicked) = pressed
            && at_bat.swing.is_none()
        {
            self.start_the_swing(at_bat, step, clicked, stage);
        } else if let Some(frames) = &mut at_bat.swing {
            *frames += 1;
        }

        let in_band = sample.in_band(at_bat.table.band);
        let met = at_bat.swing.and_then(|frames| {
            let (quality, power) = pitch::meets(&at_bat.table, frames)?;
            Some(Met {
                frames,
                quality,
                power,
            })
        });
        if let (true, Some(met)) = (in_band, met) {
            self.the_bat_meets_the_ball(at_bat, sample, met, game, stage);
        } else {
            self.phase = Phase::Flight { step: step + 1 };
        }
    }

    /// The batter swings, at a click at this point of the stage, on this
    /// step of the pitch's flight.
    fn start_the_swing(
        &mut self,
        at_bat: &mut AtBat,
        step: usize,
        clicked: Point,
        stage: &mut Stage,
    ) {
        let (x, y) = clicked;
        // The swing is high, level or low by where the click was.
        let pointer = stage
            .from_stage(&at_bat.parts.main, x, y)
            .unwrap_or(at_bat.aim);
        let label = match pointer.1 {
            y if y <= 200.0 => "hitHigh",
            y if y <= 280.0 => "hitMed",
            _ => "hitLow",
        };
        stage.goto_label(&at_bat.parts.hitter, label, true);
        if !self.mode.is_arcade() {
            Match::sound(stage, "batSwing_fast");
        }
        at_bat.swing = Some(0);
        if self.mode.full().is_some() {
            at_bat.swing_off = timing::Timing::of(&at_bat.pitch, &at_bat.table)
                .best()
                .map(|(first, last)| step as i32 - step.clamp(first, last) as i32);
        }
        at_bat.under = at_bat.aim.1 - at_bat.pitch.crosses.1;
        at_bat.across = at_bat.aim.0 - at_bat.pitch.crosses.0;
        if let Some(bar) = &mut at_bat.timing {
            bar.swung(step, stage);
        }
    }

    /// The bat has met the ball: the crowd says how well, the ball leaves
    /// the bat, and the view watches it go.
    fn the_bat_meets_the_ball(
        &mut self,
        at_bat: &mut AtBat,
        sample: Sample,
        met: Met,
        game: &Game,
        stage: &mut Stage,
    ) {
        let rules = &game.rules;
        let Met {
            frames,
            quality,
            power,
        } = met;
        at_bat.met = Some(quality);
        self.mods.the_bat_met_the_ball();
        // With the zinger mod on, whatever the bat meets is on its way
        // out of the ground.
        let zinger = self
            .mods
            .every_hit_is_a_home_run()
            .then(|| {
                let ring = (at_bat.across, at_bat.under);
                let home = at_bat.parts.home;
                let difficulty = game.settings.difficulty;
                Zinger::of(&at_bat.table, frames, ring, home, difficulty, rules)
            })
            .flatten();
        Match::cheer_the_hit(quality, zinger.as_ref(), stage);
        let contact = Match::contact_made(at_bat, power, zinger.as_ref(), rules);
        self.send_the_ball_off(at_bat, sample, &contact, zinger, rules, stage);
        at_bat.contact = Some(contact);
        // He is 34 frames into his swing when he drops the bat.
        at_bat.run_in = Some(34u32.saturating_sub(at_bat.swing.unwrap_or(0)));
        self.phase = Phase::Watching {
            left: if self.mode.is_arcade() {
                at_bat.run_in = None;
                rules.arcade.watch
            } else {
                rules.hit.watch
            },
        };
    }

    /// The sound of the bat on the ball and the crowd's answer to it, by
    /// how well it was met, or by the zinger it is.
    fn cheer_the_hit(quality: Quality, zinger: Option<&Zinger>, stage: &mut Stage) {
        let (hit, cheer): (&str, &[&str]) = match quality {
            Quality::Poor => ("batHit_poorly", &["crowd_smallClap"]),
            Quality::MediumPoor => ("batHit_mediumPoor", &["crowd_smallCheer"]),
            Quality::Medium => ("batHit_medium", &["crowd_smallCheer", "crowd_smallClap"]),
            Quality::Good => ("batHit_good", &["crowd_bigClap"]),
        };
        let (hit, cheer) = zinger.map_or((hit, cheer), |zinger| zinger.hit_sounds());
        Match::sound(stage, hit);
        for name in cheer {
            Match::sound(stage, name);
        }
    }

    /// How the bat met the ball: how hard, how far under it and how far to
    /// the side, which is by where the art's pointer shows the hit going.
    fn contact_made(
        at_bat: &mut AtBat,
        power: f32,
        zinger: Option<&Zinger>,
        rules: &Rules,
    ) -> Contact {
        // The hit goes by where the ball really was, if that is not
        // where the marker showed it.
        if at_bat.marker_at != at_bat.pitch.crosses {
            let (crosses, centre) = (at_bat.pitch.crosses.0, at_bat.parts.centre_x);
            at_bat.aim_area_x = hit_towards(crosses, at_bat.aim.0, centre, rules.hit.pull);
        }
        if zinger.is_some() {
            at_bat.aim_area_x = zinger::fair(at_bat.aim_area_x, &at_bat.parts, &rules.field);
        }
        let aside = at_bat.aim_area_x - at_bat.parts.centre_x;
        match zinger {
            Some(zinger) => zinger.contact(aside),
            None => Contact {
                power,
                under: at_bat.under,
                aside,
            },
        }
    }

    /// Sends the ball off the bat: in the batting view, where it is
    /// watched for a moment, and over the field, where it will be played.
    fn send_the_ball_off(
        &mut self,
        at_bat: &mut AtBat,
        sample: Sample,
        contact: &Contact,
        zinger: Option<Zinger>,
        rules: &Rules,
        stage: &mut Stage,
    ) {
        // In the batting view the ball flies off towards where the
        // art's pointer showed, dropping further the weaker the hit.
        let parts = &at_bat.parts;
        at_bat.fly = (
            sample.shadow,
            sample.shadow.1 - sample.ball.1,
            zinger.map_or(contact.lift(&rules.hit), |zinger| zinger.lift(&rules.hit)),
        );
        // It leaves the bat the size it had come to, and shrinks from
        // there as it goes away.
        at_bat.fly_size = sample.size;
        if let Some(shadow) = &parts.fly_shadow {
            let place = at(stage, shadow);
            put(stage, shadow, place, sample.size);
        }
        at_bat.fly_target = (
            at_bat.aim_area_x,
            parts.fly_mark.1 + contact.power + contact.miss() / 2.0,
        );
        // And over the field it heads for the mark, pushed aside by
        // the same amount.
        let mark = (
            parts.field_mark.0 + contact.aside / rules.field.aim_share,
            parts.field_mark.1,
        );
        at_bat.ball = Some(match &zinger {
            Some(zinger) => zinger.ball(parts.home, mark),
            None => Ball::hit(parts.home, mark, contact, &rules.hit, &rules.field),
        });
        at_bat.zinger = zinger;
        if let (Some(arcade), Some(zinger)) = (self.mode.arcade_mut(), zinger) {
            // In the arcade game a zinger scores by how far it goes.
            arcade.owed = Some(zinger.feet);
        }
        show(stage, &parts.ball, false);
        show(stage, &parts.shadow, false);
    }
}
