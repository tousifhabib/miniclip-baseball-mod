//! Plays a timeline: moves a clip's playhead, and keeps its display list as
//! the frame it is on has it.
//!
//! This follows Flash's rules. A timeline stores changes per frame, so moving
//! the playhead forward applies each frame's changes in turn, and moving it
//! back replays from frame 1. An object that the replay puts back where it
//! already was is kept, not rebuilt, so a nested clip keeps its own position.

use std::collections::BTreeMap;

use bb_format::{Look, Op, Place, PlaceAction, SymbolId, SymbolInfo};

use super::{ButtonMode, ButtonState, Child, Children, ClipState, Content, Event, Held, Path};
use crate::library::Library;
use crate::math::{ColorTransform, Matrix};

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

/// Applies one change to `children`, the display list of the clip at `path`.
fn apply(
    children: &mut Children,
    op: &Op,
    frame: u16,
    library: &Library,
    events: &mut Vec<Event>,
    path: &mut Path,
) {
    // The path of whatever this change puts at its depth.
    let mut placed = |symbol: SymbolId, depth: u16, events: &mut Vec<Event>| {
        path.push(depth);
        let child = new_child(symbol, frame, library, events, path);
        path.pop();
        child
    };
    match op {
        Op::Remove { depth } => {
            children.remove(depth);
        }
        Op::Place(place) => match place.action {
            PlaceAction::Place(symbol) => {
                // Flash ignores a placement at a depth that is already taken.
                if !children.contains_key(&place.depth)
                    && let Some(mut child) = placed(symbol, place.depth, events)
                {
                    update(&mut child, place);
                    children.insert(place.depth, child);
                }
            }
            PlaceAction::Modify => {
                if let Some(child) = children.get_mut(&place.depth) {
                    update(child, place);
                }
            }
            PlaceAction::Replace(symbol) => {
                let Some(mut child) = placed(symbol, place.depth, events) else {
                    return;
                };
                // The new object takes over the old one's settings, apart
                // from the ones this placement gives.
                if let Some(old) = children.remove(&place.depth) {
                    child.matrix = old.matrix;
                    child.color = old.color;
                    child.ratio = old.ratio;
                    child.clip_depth = old.clip_depth;
                    child.name = old.name;
                    child.visible = old.visible;
                    child.filters = old.filters;
                    child.held = old.held;
                }
                update(&mut child, place);
                children.insert(place.depth, child);
            }
        },
    }
}

/// Applies the settings a placement names, leaving the rest alone.
fn update(child: &mut Child, place: &Place) {
    if let Some(matrix) = place.matrix
        && !child.held.matrix
    {
        child.matrix = matrix.into();
    }
    if let Some(color) = place.color
        && !child.held.color
    {
        child.color = color.into();
    }
    if let Some(ratio) = place.ratio {
        child.ratio = ratio;
    }
    if let Some(name) = &place.name {
        child.name = Some(name.clone());
    }
    if let Some(clip_depth) = place.clip_depth {
        child.clip_depth = Some(clip_depth);
    }
    if let Some(visible) = place.visible
        && !child.held.visible
    {
        child.visible = visible;
    }
    if let Some(filters) = &place.filters {
        child.filters = filters.clone();
    }
}

/// A fresh instance of `symbol` to sit at `path`, or `None` if there is
/// nothing to show for it.
fn new_child(
    symbol: SymbolId,
    placed_on: u16,
    library: &Library,
    events: &mut Vec<Event>,
    path: &mut Path,
) -> Option<Child> {
    let content = match &library.manifest.symbols.get(&symbol)?.info {
        SymbolInfo::Clip { .. } => {
            Content::Clip(ClipState::new(Some(symbol), library, events, path))
        }
        SymbolInfo::Button => {
            let button = library.buttons.get(&symbol)?;
            let mut state = ButtonState {
                mode: ButtonMode::Up,
                up: Children::new(),
                over: Children::new(),
                down: Children::new(),
                hit: Children::new(),
            };
            for record in &button.records {
                for name in &record.states {
                    let children = match name {
                        Look::Up => &mut state.up,
                        Look::Over => &mut state.over,
                        Look::Down => &mut state.down,
                        Look::Hit => &mut state.hit,
                        Look::Other(_) => continue,
                    };
                    path.push(record.depth);
                    let made = new_child(record.symbol, 0, library, events, path);
                    path.pop();
                    if let Some(mut child) = made {
                        child.matrix = record.matrix.into();
                        if let Some(color) = record.color {
                            child.color = color.into();
                        }
                        child.filters = record.filters.clone();
                        children.insert(record.depth, child);
                    }
                }
            }
            Content::Button(state)
        }
        SymbolInfo::Shape { .. }
        | SymbolInfo::MorphShape
        | SymbolInfo::Text
        | SymbolInfo::EditText => Content::Graphic,
        SymbolInfo::Bitmap { .. } | SymbolInfo::Sound { .. } | SymbolInfo::Font { .. } => {
            return None;
        }
    };
    Some(Child {
        symbol,
        matrix: Matrix::IDENTITY,
        color: ColorTransform::IDENTITY,
        ratio: 0,
        clip_depth: None,
        name: None,
        visible: true,
        filters: Vec::new(),
        placed_on,
        held: Held::default(),
        attached: false,
        said: None,
        content,
    })
}

