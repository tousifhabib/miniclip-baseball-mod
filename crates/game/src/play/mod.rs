//! A match in progress: the last innings, batting to overtake the other
//! side, or every innings of a full match.
//!
//! The art builds the batting view afresh for every pitch, so everything
//! that lasts from one pitch to the next is kept here: the score, the count,
//! the outs, and where every runner stands.

mod arcade;
pub mod book;
pub mod bullet;
mod called;
pub mod field;
mod fielding;
pub mod full;
pub mod night;
pub(crate) mod overlay;
pub mod paper;
mod pinball;
pub mod pitch;
pub mod shift;
pub mod sign;
pub mod southpaw;
mod steal;
pub mod timing;
pub mod zinger;

use bb_engine::display::{ButtonEvent, Content, Event, Path, child_bounds};
use bb_engine::library::Library;
use bb_engine::math::{ColorTransform, Matrix};
use bb_engine::stage::Stage;
use bb_format::SymbolId;

use crate::art;
use crate::look::{self, Look, Rgb};
use crate::menu::Game;
use crate::mods::Mod;
use crate::rng::Rng;
use crate::rules::{FieldRules, HitRules, PitchRules};
use book::{End, ORDER, Thrown};
use field::{Ball, Contact, Ground, Happened, reach};
use overlay::Notice;
use pitch::{Choice, Kind, Mound, Pitch, Point, Quality};
use zinger::Zinger;

/// How a match ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Won,
    Lost,
    Tied,
    /// The arcade game's pitches are used up.
    ArcadeOver,
    /// In a full match, the player's side is out and the other side has
    /// batted: there is a board to read, and then more to play.
    Interval,
}

/// Where a batter has got to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Place {
    /// At the plate, batting.
    AtBat,
    /// Standing on first, second or third.
    Base(u8),
    Out,
    /// Round all the bases: a run.
    Home,
}

#[derive(Clone, Debug)]
pub(crate) struct Runner {
    pub place: Place,
    /// The base he is running to now, home being 4.
    pub running_to: Option<u8>,
    pub sliding: bool,
    pub runs: u32,
    /// His place in the batting order, counting from nought. A full match
    /// has nine, who come round again. Otherwise every batter is new.
    pub order: usize,
    /// The base he left to steal the next, on the pitch in hand.
    pub stole_from: Option<u8>,
    pub skin: Option<Rgb>,
    pub logo: Option<String>,
    /// His clip on the field, for as long as this pitch's view lasts.
    pub path: Option<Path>,
}

/// What stage a pitch has reached.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Phase {
    /// The batting view is being put up.
    Arriving,
    /// The pitcher stands and waits.
    Settling {
        left: u32,
    },
    WindUp,
    /// The ball is on its way.
    Flight {
        step: usize,
    },
    /// A strike or a ball has been called, and is being shown.
    Called {
        left: u32,
    },
    /// The ball is seen leaving the bat.
    Watching {
        left: u32,
    },
    /// Four balls: a moment, then the batter walks.
    Walking {
        left: u32,
    },
    /// The overhead view: the ball, the fielders and the runners.
    Fielding,
    /// The play is over and the next-ball button is up.
    Ready,
    /// The button has been pressed and the view is about to be rebuilt.
    Leaving {
        left: u32,
    },
    Over,
}

/// Something to do to a clip when it reaches a frame, where the art has
/// nothing to do it.
#[derive(Clone, Debug)]
pub(crate) struct Cue {
    pub path: Path,
    pub frame: u16,
    /// Go back to the first frame and wait there. Otherwise just stop.
    pub rewind: bool,
}

/// Where the parts of the view are, found afresh for each pitch.
#[derive(Clone, Debug)]
pub(crate) struct Parts {
    pub main: Path,
    pub pitcher: Path,
    pub hitter: Path,
    pub aim: Path,
    pub aim_shadow: Path,
    pub marker: Path,
    pub ball: Path,
    pub shadow: Path,
    pub fly: Path,
    pub fly_ball: Path,
    pub fly_shadow: Option<Path>,
    pub aim_area: Path,
    pub field: Path,
    pub scoreboard: Option<Path>,
    pub strike_anim: Option<Path>,
    pub transitions: Path,
    pub next: Path,
    pub flare: Option<Path>,
    pub field_ball: Path,
    pub field_ball_inner: Path,
    pub holder: Option<Path>,
    pub fielders: Vec<Path>,
    pub umpires: Vec<Path>,
    pub field_scoreboard: Option<Path>,
    /// Fixed points of the batting view.
    pub centre_x: f32,
    pub fly_mark: Point,
    pub ground_y: f32,
    /// The box the aiming ring is kept inside: left, top, right, bottom.
    pub aim_box: [f32; 4],
    /// Fixed points of the field.
    pub home: Point,
    pub field_mark: Point,
    pub foul: (f32, f32),
    pub bases: [Point; 4],
}

/// The pitch being played.
pub(crate) struct AtBat {
    pub parts: Parts,
    pub table: PitchRules,
    pub pitch: Pitch,
    pub marker_shown: bool,
    /// Where the marker shows the pitch crossing, which the knuckleball
    /// mod makes only roughly right.
    pub marker_at: Point,
    /// What the pitch is, with the mystery pitch mod on.
    pub kind: Option<Kind>,
    /// The pitch is a golden ball.
    pub golden: bool,
    /// Where the batter has said his hit will come down, with the called
    /// shot mod on.
    pub called: Option<called::Called>,
    pub aim: Point,
    /// Where the hit would go sideways, as the art's indicator shows it.
    pub aim_area_x: f32,
    /// Frames since the swing began.
    pub swing: Option<u32>,
    /// How far below the ball the ring was when the swing began, and how
    /// far to the right of it.
    pub under: f32,
    pub across: f32,
    pub contact: Option<Contact>,
    /// The ball leaving the bat, in the batting view: where it is, how high,
    /// and how fast it is rising.
    pub fly: (Point, f32, f32),
    /// The size the ball had grown to when the bat met it.
    pub fly_size: f32,
    pub fly_target: Point,
    /// Frames until the batter drops his bat and runs.
    pub run_in: Option<u32>,
    pub ball: Option<Ball>,
    pub fielding: Option<fielding::Fielding>,
    /// The timing bar, while that mod is on and the batting view is up.
    pub timing: Option<timing::Indicator>,
    /// The hit, if that mod made a zinger of it, and what the player is
    /// shown of it over the field.
    pub zinger: Option<Zinger>,
    pub zinger_show: Option<zinger::Show>,
    /// The ball went over the wall while it was still being watched leaving
    /// the bat, which the view of the field has yet to be told.
    pub over_wall: bool,
    /// What the mods have written up in the view.
    pub notices: Vec<overlay::Notice>,
    /// Where the hit first came down, if it has and the called shot mod
    /// wants to know.
    pub came_down: Option<Point>,
    /// How many times the wall or a foul line has sent the ball back, in
    /// a pinball park.
    pub rebounds: u32,
    /// In a full match, the word on each scoreboard over the other side's
    /// score.
    pub them: Vec<full::Them>,
    /// For a full match's book: how many frames after the best moment for
    /// it the swing began, and how well the bat met the ball.
    pub swing_off: Option<i32>,
    pub met: Option<Quality>,
    /// The runners' marks on the little field, with the stolen bases mod
    /// on and anyone on base.
    pub leads: Option<steal::Leads>,
    /// The signs on the wall, with the hit the sign mod on.
    pub signs: Option<sign::Board>,
    /// The meter in the corner of the view, with the bullet time mod on.
    pub meter: Option<bullet::Meter>,
}

impl Parts {
    /// How far across the batting view a place this far across the field
    /// is: where the art's pointer stands for a ball that comes down there.
    pub(crate) fn across_view(&self, across: f32, rules: &FieldRules) -> f32 {
        let mark = self.foul.0 + across * (self.foul.1 - self.foul.0);
        self.centre_x + (mark - self.field_mark.0) * rules.aim_share
    }

    /// The fixed points of the field that a hit is placed by.
    pub(crate) fn ground(&self, rules: &FieldRules) -> Ground {
        Ground {
            home: self.home,
            mark_y: self.field_mark.1,
            foul: self.foul,
            wall: rules.wall,
            infield: reach(self.home, self.bases[1]),
            ..Ground::default()
        }
    }
}

impl AtBat {
    /// The view is changing to the field, where the timing bar has no
    /// place.
    pub(crate) fn leave_batting_view(&mut self, stage: &mut Stage) {
        if let Some(bar) = self.timing.take() {
            bar.put_away(stage);
        }
    }
}

