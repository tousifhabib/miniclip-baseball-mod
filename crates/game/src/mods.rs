//! The mods: changes to the game that the player switches on and off from
//! the menu, and the page of the menu they are listed on.

use std::collections::BTreeSet;
use std::path::{Path as FilePath, PathBuf};

use anyhow::{Context, Result};
use bb_engine::display::Path;
use bb_engine::library::Library;
use bb_engine::math::Matrix;
use bb_engine::stage::Stage;
use bb_format::SymbolId;
use serde::{Deserialize, Serialize};

use crate::art;
use crate::locate::APP_ID;
use crate::look::{self, Rgb};

/// One change to the game. To add a mod, add it here and to [`Mod::ALL`],
/// and have the rules ask [`Mods::is_on`] for it: the menu lists whatever
/// is in `ALL`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mod {
    /// A bar in the batting view that shows when to swing.
    TimingIndicator,
    /// Only the pitcher goes after a ball that has been hit. The rest of
    /// the side stands where it is, and nobody throws the ball on.
    LonePitcher,
    /// Every ball the bat meets is a home run, and the better the swing was
    /// timed the further it goes.
    ZingerHit,
}

impl Mod {
    /// Every mod, in the order the menu lists them.
    pub const ALL: [Mod; 3] = [Mod::TimingIndicator, Mod::LonePitcher, Mod::ZingerHit];

    /// The name the mod is saved under, and asked for by on the command
    /// line.
    pub fn key(self) -> &'static str {
        match self {
            Mod::TimingIndicator => "timing_indicator",
            Mod::LonePitcher => "lone_pitcher",
            Mod::ZingerHit => "zinger_hit",
        }
    }

    /// The mod with this key.
    pub fn from_key(key: &str) -> Option<Mod> {
        Mod::ALL.into_iter().find(|each| each.key() == key)
    }

    /// What the menu calls it.
    pub fn name(self) -> &'static str {
        match self {
            Mod::TimingIndicator => "TIMING INDICATOR",
            Mod::LonePitcher => "LONE PITCHER",
            Mod::ZingerHit => "ZINGER HIT",
        }
    }

    /// What the menu says it does.
    pub fn about(self) -> &'static str {
        match self {
            Mod::TimingIndicator => "A BAR THAT SHOWS WHEN TO SWING",
            Mod::LonePitcher => "ONLY THE PITCHER GOES AFTER THE BALL",
            Mod::ZingerHit => "EVERY HIT IS A HOME RUN, BIGGER THE BETTER TIMED",
        }
    }
}

/// The mods that are switched on. They last from one game to the next.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    on: BTreeSet<Mod>,
}

/// The mods as they are written to their file: the keys of the ones that
/// are on.
#[derive(Default, Serialize, Deserialize)]
struct Saved {
    #[serde(default)]
    on: Vec<String>,
}

impl Mods {
    pub fn is_on(&self, which: Mod) -> bool {
        self.on.contains(&which)
    }

    pub fn set(&mut self, which: Mod, on: bool) {
        if on {
            self.on.insert(which);
        } else {
            self.on.remove(&which);
        }
    }

    /// Switches a mod over, and returns whether that left it on.
    pub fn toggle(&mut self, which: Mod) -> bool {
        let on = !self.is_on(which);
        self.set(which, on);
        on
    }

    /// The mods that are on, in the menu's order.
    pub fn all_on(&self) -> impl Iterator<Item = Mod> + '_ {
        self.on.iter().copied()
    }

    /// Where the choice is kept: beside the scores, in the game's folder
    /// under Application Support.
    pub fn usual_file() -> Option<PathBuf> {
        let home = std::env::var_os("HOME")?;
        Some(
            PathBuf::from(home)
                .join("Library/Application Support")
                .join(APP_ID)
                .join("mods.toml"),
        )
    }

    /// Reads which mods are on. A file that is missing or cannot be read
    /// leaves every mod off, and a mod the game no longer has is passed
    /// over: neither should stop anyone playing.
    pub fn load(file: &FilePath) -> Mods {
        let saved: Saved = std::fs::read_to_string(file)
            .ok()
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default();
        Mods {
            on: saved
                .on
                .iter()
                .filter_map(|key| Mod::from_key(key))
                .collect(),
        }
    }

    pub fn save(&self, file: &FilePath) -> Result<()> {
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder)
                .with_context(|| format!("making {}", folder.display()))?;
        }
        let saved = Saved {
            on: self.all_on().map(|each| each.key().to_owned()).collect(),
        };
        let text = toml::to_string(&saved).context("writing out the mods")?;
        std::fs::write(file, text).with_context(|| format!("writing {}", file.display()))
    }
}

