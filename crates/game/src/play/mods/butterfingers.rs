//! Butterfingers: fielders drop and fumble the ball, as often as you set.
//!
//! A fielder under a fly ball drops it, so the batter is not out and the
//! ball is on the ground. A fielder bending for a ball on the ground
//! fumbles it, and has to go after it again. And the fielder at a base
//! fails to hold a throw, so the runner it would have beaten is safe.
//! Whoever did it has a word over him for a moment. The arcade game has no
//! fielders, and plays as it did.

use crate::rng::Rng;
use crate::rules::ButterfingersRules;

pub(crate) struct Butterfingers {
    /// How many goes in a hundred a fielder lets the ball go.
    chance: u32,
    /// How often one has, this game.
    slips: u32,
}

impl Butterfingers {
    /// `level` is the setting the mod is at, the first being 1.
    pub fn new(rules: &ButterfingersRules, level: u8) -> Butterfingers {
        Butterfingers {
            chance: rules.chance.at(level).unwrap_or(0),
            slips: 0,
        }
    }

    pub fn slips(&self) -> u32 {
        self.slips
    }

    /// Whether a fielder having a go at the ball lets it go. One number is
    /// drawn for every go.
    pub fn lets_go(&mut self, rng: &mut Rng) -> bool {
        let slips = rng.below(100) < self.chance;
        if slips {
            self.slips += 1;
        }
        slips
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rules;

    #[test]
    fn at_the_top_setting_every_go_is_let_go_and_each_is_counted() {
        let rules = Rules::default().butterfingers;
        let mut butter = Butterfingers::new(&rules, rules.chance.count());
        let mut rng = Rng::new(1);
        assert!((0..50).all(|_| butter.lets_go(&mut rng)));
        assert_eq!(butter.slips(), 50);
    }

    #[test]
    fn at_the_bottom_setting_about_a_fifth_are_let_go() {
        let rules = Rules::default().butterfingers;
        let mut butter = Butterfingers::new(&rules, 1);
        let mut rng = Rng::new(1);
        let slips = (0..2000).filter(|_| butter.lets_go(&mut rng)).count();
        assert!((320..480).contains(&slips), "{slips}");
        assert_eq!(butter.slips() as usize, slips);
    }
}
