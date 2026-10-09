//! The overhead view: the ball in play, the fielder going after it, the
//! throws to the bases, and the runners.

use bb_engine::display::{Content, Path};
use bb_engine::library::Library;
use bb_engine::stage::Stage;
use bb_format::SymbolId;

use super::book::{End, Hit, Thrown};
use super::field::{Ball, Facing, Happened, distance, reach, seen_size};
use super::overlay::{Notices, Says};
use super::pinball;
use super::pitch::Point;
use super::zinger;
use super::{AtBat, Match, Parts, Phase, Place, at, frame_of, put, show};
use crate::look::Rgb;
use crate::menu::Game;

/// What the fielder with the ball, or going for it, is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Job {
    /// Running to where the ball will come down, or after it once it has.
    Chase,
    /// Standing under a ball still in the air.
    WaitCatch,
    PickUp {
        left: u32,
    },
    /// Drawing back to throw.
    WindUp {
        left: u32,
    },
    /// The ball is in the air between fielders.
    Throwing {
        step: Point,
    },
    /// He has let the ball go, and is at a loss for a moment before he
    /// goes after it.
    Fumbling {
        left: u32,
    },
    /// At a base, gathering a throw he dropped.
    Gather {
        left: u32,
    },
    Rest,
}

pub(crate) struct Fielding {
    /// Nobody hit it: the batter walks to first on four balls.
    walk: bool,
    foul: bool,
    home_run: bool,
    /// A zinger is over the wall, and nothing is called until it comes
    /// down.
    gone: bool,
    /// The outfielders on their way back to the wall to watch a zinger go
    /// over it, counting from 0.
    watchers: Vec<usize>,
    /// The ball is in play: runners may be put out, and may go on.
    live: bool,
    caught: bool,
    /// Which fielder has the job, counting from 0.
    fielder: usize,
    job: Job,
    /// Where the ball will first come down.
    land: Point,
    facing: Facing,
    /// The base the ball is being thrown to, home being 4.
    throw_to: u8,
    frames: u32,
    /// Frames since the play was settled by a foul or a home run.
    since_settled: u32,
    /// The ball on the ground has been fumbled once, and will not be
    /// again before somebody has hold of it.
    fumbled: bool,
    /// Who hit the ball: his place among the runners.
    batter: Option<usize>,
    /// A fielder had the ball in his glove before it came down, and let it
    /// go.
    dropped: bool,
    /// Nobody hit it: the catcher is throwing to a base that a runner is
    /// stealing.
    steal: bool,
}

/// The runner clip's frame labels for running and sliding to each base.
const RUN: [&str; 4] = ["runToFirst", "runToSecond", "runToThird", "runToFourth"];
const SLIDE: [&str; 4] = [
    "slideToFirst",
    "slideToSecond",
    "slideToThird",
    "slideToFourth",
];
/// The frame on which a runner gets to each base, the frame his slide ends
/// on, and the frame a slide rejoins the run at.
pub(super) const ARRIVES: [u16; 4] = [211, 421, 630, 840];
const SLIDE_ENDS: [u16; 4] = [873, 890, 906, 922];
const SLIDE_JOINS: [u16; 4] = [210, 420, 629, 839];
/// Frames from here on are slides, not the run round the bases.
const FIRST_SLIDE_FRAME: u16 = 850;
/// The buttons that send a runner on from first, second and third.
const RUN_BUTTONS: [SymbolId; 3] = [1501, 1503, 1520];
const SLIDE_BUTTONS: [SymbolId; 5] = [1494, 1495, 1502, 1519, 1521];

/// The fielder who stands on the mound, counting from 0 as
/// [`Fielding::fielder`] does: the art's `fielder3`.
const PITCHER: usize = 2;
/// The fielder behind the plate, the same way: the art's `fielder9`.
const CATCHER: usize = 8;
/// How many of the five in the field are outfielders: the ones who stand
/// furthest from home.
const OUTFIELDERS: usize = 3;
/// How far short of the wall an outfielder stops to watch a zinger go over
/// it, as the field measures distance.
const WARNING_TRACK: f32 = 40.0;
/// The word that goes up over a fielder who has let the ball go: how far
/// over him its top is, in pixels of the view, the size of its lettering,
/// its own being 1, how near the sides of the view its middle may come,
/// and its colour.
const TOLD_ABOVE: f32 = 30.0;
const TOLD_SIZE: f32 = 0.85;
const TOLD_MARGIN: f32 = 55.0;
const TOLD_COLOUR: Rgb = [0xff, 0x9a, 0x3c];
/// Where the view of the field says that a called shot came off: how far
/// down, and in what colour.
const CALLED_TOP: f32 = 232.0;
const CALLED_COLOUR: Rgb = [0xff, 0xe2, 0x4a];

/// The frames of a fielder on which he picks the ball up, throws it or
/// catches it. Each shows a clip inside him that is meant to play once.
const ONCE_ONLY: std::ops::RangeInclusive<u16> = 46..=145;

impl Match {
    /// Stops a fielder's pick-up, throw or catch when it has played.
    ///
    /// Each is a clip inside the fielder, and the art stopped most of them
    /// from scripts on their last frames. Left alone they start again, and
    /// he throws the same ball over and over.
    pub(crate) fn settle_fielders(parts: &Parts, stage: &mut Stage, library: &Library) {
        let mut played = Vec::new();
        for fielder in &parts.fielders {
            let Some(clip) = stage.clip(fielder) else {
                continue;
            };
            if !ONCE_ONLY.contains(&clip.frame) {
                continue;
            }
            for (&depth, child) in &clip.children {
                if let Content::Clip(part) = &child.content
                    && part.playing
                    && part.frame_count(library) > 1
                    && part.frame >= part.frame_count(library)
                {
                    let mut path = fielder.clone();
                    path.push(depth);
                    played.push(path);
                }
            }
        }
        for path in played {
            if let Some(part) = stage.clip_mut(&path) {
                part.playing = false;
            }
        }
    }

