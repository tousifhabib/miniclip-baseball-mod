//! The lone pitcher mod: who goes after a ball in play, and who does not.

mod common;

use bb_game::mods::Mod;
use bb_game::script::Script;
use common::game_modded;

/// The pitcher's place among the fielders, counting from 0: the art's
/// `fielder3`, who stands on the mound.
const PITCHER: usize = 2;
/// The fielder who minds second base: the art's `fielder7`.
const AT_SECOND: usize = 6;

/// The number that follows `before` in a state line.
fn number(state: &str, before: &str) -> Option<f32> {
    let rest = state.split(before).nth(1)?;
    let digits: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    digits.parse().ok()
}

/// Whether the match is still going, or has not yet begun.
fn in_match(state: &str) -> bool {
    state.starts_with("Match,") || state.starts_with("Loading")
}

/// A match being played by a batter who times every swing off the timing
/// bar, with a watch kept on the fielders.
struct Batting {
    script: Script,
    /// For each fielder, whether he has moved or done anything while a ball
    /// was in play.
    stirred: [bool; 9],
    /// How many times the view has changed to the field, and for how many
    /// frames in all.
    plays: u32,
    frames: u32,
}

impl Batting {
    fn new(seed: u64, mods: &[Mod]) -> Option<Batting> {
        let mut with_bar = mods.to_vec();
        with_bar.push(Mod::TimingIndicator);
        Some(Batting {
            script: game_modded("match", seed, &with_bar)?,
            stirred: [false; 9],
            plays: 0,
            frames: 0,
        })
    }

    fn state(&mut self) -> String {
        self.script.run("state").unwrap().pop().unwrap_or_default()
    }

    /// Where each of the nine fielders is on the field, and the frame of
    /// his clip, which is what he is doing.
    fn fielders(&self) -> Vec<(f32, f32, u16)> {
        let stage = &self.script.runner.stage;
        let Some(main) = stage.find_named(&[], "gameMain") else {
            return Vec::new();
        };
        (1..=9)
            .filter_map(|number| {
                let path = stage.find(&main, &["field", &format!("fielder{number}")])?;
                let fielder = stage.child(&path)?;
                let frame = stage.clip(&path)?.frame;
                Some((fielder.matrix.tx, fielder.matrix.ty, frame))
            })
            .collect()
    }

    /// Plays one pitch. The swing begins `late` steps after the first step
    /// the bar calls best, with the ring held `off` away from where the
    /// ball will cross. Returns the state once the play is over: the next
    /// pitch is on offer, or the match has ended.
    fn pitch(&mut self, late: i32, off: (f32, f32)) -> String {
        // How the fielders stood as the play began.
        let mut marks: Option<Vec<(f32, f32, u16)>> = None;
        // Far more frames than any pitch needs: a play that never ends
        // fails here instead of hanging the test.
        for _ in 0..20_000 {
            let now = self.state();
            if !in_match(&now) || now.contains(": Ready") {
                return now;
            }
            if now.contains(": Fielding") {
                self.frames += 1;
                let seen = self.fielders();
                match &marks {
                    None => {
                        self.plays += 1;
                        marks = Some(seen);
                    }
                    Some(marks) => {
                        for (index, (was, is)) in marks.iter().zip(&seen).enumerate() {
                            self.stirred[index] |= was != is;
                        }
                    }
                }
            }
            let ring = number(&now, "crossing ")
                .zip(number(now.split("crossing ").nth(1).unwrap_or(""), ","))
                .map(|(x, y)| (x + off.0, y + off.1));
            let step = number(&now, "Flight { step: ").map(|step| step as i32);
            let best = number(&now, "best swung on steps ").map(|best| best as i32);
            let steps = match (ring, step, best) {
                (Some((x, y)), Some(step), Some(best)) if step == best + late => {
                    format!("click {x} {y}")
                }
                (Some((x, y)), None, _) if now.contains("Settling") => {
                    format!("move {x} {y}; wait 1")
                }
                _ => "wait 1".to_owned(),
            };
            self.script.run(&steps).unwrap();
        }
        panic!("the pitch never ended: {}", self.state());
    }

