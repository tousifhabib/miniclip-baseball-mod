//! The zinger hit mod: every ball the bat meets is a home run, and the
//! better the swing was timed the further it goes.

use crate::common::pitch_seen;
mod distance;
mod scoring;
mod shown;

#[path = "../common/mod.rs"]
mod common;

use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{game_modded, number, said};

/// The mods a game is played with here: the timing bar always, which says
/// when to swing, and the zinger or not.
fn mods(zinger: bool) -> Vec<Mod> {
    let mut mods = vec![Mod::TimingIndicator];
    if zinger {
        mods.push(Mod::ZingerHit);
    }
    mods
}

fn game(screen: &str, seed: u64, zinger: bool) -> Option<Script> {
    game_modded(screen, seed, &mods(zinger))
}

/// How far the state says the ball went, if it was hit for a zinger.
fn feet(state: &str) -> Option<u32> {
    number(state, "a zinger of ").map(|feet| feet as u32)
}

/// The feet a line such as "850 FT" gives.
fn feet_said(script: &Script) -> Option<u32> {
    let said = said(script, "zingerFeet");
    number(&format!("={}", said.first()?), "=").map(|feet| feet as u32)
}

/// Where each of the nine fielders is on the field.
fn fielders(script: &Script) -> Vec<(f32, f32)> {
    let stage = &script.runner.stage;
    let Some(main) = stage.find_named(&[], "gameMain") else {
        return Vec::new();
    };
    (1..=9)
        .filter_map(|number| {
            let path = stage.find(&main, &["field", &format!("fielder{number}")])?;
            let fielder = stage.child(&path)?;
            Some((fielder.matrix.tx, fielder.matrix.ty))
        })
        .collect()
}

/// Ways to hold the ring that spoil a hit as the game was, each with a
/// game whose first pitch it spoils: far above the ball, which tops it into
/// the ground, far below, which skies it, and far to either side, which
/// sends it foul past one line or the other.
const SPOILT: [(u64, (f32, f32)); 4] = [
    (1, (0.0, -80.0)),
    (1, (0.0, 80.0)),
    (1, (-140.0, 0.0)),
    (4, (170.0, 0.0)),
];

/// Plays the first pitch of a match with the ring held `off`, timed as
/// well as can be. Returns how many frames after the swing the run was
/// scored, the furthest the count had got while the score stood at
/// nothing, and the state after.
fn timed_to_the_run(off: (f32, f32)) -> Option<(u32, u32, String)> {
    let mut script = game("match", 1, true)?;
    let (mut since, mut scored, mut counted) = (0, None, 0);
    let after = pitch_seen(&mut script, 0, off, |script, now| {
        if feet(now).is_none() {
            return;
        }
        since += 1;
        if now.contains("score 0 of") {
            counted = counted.max(feet_said(script).unwrap_or(0));
        } else if scored.is_none() {
            scored = Some(since);
        }
    });
    Some((scored.expect("a run to be scored"), counted, after))
}
