//! What the match writes on the scoreboard and on the result screens,
//! and how the side at bat is dressed.

use bb_engine::stage::Stage;

use super::Match;
use crate::look::{Look, Rgb};
use crate::play::book::ORDER;

impl Match {
    /// How the side should look just now, given the team's own colour.
    pub fn look(&self, clothes: Option<Rgb>) -> Look {
        let batter = self.runners.batter().map(|index| &self.runners[index]);
        Look {
            clothes,
            skin: batter.and_then(|runner| runner.skin),
            logo: batter.and_then(|runner| runner.logo.clone()),
            second_skin: self
                .runners
                .on_base(2)
                .and_then(|index| self.runners[index].skin),
        }
    }

    /// Writes the numbers the scoreboards show.
    pub(crate) fn show_numbers(&self, stage: &mut Stage) {
        let batter = self
            .runners
            .batter()
            .map_or(self.came_up, |index| self.runners[index].order + 1);
        // In a full match the board shows the other side's score where it
        // would show the score to beat.
        let shown_target = match self.mode.full() {
            Some(_) => self.target - 1,
            None => self.target,
        };
        for (name, value) in [
            ("score", self.score),
            ("out", self.outs),
            ("strikes", self.count.strikes),
            ("noBalls", self.count.balls),
            ("scoreTarget", shown_target),
            ("oppositionScore", self.target - 1),
            ("maximumOuts", self.max_outs),
            ("runsToGet", self.target.saturating_sub(self.score)),
            ("ballsPitched", self.pitched),
            (
                "points_total",
                self.mode.arcade().map_or(0, |arcade| arcade.points),
            ),
            ("batsmanOnStrike", batter as u32),
        ] {
            stage.set_text(name, value.to_string());
        }
        for (order, runs) in self
            .runners
            .runs_by_order(&self.tally)
            .into_iter()
            .enumerate()
        {
            stage.set_text(&format!("batsman{}_score", order + 1), runs.to_string());
        }
    }

    /// What the result screens say about the match just played.
    pub fn show_result(&self, stage: &mut Stage) {
        self.show_numbers(stage);
        // A full match's outs are those of all its innings. A play that
        // put out more than were left to get is not counted for more.
        let outs = self.outs_before + self.outs.min(self.max_outs);
        stage.set_text("out", outs.to_string());
        for order in self.runners.runs_by_order(&self.tally).len()..ORDER {
            stage.set_text(&format!("batsman{}_score", order + 1), "0");
        }
    }
}
