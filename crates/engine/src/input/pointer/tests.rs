use bb_format::{
    Button, ButtonRecord, ButtonSounds, Op, PlaceAction, SoundEvent, SoundStart, SymbolInfo,
};

use super::*;
use crate::testing::{OTHER_SHAPE, SHAPE, frame, library_with, place, symbol};

const BUTTON: SymbolId = 20;
const CLICK: SymbolId = 30;

/// Treats every symbol as the square from (0, 0) to (10, 10).
struct Squares;

impl Geometry for Squares {
    fn contains(&mut self, _: &Library, _: SymbolId, _: u16, x: f32, y: f32) -> bool {
        (0.0..=10.0).contains(&x) && (0.0..=10.0).contains(&y)
    }
}

fn click_sound() -> SoundStart {
    SoundStart {
        sound: CLICK,
        event: SoundEvent::Event,
        loops: 1,
        in_sample: None,
        out_sample: None,
        envelope: Vec::new(),
    }
}

/// A main timeline with one button at (20, 20), 10 by 10.
pub(super) fn scene() -> (Library, ClipState) {
    let at = bb_format::Place {
        matrix: Some([1.0, 0.0, 0.0, 1.0, 20.0, 20.0]),
        ..place(1, PlaceAction::Place(BUTTON))
    };
    let mut library = library_with(vec![frame(vec![Op::Place(Box::new(at))])], vec![]);
    let record = |states: &[&str], symbol| ButtonRecord {
        states: states.iter().map(|s| (*s).into()).collect(),
        symbol,
        depth: 1,
        matrix: bb_format::IDENTITY,
        color: None,
        filters: Vec::new(),
    };
    library.buttons.insert(
        BUTTON,
        Button {
            id: BUTTON,
            track_as_menu: false,
            records: vec![
                record(&["up", "hit"], SHAPE),
                record(&["over", "down"], OTHER_SHAPE),
            ],
            actions: Vec::new(),
            sounds: Some(ButtonSounds {
                over_to_down: Some(click_sound()),
                ..ButtonSounds::default()
            }),
        },
    );
    library
        .manifest
        .symbols
        .insert(BUTTON, symbol("buttons/20.json", SymbolInfo::Button));
    let root = ClipState::new(None, &library, &mut Vec::new(), &mut Path::new());
    (library, root)
}

/// Moves the pointer and returns what happened to buttons, leaving out
/// sounds.
pub(super) fn act(
    pointer: &mut Pointer,
    root: &mut ClipState,
    library: &Library,
    x: f32,
    down: bool,
) -> Vec<ButtonEvent> {
    let mut events = Vec::new();
    pointer.update(root, (x, 25.0), down, library, &mut Squares, &mut events);
    events
        .into_iter()
        .filter_map(|event| match event {
            Event::Button { event, .. } => Some(event),
            Event::Sound(_) | Event::Frame { .. } => None,
        })
        .collect()
}

pub(super) fn mode(root: &ClipState) -> ButtonMode {
    match &root.children[&1].content {
        Content::Button(button) => button.mode,
        other => panic!("expected a button, found {other:?}"),
    }
}

#[test]
fn a_pointer_nobody_has_moved_hovers_over_nothing() {
    // The button below is moved to the corner of the stage, where a
    // pointer resting at (0, 0) would be on top of it.
    let (library, mut root) = scene();
    root.children.get_mut(&1).unwrap().matrix = crate::math::Matrix::IDENTITY;
    let mut pointer = Pointer::default();
    let mut events = Vec::new();
    let (x, y, down) = (pointer.x, pointer.y, pointer.down);
    pointer.update(&mut root, (x, y), down, &library, &mut Squares, &mut events);
    assert!(events.is_empty());
    assert_eq!(mode(&root), ButtonMode::Up);
}