#[cfg(test)]
mod tests {
    use bb_format::{Frame, SoundEvent, SoundStart};
    use proptest::prelude::*;

    use super::*;
    use crate::display::{Command, Texts, commands};
    use crate::testing::{
        INNER, OTHER_SHAPE, SHAPE, frame, library, library_with, place, put, start, tick,
    };

    fn sound(id: SymbolId) -> SoundStart {
        SoundStart {
            sound: id,
            event: SoundEvent::Event,
            loops: 1,
            in_sample: None,
            out_sample: None,
            envelope: Vec::new(),
        }
    }

    fn inner_frame(clip: &ClipState, depth: u16) -> u16 {
        match &clip.children[&depth].content {
            Content::Clip(inner) => inner.frame,
            other => panic!("expected a clip, found {other:?}"),
        }
    }

    #[test]
    fn a_new_clip_shows_its_first_frame() {
        let library = library(vec![frame(vec![put(1, SHAPE)]), frame(vec![])], 1);
        let root = start(&library);
        assert_eq!(root.frame, 1);
        assert_eq!(root.children[&1].symbol, SHAPE);
    }

    #[test]
    fn frames_apply_their_changes_in_turn() {
        let moved = Place {
            matrix: Some([1.0, 0.0, 0.0, 1.0, 5.0, 7.0]),
            ..place(1, PlaceAction::Modify)
        };
        let library = library(
            vec![
                frame(vec![put(1, SHAPE)]),
                frame(vec![Op::Place(Box::new(moved))]),
                frame(vec![Op::Remove { depth: 1 }]),
            ],
            1,
        );
        let mut root = start(&library);
        tick(&mut root, &library);
        assert_eq!(root.children[&1].matrix, Matrix::translate(5.0, 7.0));
        tick(&mut root, &library);
        assert!(root.children.is_empty());
    }

    #[test]
    fn the_timeline_leaves_alone_what_the_rules_have_moved() {
        let moved = Place {
            matrix: Some([1.0, 0.0, 0.0, 1.0, 5.0, 7.0]),
            ..place(1, PlaceAction::Modify)
        };
        let library = library(
            vec![
                frame(vec![put(1, SHAPE), put(2, SHAPE)]),
                frame(vec![
                    Op::Place(Box::new(moved.clone())),
                    Op::Place(Box::new(Place { depth: 2, ..moved })),
                ]),
            ],
            1,
        );
        let mut root = start(&library);
        root.children.get_mut(&1).unwrap().move_to(40.0, 50.0);
        tick(&mut root, &library);
        assert_eq!(root.children[&1].matrix, Matrix::translate(40.0, 50.0));
        // The one the rules never touched still follows the timeline.
        assert_eq!(root.children[&2].matrix, Matrix::translate(5.0, 7.0));
    }

    #[test]
    fn what_the_rules_have_set_survives_a_loop() {
        let library = library(vec![frame(vec![put(1, SHAPE)]), frame(vec![])], 1);
        let mut root = start(&library);
        let child = root.children.get_mut(&1).unwrap();
        child.move_to(40.0, 50.0);
        child.set_visible(false);
        child.set_alpha(0.5);
        tick(&mut root, &library);
        tick(&mut root, &library);
        assert_eq!(root.frame, 1);
        let child = &root.children[&1];
        assert_eq!(child.matrix, Matrix::translate(40.0, 50.0));
        assert!(!child.visible);
        assert_eq!(child.color.mult[3], 0.5);
    }

