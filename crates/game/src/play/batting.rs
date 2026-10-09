//! At the plate: the frames of a pitch, from the pitcher standing and
//! waiting to the bat meeting the ball or the umpire's call.
//!
//! What lasts from pitch to pitch is the match's. What happens on each
//! frame of one pitch is here, a function for each part of it: the wait, the
//! wind-up, the ball's flight, the call, and watching a hit go.

use bb_engine::library::Library;
use bb_engine::stage::Stage;

use super::book::{End, Thrown};
use super::field::{Ball, Contact, Happened};
use super::pitch::{self, Point, Quality};
use super::zinger::Zinger;
use super::{
    AtBat, Cue, MYSTERY_TOP, Match, PITCH, Parts, Phase, Place, at, called, frame_of, hit_towards,
    put, show, southpaw, timing, zinger,
};
use crate::art;
use crate::menu::Game;
use crate::rules::HitRules;

impl Match {
    /// Keeps the batting view as it should be for this frame: the pointer
    /// out of sight while the ring stands for it, the batter still once he
    /// has swung, and whatever the mods have drawn kept up.
    pub(super) fn keep_the_view(
        &mut self,
        at_bat: &mut AtBat,
        slowed: bool,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        let rules = &game.rules;
        // While the ring is being aimed it stands for the pointer, which
        // would only get in its way.
        let aiming = at_bat.contact.is_none()
            && matches!(
                self.phase,
                Phase::Settling { .. } | Phase::WindUp | Phase::Flight { .. }
            );
        stage.hide_pointer = aiming
            && stage
                .from_stage(&at_bat.parts.main, stage.pointer.x, stage.pointer.y)
                .is_some_and(|(x, y)| {
                    let [left, top, right, bottom] = at_bat.parts.aim_box;
                    (left..=right).contains(&x) && (top..=bottom).contains(&y)
                });
        Match::still_batter(stage, &at_bat.parts.hitter, library);
        if let Some(glow) = self.mods.glow_of_the_bat() {
            // The mark on the bat glows, hotter the longer the run of hits.
            for mark in art::all_named(stage, &at_bat.parts.hitter, "batLogo") {
                if let Some(mark) = stage.child_mut(&mark) {
                    mark.set_color(glow);
                }
            }
        }
        Match::settle_fielders(&at_bat.parts, stage, library);
        for them in &at_bat.them {
            them.keep(stage);
        }
        at_bat.notices.fade(stage);
        if let Some(leads) = &mut at_bat.leads {
            leads.keep(self, self.phase == Phase::WindUp, stage);
            self.hold_stealers(stage);
        }
        if let Some(signs) = &mut at_bat.signs {
            signs.keep(stage);
        }
        if let (Some(meter), Some(left)) = (&at_bat.meter, self.bullet) {
            let full = rules.bullet_time.full.max(1) as f32;
            meter.keep(left as f32 / full, slowed, stage);
        }
        if at_bat.contact.is_none() {
            Match::aim(at_bat, stage);
            Match::point_hit(at_bat, &rules.hit, stage, library);
        }
    }

    /// The pitcher stands and waits, with `left` frames to go before he
    /// winds up. While he does, a click on the outfield calls the shot.
    pub(super) fn wait_for_the_wind_up(
        &mut self,
        at_bat: &mut AtBat,
        left: u32,
        pressed: Option<Point>,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        if let Some((x, y)) = pressed
            && self.mods.shots_are_called()
            && let Some(pointer) = stage.from_stage(&at_bat.parts.main, x, y)
        {
            let called = &mut at_bat.called;
            called::Called::call(called, pointer, &at_bat.parts, &game.rules, stage, library);
        }
        if left == 0 {
            stage.goto_label(&at_bat.parts.pitcher, PITCH, true, library);
            self.phase = Phase::WindUp;
            self.ask_for_steals(at_bat, stage, library);
        } else {
            self.phase = Phase::Settling { left: left - 1 };
        }
    }