/// One mod as the page lists it: the box, and the tick that shows in it.
struct Line {
    which: Mod,
    button: Path,
    tick: Path,
}

/// The page of the menu the mods are listed on. It is the high-score page
/// with the table's own drawings taken off its panel and the list put there
/// instead, so it arrives, leaves and has a way back as that page does.
#[derive(Default)]
pub struct ModsPage {
    /// The lines on the panel, while the page is up.
    lines: Vec<Line>,
}

impl ModsPage {
    /// Where the first line's box goes on the panel, and how far down each
    /// line after it is.
    const FIRST: (f32, f32) = (-164.0, -78.0);
    const PITCH: f32 = 40.0;
    /// The sizes of a mod's name and of what is said about it, the
    /// lettering's own size being 1.
    const NAME_SIZE: f32 = 0.85;
    const ABOUT_SIZE: f32 = 0.6;

    /// Puts the list on the panel once the page has arrived, and keeps its
    /// ticks true to `mods` for as long as it is up.
    pub fn show(&mut self, showing: bool, mods: &Mods, stage: &mut Stage, library: &Library) {
        // The lines go when the panel does, as the page is left.
        if self
            .lines
            .first()
            .is_some_and(|line| stage.child(&line.button).is_none())
        {
            self.lines.clear();
        }
        if !showing {
            return;
        }
        if self.lines.is_empty() {
            self.lay_out(stage, library);
        }
        for line in &self.lines {
            if let Some(tick) = stage.child_mut(&line.tick) {
                tick.set_visible(mods.is_on(line.which));
            }
        }
    }

    /// The mod whose box is at `path`, if that is one of the page's boxes.
    pub fn clicked(&self, path: &[u16]) -> Option<Mod> {
        self.lines
            .iter()
            .find(|line| line.button == path)
            .map(|line| line.which)
    }

    fn lay_out(&mut self, stage: &mut Stage, library: &Library) {
        let Some(panel) =
            art::shell(stage).and_then(|shell| stage.find_symbol(&shell, art::SCORE_PANEL))
        else {
            return;
        };
        // What the panel was drawn with for the scores: its backing with
        // the table's tabs, the publisher's mark, and the notice.
        let table: Vec<Path> = stage.clip(&panel).map_or(Vec::new(), |clip| {
            clip.children
                .iter()
                .filter(|(_, child)| {
                    art::SCORE_PANEL_TABLE.contains(&child.symbol)
                        || art::SCORE_PANEL_NOTICE.contains(&child.symbol)
                })
                .map(|(&depth, _)| {
                    let mut path = panel.clone();
                    path.push(depth);
                    path
                })
                .collect()
        });
        if table.is_empty() {
            // The panel has not finished arriving.
            return;
        }
        for path in table {
            if let Some(child) = stage.child_mut(&path) {
                child.set_visible(false);
            }
        }
        // A backing without the tabs goes under the panel's border, which
        // stays.
        stage.attach(
            &panel,
            art::MODS_PANEL,
            art::MODS_PANEL_DEPTH,
            "modsPanel",
            library,
        );

        const DARK: Rgb = [0x0b, 0x3a, 0x5e];
        const SOFT: Rgb = [0x4a, 0x6f, 0x8c];
        const WHITE: Rgb = [0xff, 0xff, 0xff];
        let mut panel = Panel {
            path: panel,
            depth: Stage::RULES_DEPTH + 300,
            library,
        };
        panel.write(stage, "MODS", (-166.0, -123.0), 0.8, WHITE);

        let (left, top) = ModsPage::FIRST;
        for (index, which) in Mod::ALL.into_iter().enumerate() {
            let down = top + index as f32 * ModsPage::PITCH;
            let words = left + 24.0;
            let size = ModsPage::NAME_SIZE;
            panel.write(stage, which.name(), (words, down - 5.0), size, DARK);
            let size = ModsPage::ABOUT_SIZE;
            panel.write(stage, which.about(), (words, down + 11.0), size, SOFT);
            // The box goes on after the words, so that its band lights the
            // whole line, and anywhere on the line ticks it.
            let button = panel.add(stage, art::MOD_BOX, "modBox", (left, down), 1.0);
            let tick = panel.add(stage, art::MOD_TICK, "modTick", (left, down), 1.0);
            if let (Some(button), Some(tick)) = (button, tick) {
                self.lines.push(Line {
                    which,
                    button,
                    tick,
                });
            }
        }
    }
}

