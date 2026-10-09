//! The record itself: where it is kept, reading it, writing it down, and
//! saying where a game stopped going as it was.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use super::playing::{Played, Traces, play_all};
use super::{EVERY, Line, asked, extracted};
use crate::games::Game;

/// How many frames of each game are played unless the whole is asked for.
const THE_START: u32 = 1_800;

/// Where a group's sums are written down for this kind of machine.
pub(super) fn record(group: &str) -> PathBuf {
    let machine = format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS);
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/as_it_was")
        .join(machine)
        .join(format!("{group}.txt"))
}

/// Reads what was written down for a group: each game's lines, by its name.
pub(super) fn read(group: &str) -> Option<BTreeMap<String, Vec<Line>>> {
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

pub(super) fn write(group: &str, games: &[Game], played: &[Option<Played>]) {
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
pub(super) fn changed(
    game: &Game,
    written: &[Line],
    played: &[Line],
    whole: bool,
) -> Option<String> {
    let mut agreed = 0;
    for (now, then) in played.iter().zip(written) {
        if now == then {
            agreed = now.frame;
            continue;
        }
        let what = if now.frame == then.frame {
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
        } else {
            format!(
                "it ended after {} frames, where it had ended after {}",
                now.frame, then.frame
            )
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
pub(super) fn check(group: &str, games: Vec<Game>) {
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
