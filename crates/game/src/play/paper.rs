//! The other side's innings, played out on paper.
//!
//! How many runs an innings of theirs comes to is settled first, by the
//! skill level. Then the innings is played here a pitch at a time, over and
//! over, until it comes to just that many, and that playing of it is the
//! one that goes in the book. Nothing is made up to fit afterwards, so
//! everything the book says of them adds up.

use super::book::{End, Hit, ORDER, Pitch, Steal, Thrown, Turn};
use super::field::Ground;
use crate::rng::Rng;
use crate::rules::{StealRules, TheirBattingRules};

/// A half of an innings as it was played on paper.
#[derive(Clone, Debug, PartialEq)]
pub struct Half {
    pub turns: Vec<Turn>,
    /// The tries at stealing a base there were in it. Each is told by how
    /// many of this half's turns were over when it was made.
    pub steals: Vec<Steal>,
    /// How many runners were left on base.
    pub left: u32,
    /// The runs each place in the order made in it.
    pub runs: [u32; ORDER],
    /// Whose turn it is next.
    pub next: usize,
}

/// How many times an innings is played before it is given up on, and how
/// many batters one playing of it may have.
const TRIES: u32 = 6000;
const MOST_TURNS: usize = 45;
/// How many pitches a batter may see before the next he swings at is put in
/// play.
const MOST_PITCHES: usize = 12;
/// How much likelier the hits are made, or rarer, after a playing that came
/// to too few runs or too many, and how far that can go.
const LEAN: f32 = 1.04;
const MOST_LEAN: f32 = 40.0;
/// How much of that the singles take, and the home runs, the doubles and
/// triples taking it as it is.
const SINGLES_LEAN: f32 = 0.3;
const HOME_RUNS_LEAN: f32 = 1.8;

/// How the runners go on. Each is how often it happens.
///
/// On a single, a runner on second comes home and one on first makes third.
const SINGLE_HOME_FROM_SECOND: f32 = 0.6;
const SINGLE_THIRD_FROM_FIRST: f32 = 0.3;
/// On a double, a runner on first comes home.
const DOUBLE_HOME_FROM_FIRST: f32 = 0.45;
/// A ground ball with a runner on first and fewer than two out puts both
/// out.
const DOUBLE_PLAY: f32 = 0.18;
/// On any other ground out, a runner on third comes home, one on second
/// goes to third, and one on first to second.
const GROUNDER_HOME: f32 = 0.45;
const GROUNDER_THIRD: f32 = 0.5;
const GROUNDER_SECOND: f32 = 0.6;
/// On a fly caught in the outfield, a runner on third tags up and comes
/// home, and one on second goes to third.
const FLY_HOME: f32 = 0.55;
const FLY_THIRD: f32 = 0.2;
/// The share of the flies that are caught that go no further than the
/// infield, and of the singles that are hit along the ground.
const POP_UP: f32 = 0.2;
const GROUND_SINGLE: f32 = 0.45;

/// What a ball put in play comes to, before it is known what the runners
/// do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Struck {
    GroundOut,
    FlyOut,
    Single,
    Double,
    Triple,
    HomeRun,
}

/// Why a playing of an innings was no good.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Miss {
    TooFew,
    TooMany,
}

/// Plays the half of an innings that is to come to `made` runs. `winning`
/// is whether those runs win the match, which then ends the moment the
/// last of them is in. `first_up` is whose turn it is. With `steals` their
/// runners try for a base now and then, as those rules say.
#[allow(clippy::too_many_arguments)]
pub fn half(
    made: u32,
    winning: bool,
    innings: u32,
    first_up: usize,
    rules: &TheirBattingRules,
    steals: Option<&StealRules>,
    ground: &Ground,
    rng: &mut Rng,
) -> Half {
    // An innings of many runs is one in which the hits came easily, so
    // each playing that falls short makes them likelier for the next.
    let mut lean = 1.0_f32;
    for _ in 0..TRIES {
        let mut play = Play::new(made, winning, innings, first_up, lean, rules, ground);
        play.steals = steals;
        match play.out(rng) {
            Ok(()) => return play.half(),
            Err(Miss::TooFew) => lean = (lean * LEAN).min(MOST_LEAN),
            Err(Miss::TooMany) => lean = (lean / LEAN).max(1.0 / MOST_LEAN),
        }
    }
    plainly(made, winning, innings, first_up, ground)
}

