//! The summary of a full match that is about to be played: the ground,
//! the skill, and the mods that are on.

use bb_engine::stage::Stage;

// The game was this file's once, and is still found here.
use super::{HEADING_SIZE, WHITE, WORDS_SIZE};
use super::{Menu, MenuPage};
use crate::art;
use crate::game::Game;
use crate::settings::Ground;
use crate::sheet::Sheet;

/// On the summary page: where the badge at the start of the heading is, the
/// middle of the top of the heading's words, the middle of the lines under
/// it, and how far down each of those is.
const SUMMARY_BADGE: (f32, f32) = (333.0, 75.1);
const SUMMARY_HEADING: (f32, f32) = (421.0, 75.0);
const SUMMARY_MIDDLE: f32 = 400.0;
const SUMMARY_DOWN: [f32; 5] = [136.0, 172.0, 191.0, 227.0, 246.0];

impl Menu {
    /// The lines of the full match's summary page.
    pub fn summary_lines(game: &Game, home: bool) -> [String; 5] {
        let innings = game.rules.full_match.innings;
        let how = match game.settings.ground {
            Ground::Toss => "The coin is tossed, and",
            Ground::Home | Ground::Away => "As you asked,",
        };
        let side = if home {
            "you are at home and bat second."
        } else {
            "you are away and bat first."
        };
        [
            format!("IT'S A FULL MATCH OF {innings} INNINGS!"),
            how.to_owned(),
            side.to_owned(),
            "The other side's innings are".to_owned(),
            "played for you. You just bat.".to_owned(),
        ]
    }

    /// On the full match's summary page, takes the art's words about the
    /// last innings out of sight and writes what a full match is to be.
    pub(crate) fn show_summary(&mut self, game: &Game, stage: &mut Stage) {
        if let Some(holder) = &self.summary
            && stage.child(holder).is_none()
        {
            self.summary = None;
        }
        if !matches!(self.page, MenuPage::FullSummary | MenuPage::ToFull) {
            return;
        }
        let Some(menu) = art::in_shell(stage, art::MENU) else {
            return;
        };
        let Some(clip) = stage.clip(&menu) else {
            return;
        };
        let (frame, last) = (clip.frame, clip.frame_count(stage.library()));
        let theirs: Vec<u16> = clip
            .children
            .iter()
            .filter(|(_, child)| art::SUMMARY_WORDS.contains(&child.symbol))
            .map(|(&depth, _)| depth)
            .collect();
        for depth in theirs {
            let mut path = menu.clone();
            path.push(depth);
            if let Some(child) = stage.child_mut(&path) {
                child.set_visible(false);
            }
        }
        if self.page == MenuPage::ToFull {
            // The page fades away over the frames that are left, and what
            // was written on it with it.
            let labels = stage
                .library()
                .timeline(Some(art::MENU))
                .map(|timeline| &timeline.labels);
            let first = labels
                .and_then(|labels| labels.get(MenuPage::ToFull.label()))
                .copied()
                .unwrap_or(frame);
            let over = f32::from(last.saturating_sub(first).max(1));
            let left = 1.0 - f32::from(frame.saturating_sub(first)) / over;
            if let Some(holder) = self.summary.as_ref().and_then(|path| stage.child_mut(path)) {
                holder.set_alpha(left.clamp(0.0, 1.0));
            }
            return;
        }
        if self.summary.is_some() || self.arriving {
            return;
        }
        let depth = Stage::RULES_DEPTH + 510;
        let Some(holder) = stage.attach(&menu, art::HOLDER, depth, "fullSummary") else {
            return;
        };
        let mut sheet = Sheet::on(holder.clone(), 1).lettered(art::MENU_FIELD);
        let badge = SUMMARY_BADGE;
        for symbol in art::BADGE {
            sheet.add(stage, symbol, "badge", badge, (1.0, 1.0));
        }
        let nine = (
            badge.0 + art::NINE_FROM_BADGE.0,
            badge.1 + art::NINE_FROM_BADGE.1,
        );
        sheet.add(stage, art::NINE, "nine", nine, (1.0, 1.0));
        let (heading, size) = (SUMMARY_HEADING, HEADING_SIZE);
        sheet.write(stage, "fullHeading", "FULL MATCH", heading, size, WHITE);
        let home = self.at_home(&game.settings);
        let lines = Menu::summary_lines(game, home);
        // The lines go on over all of that, from a depth of their own.
        sheet.depth = 10;
        for (line, down) in lines.iter().zip(SUMMARY_DOWN) {
            let top = (SUMMARY_MIDDLE, down);
            sheet.write(stage, "fullLine", line, top, WORDS_SIZE, WHITE);
        }
        self.summary = Some(holder);
    }
}
