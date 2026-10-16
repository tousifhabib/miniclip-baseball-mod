//! Finds the extracted art, and the player's own folder for the game.
//!
//! Run from a terminal, the game is told where the art is. Run as an app, it
//! is told nothing, and has to look.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

/// The app's identifier, which names its folder among the player's own.
pub const APP_ID: &str = "io.github.tousifhabib.baseball-mod";

/// The name of the folder that holds the art, wherever it is kept.
const FOLDER: &str = "extracted";

/// The two ways of keeping a player's folders that the game knows of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum System {
    /// Under `Library/Application Support` in the home folder. Every kind of
    /// machine but Windows is taken to keep them so.
    Mac,
    /// In the folder that `APPDATA` names, which roams with the player.
    Windows,
}

impl System {
    /// The kind of machine the game was built for.
    const HERE: System = if cfg!(windows) {
        System::Windows
    } else {
        System::Mac
    };
}

/// The player's own folder for the game: where what it keeps from one run to
/// the next is written, and where a copy of the art may be put. `None` if
/// the machine does not say where the player's folders are.
pub fn own_folder() -> Option<PathBuf> {
    own_folder_on(System::HERE, |name| std::env::var_os(name))
}

/// The player's own folder for the game on a kind of machine, by what `var`
/// says the environment holds.
fn own_folder_on(system: System, var: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let set = |name| {
        var(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    let folders = match system {
        System::Mac => set("HOME")?.join("Library").join("Application Support"),
        System::Windows => set("APPDATA")?,
    };
    Some(folders.join(APP_ID))
}

/// The places the art may be, most particular first:
///
/// 1. inside the Mac app bundle the program is running from, in its
///    resources;
/// 2. the player's own copy, in the game's own folder;
/// 3. beside the program;
/// 4. in the folder the program was started from.
///
/// `program` is the path of the running program, `own` the game's own folder
/// (see [`own_folder`]) and `current` the folder the program was started
/// from. Any of them may be unknown.
pub fn candidates(
    program: Option<&Path>,
    own: Option<&Path>,
    current: Option<&Path>,
) -> Vec<PathBuf> {
    let mut places = Vec::new();
    let beside = program.and_then(Path::parent);
    // In a bundle the program is at `Name.app/Contents/MacOS/program`.
    if let Some(contents) = beside
        .filter(|folder| folder.ends_with("Contents/MacOS"))
        .and_then(Path::parent)
    {
        places.push(contents.join("Resources").join(FOLDER));
    }
    if let Some(own) = own {
        places.push(own.join(FOLDER));
    }
    if let Some(beside) = beside {
        places.push(beside.join(FOLDER));
    }
    if let Some(current) = current {
        places.push(current.join(FOLDER));
    }
    places.dedup();
    places
}

/// Whether the program is running from inside an app bundle, and so has no
/// terminal to speak through.
pub fn in_app_bundle() -> bool {
    std::env::current_exe()
        .ok()
        .as_deref()
        .and_then(Path::parent)
        .is_some_and(|folder| folder.ends_with("Contents/MacOS"))
}

/// Whether `dir` holds extracted art.
fn holds_art(dir: &Path) -> bool {
    dir.join("manifest.json").is_file()
}

/// The folder to load the art from: `asked` if the player named one, or
/// else the first of the usual places that has it.
pub fn find(asked: Option<&Path>) -> Result<PathBuf> {
    if let Some(asked) = asked {
        // A folder that was asked for by name is never swapped for another.
        if !holds_art(asked) {
            bail!(
                "{} has no extracted art in it (no manifest.json)",
                asked.display()
            );
        }
        return Ok(asked.to_owned());
    }
    let program = std::env::current_exe().ok();
    let own = own_folder();
    let current = std::env::current_dir().ok();
    let places = candidates(program.as_deref(), own.as_deref(), current.as_deref());
    if let Some(found) = places.iter().find(|place| holds_art(place)) {
        return Ok(found.clone());
    }
    let looked: Vec<String> = places
        .iter()
        .map(|place| format!("  {}", place.display()))
        .collect();
    bail!(
        "The game's art was not found. Looked in:\n{}\n\
         It is the `extracted` folder of this project: put it in one of those places \
         or name it when starting the game.",
        looked.join("\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The game's own folder on a Mac, for a player called Pat.
    const PATS: &str = "/Users/pat/Library/Application Support/io.github.tousifhabib.baseball-mod";

    // Paths are compared a part at a time, so that a test written with
    // forward slashes means the same on Windows, which joins with the other.
    fn places(program: &str, own: &str, current: &str) -> Vec<PathBuf> {
        candidates(
            Some(Path::new(program)),
            Some(Path::new(own)),
            Some(Path::new(current)),
        )
    }

    fn paths(all: &[&str]) -> Vec<PathBuf> {
        all.iter().map(PathBuf::from).collect()
    }

    /// An environment that holds these and nothing else. It keeps its own
    /// copy of them, and so borrows nothing.
    fn holding(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> + use<> {
        let pairs: Vec<(String, OsString)> = pairs
            .iter()
            .map(|(name, value)| ((*name).to_owned(), OsString::from(value)))
            .collect();
        move |asked| {
            pairs
                .iter()
                .find(|(name, _)| name == asked)
                .map(|(_, value)| value.clone())
        }
    }

    #[test]
    fn an_app_looks_in_its_own_resources_first() {
        assert_eq!(
            places(
                "/Applications/Baseball.app/Contents/MacOS/bb-game",
                PATS,
                "/"
            ),
            paths(&[
                "/Applications/Baseball.app/Contents/Resources/extracted",
                "/Users/pat/Library/Application Support/io.github.tousifhabib.baseball-mod/extracted",
                "/Applications/Baseball.app/Contents/MacOS/extracted",
                "/extracted",
            ])
        );
    }

    #[test]
    fn a_plain_program_has_no_bundle_to_look_in() {
        assert_eq!(
            places("/work/target/release/bb-game", PATS, "/work"),
            paths(&[
                "/Users/pat/Library/Application Support/io.github.tousifhabib.baseball-mod/extracted",
                "/work/target/release/extracted",
                "/work/extracted",
            ])
        );
    }

    #[test]
    fn the_same_place_is_not_looked_in_twice() {
        assert_eq!(
            places("/work/bb-game", PATS, "/work"),
            paths(&[
                "/Users/pat/Library/Application Support/io.github.tousifhabib.baseball-mod/extracted",
                "/work/extracted",
            ])
        );
    }

    #[test]
    fn nothing_known_means_nowhere_to_look() {
        assert!(candidates(None, None, None).is_empty());
    }

    #[test]
    fn a_mac_keeps_the_game_under_application_support() {
        let environment = holding(&[("HOME", "/Users/pat"), ("APPDATA", "/elsewhere")]);
        assert_eq!(
            own_folder_on(System::Mac, environment),
            Some(PathBuf::from(PATS))
        );
    }

    #[test]
    fn windows_keeps_the_game_where_appdata_says() {
        let roaming = r"C:\Users\pat\AppData\Roaming";
        let environment = holding(&[("HOME", "/Users/pat"), ("APPDATA", roaming)]);
        assert_eq!(
            own_folder_on(System::Windows, environment),
            Some(Path::new(roaming).join(APP_ID))
        );
    }

    #[test]
    fn a_machine_that_does_not_say_where_the_folders_are_has_none() {
        for system in [System::Mac, System::Windows] {
            assert_eq!(own_folder_on(system, holding(&[])), None);
            let blank = holding(&[("HOME", ""), ("APPDATA", "")]);
            assert_eq!(own_folder_on(system, blank), None);
        }
    }

    #[test]
    fn a_folder_asked_for_by_name_must_hold_the_art() {
        let missing = Path::new("/nowhere/at/all");
        let error = find(Some(missing)).unwrap_err().to_string();
        assert!(error.contains("/nowhere/at/all"), "{error}");
    }
}
