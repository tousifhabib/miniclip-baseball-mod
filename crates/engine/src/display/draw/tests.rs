use bb_format::{Op, Place, PlaceAction};

use super::*;
use crate::testing::{FIELD, OTHER_SHAPE, SHAPE, add_field, frame, library, place, put, start};

fn draw(clip: &ClipState, base: Matrix, library: &Library) -> Vec<Command> {
    commands(clip, base, library, &Texts::new())
}

fn kinds(commands: &[Command]) -> Vec<String> {
    commands
        .iter()
        .map(|command| match command {
            Command::Draw { symbol, .. } => format!("draw {symbol}"),
            Command::BeginBlur { .. } => "BeginBlur".to_owned(),
            other => format!("{other:?}"),
        })
        .collect()
}

#[test]
fn a_text_field_shows_what_the_game_has_set() {
    let mut library = library(vec![frame(vec![put(1, FIELD)])], 1);
    add_field(&mut library, "_root.game.score", &["read_only"]);
    let root = start(&library);
    let said = |texts: &Texts| match &commands(&root, Matrix::IDENTITY, &library, texts)[0] {
        Command::Draw { text, .. } => text.clone(),
        other => panic!("expected a draw, found {other:?}"),
    };
    assert_eq!(said(&Texts::new()), None);
    // Only the last part of the field's variable name counts.
    let texts = Texts::from([("score".to_owned(), "12".to_owned())]);
    assert_eq!(said(&texts), Some("12".to_owned()));
}

#[test]
fn text_in_a_mirrored_clip_is_drawn_the_right_way_round_when_asked() {
    let mut library = library(vec![frame(vec![put(1, FIELD)])], 1);
    add_field(&mut library, "score", &["read_only"]);
    let root = start(&library);
    // The whole thing turned over about the line 100 across.
    let mirror = Matrix {
        a: -1.0,
        tx: 200.0,
        ..Matrix::IDENTITY
    };
    let drawn = |upright: bool| {
        let texts = Texts::new();
        match &commands_upright(&root, mirror, &library, &texts, upright)[0] {
            Command::Draw { matrix, .. } => *matrix,
            other => panic!("expected a draw, found {other:?}"),
        }
    };
    // Left alone it is mirrored: the field, 50 wide, runs from 200
    // back to 150.
    let plain = drawn(false);
    assert_eq!((plain.a, plain.apply(0.0, 0.0).0), (-1.0, 200.0));
    // Kept upright it covers the same 150 to 200, and reads forwards.
    let upright = drawn(true);
    assert_eq!(upright.a, 1.0);
    assert_eq!(upright.apply(0.0, 0.0).0, 150.0);
    assert_eq!(upright.apply(50.0, 0.0).0, 200.0);
    // Nothing is done to what is not mirrored.
    let texts = Texts::new();
    let same = commands_upright(&root, Matrix::IDENTITY, &library, &texts, true);
    assert_eq!(same, commands(&root, Matrix::IDENTITY, &library, &texts));
}

#[test]
fn a_mask_wraps_the_depths_it_covers() {
    let mask = Place {
        clip_depth: Some(2),
        ..place(1, PlaceAction::Place(SHAPE))
    };
    let library = library(
        vec![frame(vec![
            Op::Place(Box::new(mask)),
            put(2, OTHER_SHAPE),
            put(3, OTHER_SHAPE),
        ])],
        1,
    );
    let root = start(&library);
    // Symbol 1 is the mask. Symbol 2 sits at depth 2, inside the mask, and
    // again at depth 3, past its end.
    assert_eq!(
        kinds(&draw(&root, Matrix::IDENTITY, &library)),
        [
            "PushMask",
            "draw 1",
            "ActivateMask",
            "draw 2",
            "DeactivateMask",
            "draw 1",
            "PopMask",
            "draw 2",
        ]
    );
}

#[test]
fn nested_transforms_multiply_outermost_first() {
    let moved = Place {
        matrix: Some([1.0, 0.0, 0.0, 1.0, 3.0, 4.0]),
        ..place(1, PlaceAction::Place(SHAPE))
    };
    let library = library(vec![frame(vec![Op::Place(Box::new(moved))])], 1);
    let root = start(&library);
    let commands = draw(&root, Matrix::scale(2.0, 2.0), &library);
    let Command::Draw { matrix, .. } = &commands[0] else {
        panic!("expected a draw");
    };
    assert_eq!(matrix.apply(0.0, 0.0), (6.0, 8.0));
}

#[test]
fn a_blurred_object_is_wrapped_with_its_area_and_scaled_blur() {
    let blurred = Place {
        matrix: Some([1.0, 0.0, 0.0, 1.0, 20.0, 30.0]),
        filters: Some(vec![Filter::Blur {
            blur_x: 5.0,
            blur_y: 4.0,
            passes: 1,
        }]),
        ..place(1, PlaceAction::Place(SHAPE))
    };
    let library = library(vec![frame(vec![Op::Place(Box::new(blurred))])], 1);
    let root = start(&library);
    let commands = draw(&root, Matrix::scale(2.0, 2.0), &library);
    assert_eq!(kinds(&commands), ["BeginBlur", "draw 1", "EndBlur"]);
    // The 10 by 10 shape at (20, 30), drawn at twice the size.
    assert_eq!(
        commands[0],
        Command::BeginBlur {
            blur_x: 10.0,
            blur_y: 8.0,
            passes: 1,
            bounds: [40.0, 60.0, 60.0, 80.0],
        }
    );
}
