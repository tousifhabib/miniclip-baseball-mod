//! Plays a timeline: moves a clip's playhead, and keeps its display list as
//! the frame it is on has it.
//!
//! This follows Flash's rules. A timeline stores changes per frame, so moving
//! the playhead forward applies each frame's changes in turn, and moving it
//! back replays from frame 1. An object that the replay puts back where it
//! already was is kept, not rebuilt, so a nested clip keeps its own position.

#[cfg(test)]
mod frame_tests;
mod placing;
#[cfg(test)]
mod properties;

use std::collections::BTreeMap;

use bb_format::{Op, PlaceAction, SymbolId};

use super::{Child, Children, ClipState, Content, Event, Path};
use crate::library::Library;
use placing::{apply, new_child};

impl ClipState {
    /// Creates an instance showing its first frame. `path` is where the
    /// instance sits in the tree, which the events it reports carry.
    pub fn new(
        symbol: Option<SymbolId>,
        library: &Library,
        events: &mut Vec<Event>,
        path: &mut Path,
    ) -> ClipState {
        let mut clip = ClipState {
            symbol,
            frame: 0,
            playing: true,
            children: Children::new(),
        };
        clip.goto(1, library, events, path);
        clip
    }

    /// Moves everything on by one frame.
    pub fn advance(&mut self, library: &Library, events: &mut Vec<Event>, path: &mut Path) {
        // Objects already here move on before this timeline does, and objects
        // this timeline adds now stay on their first frame until the next
        // tick. That is the order Flash uses.
        advance_children(&mut self.children, library, events, path);
        if !self.playing {
            return;
        }
        let count = self.frame_count(library);
        if count <= 1 {
            return;
        }
        let next = if self.frame >= count {
            1
        } else {
            self.frame + 1
        };
        self.goto(next, library, events, path);
    }

    /// Moves the playhead to the frame with this label. Returns whether the
    /// timeline has such a label.
    pub fn goto_label(
        &mut self,
        label: &str,
        library: &Library,
        events: &mut Vec<Event>,
        path: &mut Path,
    ) -> bool {
        let frame = library
            .timeline(self.symbol)
            .and_then(|timeline| timeline.labels.get(label).copied());
        if let Some(frame) = frame {
            self.goto(frame, library, events, path);
        }
        frame.is_some()
    }

    /// Moves the playhead to `frame`, clamped to the timeline's length.
    pub fn goto(
        &mut self,
        frame: u16,
        library: &Library,
        events: &mut Vec<Event>,
        path: &mut Path,
    ) {
        let Some(timeline) = library.timeline(self.symbol) else {
            return;
        };
        let count = timeline.frames.len() as u16;
        if count == 0 {
            return;
        }
        let target = frame.clamp(1, count);
        if target == self.frame {
            return;
        }
        // This clip's own frame is reported ahead of anything the objects it
        // places report, as a script on the frame would run before theirs.
        let mark = events.len();
        if target > self.frame {
            for frame in self.frame + 1..=target {
                for op in &timeline.frame(frame).ops {
                    apply(&mut self.children, op, frame, library, events, path);
                }
            }
        } else {
            self.rewind_to(target, library, events, path);
        }
        self.frame = target;
        // Only the frame the playhead lands on is heard, not the ones it
        // passed over on the way.
        let landed = timeline.frame(target);
        events.extend(landed.sounds.iter().cloned().map(Event::Sound));
        if landed.has_script {
            let event = Event::Frame {
                symbol: self.symbol,
                path: path.clone(),
                frame: target,
            };
            events.insert(mark, event);
        }
        if landed.stops && library.obey_stops {
            self.playing = false;
        }
    }

    /// Rebuilds the display list as it stands on `target`, an earlier frame.
    fn rewind_to(
        &mut self,
        target: u16,
        library: &Library,
        events: &mut Vec<Event>,
        path: &mut Path,
    ) {
        let Some(timeline) = library.timeline(self.symbol) else {
            return;
        };
        let mut old = std::mem::take(&mut self.children);
        // What the objects built by the replay asked for, by depth. Held back
        // until it is known which of them are really new.
        let mut pending: BTreeMap<u16, Vec<Event>> = BTreeMap::new();
        for frame in 1..=target {
            for op in &timeline.frame(frame).ops {
                let mut made = Vec::new();
                apply(&mut self.children, op, frame, library, &mut made, path);
                match op {
                    Op::Remove { depth } => {
                        pending.remove(depth);
                    }
                    Op::Place(place) if place.action != PlaceAction::Modify => {
                        pending.insert(place.depth, made);
                    }
                    Op::Place(_) => {}
                }
            }
        }
        // Keep the old object wherever the replay made the same one, so that
        // it carries on from where it was instead of starting over.
        for (depth, child) in &mut self.children {
            if let Some(previous) = old.remove(depth) {
                if previous.symbol == child.symbol
                    && previous.placed_on == child.placed_on
                    && !previous.attached
                {
                    // What the rules had set stays set.
                    if previous.held.matrix {
                        child.matrix = previous.matrix;
                    }
                    if previous.held.color {
                        child.color = previous.color;
                    }
                    if previous.held.visible {
                        child.visible = previous.visible;
                    }
                    child.held = previous.held;
                    child.content = previous.content;
                    pending.remove(depth);
                } else {
                    old.insert(*depth, previous);
                }
            }
        }
        // Objects the rules added are no part of any frame, so going back
        // does not remove them. One that the replay has covered is lost, as
        // a placement on a taken depth would have been ignored going forward.
        for (depth, child) in old {
            if child.attached {
                self.children.entry(depth).or_insert(child);
            }
        }
        events.extend(pending.into_values().flatten());
    }

    /// Puts a new instance of `symbol` on this clip's display list at
    /// `depth`, in place of anything already there, as `attachMovie` did.
    /// `path` is where this clip sits in the tree. Returns the new object,
    /// or `None` if `symbol` is nothing that can be shown.
    pub fn attach(
        &mut self,
        symbol: SymbolId,
        depth: u16,
        name: Option<&str>,
        library: &Library,
        events: &mut Vec<Event>,
        path: &mut Path,
    ) -> Option<&mut Child> {
        path.push(depth);
        let made = new_child(symbol, self.frame, library, events, path);
        path.pop();
        let mut child = made?;
        child.name = name.map(str::to_owned);
        child.attached = true;
        self.children.insert(depth, child);
        self.children.get_mut(&depth)
    }
}

fn advance_children(
    children: &mut Children,
    library: &Library,
    events: &mut Vec<Event>,
    path: &mut Path,
) {
    for (&depth, child) in children.iter_mut() {
        path.push(depth);
        match &mut child.content {
            Content::Clip(clip) => clip.advance(library, events, path),
            Content::Button(button) => {
                advance_children(button.shown_mut(), library, events, path);
            }
            Content::Graphic => {}
        }
        path.pop();
    }
}

#[cfg(test)]
mod tests;
