use proptest::prelude::*;

use super::*;
use crate::input::pointer::tests::{act, mode, scene};

/// Whether a pointer this far across the scene is on its button, which
/// runs from 20 to 30.
fn on_the_button(x: f32) -> bool {
    (20.0..=30.0).contains(&x)
}

/// Things a pointer might do one after another: where it is across the
/// scene, on the button about as often as off it, and whether its own
/// button is held down.
fn pointing() -> impl Strategy<Value = Vec<(f32, bool)>> {
    let across = prop_oneof![20.0f32..=30.0, 0.0f32..50.0];
    prop::collection::vec((across, any::<bool>()), 1..40)
}

proptest! {
    #[test]
    fn a_button_looks_pressed_exactly_while_a_press_that_began_on_it_is_held_over_it(
        moves in pointing(),
    ) {
        let (library, mut root) = scene();
        let mut pointer = Pointer::default();
        // Whether a press of the button is still going on, by what has
        // been said of it, and whether the pointer's own button was
        // down the last time.
        let (mut held, mut was_down) = (false, false);
        for (x, down) in moves {
            for event in act(&mut pointer, &mut root, &library, x, down) {
                match event {
                    // A press is the pointer's button going down on the
                    // button, and there is one press at a time.
                    ButtonEvent::Press => {
                        prop_assert!(!held, "pressed while it was pressed");
                        prop_assert!(down && !was_down && on_the_button(x));
                        held = true;
                    }
                    // It ends when the pointer's button comes up, which
                    // is a release only if that is over the button.
                    ButtonEvent::Release | ButtonEvent::ReleaseOutside => {
                        prop_assert!(held, "let go of when it was not pressed");
                        prop_assert!(!down);
                        prop_assert_eq!(event == ButtonEvent::Release, on_the_button(x));
                        held = false;
                    }
                    _ => {}
                }
            }
            // And it does not outlast the pointer's button being held.
            prop_assert!(down || !held, "still pressed with nothing held");
            let looks = mode(&root);
            prop_assert_eq!(
                looks == ButtonMode::Down,
                held && on_the_button(x),
                "it looks {:?} at {}",
                looks,
                x
            );
            // The hand is shown for as long as the button does not look
            // as it does left alone.
            prop_assert_eq!(pointer.on_button(), looks != ButtonMode::Up);
            was_down = down;
        }
    }

    #[test]
    fn every_click_is_the_buttons_or_is_kept_for_the_frame_and_never_both(
        moves in pointing(),
    ) {
        let (library, mut root) = scene();
        let mut pointer = Pointer::default();
        let mut was_down = false;
        for (x, down) in moves {
            // As if a frame had been played since the last, and had
            // taken whatever click was kept for it.
            pointer.went_down = None;
            let events = act(&mut pointer, &mut root, &library, x, down);
            let clicked = down && !was_down;
            let pressed = events.contains(&ButtonEvent::Press);
            // Nothing but a click presses the button.
            prop_assert!(clicked || !pressed);
            let kept = (clicked && !pressed).then_some((x, 25.0));
            prop_assert_eq!(pointer.went_down, kept);
            was_down = down;
        }
    }
}
