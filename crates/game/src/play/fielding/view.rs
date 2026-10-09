//! The change of view, from behind the batter to over the field.

use bb_engine::display::Content;
use bb_engine::library::Library;
use bb_engine::stage::Stage;

use super::CATCHER;
use super::play::{Fair, Fielding, Job, Play};
use crate::game::Game;
use crate::play::field::{Ball, distance, reach};
use crate::play::fielding::loose_ball::unreachable_ball;
use crate::play::pitch::Point;
use crate::play::{AtBat, Match, Parts, Phase, at, show, steal, zinger};

/// The frames of a fielder on which he picks the ball up, throws it or
/// catches it. Each shows a clip inside him that is meant to play once.
const ONCE_ONLY: std::ops::RangeInclusive<u16> = 46..=145;

/// How many of the five in the field are outfielders: the ones who stand
/// furthest from home.
const OUTFIELDERS: usize = 3;

/// The fielder who stands on the mound, counting from 0 as
/// [`Fielding::fielder`] does: the art's `fielder3`.
const PITCHER: usize = 2;

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
        let play = if walk {
            Play::Walk
        } else {
            Play::Fair(Fair::Live)
        };
        let mut fielding = Fielding::begun(play, parts.home, self.runners.batter());
        self.phase = Phase::Fielding;

        if walk {
            show(stage, &parts.field_ball, false);
            // A runner the walk pushes on has stolen nothing.
            self.runners.a_walk_takes_the_steals_it_pushes();
            let stealing = self.runners.anyone_stealing();
            self.mods.a_steal_is_in_play(stealing);
            self.start_runners(stage, library);
        } else if let (Some(ball), Some(contact)) = (at_bat.ball, at_bat.contact) {
            let mark_x = parts.field_mark.0 + contact.aside / rules.field.aim_share;
            if mark_x < parts.foul.0 || mark_x > parts.foul.1 {
                // A foul is a strike, but never the last one.
                fielding.play = Play::Foul { called: 0 };
                steal::send_back(&mut self.runners, stage, library);
                let allowed = self.strikes_allowed(game);
                if self.count.foul(allowed) {
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
                let land = fielding.land;
                let far = |index: usize| distance(at(stage, &parts.fielders[index]), land);
                fielding.fielder = if self.mods.the_pitcher_fields_alone() {
                    PITCHER
                } else {
                    (0..5)
                        .min_by(|&a, &b| far(a).total_cmp(&far(b)))
                        .unwrap_or(0)
                };
                fielding.job = Job::Chase;
                self.runners.steals_are_runs();
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
        let to = self.runners.bases_being_stolen().max();
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
            fielder: CATCHER,
            // The last of his wait is the drawing back of his arm.
            job: Job::PickUp {
                left: wait.saturating_sub(rules.field.throw_time),
            },
            throw_to: to,
            ..Fielding::begun(Play::Steal { held: false }, parts.home, None)
        });
        self.show_numbers(stage);
    }

    /// The base to throw to: the nearest one that a runner is making for.
    /// With nobody running, the nearest base at all.
    pub(super) fn pick_base(&self, from: Point, parts: &Parts) -> u8 {
        let wanted = |base: u8| self.runners.anyone_making_for(base);
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
}
