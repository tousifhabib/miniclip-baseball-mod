//! Playing the games of the record: each from its start, a frame at a
//! time, with the sums taken as it goes.

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

use super::{EVERY, Line, asked, extracted};
use crate::games::Game;
use crate::seeing;
use crate::seeing::{Listening, Sounds};
use crate::sums::{Page, Sink, Sum};

/// How a game went: its sums, and what it said of itself at the last.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Played {
    pub(super) lines: Vec<Line>,
    pub(super) ended: String,
}

/// Which frames of which games are to be written out in full, and where.
pub(super) struct Traces {
    dir: PathBuf,
    only: String,
    from: u32,
    pub(super) to: u32,
}

impl Traces {
    pub(super) fn asked() -> Option<Traces> {
        let frame = |name: &str| asked(name).map(|frame| frame.parse().expect("a frame number"));
        Some(Traces {
            dir: PathBuf::from(asked("BB_TRACES")?),
            only: asked("BB_ONLY").unwrap_or_default(),
            from: frame("BB_FROM").unwrap_or(0),
            to: frame("BB_TO").unwrap_or(u32::MAX),
        })
    }

    pub(super) fn wants(&self, game: &Game) -> bool {
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
pub(super) fn play(game: &Game, most: u32, traces: Option<&Traces>) -> Option<Played> {
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
pub(super) fn play_all(
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
