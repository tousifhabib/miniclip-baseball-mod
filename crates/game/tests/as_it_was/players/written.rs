//! A player who follows steps written out for it, pressing buttons by
//! the words on them.

use std::collections::VecDeque;

use bb_game::art::ButtonLabels;
use bb_game::script::Script;

use super::{Player, buttons, run};

/// Plays written steps, a frame at a time. The steps are the script's own,
/// and one more: `find WORDS` clicks the button that says just that, if it
/// can be seen, and lets a frame go by if not.
pub struct Written {
    steps: VecDeque<String>,
    labels: Option<ButtonLabels>,
}

impl Written {
    pub fn new(steps: &str) -> Written {
        Written {
            steps: steps
                .split(';')
                .map(str::trim)
                .filter(|step| !step.is_empty())
                .map(str::to_owned)
                .collect(),
            labels: None,
        }
    }

    /// The middle of the button that says `words`, where it can be seen.
    fn find(&mut self, words: &str, script: &Script) -> Option<(f32, f32)> {
        let labels = self
            .labels
            .get_or_insert_with(|| ButtonLabels::read(script.runner.library()));
        buttons(script)
            .into_iter()
            .find(|&(button, _, _)| labels.get(button) == Some(words))
            .map(|(_, x, y)| (x, y))
    }
}

impl Player for Written {
    fn frame(&mut self, script: &mut Script) -> bool {
        while let Some(step) = self.steps.pop_front() {
            let (word, rest) = step.split_once(' ').unwrap_or((&step, ""));
            match word {
                "wait" => {
                    let frames: u32 = rest.parse().expect("a number of frames");
                    if frames == 0 {
                        continue;
                    }
                    self.steps.push_front(format!("wait {}", frames - 1));
                    run(script, "wait 1");
                    return true;
                }
                "click" => {
                    run(script, &step);
                    return true;
                }
                "find" => {
                    match self.find(rest, script) {
                        Some((x, y)) => run(script, &format!("click {x} {y}")),
                        None => {
                            eprintln!("as it was: no button says `{rest}` just now");
                            run(script, "wait 1");
                        }
                    }
                    return true;
                }
                // Anything else plays no frame, and goes with the next that
                // does.
                _ => run(script, &step),
            }
        }
        false
    }
}

/// One player, and when he has nothing left to play, another.
pub struct Then(pub Box<dyn Player>, pub Box<dyn Player>);

impl Player for Then {
    fn frame(&mut self, script: &mut Script) -> bool {
        self.0.frame(script) || self.1.frame(script)
    }
}
