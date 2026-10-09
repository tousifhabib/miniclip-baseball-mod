//! The scorebook of a full match: every pitch to every batter of both
//! sides, and the figures that come of them.
//!
//! The player's own side is written up as it is played. The other side's
//! innings are played out on paper, and written up the same way, so that
//! everything said of either side is worked out from the same kind of
//! record by the same sums.

use super::field::Ground;
use super::pitch::{Point, Quality};

/// How many batters make up the order before it comes round again.
pub const ORDER: usize = 9;

/// How a pitch ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Thrown {
    Ball,
    /// A strike the batter let go by.
    Called,
    /// A strike he swung at and missed.
    Swinging,
    Foul,
    InPlay,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pitch {
    pub in_zone: bool,
    pub thrown: Thrown,
    /// How many frames after the best moment for it the swing began, or
    /// before it if less than nought. Only a batter who is really played
    /// has one.
    pub off: Option<i32>,
    /// How well the bat met the ball, if it did.
    pub quality: Option<Quality>,
}

impl Pitch {
    pub fn swung(&self) -> bool {
        matches!(
            self.thrown,
            Thrown::Swinging | Thrown::Foul | Thrown::InPlay
        )
    }

    pub fn strike(&self) -> bool {
        self.thrown != Thrown::Ball
    }

    /// Whether the bat met the ball.
    pub fn met(&self) -> bool {
        matches!(self.thrown, Thrown::Foul | Thrown::InPlay)
    }
}

/// How a batter's turn ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    Strikeout,
    Walk,
    Single,
    Double,
    Triple,
    HomeRun,
    FlyOut,
    GroundOut,
    /// A fly that was caught, on which a runner came home.
    SacrificeFly,
    /// A ground ball that put two out.
    DoublePlay,
    /// Safe because a fielder dropped a catch.
    Error,
}

impl End {
    /// How many bases a hit is worth. Nought if it was not a hit.
    pub fn bases(self) -> u32 {
        match self {
            End::Single => 1,
            End::Double => 2,
            End::Triple => 3,
            End::HomeRun => 4,
            _ => 0,
        }
    }

    pub fn hit(self) -> bool {
        self.bases() > 0
    }

    /// Whether the turn counts as an at-bat: all do but a walk and a
    /// sacrifice.
    pub fn at_bat(self) -> bool {
        !matches!(self, End::Walk | End::SacrificeFly)
    }

    /// Whether the ball was put in play.
    pub fn in_play(self) -> bool {
        !matches!(self, End::Strikeout | End::Walk)
    }
}

/// Where a ball that was put in play went.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// Where it first came down, was caught, or would have come down had
    /// the wall not been in the way, in the field's own pixels.
    pub at: Point,
    /// How far across the field that is: nought on the left foul line and
    /// one on the right.
    pub across: f32,
    pub feet: u32,
    /// It was in the air: a fly or a line drive, not a ground ball.
    pub fly: bool,
    /// It went beyond the infield.
    pub deep: bool,
}

impl Hit {
    /// A ball that came down at `at`. `feet` is how far it went if that is
    /// known better than by where it came down.
    pub fn at(ground: &Ground, at: Point, fly: bool, feet: Option<u32>) -> Hit {
        Hit {
            at,
            across: ground.across(at),
            feet: feet.unwrap_or_else(|| ground.feet(at)),
            fly,
            deep: super::field::reach(ground.home, at) >= ground.infield,
        }
    }

    /// The part of the field it went to, as a scorer would say it.
    pub fn place(&self) -> &'static str {
        let fifth = (self.across.clamp(0.0, 0.999) * 5.0) as usize;
        if self.deep {
            ["LEFT", "LEFT CENTRE", "CENTRE", "RIGHT CENTRE", "RIGHT"][fifth]
        } else {
            ["THIRD", "SHORT", "THE MOUND", "SECOND", "FIRST"][fifth]
        }
    }

    /// Which third of the field it went to: left, centre or right.
    pub fn third(&self) -> usize {
        (self.across.clamp(0.0, 0.999) * 3.0) as usize
    }
}