    /// The pitcher winds up, shows where the pitch is going, and lets the
    /// ball go. While he winds up, a click on the little field sends a
    /// runner.
    pub(super) fn wind_up_and_throw(
        &mut self,
        at_bat: &mut AtBat,
        pressed: Option<Point>,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        if let Some((x, y)) = pressed
            && let Some(pointer) = stage.from_stage(&at_bat.parts.main, x, y)
        {
            self.steal_click(at_bat, pointer, stage, library);
        }
        let frame = frame_of(stage, &at_bat.parts.pitcher);
        if !at_bat.marker_shown && frame >= at_bat.table.marker_frame {
            at_bat.marker_shown = true;
            put(stage, &at_bat.parts.marker, at_bat.marker_at, 1.0);
        }
        if frame < game.rules.throw.release_frame {
            return;
        }
        show(stage, &at_bat.parts.ball, true);
        show(stage, &at_bat.parts.shadow, true);
        self.pitched += 1;
        self.mods.the_ball_was_thrown();
        self.stop_asking_for_steals(at_bat, stage);
        self.book_thrown();
        if let Some(arcade) = self.mode.arcade_mut() {
            arcade.left = arcade.left.saturating_sub(1);
        }
        self.show_numbers(stage);
        if let (Some(kind), Some(mystery)) = (at_bat.kind, &self.mods.mystery_pitch) {
            // Now it can be told what he threw.
            let top = (at_bat.parts.centre_x, MYSTERY_TOP);
            at_bat
                .notices
                .put(mystery.news(kind).at(top), &at_bat.parts, stage, library);
        }
        self.phase = Phase::Flight { step: 0 };
    }

    /// The next pitch has been asked for and the last has been cleared
    /// away: the art builds the view again, which starts the next pitch.
    pub(super) fn ask_for_a_new_view(
        &mut self,
        at_bat: &AtBat,
        stage: &mut Stage,
        library: &Library,
    ) {
        self.zinger_unseen();
        let mut holder = at_bat.parts.main.clone();
        holder.pop();
        stage.goto_clip(&holder, 1, library);
        if let Some(clip) = stage.clip_mut(&holder) {
            clip.playing = true;
        }
        self.phase = Phase::Arriving;
    }

    /// Moves the timing bar's marker on, where the bar is up.
    pub(super) fn point_the_timing_bar(&self, at_bat: &mut AtBat, game: &Game, stage: &mut Stage) {
        let Some(bar) = &mut at_bat.timing else {
            return;
        };
        match self.phase {
            // A swing made now begins on the step the flight has come to.
            Phase::Flight { step } => bar.point(step as i32, stage),
            // Until the ball is thrown, the steps to go are the frames left
            // of the wind-up.
            Phase::WindUp => {
                let frame = frame_of(stage, &at_bat.parts.pitcher);
                let to_go = i32::from(game.rules.throw.release_frame) - i32::from(frame);
                bar.point(-to_go, stage);
            }
            _ => {}
        }
    }

    /// Moves the aiming ring a step towards the pointer, and with it the
    /// art's pointer to where a hit would go.
    fn aim(at_bat: &mut AtBat, stage: &mut Stage) {
        let parts = &at_bat.parts;
        let pointer = (stage.pointer.x, stage.pointer.y);
        // A pointer nobody has moved yet is nowhere.
        if pointer.0 < -1.0e5 {
            return;
        }
        let Some(pointer) = stage.from_stage(&parts.main, pointer.0, pointer.1) else {
            return;
        };
        let [left, top, right, bottom] = parts.aim_box;
        let ease = at_bat.table.aim_ease.max(1.0);
        at_bat.aim.0 += (pointer.0 - at_bat.aim.0) / ease;
        at_bat.aim.1 += (pointer.1 - at_bat.aim.1) / ease;
        at_bat.aim.0 = at_bat.aim.0.clamp(left + 1.0, right - 1.0);
        at_bat.aim.1 = at_bat.aim.1.clamp(top + 1.0, bottom - 1.0);
        if let Some(ring) = stage.child_mut(&parts.aim) {
            ring.move_to(at_bat.aim.0, at_bat.aim.1);
        }
        let shadow_y = at(stage, &parts.aim_shadow).1;
        if let Some(shadow) = stage.child_mut(&parts.aim_shadow) {
            shadow.move_to(at_bat.aim.0, shadow_y);
        }
    }

    /// Works out where a hit made now would go sideways, and shows it.
    fn point_hit(at_bat: &mut AtBat, rules: &HitRules, stage: &mut Stage, library: &Library) {
        if at_bat.contact.is_some() {
            return;
        }
        let parts = &at_bat.parts;
        let pull = rules.pull;
        // Unless the rules say otherwise, the pointer is kept out of sight
        // until there is a crossing point for it to answer to.
        show(
            stage,
            &parts.aim_area,
            at_bat.marker_shown || rules.pointer_before_pitch,
        );
        // Until the player has been shown where this pitch will cross, the
        // pointer answers the ring as if it were coming down the middle.
        // It answers to where the player has been shown the ball crossing,
        // which is not always quite where it will.
        let crosses = if at_bat.marker_shown {
            at_bat.marker_at.0
        } else {
            parts.centre_x
        };
        at_bat.aim_area_x = hit_towards(crosses, at_bat.aim.0, parts.centre_x, pull);
        let y = at(stage, &parts.aim_area).1;
        if let Some(area) = stage.child_mut(&parts.aim_area) {
            area.move_to(at_bat.aim_area_x, y);
        }
        // The pointer's look is drawn for every position, one a frame.
        let frame = at_bat.aim_area_x.clamp(1.0, 550.0) as u16;
        stage.goto_clip(&parts.aim_area, frame, library);
        if let Some(clip) = stage.clip_mut(&parts.aim_area) {
            clip.playing = false;
        }
    }

