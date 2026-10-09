//! The fielder whose ball it is, until he has it in his hands: running
//! for it, waiting under it, catching it or letting it go.

use bb_engine::display::Path;
use bb_engine::library::Library;
use bb_engine::stage::Stage;

use super::loose_ball::unreachable_ball;
use super::play::{Catch, Fielding, Job};
use super::{Scene, WARNING_TRACK};
use crate::game::Game;
use crate::look::Rgb;
use crate::play::field::{Ball, Facing, distance, reach};
use crate::play::overlay::{Notices, Says};
use crate::play::pitch::Point;
use crate::play::{AtBat, Match, Parts, Place, put, show};

/// The word that goes up over a fielder who has let the ball go: how far
/// over him its top is, in pixels of the view, the size of its lettering,
/// its own being 1, how near the sides of the view its middle may come,
/// and its colour.
const TOLD_ABOVE: f32 = 30.0;
const TOLD_SIZE: f32 = 0.85;
const TOLD_MARGIN: f32 = 55.0;
const TOLD_COLOUR: Rgb = [0xff, 0x9a, 0x3c];

impl Match {
    /// Runs the fielder a step towards where the ball will come down, or
    /// towards the ball itself once it has. When he gets there he waits
    /// under it, picks it up, or lets it squirt away. A ball too far out
    /// for him he gives up on.
    pub(super) fn go_after_the_ball(
        &mut self,
        at_bat: &mut AtBat,
        state: &mut Fielding,
        fielder: &Path,
        here: Point,
        scene: Scene<'_>,
    ) {
        let Scene {
            parts,
            game,
            stage,
            library,
        } = scene;
        let rules = &game.rules.field;
        let ball = at_bat.ball.unwrap_or_else(|| unreachable_ball(state.land));
        let target = if ball.bounced { ball.at } else { state.land };
        let gap = distance(here, target);
        let speed = *rules.fielder_speed.at(game.settings.difficulty);
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
        stage.goto_label(fielder, state.facing.run_label(), false, library);
        // He is drawn smaller the further up the field he is.
        let out = reach(parts.home, next);
        put(stage, fielder, next, (0.6 - out / 5000.0).max(0.2));
        // In a pinball park a ball that is hopping goes by over his
        // head.
        let too_high = ball.bounced
            && self.mods.the_park_is_a_pinball_table()
            && ball.height > game.rules.pinball.low;
        if distance(next, target) <= 2.0 && !too_high {
            if ball.bounced && !state.fumbled && self.lets_go() {
                // It squirts out of his hands as he bends for it.
                state.fumbled = true;
                stage.goto_label(fielder, state.facing.pick_label(), false, library);
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
                stage.goto_label(fielder, state.facing.pick_label(), false, library);
                state.throw_to = self.pick_base(next, parts);
                state.job = Job::PickUp {
                    left: rules.pick_time,
                };
            } else {
                stage.goto_label(fielder, "waitingToCatch", false, library);
                state.job = Job::WaitCatch;
            }
        } else if out >= rules.fielder_reach {
            stage.goto_label(fielder, "waiting", false, library);
            state.job = Job::Rest;
        }
    }

    /// The fielder stands under a ball in the air. He catches it when it
    /// comes down to him, or has it in his glove and lets it go, or goes
    /// after it if it gets down first.
    pub(super) fn wait_under_the_ball(
        &mut self,
        at_bat: &mut AtBat,
        state: &mut Fielding,
        fielder: &Path,
        here: Point,
        scene: Scene<'_>,
    ) {
        let Scene {
            parts,
            game,
            stage,
            library,
        } = scene;
        let rules = &game.rules.field;
        let Some(ball) = at_bat.ball else {
            return;
        };
        if ball.bounced {
            // It got down before he could take it.
            state.job = Job::Chase;
        } else if ball.lift < 0.0 && ball.height <= rules.catch_height && self.lets_go() {
            // It is in his glove and out again: nobody is out,
            // and the ball is on the ground.
            state.catch = Some(Catch::Dropped);
            self.let_go(&mut at_bat.ball, parts, game);
            Match::sound(stage, library, "ballCatch_3");
            Match::sound(stage, library, "crowd_smallCheer");
            let told = &mut at_bat.notices;
            Match::tell(told, "DROPPED!", here, parts, game, stage, library);
            state.job = Job::Fumbling {
                left: game.rules.butterfingers.fumble_time,
            };
        } else if ball.lift < 0.0 && ball.height <= rules.catch_height {
            state.catch = Some(Catch::Made);
            show(stage, &parts.field_ball, false);
            Match::sound(stage, library, "ballCatch_3");
            Match::sound(stage, library, "umpire_out_1");
            Match::sound(stage, library, "crowd_unhappy");
            self.the_batter_is_caught_out(state.batter, stage, library);
            state.throw_to = self.pick_base(here, parts);
            state.job = Match::wind_up(state, here, fielder, parts, game, stage, library);
        }
    }

    /// Caught: the batter is out wherever he has got to, which on a ball
    /// that hung a long time may be a base, or all the way round. A run he
    /// scored on it is no run.
    fn the_batter_is_caught_out(
        &mut self,
        batter: Option<usize>,
        stage: &mut Stage,
        library: &Library,
    ) {
        let batter = batter.filter(|&batter| {
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
    }

    /// Whether a fielder having a go at the ball lets it go: with the
    /// butterfingers mod on, as often as the level it is set to says.
    pub(super) fn lets_go(&mut self) -> bool {
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
    pub(super) fn tell(
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
}