    #[test]
    fn an_object_the_rules_added_stays_through_a_loop() {
        let library = library(
            vec![frame(vec![put(1, SHAPE)]), frame(vec![put(2, OTHER_SHAPE)])],
            10,
        );
        let mut root = start(&library);
        let added = root.attach(
            INNER,
            100,
            Some("extra"),
            &library,
            &mut Vec::new(),
            &mut Path::new(),
        );
        assert!(added.is_some());
        tick(&mut root, &library);
        assert!(root.children.contains_key(&2));
        tick(&mut root, &library);
        assert_eq!(root.frame, 1);
        assert!(!root.children.contains_key(&2));
        assert_eq!(root.children[&100].name.as_deref(), Some("extra"));
        // It was not rebuilt either: it has kept counting.
        assert_eq!(inner_frame(&root, 100), 3);
    }

    #[test]
    fn attaching_something_that_cannot_be_shown_adds_nothing() {
        let library = library(vec![frame(vec![])], 1);
        let mut root = start(&library);
        let added = root.attach(999, 100, None, &library, &mut Vec::new(), &mut Path::new());
        assert!(added.is_none());
        assert!(root.children.is_empty());
    }

    #[test]
    fn replacing_keeps_the_old_position() {
        let moved = Place {
            matrix: Some([1.0, 0.0, 0.0, 1.0, 5.0, 7.0]),
            ..place(1, PlaceAction::Place(SHAPE))
        };
        let swap = place(1, PlaceAction::Replace(OTHER_SHAPE));
        let library = library(
            vec![
                frame(vec![Op::Place(Box::new(moved))]),
                frame(vec![Op::Place(Box::new(swap))]),
            ],
            1,
        );
        let mut root = start(&library);
        tick(&mut root, &library);
        assert_eq!(root.children[&1].symbol, OTHER_SHAPE);
        assert_eq!(root.children[&1].matrix, Matrix::translate(5.0, 7.0));
    }

    #[test]
    fn a_nested_clip_plays_on_its_own_but_not_on_the_tick_it_appears() {
        let library = library(
            vec![frame(vec![]), frame(vec![put(1, INNER)]), frame(vec![])],
            5,
        );
        let mut root = start(&library);
        tick(&mut root, &library);
        assert_eq!(inner_frame(&root, 1), 1);
        tick(&mut root, &library);
        assert_eq!(inner_frame(&root, 1), 2);
    }

    #[test]
    fn looping_keeps_objects_from_the_first_frame_and_drops_later_ones() {
        let library = library(
            vec![
                frame(vec![put(1, INNER)]),
                frame(vec![put(2, SHAPE)]),
                frame(vec![]),
            ],
            10,
        );
        let mut root = start(&library);
        tick(&mut root, &library);
        tick(&mut root, &library);
        assert_eq!(root.frame, 3);
        assert!(root.children.contains_key(&2));

        tick(&mut root, &library);
        assert_eq!(root.frame, 1);
        assert!(!root.children.contains_key(&2));
        // The inner clip was not rebuilt: it has kept counting.
        assert_eq!(inner_frame(&root, 1), 4);
    }

    #[test]
    fn a_stopped_clip_stays_put_while_its_children_play() {
        let library = library(vec![frame(vec![put(1, INNER)]), frame(vec![])], 5);
        let mut root = start(&library);
        root.playing = false;
        tick(&mut root, &library);
        assert_eq!(root.frame, 1);
        assert_eq!(inner_frame(&root, 1), 2);
    }

    #[test]
    fn going_back_rebuilds_the_list_for_that_frame() {
        let library = library(
            vec![
                frame(vec![put(1, SHAPE)]),
                frame(vec![Op::Remove { depth: 1 }, put(2, OTHER_SHAPE)]),
            ],
            1,
        );
        let mut root = start(&library);
        root.goto(2, &library, &mut Vec::new(), &mut Path::new());
        assert_eq!(root.children.keys().copied().collect::<Vec<_>>(), [2]);
        root.goto(1, &library, &mut Vec::new(), &mut Path::new());
        assert_eq!(root.children.keys().copied().collect::<Vec<_>>(), [1]);
    }

