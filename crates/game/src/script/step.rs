//! One written step of a script, read from words and written back as
//! them.

use anyhow::Result;
use bb_engine::input::Key;

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

impl std::fmt::Display for Step {
    /// Writes the step as a script has it. Read back, that is the same
    /// step, so long as it is one that can be written: a `shot` has to
    /// have a file with a name and no spaces in it, and what is typed
    /// cannot start with one.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Step::Wait(frames) => write!(f, "wait {frames}"),
            Step::Move(x, y) => write!(f, "move {x} {y}"),
            Step::Click(x, y) => write!(f, "click {x} {y}"),
            Step::Press => write!(f, "press"),
            Step::Release => write!(f, "release"),
            Step::Type(text) => write!(f, "type {text}"),
            Step::Key(key) => write!(f, "key {}", key.name()),
            Step::Hold(key) => write!(f, "hold {}", key.name()),
            Step::Lift(key) => write!(f, "lift {}", key.name()),
            Step::State => write!(f, "state"),
            Step::Events => write!(f, "events"),
            Step::Tree => write!(f, "tree"),
            Step::Shot(file) => write!(f, "shot {file}"),
        }
    }
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
    use proptest::prelude::*;

    /// Any key a script can name: one with a name of its own, or a letter,
    /// a figure or a mark that is not a space.
    fn any_key() -> impl Strategy<Value = Key> {
        prop_oneof![
            Just(Key::Backspace),
            Just(Key::Enter),
            Just(Key::Tab),
            Just(Key::Escape),
            Just(Key::Left),
            Just(Key::Right),
            Just(Key::Up),
            Just(Key::Down),
            Just(Key::Char(' ')),
            "[!-~]".prop_map(|letter| Key::Char(letter.chars().next().unwrap_or('a'))),
        ]
    }

    /// Any step a script can write.
    fn any_step() -> impl Strategy<Value = Step> {
        let place = || (-2000.0f32..2000.0, -2000.0f32..2000.0);
        prop_oneof![
            // Frames are read as a number with a point in it, which holds
            // every whole number up to this exactly.
            (0u32..=16_000_000).prop_map(Step::Wait),
            place().prop_map(|(x, y)| Step::Move(x, y)),
            place().prop_map(|(x, y)| Step::Click(x, y)),
            Just(Step::Press),
            Just(Step::Release),
            "([!-~][ -~]{0,20})?".prop_map(Step::Type),
            any_key().prop_map(Step::Key),
            any_key().prop_map(Step::Hold),
            any_key().prop_map(Step::Lift),
            Just(Step::State),
            Just(Step::Events),
            Just(Step::Tree),
            "[!-~]{1,20}".prop_map(Step::Shot),
        ]
    }

    proptest! {
        #[test]
        fn a_step_written_out_reads_back_as_the_same_step(step in any_step()) {
            let written = step.to_string();
            let read = Step::read(&written);
            prop_assert_eq!(read.ok(), Some(step), "from `{}`", written);
        }
    }

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