/// One batter's turn at the plate.
#[derive(Clone, Debug, PartialEq)]
pub struct Turn {
    /// The innings it was in, counting from 1.
    pub innings: u32,
    /// His place in the order, counting from nought.
    pub order: usize,
    /// How many were out when he came up, and which bases had runners on.
    pub outs: u32,
    pub on: [bool; 3],
    pub pitches: Vec<Pitch>,
    pub end: End,
    pub ball: Option<Hit>,
    /// The runs that came in on it.
    pub runs_in: u32,
    /// The outs made on it, his own and any runner's.
    pub outs_made: u32,
}

impl Turn {
    /// Whether a runner was on second or third when he came up.
    pub fn in_scoring_position(&self) -> bool {
        self.on[1] || self.on[2]
    }

    /// The turn in a line, for the list of an innings: who, what he did,
    /// what came of it, and how many pitches he saw.
    pub fn words(&self) -> String {
        let place = self.ball.map_or("", |ball| ball.place());
        let feet = self.ball.map_or(0, |ball| ball.feet);
        let looking = self.pitches.last().map(|pitch| pitch.thrown) == Some(Thrown::Called);
        let mut what = match self.end {
            End::Strikeout if looking => "STRUCK OUT LOOKING".to_owned(),
            End::Strikeout => "STRUCK OUT SWINGING".to_owned(),
            End::Walk => "WALKED".to_owned(),
            End::Single => format!("SINGLE TO {place}"),
            End::Double => format!("DOUBLE TO {place}"),
            End::Triple => format!("TRIPLE TO {place}"),
            End::HomeRun => format!("HOME RUN TO {place}, {feet} FT"),
            End::FlyOut => format!("FLEW OUT TO {place}"),
            End::GroundOut => format!("GROUNDED OUT TO {place}"),
            End::SacrificeFly => format!("SACRIFICE FLY TO {place}"),
            End::DoublePlay => format!("DOUBLE PLAY TO {place}"),
            End::Error => format!("SAFE ON AN ERROR AT {place}"),
        };
        // An out that was not the batter's own, or not his alone.
        let own = match self.end {
            End::DoublePlay => 2,
            End::Strikeout | End::FlyOut | End::GroundOut | End::SacrificeFly => 1,
            _ => 0,
        };
        if self.outs_made > own {
            what += ", RUNNER OUT";
        }
        let runs = match self.runs_in {
            0 => String::new(),
            1 => ", 1 RUN".to_owned(),
            runs => format!(", {runs} RUNS"),
        };
        format!("{} {what}{runs} ({})", self.order + 1, self.pitches.len())
    }
}

/// A runner's try at stealing a base.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Steal {
    /// The innings it was in, counting from 1.
    pub innings: u32,
    /// The runner's place in the order, counting from nought.
    pub order: usize,
    /// The base he went for: 2 for second, 3 for third.
    pub base: u8,
    /// Whether he got there. If not he was thrown out.
    pub safe: bool,
    /// How many of his side's turns were over when he went, which is where
    /// among them it is told.
    pub at: usize,
}

impl Steal {
    /// The try in a line, for the list of an innings.
    pub fn words(&self) -> String {
        let base = match self.base {
            2 => "SECOND",
            3 => "THIRD",
            _ => "HOME",
        };
        let what = if self.safe {
            "STOLE"
        } else {
            "CAUGHT STEALING"
        };
        format!("{} {what} {base}", self.order + 1)
    }
}

/// A turn that is still going on.
#[derive(Clone, Debug, PartialEq)]
struct Open {
    innings: u32,
    order: usize,
    outs: u32,
    on: [bool; 3],
    pitches: Vec<Pitch>,
}

/// One side's part of the book.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Side {
    pub turns: Vec<Turn>,
    open: Option<Open>,
    /// The runs made by each place in the order.
    pub runs: [u32; ORDER],
    /// How many runners were left on base at the end of each half.
    pub left: Vec<u32>,
    /// The errors this side made in the field.
    pub errors: u32,
    /// Its runners' tries at stealing a base, in the order they were made.
    pub steals: Vec<Steal>,
}

impl Side {
    /// A batter comes up, unless one is up already.
    pub fn come_up(&mut self, innings: u32, order: usize, outs: u32, on: [bool; 3]) {
        if self.open.is_none() {
            self.open = Some(Open {
                innings,
                order,
                outs,
                on,
                pitches: Vec::new(),
            });
        }
    }

