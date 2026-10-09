//! The menu's pages for a full match: the choice of ground, and the
//! summary of what is about to be played.

mod summary;

use bb_engine::display::Path;
use bb_engine::stage::Stage;

// The game was this file's once, and is still found here.
use super::{Menu, MenuPage};
use crate::art;
use crate::look::{self, Rgb};
use crate::rng::{Rng, mixed_with};
use crate::settings::{Ground, Settings};
use crate::sheet::Sheet;

/// What fills the box of the ground chosen: how far into the box it sits,
/// its size, the art's block being 1, and its colour.
const FILL_IN: f32 = 2.0;
const FILL_SIZE: f32 = 0.58;
const FILL_COLOUR: Rgb = look::NAVY;

/// On the setup page, where the choice of ground is: the middle of the top
/// of its heading, the corner of the first of its boxes, how far apart the
/// boxes are, and where each one's word is from its box.
const GROUND_HEADING: (f32, f32) = (452.5, 280.0);
const GROUND_FIRST: (f32, f32) = (357.0, 309.0);
const GROUND_PITCH: f32 = 66.0;
const GROUND_WORD: (f32, f32) = (37.0, -5.0);

/// The lettering the full match's pages are written in is drawn 18 high.
/// These are the sizes of its lines, that being 1.
const HEADING_SIZE: f32 = 17.0 / 18.0;
const WORDS_SIZE: f32 = 14.0 / 18.0;
const CHOICE_SIZE: f32 = 12.5 / 18.0;
const WHITE: Rgb = look::WHITE;

/// The choice of ground on the full match's setup page: the clip it is all
/// in, and each ground's box.
pub(super) struct Grounds {
    holder: Path,
    boxes: Vec<GroundBox>,
}

/// One ground to choose: its box, and what fills the box when it is the one
/// chosen.
struct GroundBox {
    ground: Ground,
    button: Path,
    fill: Path,
}

impl Menu {
    /// Whether the side is at home in the full match about to begin: as
    /// chosen, or as a coin comes down. Once asked, the answer stands until
    /// it is taken.
    pub(super) fn at_home(&mut self, settings: &Settings) -> bool {
        if let Some(home) = self.home {
            return home;
        }
        let home = match settings.ground {
            Ground::Home => true,
            Ground::Away => false,
            Ground::Toss => {
                let coin = self
                    .coin
                    .get_or_insert_with(|| Rng::new(Rng::seed_from_clock() ^ mixed_with::THE_COIN));
                coin.below(2) == 0
            }
        };
        self.home = Some(home);
        home
    }

    /// The same, for the match that is now beginning. The next one is
    /// settled afresh.
    pub fn take_home(&mut self, settings: &Settings) -> bool {
        let home = self.at_home(settings);
        self.home = None;
        home
    }

    /// Takes down what a full match has put on the menu's pages.
    pub(super) fn clear_full(&mut self, stage: &mut Stage) {
        if let Some(grounds) = self.grounds.take() {
            stage.remove(&grounds.holder);
        }
        if let Some(holder) = self.summary.take() {
            stage.remove(&holder);
        }
    }

    /// On the full match's setup page, puts the choice of ground under the
    /// skill levels once the page is there, and keeps the box of the one
    /// chosen filled.
    pub(super) fn show_grounds(&mut self, settings: &Settings, stage: &mut Stage) {
        // The menu may have been left and come back to since.
        if let Some(grounds) = &self.grounds
            && stage.child(&grounds.holder).is_none()
        {
            self.grounds = None;
        }
        if self.page != MenuPage::FullSetup || self.arriving {
            return;
        }
        if self.grounds.is_none() {
            let Some(menu) = art::in_shell(stage, art::MENU) else {
                return;
            };
            let depth = Stage::RULES_DEPTH + 500;
            let Some(holder) = stage.attach(&menu, art::HOLDER, depth, "grounds") else {
                return;
            };
            let mut sheet = Sheet::on(holder.clone(), 1).lettered(art::MENU_FIELD);
            let (heading, size) = (GROUND_HEADING, WORDS_SIZE);
            sheet.write(
                stage,
                "groundHeading",
                "Home or Away:",
                heading,
                size,
                WHITE,
            );
            let mut boxes = Vec::new();
            for (index, ground) in Ground::ALL.into_iter().enumerate() {
                // Each ground's things have ten depths to themselves.
                sheet.depth = 10 + index as u16 * 10;
                let at = (GROUND_FIRST.0 + GROUND_PITCH * index as f32, GROUND_FIRST.1);
                let word = (at.0 + GROUND_WORD.0, at.1 + GROUND_WORD.1);
                let size = CHOICE_SIZE;
                sheet.write(stage, "groundWord", ground.word(), word, size, WHITE);
                // The box goes on after its word, so that a click on the
                // word is a click on the box.
                let button = sheet.add(stage, art::CHOICE, "ground", at, (1.0, 1.0));
                let inside = (at.0 + FILL_IN, at.1 + FILL_IN);
                let size = (FILL_SIZE, FILL_SIZE);
                let fill = sheet.add(stage, art::BLOCK, "groundFill", inside, size);
                if let (Some(button), Some(fill)) = (button, fill) {
                    if let Some(fill) = stage.child_mut(&fill) {
                        fill.set_color(look::tint(FILL_COLOUR));
                    }
                    boxes.push(GroundBox {
                        ground,
                        button,
                        fill,
                    });
                }
            }
            self.grounds = Some(Grounds { holder, boxes });
        }
        for each in self.grounds.iter().flat_map(|grounds| &grounds.boxes) {
            if let Some(fill) = stage.child_mut(&each.fill) {
                fill.set_visible(each.ground == settings.ground);
            }
        }
    }

    /// Takes in a click on the button at `path`, which may be one of the
    /// choices of ground on the full match's setup page.
    pub fn chose(&mut self, path: &[u16], settings: &mut Settings) {
        let chosen = self
            .grounds
            .iter()
            .flat_map(|grounds| &grounds.boxes)
            .find(|each| each.button == path);
        if let Some(chosen) = chosen {
            settings.ground = chosen.ground;
        }
    }
}