    /// Sets a runner off for a base.
    pub(super) fn send(&mut self, runner: usize, to: u8, stage: &mut Stage, library: &Library) {
        self.runners[runner].running_to = Some(to);
        self.runners[runner].sliding = false;
        if let Some(path) = &self.runners[runner].path {
            stage.goto_label(path, RUN[usize::from(to) - 1], true, library);
        }
    }

    /// The batter runs to first, and pushes on anyone in his way.
    fn start_runners(&mut self, stage: &mut Stage, library: &Library) {
        let mut going = Vec::new();
        if let Some(batter) = self.batter() {
            going.push((batter, 1));
        }
        for base in 1..=3 {
            // Only a runner with someone coming up behind him has to go.
            match self.on_base(base) {
                Some(runner) if going.len() == usize::from(base) => {
                    going.push((runner, base + 1));
                }
                _ => break,
            }
        }
        for (runner, to) in going {
            self.send(runner, to, stage, library);
        }
    }

    /// Puts a runner out.
    fn put_out(&mut self, runner: usize, stage: &mut Stage, library: &Library) {
        let stealing = self.runners[runner].stole_from.take();
        let making_for = self.runners[runner].running_to.take();
        self.runners[runner].place = Place::Out;
        self.outs += 1;
        self.mods.somebody_is_out();
        match (stealing, making_for) {
            // A runner caught stealing is out, and the batter's count is
            // as it was.
            (Some(_), Some(base)) if self.mods.is_a_steal_in_play() => {
                self.stole(runner, base, false);
            }
            _ => self.clear_count(),
        }
        self.announce = true;
        let Some(path) = self.runners[runner].path.clone() else {
            return;
        };
        if let Some(clip) = stage.clip_mut(&path) {
            clip.playing = false;
        }
        for name in ["runner", "btn_slide", "runBtn"] {
            if let Some(part) = stage.find(&path, &[name]) {
                show(stage, &part, false);
            }
        }
        // "OUT" comes up over him, and he walks off.
        for name in ["outText", "outWalk"] {
            if let Some(part) = stage.find(&path, &[name]) {
                stage.goto_clip(&part, 2, library);
                if let Some(clip) = stage.clip_mut(&part) {
                    clip.playing = true;
                }
            }
        }
    }

    /// A runner has got to the base he was running to.
    fn arrive(&mut self, runner: usize, parts: &Parts, stage: &mut Stage, library: &Library) {
        let Some(base) = self.runners[runner].running_to.take() else {
            return;
        };
        let was_batting = self.runners[runner].place == Place::AtBat;
        let path = self.runners[runner].path.clone();
        self.runners[runner].sliding = false;
        if self.runners[runner].stole_from.take().is_some() && self.mods.is_a_steal_in_play() {
            self.stole(runner, base, true);
        }
        if base == 4 {
            self.runners[runner].place = Place::Home;
            self.runners[runner].runs += self.run_worth;
            self.score += self.run_worth;
            if let Some(path) = &path {
                stage.goto_label(path, "addRun", false, library);
                if let Some(walk) = stage.find(path, &["outWalk"]) {
                    stage.goto_clip(&walk, 2, library);
                    if let Some(clip) = stage.clip_mut(&walk) {
                        clip.playing = true;
                    }
                }
            }
        } else {
            self.runners[runner].place = Place::Base(base);
            if let Some(path) = &path {
                stage.goto_label(path, &format!("base{base}"), false, library);
            }
            let umpire = parts.umpires[usize::from(base) - 1].clone();
            self.play_section(&umpire, "safe", 49, stage, library);
        }
        if was_batting {
            self.mods.the_batter_reached_base();
            self.clear_count();
        }
        self.show_numbers(stage);
    }

