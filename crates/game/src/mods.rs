//! The mods: changes to the game that the player switches on and off from
//! the menu, and the page of the menu they are listed on.

use std::collections::{BTreeMap, BTreeSet};
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
use crate::rules::Rules;

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
    /// Fielders let the ball go: they drop catches, fumble pick-ups, and
    /// fail to hold throws at the bases. How often is set on the menu.
    Butterfingers,
    /// Pitches sway from side to side on the way in, and the marker shows
    /// only roughly where they will cross.
    Knuckleball,
    /// Every run scored makes the next pitch faster, and every strike
    /// slows the pitches down again.
    HeatCheck,
    /// Each pitch is a fastball, a slow change-up or a big curve, by
    /// chance, and nothing shows which until it leaves his hand.
    MysteryPitch,
    /// Before a pitch the player clicks a spot on the outfield, and a hit
    /// that comes down near it is worth runs on top.
    CalledShot,
    /// Each hit in a row widens the timing window for the next swing,
    /// and a strike takes it back to what it was.
    HotBat,
    /// One strike puts a batter out, and every run counts for two.
    SuddenDeath,
    /// Every fifth pitch is a golden ball: runs scored off it count for
    /// three, and a strike on it puts the batter out.
    GoldenBall,
    /// The ball keeps most of its speed when it bounces, comes off the
    /// wall like a ball off a cushion, and is kept in by the foul lines
    /// too. How much it keeps is set on the menu.
    PinballPark,
    /// A hit floats: it goes where it would have gone, taking several
    /// times as long to get there. How many times is set on the menu.
    MoonBall,
    /// Runners go round the bases several times as fast, and can be
    /// sent on at any time while the ball is in play, in the air or not.
    /// How fast is set on the menu.
    TurboRunners,
}

/// What the menu and the files know a mod by.
struct Info {
    /// The name it is saved under, and asked for by on the command line.
    key: &'static str,
    /// What the menu calls it, and what the menu says it does.
    name: &'static str,
    about: &'static str,
    /// What the menu calls its setting, if it has one besides being on or
    /// off, and the level that is at until it is set to another. A setting
    /// is a level, counted from 1.
    setting: Option<(&'static str, u8)>,
}

impl Mod {
    /// Every mod, in the order the menu lists them.
    pub const ALL: [Mod; 14] = [
        Mod::TimingIndicator,
        Mod::LonePitcher,
        Mod::ZingerHit,
        Mod::Butterfingers,
        Mod::Knuckleball,
        Mod::HeatCheck,
        Mod::MysteryPitch,
        Mod::CalledShot,
        Mod::HotBat,
        Mod::SuddenDeath,
        Mod::GoldenBall,
        Mod::PinballPark,
        Mod::MoonBall,
        Mod::TurboRunners,
    ];

