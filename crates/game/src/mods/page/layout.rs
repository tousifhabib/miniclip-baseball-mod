//! Laying a page of the list out: a box, a name and a line of words for
//! each mod on it, and the pips of one that has a setting.

use bb_engine::display::Path;
use bb_engine::stage::Stage;

use super::{Line, ModsPage, Pager};
use crate::art;
use crate::look::{self, Rgb};
use crate::rules::Rules;
use crate::sheet::Sheet;

/// What every line of words on the panel is named.
const WORDS: &str = "modsWords";

const WHITE: Rgb = look::WHITE;

const SOFT: Rgb = [0x4a, 0x6f, 0x8c];

const DARK: Rgb = look::NAVY;

impl ModsPage {
    /// Takes the score table's drawings off the panel and puts on what
    /// every page of the list has. Returns whether the panel was there to
    /// do it to.
    pub(super) fn make_ready(&mut self, panel: &Path, rules: &Rules, stage: &mut Stage) -> bool {
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
        stage.attach(panel, art::MODS_PANEL, art::MODS_PANEL_DEPTH, "modsPanel");
        self.pages = ModsPage::pages(rules).len();
        let mut on = Sheet::on(panel.clone(), Stage::RULES_DEPTH + 301);
        self.heading = on.label(stage, WORDS, "MODS", (-166.0, -123.0), 0.8, WHITE);
        if self.pages > 1 {
            let down = ModsPage::PAGER_DOWN;
            let [back, words, forward] = ModsPage::PAGER_ACROSS;
            let size = ModsPage::ABOUT_SIZE;
            let back = on.add(stage, art::PAGE_TURN, "modsBack", (back, down), (1.0, 1.0));
            // The art's arrow points on. The one back is the same, turned
            // round.
            if let Some(arrow) = back.as_ref().and_then(|path| stage.child_mut(path)) {
                let mut turned = arrow.matrix;
                turned.a = -turned.a;
                arrow.set_matrix(turned);
            }
            let forward = on.add(stage, art::PAGE_TURN, "modsOn", (forward, down), (1.0, 1.0));
            let words = on.label(stage, WORDS, "", (words, down + 1.0), size, SOFT);
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

    /// Puts a mod's setting under its line: what it is called, a small box
    /// for each level with what fills it, and room after them for the
    /// level in words. `from` is where the name of the setting begins.
    fn lay_out_setting(
        line: &mut Line,
        setting: &str,
        levels: u8,
        from: (f32, f32),
        panel: &mut Sheet,
        stage: &mut Stage,
    ) {
        let (words, row) = from;
        let size = ModsPage::ABOUT_SIZE;
        panel.label(stage, WORDS, setting, (words, row), size, SOFT);
        let along = |pip: u8| words + ModsPage::PIPS_ALONG + f32::from(pip) * ModsPage::PIP_PITCH;
        for pip in 0..levels {
            let at = (along(pip), row + 1.0);
            let inside = (at.0 + ModsPage::FILL_IN, at.1 + ModsPage::FILL_IN);
            let small = panel.add(stage, art::MOD_PIP, "modPip", at, (1.0, 1.0));
            let fill = (ModsPage::FILL_SIZE, ModsPage::FILL_SIZE);
            let fill = panel.add(stage, art::BLOCK, "modPipFill", inside, fill);
            if let (Some(small), Some(fill)) = (small, fill) {
                if let Some(fill) = stage.child_mut(&fill) {
                    fill.set_color(look::tint(DARK));
                }
                line.pips.push((small, fill));
            }
        }
        let after = (along(levels) + 4.0, row);
        line.level_words = panel.label(stage, WORDS, "", after, size, DARK);
    }

    /// Puts the mods of the page that is up on the panel.
    pub(super) fn lay_out(&mut self, panel: &Path, rules: &Rules, stage: &mut Stage) {
        let listed = ModsPage::pages(rules)
            .into_iter()
            .nth(self.page)
            .unwrap_or_default();
        let mut panel = Sheet::on(panel.clone(), self.depth);
        let (left, mut down) = ModsPage::FIRST;
        for which in listed {
            let words = left + 24.0;
            let size = ModsPage::NAME_SIZE;
            panel.label(stage, WORDS, which.name(), (words, down - 5.0), size, DARK);
            let size = ModsPage::ABOUT_SIZE;
            panel.label(
                stage,
                WORDS,
                which.about(),
                (words, down + 11.0),
                size,
                SOFT,
            );
            // The box goes on after the words, so that its band lights the
            // whole line, and anywhere on the line ticks it.
            let button = panel.add(stage, art::MOD_BOX, "modBox", (left, down), (1.0, 1.0));
            let tick = panel.add(stage, art::MOD_TICK, "modTick", (left, down), (1.0, 1.0));
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
                let from = (words, down + ModsPage::SETTING_DOWN);
                ModsPage::lay_out_setting(&mut line, setting, levels, from, &mut panel, stage);
            }
            down += ModsPage::room_for(which, rules);
            self.lines.push(line);
        }
        self.drawn = panel.put_since(self.depth);
        self.depth = panel.depth;
    }
}
