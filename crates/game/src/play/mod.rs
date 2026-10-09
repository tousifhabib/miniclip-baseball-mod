//! A match in progress: the last innings, batting to overtake the other
//! side, or every innings of a full match.
//!
//! The art builds the batting view afresh for every pitch, so everything
//! that lasts from one pitch to the next is kept here: the score, the count,
//! the outs, and where every runner stands.

mod arcade;
mod at_bat;
mod batting;
pub mod book;
mod describe;
pub mod field;
mod fielding;
pub mod full;
mod mode;
pub(crate) mod mods;
pub mod paper;
mod phase;
pub mod pitch;
pub(crate) mod runners;
mod scoreboard;
mod set_up;
mod snapshot;
mod standing;
pub(crate) mod view;
mod zingers;

use bb_engine::display::{ButtonEvent, Event, Path};
use bb_engine::library::Library;
use bb_engine::stage::Stage;
use bb_format::SymbolId;

use crate::game::Game;
use crate::look::Rgb;
use crate::rng::Rng;
use at_bat::AtBat;
use mode::Mode;
pub(crate) use mods::bullet_time as bullet;
use mods::{
    ModsInPlay, called_shot as called, hit_the_sign as sign, night_game, pinball_park as pinball,
    southpaw, stolen_bases as steal, timing_indicator as timing, zinger_hit as zinger,
};
pub use phase::Outcome;
use phase::Phase;
pub(crate) use runners::{Count, Place, Runner, Runners};
pub(crate) use view::{Cue, Parts, at, frame_of, overlay, play_from, put, show};

pub struct Match {
    pub(crate) score: u32,
    pub(crate) target: u32,
    pub(crate) outs: u32,
    pub(crate) max_outs: u32,
    pub(crate) count: Count,
    pub(crate) pitched: u32,
    pub(crate) runners: Runners,
    pub(crate) rng: Rng,
    pub(crate) phase: Phase,
    /// The last pitch ended a batter's turn, which the scoreboard marks at
    /// the start of the next.
    pub(crate) announce: bool,
    pub(crate) at: Option<AtBat>,
    pub(crate) cues: Vec<Cue>,
    /// Clips to send back to their first frame, where they show nothing,
    /// once this many more frames have gone by.
    put_away: Vec<(Path, u32)>,
    /// Which kind of game this is, with what only that kind keeps.
    pub(crate) mode: Mode,
    /// The mods that are on for this game, and what each of them keeps.
    pub(crate) mods: ModsInPlay,
    /// How many batters have come to the plate, and the skins of those in
    /// a full match's batting order, as far as they have been seen.
    came_up: usize,
    line_up: Vec<Option<Rgb>>,
    /// In a full match: the runs made by each place in the order, and the
    /// outs there were, in the innings gone by.
    pub(crate) tally: Vec<u32>,
    pub(crate) outs_before: u32,
    /// The score and the outs when the pitch in hand was thrown, by which a
    /// full match's book knows what came of it.
    pub(crate) thrown_at: (u32, u32),
    /// How many a run counts for on the pitch being played: one, unless a
    /// mod says more.
    pub(crate) run_worth: u32,
    runner_symbol: Option<SymbolId>,
}

/// How far down the batting view the mystery pitch mod names the pitch,
/// which is between the scoreboard and the pitcher. The tired arm mod says
/// there that a new pitcher has come in.
const MYSTERY_TOP: f32 = 141.0;
/// The button on the next-ball panel.
const NEXT_BALL_BUTTON: SymbolId = 1618;

impl Match {
    pub fn new(game: &Game, seed: u64, library: &Library) -> Match {
        let mods = ModsInPlay::for_game(game, seed, false);
        // With every hit a home run there are more runs to get.
        let runs_down = if mods.every_hit_is_a_home_run() {
            &game.rules.zinger.runs_down
        } else {
            &game.rules.game.runs_down
        };
        let behind = *runs_down.at(game.settings.difficulty);
        Match {
            score: 0,
            // Drawing level is not enough: the target is one run more.
            target: behind + 1,
            outs: 0,
            max_outs: game.rules.game.outs,
            count: Count::default(),
            pitched: 0,
            runners: Runners::default(),
            rng: Rng::new(seed),
            phase: Phase::Arriving,
            announce: false,
            at: None,
            cues: Vec::new(),
            put_away: Vec::new(),
            mode: Mode::LastInnings,
            mods,
            came_up: 0,
            line_up: Vec::new(),
            tally: Vec::new(),
            outs_before: 0,
            thrown_at: (0, 0),
            run_worth: 1,
            runner_symbol: library.manifest.exports.get("runner").copied(),
        }
    }

