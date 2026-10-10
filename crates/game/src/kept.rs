//! What the game keeps from one run to the next: where its files are, and
//! how they are read and written.
//!
//! The scores and the choice of mods are each a small file in the game's
//! folder under Application Support. Neither is worth stopping anyone
//! playing for, so a file that is missing or damaged reads as nothing kept.
//! A file is written beside where it goes and then moved into place, so
//! that a run cut short in the middle of writing leaves the old file whole.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::locate::APP_ID;

/// Where a file of this name is kept. `None` if there is no home folder to
/// keep it under.
pub fn usual_file(name: &str) -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join("Library/Application Support")
            .join(APP_ID)
            .join(name),
    )
}

/// Reads what was kept in `file`. `None` if it is missing or cannot be
/// read.
pub fn read<T: DeserializeOwned>(file: &Path) -> Option<T> {
    let text = std::fs::read_to_string(file).ok()?;
    toml::from_str(&text).ok()
}

/// Writes `what` to `file`, making its folder if need be. `called` is what
/// to call it if it cannot be written out, as in "writing out the scores".
pub fn write<T: Serialize>(file: &Path, what: &T, called: &str) -> Result<()> {
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder).with_context(|| format!("making {}", folder.display()))?;
    }
    let text = toml::to_string(what).with_context(|| format!("writing out {called}"))?;
    // Written beside the file and moved over it, which is done all at once.
    let beside = beside(file);
    let moved = std::fs::write(&beside, text)
        .and_then(|()| std::fs::rename(&beside, file))
        .with_context(|| format!("writing {}", file.display()));
    if moved.is_err() {
        // Nothing half-written is left lying about.
        let _ = std::fs::remove_file(&beside);
    }
    moved
}

/// Takes away what was kept in `file`. A file that is not there is one
/// with nothing kept in it already. `called` is what to call it if it
/// cannot be taken away.
pub fn forget(file: &Path, called: &str) -> Result<()> {
    match std::fs::remove_file(file) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            Err(error).with_context(|| format!("taking away {called}, in {}", file.display()))
        }
        _ => Ok(()),
    }
}

/// The file that `file` is first written as: the same name with `.new` on
/// the end, in the same folder.
fn beside(file: &Path) -> PathBuf {
    let mut name = file.file_name().unwrap_or_default().to_owned();
    name.push(".new");
    file.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("bb-kept-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        folder
    }

    #[test]
    fn what_is_written_reads_back_and_leaves_nothing_else_behind() {
        let folder = folder("round");
        let file = folder.join("deep").join("numbers.toml");
        let numbers = BTreeMap::from([("one".to_owned(), 1), ("two".to_owned(), 2)]);
        write(&file, &numbers, "the numbers").expect("a file that writes");
        assert_eq!(read::<BTreeMap<String, i32>>(&file), Some(numbers));
        let left: Vec<_> = std::fs::read_dir(file.parent().expect("a folder"))
            .expect("the folder")
            .map(|entry| entry.expect("an entry").file_name())
            .collect();
        assert_eq!(left, ["numbers.toml"]);
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn writing_again_takes_the_place_of_what_was_there() {
        let folder = folder("again");
        let file = folder.join("numbers.toml");
        for count in [1, 2] {
            let numbers = BTreeMap::from([("count".to_owned(), count)]);
            write(&file, &numbers, "the numbers").expect("a file that writes");
        }
        let kept: BTreeMap<String, i32> = read(&file).expect("a file that reads");
        assert_eq!(kept["count"], 2);
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn a_file_that_is_missing_or_makes_no_sense_reads_as_nothing_kept() {
        let folder = folder("missing");
        let file = folder.join("numbers.toml");
        assert_eq!(read::<BTreeMap<String, i32>>(&file), None);
        std::fs::create_dir_all(&folder).expect("a folder");
        std::fs::write(&file, "this is = = not toml").expect("a file");
        assert_eq!(read::<BTreeMap<String, i32>>(&file), None);
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn what_was_kept_can_be_taken_away_and_nothing_can_be_taken_away_twice() {
        let folder = folder("forget");
        let file = folder.join("numbers.toml");
        write(
            &file,
            &BTreeMap::from([("one".to_owned(), 1)]),
            "the numbers",
        )
        .expect("a file that writes");
        forget(&file, "the numbers").expect("a file that goes");
        assert_eq!(read::<BTreeMap<String, i32>>(&file), None);
        forget(&file, "the numbers").expect("nothing there to go");
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn a_file_is_written_beside_itself_under_the_same_name_with_new_on_the_end() {
        assert_eq!(
            beside(Path::new("/a/folder/scores.toml")),
            Path::new("/a/folder/scores.toml.new")
        );
    }
}
