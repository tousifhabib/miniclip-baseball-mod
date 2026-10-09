//! Plays the game with no window, following written steps. This is how the
//! rules are checked without a mouse, by hand or from a test.

mod step;

use anyhow::{Context, Result, bail};
use bb_engine::app::{Note, Runner};
use bb_engine::display::describe_tree;
use bb_engine::gpu::Renderer;
use bb_engine::input::{Geometry, Key};
use bb_engine::math::Matrix;
use bb_engine::meshes::Meshes;

pub use step::{Step, StepFault};

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
                describe_tree(&runner.stage.root.children, runner.library())
                    .lines()
                    .map(str::to_owned),
            ),
            Step::Shot(file) => {
                let scale = self.scale;
                let size = runner.library().picture_size(scale);
                let background = runner.library().background();
                let commands = runner.stage.commands(Matrix::scale(scale, scale));
                let renderer = self.eyes.renderer()?;
                renderer.min_stroke = scale.max(1.0);
                renderer
                    .capture(runner.library(), &commands, size, background)?
                    .save(&file)
                    .with_context(|| format!("writing {file}"))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {}
