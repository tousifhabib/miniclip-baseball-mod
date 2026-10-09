//! Plays a great many games by written steps and checks that each still goes
//! exactly as it did when it was last written down.
//!
//! This is the net under any change that is meant to leave the game as it
//! was. Every frame of every game, three things are added to running sums:
//! what the game said of itself, what was heard, and everything on the
//! stage. Every so many frames the sums are compared with the ones written
//! down in the folder beside this file. A game that has gone differently
//! fails, and says which game, by which frame, and in which of the three.
//!
//! - `cargo test -p bb-game --test as_it_was` plays the start of every game.
//! - `BB_WHOLE=1` plays each to its end. `scripts/as-it-was.sh` does that
//!   with the game built to run fast.
//! - `BB_WRITE_DOWN=1` writes down how the games go now, in place of what
//!   was written before. Only a change that is meant to alter the game
//!   should need it, and it should say so.
//! - `BB_TRACES=folder`, with `BB_ONLY=part of a name` and perhaps
//!   `BB_FROM=frame` and `BB_TO=frame`, writes out in full what was taken in
//!   of those frames of those games, to compare by eye with the same from
//!   another version of the game. `scripts/what-changed.sh` does both and
//!   shows where they part.
//!
//! The triangles the art is cut into are written down beside the games, and
//! checked the same way: see `triangles.rs`.
//!
//! What is written down is for one kind of machine, since a few of the
//! game's sums are done by the machine's own mathematics. Where nothing is
//! written down for the machine in hand, each game is played twice and the
//! two are checked against each other.
//!
//! These tests need the extracted art. Where it is missing they pass
//! without checking anything, and say so.

mod games;
mod players;
mod seeing;
mod sums;
mod triangles;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};

use bb_engine::app::Runner;
use bb_engine::library::Library;
use bb_engine::stage::Stage;
use bb_game::baseball::{Baseball, Screen};
use bb_game::rules::Rules;
use bb_game::script::Script;

use games::Game;
use seeing::{Listening, Sounds};
use sums::{Page, Sink, Sum};

/// How many frames go by between one writing down of the sums and the next.
const EVERY: u32 = 600;
/// How many frames of each game are played unless the whole is asked for.
const THE_START: u32 = 1_800;

/// The folder that holds the art, if it is there.
fn extracted() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted");
    dir.join("manifest.json").exists().then_some(dir)
}

/// Where a group's sums are written down for this kind of machine.
fn record(group: &str) -> PathBuf {
    let machine = format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS);
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/as_it_was")
        .join(machine)
        .join(format!("{group}.txt"))
}

fn asked(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

/// The sums of a game after so many frames of it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Line {
    frame: u32,
    said: String,
    heard: String,
    seen: String,
}

/// How a game went: its sums, and what it said of itself at the last.
#[derive(Debug, PartialEq, Eq)]
struct Played {
    lines: Vec<Line>,
    ended: String,
}

/// Which frames of which games are to be written out in full, and where.
struct Traces {
    dir: PathBuf,
    only: String,
    from: u32,
    to: u32,
}

impl Traces {
    fn asked() -> Option<Traces> {
        let frame = |name: &str| asked(name).map(|frame| frame.parse().expect("a frame number"));
        Some(Traces {
            dir: PathBuf::from(asked("BB_TRACES")?),
            only: asked("BB_ONLY").unwrap_or_default(),
            from: frame("BB_FROM").unwrap_or(0),
            to: frame("BB_TO").unwrap_or(u32::MAX),
        })
    }

    fn wants(&self, game: &Game) -> bool {
        game.name.contains(&self.only)
    }
}

/// A fact goes to both.
struct Both<'a, A, B>(&'a mut A, &'a mut B);

impl<A: Sink, B: Sink> Sink for Both<'_, A, B> {
    fn number(&mut self, name: &str, number: u64) {
        self.0.number(name, number);
        self.1.number(name, number);
    }

    fn float(&mut self, name: &str, float: f32) {
        self.0.float(name, float);
        self.1.float(name, float);
    }

    fn words(&mut self, name: &str, words: &str) {
        self.0.words(name, words);
        self.1.words(name, words);
    }

    fn next(&mut self) {
        self.0.next();
        self.1.next();
    }
}