pub struct Match {
    pub(crate) score: u32,
    pub(crate) target: u32,
    pub(crate) outs: u32,
    pub(crate) max_outs: u32,
    pub(crate) strikes: u32,
    pub(crate) balls: u32,
    pub(crate) pitched: u32,
    pub(crate) runners: Vec<Runner>,
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
    /// The arcade game's own state, when that is what is being played.
    pub(crate) arcade: Option<arcade::Arcade>,
    /// The match all of whose innings are being played, when that is the
    /// game.
    pub(crate) full: Option<full::FullMatch>,
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
    /// How many times a fielder has let the ball go in this game, with the
    /// butterfingers mod on.
    pub(crate) slips: u32,
    /// With the hot bat mod on: how many swings in a row have met the
    /// ball.
    pub(crate) streak: u32,
    /// How many a run counts for on the pitch being played: one, unless a
    /// mod says more.
    pub(crate) run_worth: u32,
    /// A home run has just been hit, which the night game mod has yet to
    /// flash the lights for, and the frames of a flash still to come.
    pub(crate) lights: bool,
    flash: u32,
    /// With the turbo runners mod on: the part of a frame that runners are
    /// owed, on top of the whole frames they have been hurried on by.
    pub(crate) hurry: f32,
    /// With the heat check mod on: how many runs' worth faster the pitches
    /// are coming, and the score when that was last worked out.
    pub(crate) heat: u32,
    heat_score: u32,
    /// With the bullet time mod on: how many frames of holding the ball
    /// back are left in the meter, how many frames it has been held back
    /// for, whether it is being held back now, and a click made on a frame
    /// it was held back on, which the next frame that moves it takes.
    pub(crate) bullet: Option<u32>,
    slow_beat: u32,
    pub(crate) slowed: bool,
    late_press: Option<Point>,
    /// How many batters in a row have reached base, with nobody put out
    /// since, and whether the rally mod is on to go by it.
    pub(crate) rally: u32,
    rallying: bool,
    /// The pitch in hand is one the clutch mod makes runs count for more
    /// on.
    clutch: bool,
    /// The batter bats left-handed, by the southpaw mod.
    southpaw: bool,
    /// With the hit the sign mod on: the innings a sign was last lit for
    /// and which it was, what the next is drawn by, the sign a ball has
    /// just struck and the runs that was worth, until that has been told,
    /// and the same for the pitch in hand once it has.
    sign: Option<(u32, usize)>,
    sign_rng: Rng,
    sign_news: Option<(usize, u32)>,
    sign_struck: Option<(usize, u32)>,
    /// With the stolen bases mod on: how many bases have been stolen in
    /// this game and how many runners caught at it, whether the play in the
    /// field is one on which a base can be stolen, and how the last try
    /// came out, until that has been told.
    pub(crate) stolen: u32,
    pub(crate) caught: u32,
    pub(crate) steal_play: bool,
    steal_news: Option<(&'static str, Rgb)>,
    /// How many pitches the pitcher on the mound has thrown, and how many
    /// pitchers have come in for the one before, which the tired arm mod
    /// goes by.
    pub(crate) arm: u32,
    pub(crate) relieved: u32,
    /// How tired he is for the pitch in hand, from 0 to 1, with that mod
    /// on.
    pub(crate) tired: Option<f32>,
    /// How far across the field each fair ball of this game came down, from
    /// 0 on the left foul line to 1 on the right, the latest last. The shift
    /// mod has the fielders stand by it, and this is how far it has moved
    /// their middle for the pitch in hand: to the left if less than nought.
    pub(crate) spray: Vec<f32>,
    pub(crate) shift: f32,
    /// The longest zinger of this game, in feet, and the longest there has
    /// ever been.
    pub(crate) longest_zinger: u32,
    pub(crate) zinger_record: u32,
    runner_symbol: Option<SymbolId>,
}

/// The pitcher's frame label for his wind-up.
const PITCH: &str = "pitch";
/// Where a full match and the mods write in the corner of the batting view,
/// under the little field: the middle of the top of the first line, and how
/// far under each line the next one is. A full match says which half of
/// which innings it is, and each mod with something to say says it under
/// that.
const CORNER_AT: Point = (60.0, 88.0);
const CORNER_ROW: f32 = 16.0;
/// What turns the white of the ball to gold, for the golden ball mod.
const GOLD: ColorTransform = ColorTransform {
    mult: [1.0, 0.8, 0.22, 1.0],
    add: [0.0, 0.0, 0.0, 0.0],
};
/// How far down the batting view the mystery pitch mod names the pitch,
/// which is between the scoreboard and the pitcher. The tired arm mod says
/// there that a new pitcher has come in.
const MYSTERY_TOP: f32 = 141.0;
/// How much of the green and the blue of a pitcher goes when he is spent,
/// with the tired arm mod on: he is flushed.
const FLUSH: f32 = 0.22;
/// What makes the choice of the lit sign, with the hit the sign mod on,
/// come out differently from the pitches, which are drawn from the seed
/// itself: the same pitches come whether the mod is on or not.
const SIGN_SEED: u64 = 0xbb67_ae85_84ca_a73b;
/// The button on the next-ball panel.
const NEXT_BALL_BUTTON: SymbolId = 1618;

pub(crate) fn at(stage: &Stage, path: &[u16]) -> Point {
    stage
        .child(path)
        .map_or((0.0, 0.0), |child| (child.matrix.tx, child.matrix.ty))
}

/// Puts an object at a point, at a size, the art's own size being 1.
pub(crate) fn put(stage: &mut Stage, path: &[u16], at: Point, size: f32) {
    if let Some(child) = stage.child_mut(path) {
        child.set_matrix(Matrix {
            a: size,
            d: size,
            tx: at.0,
            ty: at.1,
            ..Matrix::IDENTITY
        });
    }
}

pub(crate) fn show(stage: &mut Stage, path: &[u16], visible: bool) {
    if let Some(child) = stage.child_mut(path) {
        child.set_visible(visible);
    }
}

/// The lines written in the corner of the batting view, each under the last.
#[derive(Default)]
struct Corner {
    lines: u32,
}

impl Corner {
    /// Where the next line goes: the middle of the top of its words.
    fn line(&mut self) -> Point {
        let at = (CORNER_AT.0, CORNER_AT.1 + self.lines as f32 * CORNER_ROW);
        self.lines += 1;
        at
    }
}

/// The colour of a bat this hot, from warm to as hot as it gets.
fn hot_colour(hot: u32, most: u32) -> Rgb {
    let share = hot as f32 / most.max(1) as f32;
    [0xff, (0xc8 as f32 - 0x98 as f32 * share) as u8, 0x20]
}

/// Where across the batting view the art's pointer shows a hit going, for a
/// ball that crosses at `crosses` with the ring held at `aim`. Aiming to
/// one side sends the ball the other way, and a ball that comes in
/// off-centre goes off further still.
fn hit_towards(crosses: f32, aim: f32, centre: f32, pull: f32) -> f32 {
    let off = (crosses - aim) + (crosses - centre);
    (crosses + off * pull).ceil()
}

pub(crate) fn frame_of(stage: &Stage, path: &[u16]) -> u16 {
    stage.clip(path).map_or(0, |clip| clip.frame)
}

impl Match {
    pub fn new(game: &Game, seed: u64, library: &Library) -> Match {
        // With every hit a home run there are more runs to get.
        let behind = if game.mods.is_on(Mod::ZingerHit) {
            game.rules.zinger.runs_down
        } else {
            game.rules.game.runs_down
        }
        .at(game.settings.difficulty);
        Match {
            score: 0,
            // Drawing level is not enough: the target is one run more.
            target: behind + 1,
            outs: 0,
            max_outs: game.rules.game.outs,
            strikes: 0,
            balls: 0,
            pitched: 0,
            runners: Vec::new(),
            rng: Rng::new(seed),
            phase: Phase::Arriving,
            announce: false,
            at: None,
            cues: Vec::new(),
            put_away: Vec::new(),
            arcade: None,
            full: None,
            came_up: 0,
            line_up: Vec::new(),
            tally: Vec::new(),
            outs_before: 0,
            thrown_at: (0, 0),
            slips: 0,
            streak: 0,
            run_worth: 1,
            lights: false,
            flash: 0,
            hurry: 0.0,
            heat: 0,
            heat_score: 0,
            bullet: None,
            slow_beat: 0,
            slowed: false,
            late_press: None,
            rally: 0,
            rallying: false,
            clutch: false,
            southpaw: false,
            sign: None,
            sign_rng: Rng::new(seed ^ SIGN_SEED),
            sign_news: None,
            sign_struck: None,
            stolen: 0,
            caught: 0,
            steal_play: false,
            steal_news: None,
            arm: 0,
            relieved: 0,
            tired: None,
            spray: Vec::new(),
            shift: 0.0,
            longest_zinger: 0,
            zinger_record: 0,
            runner_symbol: library.manifest.exports.get("runner").copied(),
        }
    }

    /// The arcade game instead of a match.
    pub fn new_arcade(game: &Game, seed: u64, library: &Library) -> Match {
        let mut arcade = Match::new(game, seed, library);
        arcade.arcade = Some(arcade::Arcade::new(game.rules.arcade.pitches));
        arcade
    }

    /// How many strikes put a batter out: three, unless a mod says
    /// otherwise.
    pub(crate) fn strikes_allowed(&self, game: &Game) -> u32 {
        if game.mods.is_on(Mod::SuddenDeath) {
            game.rules.sudden_death.strikes
        } else {
            game.rules.count.strikes
        }
    }

