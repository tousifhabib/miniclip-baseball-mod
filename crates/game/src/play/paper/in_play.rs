//! A ball put in play on paper: where it went, and what that does to
//! the batter and to the runners.

use super::OUTS;
use super::pitches::Struck;
use super::play::Play;
use crate::play::book::{End, Hit};
use crate::play::field::Ground;
use crate::rng::Rng;

/// The share of the flies that are caught that go no further than the
/// infield, and of the singles that are hit along the ground.
const POP_UP: f32 = 0.2;
const GROUND_SINGLE: f32 = 0.45;

/// On a fly caught in the outfield, a runner on third tags up and comes
/// home, and one on second goes to third.
const FLY_HOME: f32 = 0.55;
const FLY_THIRD: f32 = 0.2;

/// On any other ground out, a runner on third comes home, one on second
/// goes to third, and one on first to second.
const GROUNDER_HOME: f32 = 0.45;
const GROUNDER_THIRD: f32 = 0.5;
const GROUNDER_SECOND: f32 = 0.6;

/// A ground ball with a runner on first and fewer than two out puts both
/// out.
const DOUBLE_PLAY: f32 = 0.18;

/// On a double, a runner on first comes home.
const DOUBLE_HOME_FROM_FIRST: f32 = 0.45;

/// How the runners go on. Each is how often it happens.
///
/// On a single, a runner on second comes home and one on first makes third.
const SINGLE_HOME_FROM_SECOND: f32 = 0.6;
const SINGLE_THIRD_FROM_FIRST: f32 = 0.3;

