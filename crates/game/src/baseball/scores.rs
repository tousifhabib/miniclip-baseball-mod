//! The table of high scores, as the scores screen shows it.

use bb_engine::display::Path;
use bb_engine::library::Library;
use bb_engine::stage::Stage;

use super::Baseball;
use super::screen::Screen;
use crate::art::{self};
use crate::look::{self, Rgb};
use crate::menu::MenuPage;
use crate::sheet::Sheet;

impl Baseball {
    /// Writes the score table over the panel on the high-score page, for as
    /// long as that page is up.
    pub(super) fn show_scores(&mut self, stage: &mut Stage, library: &Library) {
        // The lines go when the panel does, as the page is left.
        if self
            .table
            .first()
            .is_some_and(|line| stage.child(line).is_none())
        {
            self.table.clear();
        }
        let on_page = self.screen == Screen::Menu && self.menu.page() == MenuPage::HighScores;
        if !on_page || !self.table.is_empty() {
            return;
        }
        let Some(panel) =
            art::shell(stage).and_then(|shell| stage.find_symbol(&shell, art::SCORE_PANEL))
        else {
            return;
        };
        // The panel says the scores are kept on a web site. Here they are
        // not, so that goes and the table takes its place.
        let notice: Vec<Path> = stage.clip(&panel).map_or(Vec::new(), |clip| {
            clip.children
                .iter()
                .filter(|(_, child)| art::SCORE_PANEL_NOTICE.contains(&child.symbol))
                .map(|(&depth, _)| {
                    let mut path = panel.clone();
                    path.push(depth);
                    path
                })
                .collect()
        });
        if notice.is_empty() {
            // The panel has not finished arriving.
            return;
        }
        for path in notice {
            if let Some(child) = stage.child_mut(&path) {
                child.set_visible(false);
            }
        }
        // What to write, where its middle goes, and in what colour: dark
        // on the panel's white, and white for the heading on its bar. The
        // heading was drawn in one piece with the notice, so it is written
        // back in.
        const DARK: Rgb = look::NAVY;
        const WHITE: Rgb = look::WHITE;
        let mut lines = vec![("HIGHSCORES".to_owned(), -104.0, -123.0, WHITE)];
        if self.scores.entries.is_empty() {
            lines.push(("NO SCORES YET".to_owned(), 0.0, -10.0, DARK));
        }
        for (place, entry) in self.scores.entries.iter().enumerate() {
            let down = -88.0 + place as f32 * 19.0;
            lines.push((format!("{}", place + 1), -150.0, down, DARK));
            lines.push((entry.name.to_uppercase(), -35.0, down, DARK));
            lines.push((entry.points.to_string(), 125.0, down, DARK));
        }
        const SIZE: f32 = 0.8;
        let mut sheet = Sheet::on(panel, Stage::RULES_DEPTH + 200, library);
        for (text, across, down, colour) in lines {
            let line = sheet.write_plain(stage, "scoreLine", &text, (across, down), SIZE, colour);
            self.table.extend(line);
        }
    }
}