    fn info(self) -> Info {
        match self {
            Mod::TimingIndicator => Info {
                key: "timing_indicator",
                name: "TIMING INDICATOR",
                about: "A BAR THAT SHOWS WHEN TO SWING",
                setting: None,
            },
            Mod::LonePitcher => Info {
                key: "lone_pitcher",
                name: "LONE PITCHER",
                about: "ONLY THE PITCHER GOES AFTER THE BALL",
                setting: None,
            },
            Mod::ZingerHit => Info {
                key: "zinger_hit",
                name: "ZINGER HIT",
                about: "EVERY HIT IS A HOME RUN, BIGGER THE BETTER TIMED",
                setting: None,
            },
            Mod::Butterfingers => Info {
                key: "butterfingers",
                name: "BUTTERFINGERS",
                about: "FIELDERS DROP AND FUMBLE THE BALL",
                setting: Some(("HOW OFTEN", 3)),
            },
            Mod::Knuckleball => Info {
                key: "knuckleball",
                name: "KNUCKLEBALL",
                about: "PITCHES SWAY, AND THE MARKER IS ONLY ROUGHLY RIGHT",
                setting: None,
            },
            Mod::HeatCheck => Info {
                key: "heat_check",
                name: "HEAT CHECK",
                about: "RUNS MAKE THE PITCHES FASTER, STRIKES SLOW THEM",
                setting: None,
            },
            Mod::MysteryPitch => Info {
                key: "mystery_pitch",
                name: "MYSTERY PITCH",
                about: "FASTBALL, CHANGE-UP OR CURVE: FIND OUT AS IT IS THROWN",
                setting: None,
            },
            Mod::CalledShot => Info {
                key: "called_shot",
                name: "CALLED SHOT",
                about: "CLICK THE OUTFIELD BEFORE A PITCH: LAND IT THERE FOR RUNS",
                setting: None,
            },
            Mod::HotBat => Info {
                key: "hot_bat",
                name: "HOT BAT",
                about: "EACH HIT IN A ROW WIDENS THE TIMING, A MISS RESETS IT",
                setting: None,
            },
            Mod::SuddenDeath => Info {
                key: "sudden_death",
                name: "SUDDEN DEATH",
                about: "ONE STRIKE AND YOU ARE OUT, BUT RUNS COUNT DOUBLE",
                setting: None,
            },
            Mod::GoldenBall => Info {
                key: "golden_ball",
                name: "GOLDEN BALL",
                about: "EVERY FIFTH PITCH IS GOLD: TRIPLE RUNS, OR OUT ON A MISS",
                setting: None,
            },
            Mod::PinballPark => Info {
                key: "pinball_park",
                name: "PINBALL PARK",
                about: "THE BALL BOUNCES OFF THE WALL AND THE GROUND, AND ON",
                setting: Some(("BOUNCE", 3)),
            },
            Mod::MoonBall => Info {
                key: "moon_ball",
                name: "MOON BALL",
                about: "EVERY HIT FLOATS: THE SAME FLIGHT, MANY TIMES SLOWER",
                setting: Some(("FLOAT", 2)),
            },
            Mod::TurboRunners => Info {
                key: "turbo_runners",
                name: "TURBO RUNNERS",
                about: "RUNNERS ARE FAST, AND CAN GO ON WITH THE BALL IN THE AIR",
                setting: Some(("SPEED", 2)),
            },
        }
    }

    /// The name the mod is saved under, and asked for by on the command
    /// line.
    pub fn key(self) -> &'static str {
        self.info().key
    }

    /// The mod with this key.
    pub fn from_key(key: &str) -> Option<Mod> {
        Mod::ALL.into_iter().find(|each| each.key() == key)
    }

    /// What the menu calls it.
    pub fn name(self) -> &'static str {
        self.info().name
    }

    /// What the menu says it does.
    pub fn about(self) -> &'static str {
        self.info().about
    }

    /// What the menu calls the mod's setting, if it has one besides being
    /// on or off.
    pub fn setting(self) -> Option<&'static str> {
        self.info().setting.map(|(name, _)| name)
    }

    /// The level the mod's setting is at until it is set to another.
    pub fn usual_level(self) -> u8 {
        self.info().setting.map_or(1, |(_, usual)| usual)
    }

    /// How many levels the mod's setting has. None, for a mod with no
    /// setting.
    pub fn levels(self, rules: &Rules) -> u8 {
        match self {
            Mod::Butterfingers => rules.butterfingers.levels(),
            Mod::PinballPark => rules.pinball.keeps.len() as u8,
            Mod::MoonBall => rules.moon.slow.len() as u8,
            Mod::TurboRunners => rules.turbo.speed.len() as u8,
            _ => 0,
        }
    }

    /// What the menu says a level of the mod's setting comes to.
    pub fn level_words(self, level: u8, rules: &Rules) -> String {
        match self {
            Mod::Butterfingers => format!("{}%", rules.butterfingers.chance_at(level)),
            Mod::PinballPark => {
                let keeps = crate::rules::level_of(&rules.pinball.keeps, level).unwrap_or(0.0);
                format!("{:.0}%", keeps * 100.0)
            }
            Mod::MoonBall => {
                let slow = crate::rules::level_of(&rules.moon.slow, level).unwrap_or(1.0);
                format!("{slow}X")
            }
            Mod::TurboRunners => {
                let speed = crate::rules::level_of(&rules.turbo.speed, level).unwrap_or(1.0);
                format!("{speed}X")
            }
            _ => String::new(),
        }
    }
}