    /// The arcade game instead of a match.
    pub fn new_arcade(game: &Game, seed: u64, library: &Library) -> Match {
        let mut arcade = Match::new(game, seed, library);
        arcade.mode = Mode::Arcade(arcade::Arcade::new(game.rules.arcade.pitches));
        arcade.mods = ModsInPlay::for_game(game, seed, true);
        arcade
    }

    /// Called once a frame while the match screen is showing. Returns how
    /// the match ended, once it has.
    pub fn tick(&mut self, game: &Game, stage: &mut Stage, library: &Library) -> Option<Outcome> {
        // Where the player has clicked since the last frame, off the
        // buttons. It is taken from the click itself and not from how the
        // pointer's button is now, which may be up again already.
        let pressed = stage.pointer.went_down;
        self.run_cues(stage, library);
        // The stadium is lit as by day, unless it is night, and is cooler
        // while bullet time holds the ball back.
        let slowed = self.mods.the_ball_was_held_back();
        if let Some(lighting) = self.mods.lighting(slowed) {
            night_game::light(lighting, stage);
        }

        if self.phase == Phase::Arriving {
            return self.set_up(game, stage, library);
        }
        let mut at_bat = self.at.take()?;
        // The view has gone: the screen was left.
        stage.clip(&at_bat.parts.main)?;
        self.keep_the_view(&mut at_bat, slowed, game, stage, library);

        // In the arcade game the ball goes on over the field while the next
        // pitch is already on offer.
        if matches!(self.phase, Phase::Ready | Phase::Leaving { .. }) {
            self.arcade_ball(&mut at_bat, game, stage, library);
        }
        match self.phase {
            Phase::Settling { left } => {
                self.wait_for_the_wind_up(&mut at_bat, left, pressed, game, stage, library);
            }
            Phase::WindUp => self.wind_up_and_throw(&mut at_bat, pressed, game, stage, library),
            Phase::Flight { step } => {
                let pressed = self.mods.late_press().or(pressed);
                let (steps, swung) = (at_bat.pitch.samples.len(), at_bat.swing.is_some());
                let key_down = stage.key_down(bullet::KEY);
                if self.mods.holds_the_ball_back(step, steps, swung, key_down) {
                    // The ball stays where it is for this frame. A click
                    // made on it is for the step the ball is on.
                    self.mods.keep_press(pressed);
                } else {
                    self.flight(&mut at_bat, step, pressed, game, stage, library);
                }
            }
            Phase::Called { left } => {
                if left == 0 {
                    self.ready(&at_bat.parts, stage, library);
                } else {
                    self.phase = Phase::Called { left: left - 1 };
                }
            }
            Phase::Watching { left } => {
                self.watch(&mut at_bat, game, stage, library);
                if left == 0 && self.mode.is_arcade() {
                    self.show_arcade_field(&mut at_bat, game, stage, library);
                } else if left == 0 {
                    self.show_field(&mut at_bat, false, game, stage, library);
                } else {
                    self.phase = Phase::Watching { left: left - 1 };
                }
            }
            Phase::Walking { left } => {
                if left == 0 {
                    self.show_field(&mut at_bat, true, game, stage, library);
                } else {
                    self.phase = Phase::Walking { left: left - 1 };
                }
            }
            Phase::Fielding => self.field(&mut at_bat, game, stage, library),
            Phase::Leaving { left } => {
                if left == 0 {
                    // The view is built again, and the batting with it.
                    self.ask_for_a_new_view(&at_bat, stage, library);
                    return None;
                }
                self.phase = Phase::Leaving { left: left - 1 };
            }
            Phase::Ready | Phase::Arriving | Phase::Over => {}
        }
        self.point_the_timing_bar(&mut at_bat, game, stage);
        self.at = Some(at_bat);
        None
    }

    /// Takes in something the stage has reported.
    pub fn event(&mut self, event: &Event, game: &Game, stage: &mut Stage, library: &Library) {
        let Event::Button {
            symbol,
            path,
            event,
        } = event
        else {
            return;
        };
        match (*symbol, *event) {
            (NEXT_BALL_BUTTON, ButtonEvent::Release) if self.phase == Phase::Ready => {
                let Some(at_bat) = &self.at else {
                    return;
                };
                // The panel plays itself out, and a flare covers the change.
                let mut panel = path.clone();
                panel.pop();
                stage.goto_label(&panel, "nextBall", true, library);
                if let Some(flare) = &at_bat.parts.flare {
                    play_from(stage, flare, 2, library);
                }
                self.phase = Phase::Leaving { left: 12 };
            }
            (_, ButtonEvent::Press) => self.runner_button(*symbol, path, game, stage, library),
            _ => {}
        }
    }
}