    /// A pitch to the batter who is up.
    pub fn pitch(&mut self, pitch: Pitch) {
        if let Some(open) = &mut self.open {
            open.pitches.push(pitch);
        }
    }

    /// The turn of the batter who is up is over.
    pub fn close(&mut self, end: End, ball: Option<Hit>, runs_in: u32, outs_made: u32) {
        if let Some(open) = self.open.take() {
            self.turns.push(Turn {
                innings: open.innings,
                order: open.order,
                outs: open.outs,
                on: open.on,
                pitches: open.pitches,
                end,
                ball,
                runs_in,
                outs_made,
            });
        }
    }

    /// A turn that was begun is not to be finished: the side is out, or
    /// the match is over.
    pub fn abandon(&mut self) {
        self.open = None;
    }

    /// The turns of one innings.
    pub fn innings(&self, innings: u32) -> impl Iterator<Item = &Turn> {
        self.turns
            .iter()
            .filter(move |turn| turn.innings == innings)
    }

    /// The hits made in one innings.
    pub fn hits_in(&self, innings: u32) -> u32 {
        self.innings(innings).filter(|turn| turn.end.hit()).count() as u32
    }

    /// A runner tried to steal a base, in the innings the side is batting
    /// in.
    pub fn stole(&mut self, innings: u32, order: usize, base: u8, safe: bool) {
        self.steals.push(Steal {
            innings,
            order,
            base,
            safe,
            at: self.turns.len(),
        });
    }

    /// How many of the side have been put out: at the plate, in the field
    /// and stealing.
    pub fn outs(&self) -> u32 {
        let batting: u32 = self.turns.iter().map(|turn| turn.outs_made).sum();
        batting + self.steals.iter().filter(|steal| !steal.safe).count() as u32
    }

    /// What happened in one innings, in order, a line for each batter's
    /// turn and each try at stealing a base, with whether it brought a run
    /// in.
    pub fn told(&self, innings: u32) -> Vec<(String, bool)> {
        let mut steals = self
            .steals
            .iter()
            .filter(|steal| steal.innings == innings)
            .peekable();
        let mut told = Vec::new();
        for (index, turn) in self.turns.iter().enumerate() {
            if turn.innings != innings {
                continue;
            }
            // A base stolen while he was up is told before he is.
            while let Some(steal) = steals.next_if(|steal| steal.at <= index) {
                told.push((steal.words(), false));
            }
            told.push((turn.words(), turn.runs_in > 0));
        }
        told.extend(steals.map(|steal| (steal.words(), false)));
        told
    }

    /// The figures of the whole side.
    pub fn figures(&self) -> Figures {
        let mut figures = Figures::of(self.turns.iter());
        figures.runs = self.runs.iter().sum();
        figures.left = self.left.iter().sum();
        figures.steal(self.steals.iter());
        figures
    }

    /// The figures of one place in the order.
    pub fn figures_of(&self, order: usize) -> Figures {
        let mut figures = Figures::of(self.turns.iter().filter(|turn| turn.order == order));
        figures.runs = self.runs.get(order).copied().unwrap_or(0);
        figures.steal(self.steals.iter().filter(|steal| steal.order == order));
        figures
    }
}

/// The book of both sides.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Book {
    pub ours: Side,
    pub theirs: Side,
}