    #[test]
    fn landing_on_a_scripted_frame_is_reported_with_where_it_happened() {
        let scripted = Frame {
            has_script: true,
            ..Frame::default()
        };
        // The inner clip has a script on its first frame; so does the second
        // frame of the main timeline, which is the one that places it.
        let second = Frame {
            has_script: true,
            ..frame(vec![put(7, INNER)])
        };
        let library = library_with(vec![frame(vec![]), second], vec![scripted]);
        let mut root = start(&library);
        assert_eq!(
            tick(&mut root, &library),
            [
                // The outer frame first, as its script would run first.
                Event::Frame {
                    symbol: None,
                    path: vec![],
                    frame: 2,
                },
                Event::Frame {
                    symbol: Some(INNER),
                    path: vec![7],
                    frame: 1,
                },
            ]
        );
    }

    #[test]
    fn a_frame_marked_as_stopping_halts_the_timeline() {
        let stopping = Frame {
            stops: true,
            ..Frame::default()
        };
        let library = library(vec![frame(vec![]), stopping, frame(vec![])], 1);
        let mut root = start(&library);
        assert!(root.playing);
        tick(&mut root, &library);
        assert_eq!(root.frame, 2);
        assert!(!root.playing);
        tick(&mut root, &library);
        assert_eq!(root.frame, 2);
    }

    #[test]
    fn stops_can_be_switched_off() {
        let stopping = Frame {
            stops: true,
            ..Frame::default()
        };
        let mut library = library(vec![frame(vec![]), stopping, frame(vec![])], 1);
        library.obey_stops = false;
        let mut root = start(&library);
        tick(&mut root, &library);
        tick(&mut root, &library);
        assert_eq!(root.frame, 3);
    }

    #[test]
    fn a_sound_is_heard_when_the_playhead_lands_on_its_frame() {
        let with_sound = |id| Frame {
            sounds: vec![sound(id)],
            ..Frame::default()
        };
        let library = library(vec![with_sound(7), with_sound(8), with_sound(9)], 1);

        let mut events = Vec::new();
        let mut root = ClipState::new(None, &library, &mut events, &mut Path::new());
        assert_eq!(events, [Event::Sound(sound(7))]);

        assert_eq!(tick(&mut root, &library), [Event::Sound(sound(8))]);

        // Jumping from frame 2 back to 1 and on to 3 passes over nothing
        // that should be heard except where it lands.
        let mut events = Vec::new();
        root.goto(1, &library, &mut events, &mut Path::new());
        root.goto(3, &library, &mut events, &mut Path::new());
        assert_eq!(events, [Event::Sound(sound(7)), Event::Sound(sound(9))]);
    }

    #[test]
    fn a_clip_kept_through_a_loop_does_not_start_its_sound_again() {
        let inner = vec![
            Frame {
                sounds: vec![sound(7)],
                ..Frame::default()
            },
            Frame::default(),
            Frame::default(),
        ];
        let library = library_with(vec![frame(vec![put(1, INNER)]), frame(vec![])], inner);
        let mut events = Vec::new();
        let mut root = ClipState::new(None, &library, &mut events, &mut Path::new());
        assert_eq!(events, [Event::Sound(sound(7))]);
        assert!(tick(&mut root, &library).is_empty());
        // Back to frame 1: the inner clip is the same one, part way through.
        assert!(tick(&mut root, &library).is_empty());
        assert_eq!(root.frame, 1);
    }

    #[test]
    fn a_clip_rebuilt_by_a_rewind_starts_its_sound() {
        let inner = vec![Frame {
            sounds: vec![sound(7)],
            ..Frame::default()
        }];
        let library = library_with(
            vec![
                frame(vec![put(1, INNER)]),
                frame(vec![Op::Remove { depth: 1 }]),
            ],
            inner,
        );
        let mut root = start(&library);
        assert!(tick(&mut root, &library).is_empty());
        assert_eq!(tick(&mut root, &library), [Event::Sound(sound(7))]);
    }

    /// One change a frame might make to its display list, at one of a few
    /// depths: put one of the two shapes there, change what is there, swap
    /// it for a shape, or take it away. A placement may say where the
    /// object is, whether it is seen and how far its morph has gone, or
    /// leave any of them alone.
    fn change() -> impl Strategy<Value = Op> {
        let shape = || prop::sample::select(&[SHAPE, OTHER_SHAPE][..]);
        let action = prop_oneof![
            shape().prop_map(PlaceAction::Place),
            Just(PlaceAction::Modify),
            shape().prop_map(PlaceAction::Replace),
        ];
        let at = prop::option::of((-50i16..50, -50i16..50));
        let visible = prop::option::of(any::<bool>());
        let ratio = prop::option::of(any::<u16>());
        let placing = (1u16..=4, action, at, visible, ratio).prop_map(
            |(depth, action, at, visible, ratio)| {
                Op::Place(Box::new(Place {
                    matrix: at.map(|(x, y)| [1.0, 0.0, 0.0, 1.0, f64::from(x), f64::from(y)]),
                    visible,
                    ratio,
                    ..place(depth, action)
                }))
            },
        );
        let removing = (1u16..=4).prop_map(|depth| Op::Remove { depth });
        prop_oneof![3 => placing, 1 => removing]
    }

