//! What a full match and a tournament each do with the page the art has
//! for saying what a match is to be: its own words are taken out of sight,
//! theirs are written in their place, and those fade away with the page.

use bb_engine::display::Path;
use bb_engine::stage::Stage;

use super::{Menu, MenuPage};
use crate::art;

/// On the summary page: the middle of the lines under the heading, and how
/// far down each of them is.
pub(super) const SUMMARY_MIDDLE: f32 = 400.0;
pub(super) const SUMMARY_DOWN: [f32; 5] = [136.0, 172.0, 191.0, 227.0, 246.0];

impl Menu {
    /// Called every frame, for the page `shown` and the one it is `fading`
    /// out on. While either is up, the art's own words are kept off the
    /// page, and on the way out what was written there fades with it.
    /// Returns the menu's clip when the page has arrived with nothing yet
    /// written on it, which is when it is to be written.
    pub(super) fn ready_the_summary(
        &mut self,
        shown: MenuPage,
        fading: MenuPage,
        stage: &mut Stage,
    ) -> Option<Path> {
        if let Some(holder) = &self.summary
            && stage.child(holder).is_none()
        {
            self.summary = None;
        }
        if self.page != shown && self.page != fading {
            return None;
        }
        let menu = art::in_shell(stage, art::MENU)?;
        let clip = stage.clip(&menu)?;
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
        if self.page == fading {
            // The page fades away over the frames that are left, and what
            // was written on it with it.
            let labels = stage
                .library()
                .timeline(Some(art::MENU))
                .map(|timeline| &timeline.labels);
            let first = labels
                .and_then(|labels| labels.get(fading.label()))
                .copied()
                .unwrap_or(frame);
            let over = f32::from(last.saturating_sub(first).max(1));
            let left = 1.0 - f32::from(frame.saturating_sub(first)) / over;
            if let Some(holder) = self.summary.as_ref().and_then(|path| stage.child_mut(path)) {
                holder.set_alpha(left.clamp(0.0, 1.0));
            }
            return None;
        }
        if self.summary.is_some() || self.arriving {
            return None;
        }
        Some(menu)
    }
}