    /// One frame of the ball on its way to the batter.
    pub(super) fn flight(
        &mut self,
        at_bat: &mut AtBat,
        step: usize,
        pressed: Option<Point>,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        let rules = &game.rules;
        let Some(&sample) = at_bat.pitch.samples.get(step) else {
            return self.call(at_bat, game, stage, library);
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

        if let Some((x, y)) = pressed
            && at_bat.swing.is_none()
        {
            // The swing is high, level or low by where the click was.
            let pointer = stage
                .from_stage(&at_bat.parts.main, x, y)
                .unwrap_or(at_bat.aim);
            let label = match pointer.1 {
                y if y <= 200.0 => "hitHigh",
                y if y <= 280.0 => "hitMed",
                _ => "hitLow",
            };
            stage.goto_label(&at_bat.parts.hitter, label, true, library);
            if !self.mode.is_arcade() {
                Match::sound(stage, library, "batSwing_fast");
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
        } else if let Some(frames) = &mut at_bat.swing {
            *frames += 1;
        }

        let in_band = sample.in_band(at_bat.table.band);
        let met = at_bat.swing.and_then(|frames| {
            let (quality, power) = pitch::meets(&at_bat.table, frames)?;
            Some((frames, quality, power))
        });
        if let (true, Some((frames, quality, power))) = (in_band, met) {
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
            let (hit, cheer): (&str, &[&str]) = match quality {
                Quality::Poor => ("batHit_poorly", &["crowd_smallClap"]),
                Quality::MediumPoor => ("batHit_mediumPoor", &["crowd_smallCheer"]),
                Quality::Medium => ("batHit_medium", &["crowd_smallCheer", "crowd_smallClap"]),
                Quality::Good => ("batHit_good", &["crowd_bigClap"]),
            };
            let (hit, cheer) = zinger.map_or((hit, cheer), |zinger| zinger.hit_sounds());
            Match::sound(stage, library, hit);
            for name in cheer {
                Match::sound(stage, library, name);
            }
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
            let contact = match &zinger {
                Some(zinger) => zinger.contact(aside),
                None => Contact {
                    power,
                    under: at_bat.under,
                    aside,
                },
            };
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
                None => Ball::hit(parts.home, mark, &contact, &rules.hit, &rules.field),
            });
            at_bat.zinger = zinger;
            if let (Some(arcade), Some(zinger)) = (self.mode.arcade_mut(), zinger) {
                // In the arcade game a zinger scores by how far it goes.
                arcade.owed = Some(zinger.feet);
            }
            show(stage, &parts.ball, false);
            show(stage, &parts.shadow, false);
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
            return;
        }
        self.phase = Phase::Flight { step: step + 1 };
    }

