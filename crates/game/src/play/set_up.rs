//! Getting the batting view ready for a pitch.
//!
//! The art builds the view afresh for every pitch, so before each one
//! everything is put back where the last left it and the pitch is decided.
//! It is done a step at a time, in an order that matters: the steps take
//! the lines in the corner of the view from the top down, numbers by chance
//! are drawn in the order the steps come, and one step's sums are done on
//! what the step before left.

use bb_engine::library::Library;
use bb_engine::stage::Stage;

use super::book::ORDER;
use super::mods::{GoldenBall, Line, TiredArm, southpaw};
use super::overlay::{Corner, Notices, Says};
use super::pitch::{Choice, Kind, Mound, Pitch, Point};
use super::zinger::Zinger;
use super::{
    AtBat, MYSTERY_TOP, Match, Outcome, Parts, Phase, Place, Runner, at, bullet, full, show, sign,
    steal, timing,
};
use crate::look;
use crate::menu::Game;
use crate::rules::PitchRules;

/// What is being got ready for the pitch that is coming, as each step adds
/// what it has to add.
struct Coming {
    parts: Parts,
    /// The numbers the pitch is picked by, which the mods change.
    table: PitchRules,
    notices: Notices,
    /// Where the next line in the corner of the view goes.
    corner: Corner,
    /// Whether the pitch is a golden ball.
    golden: bool,
}

impl Coming {
    /// Writes a mod's line in the corner of the view, under the lines
    /// already there.
    fn write(&mut self, line: &Line, stage: &mut Stage, library: &Library) {
        let top = self.corner.line();
        self.notices.put(
            Says::line(line.name, &line.words, line.colour).at(top),
            &self.parts,
            stage,
            library,
        );
    }
}

/// The pitch that has been decided on.
struct Decided {
    /// What a mystery pitch turned out to be.
    kind: Option<Kind>,
    pitch: Pitch,
    /// Where the marker of where it will cross is shown.
    marker_at: Point,
    /// How many frames the pitcher waits for it, on top of what he always
    /// waits.
    wait: usize,
}