    /// Changes the view to the field, for a ball that was hit or a walk.
    pub(crate) fn show_field(
        &mut self,
        at_bat: &mut AtBat,
        walk: bool,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        let rules = &game.rules;
        let parts = at_bat.parts.clone();
        at_bat.leave_batting_view(stage);
        let y = at(stage, &parts.field).1;
        if let Some(field) = stage.child_mut(&parts.field) {
            field.move_to(rules.field.x, y);
        }
        let mut fielding = Fielding {
            walk,
            foul: false,
            home_run: false,
            gone: false,
            watchers: Vec::new(),
            live: !walk,
            caught: false,
            fielder: 0,
            job: Job::Rest,
            land: parts.home,
            facing: Facing::Down,
            throw_to: 1,
            frames: 0,
            since_settled: 0,
            fumbled: false,
            batter: self.batter(),
            dropped: false,
            steal: false,
        };
        self.phase = Phase::Fielding;

        if walk {
            show(stage, &parts.field_ball, false);
            self.steals_on_a_walk();
            self.start_runners(stage, library);
        } else if let (Some(ball), Some(contact)) = (at_bat.ball, at_bat.contact) {
            let mark_x = parts.field_mark.0 + contact.aside / rules.field.aim_share;
            if mark_x < parts.foul.0 || mark_x > parts.foul.1 {
                // A foul is a strike, but never the last one.
                fielding.foul = true;
                fielding.live = false;
                self.steals_go_back(stage, library);
                if self.strikes + 1 < self.strikes_allowed(game) {
                    self.strikes += 1;
                    self.mods.a_foul_took_a_strike();
                }
                stage.goto_label(&parts.transitions, "foulHit", true, library);
            } else {
                fielding.land = if ball.bounced {
                    ball.at
                } else {
                    ball.landing(parts.home, contact.miss(), &rules.field)
                };
                // Whoever of the five in the field is nearest goes for it,
                // unless the pitcher has been left to do it all.
                fielding.fielder = if self.mods.the_pitcher_fields_alone() {
                    PITCHER
                } else {
                    (0..5)
                        .min_by(|&a, &b| {
                            let far = |index: usize| {
                                distance(at(stage, &parts.fielders[index]), fielding.land)
                            };
                            far(a).total_cmp(&far(b))
                        })
                        .unwrap_or(0)
                };
                fielding.job = Job::Chase;
                self.steals_are_runs();
                self.start_runners(stage, library);
                if let Some(zinger) = at_bat.zinger {
                    at_bat.zinger_show =
                        zinger::Show::new(zinger, &ball, &parts, rules, stage, library);
                    // The outfielders go back to the wall to watch it over,
                    // unless the pitcher has been left to do it all.
                    if !self.mods.the_pitcher_fields_alone() {
                        let far =
                            |index: usize| reach(parts.home, at(stage, &parts.fielders[index]));
                        let mut field: Vec<usize> = (0..5.min(parts.fielders.len())).collect();
                        field.sort_by(|&a, &b| far(b).total_cmp(&far(a)));
                        field.truncate(OUTFIELDERS);
                        field.retain(|&index| index != fielding.fielder);
                        fielding.watchers = field;
                    }
                }
            }
        }
        self.show_numbers(stage);
        at_bat.fielding = Some(fielding);
    }

    /// Changes the view to the field for a pitch that nobody hit, with a
    /// runner on his way to steal a base: the catcher throws there.
    pub(crate) fn show_steal(
        &mut self,
        at_bat: &mut AtBat,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        let rules = &game.rules;
        let parts = at_bat.parts.clone();
        // He throws for the runner who is furthest on.
        let to = self
            .runners
            .iter()
            .filter(|runner| runner.stole_from.is_some())
            .filter_map(|runner| runner.running_to)
            .max();
        let (Some(to), Some(catcher)) = (to, parts.fielders.get(CATCHER)) else {
            return self.ready(&parts, stage, library);
        };
        at_bat.leave_batting_view(stage);
        let y = at(stage, &parts.field).1;
        if let Some(field) = stage.child_mut(&parts.field) {
            field.move_to(rules.field.x, y);
        }
        // The badge that says a strike was called would lie over second
        // base. The scoreboard over the field has the count.
        if let Some(badge) = &parts.strike_anim {
            show(stage, badge, false);
        }
        // The ball is in his glove, and stays out of sight until he lets
        // go of it.
        at_bat.ball = Some(Ball {
            height: rules.field.catch_height,
            ..unreachable_ball(at(stage, catcher))
        });
        show(stage, &parts.field_ball, false);
        let pop = &rules.steal.pop;
        let wait = pop.low + self.rng.below(pop.high.saturating_sub(pop.low) + 1);
        self.mods.a_steal_is_in_play(true);
        self.phase = Phase::Fielding;
        at_bat.fielding = Some(Fielding {
            walk: false,
            foul: false,
            home_run: false,
            gone: false,
            watchers: Vec::new(),
            live: true,
            caught: false,
            fielder: CATCHER,
            // The last of his wait is the drawing back of his arm.
            job: Job::PickUp {
                left: wait.saturating_sub(rules.field.throw_time),
            },
            land: parts.home,
            facing: Facing::Down,
            throw_to: to,
            frames: 0,
            since_settled: 0,
            fumbled: false,
            batter: None,
            dropped: false,
            steal: true,
        });
        self.show_numbers(stage);
    }

    /// The base to throw to: the nearest one that a runner is making for.
    /// With nobody running, the nearest base at all.
    fn pick_base(&self, from: Point, parts: &Parts) -> u8 {
        let wanted = |base: u8| {
            self.runners
                .iter()
                .any(|runner| runner.running_to == Some(base))
        };
        let nearest = |bases: &mut dyn Iterator<Item = u8>| {
            bases.min_by(|&a, &b| {
                let far = |base: u8| distance(from, parts.bases[usize::from(base) - 1]);
                far(a).total_cmp(&far(b))
            })
        };
        nearest(&mut (1..=4).filter(|&base| wanted(base)))
            .or_else(|| nearest(&mut (1..=4)))
            .unwrap_or(1)
    }

    /// One frame of the play in the field.
    pub(crate) fn field(
        &mut self,
        at_bat: &mut AtBat,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        let parts = at_bat.parts.clone();
        let Some(mut state) = at_bat.fielding.take() else {
            return;
        };
        state.frames += 1;
        self.move_the_loose_ball(at_bat, &mut state, &parts, game, stage, library);
        Match::watch_zinger(&mut state, &parts, game, stage, library);
        self.do_the_fielders_job(at_bat, &mut state, &parts, game, stage, library);

        let down = at_bat.ball.is_some_and(|ball| ball.bounced);
        self.move_runners(&state, down, &parts, stage, library);
        self.tell_steal(at_bat, game.rules.steal.told_time, stage, library);
        self.tell_sign(at_bat, game.rules.sign.told_time, stage, library);

        if self.the_play_is_over(&mut state, &parts, game, stage, library) {
            self.end_the_play(at_bat, &state, &parts, game, stage, library);
        }
        at_bat.fielding = Some(state);
    }

