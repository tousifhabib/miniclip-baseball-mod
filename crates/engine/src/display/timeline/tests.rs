use bb_format::{Place, SoundEvent, SoundStart};

use super::*;
use crate::math::Matrix;
use crate::testing::{INNER, OTHER_SHAPE, SHAPE, frame, library, place, put, start, tick};

pub(crate) fn sound(id: SymbolId) -> SoundStart {
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
