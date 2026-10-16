//! The figures a side's turns add up to, and the averages and rates
//! worked out from them.

use serde::{Deserialize, Serialize};

use super::turn::{End, Steal, Thrown, Turn};

/// What a batter's or a side's turns add up to. They can be written out
/// and read back, a count that was not written reading as nought.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Figures {
    pub turns: u32,
    pub at_bats: u32,
    pub runs: u32,
    pub hits: u32,
    pub singles: u32,
    pub doubles: u32,
    pub triples: u32,
    pub home_runs: u32,
    pub total_bases: u32,
    pub runs_in: u32,
    pub walks: u32,
    pub strikeouts: u32,
    pub sacrifices: u32,
    pub double_plays: u32,
    /// Runners left on base. Only a whole side has these.
    pub left: u32,
    /// At-bats and hits with a runner on second or third.
    pub chances: u32,
    pub chances_taken: u32,
    /// Runs that came in with two out.
    pub two_out_runs: u32,
    pub pitches: u32,
    pub strikes: u32,
    pub called: u32,
    pub swinging: u32,
    pub fouls: u32,
    pub swings: u32,
    /// Pitches outside the zone, and the swings at them.
    pub outside: u32,
    pub chases: u32,
    /// Balls put in play: how many in the air, how many on the ground, how
    /// many to each third of the field, how far they went in all, and the
    /// furthest.
    pub in_play: u32,
    pub flies: u32,
    pub grounders: u32,
    pub thirds: [u32; 3],
    pub feet: u32,
    pub longest: u32,
    /// Bases stolen, and runners caught stealing. Only a side or one of
    /// its batters has these: they are not part of anyone's turn.
    pub stolen: u32,
    pub caught: u32,
}

/// One number over another, where there is anything to divide by.
fn share(part: u32, whole: u32) -> Option<f32> {
    (whole > 0).then(|| part as f32 / whole as f32)
}

impl Figures {
    pub fn of<'a>(turns: impl Iterator<Item = &'a Turn>) -> Figures {
        let mut figures = Figures::default();
        for turn in turns {
            figures.turns += 1;
            figures.runs_in += turn.runs_in;
            if turn.outs == 2 {
                figures.two_out_runs += turn.runs_in;
            }
            if turn.end.at_bat() {
                figures.at_bats += 1;
                if turn.in_scoring_position() {
                    figures.chances += 1;
                    figures.chances_taken += u32::from(turn.end.hit());
                }
            }
            figures.total_bases += turn.end.bases();
            match turn.end {
                End::Single => figures.singles += 1,
                End::Double => figures.doubles += 1,
                End::Triple => figures.triples += 1,
                End::HomeRun => figures.home_runs += 1,
                End::Walk => figures.walks += 1,
                End::Strikeout => figures.strikeouts += 1,
                End::SacrificeFly => figures.sacrifices += 1,
                End::DoublePlay => figures.double_plays += 1,
                End::FlyOut | End::GroundOut | End::Error => {}
            }
            for pitch in &turn.pitches {
                figures.pitches += 1;
                figures.strikes += u32::from(pitch.strike());
                figures.swings += u32::from(pitch.swung());
                match pitch.thrown {
                    Thrown::Called => figures.called += 1,
                    Thrown::Swinging => figures.swinging += 1,
                    Thrown::Foul => figures.fouls += 1,
                    Thrown::Ball | Thrown::InPlay => {}
                }
                if !pitch.in_zone {
                    figures.outside += 1;
                    figures.chases += u32::from(pitch.swung());
                }
            }
            if let Some(ball) = turn.ball {
                figures.in_play += 1;
                if ball.fly {
                    figures.flies += 1;
                } else {
                    figures.grounders += 1;
                }
                figures.thirds[ball.third()] += 1;
                figures.feet += ball.feet;
                figures.longest = figures.longest.max(ball.feet);
            }
        }
        figures.hits = figures.singles + figures.doubles + figures.triples + figures.home_runs;
        figures
    }

    /// Counts in these tries at stealing a base.
    pub(super) fn steal<'a>(&mut self, steals: impl Iterator<Item = &'a Steal>) {
        for steal in steals {
            if steal.safe {
                self.stolen += 1;
            } else {
                self.caught += 1;
            }
        }
    }

    /// Bases stolen, of the tries there were, as the board writes them.
    pub fn stolen_of(&self) -> String {
        format!("{} OF {}", self.stolen, self.stolen + self.caught)
    }

    /// Hits for each at-bat.
    pub fn average(&self) -> Option<f32> {
        share(self.hits, self.at_bats)
    }

    /// How often a turn ends with the batter on base by a hit or a walk.
    pub fn on_base(&self) -> Option<f32> {
        share(
            self.hits + self.walks,
            self.at_bats + self.walks + self.sacrifices,
        )
    }

    /// Bases for each at-bat.
    pub fn slugging(&self) -> Option<f32> {
        share(self.total_bases, self.at_bats)
    }

    /// The last two added together.
    pub fn on_base_plus_slugging(&self) -> Option<f32> {
        Some(self.on_base()? + self.slugging()?)
    }

    /// Hits for each ball put in play that stayed in the park.
    pub fn in_play_average(&self) -> Option<f32> {
        let balls = self.at_bats + self.sacrifices;
        share(
            self.hits - self.home_runs,
            balls.saturating_sub(self.strikeouts + self.home_runs),
        )
    }

    /// Hits for each at-bat with a runner on second or third.
    pub fn chance_average(&self) -> Option<f32> {
        share(self.chances_taken, self.chances)
    }

    pub fn strikeout_rate(&self) -> Option<f32> {
        share(self.strikeouts, self.turns)
    }

    pub fn walk_rate(&self) -> Option<f32> {
        share(self.walks, self.turns)
    }

    /// The share of the pitches that were strikes of any kind: called,
    /// swung at and missed, fouled off or put in play.
    pub fn strike_rate(&self) -> Option<f32> {
        share(self.strikes, self.pitches)
    }

    pub fn swing_rate(&self) -> Option<f32> {
        share(self.swings, self.pitches)
    }

    /// The share of the swings that met the ball.
    pub fn contact_rate(&self) -> Option<f32> {
        share(self.swings - self.swinging, self.swings)
    }

    /// The share of the swings that missed.
    pub fn miss_rate(&self) -> Option<f32> {
        share(self.swinging, self.swings)
    }

    /// The share of the pitches outside the zone that were swung at.
    pub fn chase_rate(&self) -> Option<f32> {
        share(self.chases, self.outside)
    }

    pub fn pitches_a_turn(&self) -> Option<f32> {
        share(self.pitches, self.turns)
    }

    /// How far a ball put in play went, on the whole, in feet.
    pub fn usual_feet(&self) -> Option<f32> {
        share(self.feet, self.in_play)
    }
}

/// An average as the game writes one: three places and no nought before the
/// point, and dashes where there was nothing to divide by.
pub fn average(value: Option<f32>) -> String {
    let Some(value) = value else {
        return "---".to_owned();
    };
    let written = format!("{value:.3}");
    written
        .strip_prefix('0')
        .map(str::to_owned)
        .unwrap_or(written)
}

/// A share as so many in the hundred.
pub fn percent(value: Option<f32>) -> String {
    match value {
        Some(value) => format!("{:.0}%", value * 100.0),
        None => "-".to_owned(),
    }
}

/// A number to one place.
pub fn tenths(value: Option<f32>) -> String {
    match value {
        Some(value) => format!("{value:.1}"),
        None => "-".to_owned(),
    }
}
