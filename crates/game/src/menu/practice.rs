//! The menu's page for batting practice: on the setup page, the choice of
//! view and of whether the timing bar is up.

use bb_engine::display::Path;
use bb_engine::stage::Stage;

use super::{CHOICE_SIZE, Menu, MenuPage, WHITE};
use crate::art;
use crate::choice::{Choice, Row};
use crate::look;
use crate::settings::{PracticeView, Settings};
use crate::sheet::Sheet;

/// On the setup page, under the skill levels: the middle of the top of the
/// word that says what each row is of, and the row.
const VIEW_WORD: (f32, f32) = (381.0, 279.0);
const VIEWS: Row = Row {
    first: (434.0, 284.0),
    pitch: 70.0,
    word: (36.0, -5.0),
    begins: false,
    a_letter: 0.0,
    size: CHOICE_SIZE,
    colour: WHITE,
    fill: look::NAVY,
    names: ["viewWord", "view", "viewFill"],
};
const BAR_WORD: (f32, f32) = (381.0, 306.0);
const BARS: Row = Row {
    first: (434.0, 311.0),
    pitch: 70.0,
    word: (24.0, -5.0),
    begins: false,
    a_letter: 0.0,
    size: CHOICE_SIZE,
    colour: WHITE,
    fill: look::NAVY,
    names: ["barWord", "bar", "barFill"],
};

/// What batting practice has put on the setup page: the clip it is all in,
/// and the two rows of boxes.
pub(super) struct Practice {
    holder: Path,
    views: Choice<PracticeView>,
    bars: Choice<bool>,
}

impl Menu {
    /// Takes down what batting practice has put on the setup page.
    pub(super) fn clear_practice(&mut self, stage: &mut Stage) {
        if let Some(practice) = self.practice.take() {
            stage.remove(&practice.holder);
        }
    }

    /// On the setup page for batting practice, puts the two choices under
    /// the skill levels once the page is there, and keeps the boxes of the
    /// ones chosen filled.
    pub(super) fn show_practice(&mut self, settings: &Settings, stage: &mut Stage) {
        // The menu may have been left and come back to since.
        if let Some(practice) = &self.practice
            && stage.child(&practice.holder).is_none()
        {
            self.practice = None;
        }
        if self.page != MenuPage::PracticeSetup || self.arriving {
            return;
        }
        if self.practice.is_none() {
            let Some(menu) = art::in_shell(stage, art::MENU) else {
                return;
            };
            let depth = Stage::RULES_DEPTH + 540;
            let Some(holder) = stage.attach(&menu, art::HOLDER, depth, "practiceChoices") else {
                return;
            };
            let mut sheet = Sheet::on(holder.clone(), 1).lettered(art::MENU_FIELD);
            let size = CHOICE_SIZE;
            sheet.write(stage, "viewHeading", "View:", VIEW_WORD, size, WHITE);
            let views = PracticeView::ALL.map(|view| (view, view.word()));
            let views = Choice::put(&views, &VIEWS, 10, &mut sheet, stage);
            sheet.depth = 50;
            sheet.write(stage, "barHeading", "Timing bar:", BAR_WORD, size, WHITE);
            let bars = Choice::put(
                &[(true, "On"), (false, "Off")],
                &BARS,
                60,
                &mut sheet,
                stage,
            );
            self.practice = Some(Practice {
                holder,
                views,
                bars,
            });
        }
        if let Some(practice) = &self.practice {
            practice.views.show(settings.practice_view, stage);
            practice.bars.show(settings.practice_bar, stage);
        }
    }

    /// Takes in a click on the button at `path`, which may be one of the
    /// boxes on the setup page for batting practice.
    pub(super) fn chose_of_practice(&self, path: &[u16], settings: &mut Settings) {
        let Some(practice) = &self.practice else {
            return;
        };
        if let Some(view) = practice.views.clicked(path) {
            settings.practice_view = view;
        }
        if let Some(bar) = practice.bars.clicked(path) {
            settings.practice_bar = bar;
        }
    }
}