/// The mods that are switched on, and what their settings are at. They
/// last from one game to the next.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    on: BTreeSet<Mod>,
    /// The levels that have been set. A mod not here is at its usual one.
    levels: BTreeMap<Mod, u8>,
}

/// The mods as they are written to their file: the keys of the ones that
/// are on, and the levels that have been set, by key.
#[derive(Default, Serialize, Deserialize)]
struct Saved {
    #[serde(default)]
    on: Vec<String>,
    #[serde(default)]
    levels: BTreeMap<String, u8>,
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

    /// The level a mod's setting is at, counted from 1. It keeps its level
    /// while the mod is off.
    pub fn level(&self, which: Mod) -> u8 {
        self.levels
            .get(&which)
            .copied()
            .unwrap_or_else(|| which.usual_level())
    }

    pub fn set_level(&mut self, which: Mod, level: u8) {
        self.levels.insert(which, level.max(1));
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
            levels: saved
                .levels
                .iter()
                .filter_map(|(key, &level)| Some((Mod::from_key(key)?, level.max(1))))
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
            levels: self
                .levels
                .iter()
                .map(|(which, &level)| (which.key().to_owned(), level))
                .collect(),
        };
        let text = toml::to_string(&saved).context("writing out the mods")?;
        std::fs::write(file, text).with_context(|| format!("writing {}", file.display()))
    }
}

/// One mod as the page lists it: the box, and the tick that shows in it.
/// A mod with a setting has a row of small boxes under it as well, one for
/// each level, filled up to the level it is at, and that level in words.
struct Line {
    which: Mod,
    button: Path,
    tick: Path,
    /// Each level's box, and what fills it.
    pips: Vec<(Path, Path)>,
    level_words: Option<Path>,
}

/// What a click on the mods' page asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asked {
    /// Switch this mod over.
    Switch(Mod),
    /// Set this mod's setting to this level.
    Level(Mod, u8),
}

/// What turns the pages of the list, when it has more than one: the arrows
/// back and on, and the words between them that say which page is up.
struct Pager {
    back: Path,
    on: Path,
    words: Path,
}

/// The page of the menu the mods are listed on. It is the high-score page
/// with the table's own drawings taken off its panel and the list put there
/// instead, so it arrives, leaves and has a way back as that page does. The
/// list has pages of its own, as many as it takes to hold every mod.
#[derive(Default)]
pub struct ModsPage {
    /// The panel has been made ready for the list. Its heading is kept, to
    /// tell when the panel has gone.
    heading: Option<Path>,
    pager: Option<Pager>,
    /// Which page of the list is up, counting from 0, which has been asked
    /// for, and how many there are.
    page: usize,
    wanted: usize,
    pages: usize,
    /// The lines of the page that is up, and everything on the panel that
    /// goes when the page is turned.
    lines: Vec<Line>,
    drawn: Vec<Path>,
    /// The depth the last thing was put on the panel at.
    depth: u16,
}

const DARK: Rgb = [0x0b, 0x3a, 0x5e];
const SOFT: Rgb = [0x4a, 0x6f, 0x8c];
const WHITE: Rgb = [0xff, 0xff, 0xff];

impl ModsPage {
    /// Where the first line's box goes on the panel, and how far down each
    /// line after it is.
    const FIRST: (f32, f32) = (-164.0, -78.0);
    const PITCH: f32 = 38.0;
    /// How much of the panel's height one page of the list may take up.
    const ROOM: f32 = 176.0;
    /// The sizes of a mod's name and of what is said about it, the
    /// lettering's own size being 1.
    const NAME_SIZE: f32 = 0.85;
    const ABOUT_SIZE: f32 = 0.6;
    /// How far under a mod's own line its setting is, how much room the
    /// setting takes, how far along the row the first of its boxes is, and
    /// how far apart they are.
    const SETTING_DOWN: f32 = 31.0;
    const SETTING_ROOM: f32 = 22.0;
    const PIPS_ALONG: f32 = 82.0;
    const PIP_PITCH: f32 = 14.0;
    /// The size of what fills a level's box, the art's block being 1, and
    /// how far into the box it sits.
    const FILL_SIZE: f32 = 0.58;
    const FILL_IN: f32 = 2.0;
    /// How far down the panel the arrows that turn the page are, how far
    /// across the one back, the words between them, and the one on.
    const PAGER_DOWN: f32 = 88.0;
    const PAGER_ACROSS: [f32; 3] = [-38.0, -30.0, 42.0];