    /// How many a run counts for on the pitch about to be thrown, which is
    /// a golden ball or is not.
    fn worth_of_a_run(&self, golden: bool, game: &Game) -> u32 {
        let mut worth = if golden { game.rules.golden.runs } else { 1 };
        if game.mods.is_on(Mod::SuddenDeath) {
            worth *= game.rules.sudden_death.runs;
        }
        // A rally makes a run worth one more for each batter in it, and
        // whatever else multiplies runs multiplies that.
        if game.mods.is_on(Mod::Rally) {
            worth *= game.rules.rally.worth(self.rally);
        }
        if self.in_the_clutch(game) {
            worth *= game.rules.clutch.runs;
        }
        worth
    }

    /// Whether the pitch about to be thrown is one the clutch mod makes
    /// runs count for more on: the side has one out left, and a runner is
    /// on second or third.
    fn in_the_clutch(&self, game: &Game) -> bool {
        game.mods.is_on(Mod::Clutch)
            && self.arcade.is_none()
            && self.outs + 1 == self.max_outs
            && (self.on_base(2).is_some() || self.on_base(3).is_some())
    }

    /// A strike has been called: with the heat check mod on, the pitches
    /// slow down by a run's worth.
    pub(crate) fn cool(&mut self, game: &Game) {
        if game.mods.is_on(Mod::HeatCheck) {
            self.heat = self.heat.saturating_sub(1);
        }
    }

    /// The longest zinger of this game, in feet. Nought if there was none.
    pub fn longest_zinger(&self) -> u32 {
        self.longest_zinger
    }

    /// The longest zinger there has ever been, as far as this game knows.
    pub fn zinger_record(&self) -> u32 {
        self.zinger_record
    }

    /// Tells the game the record its zingers have to beat.
    pub fn set_zinger_record(&mut self, feet: u32) {
        self.zinger_record = feet;
    }

    /// A zinger has gone `feet`: it is counted, and may be a record.
    fn count_zinger(&mut self, feet: u32) -> bool {
        self.longest_zinger = self.longest_zinger.max(feet);
        let record = feet > self.zinger_record;
        if record {
            self.zinger_record = feet;
        }
        record
    }

    /// A zinger has come down: the player is told how far it went and
    /// where, and the crowd is heard.
    pub(crate) fn zinger_down(
        &mut self,
        show: &mut zinger::Show,
        stage: &mut Stage,
        library: &Library,
    ) {
        let record = self.count_zinger(show.zinger.feet);
        show.landed(record, stage);
        self.lights = true;
        let mut sounds = show.place.cheers().to_vec();
        if record && !sounds.contains(&"baseball_organ_FX") {
            sounds.push("baseball_organ_FX");
        }
        for name in sounds {
            Match::sound(stage, library, name);
        }
    }

    /// How the match stands, if it is over.
    fn outcome(&self) -> Option<Outcome> {
        if let Some(arcade) = &self.arcade {
            return (arcade.left == 0).then_some(Outcome::ArcadeOver);
        }
        if let Some(full) = &self.full {
            // Batting last with every innings all but played, to be ahead
            // is to have won. Otherwise the side bats until it is out, and
            // what that comes to is worked out once the other side has
            // batted.
            if full.sudden() && self.score > full.theirs() {
                return Some(Outcome::Won);
            }
            return (self.outs >= self.max_outs).then_some(Outcome::Interval);
        }
        let level = self.target - 1;
        if self.score >= self.target {
            Some(Outcome::Won)
        } else if self.outs < self.max_outs {
            None
        } else if self.score == level {
            Some(Outcome::Tied)
        } else {
            Some(Outcome::Lost)
        }
    }

    /// The batter at the plate: his place in `runners`.
    pub(crate) fn batter(&self) -> Option<usize> {
        self.runners
            .iter()
            .position(|runner| runner.place == Place::AtBat)
    }

    /// The runner standing on a base, if there is one.
    pub(crate) fn on_base(&self, base: u8) -> Option<usize> {
        self.runners
            .iter()
            .position(|runner| runner.place == Place::Base(base) && runner.running_to.is_none())
    }

    pub(crate) fn anyone_running(&self) -> bool {
        self.runners
            .iter()
            .any(|runner| runner.running_to.is_some())
    }

    /// The arcade game's points with the skill level counted in.
    pub fn arcade_score(&self, game: &Game) -> Option<u32> {
        let arcade = self.arcade.as_ref()?;
        Some(arcade.points * game.rules.arcade.multiplier.at(game.settings.difficulty))
    }

    /// How the side should look just now, given the team's own colour.
    pub fn look(&self, clothes: Option<Rgb>) -> Look {
        let batter = self.batter().map(|index| &self.runners[index]);
        Look {
            clothes,
            skin: batter.and_then(|runner| runner.skin),
            logo: batter.and_then(|runner| runner.logo.clone()),
            second_skin: self.on_base(2).and_then(|index| self.runners[index].skin),
        }
    }

    /// The batter's turn is over: the next one starts with a clean count.
    pub(crate) fn clear_count(&mut self) {
        self.strikes = 0;
        self.balls = 0;
    }

    /// Writes the numbers the scoreboards show.
    pub(crate) fn show_numbers(&self, stage: &mut Stage) {
        let batter = self
            .batter()
            .map_or(self.came_up, |index| self.runners[index].order + 1);
        // In a full match the board shows the other side's score where it
        // would show the score to beat.
        let shown_target = match self.full {
            Some(_) => self.target - 1,
            None => self.target,
        };
        for (name, value) in [
            ("score", self.score),
            ("out", self.outs),
            ("strikes", self.strikes),
            ("noBalls", self.balls),
            ("scoreTarget", shown_target),
            ("oppositionScore", self.target - 1),
            ("maximumOuts", self.max_outs),
            ("runsToGet", self.target.saturating_sub(self.score)),
            ("ballsPitched", self.pitched),
            (
                "points_total",
                self.arcade.as_ref().map_or(0, |arcade| arcade.points),
            ),
            ("batsmanOnStrike", batter as u32),
        ] {
            stage.set_text(name, value.to_string());
        }
        for (order, runs) in self.runs_by_order().into_iter().enumerate() {
            stage.set_text(&format!("batsman{}_score", order + 1), runs.to_string());
        }
    }

    /// The runs made by each place in the batting order.
    pub(crate) fn runs_by_order(&self) -> Vec<u32> {
        let mut runs = self.tally.clone();
        for runner in &self.runners {
            if runs.len() <= runner.order {
                runs.resize(runner.order + 1, 0);
            }
            runs[runner.order] += runner.runs;
        }
        runs
    }

    /// What the result screens say about the match just played.
    pub fn show_result(&self, stage: &mut Stage) {
        self.show_numbers(stage);
        // A full match's outs are those of all its innings. A play that
        // put out more than were left to get is not counted for more.
        let outs = self.outs_before + self.outs.min(self.max_outs);
        stage.set_text("out", outs.to_string());
        for order in self.runs_by_order().len()..ORDER {
            stage.set_text(&format!("batsman{}_score", order + 1), "0");
        }
    }

    pub(crate) fn sound(stage: &mut Stage, library: &Library, name: &str) {
        stage.play_sound(name, 1, library);
    }

    /// Sets a clip playing from a label, to be sent back to its first frame
    /// when it reaches `end`, where the art's own script did that.
    pub(crate) fn play_section(
        &mut self,
        path: &[u16],
        label: &str,
        end: u16,
        stage: &mut Stage,
        library: &Library,
    ) {
        self.cues.retain(|cue| cue.path != path);
        if stage.goto_label(path, label, true, library) {
            self.cues.push(Cue {
                path: path.to_vec(),
                frame: end,
                rewind: true,
            });
        }
    }

    fn run_cues(&mut self, stage: &mut Stage, library: &Library) {
        self.put_away.retain_mut(|(path, left)| {
            if *left > 0 {
                *left -= 1;
                return true;
            }
            stage.goto_clip(path, 1, library);
            if let Some(clip) = stage.clip_mut(path) {
                clip.playing = false;
            }
            false
        });
        let mut due = Vec::new();
        self.cues.retain(|cue| match stage.clip(&cue.path) {
            Some(clip) if clip.frame >= cue.frame => {
                due.push(cue.clone());
                false
            }
            Some(_) => true,
            None => false,
        });
        for cue in due {
            if cue.rewind {
                stage.goto_clip(&cue.path, 1, library);
            }
            if let Some(clip) = stage.clip_mut(&cue.path) {
                clip.playing = false;
            }
        }
    }

