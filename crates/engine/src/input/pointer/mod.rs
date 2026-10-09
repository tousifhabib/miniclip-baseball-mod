//! The pointer, and the buttons it rolls over and presses.

#[cfg(test)]
mod properties;

use bb_format::SymbolId;

use super::Geometry;
use crate::display::{ButtonEvent, ButtonMode, Children, ClipState, Content, Event, Path};
use crate::library::Library;

/// One button in the tree.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Target {
    path: Path,
    symbol: SymbolId,
}

/// Where the pointer is and which button it is dealing with.
#[derive(Debug)]
pub struct Pointer {
    pub x: f32,
    pub y: f32,
    pub down: bool,
    /// Where the pointer's button went down since the last frame was
    /// played, if it did and not on one of the game's buttons. A click is
    /// kept here as it arrives, so that one which is over before the next
    /// frame is played is not missed by it.
    pub went_down: Option<(f32, f32)>,
    /// The button the pointer is resting on.
    over: Option<Target>,
    /// The button a press began on. It keeps the press until release, even
    /// if the pointer wanders off it.
    pressed: Option<Target>,
    /// While pressing: whether the pointer is still on that button.
    inside: bool,
}

impl Default for Pointer {
    /// A pointer that has not been seen yet. It starts far from everything,
    /// so that nothing counts as hovered until the host says where it is.
    fn default() -> Pointer {
        Pointer {
            x: Pointer::NOWHERE,
            y: Pointer::NOWHERE,
            down: false,
            went_down: None,
            over: None,
            pressed: None,
            inside: false,
        }
    }
}

impl Pointer {
    /// A coordinate well outside any stage.
    pub const NOWHERE: f32 = -1.0e6;

    /// Whether the pointer is on a button, for showing a hand cursor.
    pub fn on_button(&self) -> bool {
        self.over.is_some() || (self.pressed.is_some() && self.inside)
    }

    /// Takes in the pointer's new position and button state, and works out
    /// what that does to the buttons in `root`. The position, `at`, is in
    /// `root`'s coordinates.
    pub fn update(
        &mut self,
        root: &mut ClipState,
        at: (f32, f32),
        down: bool,
        library: &Library,
        geometry: &mut dyn Geometry,
        events: &mut Vec<Event>,
    ) {
        let was_down = self.down;
        let (x, y) = at;
        (self.x, self.y, self.down) = (x, y, down);
        let hit = button_at(&root.children, x, y, library, geometry, &mut Path::new());
        let mut change = |target: &Target, event: ButtonEvent, mode: ButtonMode| {
            if let Some(child) = root.child_mut(&target.path)
                && let Content::Button(button) = &mut child.content
            {
                button.mode = mode;
            }
            events.push(Event::Button {
                symbol: target.symbol,
                path: target.path.clone(),
                event,
            });
            let sounds = library
                .buttons
                .get(&target.symbol)
                .and_then(|button| button.sounds.as_ref());
            let sound = sounds.and_then(|sounds| match event {
                ButtonEvent::RollOver => sounds.up_to_over.as_ref(),
                ButtonEvent::RollOut => sounds.over_to_up.as_ref(),
                ButtonEvent::Press => sounds.over_to_down.as_ref(),
                ButtonEvent::Release => sounds.down_to_over.as_ref(),
                _ => None,
            });
            events.extend(sound.cloned().map(Event::Sound));
        };

        if let Some(pressed) = self.pressed.clone() {
            let inside = hit.as_ref() == Some(&pressed);
            if down {
                if inside != self.inside {
                    self.inside = inside;
                    if inside {
                        change(&pressed, ButtonEvent::DragOver, ButtonMode::Down);
                    } else {
                        change(&pressed, ButtonEvent::DragOut, ButtonMode::Over);
                    }
                }
                return;
            }
            self.pressed = None;
            if inside {
                change(&pressed, ButtonEvent::Release, ButtonMode::Over);
                self.over = Some(pressed);
            } else {
                change(&pressed, ButtonEvent::ReleaseOutside, ButtonMode::Up);
                self.over = None;
            }
        }

        if self.over != hit {
            if let Some(old) = self.over.take() {
                change(&old, ButtonEvent::RollOut, ButtonMode::Up);
            }
            // A button does not light up for a pointer that arrives already
            // held down.
            if !down && let Some(new) = &hit {
                change(new, ButtonEvent::RollOver, ButtonMode::Over);
                self.over = hit;
            }
        }
        if down && !was_down {
            match self.over.clone() {
                Some(target) => {
                    change(&target, ButtonEvent::Press, ButtonMode::Down);
                    self.pressed = Some(target);
                    self.inside = true;
                }
                // Of two clicks before the same frame, the first is the
                // one the frame is told of.
                None => {
                    self.went_down.get_or_insert((x, y));
                }
            }
        }
    }
}

/// The topmost button whose hit area holds the point. `x` and `y` are in the
/// coordinates of whatever owns `children`.
fn button_at(
    children: &Children,
    x: f32,
    y: f32,
    library: &Library,
    geometry: &mut dyn Geometry,
    path: &mut Path,
) -> Option<Target> {
    for (&depth, child) in children.iter().rev() {
        // Masks and hidden objects take no part.
        if !child.visible || child.clip_depth.is_some() {
            continue;
        }
        let Some(inverse) = child.matrix.inverse() else {
            continue;
        };
        let (x, y) = inverse.apply(x, y);
        match &child.content {
            Content::Button(button) => {
                if area_contains(&button.hit, x, y, library, geometry) {
                    path.push(depth);
                    return Some(Target {
                        path: path.clone(),
                        symbol: child.symbol,
                    });
                }
            }
            Content::Clip(clip) => {
                path.push(depth);
                if let Some(found) = button_at(&clip.children, x, y, library, geometry, path) {
                    return Some(found);
                }
                path.pop();
            }
            // Plain artwork does not stop the pointer reaching what is under
            // it.
            Content::Graphic => {}
        }
    }
    None
}

/// Whether any of `children` draws something over the point.
fn area_contains(
    children: &Children,
    x: f32,
    y: f32,
    library: &Library,
    geometry: &mut dyn Geometry,
) -> bool {
    children.values().any(|child| {
        let Some(inverse) = child.matrix.inverse() else {
            return false;
        };
        let (x, y) = inverse.apply(x, y);
        match &child.content {
            Content::Graphic => geometry.contains(library, child.symbol, child.ratio, x, y),
            Content::Clip(clip) => area_contains(&clip.children, x, y, library, geometry),
            Content::Button(button) => area_contains(&button.hit, x, y, library, geometry),
        }
    })
}

#[cfg(test)]
mod tests;