    /// The ball has gone by: a strike, or a ball.
    fn call(&mut self, at_bat: &mut AtBat, game: &Game, stage: &mut Stage, library: &Library) {
        let rules = &game.rules;
        let parts = at_bat.parts.clone();
        show(stage, &parts.ball, false);
        show(stage, &parts.shadow, false);
        if self.mode.is_arcade() {
            // No count in the arcade game: a miss is just a pitch gone.
            return self.ready(&parts, stage, library);
        }
        Match::sound(stage, library, "ballCatch_1");
        if !at_bat.pitch.in_zone && at_bat.swing.is_none() {
            self.book_pitch(at_bat, Thrown::Ball);
            self.balls += 1;
            if let Some(board) = &parts.scoreboard {
                self.play_section(board, "noBall", 261, stage, library);
            }
            self.show_numbers(stage);
            if self.balls >= rules.count.balls {
                self.phase = Phase::Walking {
                    left: rules.hit.walk_wait,
                };
            } else if self.anyone_stealing() {
                // The catcher has the ball, and a runner to throw out.
                self.show_steal(at_bat, game, stage, library);
            } else {
                self.ready(&parts, stage, library);
            }
            return;
        }
        let thrown = match at_bat.swing {
            Some(_) => Thrown::Swinging,
            None => Thrown::Called,
        };
        self.book_pitch(at_bat, thrown);
        self.strikes += 1;
        if at_bat.golden {
            // A strike on a golden ball is all the strikes there are.
            self.strikes = self.strikes.max(self.strikes_allowed(game));
        }
        self.mods.a_strike_was_called();
        if let Some(anim) = &parts.strike_anim {
            let label = format!("strike{}", self.strikes.min(3));
            stage.goto_label(anim, &label, false, library);
            // The badge plays for 69 frames and is then taken down. Left
            // up, it would play again and again over the scoreboard.
            self.put_away.push((anim.clone(), 68));
        }
        if let Some(board) = &parts.scoreboard {
            self.play_section(board, "strike", 136, stage, library);
        }
        if self.strikes >= self.strikes_allowed(game) {
            let call = ["1", "2", "3"][self.rng.below(3) as usize];
            Match::sound(stage, library, &format!("umpire_yourOuttaHere_{call}"));
            Match::sound(stage, library, "crowd_unhappy");
            if let Some(batter) = self.batter() {
                self.runners[batter].place = Place::Out;
            }
            self.outs += 1;
            self.mods.somebody_is_out();
            self.clear_count();
            self.announce = true;
            self.book_end(End::Strikeout, None);
        } else {
            Match::sound(stage, library, "umpire_Strike_grunt");
            if self.strikes + 1 == self.strikes_allowed(game) {
                let organ = ["baseball_organ_FX", "baseball_organ_tense_FX"];
                Match::sound(stage, library, organ[self.rng.below(2) as usize]);
            }
        }
        self.show_numbers(stage);
        if self.anyone_stealing() && self.outs < self.max_outs {
            return self.show_steal(at_bat, game, stage, library);
        }
        // With the side out, nobody has anywhere to steal to.
        self.steals_go_back(stage, library);
        // The call is left up for a moment before the next pitch is offered.
        self.phase = Phase::Called { left: 58 };
    }

    /// One frame of the ball leaving the bat, seen from behind the batter.
    pub(super) fn watch(
        &mut self,
        at_bat: &mut AtBat,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        let rules = &game.rules;
        let Some(contact) = at_bat.contact else {
            return;
        };
        let parts = &at_bat.parts;
        let (at_point, height, lift) = &mut at_bat.fly;
        at_point.0 -= (at_point.0 - at_bat.fly_target.0) / contact.power;
        at_point.1 -= (at_point.1 - at_bat.fly_target.1) / contact.power;
        *height += *lift;
        if *lift >= -5.0 {
            *lift -= rules.hit.gravity;
        }
        if *height < 0.0 {
            *height = 0.0;
            *lift = -*lift * rules.hit.bounce;
        }
        let size = (1.0 + (at_point.1 - parts.ground_y) / 190.0).max(0.05);
        put(stage, &parts.fly, *at_point, size);
        let across = at(stage, &parts.fly_ball).0;
        put(stage, &parts.fly_ball, (across, -*height), at_bat.fly_size);
        if let Some(left) = at_bat.run_in {
            if left == 0 {
                at_bat.run_in = None;
                stage.goto_label(&parts.hitter, "run", true, library);
                if self.mods.southpaw.is_some() {
                    southpaw::run(parts, stage, library);
                }
                let last = stage
                    .clip(&parts.hitter)
                    .map_or(1, |clip| clip.frame_count(library));
                self.cues.push(Cue {
                    path: parts.hitter.clone(),
                    frame: last,
                    rewind: false,
                });
            } else {
                at_bat.run_in = Some(left - 1);
            }
        }
        // The ball is already on its way over the field, out of sight.
        let ground = parts.ground(&rules.field);
        let mut at_wall = None;
        if let Some(ball) = &mut at_bat.ball {
            let happened = ball.step(parts.home, contact.miss(), &rules.field);
            if happened == Happened::Cleared {
                at_bat.over_wall = true;
            }
            if matches!(happened, Happened::Cleared | Happened::HitWall) {
                at_wall = Some((ground.across(ball.at), ball.height));
            }
        }
        if let Some((across, height)) = at_wall {
            self.strike_sign(at_bat, across, height, &rules.sign);
        }
    }

    /// The play is over: offers the next pitch.
    pub(crate) fn ready(&mut self, parts: &Parts, stage: &mut Stage, library: &Library) {
        if self.phase == Phase::Ready {
            return;
        }
        self.phase = Phase::Ready;
        stage.goto_clip(&parts.next, 2, library);
        self.show_numbers(stage);
    }
}