    /// Moves the ball on by a frame, for as long as nobody has hold of it:
    /// through the air, off the ground, off the wall or over it, with
    /// whatever comes of each.
    fn move_the_loose_ball(
        &mut self,
        at_bat: &mut AtBat,
        state: &mut Fielding,
        parts: &Parts,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        let rules = &game.rules.field;
        let miss = at_bat.contact.map_or(0.0, |contact| contact.miss());

        // The ball, for as long as nobody has hold of it.
        let loose = matches!(
            state.job,
            Job::Chase | Job::WaitCatch | Job::Rest | Job::Fumbling { .. }
        );
        // Where across the field it has come to the wall, and how high, if
        // it has on this frame.
        let mut at_wall = None;
        if let (Some(ball), true, false) = (&mut at_bat.ball, loose, state.walk || state.foul) {
            let before = *ball;
            let was_down = before.bounced;
            let pinball = self.mods.the_park_is_a_pinball_table();
            // In a pinball park the air takes nothing from a ball that has
            // been down, however it was hit.
            let miss = if pinball && was_down { 0.0 } else { miss };
            let mut happened = if state.home_run {
                Happened::Nothing
            } else {
                ball.step(parts.home, miss, rules)
            };
            if pinball && !state.home_run && !state.gone {
                // In a pinball park the wall and the foul lines send it
                // back, and whoever is nearest takes up the chase.
                let park = pinball::Park::of(parts);
                happened = pinball::rebound(ball, before, happened, &park, rules);
                if happened == Happened::HitWall {
                    at_bat.rebounds += 1;
                    if state.live && !self.mods.the_pitcher_fields_alone() {
                        let far =
                            |index: usize| distance(at(stage, &parts.fielders[index]), ball.at);
                        let nearest = (0..5.min(parts.fielders.len()))
                            .min_by(|&a, &b| far(a).total_cmp(&far(b)))
                            .unwrap_or(state.fielder);
                        if nearest != state.fielder {
                            let was = parts.fielders[state.fielder].clone();
                            stage.goto_label(&was, "waiting", false, library);
                            state.fielder = nearest;
                        }
                    }
                }
            }
            if matches!(happened, Happened::Cleared | Happened::HitWall) {
                at_wall = Some((parts.ground(rules).across(ball.at), ball.height));
            }
            // A ball that went over the wall before the view changed has
            // gone over it as far as this view knows now.
            if std::mem::take(&mut at_bat.over_wall) && happened == Happened::Nothing {
                happened = Happened::Cleared;
            }
            put(
                stage,
                &parts.field_ball,
                ball.at,
                seen_size(parts.home, ball.at),
            );
            if let Some(inner) = stage.child_mut(&parts.field_ball_inner) {
                inner.move_to(inner.matrix.tx, -ball.height);
            }
            if let (Some(shown), false) = (&mut at_bat.zinger_show, ball.bounced) {
                shown.follow(ball, parts, stage);
            }
            // The first time it comes down, a shot that was called for
            // there comes off.
            if happened == Happened::Landed && !was_down && self.mods.shots_are_called() {
                at_bat.came_down = Some(ball.at);
                if let Some(called) = &mut at_bat.called {
                    let runs = called.landed(ball.at, &game.rules, stage, library);
                    if runs > 0 {
                        self.score += runs;
                        self.show_numbers(stage);
                        Match::sound(stage, library, "crowd_bigClap");
                        Match::sound(stage, library, "baseball_organ_FX");
                        at_bat.notices.put(
                            Says::news(
                                "calledIt",
                                &format!("CALLED IT! +{runs}"),
                                CALLED_COLOUR,
                                game.rules.called_shot.told_time,
                            )
                            .at((parts.centre_x, CALLED_TOP))
                            .sized(1.2),
                            parts,
                            stage,
                            library,
                        );
                    }
                }
            }
            match happened {
                // A zinger is followed on to where it comes down before
                // anything is called.
                Happened::Cleared if state.live && at_bat.zinger_show.is_some() => {
                    state.gone = true;
                    state.job = Job::Rest;
                    let fielder = parts.fielders[state.fielder].clone();
                    stage.goto_label(&fielder, "waiting", false, library);
                }
                Happened::Cleared if state.live => {
                    // Where it would come down, beyond the wall.
                    state.land = ball.landing(parts.home, miss, rules);
                    self.home_run(state, parts, stage, library);
                }
                // Back off the wall: somebody has to go and get it.
                Happened::HitWall if state.live => state.job = Job::Chase,
                Happened::Landed if state.gone && !state.home_run => {
                    state.land = ball.at;
                    self.home_run(state, parts, stage, library);
                    if let Some(shown) = &mut at_bat.zinger_show {
                        self.zinger_down(shown, stage, library);
                    }
                }
                _ => {}
            }
        }
        if let Some((across, height)) = at_wall {
            self.strike_sign(at_bat, across, height, &game.rules.sign);
        }
    }