/// What a batter's or a side's turns add up to.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
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
    fn steal<'a>(&mut self, steals: impl Iterator<Item = &'a Steal>) {
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

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn pitch(thrown: Thrown, in_zone: bool) -> Pitch {
        Pitch {
            in_zone,
            thrown,
            off: None,
            quality: None,
        }
    }

    #[expect(clippy::unnecessary_wraps, reason = "it is handed straight to `close`")]
    fn ball(across: f32, far: f32, fly: bool) -> Option<Hit> {
        let ground = Ground::default();
        Some(Hit::at(&ground, ground.point(across, far), fly, None))
    }

    /// A side that has had six turns: a strikeout looking, a walk, a
    /// double with the walk aboard, a home run, a sacrifice fly and a
    /// ground ball for two outs.
    fn side() -> Side {
        let mut side = Side::default();
        side.come_up(1, 0, 0, [false; 3]);
        side.pitch(pitch(Thrown::Swinging, true));
        side.pitch(pitch(Thrown::Foul, false));
        side.pitch(pitch(Thrown::Called, true));
        side.close(End::Strikeout, None, 0, 1);
        side.come_up(1, 1, 1, [false; 3]);
        for _ in 0..4 {
            side.pitch(pitch(Thrown::Ball, false));
        }
        side.close(End::Walk, None, 0, 0);
        side.come_up(1, 2, 1, [true, false, false]);
        side.pitch(pitch(Thrown::InPlay, true));
        side.close(End::Double, ball(0.1, 700.0, true), 0, 0);
        side.come_up(1, 3, 1, [false, true, true]);
        side.pitch(pitch(Thrown::Ball, false));
        side.pitch(pitch(Thrown::InPlay, true));
        side.close(End::HomeRun, ball(0.5, 900.0, true), 3, 0);
        side.come_up(1, 4, 1, [false, false, true]);
        side.pitch(pitch(Thrown::InPlay, false));
        side.close(End::SacrificeFly, ball(0.9, 650.0, true), 1, 1);
        side.come_up(1, 5, 2, [true, false, false]);
        side.pitch(pitch(Thrown::InPlay, true));
        side.close(End::GroundOut, ball(0.3, 300.0, false), 0, 1);
        side.runs = [0, 1, 1, 1, 0, 0, 0, 0, 0];
        side.left = vec![1];
        side
    }

    #[test]
    fn a_sides_turns_add_up_as_a_scorer_would_add_them() {
        let figures = side().figures();
        assert_eq!(figures.turns, 6);
        // A walk and a sacrifice are not at-bats.
        assert_eq!(figures.at_bats, 4);
        assert_eq!(
            (figures.hits, figures.doubles, figures.home_runs),
            (2, 1, 1)
        );
        assert_eq!(figures.total_bases, 6);
        assert_eq!((figures.runs, figures.runs_in, figures.left), (3, 4, 1));
        assert_eq!(
            (figures.walks, figures.strikeouts, figures.sacrifices),
            (1, 1, 1)
        );
        assert_eq!(average(figures.average()), ".500");
        // Two hits and a walk in four at-bats, a walk and a sacrifice.
        assert_eq!(average(figures.on_base()), ".500");
        assert_eq!(average(figures.slugging()), "1.500");
        assert_eq!(average(figures.on_base_plus_slugging()), "2.000");
        // The double is the one hit that stayed in the park, of three
        // balls that did.
        assert_eq!(average(figures.in_play_average()), ".333");
        // With a runner on second or third: the home run, in one at-bat.
        assert_eq!((figures.chances, figures.chances_taken), (1, 1));
        assert_eq!(figures.two_out_runs, 0);
    }

    #[test]
    fn the_pitches_are_counted_by_what_came_of_them() {
        let figures = side().figures();
        assert_eq!(figures.pitches, 12);
        // Five balls, and seven strikes of one kind or another.
        assert_eq!(figures.strikes, 7);
        assert_eq!(percent(figures.strike_rate()), "58%");
        assert_eq!((figures.called, figures.swinging, figures.fouls), (1, 1, 1));
        // Six swings, of which one missed.
        assert_eq!(figures.swings, 6);
        assert_eq!(percent(figures.contact_rate()), "83%");
        assert_eq!(percent(figures.miss_rate()), "17%");
        // Seven pitches outside the zone, and two of them swung at.
        assert_eq!((figures.outside, figures.chases), (7, 2));
        assert_eq!(percent(figures.chase_rate()), "29%");
        assert_eq!(tenths(figures.pitches_a_turn()), "2.0");
        assert_eq!(percent(figures.strikeout_rate()), "17%");
    }

    #[test]
    fn balls_in_play_are_counted_by_where_they_went() {
        let figures = side().figures();
        assert_eq!(
            (figures.in_play, figures.flies, figures.grounders),
            (4, 3, 1)
        );
        assert_eq!(figures.thirds, [2, 1, 1]);
        assert_eq!(figures.longest, 439);
        assert_eq!(figures.feet, 341 + 439 + 317 + 146);
    }

    #[test]
    fn one_batters_figures_are_his_own() {
        let side = side();
        let fourth = side.figures_of(3);
        assert_eq!((fourth.at_bats, fourth.hits, fourth.home_runs), (1, 1, 1));
        assert_eq!((fourth.runs, fourth.runs_in), (1, 3));
        assert_eq!(average(fourth.average()), "1.000");
        // A batter who has not been up has no average.
        assert_eq!(average(side.figures_of(8).average()), "---");
        assert_eq!(percent(side.figures_of(8).strike_rate()), "-");
        assert_eq!(side.hits_in(1), 2);
        assert_eq!(side.hits_in(2), 0);
    }

    #[test]
    fn a_base_stolen_is_the_runners_and_is_told_where_it_happened() {
        let mut side = Side::default();
        side.come_up(1, 0, 0, [false; 3]);
        for _ in 0..4 {
            side.pitch(pitch(Thrown::Ball, false));
        }
        side.close(End::Walk, None, 0, 0);
        // He steals second while the next man is up, who strikes out.
        side.come_up(1, 1, 0, [true, false, false]);
        side.pitch(pitch(Thrown::Called, true));
        side.stole(1, 0, 2, true);
        side.pitch(pitch(Thrown::Swinging, true));
        side.pitch(pitch(Thrown::Swinging, true));
        side.close(End::Strikeout, None, 0, 1);
        // And is thrown out going for third before the one after grounds
        // out.
        side.come_up(1, 2, 1, [false, true, false]);
        side.stole(1, 0, 3, false);
        side.pitch(pitch(Thrown::InPlay, true));
        side.close(End::GroundOut, ball(0.3, 300.0, false), 0, 1);
        let figures = side.figures();
        assert_eq!((figures.stolen, figures.caught), (1, 1));
        assert_eq!(figures.stolen_of(), "1 OF 2");
        assert_eq!(side.figures_of(0).stolen_of(), "1 OF 2");
        assert_eq!(side.figures_of(1).stolen_of(), "0 OF 0");
        // Three are out, one of them on the bases.
        assert_eq!(side.outs(), 3);
        let told: Vec<String> = side.told(1).into_iter().map(|(line, _)| line).collect();
        assert_eq!(
            told,
            [
                "1 WALKED (4)",
                "1 STOLE SECOND",
                "2 STRUCK OUT SWINGING (3)",
                "1 CAUGHT STEALING THIRD",
                "3 GROUNDED OUT TO SHORT (1)",
            ]
        );
        assert!(side.told(2).is_empty());
    }

    #[test]
    fn a_turn_is_told_in_a_line() {
        let side = side();
        let lines: Vec<String> = side.turns.iter().map(Turn::words).collect();
        assert_eq!(
            lines,
            [
                "1 STRUCK OUT LOOKING (3)",
                "2 WALKED (4)",
                "3 DOUBLE TO LEFT (1)",
                "4 HOME RUN TO CENTRE, 439 FT, 3 RUNS (2)",
                "5 SACRIFICE FLY TO RIGHT, 1 RUN (1)",
                "6 GROUNDED OUT TO SHORT (1)",
            ]
        );
    }

    #[test]
    fn a_turn_that_is_not_begun_takes_no_pitches_and_one_begun_can_be_dropped() {
        let mut side = Side::default();
        side.pitch(pitch(Thrown::Ball, false));
        side.close(End::Walk, None, 0, 0);
        assert!(side.turns.is_empty());
        side.come_up(2, 0, 0, [false; 3]);
        side.pitch(pitch(Thrown::Ball, false));
        side.abandon();
        side.close(End::Walk, None, 0, 0);
        assert!(side.turns.is_empty());
    }

    /// A turn at the plate such as no game need ever have had: its pitches,
    /// how it ended and where the ball went go together no better than
    /// chance has them. The sums are to come out all the same.
    fn any_turn() -> impl Strategy<Value = Turn> {
        let thrown = prop::sample::select(
            &[
                Thrown::Ball,
                Thrown::Called,
                Thrown::Swinging,
                Thrown::Foul,
                Thrown::InPlay,
            ][..],
        );
        let end = prop::sample::select(
            &[
                End::Strikeout,
                End::Walk,
                End::Single,
                End::Double,
                End::Triple,
                End::HomeRun,
                End::FlyOut,
                End::GroundOut,
                End::SacrificeFly,
                End::DoublePlay,
                End::Error,
            ][..],
        );
        let came_up = (1u32..=9, 0..ORDER, 0u32..3, any::<[bool; 3]>());
        let pitches = prop::collection::vec((thrown, any::<bool>()), 0..8);
        let hit = prop::option::of((0.0f32..=1.0, 100.0f32..1000.0, any::<bool>()));
        (came_up, pitches, end, hit, 0u32..=4, 0u32..=2).prop_map(
            |((innings, order, outs, on), pitches, end, hit, runs_in, outs_made)| {
                // Written up as a turn is: he comes up, sees his pitches
                // and is done.
                let mut side = Side::default();
                side.come_up(innings, order, outs, on);
                for (thrown, in_zone) in pitches {
                    side.pitch(pitch(thrown, in_zone));
                }
                let hit = hit.and_then(|(across, far, fly)| ball(across, far, fly));
                side.close(end, hit, runs_in, outs_made);
                side.turns.remove(0)
            },
        )
    }

    fn any_turns() -> impl Strategy<Value = Vec<Turn>> {
        prop::collection::vec(any_turn(), 0..12)
    }

    /// One side's part of a book, made up the same way: its turns, the
    /// tries at a base its runners made between them, the runs each batter
    /// made and the runners it left on.
    fn any_side() -> impl Strategy<Value = Side> {
        let steal = prop::option::of((0..ORDER, 2u8..=3, any::<bool>()));
        let turns = prop::collection::vec((steal, any_turn()), 0..12);
        let runs = prop::array::uniform9(0u32..5);
        let left = prop::collection::vec(0u32..=3, 0..9);
        (turns, runs, left).prop_map(|(turns, runs, left)| {
            let mut side = Side {
                runs,
                left,
                ..Side::default()
            };
            for (steal, turn) in turns {
                if let Some((order, base, safe)) = steal {
                    side.stole(turn.innings, order, base, safe);
                }
                side.turns.push(turn);
            }
            side
        })
    }

    /// Every count there is in a set of figures. The longest ball is not
    /// one: it is the longest, not how many.
    fn counts(figures: &Figures) -> Vec<u32> {
        // Taken apart by name, so that a count added to the figures has to
        // be added here before this will build.
        let Figures {
            turns,
            at_bats,
            runs,
            hits,
            singles,
            doubles,
            triples,
            home_runs,
            total_bases,
            runs_in,
            walks,
            strikeouts,
            sacrifices,
            double_plays,
            left,
            chances,
            chances_taken,
            two_out_runs,
            pitches,
            strikes,
            called,
            swinging,
            fouls,
            swings,
            outside,
            chases,
            in_play,
            flies,
            grounders,
            thirds: [to_left, to_centre, to_right],
            feet,
            longest: _,
            stolen,
            caught,
        } = *figures;
        vec![
            turns,
            at_bats,
            runs,
            hits,
            singles,
            doubles,
            triples,
            home_runs,
            total_bases,
            runs_in,
            walks,
            strikeouts,
            sacrifices,
            double_plays,
            left,
            chances,
            chances_taken,
            two_out_runs,
            pitches,
            strikes,
            called,
            swinging,
            fouls,
            swings,
            outside,
            chases,
            in_play,
            flies,
            grounders,
            to_left,
            to_centre,
            to_right,
            feet,
            stolen,
            caught,
        ]
    }

    proptest! {
        #[test]
        fn the_figures_of_two_lists_of_turns_add_up_to_the_figures_of_both_together(
            first in any_turns(),
            second in any_turns(),
        ) {
            let (one, other) = (Figures::of(first.iter()), Figures::of(second.iter()));
            let both = Figures::of(first.iter().chain(&second));
            let added: Vec<u32> = counts(&one)
                .iter()
                .zip(counts(&other))
                .map(|(one, other)| one + other)
                .collect();
            prop_assert_eq!(counts(&both), added);
            // The longest ball of them all is the longer of the two longest.
            prop_assert_eq!(both.longest, one.longest.max(other.longest));
        }

        #[test]
        fn the_figures_of_any_turns_agree_with_one_another(turns in any_turns()) {
            let figures = Figures::of(turns.iter());
            // Every turn is an at-bat, a walk or a sacrifice.
            let Figures { at_bats, walks, sacrifices, .. } = figures;
            prop_assert_eq!(figures.turns, at_bats + walks + sacrifices);
            // A hit is a single, a double, a triple or a home run, and is
            // worth that many bases.
            let Figures { singles, doubles, triples, home_runs, .. } = figures;
            prop_assert_eq!(figures.hits, singles + doubles + triples + home_runs);
            let bases = singles + 2 * doubles + 3 * triples + 4 * home_runs;
            prop_assert_eq!(figures.total_bases, bases);
            // A strike was let go by or was swung at.
            prop_assert_eq!(figures.strikes, figures.called + figures.swings);
            // A ball in play went in the air or along the ground, and to
            // one third of the field or another.
            prop_assert_eq!(figures.in_play, figures.flies + figures.grounders);
            prop_assert_eq!(figures.in_play, figures.thirds.iter().sum::<u32>());
            // And no part is more than what it is a part of.
            let parts = [
                (figures.hits, at_bats),
                (figures.chances_taken, figures.chances),
                (figures.chances, at_bats),
                (figures.two_out_runs, figures.runs_in),
                (figures.swinging + figures.fouls, figures.swings),
                (figures.chases, figures.outside),
                (figures.outside, figures.pitches),
                (figures.longest, figures.feet),
            ];
            for (number, (part, whole)) in parts.into_iter().enumerate() {
                prop_assert!(part <= whole, "{} of {}, in pair {}", part, whole, number);
            }
        }

        #[test]
        fn every_average_is_there_when_there_is_something_to_divide_by_and_is_one_that_can_be(
            turns in any_turns(),
        ) {
            let figures = Figures::of(turns.iter());
            // Each average with what it is so many for each of, and the
            // most it can come to: a base at a time, or four.
            let averages = [
                (figures.average(), figures.at_bats, 1.0),
                (figures.on_base(), figures.turns, 1.0),
                (figures.slugging(), figures.at_bats, 4.0),
                (figures.chance_average(), figures.chances, 1.0),
                (figures.strikeout_rate(), figures.turns, 1.0),
                (figures.walk_rate(), figures.turns, 1.0),
                (figures.strike_rate(), figures.pitches, 1.0),
                (figures.swing_rate(), figures.pitches, 1.0),
                (figures.contact_rate(), figures.swings, 1.0),
                (figures.miss_rate(), figures.swings, 1.0),
                (figures.chase_rate(), figures.outside, 1.0),
            ];
            for (number, (average, of, most)) in averages.into_iter().enumerate() {
                prop_assert_eq!(average.is_some(), of > 0, "average {}", number);
                let within = average.is_none_or(|average| (0.0..=most).contains(&average));
                prop_assert!(within, "average {} is {:?}", number, average);
            }
            // Hits for each ball that stayed in the park are no more than
            // one for one, whatever there is to divide by.
            let in_play = figures.in_play_average();
            prop_assert!(in_play.is_none_or(|average| (0.0..=1.0).contains(&average)));
            // A swing met the ball or missed it.
            if let (Some(met), Some(missed)) = (figures.contact_rate(), figures.miss_rate()) {
                prop_assert!((met + missed - 1.0).abs() < 1e-6, "{} and {}", met, missed);
            }
            // No ball went further than the longest, so nor did they on
            // the whole.
            let usual = figures.usual_feet();
            prop_assert!(usual.is_none_or(|feet| feet <= figures.longest as f32));
        }

        #[test]
        fn what_the_nine_batters_did_adds_up_to_what_their_side_did(side in any_side()) {
            let whole = side.figures();
            let each: Vec<Figures> = (0..ORDER).map(|order| side.figures_of(order)).collect();
            let mut added = vec![0; counts(&whole).len()];
            for figures in &each {
                for (sum, count) in added.iter_mut().zip(counts(figures)) {
                    *sum += count;
                }
            }
            // But for the runners left on base, who are the side's and no
            // one batter's.
            let but_left = Figures { left: 0, ..whole };
            prop_assert_eq!(added, counts(&but_left));
            let longest = each.iter().map(|figures| figures.longest).max();
            prop_assert_eq!(longest, Some(whole.longest));
        }
    }
}
