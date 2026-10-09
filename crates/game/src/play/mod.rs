//! A match in progress: the last innings, batting to overtake the other
//! side, or every innings of a full match.
//!
//! The art builds the batting view afresh for every pitch, so everything
//! that lasts from one pitch to the next is kept here: the score, the count,
//! the outs, and where every runner stands.

mod arcade;
mod batting;
pub mod book;
pub mod field;
mod fielding;
pub mod full;
mod mode;
mod mods;
pub(crate) mod overlay;
pub mod paper;
pub mod pitch;
mod set_up;
mod snapshot;

use bb_engine::display::{ButtonEvent, Content, Event, Path, child_bounds};
use bb_engine::library::Library;
use bb_engine::math::Matrix;
use bb_engine::stage::Stage;
use bb_format::SymbolId;

use crate::look::{Look, Rgb};
use crate::menu::Game;
use crate::rng::Rng;
use crate::rules::{FieldRules, PitchRules};
use book::ORDER;
use field::{Ball, Contact, Ground, reach};
use mode::Mode;
pub(crate) use mods::bullet_time as bullet;
use mods::{
    ModsInPlay, called_shot as called, hit_the_sign as sign, night_game, pinball_park as pinball,
    southpaw, stolen_bases as steal, timing_indicator as timing, zinger_hit as zinger,
};
use overlay::Notices;
use pitch::{Kind, Mound, Pitch, Point, Quality};
use snapshot::{ModsSeen, PitchSeen, Score, Snapshot, Standing};
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
    pub notices: Notices,
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
    /// With the stolen bases mod on: how many bases have been stolen in
    /// this game and how many runners caught at it, whether the play in the
    /// field is one on which a base can be stolen, and how the last try
    /// came out, until that has been told.
    pub(crate) stolen: u32,
    pub(crate) caught: u32,
    pub(crate) steal_play: bool,
    steal_news: Option<(&'static str, Rgb)>,
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
/// How far down the batting view the mystery pitch mod names the pitch,
/// which is between the scoreboard and the pitcher. The tired arm mod says
/// there that a new pitcher has come in.
const MYSTERY_TOP: f32 = 141.0;
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
        let mods = ModsInPlay::for_game(game, seed, false);
        // With every hit a home run there are more runs to get.
        let behind = if mods.every_hit_is_a_home_run() {
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
            mode: Mode::LastInnings,
            mods,
            came_up: 0,
            line_up: Vec::new(),
            tally: Vec::new(),
            outs_before: 0,
            thrown_at: (0, 0),
            run_worth: 1,
            stolen: 0,
            caught: 0,
            steal_play: false,
            steal_news: None,
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

    /// How many strikes put a batter out: three, unless a mod says
    /// otherwise.
    pub(crate) fn strikes_allowed(&self, game: &Game) -> u32 {
        self.mods.strikes_allowed(game.rules.count.strikes)
    }

    /// How many a run counts for on the pitch about to be thrown, which is
    /// a golden ball or is not.
    fn worth_of_a_run(&self, golden: bool) -> u32 {
        self.mods.worth_of_a_run(golden, self.in_the_clutch())
    }

    /// Whether the pitch about to be thrown is one the clutch mod makes
    /// runs count for more on: the side has one out left, and a runner is
    /// on second or third.
    fn in_the_clutch(&self) -> bool {
        let one_out_left = self.outs + 1 == self.max_outs;
        let runner_in_reach_of_home = self.on_base(2).is_some() || self.on_base(3).is_some();
        self.mods
            .in_the_clutch(one_out_left, runner_in_reach_of_home)
    }

    /// The longest zinger of this game, in feet. Nought if there was none.
    pub fn longest_zinger(&self) -> u32 {
        self.mods.longest_zinger()
    }

    /// The longest zinger there has ever been, as far as this game knows.
    pub fn zinger_record(&self) -> u32 {
        self.mods.zinger_record()
    }

    /// Tells the game the record its zingers have to beat.
    pub fn set_zinger_record(&mut self, feet: u32) {
        self.mods.set_zinger_record(feet);
    }

    /// A zinger has come down: the player is told how far it went and
    /// where, and the crowd is heard.
    pub(crate) fn zinger_down(
        &mut self,
        show: &mut zinger::Show,
        stage: &mut Stage,
        library: &Library,
    ) {
        let record = self.mods.a_zinger_went(show.zinger.feet);
        show.landed(record, stage);
        self.mods.a_home_run_was_hit();
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
        match &self.mode {
            Mode::Arcade(arcade) => (arcade.left == 0).then_some(Outcome::ArcadeOver),
            Mode::Full(full) => {
                // Batting last with every innings all but played, to be
                // ahead is to have won. Otherwise the side bats until it is
                // out, and what that comes to is worked out once the other
                // side has batted.
                if full.sudden() && self.score > full.theirs() {
                    Some(Outcome::Won)
                } else {
                    (self.outs >= self.max_outs).then_some(Outcome::Interval)
                }
            }
            Mode::LastInnings => {
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
        let arcade = self.mode.arcade()?;
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
        let shown_target = match self.mode.full() {
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
                self.mode.arcade().map_or(0, |arcade| arcade.points),
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
                if self.held_back(&at_bat, step, stage) {
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

    /// How the game stands, as plain facts.
    pub(crate) fn snapshot(&self) -> Snapshot {
        // Where this pitch crosses and how many frames it takes, which a
        // script needs to know to time a swing. With the timing bar up, the
        // steps it shows as the best to swing on are given too, and how far
        // the ball went if it was hit for a zinger.
        let pitch = self.at.as_ref().map(|at_bat| PitchSeen {
            crosses: at_bat.pitch.crosses,
            frames: at_bat.pitch.samples.len(),
            in_zone: at_bat.pitch.in_zone,
            best: at_bat.timing.as_ref().and_then(|bar| bar.timing.best()),
            zinger_feet: at_bat.zinger.map(|zinger| zinger.feet),
            mystery: at_bat.kind,
            golden: at_bat.golden,
            rebounds: at_bat.rebounds,
            called: at_bat.called.as_ref().map(|called| called.at),
            came_down: at_bat.came_down,
        });
        let stealing = self
            .runners
            .iter()
            .filter(|runner| runner.stole_from.is_some())
            .filter_map(|runner| runner.running_to)
            .collect();
        let mods = ModsSeen {
            let_go: self.mods.let_go(),
            heat: self.mods.heat(),
            hits_in_a_row: self.mods.hits_in_a_row(),
            rally: self.mods.in_a_row(),
            clutch: self.mods.clutch_this_pitch(),
            southpaw: self.mods.batting_left_handed(),
            bullet_time: self.mods.bullet_time(),
            sign_lit: self.mods.sign_lit(),
            sign_struck: self.mods.sign_struck(),
            stealing,
            stolen: self.stolen,
            caught: self.caught,
            arm: self.mods.arm(),
            shifted: self.mods.shifted(),
        };
        let in_a_match = |score: Score, innings: Option<String>| Standing::Match {
            score,
            outs: self.outs,
            balls: self.balls,
            strikes: self.strikes,
            bases: [1, 2, 3].map(|base| self.on_base(base).is_some()),
            pitched: self.pitched,
            innings,
        };
        let standing = match &self.mode {
            Mode::Arcade(arcade) => Standing::Arcade {
                points: arcade.points,
                pitches_left: arcade.left,
            },
            Mode::LastInnings => {
                let score = Score::Of {
                    score: self.score,
                    target: self.target,
                };
                in_a_match(score, None)
            }
            // A full match has no score to reach: it says which innings it
            // is and what both sides have made, and, at the end, what each
            // made in every innings so far.
            Mode::Full(full) => {
                let score = Score::Against {
                    batting_in: full.batting_in(),
                    score: self.score,
                    theirs: full.theirs(),
                };
                in_a_match(score, Some(full.describe()))
            }
        };
        Snapshot {
            phase: self.phase,
            standing,
            pitch,
            mods,
        }
    }

    /// How the game stands, in one line, for a script, a test or an
    /// inspector. The tests read it, so what it says is fixed where the
    /// snapshot is printed.
    pub fn describe(&self) -> String {
        self.snapshot().to_string()
    }
}