    /// A main timeline of a few frames, each making a few such changes. No
    /// frame stops it or says anything.
    fn timeline() -> impl Strategy<Value = Vec<Frame>> {
        prop::collection::vec(prop::collection::vec(change(), 0..4).prop_map(frame), 1..8)
    }

    /// The same, with frames that may have a script, start a sound and stop
    /// the timeline.
    fn noisy_timeline() -> impl Strategy<Value = Vec<Frame>> {
        let noisy = (
            prop::collection::vec(change(), 0..4),
            any::<bool>(),
            prop::option::of(7u16..10),
            any::<bool>(),
        )
            .prop_map(|(ops, has_script, heard, stops)| Frame {
                has_script,
                sounds: heard.into_iter().map(sound).collect(),
                stops,
                ..frame(ops)
            });
        prop::collection::vec(noisy, 1..8)
    }

    /// A way of moving a clip's playhead: on by a frame, as playing does,
    /// or straight to a frame.
    #[derive(Clone, Copy, Debug)]
    enum Move {
        Tick,
        Goto(u16),
    }

    fn moves() -> impl Strategy<Value = Vec<Move>> {
        // Some of the frames asked for are past the end of the timeline.
        let one = prop_oneof![Just(Move::Tick), (0u16..10).prop_map(Move::Goto)];
        prop::collection::vec(one, 1..12)
    }

    /// Everything there is to tell of a clip: what would be drawn for it,
    /// and the clip itself written out, down to the frame that put each
    /// object where it is.
    fn all_of(clip: &ClipState, library: &Library) -> (Vec<Command>, String) {
        let drawn = commands(clip, Matrix::IDENTITY, library, &Texts::new());
        (drawn, format!("{clip:?}"))
    }

    proptest! {
        #[test]
        fn however_a_clip_came_to_a_frame_it_shows_what_a_fresh_one_sent_straight_there_shows(
            frames in timeline(),
            moves in moves(),
        ) {
            let library = library(frames, 1);
            let mut clip = start(&library);
            for one in moves {
                // Going back, and going round from the last frame to the
                // first, build the list again from the first frame.
                match one {
                    Move::Tick => {
                        tick(&mut clip, &library);
                    }
                    Move::Goto(frame) => {
                        clip.goto(frame, &library, &mut Vec::new(), &mut Path::new());
                    }
                }
                let mut fresh = start(&library);
                fresh.goto(clip.frame, &library, &mut Vec::new(), &mut Path::new());
                prop_assert_eq!(
                    all_of(&clip, &library),
                    all_of(&fresh, &library),
                    "on frame {} after {:?}",
                    clip.frame,
                    one
                );
            }
        }

        #[test]
        fn going_to_the_frame_a_clip_is_on_already_says_nothing_and_changes_nothing(
            frames in noisy_timeline(),
            moves in moves(),
        ) {
            let library = library(frames, 1);
            let mut clip = start(&library);
            let last = clip.frame_count(&library);
            for one in moves {
                match one {
                    Move::Tick => {
                        tick(&mut clip, &library);
                    }
                    Move::Goto(frame) => {
                        clip.goto(frame, &library, &mut Vec::new(), &mut Path::new());
                    }
                }
                // A frame before the first is the first, and one past the
                // last is the last.
                let mut same = vec![clip.frame];
                if clip.frame == 1 {
                    same.push(0);
                }
                if clip.frame == last {
                    same.push(u16::MAX);
                }
                let before = all_of(&clip, &library);
                for frame in same {
                    let mut events = Vec::new();
                    clip.goto(frame, &library, &mut events, &mut Path::new());
                    prop_assert!(events.is_empty(), "asked for {}: {:?}", frame, events);
                    prop_assert_eq!(all_of(&clip, &library), before.clone());
                }
            }
        }
    }
}
