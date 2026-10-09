use bb_format::{Frame, Place};
use proptest::prelude::*;

use super::*;
use crate::display::timeline::tests::sound;
use crate::display::{Command, Texts, commands};
use crate::math::Matrix;
use crate::testing::{OTHER_SHAPE, SHAPE, frame, library, place, start, tick};

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
    let placing =
        (1u16..=4, action, at, visible, ratio).prop_map(|(depth, action, at, visible, ratio)| {
            Op::Place(Box::new(Place {
                matrix: at.map(|(x, y)| [1.0, 0.0, 0.0, 1.0, f64::from(x), f64::from(y)]),
                visible,
                ratio,
                ..place(depth, action)
            }))
        });
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