/// Starts a game with no window, as the other tests do, with every sound it
/// asks for listened to.
fn start(game: &Game) -> Option<(Script, Sounds)> {
    let library = Library::load(&extracted()?).expect("loading the extracted art");
    let stage = Stage::new(None, &library);
    let mut baseball = Box::new(Baseball::new(&library));
    baseball.start_on(Screen::from_label(game.screen).expect("a screen with that label"));
    baseball.seed(game.seed);
    if !game.rules.is_empty() {
        let rules = Rules::layered(&[("the game's own rules", game.rules)]);
        baseball.play_by(rules.expect("rules that read"));
    }
    for &which in &game.mods {
        baseball.switch_mod(which, true);
    }
    for &(which, level) in &game.levels {
        baseball.set_mod_level(which, level);
    }
    if let Some(ground) = game.ground {
        baseball.play_on(ground);
    }
    let sounds = Sounds::default();
    let listening = Box::new(Listening {
        game: baseball,
        sounds: sounds.clone(),
    });
    let runner = Runner::new(library, stage, listening, None);
    let script = Script::new(runner).expect("a renderer with no window");
    Some((script, sounds))
}

/// Plays up to `most` frames of a game and returns its sums, every so many
/// frames and at its end. `None` when there is no art to play.
fn play(game: &Game, most: u32, traces: Option<&Traces>) -> Option<Played> {
    let (mut script, sounds) = start(game)?;
    let mut player = game.player();
    let (mut said, mut heard, mut seen) = (Sum::new(), Sum::new(), Sum::new());
    let traces = traces.filter(|traces| traces.wants(game));
    let mut pages = String::new();
    let mut lines = Vec::new();
    let mut frame = 0;
    let line = |frame, script: &mut Script, said: &Sum, heard: &Sum, seen: &mut Sum| {
        // The list the `tree` step gives is taken in too, so that it is
        // kept as it was along with everything else.
        seeing::tree(script, seen);
        Line {
            frame,
            said: said.written(),
            heard: heard.written(),
            seen: seen.written(),
        }
    };
    while frame < most && player.frame(&mut script) {
        frame += 1;
        match traces.filter(|traces| (traces.from..=traces.to).contains(&frame)) {
            Some(_) => {
                let mut page = Page::default();
                writeln!(page.0, "== frame {frame} ==\n-- said --").expect("writing to a string");
                seeing::said(&script, &mut Both(&mut said, &mut page));
                page.0.push_str("-- heard --\n");
                seeing::heard(&mut script, &sounds, &mut Both(&mut heard, &mut page));
                page.0.push_str("-- seen --\n");
                seeing::seen(&script, &mut Both(&mut seen, &mut page));
                pages.push_str(&page.0);
            }
            None => {
                seeing::said(&script, &mut said);
                seeing::heard(&mut script, &sounds, &mut heard);
                seeing::seen(&script, &mut seen);
            }
        }
        if frame.is_multiple_of(EVERY) {
            lines.push(line(frame, &mut script, &said, &heard, &mut seen));
        }
    }
    if !frame.is_multiple_of(EVERY) {
        lines.push(line(frame, &mut script, &said, &heard, &mut seen));
    }
    if let Some(traces) = traces {
        std::fs::create_dir_all(&traces.dir).expect("making the folder for the traces");
        let file = traces
            .dir
            .join(format!("{}.txt", game.name.replace([' ', ','], "_")));
        std::fs::write(&file, pages).expect("writing a trace");
        eprintln!("as it was: wrote {}", file.display());
    }
    Some(Played {
        lines,
        ended: script.runner.describe(),
    })
}

/// How many games are being played at this moment, by every test here. Each
/// holds the whole of the art in memory, so only as many are played at once
/// as the machine has processors.
static PLAYING: Mutex<usize> = Mutex::new(0);
static ONE_ENDED: Condvar = Condvar::new();

fn at_once() -> usize {
    std::thread::available_parallelism().map_or(4, usize::from)
}

fn when_there_is_room<T>(play: impl FnOnce() -> T) -> T {
    let mut playing = PLAYING.lock().expect("the count of games");
    while *playing >= at_once() {
        playing = ONE_ENDED.wait(playing).expect("the count of games");
    }
    *playing += 1;
    drop(playing);
    let played = play();
    *PLAYING.lock().expect("the count of games") -= 1;
    ONE_ENDED.notify_one();
    played
}

/// Plays every game, several at a time, each for as many frames as `most`
/// says, and returns how they went in the order the games were given.
fn play_all(
    games: &[Game],
    most: impl Fn(&Game) -> u32 + Sync,
    traces: Option<&Traces>,
) -> Vec<Option<Played>> {
    let next = AtomicUsize::new(0);
    let played: Vec<Mutex<Option<Played>>> = games.iter().map(|_| Mutex::new(None)).collect();
    std::thread::scope(|scope| {
        for _ in 0..at_once().min(games.len()) {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(game) = games.get(index) else {
                        return;
                    };
                    let game = when_there_is_room(|| play(game, most(game), traces));
                    *played[index].lock().expect("a game's sums") = game;
                }
            });
        }
    });
    played
        .into_iter()
        .map(|game| game.into_inner().expect("a game's sums"))
        .collect()
}