    /// Asks for the next pitch.
    fn next(&mut self) {
        // The button takes a moment to come up.
        for _ in 0..200 {
            self.script.run("click 545 355; wait 2").unwrap();
            if !self.state().contains(": Ready") {
                return;
            }
        }
        panic!("the next pitch never came: {}", self.state());
    }

    /// Plays the match to its end, swinging at every pitch on a step the
    /// bar calls best. The ring is held a different way off the ball for
    /// each pitch, to send the hits all over the field. Returns the screen
    /// the match ended on.
    fn play_out(&mut self) -> String {
        const OFF: [(f32, f32); 6] = [
            (0.0, -18.0),
            (-25.0, 6.0),
            (25.0, -8.0),
            (-12.0, 14.0),
            (14.0, -24.0),
            (0.0, 4.0),
        ];
        for pitch in 0..500 {
            let now = self.pitch(0, OFF[pitch % OFF.len()]);
            if !in_match(&now) {
                return now;
            }
            self.next();
        }
        panic!("the match never ended: {}", self.state());
    }
}

const SEEDS: [u64; 3] = [1, 2, 3];

#[test]
fn only_the_pitcher_goes_after_the_ball() {
    let mut pitcher_went = false;
    for seed in SEEDS {
        let Some(mut batting) = Batting::new(seed, &[Mod::LonePitcher]) else {
            return;
        };
        let end = batting.play_out();
        assert!(batting.plays > 0, "seed {seed}: nothing was hit");
        for (index, stirred) in batting.stirred.into_iter().enumerate() {
            assert!(
                index == PITCHER || !stirred,
                "seed {seed}: fielder {} did not keep still",
                index + 1
            );
        }
        pitcher_went |= batting.stirred[PITCHER];
        // The match still comes to an end, one way or another.
        assert!(
            ["MatchWon", "MatchLost", "InningsTied"]
                .iter()
                .any(|screen| end.starts_with(screen)),
            "seed {seed}: {end}"
        );
    }
    assert!(pitcher_went);
}

#[test]
fn without_the_mod_whoever_is_nearest_goes() {
    // The same matches as the test above plays, as a check that it would
    // see another fielder going for the ball if one did.
    let mut others = false;
    for seed in SEEDS {
        let Some(mut batting) = Batting::new(seed, &[]) else {
            return;
        };
        batting.play_out();
        let mut stirred = batting.stirred;
        stirred[PITCHER] = false;
        others |= stirred.contains(&true);
    }
    assert!(others);
}

/// A hit that puts a runner on first, and then a weak one back towards the
/// mound: the pitcher has it soon enough to throw the runner out at second
/// while the batter is still on his way to first. Returns the state after
/// the second play.
fn runner_on_first_and_a_weak_hit(batting: &mut Batting) -> String {
    let first = batting.pitch(0, (0.0, -18.0));
    assert!(first.contains("bases x--"), "{first}");
    batting.next();
    (batting.frames, batting.stirred) = (0, [false; 9]);
    batting.pitch(3, (0.0, -45.0))
}

#[test]
fn the_pitcher_still_throws_a_runner_out_but_nobody_throws_the_ball_on() {
    let Some(mut usual) = Batting::new(1, &[]) else {
        return;
    };
    let after = runner_on_first_and_a_weak_hit(&mut usual);
    // As the game was, the fielder at second takes the throw and sends the
    // ball on to first after the batter.
    assert!(after.contains("outs 1,"), "{after}");
    assert!(usual.stirred[AT_SECOND], "{:?}", usual.stirred);

    let mut alone = Batting::new(1, &[Mod::LonePitcher]).unwrap();
    let after = runner_on_first_and_a_weak_hit(&mut alone);
    // The throw to second is still an out. There the play ends: the
    // fielder at second does nothing with the ball, and the batter has
    // first.
    assert!(after.contains("outs 1,"), "{after}");
    assert!(after.contains("bases x--"), "{after}");
    let mut stirred = alone.stirred;
    assert!(stirred[PITCHER]);
    stirred[PITCHER] = false;
    assert_eq!(stirred, [false; 9]);
    assert!(alone.frames < usual.frames);
}
