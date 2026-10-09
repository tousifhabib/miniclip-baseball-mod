//! The arcade game as the match plays it: its view, the ball over its
//! field, and its result.

use bb_engine::stage::Stage;

use crate::game::Game;
use crate::play::field::{Happened, seen_size};
use crate::play::view::Lie;
use crate::play::{AtBat, Match, Parts, at, play_from, put, show, zinger};

impl Match {
    /// Gets the arcade game's own parts of a new batting view ready.
    pub(crate) fn set_up_arcade(&mut self, parts: &Parts, game: &Game, stage: &mut Stage) {
        let rules = &game.rules.arcade;
        let Some(arcade) = self.mode.arcade_mut() else {
            return;
        };
        arcade.lit = vec![false; rules.rings.len()];
        arcade.last_distance = f32::MAX;
        arcade.cleared = false;
        arcade.flying = false;
        arcade.owed = None;
        // With the zinger mod on there is no target to drop the ball on.
        let target_shown = !self.mods.every_hit_is_a_home_run();
        let area = &rules.target;
        arcade.target = (
            area.x + self.rng.below(area.width as u32) as f32,
            area.y + self.rng.below(area.height as u32) as f32,
        );
        let target = arcade.target;
        let left = arcade.left;

        if let Some(mark) = stage.find(&parts.field, &["landMarker"])
            && let Some(child) = stage.child_mut(&mark)
        {
            child.move_to(target.0, target.1);
            child.set_visible(target_shown);
        }
        // The batting view has a copy of the target lying on the outfield,
        // drawn smaller and flatter the further up the field it is.
        let lie = Lie::read(parts, stage);
        if let Some(copy) = stage.find(&parts.main, &["landMarker"])
            && let Some(child) = stage.child_mut(&copy)
        {
            child.set_matrix(lie.in_view(target, 1.0));
            child.set_visible(target_shown);
        }
        // One ball lit for every pitch still to come, this one included.
        if let Some(row) = stage.find(&parts.main, &["onScreen_ballsLeft"]) {
            stage.goto_clip(&row, left.max(1) as u16);
            if let Some(clip) = stage.clip_mut(&row) {
                clip.playing = false;
            }
        }
    }

    /// Changes the view to the overhead field, where the ball is followed
    /// to the target. The next pitch is on offer from this moment.
    pub(crate) fn show_arcade_field(&mut self, at_bat: &mut AtBat, game: &Game, stage: &mut Stage) {
        let parts = at_bat.parts.clone();
        at_bat.leave_batting_view(stage);
        let y = at(stage, &parts.field).1;
        if let Some(field) = stage.child_mut(&parts.field) {
            field.move_to(game.rules.field.x, y);
        }
        if let Some(arcade) = self.mode.arcade_mut() {
            arcade.flying = true;
        }
        if let (Some(zinger), Some(ball)) = (at_bat.zinger, at_bat.ball) {
            at_bat.zinger_show = zinger::Show::new(zinger, &ball, &parts, &game.rules, stage);
        }
        self.ready(&parts, stage);
    }

    /// A zinger still in the air when the view is left for the next pitch
    /// scores what it was going to, unseen.
    pub(crate) fn zinger_unseen(&mut self) {
        let Some(arcade) = self.mode.arcade_mut() else {
            return;
        };
        if let Some(feet) = arcade.owed.take() {
            arcade.points += feet;
            self.mods.a_zinger_went(feet);
        }
    }

    /// One frame of the ball over the arcade game's field.
    pub(crate) fn arcade_ball(&mut self, at_bat: &mut AtBat, game: &Game, stage: &mut Stage) {
        let rules = &game.rules;
        let (Some(arcade), Some(ball), Some(contact)) =
            (self.mode.arcade_mut(), &mut at_bat.ball, at_bat.contact)
        else {
            return;
        };
        if !arcade.flying {
            return;
        }
        let parts = &at_bat.parts;
        let happened = ball.step(parts.home, contact.miss(), &rules.field);
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
        // The ring a ball lit, if it lit one, and the points it scored.
        let mut scored = None;
        let mut zinger_down = false;
        match happened {
            // A zinger scores by how far it went, wherever it came down.
            Happened::Landed if arcade.owed.is_some() => {
                let feet = arcade.owed.take().unwrap_or(0);
                arcade.points += feet;
                scored = Some((None, feet));
                zinger_down = true;
                show(stage, &parts.field_ball, false);
            }
            Happened::Cleared => arcade.cleared = true,
            Happened::Landed if arcade.cleared => show(stage, &parts.field_ball, false),
            Happened::Landed => {
                scored = arcade
                    .touch(arcade.last_distance, &rules.arcade)
                    .map(|(ring, points)| (Some(ring), points));
            }
            _ => {}
        }
        arcade.last_distance = arcade.off_target(ball.at, &rules.arcade);
        if zinger_down && let Some(shown) = &mut at_bat.zinger_show {
            self.zinger_down(shown, stage);
        }

        if let Some((ring, points)) = scored {
            // A ball that scores puts some of bullet time's meter back.
            self.mods.a_hit_came_off(false);
            if let Some(ring) = ring {
                // The art numbers its rings from the outside in.
                let name = format!("ring{}", rules.arcade.rings.len() - ring);
                if let Some(path) = stage.find(&parts.field, &["landMarker", &name]) {
                    stage.goto_clip(&path, 2);
                }
            }
            stage.set_text("thisScore", points.to_string());
            if let Some(pulse) = stage.find(&parts.main, &["onScreenScore", "scoreAnim"]) {
                play_from(stage, &pulse, 2);
            }
            self.show_numbers(stage);
        }
    }

    /// The arcade game's points and what they come to with the skill level
    /// counted in, for the finish screen.
    pub fn show_arcade_result(&self, game: &Game, stage: &mut Stage) {
        let points = self.mode.arcade().map_or(0, |arcade| arcade.points);
        let times = *game.rules.arcade.multiplier.at(game.settings.difficulty);
        stage.set_text("points_total", points.to_string());
        stage.set_text("points_final", (points * times).to_string());
    }
}
