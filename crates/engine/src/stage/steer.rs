//! What the rules do to the stage: add a thing, take one away, send a
//! clip to a frame, set the words of a field.

use std::sync::Arc;

use bb_format::SymbolId;

use super::Stage;
use crate::display::Path;

impl Stage {
    /// Jumps the top timeline to `frame`.
    pub fn goto(&mut self, frame: u16) {
        self.goto_clip(&[], frame);
    }

    /// Puts a new instance of `symbol` inside the clip at `parent`, at
    /// `depth`, as `attachMovie` did. Use depths from
    /// [`Stage::RULES_DEPTH`] up. Returns where the new object is.
    pub fn attach(
        &mut self,
        parent: &[u16],
        symbol: SymbolId,
        depth: u16,
        name: &str,
    ) -> Option<Path> {
        let library = Arc::clone(&self.library);
        let mut events = std::mem::take(&mut self.events);
        let made = self.clip_mut(parent).is_some_and(|clip| {
            clip.attach(
                symbol,
                depth,
                Some(name),
                &library,
                &mut events,
                &mut parent.to_vec(),
            )
            .is_some()
        });
        self.events = events;
        made.then(|| {
            let mut path = parent.to_vec();
            path.push(depth);
            path
        })
    }

    /// Takes the object at `path` off the stage. Returns whether it was
    /// there.
    pub fn remove(&mut self, path: &[u16]) -> bool {
        let Some((depth, parent)) = path.split_last() else {
            return false;
        };
        self.clip_mut(parent)
            .is_some_and(|clip| clip.children.remove(depth).is_some())
    }

    /// Jumps the clip at `path` to `frame`.
    pub fn goto_clip(&mut self, path: &[u16], frame: u16) {
        let library = Arc::clone(&self.library);
        let mut events = std::mem::take(&mut self.events);
        if let Some(clip) = self.clip_mut(path) {
            clip.goto(frame, &library, &mut events, &mut path.to_vec());
        }
        self.events = events;
    }

    /// Jumps the clip at `path` to the frame with this label, and sets it
    /// playing or stopped. Returns whether there is such a clip and label.
    pub fn goto_label(&mut self, path: &[u16], label: &str, play: bool) -> bool {
        let library = Arc::clone(&self.library);
        let mut events = std::mem::take(&mut self.events);
        let found = self.clip_mut(path).is_some_and(|clip| {
            let found = clip.goto_label(label, &library, &mut events, &mut path.to_vec());
            if found {
                clip.playing = play;
            }
            found
        });
        self.events = events;
        found
    }

    /// Makes every text field that shows `variable` say `value`.
    pub fn set_text(&mut self, variable: &str, value: impl Into<String>) {
        self.texts.insert(variable.to_owned(), value.into());
    }
}