/// An innings of just so many runs, for when none would come out that way
/// by itself: a home run for each, and then three strikeouts.
fn plainly(made: u32, winning: bool, innings: u32, first_up: usize, ground: &Ground) -> Half {
    let mut half = Half {
        turns: Vec::new(),
        steals: Vec::new(),
        left: 0,
        runs: [0; ORDER],
        next: first_up,
    };
    let strike = |thrown| Pitch {
        in_zone: true,
        thrown,
        off: None,
        quality: None,
    };
    let turn = |half: &mut Half, end: End, outs: u32| {
        let order = half.next % ORDER;
        half.next = (order + 1) % ORDER;
        let home_run = end == End::HomeRun;
        if home_run {
            half.runs[order] += 1;
        }
        half.turns.push(Turn {
            innings,
            order,
            outs,
            on: [false; 3],
            pitches: if home_run {
                vec![strike(Thrown::InPlay)]
            } else {
                vec![
                    strike(Thrown::Called),
                    strike(Thrown::Swinging),
                    strike(Thrown::Swinging),
                ]
            },
            end,
            ball: home_run
                .then(|| Hit::at(ground, ground.point(0.5, ground.wall + 60.0), true, None)),
            runs_in: u32::from(home_run),
            outs_made: u32::from(!home_run),
        });
    };
    for _ in 0..made {
        turn(&mut half, End::HomeRun, 0);
    }
    if !winning {
        for outs in 0..3 {
            turn(&mut half, End::Strikeout, outs);
        }
    }
    half
}

/// One playing of a half.
struct Play<'a> {
    made: u32,
    winning: bool,
    innings: u32,
    lean: f32,
    rules: &'a TheirBattingRules,
    /// What their runners steal by, if they steal at all.
    steals: Option<&'a StealRules>,
    ground: &'a Ground,
    /// Who is on first, second and third: his place in the order.
    bases: [Option<usize>; 3],
    outs: u32,
    runs: u32,
    up: usize,
    turns: Vec<Turn>,
    stolen: Vec<Steal>,
    by_order: [u32; ORDER],
    /// Runners who were on their way home when the match was won, and so
    /// never got there.
    stranded: u32,
}