    /// Has the fielder whose ball it is do what he is doing for a frame:
    /// run for it, wait under it, pick it up, draw back, throw it, or go
    /// after it again when he has let it go.
    fn do_the_fielders_job(
        &mut self,
        at_bat: &mut AtBat,
        state: &mut Fielding,
        parts: &Parts,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        let rules = &game.rules.field;
        let fielder = parts.fielders[state.fielder].clone();
        let here = at(stage, &fielder);
        match state.job {
            Job::Chase => {
                let ball = at_bat.ball.unwrap_or_else(|| unreachable_ball(state.land));
                let target = if ball.bounced { ball.at } else { state.land };
                let gap = distance(here, target);
                let speed = rules.fielder_speed.at(game.settings.difficulty);
                let next = if gap <= speed {
                    target
                } else {
                    (
                        here.0 + (target.0 - here.0) / gap * speed,
                        here.1 + (target.1 - here.1) / gap * speed,
                    )
                };
                if gap > 0.01 {
                    state.facing = Facing::towards(here, target);
                }
                stage.goto_label(&fielder, state.facing.run_label(), false, library);
                // He is drawn smaller the further up the field he is.
                let out = reach(parts.home, next);
                put(stage, &fielder, next, (0.6 - out / 5000.0).max(0.2));
                // In a pinball park a ball that is hopping goes by over his
                // head.
                let too_high = ball.bounced
                    && self.mods.the_park_is_a_pinball_table()
                    && ball.height > game.rules.pinball.low;
                if distance(next, target) <= 2.0 && !too_high {
                    if ball.bounced && !state.fumbled && self.lets_go() {
                        // It squirts out of his hands as he bends for it.
                        state.fumbled = true;
                        stage.goto_label(&fielder, state.facing.pick_label(), false, library);
                        self.let_go(&mut at_bat.ball, parts, game);
                        Match::sound(stage, library, "crowd_smallCheer");
                        let told = &mut at_bat.notices;
                        Match::tell(told, "FUMBLED!", next, parts, game, stage, library);
                        state.job = Job::Fumbling {
                            left: game.rules.butterfingers.fumble_time,
                        };
                    } else if ball.bounced {
                        state.fumbled = false;
                        show(stage, &parts.field_ball, false);
                        stage.goto_label(&fielder, state.facing.pick_label(), false, library);
                        state.throw_to = self.pick_base(next, parts);
                        state.job = Job::PickUp {
                            left: rules.pick_time,
                        };
                    } else {
                        stage.goto_label(&fielder, "waitingToCatch", false, library);
                        state.job = Job::WaitCatch;
                    }
                } else if out >= rules.fielder_reach {
                    stage.goto_label(&fielder, "waiting", false, library);
                    state.job = Job::Rest;
                }
            }
            Job::WaitCatch => {
                if let Some(ball) = at_bat.ball {
                    if ball.bounced {
                        // It got down before he could take it.
                        state.job = Job::Chase;
                    } else if ball.lift < 0.0 && ball.height <= rules.catch_height && self.lets_go()
                    {
                        // It is in his glove and out again: nobody is out,
                        // and the ball is on the ground.
                        state.dropped = true;
                        self.let_go(&mut at_bat.ball, parts, game);
                        Match::sound(stage, library, "ballCatch_3");
                        Match::sound(stage, library, "crowd_smallCheer");
                        let told = &mut at_bat.notices;
                        Match::tell(told, "DROPPED!", here, parts, game, stage, library);
                        state.job = Job::Fumbling {
                            left: game.rules.butterfingers.fumble_time,
                        };
                    } else if ball.lift < 0.0 && ball.height <= rules.catch_height {
                        state.caught = true;
                        show(stage, &parts.field_ball, false);
                        Match::sound(stage, library, "ballCatch_3");
                        Match::sound(stage, library, "umpire_out_1");
                        Match::sound(stage, library, "crowd_unhappy");
                        // Caught: the batter is out wherever he has got to,
                        // which on a ball that hung a long time may be a
                        // base, or all the way round. A run he scored on it
                        // is no run.
                        let batter = state.batter.filter(|&batter| {
                            self.runners
                                .get(batter)
                                .is_some_and(|runner| runner.place != Place::Out)
                        });
                        if let Some(batter) = batter {
                            if self.runners[batter].place == Place::Home {
                                let worth = self.run_worth.min(self.runners[batter].runs);
                                self.runners[batter].runs -= worth;
                                self.score = self.score.saturating_sub(worth);
                            }
                            self.put_out(batter, stage, library);
                        }
                        state.throw_to = self.pick_base(here, parts);
                        state.job =
                            Match::wind_up(state, here, &fielder, parts, game, stage, library);
                    }
                }
            }
            Job::PickUp { left } => {
                state.job = if left == 0 {
                    Match::wind_up(state, here, &fielder, parts, game, stage, library)
                } else {
                    Job::PickUp { left: left - 1 }
                };
            }
            Job::WindUp { left } => {
                if left == 0 {
                    let to = parts.bases[usize::from(state.throw_to) - 1];
                    let gap = distance(here, to).max(0.001);
                    // A catcher's throw to a base being stolen has a speed
                    // of its own.
                    let speed = if state.steal && state.fielder == CATCHER {
                        game.rules.steal.throw_speed
                    } else {
                        rules.throw_speed
                    };
                    let step = ((to.0 - here.0) / gap * speed, (to.1 - here.1) / gap * speed);
                    if let Some(ball) = &mut at_bat.ball {
                        ball.at = here;
                        ball.height = rules.catch_height;
                    }
                    show(stage, &parts.field_ball, true);
                    state.job = Job::Throwing { step };
                } else {
                    state.job = Job::WindUp { left: left - 1 };
                }
            }
            Job::Throwing { step } => {
                let to = parts.bases[usize::from(state.throw_to) - 1];
                if let Some(ball) = &mut at_bat.ball {
                    ball.at = (ball.at.0 + step.0, ball.at.1 + step.1);
                    let size = (0.6 + (ball.at.1 - parts.home.1) / 1000.0).max(0.1);
                    put(stage, &parts.field_ball, ball.at, size);
                    if let Some(inner) = stage.child_mut(&parts.field_ball_inner) {
                        inner.move_to(inner.matrix.tx, -ball.height);
                    }
                    if distance(ball.at, to) < rules.throw_near {
                        let told = &mut at_bat.notices;
                        self.ball_at_base(state, told, parts, game, stage, library);
                    }
                }
            }
            Job::Fumbling { left } => {
                state.job = if left == 0 {
                    Job::Chase
                } else {
                    Job::Fumbling { left: left - 1 }
                };
            }
            Job::Gather { left } => {
                if left == 0 {
                    show(stage, &parts.field_ball, false);
                    stage.goto_label(&fielder, "baseWaiting", false, library);
                    self.hold_or_throw_on(state, parts, game, stage, library);
                } else {
                    state.job = Job::Gather { left: left - 1 };
                }
            }
            Job::Rest => {}
        }
    }

