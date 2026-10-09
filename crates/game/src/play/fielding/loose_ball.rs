//! The ball while nobody has hold of it: through the air, along the
//! ground, off the wall or over it.

use bb_engine::library::Library;
use bb_engine::stage::Stage;

use super::WARNING_TRACK;
use super::play::{Fair, Fielding, Job, Play};
use crate::game::Game;
use crate::look::Rgb;
use crate::play::field::{Ball, Facing, Happened, distance, reach, seen_size};
use crate::play::overlay::Says;
use crate::play::pinball;
use crate::play::pitch::Point;
use crate::play::{AtBat, Match, Parts, at, put};

/// Where the view of the field says that a called shot came off: how far
/// down, and in what colour.
const CALLED_TOP: f32 = 232.0;
const CALLED_COLOUR: Rgb = [0xff, 0xe2, 0x4a];

impl Match {
    /// Moves the ball on by a frame, for as long as nobody has hold of it:
    /// through the air, off the ground, off the wall or over it, with
    /// whatever comes of each.
    pub(super) fn move_the_loose_ball(
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
        // A walk has no ball in the field, and a foul's is left where it is.
        let nothing_to_move = matches!(state.play, Play::Walk | Play::Foul { .. });
        let (Some(mut ball), true, false) = (at_bat.ball, loose, nothing_to_move) else {
            return;
        };
        let before = ball;
        let was_down = before.bounced;
        let pinball = self.mods.the_park_is_a_pinball_table();
        // In a pinball park the air takes nothing from a ball that has
        // been down, however it was hit.
        let miss = if pinball && was_down { 0.0 } else { miss };
        let mut happened = if state.is_home_run() {
            Happened::Nothing
        } else {
            ball.step(parts.home, miss, rules)
        };
        let over_the_wall = matches!(state.play, Play::Fair(Fair::Gone | Fair::HomeRun { .. }));
        if pinball && !over_the_wall {
            // In a pinball park the wall and the foul lines send it
            // back, and whoever is nearest takes up the chase.
            let park = pinball::Park::of(parts);
            happened = pinball::rebound(&mut ball, before, happened, &park, rules);
            if happened == Happened::HitWall {
                at_bat.rebounds += 1;
                if state.play.is_live() && !self.mods.the_pitcher_fields_alone() {
                    Match::the_nearest_takes_up_the_chase(state, ball.at, parts, stage, library);
                }
            }
        }
        // Where across the field it has come to the wall, and how high, if
        // it has on this frame.
        let at_wall = matches!(happened, Happened::Cleared | Happened::HitWall)
            .then(|| (parts.ground(rules).across(ball.at), ball.height));
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
            shown.follow(&ball, parts, stage);
        }
        // The first time it comes down, a shot that was called for
        // there comes off.
        if happened == Happened::Landed && !was_down && self.mods.shots_are_called() {
            self.a_called_shot_comes_down(at_bat, ball.at, parts, game, stage, library);
        }
        match happened {
            // A zinger is followed on to where it comes down before
            // anything is called.
            Happened::Cleared if state.play.is_live() && at_bat.zinger_show.is_some() => {
                state.play = Play::Fair(Fair::Gone);
                state.job = Job::Rest;
                let fielder = parts.fielders[state.fielder].clone();
                stage.goto_label(&fielder, "waiting", false, library);
            }
            Happened::Cleared if state.play.is_live() => {
                // Where it would come down, beyond the wall.
                state.land = ball.landing(parts.home, miss, rules);
                self.home_run(state, parts, stage, library);
            }
            // Back off the wall: somebody has to go and get it.
            Happened::HitWall if state.play.is_live() => state.job = Job::Chase,
            Happened::Landed if state.play == Play::Fair(Fair::Gone) => {
                state.land = ball.at;
                self.home_run(state, parts, stage, library);
                if let Some(shown) = &mut at_bat.zinger_show {
                    self.zinger_down(shown, stage, library);
                }
            }
            _ => {}
        }
        at_bat.ball = Some(ball);
        if let Some((across, height)) = at_wall {
            self.strike_sign(at_bat, across, height, &game.rules.sign);
        }
    }

    /// In a pinball park, a ball that comes back off the wall is chased by
    /// whichever of the fielders who chase is nearest it now.
    fn the_nearest_takes_up_the_chase(
        state: &mut Fielding,
        ball_at: Point,
        parts: &Parts,
        stage: &mut Stage,
        library: &Library,
    ) {
        let far = |index: usize| distance(at(stage, &parts.fielders[index]), ball_at);
        let nearest = (0..5.min(parts.fielders.len()))
            .min_by(|&a, &b| far(a).total_cmp(&far(b)))
            .unwrap_or(state.fielder);
        if nearest != state.fielder {
            let was = parts.fielders[state.fielder].clone();
            stage.goto_label(&was, "waiting", false, library);
            state.fielder = nearest;
        }
    }

    /// A hit has come down for the first time, at `place`. With shots
    /// being called, where it did is kept, and a shot that was called for
    /// there comes off: its runs are the batter's side's, and it is told.
    fn a_called_shot_comes_down(
        &mut self,
        at_bat: &mut AtBat,
        place: Point,
        parts: &Parts,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        at_bat.came_down = Some(place);
        let Some(called) = &mut at_bat.called else {
            return;
        };
        let runs = called.landed(place, &game.rules, stage, library);
        if runs == 0 {
            return;
        }
        self.score += runs;
        self.show_numbers(stage);
        Match::sound(stage, library, "crowd_bigClap");
        Match::sound(stage, library, "baseball_organ_FX");
        let words = format!("CALLED IT! +{runs}");
        let frames = game.rules.called_shot.told_time;
        let says = Says::news("calledIt", &words, CALLED_COLOUR, frames)
            .at((parts.centre_x, CALLED_TOP))
            .sized(1.2);
        at_bat.notices.put(says, parts, stage, library);
    }

    /// Moves the outfielders who are going back to watch a zinger a step
    /// nearer the wall, straight away from home. Each stops at the foot of
    /// it, and they all stop where they are once the home run is called.
    pub(super) fn watch_zinger(
        state: &mut Fielding,
        parts: &Parts,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        let rules = &game.rules.field;
        let speed = *rules.fielder_speed.at(game.settings.difficulty);
        let called = state.is_home_run();
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
}

/// A ball that is not there: nothing was hit. Only reached if a fielder is
/// somehow sent after one, and then he runs to where he was told.
pub(super) fn unreachable_ball(at: Point) -> Ball {
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