/// Reads what was written down for a group: each game's lines, by its name.
fn read(group: &str) -> Option<BTreeMap<String, Vec<Line>>> {
    let text = std::fs::read_to_string(record(group)).ok()?;
    let mut games: BTreeMap<String, Vec<Line>> = BTreeMap::new();
    for line in text.lines().filter(|line| !line.starts_with('#')) {
        let parts: Vec<&str> = line.split('\t').collect();
        let [name, frame, said, heard, seen] = parts[..] else {
            panic!("a line of {group}.txt is not as it should be: {line}");
        };
        games.entry(name.to_owned()).or_default().push(Line {
            frame: frame.parse().expect("a frame number"),
            said: said.to_owned(),
            heard: heard.to_owned(),
            seen: seen.to_owned(),
        });
    }
    Some(games)
}

fn write(group: &str, games: &[Game], played: &[Option<Played>]) {
    let mut text = String::from(
        "# How these games went when they were last written down: each game's sums of what\n\
         # was said, heard and seen, every so many frames. See tests/as_it_was/main.rs.\n",
    );
    for (game, played) in games.iter().zip(played) {
        let Some(played) = played else {
            continue;
        };
        // What each game came to is said, so that whoever writes them down
        // can see that they got as far as they were meant to.
        eprintln!(
            "as it was: `{}` after {} frames: {}",
            game.name,
            played.lines.last().map_or(0, |line| line.frame),
            played.ended
        );
        for line in &played.lines {
            let Line {
                frame,
                said,
                heard,
                seen,
            } = line;
            writeln!(text, "{}\t{frame}\t{said}\t{heard}\t{seen}", game.name)
                .expect("writing to a string");
        }
    }
    let file = record(group);
    std::fs::create_dir_all(file.parent().expect("a folder")).expect("making the folder");
    std::fs::write(&file, text).expect("writing down the sums");
    eprintln!("as it was: wrote {}", file.display());
}

/// Where a game stopped going as it was written down, if it did.
fn changed(game: &Game, written: &[Line], played: &[Line], whole: bool) -> Option<String> {
    let mut agreed = 0;
    for (now, then) in played.iter().zip(written) {
        if now == then {
            agreed = now.frame;
            continue;
        }
        let what = if now.frame != then.frame {
            format!(
                "it ended after {} frames, where it had ended after {}",
                now.frame, then.frame
            )
        } else {
            let which = [
                ("said", &now.said, &then.said),
                ("heard", &now.heard, &then.heard),
                ("seen", &now.seen, &then.seen),
            ]
            .iter()
            .filter(|(_, now, then)| now != then)
            .map(|(what, _, _)| *what)
            .collect::<Vec<_>>()
            .join(", ");
            format!("by frame {} what was {which} had changed", now.frame)
        };
        return Some(format!(
            "`{}` went as it was written down for {agreed} frames, and {what}.\n    \
             To see how: scripts/what-changed.sh <a commit that was right> '{}' {agreed} {}",
            game.name, game.name, now.frame
        ));
    }
    // Fewer lines than were written down is as it should be only when the
    // start alone was played, and played to where the start ends.
    let cut_short = played.len() < written.len()
        && (whole
            || played
                .last()
                .is_none_or(|last| !last.frame.is_multiple_of(EVERY)));
    (cut_short || played.len() > written.len()).then(|| {
        format!(
            "`{}` was written down for {} frames and has now been played for {}.",
            game.name,
            written.last().map_or(0, |line| line.frame),
            played.last().map_or(0, |line| line.frame),
        )
    })
}

