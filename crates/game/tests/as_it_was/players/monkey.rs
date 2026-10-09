//! A player who looks at nothing: moves, clicks, drags, holds keys and
//! types, all by its own dice.

use bb_game::script::Script;

use super::{Dice, Hand, Player, STAGE, run};

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
