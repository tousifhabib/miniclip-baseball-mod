//! Whole matches, played by a script that reads where each pitch will cross
//! and swings at it.

mod endings;
mod view;

#[path = "../common/mod.rs"]
mod common;

/// What the state line says about the pitch in hand.
struct Seen {
    phase: String,
    crossing: Option<(f32, f32)>,
    frames: u32,
    in_zone: bool,
}

fn seen(state: &str) -> Seen {
    let phase = state
        .split(": ")
        .nth(1)
        .and_then(|rest| rest.split([' ', ',']).next())
        .unwrap_or_default()
        .to_owned();
    let crossing = state.split("crossing ").nth(1).and_then(|rest| {
        let (x, rest) = rest.split_once(',')?;
        let y = rest.split(' ').next()?;
        Some((x.parse().ok()?, y.parse().ok()?))
    });
    let frames = state
        .split(" after ")
        .nth(1)
        .and_then(|rest| rest.split(' ').next())
        .and_then(|frames| frames.parse().ok())
        .unwrap_or(0);
    Seen {
        phase,
        crossing,
        frames,
        in_zone: !state.contains("outside the zone"),
    }
}

/// Whether the match is still going, or has not yet begun.
fn in_match(state: &str) -> bool {
    state.starts_with("Match,") || state.starts_with("Arcade,") || state.starts_with("Loading")
}

/// Plays a match to its end, swinging at every pitch in the zone, and
/// returns the screen it ended on and how many pitches it took. The ring is
/// held `off` away from where the ball will cross, and the swing comes
/// `swing_early_by` frames before the ball is gone.
fn play_out(seed: u64, swing_early_by: u32, off: (f32, f32)) -> (String, u32) {
    let (end, pitches, _) = play_screen("match", seed, swing_early_by, off);
    (end, pitches)
}

/// The same for any game screen. Also returns the arcade game's points.
fn play_screen(
    screen: &str,
    seed: u64,
    swing_early_by: u32,
    off: (f32, f32),
) -> (String, u32, u32) {
    let Some(mut script) = game_seeded(screen, seed) else {
        return (String::new(), 0, 0);
    };
    let state = |script: &mut bb_game::script::Script| {
        script.run("state").unwrap().pop().unwrap_or_default()
    };
    let mut pitches = 0;
    // Far more frames than any match needs: a play that never ends fails
    // here instead of hanging the test.
    for _ in 0..200_000 {
        let now = state(&mut script);
        if !in_match(&now) {
            let points = script
                .runner
                .stage
                .text("points_total")
                .and_then(|points| points.parse().ok())
                .unwrap_or(0);
            return (now, pitches, points);
        }
        let look = seen(&now);
        match look.phase.as_str() {
            "Settling" => {
                // Put the ring where the ball will cross, and wait for it.
                if let Some((x, y)) = look.crossing {
                    let (x, y) = (x + off.0, y + off.1);
                    script.run(&format!("move {x} {y}")).unwrap();
                }
                script.run("wait 1").unwrap();
            }
            "Flight" => {
                pitches += 1;
                if look.in_zone {
                    let step: u32 = now
                        .split("step: ")
                        .nth(1)
                        .and_then(|rest| rest.split(' ').next())
                        .and_then(|step| step.parse().ok())
                        .unwrap_or(0);
                    let swing_at = look.frames.saturating_sub(swing_early_by);
                    let wait = swing_at.saturating_sub(step);
                    let (x, y) = look.crossing.unwrap();
                    let (x, y) = (x + off.0, y + off.1);
                    script.run(&format!("wait {wait}; click {x} {y}")).unwrap();
                }
                // Let the pitch finish, one way or the other.
                while seen(&state(&mut script)).phase == "Flight" {
                    script.run("wait 1").unwrap();
                }
            }
            "Ready" => {
                // In the arcade game the next pitch is offered while the
                // ball is still in the air, and taking it loses the points.
                if now.starts_with("Arcade") {
                    script.run("wait 400").unwrap();
                }
                script.run("click 545 355; wait 2").unwrap();
            }
            _ => {
                script.run("wait 1").unwrap();
            }
        }
    }
    panic!("the match never ended: {}", state(&mut script));
}

fn game_seeded(screen: &str, seed: u64) -> Option<bb_game::script::Script> {
    common::game_with(screen, Some(seed))
}
