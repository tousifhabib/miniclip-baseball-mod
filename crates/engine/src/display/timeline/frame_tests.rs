use bb_format::Frame;

use super::*;
use crate::display::timeline::tests::sound;
use crate::testing::{INNER, frame, library, library_with, put, start, tick};

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
