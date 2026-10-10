//! Starting the game: on a screen, by a seed, with mods on, by rules of
//! the test's own.

use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use bb_engine::app::Runner;
use bb_engine::library::Library;
use bb_engine::stage::Stage;
use bb_game::art::all_named;
use bb_game::baseball::{Baseball, Screen};
use bb_game::mods::Mod;
use bb_game::rules::Rules;
use bb_game::script::Script;
use bb_game::settings::Ground;
use bb_game::tournament::Format;

/// The folder that holds the art, if it is there.
fn extracted() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted");
    dir.join("manifest.json").exists().then_some(dir)
}

/// The art, read once for all the tests of a file and played from by each.
/// `None` when there is none to read.
static ART: LazyLock<Option<Arc<Library>>> = LazyLock::new(|| {
    let library = Library::load(&extracted()?).expect("loading the extracted art");
    Some(Arc::new(library))
});

/// The game, opened on the screen with this label. `None` when there is no
/// extracted art to play.
pub fn game(screen: &str) -> Option<Script> {
    game_with(screen, Some(1))
}

/// The game opened on a screen, with its chances worked out from `seed`.
pub fn game_with(screen: &str, seed: Option<u64>) -> Option<Script> {
    game_ruled(screen, seed, None)
}

/// The same, played by `rules` instead of the ones built in.
pub fn game_ruled(screen: &str, seed: Option<u64>, rules: Option<Rules>) -> Option<Script> {
    let start = Start {
        seed,
        rules,
        ..Start::default()
    };
    game_made(screen, start)
}

/// The game opened on a screen, with these mods switched on.
pub fn game_modded(screen: &str, seed: u64, mods: &[Mod]) -> Option<Script> {
    let start = Start {
        seed: Some(seed),
        mods,
        ..Start::default()
    };
    game_made(screen, start)
}

/// A full match with these mods on, played at home or away, by `rules` if
/// any are given.
pub fn full_match(seed: u64, ground: Ground, mods: &[Mod], rules: Option<Rules>) -> Option<Script> {
    let start = Start {
        seed: Some(seed),
        rules,
        mods,
        ground: Some(ground),
        ..Start::default()
    };
    game_made(Screen::FULL_MATCH, start)
}

/// The rules of a full match of this many innings, in which the other side
/// makes `their` runs every time it bats.
pub fn match_to_order(innings: u32, their: usize) -> Rules {
    let chances = format!("[{}1]", "0, ".repeat(their));
    let text = format!(
        "[full_match]\ninnings = {innings}\n[full_match.runs]\n\
         easy = {chances}\nmedium = {chances}\nhard = {chances}\n"
    );
    Rules::layered(&[("a full match to order", &text)]).expect("rules that read")
}

/// The mods of a game played with the timing bar up, which says when to
/// swing, and with `which` as well if it is `on`.
pub fn timing_bar_and(which: Mod, on: bool) -> Vec<Mod> {
    let mut mods = vec![Mod::TimingIndicator];
    if on {
        mods.push(which);
    }
    mods
}

/// A match with these mods on that takes a long time to win or lose, for
/// tests that need a good many pitches: forty runs behind, with thirty outs
/// to get them in.
pub fn long_match(seed: u64, mods: &[Mod]) -> Option<Script> {
    long_match_ruled(seed, mods, "")
}

/// The same, with the numbers in `layer` laid over the rules: what a test
/// changes of them to see a mod's whole effect in a few pitches.
pub fn long_match_ruled(seed: u64, mods: &[Mod], layer: &str) -> Option<Script> {
    let long = "[match]\nouts = 30\n[match.runs_down]\neasy = 40\nmedium = 40\nhard = 40\n";
    let rules = Rules::layered(&[("a long match", long), ("the test's own rules", layer)])
        .expect("rules that read");
    let start = Start {
        seed: Some(seed),
        rules: Some(rules),
        mods,
        ..Start::default()
    };
    game_made("match", start)
}

