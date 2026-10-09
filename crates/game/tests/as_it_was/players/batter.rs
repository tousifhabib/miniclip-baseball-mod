//! A player who bats: swings at pitches, timed well or badly by its own
//! dice, and uses what the mods in play give it to use.

use bb_game::script::Script;

use super::{Hand, Player, run};

/// How long the batter stays once the game is over, to see what the result
/// screen does, and how often he presses something there.
const STAYS: u32 = 480;
const PRESSES_EVERY: u32 = 16;

/// The button that asks for the next pitch, the one that brings the side in
/// again after the other has batted, a spot on the outfield, and the little
/// field in the corner.
const NEXT_PITCH: &str = "click 545 355";
const NEXT_INNINGS: &str = "click 542 357";
const OUTFIELD: &str = "click 300 200";
const LITTLE_FIELD: &str = "click 60 45";

/// How far from where the ball will cross the batter holds the ring: on it,
/// over it, under it, to one side, and nowhere near.
const OFF: [(f32, f32); 7] = [
    (0.0, 0.0),
    (0.0, 0.0),
    (0.0, 0.0),
    (0.0, -12.0),
    (0.0, 12.0),
    (25.0, 0.0),
    (60.0, 40.0),
];

/// What the state line says about the pitch in hand.
struct Pitch {
    phase: String,
    crossing: Option<(f32, f32)>,
    frames: u32,
    step: u32,
    in_zone: bool,
}

fn pitch(state: &str) -> Pitch {
    let after = |before: &str| state.split(before).nth(1);
    let phase = after(": ")
        .and_then(|rest| rest.split([' ', ',']).next())
        .unwrap_or_default()
        .to_owned();
    let crossing = after("crossing ").and_then(|rest| {
        let (x, rest) = rest.split_once(',')?;
        let y = rest.split(' ').next()?;
        Some((x.parse().ok()?, y.parse().ok()?))
    });
    let number = |before: &str| {
        after(before)
            .and_then(|rest| rest.split(' ').next())
            .and_then(|number| number.parse().ok())
            .unwrap_or(0)
    };
    Pitch {
        phase,
        crossing,
        frames: number(" after "),
        step: number("step: "),
        in_zone: !state.contains("outside the zone"),
    }
}

/// What a batter does besides swing, for the mods that ask something of
/// the player.
#[derive(Clone, Copy, Default)]
pub struct Tricks {
    /// Holds the space bar as the pitch comes in, for bullet time.
    pub slows: bool,
    /// Clicks the outfield before the pitch, to call the shot.
    pub calls: bool,
    /// Clicks the little field in the wind-up, to send a runner.
    pub steals: bool,
}

/// Plays a game as a batter would: puts the ring where the pitch is going,
/// swings at most of them, sends runners on now and then, and asks for the
/// next pitch.
pub struct Batter {
    hand: Hand,
    tricks: Tricks,
    /// What he means to do with the pitch in hand, once he has seen it.
    plan: Option<Plan>,
    swung: bool,
    called: bool,
    sent: bool,
    holding: bool,
    /// Frames the next pitch has been on offer, on the board between
    /// innings, and since the game ended.
    ready: u32,
    between: u32,
    over: u32,
}

#[derive(Clone, Copy)]
struct Plan {
    swings: bool,
    /// How many frames before the ball is gone the swing begins.
    early: u32,
    off: (f32, f32),
    /// How long he leaves the next pitch on offer.
    waits: u32,
}

impl Batter {
    pub fn new(seed: u64, tricks: Tricks) -> Batter {
        Batter {
            hand: Hand::new(seed),
            tricks,
            plan: None,
            swung: false,
            called: false,
            sent: false,
            holding: false,
            ready: 0,
            between: 0,
            over: 0,
        }
    }

    fn plan(&mut self, seen: &Pitch, arcade: bool) -> Plan {
        if let Some(plan) = self.plan {
            return plan;
        }
        // He lets some go by, and goes after a few that are not strikes.
        let dice = &mut self.hand.dice;
        let swings = if seen.in_zone {
            !dice.chance(15)
        } else {
            dice.chance(20)
        };
        let plan = Plan {
            swings,
            early: 20 + dice.below(9),
            off: dice.pick(&OFF),
            // In the arcade game the next pitch is offered while the ball
            // is still in the air, and taking it then loses the points.
            waits: if arcade { dice.pick(&[5, 150, 400]) } else { 2 },
        };
        self.plan = Some(plan);
        plan
    }
}

impl Player for Batter {
    fn frame(&mut self, script: &mut Script) -> bool {
        let now = script.runner.describe();
        let seen = pitch(&now);
        if self.holding && seen.phase != "Flight" {
            run(script, "lift space");
            self.holding = false;
        }
        if now.starts_with("Interval") {
            self.between += 1;
            let step = if self.between.is_multiple_of(30) {
                NEXT_INNINGS
            } else {
                "wait 1"
            };
            run(script, step);
            return true;
        }
        self.between = 0;
        let playing = ["Match,", "FullMatch,", "Arcade,", "Loading"]
            .iter()
            .any(|screen| now.starts_with(screen));
        if !playing {
            if self.over == STAYS {
                return false;
            }
            self.over += 1;
            if self.over.is_multiple_of(PRESSES_EVERY) {
                // A result screen's way out is a badge that is drawn, where
                // the next pitch was asked for.
                self.hand.click_a_button(script, NEXT_PITCH, false);
            } else {
                run(script, "wait 1");
            }
            return true;
        }
        if seen.phase != "Ready" {
            self.ready = 0;
        }
        let arcade = now.starts_with("Arcade");
        match seen.phase.as_str() {
            "Settling" => {
                let plan = self.plan(&seen, arcade);
                if self.tricks.calls && !self.called {
                    self.called = true;
                    run(script, OUTFIELD);
                    return true;
                }
                if let Some((x, y)) = seen.crossing {
                    let (x, y) = (x + plan.off.0, y + plan.off.1);
                    run(script, &format!("move {x} {y}"));
                }
                run(script, "wait 1");
            }
            "WindUp" if self.tricks.steals && !self.sent => {
                self.sent = true;
                run(script, LITTLE_FIELD);
            }
            "Flight" => {
                let plan = self.plan(&seen, arcade);
                if self.tricks.slows && !self.holding && !self.swung && seen.step >= 10 {
                    run(script, "hold space");
                    self.holding = true;
                }
                let swing_on = seen.frames.saturating_sub(plan.early);
                match seen.crossing {
                    Some((x, y)) if plan.swings && !self.swung && seen.step >= swing_on => {
                        self.swung = true;
                        let (x, y) = (x + plan.off.0, y + plan.off.1);
                        run(script, &format!("click {x} {y}"));
                    }
                    _ => run(script, "wait 1"),
                }
            }
            // A runner is sent on now and then, or told to slide.
            "Fielding" if self.hand.dice.chance(6) => {
                self.hand.click_a_button(script, "wait 1", false);
            }
            "Ready" => {
                let waits = self.plan.map_or(2, |plan| plan.waits);
                self.ready += 1;
                if self.ready >= waits && (self.ready - waits).is_multiple_of(3) {
                    run(script, NEXT_PITCH);
                } else {
                    run(script, "wait 1");
                }
                (self.swung, self.called, self.sent) = (false, false, false);
            }
            _ => run(script, "wait 1"),
        }
        // The plan lasts until the next pitch is asked for and comes.
        if seen.phase == "Leaving" || seen.phase == "Arriving" {
            self.plan = None;
        }
        true
    }
}
