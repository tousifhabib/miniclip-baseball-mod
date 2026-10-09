//! Plays the game with no window, following written steps. This is how the
//! rules are checked without a mouse, by hand or from a test.

use anyhow::{Context, Result, bail};
use bb_engine::app::{Note, Runner};
use bb_engine::display::describe_tree;
use bb_engine::gpu::Renderer;
use bb_engine::input::{Geometry, Key};
use bb_engine::math::Matrix;
use bb_engine::meshes::Meshes;

/// A game being played by a script.
pub struct Script {
    pub runner: Runner,
    eyes: Eyes,
    /// Picture pixels per stage pixel, for `shot`.
    pub scale: f32,
}

/// What the script looks at the stage with: its triangles alone, which is
/// all a pointer needs, until a picture is asked for. Only then is a
/// graphics device opened, so a game that is played and never drawn needs
/// none.
#[expect(
    clippy::large_enum_variant,
    reason = "a game has one of these, so its size is of no account"
)]
enum Eyes {
    Triangles(Meshes),
    Renderer(Box<Renderer>),
}

impl Eyes {
    /// What tells whether the pointer is on a thing.
    fn geometry(&mut self) -> &mut dyn Geometry {
        match self {
            Eyes::Triangles(meshes) => meshes,
            Eyes::Renderer(renderer) => renderer.as_mut(),
        }
    }

    /// What draws a picture, opened now if it has not been.
    fn renderer(&mut self) -> Result<&mut Renderer> {
        if let Eyes::Triangles(meshes) = self {
            let renderer = Renderer::headless()?;
            // The triangles made so far go with it.
            let renderer = renderer.with_meshes(std::mem::take(meshes));
            *self = Eyes::Renderer(Box::new(renderer));
        }
        match self {
            Eyes::Renderer(renderer) => Ok(renderer),
            Eyes::Triangles(_) => bail!("the graphics device did not open"),
        }
    }

    fn problems(&self) -> &[String] {
        match self {
            Eyes::Triangles(meshes) => meshes.problems(),
            Eyes::Renderer(renderer) => renderer.problems(),
        }
    }
}

impl Script {
    pub fn new(runner: Runner) -> Result<Script> {
        Ok(Script {
            runner,
            eyes: Eyes::Triangles(Meshes::default()),
            scale: 1.0,
        })
    }

    /// Does each step in turn, and returns the lines that `state`, `events`
    /// and `tree` gave. Steps are separated by semicolons, and [`Step`] says
    /// what each may be. A step that cannot be read stops the script there,
    /// with the steps before it done.
    pub fn run(&mut self, steps: &str) -> Result<Vec<String>> {
        let mut lines = Vec::new();
        for step in steps
            .split(';')
            .map(str::trim)
            .filter(|step| !step.is_empty())
        {
            self.take(Step::read(step)?, &mut lines)?;
        }
        Ok(lines)
    }

    /// What could not be drawn, if anything.
    pub fn problems(&self) -> &[String] {
        self.eyes.problems()
    }

    /// Does one step, adding whatever it gives to `lines`.
    fn take(&mut self, step: Step, lines: &mut Vec<String>) -> Result<()> {
        let (runner, renderer) = (&mut self.runner, self.eyes.geometry());
        match step {
            Step::Wait(frames) => {
                for _ in 0..frames {
                    runner.tick(renderer);
                }
            }
            Step::Move(x, y) => {
                let down = runner.stage.pointer.down;
                runner.pointer(x, y, down, renderer);
            }
            Step::Click(x, y) => {
                // Arrive, press, let a frame pass, let go: what a hand does.
                runner.pointer(x, y, false, renderer);
                runner.pointer(x, y, true, renderer);
                runner.tick(renderer);
                runner.pointer(x, y, false, renderer);
            }
            Step::Press | Step::Release => {
                let (x, y) = (runner.stage.pointer.x, runner.stage.pointer.y);
                runner.pointer(x, y, step == Step::Press, renderer);
            }
            Step::Type(text) => {
                for c in text.chars() {
                    runner.key(Key::Char(c));
                }
            }
            Step::Key(key) => {
                runner.key(key);
            }
            Step::Hold(key) => {
                runner.hold(key, true);
                // A key that goes down is a key pressed, too.
                runner.key(key);
            }
            Step::Lift(key) => runner.hold(key, false),
            Step::State => lines.push(runner.describe()),
            Step::Events => {
                // Frames are reported in their hundreds; buttons, keys and
                // sounds are what a script wants to see.
                lines.extend(
                    runner
                        .take_notes()
                        .into_iter()
                        .filter(|note| !matches!(note, Note::Frame { .. }))
                        .map(|note| format!("  {note}")),
                );
            }
            Step::Tree => lines.extend(
                describe_tree(&runner.stage.root.children, &runner.library)
                    .lines()
                    .map(str::to_owned),
            ),
            Step::Shot(file) => {
                let scale = self.scale;
                let size = runner.library.picture_size(scale);
                let background = runner.library.background();
                let commands = runner
                    .stage
                    .commands(Matrix::scale(scale, scale), &runner.library);
                let renderer = self.eyes.renderer()?;
                renderer.min_stroke = scale.max(1.0);
                renderer
                    .capture(&runner.library, &commands, size, background)?
                    .save(&file)
                    .with_context(|| format!("writing {file}"))?;
            }
        }
        Ok(())
    }
}