    /// Finds the parts of a batting view that has just been built.
    fn parts(stage: &Stage, library: &Library) -> Option<Parts> {
        let main = stage.find_named(&[], "gameMain")?;
        let part = |names: &[&str]| stage.find(&main, names);
        let field = part(&["field"])?;
        let in_field = |name: &str| stage.find(&field, &[name]);
        let point = |path: Option<Path>| path.map(|path| at(stage, &path));
        let aim_box = stage
            .child(&part(&["acl"])?)
            .and_then(|child| child_bounds(child, Matrix::IDENTITY, library))?;
        let fly = part(&["ballFly"])?;
        let field_ball = in_field("ballFly")?;
        Some(Parts {
            pitcher: part(&["pitcher"])?,
            hitter: part(&["hitter"])?,
            aim: part(&["aimCircle"])?,
            aim_shadow: part(&["aimCircleShadow"])?,
            marker: part(&["ballPassesBat_marker"])?,
            ball: part(&["ballAll"])?,
            shadow: part(&["ballShadow"])?,
            fly_ball: stage.find(&fly, &["ball"])?,
            fly_shadow: stage.find(&fly, &["ballShadow"]),
            fly,
            aim_area: part(&["aimArea"])?,
            scoreboard: part(&["scoreboard"]),
            strike_anim: part(&["strikeAnim_old"]).or_else(|| part(&["strikeAnim"])),
            transitions: part(&["transitions"])?,
            next: part(&["btn_nextBall"])?,
            flare: part(&["lightFlare"]),
            field_ball_inner: stage.find(&field_ball, &["ball"])?,
            field_ball,
            holder: in_field("runnerHolder"),
            fielders: (1..=9)
                .filter_map(|number| in_field(&format!("fielder{number}")))
                .collect(),
            umpires: (1..=3)
                .filter_map(|number| in_field(&format!("umpire{number}")))
                .collect(),
            field_scoreboard: in_field("scoreboard"),
            centre_x: point(part(&["centreMarker"]))?.0,
            fly_mark: point(part(&["shadowFlyMarker"]))?,
            ground_y: point(part(&["uMarker"]))?.1,
            aim_box,
            home: point(in_field("startPointMarker"))?,
            field_mark: point(in_field("shadowFlyMarker"))?,
            // The arcade game's field has no foul lines and no bases.
            foul: (
                point(in_field("foulMarkerLeft")).map_or(f32::MIN, |at| at.0),
                point(in_field("foulMarkerRight")).map_or(f32::MAX, |at| at.0),
            ),
            bases: [1, 2, 3, 4]
                .map(|base| point(in_field(&format!("base{base}"))).unwrap_or_default()),
            field,
            main,
        })
    }

    /// The fixed points a pitch is drawn between.
    fn mound(parts: &Parts, stage: &Stage, library: &Library) -> Option<Mound> {
        let test = stage.find(&parts.main, &["test"])?;
        let point = |name: &str| stage.find(&test, &[name]).map(|path| at(stage, &path));
        // With no strike zone to miss, as in the arcade game, no pitch is
        // ever outside it.
        let zone = stage
            .find(&parts.main, &["strikeZone"])
            .and_then(|path| stage.child(&path))
            .and_then(|child| child_bounds(child, Matrix::IDENTITY, library))
            .unwrap_or([f32::MIN, f32::MIN, f32::MAX, f32::MAX]);
        Some(Mound {
            ball: point("ballAll")?,
            shadow: point("ballShadow")?,
            ball_from: point("startpointMarker")?,
            shadow_from: point("startpointShadowMarker")?,
            plate: point("shadowMarker")?.1,
            zone,
        })
    }

