//! Random numbers for the rules.
//!
//! The generator is our own so that a seed always gives the same game: a
//! test can replay one, and so can anybody chasing a fault. It is
//! xoshiro256**, started from the seed with SplitMix64, as its authors
//! advise.

use std::time::{SystemTime, UNIX_EPOCH};

/// What a game's seed is mixed with by each thing that draws numbers of
/// its own, so that none of them draws the numbers another does. The
/// pitches are drawn from the seed itself, and so come the same whichever
/// of these are in play. They are kept in one place so that it can be
/// seen, and is checked, that no two are the same.
pub(crate) mod mixed_with {
    /// The other side's innings in a full match: the same pitches come
    /// whichever side bats first.
    pub const THEIR_INNINGS: u64 = 0x6a09_e667_f3bc_c908;
    /// Which sign on the wall is lit: the same pitches come whether the
    /// hit the sign mod is on or not.
    pub const THE_SIGNS: u64 = 0xa54f_f53a_5f1d_36f1;
    /// The playing out of each of their innings on paper, many times over
    /// by the number of the innings, so that each comes out differently
    /// from the last.
    pub const THEIR_INNINGS_ON_PAPER: u64 = 0x3c6e_f372_fe94_f82b;
    /// The toss of the coin for where a full match is played.
    pub const THE_COIN: u64 = 0xbb67_ae85_84ca_a73b;
    /// The draw of a tournament: which sides are in it, and in what
    /// order.
    pub const THE_DRAW: u64 = 0x510e_527f_ade6_82d1;
    /// A fixture of a tournament, many times over by its number, so that
    /// each is played from numbers of its own. It is odd, so that no two
    /// multiples of it are the same.
    pub const A_FIXTURE: u64 = 0x9b05_688c_2b3e_6c1f;
}

#[derive(Clone, Debug)]
pub struct Rng {
    state: [u64; 4],
}

impl Rng {
    /// A generator that gives the same numbers every time for the same seed.
    pub fn new(seed: u64) -> Rng {
        let mut mixer = seed;
        let mut next = || {
            mixer = mixer.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = mixer;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        };
        Rng {
            state: [next(), next(), next(), next()],
        }
    }

    /// A seed that differs from one run to the next.
    pub fn seed_from_clock() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos() as u64)
    }

    fn next(&mut self) -> u64 {
        let [a, b, c, d] = &mut self.state;
        let result = b.wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let shifted = *b << 17;
        *c ^= *a;
        *d ^= *b;
        *b ^= *c;
        *a ^= *d;
        *c ^= shifted;
        *d = d.rotate_left(45);
        result
    }

    /// A number from 0 up to, but not reaching, 1.
    pub fn unit(&mut self) -> f32 {
        // The top 24 bits: every value a float can hold exactly in that span.
        (self.next() >> 40) as f32 / (1u32 << 24) as f32
    }

    /// A number from `low` up to, but not reaching, `high`.
    pub fn between(&mut self, low: f32, high: f32) -> f32 {
        let drawn = low + (high - low) * self.unit();
        // What is worked out is rounded to the nearest number a float can
        // hold, and now and then that is `high` itself. The number just
        // under it is as near as can be got without reaching it.
        if low < high && drawn >= high {
            high.next_down()
        } else {
            drawn
        }
    }

    /// A whole number from 0 up to, but not reaching, `count`. Zero if
    /// `count` is zero.
    pub fn below(&mut self, count: u32) -> u32 {
        // The high half of a 64 by 32 bit product: even enough for a game,
        // with no division.
        (((self.next() >> 32) * u64::from(count)) >> 32) as u32
    }

    /// True this share of the time: never for 0, always for 1.
    pub fn chance(&mut self, share: f32) -> bool {
        self.unit() < share
    }
}

#[cfg(test)]
mod tests {
    // By name, because all of what proptest offers includes a generator of
    // its own called `Rng`.
    use proptest::prelude::{prop_assert, prop_assert_eq, proptest};

    use super::*;