#[test]
fn rolling_over_pressing_and_releasing_a_button() {
    let (library, mut root) = scene();
    let mut pointer = Pointer::default();

    assert!(act(&mut pointer, &mut root, &library, 5.0, false).is_empty());
    assert_eq!(mode(&root), ButtonMode::Up);

    assert_eq!(
        act(&mut pointer, &mut root, &library, 25.0, false),
        [ButtonEvent::RollOver]
    );
    assert_eq!(mode(&root), ButtonMode::Over);
    assert!(pointer.on_button());

    assert_eq!(
        act(&mut pointer, &mut root, &library, 25.0, true),
        [ButtonEvent::Press]
    );
    assert_eq!(mode(&root), ButtonMode::Down);

    assert_eq!(
        act(&mut pointer, &mut root, &library, 25.0, false),
        [ButtonEvent::Release]
    );
    assert_eq!(mode(&root), ButtonMode::Over);

    assert_eq!(
        act(&mut pointer, &mut root, &library, 5.0, false),
        [ButtonEvent::RollOut]
    );
    assert_eq!(mode(&root), ButtonMode::Up);
    assert!(!pointer.on_button());
}

#[test]
fn a_click_off_the_buttons_is_kept_though_it_is_over_at_once() {
    let (library, mut root) = scene();
    let mut pointer = Pointer::default();
    act(&mut pointer, &mut root, &library, 5.0, false);
    assert_eq!(pointer.went_down, None);
    // Down and up again with no frame played in between.
    assert!(act(&mut pointer, &mut root, &library, 5.0, true).is_empty());
    act(&mut pointer, &mut root, &library, 5.0, false);
    assert!(!pointer.down);
    assert_eq!(pointer.went_down, Some((5.0, 25.0)));
    // A second click before the frame does not take its place, and
    // holding the button is not a click at all.
    act(&mut pointer, &mut root, &library, 7.0, true);
    act(&mut pointer, &mut root, &library, 8.0, true);
    assert_eq!(pointer.went_down, Some((5.0, 25.0)));
}

#[test]
fn a_click_on_a_button_is_the_buttons_alone() {
    let (library, mut root) = scene();
    let mut pointer = Pointer::default();
    act(&mut pointer, &mut root, &library, 25.0, false);
    assert_eq!(
        act(&mut pointer, &mut root, &library, 25.0, true),
        [ButtonEvent::Press]
    );
    // Not even once the pointer has been dragged off it.
    act(&mut pointer, &mut root, &library, 5.0, true);
    act(&mut pointer, &mut root, &library, 5.0, false);
    assert_eq!(pointer.went_down, None);
}

#[test]
fn dragging_off_a_pressed_button_and_letting_go_outside() {
    let (library, mut root) = scene();
    let mut pointer = Pointer::default();
    act(&mut pointer, &mut root, &library, 25.0, false);
    act(&mut pointer, &mut root, &library, 25.0, true);

    assert_eq!(
        act(&mut pointer, &mut root, &library, 5.0, true),
        [ButtonEvent::DragOut]
    );
    assert_eq!(
        act(&mut pointer, &mut root, &library, 25.0, true),
        [ButtonEvent::DragOver]
    );
    assert_eq!(mode(&root), ButtonMode::Down);

    act(&mut pointer, &mut root, &library, 5.0, true);
    assert_eq!(
        act(&mut pointer, &mut root, &library, 5.0, false),
        [ButtonEvent::ReleaseOutside]
    );
    assert_eq!(mode(&root), ButtonMode::Up);
}

#[test]
fn a_pointer_that_arrives_held_down_does_not_press() {
    let (library, mut root) = scene();
    let mut pointer = Pointer::default();
    act(&mut pointer, &mut root, &library, 5.0, true);
    assert!(act(&mut pointer, &mut root, &library, 25.0, true).is_empty());
    assert_eq!(mode(&root), ButtonMode::Up);
    // Letting go over the button is a roll over, not a release.
    assert_eq!(
        act(&mut pointer, &mut root, &library, 25.0, false),
        [ButtonEvent::RollOver]
    );
}

#[test]
fn a_press_plays_the_buttons_sound_and_names_the_button() {
    let (library, mut root) = scene();
    let mut pointer = Pointer::default();
    act(&mut pointer, &mut root, &library, 25.0, false);

    let mut events = Vec::new();
    pointer.update(
        &mut root,
        (25.0, 25.0),
        true,
        &library,
        &mut Squares,
        &mut events,
    );
    assert_eq!(
        events,
        [
            Event::Button {
                symbol: BUTTON,
                path: vec![1],
                event: ButtonEvent::Press,
            },
            Event::Sound(click_sound()),
        ]
    );
}
