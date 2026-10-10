//! The menu's pages for a full match: the choice of ground, and the
//! summary of what is about to be played.

mod summary;

use bb_engine::display::Path;
use bb_engine::stage::Stage;

use super::{CHOICE_SIZE, Menu, MenuPage, WHITE, WORDS_SIZE};
use crate::art;
use crate::choice::{Choice, Row};
use crate::look::{self, Rgb};
use crate::rng::{Rng, mixed_with};
use crate::settings::{Ground, Settings};
use crate::sheet::Sheet;

/// What fills the box of the ground chosen.
const FILL_COLOUR: Rgb = look::NAVY;

/// On the setup page, where the choice of ground is: the middle of the top
/// of its heading, the corner of the first of its boxes, how far apart the
/// boxes are, and where each one's word is from its box.
const GROUND_HEADING: (f32, f32) = (452.5, 280.0);
const GROUND_FIRST: (f32, f32) = (357.0, 309.0);
const GROUND_PITCH: f32 = 66.0;
const GROUND_WORD: (f32, f32) = (37.0, -5.0);

/// The row of boxes the ground is chosen by.
const GROUNDS: Row = Row {
    first: GROUND_FIRST,
    pitch: GROUND_PITCH,
    word: GROUND_WORD,
    begins: false,
    a_letter: 0.0,
    size: CHOICE_SIZE,
    colour: WHITE,
    fill: FILL_COLOUR,
    names: ["groundWord", "ground", "groundFill"],
};

/// The choice of ground on the full match's setup page: the clip it is all
/// in, and a box for each ground.
pub(super) struct Grounds {
    holder: Path,
    boxes: Choice<Ground>,
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

    /// Takes down what a full match or a tournament has put on the menu's
    /// pages.
    pub(super) fn clear_full(&mut self, stage: &mut Stage) {
        if let Some(grounds) = self.grounds.take() {
            stage.remove(&grounds.holder);
        }
        if let Some(holder) = self.summary.take() {
            stage.remove(&holder);
        }
        self.clear_tournament(stage);
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
            let all = Ground::ALL.map(|ground| (ground, ground.word()));
            let boxes = Choice::put(&all, &GROUNDS, 10, &mut sheet, stage);
            self.grounds = Some(Grounds { holder, boxes });
        }
        if let Some(grounds) = &self.grounds {
            grounds.boxes.show(settings.ground, stage);
        }
    }

    /// Takes in a click on the button at `path`, which may be one of the
    /// choices of ground on the full match's setup page, or one of the
    /// boxes a tournament has put on its pages.
    pub fn chose(&mut self, path: &[u16], settings: &mut Settings) {
        let chosen = self
            .grounds
            .as_ref()
            .and_then(|grounds| grounds.boxes.clicked(path));
        if let Some(ground) = chosen {
            settings.ground = ground;
        }
        self.chose_of_a_tournament(path, settings);
    }
}