    #[test]
    fn the_same_seed_gives_the_same_numbers() {
        let (mut a, mut b) = (Rng::new(7), Rng::new(7));
        let firsts: Vec<u64> = (0..8).map(|_| a.next()).collect();
        let seconds: Vec<u64> = (0..8).map(|_| b.next()).collect();
        assert_eq!(firsts, seconds);
        assert_ne!(
            firsts,
            (0..8).map(|_| Rng::new(8).next()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_first_numbers_for_a_seed_never_change() {
        // If this fails, every recorded game and seeded test has changed
        // with it.
        let mut rng = Rng::new(1);
        assert_eq!(
            [rng.next(), rng.next(), rng.next()],
            [
                0xb3f2_af6d_0fc7_10c5,
                0x853b_5596_4736_4cea,
                0x92f8_9756_082a_4514,
            ]
        );
    }

    #[test]
    fn numbers_stay_inside_what_was_asked_for() {
        let mut rng = Rng::new(3);
        for _ in 0..10_000 {
            let unit = rng.unit();
            assert!((0.0..1.0).contains(&unit));
            let between = rng.between(-5.0, 12.0);
            assert!((-5.0..12.0).contains(&between));
            assert!(rng.below(6) < 6);
        }
        assert_eq!(rng.below(0), 0);
    }

    #[test]
    fn every_outcome_turns_up_about_as_often() {
        let mut rng = Rng::new(11);
        let mut counts = [0u32; 6];
        for _ in 0..60_000 {
            counts[rng.below(6) as usize] += 1;
        }
        for count in counts {
            assert!((9_500..10_500).contains(&count), "{counts:?}");
        }
    }

    #[test]
    fn chance_follows_its_share() {
        let mut rng = Rng::new(5);
        assert!((0..1000).all(|_| !rng.chance(0.0)));
        assert!((0..1000).all(|_| rng.chance(1.0)));
        let hits = (0..20_000).filter(|_| rng.chance(0.25)).count();
        assert!((4_700..5_300).contains(&hits), "{hits}");
    }

    proptest! {
        #[test]
        fn whatever_the_seed_the_numbers_stay_inside_what_was_asked_for(
            seed: u64,
            count in 1u32..,
            low in -1.0e6f32..1.0e6,
            more in 0.0f32..1.0e6,
        ) {
            let high = low + more;
            let mut rng = Rng::new(seed);
            for _ in 0..32 {
                let unit = rng.unit();
                prop_assert!((0.0..1.0).contains(&unit), "{}", unit);
                prop_assert!(rng.below(count) < count);
                prop_assert_eq!(rng.below(0), 0);
                // No lower than the one, and short of the other, unless
                // the two are the same and there is nothing between them.
                let between = rng.between(low, high);
                if low < high {
                    prop_assert!((low..high).contains(&between), "{}", between);
                } else {
                    prop_assert_eq!(between, low);
                }
                prop_assert!(!rng.chance(0.0));
                prop_assert!(rng.chance(1.0));
            }
        }
    }

    #[test]
    fn no_two_things_that_draw_numbers_of_their_own_mix_the_seed_with_the_same() {
        let all = [
            mixed_with::THEIR_INNINGS,
            mixed_with::THE_SIGNS,
            mixed_with::THEIR_INNINGS_ON_PAPER,
            mixed_with::THE_COIN,
            mixed_with::THE_DRAW,
            mixed_with::A_FIXTURE,
        ];
        for (index, one) in all.iter().enumerate() {
            assert!(!all[index + 1..].contains(one), "{one:#x} is there twice");
        }
        // And so the first number each draws for a game is its own.
        let firsts: Vec<u32> = all
            .iter()
            .map(|with| Rng::new(7 ^ with).below(1_000_000))
            .collect();
        for (index, first) in firsts.iter().enumerate() {
            assert!(!firsts[index + 1..].contains(first), "{firsts:?}");
        }
    }

    #[test]
    fn a_number_between_two_is_never_the_higher_of_them_however_far_from_nought() {
        // This far from nought the numbers a float can hold are two apart,
        // so anything worked out over half way from `low` is rounded up to
        // `high` itself, and once was handed back as that.
        let (low, high) = (16_777_216.0, 16_777_218.0);
        let mut rng = Rng::new(1);
        let drawn: Vec<f32> = (0..100).map(|_| rng.between(low, high)).collect();
        assert!(drawn.iter().all(|&number| number == low), "{drawn:?}");
        // And at the size the game asks for, the very top of what can be
        // drawn stops short too.
        let mut rng = Rng::new(1);
        for _ in 0..100_000 {
            let number = rng.between(150.0, 400.0);
            assert!((150.0..400.0).contains(&number), "{number}");
        }
    }
}