impl Match {
    /// Gets a freshly built batting view ready for a pitch. Returns how the
    /// match ended if it has.
    pub(super) fn set_up(
        &mut self,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<Outcome> {
        let mut parts = Parts::find(stage, library)?;
        self.cues.clear();
        self.put_away.clear();
        self.stand_the_batter(&mut parts, stage);
        if let Some(outcome) = self.outcome() {
            self.phase = Phase::Over;
            return Some(self.close_half(outcome));
        }
        self.bring_up_a_batter(game);

        let mut coming = Coming {
            parts,
            table: game.rules.pitch.at(game.settings.difficulty).clone(),
            notices: Notices::default(),
            corner: Corner::default(),
            golden: false,
        };
        self.say_the_innings(&mut coming, stage, library);
        self.gild_the_ball(&mut coming, stage);
        self.heat_the_pitch(&mut coming, stage, library);
        self.tire_the_arm(&mut coming, stage, library);
        self.widen_for_a_hot_bat(&mut coming, stage, library);
        Match::say_the_ball_is_golden(&mut coming, stage, library);
        self.say_it_is_the_clutch(&mut coming, stage, library);
        self.say_what_a_rally_is_worth(&mut coming, stage, library);
        let meter = self.put_up_the_meter(&mut coming, stage, library);
        self.shift_the_fielders(&mut coming, game, stage, library);
        Match::clear_the_plate(&coming, stage);
        self.stand_the_runners(&coming, stage, library);
        let leads = self.mark_the_leads(&mut coming, stage, library);
        let signs = self.put_up_the_signs(&coming, game, stage, library);
        self.mark_the_field(&coming, stage, library);
        let them = match self.mode.full() {
            Some(_) => full::Them::put(&coming.parts, stage, library),
            None => Vec::new(),
        };
        self.set_up_arcade(&coming.parts, game, stage, library);
        self.show_numbers(stage);

        let mound = Match::mound(&coming.parts, stage, library)?;
        let decided = self.decide_the_pitch(&mut coming, &mound, game);
        let timing = self.put_up_the_timing_bar(&coming, &decided.pitch, game, stage, library);
        self.phase = Phase::Settling {
            left: game.rules.throw.settle + decided.wait as u32,
        };
        let Coming {
            parts,
            table,
            notices,
            golden,
            ..
        } = coming;
        self.at = Some(AtBat {
            aim: at(stage, &parts.aim),
            aim_area_x: at(stage, &parts.aim_area).0,
            parts,
            table,
            pitch: decided.pitch,
            marker_shown: false,
            marker_at: decided.marker_at,
            kind: decided.kind,
            golden,
            called: None,
            swing: None,
            under: 0.0,
            across: 0.0,
            contact: None,
            fly: ((0.0, 0.0), 0.0, 0.0),
            fly_size: 1.0,
            fly_target: (0.0, 0.0),
            run_in: None,
            ball: None,
            fielding: None,
            timing,
            zinger: None,
            zinger_show: None,
            over_wall: false,
            notices,
            came_down: None,
            rebounds: 0,
            them,
            swing_off: None,
            met: None,
            leads,
            signs,
            meter,
        });
        None
    }

    /// With the southpaw mod on the batter stands on the other side of the
    /// plate, turned round. The number on his shirt is not.
    fn stand_the_batter(&mut self, parts: &mut Parts, stage: &mut Stage) {
        stage.upright_text = self.mods.southpaw.is_some();
        if let Some(southpaw) = &mut self.mods.southpaw {
            southpaw.stand(parts, stage);
        }
    }

    /// Sends a new batter to the plate, if nobody is there.
    ///
    /// Each batter in a match has his own skin, and they carry the bat
    /// logos in turn. The arcade game's one batter is as chosen. A full
    /// match's nine come round again, each as he was.
    fn bring_up_a_batter(&mut self, game: &Game) {
        if self.runners.batter().is_some() {
            return;
        }
        let team = &game.rules.team;
        let order = match self.mode.full() {
            Some(_) => self.came_up % ORDER,
            None => self.came_up,
        };
        let (skin, logo) = if self.mode.is_arcade() {
            (game.settings.skin, game.settings.logo.clone())
        } else {
            let known = self.mode.full().and(self.line_up.get(order).copied());
            let skin = known.unwrap_or_else(|| {
                let pick = self.rng.below(team.skins.len() as u32) as usize;
                team.skins.get(pick).and_then(|skin| look::rgb(skin))
            });
            if self.mode.full().is_some() && self.line_up.len() <= order {
                self.line_up.resize(order + 1, None);
                self.line_up[order] = skin;
            }
            (
                skin,
                team.logos.get(order % team.logos.len().max(1)).cloned(),
            )
        };
        self.came_up += 1;
        self.runners.push(Runner {
            place: Place::AtBat,
            running_to: None,
            sliding: false,
            runs: 0,
            order,
            stole_from: None,
            skin,
            logo,
            path: None,
        });
        self.clear_count();
    }

    /// A full match says which half of which innings this is, and what the
    /// mods say goes under that.
    fn say_the_innings(&self, coming: &mut Coming, stage: &mut Stage, library: &Library) {
        if let Some(full) = self.mode.full() {
            coming.notices.put(
                Says::line("innings", &full.half_words(), [0xfd, 0xf6, 0xc0])
                    .at(coming.corner.line()),
                &coming.parts,
                stage,
                library,
            );
        }
    }

    /// Settles whether the pitch is a golden ball, and with that what a run
    /// is worth on it, and colours a golden ball gold.
    ///
    /// The pitch about to be thrown is one more than have been. The arcade
    /// game has no runs and no outs for a golden ball to change.
    fn gild_the_ball(&mut self, coming: &mut Coming, stage: &mut Stage) {
        let golden = self.mods.is_golden(self.pitched + 1);
        coming.golden = golden;
        self.run_worth = self.worth_of_a_run(golden);
        if golden {
            GoldenBall::gild(&coming.parts, stage);
        }
    }

    /// With the heat check mod on, every run since the last pitch makes
    /// this one faster.
    fn heat_the_pitch(&mut self, coming: &mut Coming, stage: &mut Stage, library: &Library) {
        if let Some(line) = self.mods.heat_the_pitch(self.score, &mut coming.table) {
            coming.write(&line, stage, library);
        }
    }

    /// With the tired arm mod on, the pitcher is slower and wilder the more
    /// he has thrown, and one who has thrown his last gives way to a fresh
    /// one.
    fn tire_the_arm(&mut self, coming: &mut Coming, stage: &mut Stage, library: &Library) {
        let Some(arm) = &mut self.mods.tired_arm else {
            return;
        };
        if arm.relieve() {
            Match::sound(stage, library, "baseball_organ_FX");
            coming.notices.put(
                arm.news().at((coming.parts.centre_x, MYSTERY_TOP)),
                &coming.parts,
                stage,
                library,
            );
        }
        let tired = arm.tire(&mut coming.table);
        if let Some(pitcher) = stage.child_mut(&coming.parts.pitcher) {
            pitcher.set_color(TiredArm::flush(tired));
        }
        coming.write(&arm.line(tired), stage, library);
    }

    /// With the hot bat mod on, every hit in a row has widened the window
    /// by a frame at each end.
    fn widen_for_a_hot_bat(&self, coming: &mut Coming, stage: &mut Stage, library: &Library) {
        if let Some(line) = self.mods.widen_for_a_hot_bat(&mut coming.table) {
            coming.write(&line, stage, library);
        }
    }

    fn say_the_ball_is_golden(coming: &mut Coming, stage: &mut Stage, library: &Library) {
        if coming.golden {
            coming.write(&GoldenBall::line(), stage, library);
        }
    }

    /// Settles whether this pitch is thrown in the clutch, and says so.
    fn say_it_is_the_clutch(&mut self, coming: &mut Coming, stage: &mut Stage, library: &Library) {
        let in_it = self.in_the_clutch();
        let Some(line) = self.mods.settle_the_clutch(in_it) else {
            return;
        };
        // The organ plays as the batter comes up to it, and not again for
        // every pitch to him.
        if self.count.is_clean() {
            Match::sound(stage, library, "baseball_organ_tense_FX");
        }
        coming.write(&line, stage, library);
    }

    /// Says what runs are worth while a rally is on.
    fn say_what_a_rally_is_worth(&self, coming: &mut Coming, stage: &mut Stage, library: &Library) {
        if let Some(line) = self.mods.rally_line() {
            coming.write(&line, stage, library);
        }
    }

    /// With bullet time on there is a meter, full when the game starts.
    fn put_up_the_meter(
        &mut self,
        coming: &mut Coming,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<bullet::Meter> {
        let bullet = self.mods.bullet_time.as_mut()?;
        bullet.fill_at_the_start();
        let (top, under) = (coming.corner.line(), coming.corner.line());
        let under = (under.0, under.1 + 2.0);
        let meter = bullet::Meter::put(&coming.parts, top, under, stage, library);
        if let (Some(meter), Some(left)) = (&meter, bullet.share_left()) {
            meter.keep(left, false, stage);
        }
        meter
    }

    /// With the shift on, the fielders stand where the last few balls went.
    fn shift_the_fielders(
        &mut self,
        coming: &mut Coming,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        let Some(the_shift) = &mut self.mods.the_shift else {
            return;
        };
        let shift = the_shift.stand();
        shift.place(&coming.parts, &game.rules.field, stage, library);
        if let Some(line) = the_shift.line(shift) {
            coming.write(&line, stage, library);
        }
    }

    /// The ball is out of sight until it is thrown, and the strike zone is
    /// drawn only where the skill level shows it.
    fn clear_the_plate(coming: &Coming, stage: &mut Stage) {
        let parts = &coming.parts;
        show(stage, &parts.ball, false);
        show(stage, &parts.shadow, false);
        if let Some(zone) = stage.find(&parts.main, &["strikeZone"]) {
            show(stage, &zone, coming.table.show_zone);
        }
    }

    /// Every runner still in the game stands where the last pitch left him.
    fn stand_the_runners(&mut self, coming: &Coming, stage: &mut Stage, library: &Library) {
        self.mods.a_steal_is_in_play(false);
        for index in 0..self.runners.len() {
            self.runners[index].path = None;
            self.runners[index].running_to = None;
            self.runners[index].stole_from = None;
            self.runners[index].sliding = false;
            let label = match self.runners[index].place {
                Place::AtBat => "waiting".to_owned(),
                Place::Base(base) => format!("base{base}"),
                Place::Out | Place::Home => continue,
            };
            let Some(symbol) = self.runner_symbol else {
                continue;
            };
            let Some(holder) = &coming.parts.holder else {
                continue;
            };
            let depth = Stage::RULES_DEPTH + index as u16;
            let name = format!("runner{}", index + 1);
            if let Some(path) = stage.attach(holder, symbol, depth, &name, library) {
                stage.goto_label(&path, &label, false, library);
                self.runners[index].path = Some(path);
            }
        }
    }

    /// With the stolen bases mod on, each runner is marked on the little
    /// field in the corner, and the mod has a line under it to write on.
    fn mark_the_leads(
        &self,
        coming: &mut Coming,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<steal::Leads> {
        if !self.mods.runners_steal() || self.runners.on_the_bases() == 0 {
            return None;
        }
        let under = coming.corner.line();
        steal::Leads::put(&self.runners, under, &coming.parts, stage, library)
    }

    /// With the hit the sign mod on, the wall has its signs, one lit for
    /// the innings. The arcade game has no runs for a sign to be worth.
    fn put_up_the_signs(
        &mut self,
        coming: &Coming,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<sign::Board> {
        self.mods.a_new_pitch_is_coming();
        let rules = &game.rules;
        let parts = &coming.parts;
        let signs = sign::Signs::of(&rules.sign);
        let lit = self.lit_sign(&signs)?;
        sign::Board::put(&signs, lit, rules, parts, stage, library)
    }

    /// The marks the art keeps on the field: whether a runner is on second,
    /// the fielders who mind the bases standing ready at them, and the
    /// runs to get called out when a side comes in.
    fn mark_the_field(&mut self, coming: &Coming, stage: &mut Stage, library: &Library) {
        let parts = &coming.parts;
        if let Some(mark) = stage.find(&parts.main, &["runnerOnSecond"]) {
            let label = if self.runners.on_base(2).is_some() {
                "full"
            } else {
                "none"
            };
            stage.goto_label(&mark, label, false, library);
        }
        for fielder in parts.fielders.iter().skip(5) {
            stage.goto_label(fielder, "baseWaiting", false, library);
        }
        if self.announce {
            self.announce = false;
            // In a full match there is a number of runs that wins it only
            // when getting ahead ends it.
            let to_win = self.mode.full().is_none_or(|full| full.sudden());
            if let (true, Some(board)) = (to_win, parts.scoreboard.clone()) {
                self.play_section(&board, "runsToGet", 361, stage, library);
            }
        }
    }

    /// Decides the pitch: what kind it is, where it is aimed, how it flies.
    /// Nothing here touches the stage.
    fn decide_the_pitch(&mut self, coming: &mut Coming, mound: &Mound, game: &Game) -> Decided {
        let rules = &game.rules;
        let table = &mut coming.table;
        // A pitcher waits longer before a slower pitch. A mystery pitch
        // would be no mystery if he did, so before one he waits as long as
        // for a pitch of the usual pace, and the marker is not shown until
        // the ball has left his hand.
        let usual_pace = table.speed.high as f32;
        let release = rules.throw.release_frame;
        let kind = self
            .mods
            .mystery_pitch
            .as_ref()
            .map(|mystery| mystery.pick(table, release, &mut self.rng));
        let mut choice = Choice::pick(table, &rules.throw, &mut self.rng);
        if self.mods.southpaw.is_some() {
            // A left-hander is pitched to as a right-hander was.
            southpaw::turn(&mut choice, coming.parts.centre_x);
        }
        let mut pitch = Pitch::throw(&choice, mound, &rules.throw);
        let wait = match kind {
            Some(_) => {
                let usual = Choice {
                    speed: usual_pace,
                    ..choice
                };
                Pitch::throw(&usual, mound, &rules.throw).samples.len()
            }
            None => pitch.samples.len(),
        };
        // The marker shows where the pitch was going before a knuckleball
        // began to sway.
        let marker_at = pitch.crosses;
        if let Some(knuckleball) = &self.mods.knuckleball {
            let start = self.rng.unit();
            knuckleball.sway(&mut pitch, start, mound);
        }
        Decided {
            kind,
            pitch,
            marker_at,
            wait,
        }
    }

    /// With the timing indicator on, the bar that shows when to swing. With
    /// the zinger mod on as well, it says how far a swing on each of its
    /// colours sends the ball at the most.
    fn put_up_the_timing_bar(
        &self,
        coming: &Coming,
        pitch: &Pitch,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<timing::Indicator> {
        if !self.mods.the_timing_bar_is_shown() {
            return None;
        }
        let (table, parts) = (&coming.table, &coming.parts);
        let difficulty = game.settings.difficulty;
        let feet = |frames: u32| {
            Zinger::of(
                table,
                frames,
                (0.0, 0.0),
                parts.home,
                difficulty,
                &game.rules,
            )
            .map(|zinger| zinger.feet)
        };
        let feet: Option<&dyn Fn(u32) -> Option<u32>> =
            self.mods.every_hit_is_a_home_run().then_some(&feet);
        timing::Indicator::new(pitch, table, parts, feet, stage, library)
    }
}
