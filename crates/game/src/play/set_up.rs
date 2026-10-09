//! Getting the batting view ready for a pitch.
//!
//! The art builds the view afresh for every pitch, so before each one
//! everything is put back where the last left it and the pitch is decided.
//! It is done a step at a time, in an order that matters: the steps take
//! the lines in the corner of the view from the top down, numbers by chance
//! are drawn in the order the steps come, and one step's sums are done on
//! what the step before left.

use bb_engine::library::Library;
use bb_engine::math::ColorTransform;
use bb_engine::stage::Stage;

use super::book::ORDER;
use super::mods::{GoldenBall, Line};
use super::overlay::{Notices, Says};
use super::pitch::{self, Choice, Kind, Mound, Pitch, Point};
use super::zinger::Zinger;
use super::{
    AtBat, Corner, FLUSH, MYSTERY_TOP, Match, Outcome, Parts, Phase, Place, Runner, at, bullet,
    full, hot_colour, shift, show, sign, southpaw, steal, timing,
};
use crate::look;
use crate::menu::Game;
use crate::mods::Mod;
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
        let mut parts = Match::parts(stage, library)?;
        self.cues.clear();
        self.put_away.clear();
        self.stand_the_batter(&mut parts, game, stage);
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
        self.heat_the_pitch(&mut coming, game, stage, library);
        self.tire_the_arm(&mut coming, game, stage, library);
        self.widen_for_a_hot_bat(&mut coming, game, stage, library);
        Match::say_the_ball_is_golden(&mut coming, stage, library);
        self.say_it_is_the_clutch(&mut coming, stage, library);
        self.say_what_a_rally_is_worth(&mut coming, stage, library);
        let meter = self.put_up_the_meter(&mut coming, game, stage, library);
        self.shift_the_fielders(&mut coming, game, stage, library);
        Match::clear_the_plate(&coming, stage);
        self.stand_the_runners(&coming, stage, library);
        let leads = self.mark_the_leads(&mut coming, game, stage, library);
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
        let timing = Match::put_up_the_timing_bar(&coming, &decided.pitch, game, stage, library);
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
    fn stand_the_batter(&mut self, parts: &mut Parts, game: &Game, stage: &mut Stage) {
        self.southpaw = game.mods.is_on(Mod::Southpaw);
        stage.upright_text = self.southpaw;
        if self.southpaw {
            southpaw::stand(parts, stage);
        }
    }

    /// Sends a new batter to the plate, if nobody is there.
    ///
    /// Each batter in a match has his own skin, and they carry the bat
    /// logos in turn. The arcade game's one batter is as chosen. A full
    /// match's nine come round again, each as he was.
    fn bring_up_a_batter(&mut self, game: &Game) {
        if self.batter().is_some() {
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
    fn heat_the_pitch(
        &mut self,
        coming: &mut Coming,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        if !game.mods.is_on(Mod::HeatCheck) {
            return;
        }
        let rules = &game.rules;
        let runs = self.score.saturating_sub(self.heat_score);
        self.heat = (self.heat + runs).min(rules.heat.most);
        self.heat_score = self.score;
        coming.table.speed = coming.table.speed.times(rules.heat.time(self.heat));
        if self.heat > 0 {
            let hot = self.heat as f32 / rules.heat.most.max(1) as f32;
            let colour = [0xff, (0xe0 as f32 - 0xa0 as f32 * hot) as u8, 0x30];
            let says = format!("HEAT {}", self.heat);
            let top = coming.corner.line();
            coming.notices.put(
                Says::line("heat", &says, colour).at(top),
                &coming.parts,
                stage,
                library,
            );
        }
    }

    /// With the tired arm mod on, the pitcher is slower and wilder the more
    /// he has thrown, and one who has thrown his last gives way to a fresh
    /// one. The arcade game is over before any arm tires.
    fn tire_the_arm(
        &mut self,
        coming: &mut Coming,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        if !game.mods.is_on(Mod::TiredArm) || self.mode.is_arcade() {
            return;
        }
        let arm = &game.rules.tired_arm;
        if self.arm >= arm.relief.max(1) {
            self.arm = 0;
            self.relieved += 1;
            Match::sound(stage, library, "baseball_organ_FX");
            coming.notices.put(
                Says::news(
                    "newPitcher",
                    "NEW PITCHER",
                    [0xc8, 0xf0, 0xff],
                    arm.told_time,
                )
                .at((coming.parts.centre_x, MYSTERY_TOP)),
                &coming.parts,
                stage,
                library,
            );
        }
        let tired = arm.tired(self.arm);
        self.tired = Some(tired);
        coming.table = arm.pitch(&coming.table, tired);
        if let Some(pitcher) = stage.child_mut(&coming.parts.pitcher) {
            let left = 1.0 - FLUSH * tired;
            pitcher.set_color(ColorTransform {
                mult: [1.0, left, left, 1.0],
                add: [0.0; 4],
            });
        }
        // From white, through yellow, to red.
        let colour = [
            0xff,
            (0xff as f32 - 0x90 as f32 * tired) as u8,
            (0xff as f32 - 0xc0 as f32 * tired.min(0.5) * 2.0) as u8,
        ];
        coming.notices.put(
            Says::line("pitches", &format!("PITCHES {}", self.arm), colour)
                .at(coming.corner.line()),
            &coming.parts,
            stage,
            library,
        );
    }

    /// With the hot bat mod on, every hit in a row has widened the window
    /// by a frame at each end.
    fn widen_for_a_hot_bat(
        &self,
        coming: &mut Coming,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        if !game.mods.is_on(Mod::HotBat) || self.streak == 0 {
            return;
        }
        let most = game.rules.hot_bat.most;
        let more = self.streak.min(most);
        coming.table.window = pitch::widened(&coming.table.window, more);
        let says = format!("HOT BAT {more}");
        let colour = hot_colour(more, most);
        coming.notices.put(
            Says::line("hotBat", &says, colour).at(coming.corner.line()),
            &coming.parts,
            stage,
            library,
        );
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
        if self.strikes + self.balls == 0 {
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
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<bullet::Meter> {
        if !game.mods.is_on(Mod::BulletTime) {
            self.bullet = None;
            return None;
        }
        let rules = &game.rules.bullet_time;
        self.bullet.get_or_insert(rules.full);
        let (top, under) = (coming.corner.line(), coming.corner.line());
        let under = (under.0, under.1 + 2.0);
        let meter = bullet::Meter::put(&coming.parts, top, under, stage, library);
        if let (Some(meter), Some(left)) = (&meter, self.bullet) {
            let full = rules.full.max(1) as f32;
            meter.keep(left as f32 / full, false, stage);
        }
        meter
    }

    /// With the shift on, the fielders stand where the last few balls went.
    /// The arcade game has no fielders to move.
    fn shift_the_fielders(
        &mut self,
        coming: &mut Coming,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        if !game.mods.is_on(Mod::TheShift) || self.mode.is_arcade() {
            return;
        }
        let rules = &game.rules;
        let shift = shift::Shift::of(&self.spray, &rules.shift);
        self.shift = shift.by();
        shift.place(&coming.parts, &rules.field, stage, library);
        if let Some(says) = shift.words(&rules.shift) {
            coming.notices.put(
                Says::line("shift", says, [0xc8, 0xf0, 0xff]).at(coming.corner.line()),
                &coming.parts,
                stage,
                library,
            );
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
        self.steal_play = false;
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
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<steal::Leads> {
        let on_base = |runner: &Runner| matches!(runner.place, Place::Base(_));
        if !game.mods.is_on(Mod::StolenBases) || !self.runners.iter().any(on_base) {
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
        self.sign_struck = None;
        if !game.mods.is_on(Mod::HitTheSign) || self.mode.is_arcade() {
            return None;
        }
        let rules = &game.rules;
        let parts = &coming.parts;
        let signs = sign::Signs::of(&rules.sign);
        let lit = self.lit_sign(&signs);
        let ground = parts.ground(&rules.field);
        let (sign, field) = (&rules.sign, &rules.field);
        sign::Board::put(&signs, lit, sign, field, parts, &ground, stage, library)
    }

    /// The marks the art keeps on the field: whether a runner is on second,
    /// the fielders who mind the bases standing ready at them, and the
    /// runs to get called out when a side comes in.
    fn mark_the_field(&mut self, coming: &Coming, stage: &mut Stage, library: &Library) {
        let parts = &coming.parts;
        if let Some(mark) = stage.find(&parts.main, &["runnerOnSecond"]) {
            let label = if self.on_base(2).is_some() {
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
        let mut kind = None;
        if game.mods.is_on(Mod::MysteryPitch) {
            let which = Kind::ALL[self.rng.below(Kind::ALL.len() as u32) as usize];
            which.shape(table, &rules.mystery, self.rng.below(2) == 0);
            table.marker_frame = rules.throw.release_frame;
            kind = Some(which);
        }
        let mut choice = Choice::pick(table, &rules.throw, &mut self.rng);
        if self.southpaw {
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
        if game.mods.is_on(Mod::Knuckleball) {
            let knuckle = &rules.knuckleball;
            let start = self.rng.unit();
            pitch.knuckle(knuckle.sway, knuckle.turns, start, mound);
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
        coming: &Coming,
        pitch: &Pitch,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<timing::Indicator> {
        if !game.mods.is_on(Mod::TimingIndicator) {
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
            game.mods.is_on(Mod::ZingerHit).then_some(&feet);
        timing::Indicator::new(pitch, table, parts, feet, stage, library)
    }
}
