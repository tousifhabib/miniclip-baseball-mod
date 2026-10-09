//! Writes the tree out in words, for looking at a scene without a window.

use super::{Children, Content};
use crate::library::Library;

/// Lists every object in `children`, one to a line and indented by how deep
/// it is nested, with the frame each clip is on. For looking at a scene
/// without a window.
pub fn describe_tree(children: &Children, library: &Library) -> String {
    fn walk(children: &Children, library: &Library, indent: usize, out: &mut String) {
        use std::fmt::Write;
        for (depth, child) in children {
            let pad = "  ".repeat(indent);
            let name = child
                .name
                .as_deref()
                .map(|name| format!(" \"{name}\""))
                .unwrap_or_default();
            let mask = if child.clip_depth.is_some() {
                " (mask)"
            } else {
                ""
            };
            let hidden = if child.visible { "" } else { " (hidden)" };
            match &child.content {
                Content::Graphic => {
                    writeln!(
                        out,
                        "{pad}{depth}: symbol {}{name}{mask}{hidden}",
                        child.symbol
                    )
                }
                Content::Clip(clip) => {
                    let state = if clip.playing { "playing" } else { "stopped" };
                    writeln!(
                        out,
                        "{pad}{depth}: clip {}{name}{mask}{hidden}, {state} on frame {} of {}",
                        child.symbol,
                        clip.frame,
                        clip.frame_count(library)
                    )
                }
                Content::Button(button) => writeln!(
                    out,
                    "{pad}{depth}: button {}{name}{mask}{hidden}, {:?}",
                    child.symbol, button.mode
                ),
            }
            .expect("writing to a string");
            match &child.content {
                Content::Clip(clip) => walk(&clip.children, library, indent + 1, out),
                Content::Button(button) => walk(button.shown(), library, indent + 1, out),
                Content::Graphic => {}
            }
        }
    }
    let mut out = String::new();
    walk(children, library, 1, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use bb_format::{Op, Place, PlaceAction};

    use super::*;
    use crate::testing::{INNER, OTHER_SHAPE, SHAPE, frame, library_with, place, put, start, tick};

    #[test]
    fn a_tree_is_written_out_a_line_to_an_object_and_set_in_by_how_deep_it_is() {
        let named = Place {
            name: Some("hitter".to_owned()),
            ..place(3, PlaceAction::Place(INNER))
        };
        let mask = Place {
            clip_depth: Some(5),
            ..place(4, PlaceAction::Place(OTHER_SHAPE))
        };
        let library = library_with(
            vec![frame(vec![
                put(1, SHAPE),
                Op::Place(Box::new(named)),
                Op::Place(Box::new(mask)),
            ])],
            vec![frame(vec![put(2, SHAPE)]), frame(vec![])],
        );
        let mut root = start(&library);
        root.children.get_mut(&1).unwrap().set_visible(false);
        let lines = [
            "  1: symbol 1 (hidden)\n",
            "  3: clip 10 \"hitter\", playing on frame 1 of 2\n",
            "    2: symbol 1\n",
            "  4: symbol 2 (mask)\n",
        ];
        assert_eq!(describe_tree(&root.children, &library), lines.concat());
        // A clip that has been stopped says so, and where.
        tick(&mut root, &library);
        if let Content::Clip(inner) = &mut root.children.get_mut(&3).unwrap().content {
            inner.playing = false;
        }
        assert_eq!(
            describe_tree(&root.children, &library).lines().nth(1),
            Some("  3: clip 10 \"hitter\", stopped on frame 2 of 2")
        );
    }
}