/// The panel while the list is being put on it.
struct Panel<'a> {
    path: Path,
    /// The depth the last thing was put at.
    depth: u16,
    library: &'a Library,
}

impl Panel<'_> {
    /// Puts a symbol on the panel at a point, at a size, its own being 1.
    fn add(
        &mut self,
        stage: &mut Stage,
        symbol: SymbolId,
        name: &str,
        at: (f32, f32),
        size: f32,
    ) -> Option<Path> {
        self.depth += 1;
        let path = stage.attach(&self.path, symbol, self.depth, name, self.library)?;
        stage.child_mut(&path)?.set_matrix(Matrix {
            a: size,
            d: size,
            tx: at.0,
            ty: at.1,
            ..Matrix::IDENTITY
        });
        Some(path)
    }

    /// Writes a line of words on the panel, starting from a point.
    fn write(&mut self, stage: &mut Stage, text: &str, at: (f32, f32), size: f32, colour: Rgb) {
        let Some(path) = self.add(stage, art::LABEL_FIELD, "modsWords", at, size) else {
            return;
        };
        if let Some(field) = stage.child_mut(&path) {
            field.said = Some(text.to_owned());
            field.set_color(look::tint(colour));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mod_is_known_by_its_own_key() {
        for which in Mod::ALL {
            assert_eq!(Mod::from_key(which.key()), Some(which));
        }
        let keys: BTreeSet<&str> = Mod::ALL.iter().map(|each| each.key()).collect();
        assert_eq!(keys.len(), Mod::ALL.len());
        assert_eq!(Mod::from_key("no_such_mod"), None);
    }

    #[test]
    fn a_mod_is_off_until_it_is_switched_on() {
        let mut mods = Mods::default();
        assert!(!mods.is_on(Mod::TimingIndicator));
        assert!(mods.toggle(Mod::TimingIndicator));
        assert!(mods.is_on(Mod::TimingIndicator));
        assert!(!mods.toggle(Mod::TimingIndicator));
        assert!(!mods.is_on(Mod::TimingIndicator));
    }

    #[test]
    fn the_choice_comes_back_as_it_was_saved() {
        let file = std::env::temp_dir().join(format!("bb-mods-{}.toml", std::process::id()));
        let mut mods = Mods::default();
        mods.set(Mod::TimingIndicator, true);
        mods.save(&file).unwrap();
        assert_eq!(Mods::load(&file), mods);
        // A mod the game does not have is passed over.
        std::fs::write(&file, "on = [\"long_gone\", \"timing_indicator\"]").unwrap();
        assert_eq!(Mods::load(&file), mods);
        // No file, or nonsense in it, leaves every mod off.
        std::fs::write(&file, "not a table at all [").unwrap();
        assert_eq!(Mods::load(&file), Mods::default());
        std::fs::remove_file(&file).unwrap();
        assert_eq!(Mods::load(&file), Mods::default());
    }
}