impl<'a> Play<'a> {
    fn new(
        made: u32,
        winning: bool,
        innings: u32,
        first_up: usize,
        lean: f32,
        rules: &'a TheirBattingRules,
        ground: &'a Ground,
    ) -> Play<'a> {
        Play {
            made,
            winning,
            innings,
            lean,
            rules,
            steals: None,
            ground,
            bases: [None; 3],
            outs: 0,
            runs: 0,
            up: first_up % ORDER,
            turns: Vec::new(),
            stolen: Vec::new(),
            by_order: [0; ORDER],
            stranded: 0,
        }
    }

    fn half(self) -> Half {
        Half {
            turns: self.turns,
            steals: self.stolen,
            left: self.bases.iter().flatten().count() as u32 + self.stranded,
            runs: self.by_order,
            next: self.up,
        }
    }

    /// Plays the half out. It is good if it comes to the runs it was to.
    fn out(&mut self, rng: &mut Rng) -> Result<(), Miss> {
        while self.outs < 3 {
            if self.turns.len() >= MOST_TURNS {
                return Err(Miss::TooMany);
            }
            // A runner thrown out stealing may be the last out there is.
            self.steal(rng);
            if self.outs >= 3 {
                break;
            }
            self.turn(rng)?;
            if self.winning && self.runs >= self.made {
                return Ok(());
            }
            if self.runs > self.made {
                return Err(Miss::TooMany);
            }
        }
        if self.winning || self.runs < self.made {
            return Err(Miss::TooFew);
        }
        Ok(())
    }

    /// Before a batter's turn, a runner with the base in front of him empty
    /// may go for it: the one on second if there is one, and if he stays
    /// the one on first. He gets there or he is out.
    fn steal(&mut self, rng: &mut Rng) {
        let Some(rules) = self.steals else {
            return;
        };
        let [first, second, third] = self.bases;
        let from = if second.is_some() && third.is_none() {
            rng.chance(rules.their_chance * rules.their_third)
                .then_some(1)
        } else if first.is_some() && second.is_none() {
            rng.chance(rules.their_chance).then_some(0)
        } else {
            None
        };
        let Some((from, runner)) = from.and_then(|from| Some((from, self.bases[from].take()?)))
        else {
            return;
        };
        let safe = rng.chance(rules.their_safe);
        if safe {
            self.bases[from + 1] = Some(runner);
        } else {
            self.outs += 1;
        }
        self.stolen.push(Steal {
            innings: self.innings,
            order: runner,
            base: from as u8 + 2,
            safe,
            at: self.turns.len(),
        });
    }

    /// One batter's turn.
    fn turn(&mut self, rng: &mut Rng) -> Result<(), Miss> {
        let order = self.up;
        let (outs, on) = (self.outs, self.bases.map(|base| base.is_some()));
        let (pitches, struck) = self.pitches(rng);
        let mut home: Vec<usize> = Vec::new();
        let mut outs_made = 0;
        let mut ball = None;
        let end = match struck {
            // Three strikes, or four balls.
            None if pitches.last().is_some_and(Pitch::strike) => {
                outs_made = 1;
                End::Strikeout
            }
            None => {
                self.walk(order, &mut home);
                End::Walk
            }
            Some(struck) => {
                let (end, hit) = self.put_in_play(struck, order, &mut home, &mut outs_made, rng);
                ball = Some(hit);
                end
            }
        };
        // The run that wins the match ends it: nobody behind it comes in,
        // unless the ball went out of the park, when they all do.
        let wanted = self.made.saturating_sub(self.runs) as usize;
        if self.winning && home.len() > wanted {
            if end == End::HomeRun {
                return Err(Miss::TooMany);
            }
            self.stranded += (home.len() - wanted) as u32;
            home.truncate(wanted);
        }
        for &scorer in &home {
            self.by_order[scorer] += 1;
        }
        self.runs += home.len() as u32;
        self.outs += outs_made;
        self.up = (order + 1) % ORDER;
        self.turns.push(Turn {
            innings: self.innings,
            order,
            outs,
            on,
            pitches,
            end,
            ball,
            runs_in: home.len() as u32,
            outs_made,
        });
        Ok(())
    }

    /// The pitches of a turn, and what the last was struck for if it was
    /// put in play.
    fn pitches(&self, rng: &mut Rng) -> (Vec<Pitch>, Option<Struck>) {
        let rules = self.rules;
        let mut pitches = Vec::new();
        let (mut balls, mut strikes) = (0, 0);
        loop {
            let in_zone = rng.chance(rules.zone);
            let which = usize::from(!in_zone);
            let tired = pitches.len() >= MOST_PITCHES;
            let swung = tired || rng.chance(rules.swing[which]);
            let thrown = if !swung {
                if in_zone {
                    Thrown::Called
                } else {
                    Thrown::Ball
                }
            } else if !tired && !rng.chance(rules.contact[which]) {
                Thrown::Swinging
            } else if !tired && rng.chance(rules.foul) {
                Thrown::Foul
            } else {
                Thrown::InPlay
            };
            pitches.push(Pitch {
                in_zone,
                thrown,
                off: None,
                quality: None,
            });
            match thrown {
                Thrown::Ball => balls += 1,
                Thrown::Called | Thrown::Swinging => strikes += 1,
                // A foul is a strike, but never the last one.
                Thrown::Foul => strikes = (strikes + 1).min(2),
                Thrown::InPlay => return (pitches, Some(self.struck(rng))),
            }
            if balls == 4 || strikes == 3 {
                return (pitches, None);
            }
        }
    }

    /// What a ball put in play comes to.
    fn struck(&self, rng: &mut Rng) -> Struck {
        let rules = self.rules;
        // Runs in number come of long hits more than of many short ones.
        let lean = |by: f32| self.lean.powf(by);
        let chances = [
            (Struck::GroundOut, rules.ground_out),
            (Struck::FlyOut, rules.fly_out),
            (Struck::Single, rules.single * lean(SINGLES_LEAN)),
            (Struck::Double, rules.double * lean(1.0)),
            (Struck::Triple, rules.triple * lean(1.0)),
            (Struck::HomeRun, rules.home_run * lean(HOME_RUNS_LEAN)),
        ];
        let all: f32 = chances.iter().map(|(_, chance)| chance.max(0.0)).sum();
        let mut left = rng.unit() * all;
        for (struck, chance) in chances {
            left -= chance.max(0.0);
            if left < 0.0 {
                return struck;
            }
        }
        Struck::GroundOut
    }

    /// Four balls: the batter takes first, and everyone he pushes moves up.
    fn walk(&mut self, order: usize, home: &mut Vec<usize>) {
        let mut coming = Some(order);
        for base in &mut self.bases {
            match coming {
                Some(_) => coming = std::mem::replace(base, coming),
                None => break,
            }
        }
        home.extend(coming);
    }

    /// Works out what a ball in play comes to: where it went, where the
    /// runners got to, who came home and who was put out.
    fn put_in_play(
        &mut self,
        struck: Struck,
        order: usize,
        home: &mut Vec<usize>,
        outs_made: &mut u32,
        rng: &mut Rng,
    ) -> (End, Hit) {
        let ground = self.ground;
        let [first, second, third] = self.bases;
        let two_out = self.outs == 2;
        // How far across the field it went and how far from home, and
        // whether in the air.
        let anywhere = |rng: &mut Rng| rng.between(0.03, 0.97);
        let (across, far, fly) = match struck {
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
        };
        let hit = Hit::at(ground, ground.point(across, far), fly, None);
        let end = match struck {
            Struck::HomeRun => {
                home.extend([third, second, first].into_iter().flatten());
                home.push(order);
                self.bases = [None; 3];
                End::HomeRun
            }
            Struck::Triple => {
                home.extend([third, second, first].into_iter().flatten());
                self.bases = [None, None, Some(order)];
                End::Triple
            }
            Struck::Double => {
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
            Struck::Single => {
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
            Struck::GroundOut => {
                if !two_out && first.is_some() && rng.chance(DOUBLE_PLAY) {
                    // The runner from first and the batter are both out,
                    // and the others stay where they are.
                    *outs_made = 2;
                    self.bases = [None, second, third];
                    return (End::DoublePlay, hit);
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
            Struck::FlyOut => {
                *outs_made = 1;
                let mut end = End::FlyOut;
                if !two_out && hit.deep {
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
        };
        (end, hit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::book::Figures;
    use crate::rules::Rules;

    fn played(made: u32, winning: bool, first_up: usize, seed: u64) -> Half {
        played_by(made, winning, first_up, seed, None)
    }

    fn played_by(
        made: u32,
        winning: bool,
        first_up: usize,
        seed: u64,
        steals: Option<&StealRules>,
    ) -> Half {
        let rules = Rules::default().full_match.their_batting;
        let mut rng = Rng::new(seed);
        half(
            made,
            winning,
            3,
            first_up,
            &rules,
            steals,
            &Ground::default(),
            &mut rng,
        )
    }

    /// Checks everything that has to be so of a half however it went.
    fn sound(half: &Half, made: u32, winning: bool, first_up: usize) {
        let runs_in: u32 = half.turns.iter().map(|turn| turn.runs_in).sum();
        assert_eq!(runs_in, made, "the runs that came in");
        assert_eq!(
            half.runs.iter().sum::<u32>(),
            made,
            "the runs each batter made"
        );
        let caught = half.steals.iter().filter(|steal| !steal.safe).count() as u32;
        let outs = half.turns.iter().map(|turn| turn.outs_made).sum::<u32>() + caught;
        if winning {
            assert!(outs < 3, "a match that was won before the side was out");
            assert!(half.turns.last().is_some_and(|turn| turn.runs_in > 0));
        } else {
            assert_eq!(outs, 3, "the outs");
        }
        // Everyone who came up is out, home or left on base.
        let reached = half.turns.len() as u32;
        assert_eq!(reached, outs + made + half.left, "where the batters went");
        let mut out_so_far = 0;
        let mut on_base = 0;
        for steal in &half.steals {
            assert!(steal.at <= half.turns.len() && steal.order < ORDER);
            assert!(steal.innings == 3 && (steal.base == 2 || steal.base == 3));
        }
        for (index, turn) in half.turns.iter().enumerate() {
            // A runner thrown out stealing before this turn is out, and
            // off the bases.
            for steal in half.steals.iter().filter(|steal| steal.at == index) {
                if !steal.safe {
                    out_so_far += 1;
                    on_base -= 1;
                }
            }
            // They come up in order, with the outs there have been.
            assert_eq!(turn.order, (first_up + index) % ORDER);
            assert_eq!(turn.outs, out_so_far);
            assert_eq!(turn.on.iter().filter(|on| **on).count() as u32, on_base);
            assert_eq!(turn.innings, 3);
            out_so_far += turn.outs_made;
            on_base = on_base + 1 - turn.outs_made - turn.runs_in;
            // The count is one that can be: the turn ends on its last
            // pitch, and not before.
            let (mut balls, mut strikes) = (0, 0);
            for (number, pitch) in turn.pitches.iter().enumerate() {
                assert!(balls < 4 && strikes < 3, "a pitch after the turn was over");
                let last = number + 1 == turn.pitches.len();
                match pitch.thrown {
                    Thrown::Ball => balls += 1,
                    Thrown::Called | Thrown::Swinging => strikes += 1,
                    Thrown::Foul => strikes = (strikes + 1).min(2),
                    Thrown::InPlay => assert!(last, "a ball in play that was not the last pitch"),
                }
                assert!(pitch.off.is_none() && pitch.quality.is_none());
            }
            let last = turn.pitches.last().expect("a turn has a pitch").thrown;
            match turn.end {
                End::Strikeout => assert_eq!(strikes, 3),
                End::Walk => assert_eq!(balls, 4),
                _ => assert_eq!(last, Thrown::InPlay),
            }
            assert_eq!(turn.ball.is_some(), turn.end.in_play());
            if let Some(ball) = turn.ball {
                assert!((0.0..=1.0).contains(&ball.across), "a fair ball");
                // A home run is over the wall, and nothing else is.
                assert_eq!(ball.feet > 400, turn.end == End::HomeRun, "{ball:?}");
            }
            match turn.end {
                End::DoublePlay => assert_eq!(turn.outs_made, 2),
                End::Strikeout | End::FlyOut | End::GroundOut | End::SacrificeFly => {
                    assert_eq!(turn.outs_made, 1)
                }
                _ => assert_eq!(turn.outs_made, 0),
            }
            if turn.end == End::SacrificeFly {
                assert!(turn.on[2] && turn.runs_in == 1 && turn.outs < 2);
            }
            if turn.end == End::DoublePlay {
                assert!(turn.on[0] && turn.outs < 2);
            }
        }
        assert_eq!(half.next, (first_up + half.turns.len()) % ORDER);
    }

    #[test]
    fn a_half_comes_to_the_runs_it_was_to_and_adds_up() {
        for seed in 0..400 {
            let made = (seed % 9) as u32;
            let first_up = (seed % 7) as usize;
            let half = played(made, false, first_up, seed);
            sound(&half, made, false, first_up);
        }
    }

    #[test]
    fn a_half_of_a_great_many_runs_still_adds_up() {
        for seed in 0..40 {
            let made = 10 + (seed % 14) as u32;
            let half = played(made, false, 0, seed);
            sound(&half, made, false, 0);
        }
    }

    #[test]
    fn a_half_that_wins_the_match_ends_on_the_run_that_wins_it() {
        for seed in 0..300 {
            let made = 1 + (seed % 5) as u32;
            let half = played(made, true, 4, seed);
            sound(&half, made, true, 4);
        }
    }

    #[test]
    fn with_runners_who_steal_a_half_still_adds_up() {
        // Runners who go every chance they get, and are out half the time.
        let keen = StealRules {
            their_chance: 1.0,
            their_third: 1.0,
            their_safe: 0.5,
            ..Rules::default().steal
        };
        let (mut stolen, mut caught, mut thirds) = (0, 0, 0);
        for seed in 0..400 {
            let made = (seed % 7) as u32;
            let winning = seed % 5 == 0 && made > 0;
            let half = played_by(made, winning, 2, seed, Some(&keen));
            sound(&half, made, winning, 2);
            stolen += half.steals.iter().filter(|steal| steal.safe).count();
            caught += half.steals.iter().filter(|steal| !steal.safe).count();
            thirds += half.steals.iter().filter(|steal| steal.base == 3).count();
        }
        assert!(stolen > 100 && caught > 100 && thirds > 20);
        // As the rules have them they go now and then, and mostly get
        // there.
        let usual = Rules::default().steal;
        let (mut tries, mut safe, mut turns) = (0, 0, 0);
        for seed in 0..600 {
            let made = [0, 0, 0, 1, 1, 2, 2, 3, 4, 6][(seed % 10) as usize];
            let half = played_by(made, false, 0, seed, Some(&usual));
            sound(&half, made, false, 0);
            tries += half.steals.len();
            safe += half.steals.iter().filter(|steal| steal.safe).count();
            turns += half.turns.len();
        }
        assert!(tries > 20 && tries * 20 < turns, "{tries} in {turns}");
        assert!(safe * 2 > tries, "{safe} of {tries}");
    }

    #[test]
    fn without_runners_who_steal_nobody_does() {
        for seed in 0..100 {
            assert!(played(3, false, 0, seed).steals.is_empty());
        }
    }

    #[test]
    fn the_same_numbers_play_the_same_half() {
        assert_eq!(played(3, false, 2, 11), played(3, false, 2, 11));
        assert_ne!(played(3, false, 2, 11), played(3, false, 2, 12));
    }

    #[test]
    fn over_many_innings_the_figures_are_ones_a_side_might_have() {
        // Innings of the runs the middle skill level gives, near enough.
        let mut turns = Vec::new();
        for seed in 0..600 {
            let made = [0, 0, 0, 1, 1, 2, 2, 3, 4, 6][(seed % 10) as usize];
            turns.extend(played(made, false, (seed % 9) as usize, seed).turns);
        }
        let figures = Figures::of(turns.iter());
        let between = |value: Option<f32>, low: f32, high: f32| {
            let value = value.expect("something to divide by");
            assert!(
                (low..=high).contains(&value),
                "{value} is not in {low}..{high}"
            );
        };
        // Two runs an innings is a great many, and takes a great many
        // hits: the average is high as the scores are.
        between(figures.average(), 0.36, 0.5);
        between(figures.strikeout_rate(), 0.12, 0.25);
        between(figures.walk_rate(), 0.05, 0.14);
        between(figures.strike_rate(), 0.57, 0.69);
        between(figures.contact_rate(), 0.7, 0.86);
        between(figures.chase_rate(), 0.2, 0.34);
        between(figures.pitches_a_turn(), 2.8, 4.0);
        assert!(figures.home_runs > 0 && figures.doubles > figures.triples);
    }

    #[test]
    fn an_innings_that_will_not_come_out_is_written_plainly() {
        let ground = Ground::default();
        let half = plainly(4, false, 3, 7, &ground);
        sound(&half, 4, false, 7);
        let winning = plainly(2, true, 3, 0, &ground);
        sound(&winning, 2, true, 0);
    }
}