/// The same, keeping its scores in `scores` and starting from what is
/// there.
pub fn game_keeping(screen: &str, seed: u64, mods: &[Mod], scores: &Path) -> Option<Script> {
    let start = Start {
        seed: Some(seed),
        mods,
        scores: Some(scores),
        ..Start::default()
    };
    game_made(screen, start)
}

/// The game opened on a screen with one mod switched on, its setting at
/// `level`, and the timing bar, which says when to swing.
pub fn game_levelled(screen: &str, seed: u64, which: Mod, level: u8) -> Option<Script> {
    let mods = [Mod::TimingIndicator, which];
    let start = Start {
        seed: Some(seed),
        mods: &mods,
        level: Some((which, level)),
        ..Start::default()
    };
    game_made(screen, start)
}

/// A tournament of this shape, with matches of this many innings, opened
/// on its tables with this many of its fixtures played on paper already,
/// the player's own among them.
pub fn tournament(seed: u64, format: Format, innings: u32, played: usize) -> Option<Script> {
    let start = Start {
        seed: Some(seed),
        tournament: Some((format, innings, played)),
        ..Start::default()
    };
    game_made(Screen::TOURNAMENT, start)
}

/// A tournament of this shape opened on its tables with nothing played,
/// with these mods on. Its matches are of one innings, in which every
/// other side makes one run for each innings' worth it is given, so that
/// a match is soon lost by a side that makes none and won by one that
/// makes two.
pub fn short_tournament(seed: u64, format: Format, mods: &[Mod]) -> Option<Script> {
    let one = "[0, 1]";
    let text = format!(
        "[tournament]\ninnings = [1]\n[full_match.runs]\neasy = {one}\nmedium = {one}\nhard = {one}\n"
    );
    let rules = Rules::layered(&[("a short tournament", &text)]).expect("rules that read");
    let start = Start {
        seed: Some(seed),
        rules: Some(rules),
        mods,
        tournament: Some((format, 1, 0)),
        ..Start::default()
    };
    game_made(Screen::TOURNAMENT, start)
}

/// What a game is started with, besides the screen it opens on. Whatever
/// a test does not set is as the game has it.
#[derive(Default)]
struct Start<'a> {
    /// What the game's chances are worked out from, if not the clock.
    seed: Option<u64>,
    /// The rules it is played by, if not the ones built in.
    rules: Option<Rules>,
    /// The mods that are switched on.
    mods: &'a [Mod],
    /// Where it keeps its scores.
    scores: Option<&'a Path>,
    /// A mod's setting, and the level it is at.
    level: Option<(Mod, u8)>,
    /// Where a full match is played.
    ground: Option<Ground>,
    /// The shape of a tournament, how many innings its matches have, and
    /// how many of its fixtures are played on paper as it is drawn.
    tournament: Option<(Format, u32, usize)>,
}

fn game_made(screen: &str, start: Start<'_>) -> Option<Script> {
    let Some(library) = ART.clone() else {
        eprintln!("skipped: there is no extracted art to play");
        return None;
    };
    let mut logic = Box::new(Baseball::new(&library));
    let stage = Stage::new(None, library);
    logic.start_on(Screen::from_label(screen).expect("a screen with that label"));
    if let Some(seed) = start.seed {
        logic.seed(seed);
    }
    if let Some(rules) = start.rules {
        logic.play_by(rules);
    }
    for &which in start.mods {
        logic.switch_mod(which, true);
    }
    if let Some(scores) = start.scores {
        logic.keep_scores_in(scores.to_owned());
    }
    if let Some((which, level)) = start.level {
        logic.set_mod_level(which, level);
    }
    if let Some(ground) = start.ground {
        logic.play_on(ground);
    }
    if let Some((format, innings, played)) = start.tournament {
        logic.choose_tournament(Some(format), Some(innings));
        logic.play_on_paper(played);
    }
    let runner = Runner::new(stage, logic, None);
    Some(Script::new(runner).expect("a renderer with no window"))
}