    /// Gets a freshly built batting view ready for a pitch. Returns how the
    /// match ended if it has.
    fn set_up(&mut self, game: &Game, stage: &mut Stage, library: &Library) -> Option<Outcome> {
        let mut parts = Match::parts(stage, library)?;
        self.cues.clear();
        self.put_away.clear();
        // With the southpaw mod on the batter stands on the other side of
        // the plate, turned round. The number on his shirt is not.
        self.southpaw = game.mods.is_on(Mod::Southpaw);
        stage.upright_text = self.southpaw;
        if self.southpaw {
            southpaw::stand(&mut parts, stage);
        }
        if let Some(outcome) = self.outcome() {
            self.phase = Phase::Over;
            return Some(self.close_half(outcome));
        }
        if self.batter().is_none() {
            // Each batter in a match has his own skin, and they carry the
            // bat logos in turn. The arcade game's one batter is as chosen.
            // A full match's nine come round again, each as he was.
            let team = &game.rules.team;
            let order = match self.full {
                Some(_) => self.came_up % ORDER,
                None => self.came_up,
            };
            let (skin, logo) = if self.arcade.is_some() {
                (game.settings.skin, game.settings.logo.clone())
            } else {
                let known = self.full.as_ref().and(self.line_up.get(order).copied());
                let skin = known.unwrap_or_else(|| {
                    let pick = self.rng.below(team.skins.len() as u32) as usize;
                    team.skins.get(pick).and_then(|skin| look::rgb(skin))
                });
                if self.full.is_some() && self.line_up.len() <= order {
                    self.line_up.resize(order + 1, None);
                    self.line_up[order] = skin;
                }
                (
                    skin,
                    team.logos.get(order % team.logos.len().max(1)).cloned(),
                )
            };
            self.came_up += 1;
            self.runners.push(Runner {
                place: Place::AtBat,
                running_to: None,
                sliding: false,
                runs: 0,
                order,
                stole_from: None,
                skin,
                logo,
                path: None,
            });
            self.clear_count();
        }

        let rules = &game.rules;
        let mut table = rules.pitch.at(game.settings.difficulty).clone();
        let mut notices = Vec::new();
        // A full match says which half of which innings this is, and what
        // the mods say goes under that.
        let mut corner = Corner::default();
        if let Some(full) = &self.full {
            Notice::put(
                &mut notices,
                &parts,
                "innings",
                &full.half_words(),
                corner.line(),
                0.8,
                [0xfd, 0xf6, 0xc0],
                None,
                stage,
                library,
            );
        }
        // The pitch about to be thrown is one more than have been. The
        // arcade game has no runs and no outs for a golden ball to change.
        let golden = game.mods.is_on(Mod::GoldenBall)
            && self.arcade.is_none()
            && rules.golden.is_gold(self.pitched + 1);
        self.run_worth = self.worth_of_a_run(golden, game);
        if golden {
            for ball in [&parts.ball, &parts.fly_ball, &parts.field_ball] {
                if let Some(ball) = stage.child_mut(ball) {
                    ball.set_color(GOLD);
                }
            }
        }
        if game.mods.is_on(Mod::HeatCheck) {
            // Every run since the last pitch makes this one faster.
            let runs = self.score.saturating_sub(self.heat_score);
            self.heat = (self.heat + runs).min(rules.heat.most);
            self.heat_score = self.score;
            table.speed = table.speed.times(rules.heat.time(self.heat));
            if self.heat > 0 {
                let hot = self.heat as f32 / rules.heat.most.max(1) as f32;
                let colour = [0xff, (0xe0 as f32 - 0xa0 as f32 * hot) as u8, 0x30];
                let says = format!("HEAT {}", self.heat);
                let top = corner.line();
                Notice::put(
                    &mut notices,
                    &parts,
                    "heat",
                    &says,
                    top,
                    0.8,
                    colour,
                    None,
                    stage,
                    library,
                );
            }
        }
        // A tired arm is slower and wilder, and one that has thrown its last
        // gives way to a fresh one. The arcade game is over before any arm
        // tires.
        if game.mods.is_on(Mod::TiredArm) && self.arcade.is_none() {
            let arm = &rules.tired_arm;
            if self.arm >= arm.relief.max(1) {
                self.arm = 0;
                self.relieved += 1;
                Match::sound(stage, library, "baseball_organ_FX");
                Notice::put(
                    &mut notices,
                    &parts,
                    "newPitcher",
                    "NEW PITCHER",
                    (parts.centre_x, MYSTERY_TOP),
                    1.0,
                    [0xc8, 0xf0, 0xff],
                    Some(arm.told_time),
                    stage,
                    library,
                );
            }
            let tired = arm.tired(self.arm);
            self.tired = Some(tired);
            table = arm.pitch(&table, tired);
            if let Some(pitcher) = stage.child_mut(&parts.pitcher) {
                let left = 1.0 - FLUSH * tired;
                pitcher.set_color(ColorTransform {
                    mult: [1.0, left, left, 1.0],
                    add: [0.0; 4],
                });
            }
            // From white, through yellow, to red.
            let colour = [
                0xff,
                (0xff as f32 - 0x90 as f32 * tired) as u8,
                (0xff as f32 - 0xc0 as f32 * tired.min(0.5) * 2.0) as u8,
            ];
            Notice::put(
                &mut notices,
                &parts,
                "pitches",
                &format!("PITCHES {}", self.arm),
                corner.line(),
                0.8,
                colour,
                None,
                stage,
                library,
            );
        }
        if game.mods.is_on(Mod::HotBat) && self.streak > 0 {
            // Every hit in a row has widened the window by a frame at
            // each end.
            let more = self.streak.min(rules.hot_bat.most);
            table.window = pitch::widened(&table.window, more);
            let says = format!("HOT BAT {more}");
            let colour = hot_colour(more, rules.hot_bat.most);
            Notice::put(
                &mut notices,
                &parts,
                "hotBat",
                &says,
                corner.line(),
                0.8,
                colour,
                None,
                stage,
                library,
            );
        }
        if golden {
            Notice::put(
                &mut notices,
                &parts,
                "goldenBall",
                "GOLDEN BALL",
                corner.line(),
                0.8,
                [0xff, 0xd2, 0x40],
                None,
                stage,
                library,
            );
        }
        self.clutch = self.in_the_clutch(game);
        if self.clutch {
            // The organ plays as the batter comes up to it, and not again
            // for every pitch to him.
            if self.strikes + self.balls == 0 {
                Match::sound(stage, library, "baseball_organ_tense_FX");
            }
            Notice::put(
                &mut notices,
                &parts,
                "clutch",
                &format!("CLUTCH: RUNS X{}", rules.clutch.runs),
                corner.line(),
                0.8,
                [0xff, 0x8a, 0x6a],
                None,
                stage,
                library,
            );
        }
        self.rallying = game.mods.is_on(Mod::Rally) && self.arcade.is_none();
        if self.rallying && self.rally > 0 {
            let worth = rules.rally.worth(self.rally);
            Notice::put(
                &mut notices,
                &parts,
                "rally",
                &format!("RALLY: RUNS X{worth}"),
                corner.line(),
                0.8,
                hot_colour(self.rally.min(rules.rally.most), rules.rally.most),
                None,
                stage,
                library,
            );
        }
        // With bullet time on there is a meter, full when the game starts.
        let meter = if game.mods.is_on(Mod::BulletTime) {
            self.bullet.get_or_insert(rules.bullet_time.full);
            let (top, under) = (corner.line(), corner.line());
            let meter = bullet::Meter::put(&parts, top, (under.0, under.1 + 2.0), stage, library);
            if let (Some(meter), Some(left)) = (&meter, self.bullet) {
                let full = rules.bullet_time.full.max(1) as f32;
                meter.keep(left as f32 / full, false, stage);
            }
            meter
        } else {
            self.bullet = None;
            None
        };
        // With the shift on, the fielders stand where the last few balls
        // went. The arcade game has no fielders to move.
        if game.mods.is_on(Mod::TheShift) && self.arcade.is_none() {
            let shift = shift::Shift::of(&self.spray, &rules.shift);
            self.shift = shift.by();
            shift.place(&parts, &rules.field, stage, library);
            if let Some(says) = shift.words(&rules.shift) {
                Notice::put(
                    &mut notices,
                    &parts,
                    "shift",
                    says,
                    corner.line(),
                    0.8,
                    [0xc8, 0xf0, 0xff],
                    None,
                    stage,
                    library,
                );
            }
        }
        show(stage, &parts.ball, false);
        show(stage, &parts.shadow, false);
        if let Some(zone) = stage.find(&parts.main, &["strikeZone"]) {
            show(stage, &zone, table.show_zone);
        }
        // Every runner still in the game stands where the last pitch left
        // him.
        self.steal_play = false;
        for index in 0..self.runners.len() {
            self.runners[index].path = None;
            self.runners[index].running_to = None;
            self.runners[index].stole_from = None;
            self.runners[index].sliding = false;
            let label = match self.runners[index].place {
                Place::AtBat => "waiting".to_owned(),
                Place::Base(base) => format!("base{base}"),
                Place::Out | Place::Home => continue,
            };
            let Some(symbol) = self.runner_symbol else {
                continue;
            };
            let Some(holder) = &parts.holder else {
                continue;
            };
            let depth = Stage::RULES_DEPTH + index as u16;
            let name = format!("runner{}", index + 1);
            if let Some(path) = stage.attach(holder, symbol, depth, &name, library) {
                stage.goto_label(&path, &label, false, library);
                self.runners[index].path = Some(path);
            }
        }
        // With the stolen bases mod on, each runner is marked on the little
        // field in the corner, and the mod has a line under it to write on.
        let on_base = |runner: &Runner| matches!(runner.place, Place::Base(_));
        let leads = (game.mods.is_on(Mod::StolenBases) && self.runners.iter().any(on_base))
            .then(|| steal::Leads::put(&self.runners, corner.line(), &parts, stage, library))
            .flatten();
        // With the hit the sign mod on, the wall has its signs, one lit for
        // the innings. The arcade game has no runs for a sign to be worth.
        self.sign_struck = None;
        let signs = if game.mods.is_on(Mod::HitTheSign) && self.arcade.is_none() {
            let signs = sign::Signs::of(&rules.sign);
            let lit = self.lit_sign(&signs);
            let ground = parts.ground(&rules.field);
            let (sign, field) = (&rules.sign, &rules.field);
            sign::Board::put(&signs, lit, sign, field, &parts, &ground, stage, library)
        } else {
            None
        };
        if let Some(mark) = stage.find(&parts.main, &["runnerOnSecond"]) {
            let label = if self.on_base(2).is_some() {
                "full"
            } else {
                "none"
            };
            stage.goto_label(&mark, label, false, library);
        }
        // The fielders who mind the bases stand ready at them.
        for fielder in parts.fielders.iter().skip(5) {
            stage.goto_label(fielder, "baseWaiting", false, library);
        }
        if self.announce {
            self.announce = false;
            // In a full match there is a number of runs that wins it only
            // when getting ahead ends it.
            let to_win = self.full.as_ref().is_none_or(|full| full.sudden());
            if let (true, Some(board)) = (to_win, parts.scoreboard.clone()) {
                self.play_section(&board, "runsToGet", 361, stage, library);
            }
        }
        let them = match self.full {
            Some(_) => full::Them::put(&parts, stage, library),
            None => Vec::new(),
        };
        self.set_up_arcade(&parts, game, stage, library);
        self.show_numbers(stage);

        let mound = Match::mound(&parts, stage, library)?;
        // A pitcher waits longer before a slower pitch. A mystery pitch
        // would be no mystery if he did, so before one he waits as long as
        // for a pitch of the usual pace, and the marker is not shown until
        // the ball has left his hand.
        let usual_pace = table.speed.high as f32;
        let mut kind = None;
        if game.mods.is_on(Mod::MysteryPitch) {
            let which = Kind::ALL[self.rng.below(Kind::ALL.len() as u32) as usize];
            which.shape(&mut table, &rules.mystery, self.rng.below(2) == 0);
            table.marker_frame = rules.throw.release_frame;
            kind = Some(which);
        }
        let mut choice = Choice::pick(&table, &rules.throw, &mut self.rng);
        if self.southpaw {
            // A left-hander is pitched to as a right-hander was.
            southpaw::turn(&mut choice, parts.centre_x);
        }
        let mut pitch = Pitch::throw(&choice, &mound, &rules.throw);
        let wait = match kind {
            Some(_) => {
                let usual = Choice {
                    speed: usual_pace,
                    ..choice
                };
                Pitch::throw(&usual, &mound, &rules.throw).samples.len()
            }
            None => pitch.samples.len(),
        };
        // The marker shows where the pitch was going before a knuckleball
        // began to sway.
        let marker_at = pitch.crosses;
        if game.mods.is_on(Mod::Knuckleball) {
            let knuckle = &rules.knuckleball;
            let start = self.rng.unit();
            pitch.knuckle(knuckle.sway, knuckle.turns, start, &mound);
        }
        // With the zinger mod on as well, the timing bar says how far a
        // swing on each of its colours sends the ball at the most.
        let difficulty = game.settings.difficulty;
        let feet = |frames: u32| {
            Zinger::of(&table, frames, (0.0, 0.0), parts.home, difficulty, rules)
                .map(|zinger| zinger.feet)
        };
        let timing = game
            .mods
            .is_on(Mod::TimingIndicator)
            .then(|| {
                let feet: Option<&dyn Fn(u32) -> Option<u32>> =
                    game.mods.is_on(Mod::ZingerHit).then_some(&feet);
                timing::Indicator::new(&pitch, &table, &parts, feet, stage, library)
            })
            .flatten();
        self.phase = Phase::Settling {
            left: rules.throw.settle + wait as u32,
        };
        self.at = Some(AtBat {
            aim: at(stage, &parts.aim),
            aim_area_x: at(stage, &parts.aim_area).0,
            parts,
            table,
            pitch,
            marker_shown: false,
            marker_at,
            kind,
            golden,
            called: None,
            swing: None,
            under: 0.0,
            across: 0.0,
            contact: None,
            fly: ((0.0, 0.0), 0.0, 0.0),
            fly_size: 1.0,
            fly_target: (0.0, 0.0),
            run_in: None,
            ball: None,
            fielding: None,
            timing,
            zinger: None,
            zinger_show: None,
            over_wall: false,
            notices,
            came_down: None,
            rebounds: 0,
            them,
            swing_off: None,
            met: None,
            leads,
            signs,
            meter,
        });
        None
    }

    /// Stills the batter once his swing is done.
    ///
    /// The swing is a clip that stops on its last frame, with his skin,
    /// shirt and helmet as clips of their own inside it, moving in step.
    /// The art stopped those from a script. Left alone they go round again
    /// over a body that has stopped, and he swings on for ever.
    fn still_batter(stage: &mut Stage, hitter: &[u16], library: &Library) {
        let Some(clip) = stage.clip(hitter) else {
            return;
        };
        let mut moving = Vec::new();
        for (&depth, child) in &clip.children {
            let Content::Clip(swing) = &child.content else {
                continue;
            };
            let last = swing.frame_count(library);
            if swing.playing || last <= 1 || swing.frame != last {
                continue;
            }
            for (&inner_depth, inner) in &swing.children {
                if let Content::Clip(part) = &inner.content
                    && part.playing
                    && part.frame_count(library) > 1
                {
                    let mut path = hitter.to_vec();
                    path.extend([depth, inner_depth]);
                    moving.push((path, part.frame_count(library)));
                }
            }
        }
        for (path, last) in moving {
            stage.goto_clip(&path, last, library);
            if let Some(part) = stage.clip_mut(&path) {
                part.playing = false;
            }
        }
    }

