//! Plays the baseball game, in a window or from a script.

use std::path::PathBuf;
use std::process::{Command, ExitCode};

use anyhow::{Context, Result};
use bb_engine::app::Runner;
use bb_engine::audio::Audio;
use bb_engine::library::Library;
use bb_engine::stage::Stage;
use bb_engine::window::{self, Options};
use bb_game::baseball::{Baseball, Screen};
use bb_game::locate;
use bb_game::mods::{Mod, Mods};
use bb_game::scores::Scores;
use bb_game::script::Script;
use bb_game::settings::Ground;
use clap::Parser;

#[derive(Parser)]
#[command(about = "Plays the baseball game")]
struct Args {
    /// The folder that holds the art. Without this, the game looks inside
    /// its own app bundle, then in its folder under Application Support,
    /// then beside the program, then for `extracted` where it was started.
    dir: Option<PathBuf>,
    /// Play no sound.
    #[arg(long)]
    mute: bool,
    /// Open with the inspector showing.
    #[arg(long)]
    inspect: bool,
    /// Quit after drawing this many frames. For testing.
    #[arg(long)]
    exit_after: Option<u32>,
    /// Start on this screen instead of the intro, by its label in the art:
    /// `menu`, `match`, `arcade`, `matchWon`, `matchLost`, `inningsTied`,
    /// `arcadeFinish` or `instructionsAll`. `fullMatch` starts a full
    /// match.
    #[arg(long)]
    screen: Option<String>,
    /// Where the side plays a full match: `home`, `away` or `toss`, which
    /// leaves it to a coin.
    #[arg(long, value_name = "WHERE")]
    ground: Option<String>,
    /// Play with no window, following these steps, separated by semicolons:
    /// `wait N`, `click X Y`, `move X Y`, `press`, `release`, `type TEXT`,
    /// `key NAME`, `hold NAME`, `lift NAME`, `state`, `events`, `tree` and
    /// `shot FILE`.
    #[arg(long)]
    run: Option<String>,
    /// Make every game go the same way: the number its chances are worked
    /// out from.
    #[arg(long)]
    seed: Option<u64>,
    /// Switch a mod on for this run, by its name, such as `zinger_hit`:
    /// the README lists them. A mod with a setting takes its level after
    /// an equals sign, as in `butterfingers=5`. May be given more than
    /// once.
    #[arg(long = "mod", value_name = "NAME")]
    mods: Vec<String>,
    /// With `--run`: picture pixels per stage pixel.
    #[arg(long, default_value_t = 1.0)]
    scale: f32,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let message = format!("{error:#}");
            eprintln!("Error: {message}");
            // An app has no terminal for that to be read in.
            if locate::in_app_bundle() {
                alert(&message);
            }
            ExitCode::FAILURE
        }
    }
}

/// Puts up a system alert saying `message`, and waits for it to be
/// dismissed.
fn alert(message: &str) {
    // AppleScript strings escape only the backslash and the double quote.
    let quoted = message.replace('\\', "\\\\").replace('"', "\\\"");
    let script =
        format!("display alert \"The game could not start\" message \"{quoted}\" as critical");
    // If this fails too there is nothing more to be done about it.
    let _ = Command::new("/usr/bin/osascript")
        .args(["-e", &script])
        .status();
}

fn run() -> Result<()> {
    let args = Args::parse();
    let library = Library::load(&locate::find(args.dir.as_deref())?)?;
    let mut logic = Box::new(Baseball::new(&library));
    let stage = Stage::new(None, library);
    if let Some(seed) = args.seed {
        logic.seed(seed);
    }
    // A scripted run is a test, and leaves the player's own table and
    // choice of mods alone.
    if args.run.is_none() {
        if let Some(file) = Scores::usual_file() {
            logic.keep_scores_in(file);
        }
        if let Some(file) = Mods::usual_file() {
            logic.keep_mods_in(file);
        }
    }
    for asked in &args.mods {
        let (name, level) = match asked.split_once('=') {
            Some((name, level)) => (name, Some(level)),
            None => (asked.as_str(), None),
        };
        let which = Mod::from_key(name).with_context(|| {
            let known: Vec<&str> = Mod::ALL.iter().map(|each| each.key()).collect();
            format!(
                "there is no mod called `{name}`: the mods are {}",
                known.join(", ")
            )
        })?;
        logic.switch_mod(which, true);
        if let Some(level) = level {
            let level = level
                .parse()
                .with_context(|| format!("`{level}` is not a level for the mod `{name}`"))?;
            logic.set_mod_level(which, level);
        }
    }
    if let Some(word) = &args.ground {
        let ground = Ground::from_word(word)
            .with_context(|| format!("`{word}` is not home, away or toss"))?;
        logic.play_on(ground);
    }
    if let Some(label) = &args.screen {
        let screen = Screen::from_label(label)
            .with_context(|| format!("there is no screen called `{label}`"))?;
        logic.start_on(screen);
    }

    if let Some(steps) = &args.run {
        let mut script = Script::new(Runner::new(stage, logic, None))?;
        script.scale = args.scale;
        for line in script.run(steps)? {
            println!("{line}");
        }
        for problem in script.problems() {
            eprintln!("problem: {problem}");
        }
        return Ok(());
    }

    let audio = if args.mute {
        None
    } else {
        // A machine with no sound device can still play, silently.
        Audio::new()
            .inspect_err(|error| eprintln!("Playing without sound: {error:#}"))
            .ok()
    };
    let runner = Runner::new(stage, logic, audio);
    let summary = window::run(
        runner,
        Options {
            title: "Miniclip Baseball (mod)".to_owned(),
            inspect: args.inspect,
            centre_origin: false,
            exit_after: args.exit_after,
        },
    )?;
    for problem in &summary.problems {
        eprintln!("problem: {problem}");
    }
    Ok(())
}
