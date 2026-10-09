//! Who plays the games that are written down: a batter who reads the pitch
//! and swings, a monkey who reads nothing, and written steps for the menu.
//!
//! Each plays one frame at a time, so that every frame can be taken in.
//! Their chances come from dice of their own, not the game's, so nothing
//! done to the game's own generator can change what they do.

use std::collections::VecDeque;

use bb_engine::display::{Children, Content, child_bounds};
use bb_engine::library::Library;
use bb_engine::math::Matrix;
use bb_format::SymbolId;
use bb_game::art::ButtonLabels;
use bb_game::script::Script;

/// The stage's size, in its own pixels.
const STAGE: (u32, u32) = (590, 400);

pub trait Player {
    /// Plays one frame. Returns false, having played nothing, once there
    /// is nothing left to play.
    fn frame(&mut self, script: &mut Script) -> bool;
}

/// Numbers by chance that are the same every time for the same seed.
pub struct Dice(u64);

impl Dice {
    pub fn new(seed: u64) -> Dice {
        Dice(seed)
    }

    fn roll(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut mixed = self.0;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        mixed ^ (mixed >> 31)
    }

    /// A whole number from nought up to but not including `count`.
    pub fn below(&mut self, count: u32) -> u32 {
        (((self.roll() >> 32) * u64::from(count)) >> 32) as u32
    }

    /// True this many times in a hundred.
    pub fn chance(&mut self, in_a_hundred: u32) -> bool {
        self.below(100) < in_a_hundred
    }

    pub fn pick<T: Copy>(&mut self, of: &[T]) -> T {
        of[self.below(of.len() as u32) as usize]
    }
}

fn run(script: &mut Script, steps: &str) {
    script.run(steps).expect("the steps to run");
}

/// Every button that can be seen on the stage: which it is, and its middle.
pub fn buttons(script: &Script) -> Vec<(SymbolId, f32, f32)> {
    fn look(
        children: &Children,
        matrix: Matrix,
        library: &Library,
        found: &mut Vec<(SymbolId, f32, f32)>,
    ) {
        for child in children.values() {
            if !child.visible || child.clip_depth.is_some() {
                continue;
            }
            match &child.content {
                Content::Button(_) => {
                    if let Some([left, top, right, bottom]) = child_bounds(child, matrix, library) {
                        found.push((child.symbol, (left + right) / 2.0, (top + bottom) / 2.0));
                    }
                }
                Content::Clip(clip) => look(
                    &clip.children,
                    matrix.then_inner(child.matrix),
                    library,
                    found,
                ),
                Content::Graphic => {}
            }
        }
    }
    let mut found = Vec::new();
    let runner = &script.runner;
    look(
        &runner.stage.root.children,
        Matrix::IDENTITY,
        &runner.library,
        &mut found,
    );
    found.retain(|&(_, x, y)| {
        (0.0..STAGE.0 as f32).contains(&x) && (0.0..STAGE.1 as f32).contains(&y)
    });
    found
}

/// The words on the button that gives a game up.
const QUIT: &str = "QUIT";

/// Picks out buttons to click, by chance, and knows what each one says.
struct Hand {
    dice: Dice,
    labels: Option<ButtonLabels>,
}

impl Hand {
    fn new(seed: u64) -> Hand {
        Hand {
            dice: Dice::new(seed),
            labels: None,
        }
    }

    /// Clicks a button that can be seen, any of them, or does `otherwise`
    /// if there is none. The button that gives the game up is passed over
    /// unless `may_quit`: a game given up at once shows very little.
    fn click_a_button(&mut self, script: &mut Script, otherwise: &str, may_quit: bool) {
        let labels = self
            .labels
            .get_or_insert_with(|| ButtonLabels::read(&script.runner.library));
        let mut seen = buttons(script);
        if !may_quit {
            seen.retain(|&(button, _, _)| labels.get(button) != Some(QUIT));
        }
        if seen.is_empty() {
            run(script, otherwise);
        } else {
            let (_, x, y) = self.dice.pick(&seen);
            run(script, &format!("click {x} {y}"));
        }
    }
}

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
/// The button that asks for the next pitch, the one that brings the side in
/// again after the other has batted, a spot on the outfield, and the little
/// field in the corner.
const NEXT_PITCH: &str = "click 545 355";
const NEXT_INNINGS: &str = "click 542 357";
const OUTFIELD: &str = "click 300 200";
const LITTLE_FIELD: &str = "click 60 45";
/// How long the batter stays once the game is over, to see what the result
/// screen does, and how often he presses something there.
const STAYS: u32 = 480;
const PRESSES_EVERY: u32 = 16;

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

/// The keys a monkey may hit.
const KEYS: [&str; 10] = [
    "enter",
    "backspace",
    "tab",
    "left",
    "right",
    "up",
    "down",
    "a",
    "7",
    "space",
];