    /// The mods that go on each page of the list, a page holding as many
    /// as there is room for.
    pub fn pages(rules: &Rules) -> Vec<Vec<Mod>> {
        let mut pages: Vec<Vec<Mod>> = Vec::new();
        let mut room = 0.0;
        for which in Mod::ALL {
            let needs = ModsPage::room_for(which, rules);
            if pages.is_empty() || room < needs {
                pages.push(Vec::new());
                room = ModsPage::ROOM;
            }
            room -= needs;
            pages.last_mut().expect("a page to put it on").push(which);
        }
        pages
    }

    /// How much of the panel's height a mod's line takes, with the row for
    /// its setting if it has one.
    fn room_for(which: Mod, rules: &Rules) -> f32 {
        let setting = which.setting().is_some() && which.levels(rules) > 0;
        ModsPage::PITCH + if setting { ModsPage::SETTING_ROOM } else { 0.0 }
    }

    /// Puts the list on the panel once the page has arrived, and keeps its
    /// ticks and levels true to `mods` for as long as it is up.
    pub fn show(
        &mut self,
        showing: bool,
        mods: &Mods,
        rules: &Rules,
        stage: &mut Stage,
        library: &Library,
    ) {
        // Everything goes when the panel does, as the page is left. The
        // list opens at its first page again the next time.
        if self
            .heading
            .as_ref()
            .is_some_and(|heading| stage.child(heading).is_none())
        {
            *self = ModsPage::default();
        }
        if !showing {
            return;
        }
        let Some(panel) =
            art::shell(stage).and_then(|shell| stage.find_symbol(&shell, art::SCORE_PANEL))
        else {
            return;
        };
        if self.heading.is_none() && !self.make_ready(&panel, rules, stage, library) {
            return;
        }
        if self.lines.is_empty() || self.wanted != self.page {
            for path in self.drawn.drain(..) {
                stage.remove(&path);
            }
            self.lines.clear();
            self.page = self.wanted;
            self.lay_out(&panel, rules, stage, library);
        }
        for line in &self.lines {
            if let Some(tick) = stage.child_mut(&line.tick) {
                tick.set_visible(mods.is_on(line.which));
            }
            let level = mods.level(line.which).min(line.pips.len() as u8);
            for (index, (_, fill)) in line.pips.iter().enumerate() {
                if let Some(fill) = stage.child_mut(fill) {
                    fill.set_visible(index < usize::from(level));
                }
            }
            if let Some(words) = line
                .level_words
                .as_ref()
                .and_then(|path| stage.child_mut(path))
            {
                let says = line.which.level_words(level, rules);
                if words.said.as_deref() != Some(says.as_str()) {
                    words.said = Some(says);
                }
            }
        }
        if let Some(pager) = &self.pager {
            let says = format!("PAGE {} OF {}", self.page + 1, self.pages);
            if let Some(words) = stage.child_mut(&pager.words)
                && words.said.as_deref() != Some(says.as_str())
            {
                words.said = Some(says);
            }
            // An arrow with nowhere to go is not there.
            for (arrow, there) in [
                (&pager.back, self.page > 0),
                (&pager.on, self.page + 1 < self.pages),
            ] {
                if let Some(arrow) = stage.child_mut(arrow) {
                    arrow.set_visible(there);
                }
            }
        }
    }

    /// What a click on the button at `path` asks for, if that is one of
    /// the page's boxes. A click on one of the arrows turns the page.
    pub fn clicked(&mut self, path: &[u16]) -> Option<Asked> {
        if let Some(pager) = &self.pager {
            if pager.back == path {
                self.wanted = self.page.saturating_sub(1);
            } else if pager.on == path {
                self.wanted = (self.page + 1).min(self.pages.saturating_sub(1));
            }
        }
        self.lines.iter().find_map(|line| {
            if line.button == path {
                return Some(Asked::Switch(line.which));
            }
            let pip = line.pips.iter().position(|(pip, _)| pip == path)?;
            Some(Asked::Level(line.which, pip as u8 + 1))
        })
    }