    /// Moves the aiming ring a step towards the pointer, and with it the
    /// art's pointer to where a hit would go.
    fn aim(at_bat: &mut AtBat, stage: &mut Stage) {
        let parts = &at_bat.parts;
        let pointer = (stage.pointer.x, stage.pointer.y);
        // A pointer nobody has moved yet is nowhere.
        if pointer.0 < -1.0e5 {
            return;
        }
        let Some(pointer) = stage.from_stage(&parts.main, pointer.0, pointer.1) else {
            return;
        };
        let [left, top, right, bottom] = parts.aim_box;
        let ease = at_bat.table.aim_ease.max(1.0);
        at_bat.aim.0 += (pointer.0 - at_bat.aim.0) / ease;
        at_bat.aim.1 += (pointer.1 - at_bat.aim.1) / ease;
        at_bat.aim.0 = at_bat.aim.0.clamp(left + 1.0, right - 1.0);
        at_bat.aim.1 = at_bat.aim.1.clamp(top + 1.0, bottom - 1.0);
        if let Some(ring) = stage.child_mut(&parts.aim) {
            ring.move_to(at_bat.aim.0, at_bat.aim.1);
        }
        let shadow_y = at(stage, &parts.aim_shadow).1;
        if let Some(shadow) = stage.child_mut(&parts.aim_shadow) {
            shadow.move_to(at_bat.aim.0, shadow_y);
        }
    }