impl Play<'_> {
    /// Works out what a ball in play comes to: where it went, where the
    /// runners got to, who came home and who was put out.
    pub(super) fn put_in_play(
        &mut self,
        struck: Struck,
        order: usize,
        home: &mut Vec<usize>,
        outs_made: &mut u32,
        rng: &mut Rng,
    ) -> (End, Hit) {
        let ground = self.ground;
        let (across, far, fly) = Play::where_it_went(struck, ground, rng);
        let hit = Hit::at(ground, ground.point(across, far), fly, None);
        let end = match struck {
            Struck::HomeRun => self.on_a_home_run(order, home),
            Struck::Triple => self.on_a_triple(order, home),
            Struck::Double => self.on_a_double(order, home, rng),
            Struck::Single => self.on_a_single(order, home, rng),
            Struck::GroundOut => self.on_a_ground_out(home, outs_made, rng),
            Struck::FlyOut => self.on_a_fly_out(hit.deep, home, outs_made, rng),
        };
        (end, hit)
    }

    /// Where a ball that was struck this way went: how far across the
    /// field and how far from home, and whether in the air.
    fn where_it_went(struck: Struck, ground: &Ground, rng: &mut Rng) -> (f32, f32, bool) {
        let anywhere = |rng: &mut Rng| rng.between(0.03, 0.97);
        match struck {
            Struck::GroundOut => (
                anywhere(rng),
                rng.between(150.0, ground.infield - 15.0),
                false,
            ),
            Struck::FlyOut if rng.chance(POP_UP) => (
                anywhere(rng),
                rng.between(220.0, ground.infield - 5.0),
                true,
            ),
            Struck::FlyOut => (
                anywhere(rng),
                rng.between(ground.infield + 40.0, ground.wall - 25.0),
                true,
            ),
            Struck::Single if rng.chance(GROUND_SINGLE) => (
                anywhere(rng),
                rng.between(230.0, ground.infield - 5.0),
                false,
            ),
            Struck::Single => (
                anywhere(rng),
                rng.between(ground.infield + 15.0, ground.infield + 200.0),
                true,
            ),
            // Doubles go down the lines and into the gaps.
            Struck::Double => {
                let spots = [(0.03, 0.14), (0.28, 0.4), (0.6, 0.72), (0.86, 0.97)];
                let (from, to) = spots[rng.below(spots.len() as u32) as usize];
                (
                    rng.between(from, to),
                    rng.between(ground.infield + 170.0, ground.wall - 4.0),
                    true,
                )
            }
            Struck::Triple => (
                rng.between(0.55, 0.97),
                rng.between(ground.wall - 110.0, ground.wall - 2.0),
                true,
            ),
            Struck::HomeRun => (
                anywhere(rng),
                rng.between(ground.wall + 12.0, ground.wall + 190.0),
                true,
            ),
        }
    }

    /// Everybody comes home, the batter with them.
    fn on_a_home_run(&mut self, order: usize, home: &mut Vec<usize>) -> End {
        let [first, second, third] = self.bases;
        home.extend([third, second, first].into_iter().flatten());
        home.push(order);
        self.bases = [None; 3];
        End::HomeRun
    }

    /// Everybody on base comes home, and the batter is on third.
    fn on_a_triple(&mut self, order: usize, home: &mut Vec<usize>) -> End {
        let [first, second, third] = self.bases;
        home.extend([third, second, first].into_iter().flatten());
        self.bases = [None, None, Some(order)];
        End::Triple
    }

    /// The runners on second and third come home. The one on first may,
    /// or is held at third.
    fn on_a_double(&mut self, order: usize, home: &mut Vec<usize>, rng: &mut Rng) -> End {
        let [first, second, third] = self.bases;
        home.extend([third, second].into_iter().flatten());
        let mut held = None;
        if let Some(runner) = first {
            if rng.chance(DOUBLE_HOME_FROM_FIRST) {
                home.push(runner);
            } else {
                held = Some(runner);
            }
        }
        self.bases = [None, Some(order), held];
        End::Double
    }

    /// The runner on third comes home. The one on second may, or stops at
    /// third, and the one on first takes third if it is free and he
    /// chances it.
    fn on_a_single(&mut self, order: usize, home: &mut Vec<usize>, rng: &mut Rng) -> End {
        let [first, second, third] = self.bases;
        home.extend(third);
        let mut on_third = None;
        if let Some(runner) = second {
            if rng.chance(SINGLE_HOME_FROM_SECOND) {
                home.push(runner);
            } else {
                on_third = Some(runner);
            }
        }
        let mut on_second = None;
        if let Some(runner) = first {
            if on_third.is_none() && rng.chance(SINGLE_THIRD_FROM_FIRST) {
                on_third = Some(runner);
            } else {
                on_second = Some(runner);
            }
        }
        self.bases = [Some(order), on_second, on_third];
        End::Single
    }

    /// The batter is out at first, or he and the runner from first both
    /// are. Short of the last out, the others may move up.
    fn on_a_ground_out(
        &mut self,
        home: &mut Vec<usize>,
        outs_made: &mut u32,
        rng: &mut Rng,
    ) -> End {
        let [first, second, third] = self.bases;
        let two_out = self.outs + 1 == OUTS;
        if !two_out && first.is_some() && rng.chance(DOUBLE_PLAY) {
            // The runner from first and the batter are both out,
            // and the others stay where they are.
            *outs_made = 2;
            self.bases = [None, second, third];
            return End::DoublePlay;
        }
        *outs_made = 1;
        if !two_out {
            let mut bases = [first, second, third];
            if bases[2].is_some() && rng.chance(GROUNDER_HOME) {
                home.extend(bases[2].take());
            }
            if bases[2].is_none() && bases[1].is_some() && rng.chance(GROUNDER_THIRD) {
                bases[2] = bases[1].take();
            }
            if bases[1].is_none() && bases[0].is_some() && rng.chance(GROUNDER_SECOND) {
                bases[1] = bases[0].take();
            }
            self.bases = bases;
        }
        End::GroundOut
    }

    /// The batter is caught out. On a fly to the outfield that is not the
    /// last out, a runner on third may tag up and come home, and one on
    /// second go to third.
    fn on_a_fly_out(
        &mut self,
        deep: bool,
        home: &mut Vec<usize>,
        outs_made: &mut u32,
        rng: &mut Rng,
    ) -> End {
        let [first, second, third] = self.bases;
        let two_out = self.outs + 1 == OUTS;
        *outs_made = 1;
        let mut end = End::FlyOut;
        if !two_out && deep {
            let mut bases = [first, second, third];
            if bases[2].is_some() && rng.chance(FLY_HOME) {
                home.extend(bases[2].take());
                end = End::SacrificeFly;
            }
            if bases[2].is_none() && bases[1].is_some() && rng.chance(FLY_THIRD) {
                bases[2] = bases[1].take();
            }
            self.bases = bases;
        }
        end
    }
}
