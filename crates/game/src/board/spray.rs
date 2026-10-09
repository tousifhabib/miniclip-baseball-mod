//! The page that marks on a picture of the field where every ball went.

use bb_engine::stage::Stage;

use super::{CREAM, GOLD, GREEN, GREY, RED, WHITE};
use crate::art;
use crate::look::Rgb;
use crate::play::book::{End, Side, Turn, percent};
use crate::sheet::Sheet;

/// The picture of the field on its page: its size, the art's being 1, and
/// where the corner of the field's own pixels goes. The picture is of more
/// than a match ever shows, so only that much of it is let through: its
/// left, top, width and height in the field's pixels.
const FIELD_SIZE: f32 = 0.42;
const FIELD_AT: (f32, f32) = (56.0, 114.0);
const FIELD_SEEN: [f32; 4] = [-46.0, -35.0, 678.0, 460.0];

/// The colour a ball in play is marked in, by what came of it.
fn mark(end: End) -> Rgb {
    match end {
        End::HomeRun => GREEN,
        End::Double | End::Triple => GOLD,
        End::Single => WHITE,
        End::Error => GREY,
        _ => RED,
    }
}

/// The page of where a side's hits went: a picture of the field with a
/// mark where each ball came down, and what they come to beside it.
pub(super) fn field(side: &Side, sheet: &mut Sheet, stage: &mut Stage) {
    const SIZE: f32 = 0.68;
    let [left, top, wide, high] = FIELD_SEEN.map(|pixels| pixels * FIELD_SIZE);
    let (left, top) = (FIELD_AT.0 + left, FIELD_AT.1 + top);
    // A block the size of what is to be seen, which the picture after it
    // is seen through.
    let side_of = art::BLOCK_SIDE;
    let window = sheet.add(
        stage,
        art::BLOCK,
        "fieldWindow",
        (left, top),
        (wide / side_of, high / side_of),
    );
    let picture = sheet.depth;
    if let Some(window) = window.and_then(|path| stage.child_mut(&path)) {
        window.clip_depth = Some(picture);
    }
    let size = (FIELD_SIZE, FIELD_SIZE);
    sheet.add(stage, art::FIELD_PICTURE, "fieldPicture", FIELD_AT, size);
    // The outs go on first, so that the hits are not hidden under them.
    let mut balls: Vec<&Turn> = side
        .turns
        .iter()
        .filter(|turn| turn.ball.is_some())
        .collect();
    balls.sort_by_key(|turn| turn.end.bases());
    for turn in balls {
        let Some(ball) = turn.ball else {
            continue;
        };
        // One that went clean out of the picture is marked at its edge.
        let at = (
            (FIELD_AT.0 + ball.at.0 * FIELD_SIZE).clamp(left + 3.0, left + wide - 3.0),
            (FIELD_AT.1 + ball.at.1 * FIELD_SIZE).clamp(top + 3.0, top + high - 3.0),
        );
        sheet.dot(stage, "fieldMark", at, 1.0, mark(turn.end));
    }
    let all = side.figures();
    let outs = all.in_play - all.hits;
    let share = |part: u32| percent((all.in_play > 0).then(|| part as f32 / all.in_play as f32));
    let usual = all
        .usual_feet()
        .map_or("-".to_owned(), |feet| format!("{feet:.0} FT"));
    let lines: [(Option<Rgb>, String); 11] = [
        (Some(GREEN), format!("HOME RUNS {}", all.home_runs)),
        (
            Some(GOLD),
            format!("DOUBLES AND TRIPLES {}", all.doubles + all.triples),
        ),
        (Some(WHITE), format!("SINGLES {}", all.singles)),
        (Some(RED), format!("OUTS AND ERRORS {outs}")),
        (None, format!("TO LEFT {}", share(all.thirds[0]))),
        (None, format!("TO CENTRE {}", share(all.thirds[1]))),
        (None, format!("TO RIGHT {}", share(all.thirds[2]))),
        (None, format!("IN THE AIR {}", all.flies)),
        (None, format!("ON THE GROUND {}", all.grounders)),
        (None, format!("LONGEST {} FT", all.longest)),
        (None, format!("USUALLY {usual}")),
    ];
    for (row, (colour, line)) in lines.into_iter().enumerate() {
        // The lines come in fours and threes, a little apart.
        let gaps = [4, 7, 9].iter().filter(|&&after| row >= after).count();
        let down = 96.0 + 16.5 * row as f32 + 7.0 * gaps as f32;
        if let Some(colour) = colour {
            sheet.dot(stage, "fieldKey", (356.0, down + 8.0), 1.4, colour);
        }
        sheet.write_left(
            stage,
            "fieldLine",
            &line,
            (368.0, down),
            SIZE,
            colour.unwrap_or(CREAM),
        );
    }
}