/// Plays a group of games and checks them against what is written down, or
/// writes them down afresh if that was asked for.
fn check(group: &str, games: Vec<Game>) {
    if extracted().is_none() {
        assert!(asked("CI").is_none(), "there is no extracted art to play");
        eprintln!("skipped: there is no extracted art to play");
        return;
    }
    if let Some(traces) = Traces::asked() {
        // Only the games asked for are played, as far as was asked, and
        // nothing is compared: writing them out is all this run is for.
        let wanted: Vec<Game> = games
            .into_iter()
            .filter(|game| traces.wants(game))
            .collect();
        play_all(&wanted, |game| game.most.min(traces.to), Some(&traces));
        return;
    }
    let writing = asked("BB_WRITE_DOWN").is_some();
    let whole = writing || asked("BB_WHOLE").is_some();
    let most = |game: &Game| {
        if whole {
            game.most
        } else {
            game.most.min(THE_START)
        }
    };
    let played = play_all(&games, most, None);
    if writing {
        write(group, &games, &played);
        return;
    }
    let Some(written) = read(group) else {
        assert!(
            asked("CI").is_none(),
            "nothing is written down for this kind of machine: {}",
            record(group).display()
        );
        eprintln!(
            "as it was: nothing is written down at {}, so each game is played twice",
            record(group).display()
        );
        let again = play_all(&games, most, None);
        let differing: Vec<&str> = games
            .iter()
            .zip(played.iter().zip(&again))
            .filter(|(_, (once, twice))| once != twice)
            .map(|(game, _)| game.name.as_str())
            .collect();
        assert!(
            differing.is_empty(),
            "played twice, these did not go the same way both times: {differing:?}"
        );
        return;
    };
    let mut wrong = Vec::new();
    for (game, played) in games.iter().zip(&played) {
        let played = played.as_ref().map_or(&[][..], |played| &played.lines);
        match written.get(&game.name) {
            Some(written) => wrong.extend(changed(game, written, played, whole)),
            None => wrong.push(format!("`{}` has never been written down.", game.name)),
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {} games did not go as they were written down in {}:\n\n{}\n\n\
         If the game was meant to change, write it down afresh with BB_WRITE_DOWN=1 \
         scripts/as-it-was.sh, in a commit that says why.\n",
        wrong.len(),
        games.len(),
        record(group).display(),
        wrong.join("\n\n"),
    );
}

#[test]
fn the_last_innings_goes_as_it_did_with_each_mod() {
    check("last-innings", games::last_innings());
}

#[test]
fn the_arcade_game_goes_as_it_did_with_each_mod() {
    check("arcade", games::arcade());
}

#[test]
fn a_full_match_at_home_goes_as_it_did_with_each_mod() {
    check("full-match-at-home", games::full_match_at_home());
}

#[test]
fn a_full_match_away_goes_as_it_did_with_each_mod() {
    check("full-match-away", games::full_match_away());
}

#[test]
fn mods_that_meet_go_together_as_they_did() {
    check("mods-together", games::mods_together());
}

#[test]
fn a_mod_set_to_its_least_and_its_most_goes_as_it_did() {
    check("levels", games::levels());
}

#[test]
fn the_easy_and_the_hard_game_go_as_they_did() {
    check("skills", games::skills());
}

#[test]
fn the_menu_goes_as_it_did() {
    check("menu", games::the_menu());
}

#[test]
fn a_match_played_to_its_end_and_its_pages_go_as_they_did() {
    check("finished-matches", games::finished_matches());
}

#[test]
fn a_monkey_at_the_controls_gets_what_it_got() {
    check("monkeys", games::monkeys());
}

#[test]
fn whole_matches_go_as_they_did() {
    // Nine innings take too long to play every time.
    let asked_for = ["BB_WHOLE", "BB_WRITE_DOWN", "BB_TRACES"];
    if asked_for.iter().all(|name| asked(name).is_none()) {
        return;
    }
    check("whole-matches", games::whole_matches());
}

#[test]
fn the_art_is_cut_into_the_triangles_it_was() {
    let Some(dir) = extracted() else {
        assert!(asked("CI").is_none(), "there is no extracted art to cut up");
        eprintln!("skipped: there is no extracted art to cut up");
        return;
    };
    let library = Library::load(&dir).expect("loading the extracted art");
    let cut = triangles::sums(&library);
    let file = record("triangles");
    if asked("BB_WRITE_DOWN").is_some() {
        let mut text = String::from(
            "# What each shape, text, text field and morph shape of the art was cut into when\n\
             # this was last written down: a sum of its triangles and their paints.\n",
        );
        for (what, sum) in &cut {
            writeln!(text, "{what}\t{sum}").expect("writing to a string");
        }
        std::fs::create_dir_all(file.parent().expect("a folder")).expect("making the folder");
        std::fs::write(&file, text).expect("writing down the sums");
        eprintln!("as it was: wrote {}", file.display());
        return;
    }
    let Ok(text) = std::fs::read_to_string(&file) else {
        assert!(
            asked("CI").is_none(),
            "nothing is written down for this kind of machine: {}",
            file.display()
        );
        eprintln!(
            "as it was: nothing is written down at {}, so the art is cut up twice",
            file.display()
        );
        assert!(
            cut == triangles::sums(&library),
            "cut up twice, it came out two ways"
        );
        return;
    };
    let written: Vec<(&str, &str)> = text
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.split_once('\t'))
        .collect();
    let now: Vec<(&str, &str)> = cut
        .iter()
        .map(|(what, sum)| (what.as_str(), sum.as_str()))
        .collect();
    let then: BTreeMap<&str, &str> = written.iter().copied().collect();
    let changed: Vec<&str> = now
        .iter()
        .filter(|(what, sum)| then.get(what) != Some(sum))
        .map(|(what, _)| *what)
        .collect();
    assert!(
        changed.is_empty() && now.len() == written.len(),
        "{} of {} things are no longer cut into the triangles written down in {} \
         (which has {}): {:?}",
        changed.len(),
        now.len(),
        file.display(),
        written.len(),
        &changed[..changed.len().min(20)],
    );
}