/// Plays a game without looking at it: moves the pointer about, clicks the
/// buttons it finds and the places that matter, drags, holds the space bar
/// and hits keys.
pub struct Monkey {
    hand: Hand,
    /// Frames left of a drag it has begun.
    dragging: u32,
    holding: bool,
}

impl Monkey {
    pub fn new(seed: u64) -> Monkey {
        Monkey {
            hand: Hand::new(seed),
            dragging: 0,
            holding: false,
        }
    }

    /// Somewhere on the stage: most often about the plate, sometimes on the
    /// little field in the corner, and otherwise anywhere at all.
    fn somewhere(&mut self) -> (u32, u32) {
        let within = |dice: &mut Dice, left: u32, top: u32, wide: u32, high: u32| {
            (left + dice.below(wide), top + dice.below(high))
        };
        let dice = &mut self.hand.dice;
        match dice.below(10) {
            0..5 => within(dice, 150, 150, 300, 200),
            5..7 => within(dice, 20, 10, 90, 80),
            _ => within(dice, 0, 0, STAGE.0, STAGE.1),
        }
    }
}

impl Player for Monkey {
    fn frame(&mut self, script: &mut Script) -> bool {
        if self.dragging > 0 {
            self.dragging -= 1;
            let (x, y) = self.somewhere();
            let then = if self.dragging == 0 { "release; " } else { "" };
            run(script, &format!("move {x} {y}; {then}wait 1"));
            return true;
        }
        match self.hand.dice.below(100) {
            0..55 => run(script, "wait 1"),
            55..70 => {
                let (x, y) = self.somewhere();
                run(script, &format!("move {x} {y}; wait 1"));
            }
            70..82 => {
                // It gives a game up only now and then.
                let may_quit = self.hand.dice.chance(4);
                self.hand.click_a_button(script, "wait 1", may_quit);
            }
            82..92 => {
                let (x, y) = self.somewhere();
                run(script, &format!("click {x} {y}"));
            }
            92..95 => {
                let (x, y) = self.somewhere();
                self.dragging = 2 + self.hand.dice.below(6);
                run(script, &format!("move {x} {y}; press; wait 1"));
            }
            95..98 => {
                self.holding = !self.holding;
                let step = if self.holding { "hold" } else { "lift" };
                run(script, &format!("{step} space; wait 1"));
            }
            _ => {
                let key = self.hand.dice.pick(&KEYS);
                run(script, &format!("key {key}; wait 1"));
            }
        }
        true
    }
}

/// Plays written steps, a frame at a time. The steps are the script's own,
/// and one more: `find WORDS` clicks the button that says just that, if it
/// can be seen, and lets a frame go by if not.
pub struct Written {
    steps: VecDeque<String>,
    labels: Option<ButtonLabels>,
}

impl Written {
    pub fn new(steps: &str) -> Written {
        Written {
            steps: steps
                .split(';')
                .map(str::trim)
                .filter(|step| !step.is_empty())
                .map(str::to_owned)
                .collect(),
            labels: None,
        }
    }

    /// The middle of the button that says `words`, where it can be seen.
    fn find(&mut self, words: &str, script: &Script) -> Option<(f32, f32)> {
        let labels = self
            .labels
            .get_or_insert_with(|| ButtonLabels::read(&script.runner.library));
        buttons(script)
            .into_iter()
            .find(|&(button, _, _)| labels.get(button) == Some(words))
            .map(|(_, x, y)| (x, y))
    }
}

impl Player for Written {
    fn frame(&mut self, script: &mut Script) -> bool {
        while let Some(step) = self.steps.pop_front() {
            let (word, rest) = step.split_once(' ').unwrap_or((&step, ""));
            match word {
                "wait" => {
                    let frames: u32 = rest.parse().expect("a number of frames");
                    if frames == 0 {
                        continue;
                    }
                    self.steps.push_front(format!("wait {}", frames - 1));
                    run(script, "wait 1");
                    return true;
                }
                "click" => {
                    run(script, &step);
                    return true;
                }
                "find" => {
                    match self.find(rest, script) {
                        Some((x, y)) => run(script, &format!("click {x} {y}")),
                        None => {
                            eprintln!("as it was: no button says `{rest}` just now");
                            run(script, "wait 1");
                        }
                    }
                    return true;
                }
                // Anything else plays no frame, and goes with the next that
                // does.
                _ => run(script, &step),
            }
        }
        false
    }
}

/// One player, and when he has nothing left to play, another.
pub struct Then(pub Box<dyn Player>, pub Box<dyn Player>);

impl Player for Then {
    fn frame(&mut self, script: &mut Script) -> bool {
        self.0.frame(script) || self.1.frame(script)
    }
}