    /// Whether the play has come to its end: a foul or a home run once its
    /// picture has played out, a walk once everyone has walked, and any
    /// other once the ball is dead or has been in play too long.
    fn the_play_is_over(
        &self,
        state: &mut Fielding,
        parts: &Parts,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) -> bool {
        let rules = &game.rules.field;
        if state.foul || state.home_run {
            state.since_settled += 1;
            // The foul and home-run pictures play themselves out first.
            if state.home_run
                && state.since_settled == 60
                && let Some(board) = &parts.field_scoreboard
            {
                stage.goto_label(board, "homeRun", true, library);
            }
            state.since_settled >= if state.foul { 91 } else { 116 }
        } else if state.walk {
            !self.anyone_running()
        } else {
            !state.live || state.frames > rules.longest
        }
    }

    /// Calls the play dead: runners between bases are given the base they
    /// were making for, the mods are told how it went, it goes in the book,
    /// and the next pitch is put on offer.
    fn end_the_play(
        &mut self,
        at_bat: &mut AtBat,
        state: &Fielding,
        parts: &Parts,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        let rules = &game.rules.field;
        // Anyone still between bases when a play is called dead is given
        // the base he was making for.
        for runner in 0..self.runners.len() {
            if self.runners[runner].running_to.is_some() {
                self.arrive(runner, parts, stage, library);
            }
        }
        if !state.walk && !state.foul && !state.steal {
            // Where it went is remembered, for the shift to go by.
            let ground = parts.ground(rules);
            self.mods.a_fair_ball_came_down(ground.across(state.land));
        }
        // A hit puts some of bullet time's meter back, and a home run
        // all of it.
        let batter = state.batter.and_then(|batter| self.runners.get(batter));
        let hit = !state.walk && !state.foul && !state.steal;
        match batter.map(|batter| batter.place) {
            Some(Place::Home) if hit => self.mods.a_hit_came_off(true),
            Some(Place::Base(_)) if hit => self.mods.a_hit_came_off(false),
            _ => {}
        }
        self.book_play(at_bat, state);
        self.ready(parts, stage, library);
    }

    /// In a full match, writes a play that is over into the book: a foul,
    /// a walk, or what came of a ball that was put in play.
    fn book_play(&mut self, at_bat: &AtBat, state: &Fielding) {
        let Some(ground) = self.mode.full().map(|full| *full.ground()) else {
            return;
        };
        if state.steal {
            // The pitch is in the book already: nobody hit it.
            return;
        }
        if state.foul {
            return self.book_pitch(at_bat, Thrown::Foul);
        }
        if state.walk {
            return self.book_end(End::Walk, None);
        }
        self.book_pitch(at_bat, Thrown::InPlay);
        let (score, outs) = self.thrown_at;
        let place = state
            .batter
            .and_then(|batter| self.runners.get(batter))
            .map(|runner| runner.place);
        let safe = matches!(place, Some(Place::Base(_) | Place::Home));
        let end = match place {
            // He would have been out, had the catch been held.
            _ if safe && state.dropped => End::Error,
            Some(Place::Home) => End::HomeRun,
            Some(Place::Base(2)) => End::Double,
            Some(Place::Base(3)) => End::Triple,
            Some(Place::Base(_)) => End::Single,
            // A run that came in on a catch that was not the last out.
            _ if state.caught && self.score > score && outs < 2 => End::SacrificeFly,
            _ if state.caught => End::FlyOut,
            _ if self.outs >= outs + 2 => End::DoublePlay,
            _ => End::GroundOut,
        };
        // It was in the air if it was caught, went out of the park, or
        // first came down beyond the infield.
        let deep = reach(ground.home, state.land) >= ground.infield;
        let fly = state.caught || state.dropped || state.home_run || deep;
        let feet = at_bat.zinger.map(|zinger| zinger.feet);
        let hit = Hit::at(&ground, state.land, fly, feet);
        if end == End::Error
            && let Some(full) = self.mode.full_mut()
        {
            full.book.theirs.errors += 1;
        }
        self.book_end(end, Some(hit));
    }

