//! The page of the menu that lists the mods, a few to a page.

mod layout;

use bb_engine::display::Path;
use bb_engine::library::Library;
use bb_engine::stage::Stage;

use super::Mod;
use super::chosen::Mods;
use crate::art;
use crate::rules::Rules;

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
    /// The depth the next thing put on the panel goes at.
    depth: u16,
}

impl ModsPage {
    /// Where the first line's box goes on the panel, and how far down each
    /// line after it is.
    const FIRST: (f32, f32) = (-164.0, -78.0);
    const PITCH: f32 = 38.0;
    /// How much of the panel's height one page of the list may take up.
    pub(super) const ROOM: f32 = 176.0;
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
    pub(super) fn room_for(which: Mod, rules: &Rules) -> f32 {
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
}
