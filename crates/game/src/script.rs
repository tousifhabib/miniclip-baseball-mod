//! Plays the game with no window, following written steps. This is how the
//! rules are checked without a mouse, by hand or from a test.

use anyhow::{Context, Result, bail};
use bb_engine::app::Runner;
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
    /// and `tree` gave. Steps are separated by semicolons:
    ///
    /// - `wait N` plays N frames
    /// - `click X Y` clicks at a stage position
    /// - `move X Y` moves the pointer
    /// - `press` and `release` work the pointer's button where it is
    /// - `type TEXT` types the rest of the step
    /// - `key NAME` presses `backspace`, `enter`, `tab`, `escape`, `left`,
    ///   `right`, `up`, `down`, `space`, or a letter or figure
    /// - `hold NAME` puts such a key down and keeps it there, and
    ///   `lift NAME` lets it up
    /// - `state` gives where the game is
    /// - `events` gives the buttons touched and sounds asked for since it
    ///   was last used
    /// - `tree` gives every object on the stage
    /// - `shot FILE` saves a picture
    pub fn run(&mut self, steps: &str) -> Result<Vec<String>> {
        let mut lines = Vec::new();
        for step in steps
            .split(';')
            .map(str::trim)
            .filter(|step| !step.is_empty())
        {
            self.step(step, &mut lines)?;
        }
        Ok(lines)
    }

    /// What could not be drawn, if anything.
    pub fn problems(&self) -> &[String] {
        self.eyes.problems()
    }

    fn step(&mut self, step: &str, lines: &mut Vec<String>) -> Result<()> {
        let (runner, renderer) = (&mut self.runner, self.eyes.geometry());
        let words: Vec<&str> = step.split_whitespace().collect();
        let number = |index: usize| -> Result<f32> {
            let word = words
                .get(index)
                .with_context(|| format!("`{step}` needs more after it"))?;
            word.parse()
                .with_context(|| format!("`{word}` in `{step}` is not a number"))
        };
        match words[0] {
            "wait" => {
                for _ in 0..number(1)? as u32 {
                    runner.tick(renderer);
                }
            }
            "move" => {
                let (x, y) = (number(1)?, number(2)?);
                let down = runner.stage.pointer.down;
                runner.pointer(x, y, down, renderer);
            }
            "click" => {
                let (x, y) = (number(1)?, number(2)?);
                // Arrive, press, let a frame pass, let go: what a hand does.
                runner.pointer(x, y, false, renderer);
                runner.pointer(x, y, true, renderer);
                runner.tick(renderer);
                runner.pointer(x, y, false, renderer);
            }
            "press" | "release" => {
                let (x, y) = (runner.stage.pointer.x, runner.stage.pointer.y);
                runner.pointer(x, y, words[0] == "press", renderer);
            }
            "type" => {
                // Everything after the word, spaces and all.
                let text = step["type".len()..].trim_start();
                for c in text.chars() {
                    runner.key(Key::Char(c));
                }
            }
            "key" => {
                let name = words
                    .get(1)
                    .with_context(|| format!("`{step}` needs the name of a key"))?;
                let Some(key) = Key::named(name) else {
                    bail!("there is no key called `{name}`");
                };
                runner.key(key);
            }
            "hold" | "lift" => {
                let name = words
                    .get(1)
                    .with_context(|| format!("`{step}` needs the name of a key"))?;
                let Some(key) = Key::named(name) else {
                    bail!("there is no key called `{name}`");
                };
                let down = words[0] == "hold";
                runner.hold(key, down);
                // A key that goes down is a key pressed, too.
                if down {
                    runner.key(key);
                }
            }
            "state" => lines.push(runner.describe()),
            "events" => {
                // Frames are reported in their hundreds; buttons, keys and
                // sounds are what a script wants to see.
                lines.extend(
                    runner
                        .take_notes()
                        .into_iter()
                        .filter(|note| !note.contains(": frame "))
                        .map(|note| format!("  {note}")),
                );
            }
            "tree" => lines.extend(
                describe_tree(&runner.stage.root.children, &runner.library)
                    .lines()
                    .map(str::to_owned),
            ),
            "shot" => {
                let file = words
                    .get(1)
                    .with_context(|| format!("`{step}` needs a file name"))?;
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
                    .save(file)
                    .with_context(|| format!("writing {file}"))?;
            }
            other => bail!("unknown step `{other}`"),
        }
        Ok(())
    }
}