    /// Turns the fielder to face the base and starts his throw.
    fn wind_up(
        state: &Fielding,
        here: Point,
        fielder: &Path,
        parts: &Parts,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) -> Job {
        let to = parts.bases[usize::from(state.throw_to) - 1];
        let label = Facing::towards(here, to).throw_label();
        stage.goto_label(fielder, label, false, library);
        Job::WindUp {
            left: game.rules.field.throw_time,
        }
    }

    /// A throw has reached its base.
    fn ball_at_base(
        &mut self,
        state: &mut Fielding,
        told: &mut Notices,
        parts: &Parts,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        let base = state.throw_to;
        Match::sound(stage, library, "ballCatch_1");
        // The fielder minding that base has the ball now, or should have.
        state.fielder = 4 + usize::from(base);
        if self.lets_go() {
            // It is through his hands. Nobody is out, and he has it to
            // gather from the ground beside him.
            let fielder = parts.fielders[state.fielder].clone();
            let here = at(stage, &fielder);
            stage.goto_label(&fielder, Facing::Down.pick_label(), false, library);
            Match::sound(stage, library, "crowd_smallCheer");
            Match::tell(told, "DROPPED!", here, parts, game, stage, library);
            state.job = Job::Gather {
                left: game.rules.butterfingers.gather_time,
            };
            return;
        }
        show(stage, &parts.field_ball, false);
        let late: Vec<usize> = (0..self.runners.len())
            .filter(|&runner| self.runners[runner].running_to == Some(base))
            .collect();
        for runner in late {
            self.put_out(runner, stage, library);
            Match::sound(stage, library, "umpire_out_2");
            Match::sound(stage, library, "crowd_unhappy");
            if base <= 3 {
                let umpire = parts.umpires[usize::from(base) - 1].clone();
                self.play_section(&umpire, "out", 99, stage, library);
            }
        }
        self.show_numbers(stage);
        self.hold_or_throw_on(state, parts, game, stage, library);
    }

    /// The fielder at a base has the ball in his hands. He throws it on if
    /// anybody is still between bases, and the play is over if nobody is.
    fn hold_or_throw_on(
        &mut self,
        state: &mut Fielding,
        parts: &Parts,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        // With the pitcher fielding alone nobody throws the ball on: the
        // play ends where his throw does, and anyone still running is given
        // his base.
        let thrown_on = self.anyone_running() && !self.mods.the_pitcher_fields_alone();
        if thrown_on {
            // Somebody is still between bases: on it goes.
            let fielder = parts.fielders[state.fielder].clone();
            let here = at(stage, &fielder);
            state.throw_to = self.pick_base(here, parts);
            state.job = Match::wind_up(state, here, &fielder, parts, game, stage, library);
        } else {
            state.live = false;
            state.job = Job::Rest;
        }
    }

    /// Whether a fielder having a go at the ball lets it go: with the
    /// butterfingers mod on, as often as the level it is set to says.
    fn lets_go(&mut self) -> bool {
        self.mods.a_fielder_lets_go(&mut self.rng)
    }

    /// Sends a ball that a fielder has let go rolling off, any way but
    /// out through the wall.
    fn let_go(&mut self, ball: &mut Option<Ball>, parts: &Parts, game: &Game) {
        let Some(ball) = ball else {
            return;
        };
        let rules = &game.rules.butterfingers;
        let turn = (self.rng.below(360) as f32).to_radians();
        ball.speed = (turn.cos() * rules.roll, turn.sin() * rules.roll);
        // Where it would be in a little while, at that rate.
        let ahead = (
            ball.at.0 + ball.speed.0 * 40.0,
            ball.at.1 + ball.speed.1 * 40.0,
        );
        if reach(parts.home, ahead) >= game.rules.field.wall - WARNING_TRACK {
            ball.speed = (-ball.speed.0, -ball.speed.1);
        }
        ball.height = ball.height.max(0.0);
        ball.lift = rules.pop;
        ball.fall = game.rules.field.gravity;
        // It has been in his hands: there is no catching it now, and
        // runners may go on as for any ball on the ground.
        ball.bounced = true;
    }

    /// Puts a word up over a fielder who has let the ball go, for a
    /// moment. `over` is where he is on the field.
    fn tell(
        told: &mut Notices,
        word: &str,
        over: Point,
        parts: &Parts,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        // The field is drawn at a size and a place of its own in the view.
        let Some(field) = stage.child(&parts.field).map(|field| field.matrix) else {
            return;
        };
        let middle =
            (field.tx + field.a * over.0).clamp(TOLD_MARGIN, parts.centre_x * 2.0 - TOLD_MARGIN);
        let top = field.ty + field.d * over.1 - TOLD_ABOVE;
        let frames = game.rules.butterfingers.told_time;
        told.put(
            Says::news("butterWord", word, TOLD_COLOUR, frames)
                .at((middle, top))
                .sized(TOLD_SIZE),
            parts,
            stage,
            library,
        );
    }

    /// The ball has cleared the wall: everybody scores.
    fn home_run(
        &mut self,
        state: &mut Fielding,
        parts: &Parts,
        stage: &mut Stage,
        library: &Library,
    ) {
        state.home_run = true;
        state.live = false;
        state.job = Job::Rest;
        self.mods.a_home_run_was_hit();
        let worth = self.run_worth;
        // The batter has reached every base there is.
        if self.batter().is_some() {
            self.mods.the_batter_reached_base();
        }
        for runner in &mut self.runners {
            if matches!(runner.place, Place::AtBat | Place::Base(_)) {
                runner.place = Place::Home;
                runner.running_to = None;
                runner.runs += worth;
                self.score += worth;
                if let Some(path) = &runner.path {
                    stage.goto_label(path, "empty", false, library);
                }
            }
        }
        self.clear_count();
        self.announce = true;
        let fielder = parts.fielders[state.fielder].clone();
        stage.goto_label(&fielder, "waiting", false, library);
        show(stage, &parts.field_ball, false);
        let transitions = parts.transitions.clone();
        self.play_section(&transitions, "homeRun", 117, stage, library);
        self.show_numbers(stage);
    }