    /// Takes the score table's drawings off the panel and puts on what
    /// every page of the list has. Returns whether the panel was there to
    /// do it to.
    fn make_ready(
        &mut self,
        panel: &Path,
        rules: &Rules,
        stage: &mut Stage,
        library: &Library,
    ) -> bool {
        // What the panel was drawn with for the scores: its backing with
        // the table's tabs, the publisher's mark, and the notice.
        let table: Vec<Path> = stage.clip(panel).map_or(Vec::new(), |clip| {
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
            return false;
        }
        for path in table {
            if let Some(child) = stage.child_mut(&path) {
                child.set_visible(false);
            }
        }
        // A backing without the tabs goes under the panel's border, which
        // stays.
        stage.attach(
            panel,
            art::MODS_PANEL,
            art::MODS_PANEL_DEPTH,
            "modsPanel",
            library,
        );
        self.pages = ModsPage::pages(rules).len();
        let mut on = Panel {
            path: panel.clone(),
            depth: Stage::RULES_DEPTH + 300,
            library,
            added: Vec::new(),
        };
        self.heading = on.write(stage, "MODS", (-166.0, -123.0), 0.8, WHITE);
        if self.pages > 1 {
            let down = ModsPage::PAGER_DOWN;
            let [back, words, forward] = ModsPage::PAGER_ACROSS;
            let size = ModsPage::ABOUT_SIZE;
            let back = on.add(stage, art::PAGE_TURN, "modsBack", (back, down), 1.0);
            // The art's arrow points on. The one back is the same, turned
            // round.
            if let Some(arrow) = back.as_ref().and_then(|path| stage.child_mut(path)) {
                let mut turned = arrow.matrix;
                turned.a = -turned.a;
                arrow.set_matrix(turned);
            }
            let forward = on.add(stage, art::PAGE_TURN, "modsOn", (forward, down), 1.0);
            let words = on.write(stage, "", (words, down + 1.0), size, SOFT);
            if let (Some(back), Some(forward), Some(words)) = (back, forward, words) {
                self.pager = Some(Pager {
                    back,
                    on: forward,
                    words,
                });
            }
        }
        self.depth = on.depth;
        self.heading.is_some()
    }

    /// Puts the mods of the page that is up on the panel.
    fn lay_out(&mut self, panel: &Path, rules: &Rules, stage: &mut Stage, library: &Library) {
        let listed = ModsPage::pages(rules)
            .into_iter()
            .nth(self.page)
            .unwrap_or_default();
        let mut panel = Panel {
            path: panel.clone(),
            depth: self.depth,
            library,
            added: Vec::new(),
        };
        let (left, mut down) = ModsPage::FIRST;
        for which in listed {
            let words = left + 24.0;
            let size = ModsPage::NAME_SIZE;
            panel.write(stage, which.name(), (words, down - 5.0), size, DARK);
            let size = ModsPage::ABOUT_SIZE;
            panel.write(stage, which.about(), (words, down + 11.0), size, SOFT);
            // The box goes on after the words, so that its band lights the
            // whole line, and anywhere on the line ticks it.
            let button = panel.add(stage, art::MOD_BOX, "modBox", (left, down), 1.0);
            let tick = panel.add(stage, art::MOD_TICK, "modTick", (left, down), 1.0);
            let (Some(button), Some(tick)) = (button, tick) else {
                continue;
            };
            let mut line = Line {
                which,
                button,
                tick,
                pips: Vec::new(),
                level_words: None,
            };
            // Its setting goes under it, clear of the band, so that a
            // click on a level sets the level and does nothing else.
            let levels = which.levels(rules);
            if let (Some(setting), true) = (which.setting(), levels > 0) {
                let row = down + ModsPage::SETTING_DOWN;
                panel.write(stage, setting, (words, row), size, SOFT);
                let along =
                    |pip: u8| words + ModsPage::PIPS_ALONG + f32::from(pip) * ModsPage::PIP_PITCH;
                for pip in 0..levels {
                    let at = (along(pip), row + 1.0);
                    let inside = (at.0 + ModsPage::FILL_IN, at.1 + ModsPage::FILL_IN);
                    let small = panel.add(stage, art::MOD_PIP, "modPip", at, 1.0);
                    let fill =
                        panel.add(stage, art::BLOCK, "modPipFill", inside, ModsPage::FILL_SIZE);
                    if let (Some(small), Some(fill)) = (small, fill) {
                        if let Some(fill) = stage.child_mut(&fill) {
                            fill.set_color(look::tint(DARK));
                        }
                        line.pips.push((small, fill));
                    }
                }
                let after = (along(levels) + 4.0, row);
                line.level_words = panel.write(stage, "", after, size, DARK);
            }
            down += ModsPage::room_for(which, rules);
            self.lines.push(line);
        }
        self.depth = panel.depth;
        self.drawn = panel.added;
    }
}

/// The panel while things are being put on it.
struct Panel<'a> {
    path: Path,
    /// The depth the last thing was put at.
    depth: u16,
    library: &'a Library,
    /// Everything that has been put on it.
    added: Vec<Path>,
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
        self.added.push(path.clone());
        Some(path)
    }

    /// Writes a line of words on the panel, starting from a point.
    fn write(
        &mut self,
        stage: &mut Stage,
        text: &str,
        at: (f32, f32),
        size: f32,
        colour: Rgb,
    ) -> Option<Path> {
        let path = self.add(stage, art::LABEL_FIELD, "modsWords", at, size)?;
        let field = stage.child_mut(&path)?;
        field.said = Some(text.to_owned());
        field.set_color(look::tint(colour));
        Some(path)
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
    fn the_list_has_as_many_pages_as_it_takes_and_every_mod_is_on_one() {
        let rules = Rules::default();
        let pages = ModsPage::pages(&rules);
        let listed: Vec<Mod> = pages.iter().flatten().copied().collect();
        assert_eq!(listed, Mod::ALL);
        for page in &pages {
            assert!(!page.is_empty());
            let room: f32 = page
                .iter()
                .map(|&which| ModsPage::room_for(which, &rules))
                .sum();
            assert!(room <= ModsPage::ROOM, "{page:?}");
        }
        // The first four fit on a page between them.
        assert_eq!(pages[0].len(), 4);
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
    fn a_setting_is_at_its_usual_level_until_it_is_set_and_keeps_it_while_off() {
        let rules = Rules::default();
        let mut mods = Mods::default();
        assert_eq!(Mod::Butterfingers.setting(), Some("HOW OFTEN"));
        assert_eq!(Mod::Butterfingers.levels(&rules), 5);
        assert_eq!(mods.level(Mod::Butterfingers), 3);
        assert_eq!(Mod::Butterfingers.level_words(3, &rules), "60%");
        mods.set_level(Mod::Butterfingers, 5);
        assert_eq!(mods.level(Mod::Butterfingers), 5);
        assert!(!mods.is_on(Mod::Butterfingers));
        mods.toggle(Mod::Butterfingers);
        mods.toggle(Mod::Butterfingers);
        assert_eq!(mods.level(Mod::Butterfingers), 5);
        // A mod with no setting has no levels.
        assert_eq!(Mod::LonePitcher.setting(), None);
        assert_eq!(Mod::LonePitcher.levels(&rules), 0);
    }

    #[test]
    fn the_choice_comes_back_as_it_was_saved() {
        let file = std::env::temp_dir().join(format!("bb-mods-{}.toml", std::process::id()));
        let mut mods = Mods::default();
        mods.set(Mod::TimingIndicator, true);
        mods.set(Mod::Butterfingers, true);
        mods.set_level(Mod::Butterfingers, 4);
        mods.save(&file).unwrap();
        assert_eq!(Mods::load(&file), mods);
        mods.set(Mod::Butterfingers, false);
        mods.levels.clear();
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