/// One step of a script: something done to the game, or asked of it.
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    /// `wait N` plays N frames.
    Wait(u32),
    /// `move X Y` moves the pointer to a stage position.
    Move(f32, f32),
    /// `click X Y` clicks at a stage position, which plays one frame.
    Click(f32, f32),
    /// `press` puts the pointer's button down where it is.
    Press,
    /// `release` lets it up again.
    Release,
    /// `type TEXT` types the rest of the step, spaces and all.
    Type(String),
    /// `key NAME` presses `backspace`, `enter`, `tab`, `escape`, `left`,
    /// `right`, `up`, `down`, `space`, or a letter or figure.
    Key(Key),
    /// `hold NAME` puts such a key down and keeps it there.
    Hold(Key),
    /// `lift NAME` lets it up.
    Lift(Key),
    /// `state` gives where the game is.
    State,
    /// `events` gives the buttons touched and sounds asked for since it was
    /// last used.
    Events,
    /// `tree` gives every object on the stage.
    Tree,
    /// `shot FILE` saves a picture.
    Shot(String),
}

/// What is wrong with a step as it was written.
#[derive(Debug, thiserror::Error)]
pub enum StepFault {
    #[error("`{step}` needs more after it")]
    NeedsMore { step: String },
    #[error("`{word}` in `{step}` is not a number")]
    NotANumber {
        word: String,
        step: String,
        source: std::num::ParseFloatError,
    },
    #[error("`{step}` needs the name of a key")]
    NeedsAKey { step: String },
    #[error("there is no key called `{name}`")]
    NoSuchKey { name: String },
    #[error("`{step}` needs a file name")]
    NeedsAFile { step: String },
    #[error("unknown step `{word}`")]
    Unknown { word: String },
}

impl Step {
    /// Reads one step as it is written, with no semicolon and nothing
    /// round it.
    pub fn read(step: &str) -> Result<Step, StepFault> {
        let words: Vec<&str> = step.split_whitespace().collect();
        let number = |index: usize| -> Result<f32, StepFault> {
            let word = words.get(index).ok_or_else(|| StepFault::NeedsMore {
                step: step.to_owned(),
            })?;
            word.parse().map_err(|source| StepFault::NotANumber {
                word: (*word).to_owned(),
                step: step.to_owned(),
                source,
            })
        };
        let key = || -> Result<Key, StepFault> {
            let name = words.get(1).ok_or_else(|| StepFault::NeedsAKey {
                step: step.to_owned(),
            })?;
            Key::named(name).ok_or_else(|| StepFault::NoSuchKey {
                name: (*name).to_owned(),
            })
        };
        Ok(match words.first().copied().unwrap_or_default() {
            // A number of frames may be written with a point in it, and
            // only the whole frames are played.
            "wait" => Step::Wait(number(1)? as u32),
            "move" => Step::Move(number(1)?, number(2)?),
            "click" => Step::Click(number(1)?, number(2)?),
            "press" => Step::Press,
            "release" => Step::Release,
            // Everything after the word, spaces and all.
            "type" => Step::Type(step["type".len()..].trim_start().to_owned()),
            "key" => Step::Key(key()?),
            "hold" => Step::Hold(key()?),
            "lift" => Step::Lift(key()?),
            "state" => Step::State,
            "events" => Step::Events,
            "tree" => Step::Tree,
            "shot" => {
                let file = words.get(1).ok_or_else(|| StepFault::NeedsAFile {
                    step: step.to_owned(),
                })?;
                Step::Shot((*file).to_owned())
            }
            other => {
                return Err(StepFault::Unknown {
                    word: other.to_owned(),
                });
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_step_is_read_as_what_it_says() {
        let read = |step| Step::read(step).expect("a step that reads");
        assert_eq!(read("wait 60"), Step::Wait(60));
        assert_eq!(read("wait 2.9"), Step::Wait(2));
        assert_eq!(read("move 10 20.5"), Step::Move(10.0, 20.5));
        assert_eq!(read("click 545 355"), Step::Click(545.0, 355.0));
        assert_eq!(read("press"), Step::Press);
        assert_eq!(read("release"), Step::Release);
        assert_eq!(read("key enter"), Step::Key(Key::Enter));
        assert_eq!(read("key a"), Step::Key(Key::Char('a')));
        assert_eq!(read("hold space"), Step::Hold(Key::Char(' ')));
        assert_eq!(read("lift space"), Step::Lift(Key::Char(' ')));
        assert_eq!(read("state"), Step::State);
        assert_eq!(read("events"), Step::Events);
        assert_eq!(read("tree"), Step::Tree);
        assert_eq!(read("shot out.png"), Step::Shot("out.png".to_owned()));
    }

    #[test]
    fn what_is_typed_keeps_the_spaces_inside_it() {
        assert_eq!(
            Step::read("type Red Sox 9").expect("a step that reads"),
            Step::Type("Red Sox 9".to_owned())
        );
        assert_eq!(
            Step::read("type").expect("a step that reads"),
            Step::Type(String::new())
        );
    }

    #[test]
    fn a_step_that_cannot_be_read_says_what_is_wrong_with_it() {
        let wrong = |step| {
            let fault = Step::read(step).expect_err("a step that does not read");
            // As it is shown to whoever ran the script, causes and all.
            format!("{:#}", anyhow::Error::from(fault))
        };
        assert_eq!(wrong("wait"), "`wait` needs more after it");
        assert_eq!(wrong("move 1"), "`move 1` needs more after it");
        assert_eq!(
            wrong("click x 2"),
            "`x` in `click x 2` is not a number: invalid float literal"
        );
        assert_eq!(wrong("key"), "`key` needs the name of a key");
        assert_eq!(wrong("hold shift"), "there is no key called `shift`");
        assert_eq!(wrong("shot"), "`shot` needs a file name");
        assert_eq!(wrong("dance"), "unknown step `dance`");
    }
}