    /// Works out where a hit made now would go sideways, and shows it.
    fn point_hit(at_bat: &mut AtBat, rules: &HitRules, stage: &mut Stage, library: &Library) {
        if at_bat.contact.is_some() {
            return;
        }
        let parts = &at_bat.parts;
        let pull = rules.pull;
        // Unless the rules say otherwise, the pointer is kept out of sight
        // until there is a crossing point for it to answer to.
        show(
            stage,
            &parts.aim_area,
            at_bat.marker_shown || rules.pointer_before_pitch,
        );
        // Until the player has been shown where this pitch will cross, the
        // pointer answers the ring as if it were coming down the middle.
        // It answers to where the player has been shown the ball crossing,
        // which is not always quite where it will.
        let crosses = if at_bat.marker_shown {
            at_bat.marker_at.0
        } else {
            parts.centre_x
        };
        at_bat.aim_area_x = hit_towards(crosses, at_bat.aim.0, parts.centre_x, pull);
        let y = at(stage, &parts.aim_area).1;
        if let Some(area) = stage.child_mut(&parts.aim_area) {
            area.move_to(at_bat.aim_area_x, y);
        }
        // The pointer's look is drawn for every position, one a frame.
        let frame = at_bat.aim_area_x.clamp(1.0, 550.0) as u16;
        stage.goto_clip(&parts.aim_area, frame, library);
        if let Some(clip) = stage.clip_mut(&parts.aim_area) {
            clip.playing = false;
        }
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
        let slowed = std::mem::take(&mut self.slowed);
        let night = game.mods.is_on(Mod::NightGame);
        if night || game.mods.is_on(Mod::BulletTime) {
            if std::mem::take(&mut self.lights) && night {
                self.flash = game.rules.night.flash_time;
            }
            let lighting = if night {
                night::lighting(self.flash, &game.rules.night)
            } else {
                night::DAY
            };
            let lighting = if slowed {
                bullet::cool(lighting)
            } else {
                lighting
            };
            night::light(lighting, stage);
            self.flash = self.flash.saturating_sub(1);
        }

        if self.phase == Phase::Arriving {
            return self.set_up(game, stage, library);
        }
        let mut at_bat = self.at.take()?;
        // The view has gone: the screen was left.
        stage.clip(&at_bat.parts.main)?;
        let rules = &game.rules;
        // While the ring is being aimed it stands for the pointer, which
        // would only get in its way.
        let aiming = at_bat.contact.is_none()
            && matches!(
                self.phase,
                Phase::Settling { .. } | Phase::WindUp | Phase::Flight { .. }
            );
        stage.hide_pointer = aiming
            && stage
                .from_stage(&at_bat.parts.main, stage.pointer.x, stage.pointer.y)
                .is_some_and(|(x, y)| {
                    let [left, top, right, bottom] = at_bat.parts.aim_box;
                    (left..=right).contains(&x) && (top..=bottom).contains(&y)
                });
        Match::still_batter(stage, &at_bat.parts.hitter, library);
        if game.mods.is_on(Mod::HotBat) && self.streak > 0 {
            // The mark on the bat glows, hotter the longer the run of hits.
            let most = rules.hot_bat.most;
            let glow = look::tint(hot_colour(self.streak.min(most), most));
            for mark in art::all_named(stage, &at_bat.parts.hitter, "batLogo") {
                if let Some(mark) = stage.child_mut(&mark) {
                    mark.set_color(glow);
                }
            }
        }
        Match::settle_fielders(&at_bat.parts, stage, library);
        for them in &at_bat.them {
            them.keep(stage);
        }
        overlay::Notice::fade(&mut at_bat.notices, stage);
        if let Some(leads) = &mut at_bat.leads {
            leads.keep(self, self.phase == Phase::WindUp, stage);
            self.hold_stealers(stage);
        }
        if let Some(signs) = &mut at_bat.signs {
            signs.keep(stage);
        }
        if let (Some(meter), Some(left)) = (&at_bat.meter, self.bullet) {
            let full = rules.bullet_time.full.max(1) as f32;
            meter.keep(left as f32 / full, slowed, stage);
        }
        if at_bat.contact.is_none() {
            Match::aim(&mut at_bat, stage);
            Match::point_hit(&mut at_bat, &rules.hit, stage, library);
        }

        // In the arcade game the ball goes on over the field while the next
        // pitch is already on offer.
        if matches!(self.phase, Phase::Ready | Phase::Leaving { .. }) {
            self.arcade_ball(&mut at_bat, game, stage, library);
        }
        match self.phase {
            Phase::Settling { left } => {
                // While the pitcher waits, a click on the outfield calls
                // the shot. The arcade game has a target of its own.
                if let Some((x, y)) = pressed
                    && self.arcade.is_none()
                    && game.mods.is_on(Mod::CalledShot)
                    && let Some(pointer) = stage.from_stage(&at_bat.parts.main, x, y)
                {
                    let called = &mut at_bat.called;
                    called::Called::call(called, pointer, &at_bat.parts, rules, stage, library);
                }
                if left == 0 {
                    stage.goto_label(&at_bat.parts.pitcher, PITCH, true, library);
                    self.phase = Phase::WindUp;
                    self.ask_for_steals(&mut at_bat, stage, library);
                } else {
                    self.phase = Phase::Settling { left: left - 1 };
                }
            }
            Phase::WindUp => {
                // While he winds up, a click on the little field sends a
                // runner.
                if let Some((x, y)) = pressed
                    && let Some(pointer) = stage.from_stage(&at_bat.parts.main, x, y)
                {
                    self.steal_click(&mut at_bat, pointer, stage, library);
                }
                let frame = frame_of(stage, &at_bat.parts.pitcher);
                if !at_bat.marker_shown && frame >= at_bat.table.marker_frame {
                    at_bat.marker_shown = true;
                    put(stage, &at_bat.parts.marker, at_bat.marker_at, 1.0);
                }
                if frame >= rules.throw.release_frame {
                    show(stage, &at_bat.parts.ball, true);
                    show(stage, &at_bat.parts.shadow, true);
                    self.pitched += 1;
                    self.arm += 1;
                    self.stop_asking_for_steals(&mut at_bat, stage);
                    self.book_thrown();
                    if let Some(arcade) = &mut self.arcade {
                        arcade.left = arcade.left.saturating_sub(1);
                    }
                    self.show_numbers(stage);
                    if let Some(kind) = at_bat.kind {
                        // Now it can be told what he threw.
                        let top = (at_bat.parts.centre_x, MYSTERY_TOP);
                        let frames = Some(rules.mystery.told_time);
                        Notice::put(
                            &mut at_bat.notices,
                            &at_bat.parts,
                            "mysteryPitch",
                            kind.words(),
                            top,
                            1.0,
                            [0xff, 0xf2, 0x8a],
                            frames,
                            stage,
                            library,
                        );
                    }
                    self.phase = Phase::Flight { step: 0 };
                }
            }
            Phase::Flight { step } => {
                let pressed = self.late_press.take().or(pressed);
                if self.held_back(&at_bat, step, game, stage) {
                    // The ball stays where it is for this frame. A click
                    // made on it is for the step the ball is on.
                    self.late_press = pressed;
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
                if left == 0 && self.arcade.is_some() {
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
                    self.zinger_unseen();
                    // Building the view again starts the next pitch.
                    let mut holder = at_bat.parts.main.clone();
                    holder.pop();
                    stage.goto_clip(&holder, 1, library);
                    if let Some(clip) = stage.clip_mut(&holder) {
                        clip.playing = true;
                    }
                    self.phase = Phase::Arriving;
                    return None;
                }
                self.phase = Phase::Leaving { left: left - 1 };
            }
            Phase::Ready | Phase::Arriving | Phase::Over => {}
        }
        if let Some(bar) = &mut at_bat.timing {
            match self.phase {
                // A swing made now begins on the step the flight has come
                // to.
                Phase::Flight { step } => bar.point(step as i32, stage),
                // Until the ball is thrown, the steps to go are the frames
                // left of the wind-up.
                Phase::WindUp => {
                    let frame = frame_of(stage, &at_bat.parts.pitcher);
                    let to_go = i32::from(rules.throw.release_frame) - i32::from(frame);
                    bar.point(-to_go, stage);
                }
                _ => {}
            }
        }
        self.at = Some(at_bat);
        None
    }

    /// One frame of the ball on its way to the batter.
    fn flight(
        &mut self,
        at_bat: &mut AtBat,
        step: usize,
        pressed: Option<Point>,
        game: &Game,
        stage: &mut Stage,
        library: &Library,
    ) {
        let rules = &game.rules;
        let Some(&sample) = at_bat.pitch.samples.get(step) else {
            return self.call(at_bat, game, stage, library);
        };
        for (path, point) in [
            (&at_bat.parts.ball, sample.ball),
            (&at_bat.parts.shadow, sample.shadow),
        ] {
            put(stage, path, point, sample.size);
            if let Some(child) = stage.child_mut(path) {
                child.set_alpha(sample.alpha);
            }
        }

        if let Some((x, y)) = pressed
            && at_bat.swing.is_none()
        {
            // The swing is high, level or low by where the click was.
            let pointer = stage
                .from_stage(&at_bat.parts.main, x, y)
                .unwrap_or(at_bat.aim);
            let label = match pointer.1 {
                y if y <= 200.0 => "hitHigh",
                y if y <= 280.0 => "hitMed",
                _ => "hitLow",
            };
            stage.goto_label(&at_bat.parts.hitter, label, true, library);
            if self.arcade.is_none() {
                Match::sound(stage, library, "batSwing_fast");
            }
            at_bat.swing = Some(0);
            if self.full.is_some() {
                at_bat.swing_off = timing::Timing::of(&at_bat.pitch, &at_bat.table)
                    .best()
                    .map(|(first, last)| step as i32 - step.clamp(first, last) as i32);
            }
            at_bat.under = at_bat.aim.1 - at_bat.pitch.crosses.1;
            at_bat.across = at_bat.aim.0 - at_bat.pitch.crosses.0;
            if let Some(bar) = &mut at_bat.timing {
                bar.swung(step, stage);
            }
        } else if let Some(frames) = &mut at_bat.swing {
            *frames += 1;
        }

        let in_band = sample.in_band(at_bat.table.band);
        let met = at_bat.swing.and_then(|frames| {
            let (quality, power) = pitch::meets(&at_bat.table, frames)?;
            Some((frames, quality, power))
        });
        if let (true, Some((frames, quality, power))) = (in_band, met) {
            at_bat.met = Some(quality);
            if game.mods.is_on(Mod::HotBat) {
                self.streak += 1;
            }
            // With the zinger mod on, whatever the bat meets is on its way
            // out of the ground.
            let zinger = game
                .mods
                .is_on(Mod::ZingerHit)
                .then(|| {
                    let ring = (at_bat.across, at_bat.under);
                    let home = at_bat.parts.home;
                    let difficulty = game.settings.difficulty;
                    Zinger::of(&at_bat.table, frames, ring, home, difficulty, rules)
                })
                .flatten();
            let (hit, cheer): (&str, &[&str]) = match quality {
                Quality::Poor => ("batHit_poorly", &["crowd_smallClap"]),
                Quality::MediumPoor => ("batHit_mediumPoor", &["crowd_smallCheer"]),
                Quality::Medium => ("batHit_medium", &["crowd_smallCheer", "crowd_smallClap"]),
                Quality::Good => ("batHit_good", &["crowd_bigClap"]),
            };
            let (hit, cheer) = zinger.map_or((hit, cheer), |zinger| zinger.hit_sounds());
            Match::sound(stage, library, hit);
            for name in cheer {
                Match::sound(stage, library, name);
            }
            // The hit goes by where the ball really was, if that is not
            // where the marker showed it.
            if at_bat.marker_at != at_bat.pitch.crosses {
                let (crosses, centre) = (at_bat.pitch.crosses.0, at_bat.parts.centre_x);
                at_bat.aim_area_x = hit_towards(crosses, at_bat.aim.0, centre, rules.hit.pull);
            }
            if zinger.is_some() {
                at_bat.aim_area_x = zinger::fair(at_bat.aim_area_x, &at_bat.parts, &rules.field);
            }
            let aside = at_bat.aim_area_x - at_bat.parts.centre_x;
            let contact = match &zinger {
                Some(zinger) => zinger.contact(aside),
                None => Contact {
                    power,
                    under: at_bat.under,
                    aside,
                },
            };
            // In the batting view the ball flies off towards where the
            // art's pointer showed, dropping further the weaker the hit.
            let parts = &at_bat.parts;
            at_bat.fly = (
                sample.shadow,
                sample.shadow.1 - sample.ball.1,
                zinger.map_or(contact.lift(&rules.hit), |zinger| zinger.lift(&rules.hit)),
            );
            // It leaves the bat the size it had come to, and shrinks from
            // there as it goes away.
            at_bat.fly_size = sample.size;
            if let Some(shadow) = &parts.fly_shadow {
                let place = at(stage, shadow);
                put(stage, shadow, place, sample.size);
            }
            at_bat.fly_target = (
                at_bat.aim_area_x,
                parts.fly_mark.1 + contact.power + contact.miss() / 2.0,
            );
            // And over the field it heads for the mark, pushed aside by
            // the same amount.
            let mark = (
                parts.field_mark.0 + contact.aside / rules.field.aim_share,
                parts.field_mark.1,
            );
            at_bat.ball = Some(match &zinger {
                Some(zinger) => zinger.ball(parts.home, mark),
                None => Ball::hit(parts.home, mark, &contact, &rules.hit, &rules.field),
            });
            at_bat.zinger = zinger;
            if let (Some(arcade), Some(zinger)) = (&mut self.arcade, zinger) {
                // In the arcade game a zinger scores by how far it goes.
                arcade.owed = Some(zinger.feet);
            }
            show(stage, &parts.ball, false);
            show(stage, &parts.shadow, false);
            at_bat.contact = Some(contact);
            // He is 34 frames into his swing when he drops the bat.
            at_bat.run_in = Some(34u32.saturating_sub(at_bat.swing.unwrap_or(0)));
            self.phase = Phase::Watching {
                left: if self.arcade.is_some() {
                    at_bat.run_in = None;
                    rules.arcade.watch
                } else {
                    rules.hit.watch
                },
            };
            return;
        }
        self.phase = Phase::Flight { step: step + 1 };
    }

    /// The ball has gone by: a strike, or a ball.
    fn call(&mut self, at_bat: &mut AtBat, game: &Game, stage: &mut Stage, library: &Library) {
        let rules = &game.rules;
        let parts = at_bat.parts.clone();
        show(stage, &parts.ball, false);
        show(stage, &parts.shadow, false);
        if self.arcade.is_some() {
            // No count in the arcade game: a miss is just a pitch gone.
            return self.ready(&parts, stage, library);
        }
        Match::sound(stage, library, "ballCatch_1");
        if !at_bat.pitch.in_zone && at_bat.swing.is_none() {
            self.book_pitch(at_bat, Thrown::Ball);
            self.balls += 1;
            if let Some(board) = &parts.scoreboard {
                self.play_section(board, "noBall", 261, stage, library);
            }
            self.show_numbers(stage);
            if self.balls >= rules.count.balls {
                self.phase = Phase::Walking {
                    left: rules.hit.walk_wait,
                };
            } else if self.anyone_stealing() {
                // The catcher has the ball, and a runner to throw out.
                self.show_steal(at_bat, game, stage, library);
            } else {
                self.ready(&parts, stage, library);
            }
            return;
        }
        let thrown = match at_bat.swing {
            Some(_) => Thrown::Swinging,
            None => Thrown::Called,
        };
        self.book_pitch(at_bat, thrown);
        self.strikes += 1;
        if at_bat.golden {
            // A strike on a golden ball is all the strikes there are.
            self.strikes = self.strikes.max(self.strikes_allowed(game));
        }
        self.streak = 0;
        self.cool(game);
        if let Some(anim) = &parts.strike_anim {
            let label = format!("strike{}", self.strikes.min(3));
            stage.goto_label(anim, &label, false, library);
            // The badge plays for 69 frames and is then taken down. Left
            // up, it would play again and again over the scoreboard.
            self.put_away.push((anim.clone(), 68));
        }
        if let Some(board) = &parts.scoreboard {
            self.play_section(board, "strike", 136, stage, library);
        }
        if self.strikes >= self.strikes_allowed(game) {
            let call = ["1", "2", "3"][self.rng.below(3) as usize];
            Match::sound(stage, library, &format!("umpire_yourOuttaHere_{call}"));
            Match::sound(stage, library, "crowd_unhappy");
            if let Some(batter) = self.batter() {
                self.runners[batter].place = Place::Out;
            }
            self.outs += 1;
            self.rally = 0;
            self.clear_count();
            self.announce = true;
            self.book_end(End::Strikeout, None);
        } else {
            Match::sound(stage, library, "umpire_Strike_grunt");
            if self.strikes + 1 == self.strikes_allowed(game) {
                let organ = ["baseball_organ_FX", "baseball_organ_tense_FX"];
                Match::sound(stage, library, organ[self.rng.below(2) as usize]);
            }
        }
        self.show_numbers(stage);
        if self.anyone_stealing() && self.outs < self.max_outs {
            return self.show_steal(at_bat, game, stage, library);
        }
        // With the side out, nobody has anywhere to steal to.
        self.steals_go_back(stage, library);
        // The call is left up for a moment before the next pitch is offered.
        self.phase = Phase::Called { left: 58 };
    }

    /// One frame of the ball leaving the bat, seen from behind the batter.
    fn watch(&mut self, at_bat: &mut AtBat, game: &Game, stage: &mut Stage, library: &Library) {
        let rules = &game.rules;
        let Some(contact) = at_bat.contact else {
            return;
        };
        let parts = &at_bat.parts;
        let (at_point, height, lift) = &mut at_bat.fly;
        at_point.0 -= (at_point.0 - at_bat.fly_target.0) / contact.power;
        at_point.1 -= (at_point.1 - at_bat.fly_target.1) / contact.power;
        *height += *lift;
        if *lift >= -5.0 {
            *lift -= rules.hit.gravity;
        }
        if *height < 0.0 {
            *height = 0.0;
            *lift = -*lift * rules.hit.bounce;
        }
        let size = (1.0 + (at_point.1 - parts.ground_y) / 190.0).max(0.05);
        put(stage, &parts.fly, *at_point, size);
        let across = at(stage, &parts.fly_ball).0;
        put(stage, &parts.fly_ball, (across, -*height), at_bat.fly_size);
        if let Some(left) = at_bat.run_in {
            if left == 0 {
                at_bat.run_in = None;
                stage.goto_label(&parts.hitter, "run", true, library);
                if self.southpaw {
                    southpaw::run(parts, stage, library);
                }
                let last = stage
                    .clip(&parts.hitter)
                    .map_or(1, |clip| clip.frame_count(library));
                self.cues.push(Cue {
                    path: parts.hitter.clone(),
                    frame: last,
                    rewind: false,
                });
            } else {
                at_bat.run_in = Some(left - 1);
            }
        }
        // The ball is already on its way over the field, out of sight.
        let ground = parts.ground(&rules.field);
        let mut at_wall = None;
        if let Some(ball) = &mut at_bat.ball {
            let happened = ball.step(parts.home, contact.miss(), &rules.field);
            if happened == Happened::Cleared {
                at_bat.over_wall = true;
            }
            if matches!(happened, Happened::Cleared | Happened::HitWall) {
                at_wall = Some((ground.across(ball.at), ball.height));
            }
        }
        if let Some((across, height)) = at_wall {
            self.strike_sign(at_bat, across, height, &rules.sign);
        }
    }

    /// The play is over: offers the next pitch.
    pub(crate) fn ready(&mut self, parts: &Parts, stage: &mut Stage, library: &Library) {
        if self.phase == Phase::Ready {
            return;
        }
        self.phase = Phase::Ready;
        stage.goto_clip(&parts.next, 2, library);
        self.show_numbers(stage);
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
                    stage.goto_clip(flare, 2, library);
                    if let Some(clip) = stage.clip_mut(flare) {
                        clip.playing = true;
                    }
                }
                self.phase = Phase::Leaving { left: 12 };
            }
            (_, ButtonEvent::Press) => self.runner_button(*symbol, path, game, stage, library),
            _ => {}
        }
    }

    pub fn describe(&self) -> String {
        let bases: String = (1..=3)
            .map(|base| {
                if self.on_base(base).is_some() {
                    'x'
                } else {
                    '-'
                }
            })
            .collect();
        // Where this pitch crosses and how many frames it takes, which a
        // script needs to know to time a swing. With the timing bar up, the
        // steps it shows as the best to swing on are given too, and how far
        // the ball went if it was hit for a zinger.
        let pitch = self.at.as_ref().map_or(String::new(), |at_bat| {
            let mut zinger = at_bat
                .zinger
                .map(|zinger| format!(", a zinger of {} feet", zinger.feet))
                .unwrap_or_default();
            if let Some(kind) = at_bat.kind {
                zinger += &format!(", mystery {}", kind.words().to_lowercase());
            }
            if at_bat.golden {
                zinger += ", golden";
            }
            if at_bat.rebounds > 0 {
                zinger += &format!(", rebounds {}", at_bat.rebounds);
            }
            if let Some(called) = &at_bat.called {
                zinger += &format!(", called {:.0},{:.0}", called.at.0, called.at.1);
            }
            if let Some(down) = at_bat.came_down {
                zinger += &format!(", came down at {:.0},{:.0}", down.0, down.1);
            }
            let best = at_bat
                .timing
                .as_ref()
                .and_then(|bar| bar.timing.best())
                .map(|(first, last)| format!(", best swung on steps {first} to {last}"))
                .unwrap_or_default();
            format!(
                ", crossing {:.0},{:.0} after {} frames{}{best}{zinger}",
                at_bat.pitch.crosses.0,
                at_bat.pitch.crosses.1,
                at_bat.pitch.samples.len(),
                if at_bat.pitch.in_zone {
                    ""
                } else {
                    " outside the zone"
                },
            )
        });
        if let Some(arcade) = &self.arcade {
            let meter = self.bullet.map_or(String::new(), |left| {
                let slowed = if self.slowed { " slowed" } else { "" };
                format!(", bullet time {left}{slowed}")
            });
            return format!(
                "{:?}, {} points, {} pitches left{pitch}{meter}",
                self.phase, arcade.points, arcade.left
            );
        }
        // How often the fielders have let the ball go, once they have.
        let mut let_go = match self.slips {
            0 => String::new(),
            times => format!(", let go {times}"),
        };
        if self.heat > 0 {
            let_go += &format!(", heat {}", self.heat);
        }
        if self.streak > 0 {
            let_go += &format!(", hits in a row {}", self.streak);
        }
        if self.rallying && self.rally > 0 {
            let_go += &format!(", rally {}", self.rally);
        }
        if self.clutch {
            let_go += ", clutch";
        }
        if self.southpaw {
            let_go += ", southpaw";
        }
        if let Some(left) = self.bullet {
            let_go += &format!(", bullet time {left}");
            if self.slowed {
                let_go += " slowed";
            }
        }
        if let Some((_, lit)) = self.sign {
            let_go += &format!(", sign {} lit", lit + 1);
        }
        if let Some((sign, runs)) = self.sign_struck.or(self.sign_news) {
            let_go += &format!(", struck sign {} for {runs}", sign + 1);
        }
        for runner in &self.runners {
            if let (Some(_), Some(to)) = (runner.stole_from, runner.running_to) {
                let_go += &format!(", stealing {to}");
            }
        }
        if self.stolen + self.caught > 0 {
            let_go += &format!(", stolen {}, caught {}", self.stolen, self.caught);
        }
        if let Some(tired) = self.tired {
            let_go += &format!(", arm {} tired {tired:.2}", self.arm);
            if self.relieved > 0 {
                let_go += &format!(", pitcher {}", self.relieved + 1);
            }
        }
        if self.shift != 0.0 {
            let way = if self.shift < 0.0 { "left" } else { "right" };
            let_go += &format!(", shifted {way} {:.2}", self.shift.abs());
        }
        // A full match has no score to reach: it says which innings it
        // is, and what both sides have made.
        let score = match &self.full {
            Some(full) => format!(
                "{}, score {} to {}",
                full.batting_in(),
                self.score,
                full.theirs()
            ),
            None => format!("score {} of {}", self.score, self.target),
        };
        // And, at the end, what each side made in every innings so far.
        if let Some(full) = &self.full {
            let_go += &format!(", {}", full.describe());
        }
        format!(
            "{:?}, {score}, outs {}, count {}-{}, bases {bases}, pitched {}{pitch}{let_go}",
            self.phase, self.outs, self.balls, self.strikes, self.pitched
        )
    }
}

/// The screen a finished match leads to.
pub fn result_screen(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Won => "matchWon",
        Outcome::Lost => "matchLost",
        Outcome::Tied | Outcome::Interval => "inningsTied",
        Outcome::ArcadeOver => "arcadeFinish",
    }
}

/// Used by the match screen's own art to find the shell, kept here so that
/// the wiring is in one place.
pub fn shell(stage: &Stage) -> Option<Path> {
    art::shell(stage)
}