    /// Moves the outfielders who are going back to watch a zinger a step
    /// nearer the wall, straight away from home. Each stops at the foot of
    /// it, and they all stop where they are once the home run is called.
    fn watch_zinger(
        state: &mut Fielding,
        parts: &Parts,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        let rules = &game.rules.field;
        let speed = rules.fielder_speed.at(game.settings.difficulty);
        let called = state.home_run;
        state.watchers.retain(|&index| {
            let fielder = &parts.fielders[index];
            let here = at(stage, fielder);
            let far = distance(parts.home, here).max(0.001);
            let next = (
                here.0 + (here.0 - parts.home.0) / far * speed,
                here.1 + (here.1 - parts.home.1) / far * speed,
            );
            let out = reach(parts.home, next);
            if called || out >= rules.wall - WARNING_TRACK {
                stage.goto_label(fielder, "waiting", false, library);
                return false;
            }
            let label = Facing::towards(here, next).run_label();
            stage.goto_label(fielder, label, false, library);
            put(stage, fielder, next, (0.6 - out / 5000.0).max(0.2));
            true
        });
    }

    /// Brings in the runners who have got to their bases, and keeps the
    /// buttons that send them on for when they can be used.
    fn move_runners(
        &mut self,
        state: &Fielding,
        ball_down: bool,
        parts: &Parts,
        stage: &mut Stage,
        library: &Library,
    ) {
        // Once the ball has been caught or has come down, a runner on a
        // base may try for the next. Turbo runners may at any time.
        let turbo = self.mods.runners_may_go_at_any_time();
        let may_go_on = state.live && (turbo || state.caught || ball_down);
        // A runner's run is a clip that plays a frame at a time. Turbo
        // runners are hurried on through it by more frames than that.
        let hurried = self.mods.hurry_the_runners();
        for runner in 0..self.runners.len() {
            let Some(path) = self.runners[runner].path.clone() else {
                continue;
            };
            let mut frame = frame_of(stage, &path);
            if let (Some(base), true) = (self.runners[runner].running_to, hurried > 0) {
                // As far as the base, or the end of his slide, and no
                // further: what comes after belongs to the next base.
                let index = usize::from(base) - 1;
                let end = if self.runners[runner].sliding {
                    SLIDE_ENDS[index]
                } else {
                    ARRIVES[index]
                };
                if frame < end {
                    frame = (frame + hurried).min(end);
                    stage.goto_clip(&path, frame, library);
                    if let Some(clip) = stage.clip_mut(&path) {
                        clip.playing = true;
                    }
                }
            }
            if let Some(base) = self.runners[runner].running_to {
                let index = usize::from(base) - 1;
                if self.runners[runner].sliding && frame >= SLIDE_ENDS[index] {
                    self.runners[runner].sliding = false;
                    stage.goto_clip(&path, SLIDE_JOINS[index], library);
                    if let Some(clip) = stage.clip_mut(&path) {
                        clip.playing = true;
                    }
                } else if frame >= ARRIVES[index] && frame < FIRST_SLIDE_FRAME {
                    self.arrive(runner, parts, stage, library);
                }
            } else if let Place::Base(base) = self.runners[runner].place {
                // The way on must be clear: nobody on the next base and
                // nobody making for it.
                let next = base + 1;
                let clear = next == 4
                    || (self.on_base(next).is_none()
                        && !self
                            .runners
                            .iter()
                            .any(|other| other.running_to == Some(next)));
                if let Some(button) = stage.find(&path, &["runBtn"]) {
                    show(stage, &button, may_go_on && clear);
                }
            }
        }
    }

    /// A press on one of a runner's own buttons: slide, or go on.
    pub(crate) fn runner_button(
        &mut self,
        symbol: SymbolId,
        path: &[u16],
        _game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        if self.phase != Phase::Fielding {
            return;
        }
        let Some(runner) = self.runners.iter().position(|runner| {
            runner
                .path
                .as_ref()
                .is_some_and(|own| path.starts_with(own))
        }) else {
            return;
        };
        let Some(own) = self.runners[runner].path.clone() else {
            return;
        };
        if SLIDE_BUTTONS.contains(&symbol) {
            if let (Some(base), false) = (
                self.runners[runner].running_to,
                self.runners[runner].sliding,
            ) {
                self.runners[runner].sliding = true;
                stage.goto_label(&own, SLIDE[usize::from(base) - 1], true, library);
            }
        } else if RUN_BUTTONS.contains(&symbol)
            && self.runners[runner].running_to.is_none()
            && let Place::Base(base) = self.runners[runner].place
        {
            // The button is only showing when he may go, so going is all
            // there is to do.
            self.send(runner, base + 1, stage, library);
        }
    }
}

/// A ball that is not there: nothing was hit. Only reached if a fielder is
/// somehow sent after one, and then he runs to where he was told.
fn unreachable_ball(at: Point) -> Ball {
    Ball {
        at,
        speed: (0.0, 0.0),
        height: 0.0,
        lift: 0.0,
        fall: 0.0,
        bounced: true,
        walled: true,
    }
}
