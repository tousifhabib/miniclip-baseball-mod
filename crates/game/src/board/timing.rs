//! The page that shows how the swings were timed, as bars.

use bb_engine::stage::Stage;

use super::{CREAM, GOLD, GREEN, MIDDLE, PALE, RED};
use crate::play::book::Side;
use crate::play::pitch::Quality;
use crate::sheet::Sheet;

/// How far either side of the best moment the page of timing shows, in
/// frames. Swings further off than that are counted with the furthest.
const TIMING_REACH: i32 = 8;

/// The page of how the player's swings were timed: a bar for each frame
/// early or late, as tall as the swings that began on it are many, and what
/// they come to under it.
pub(super) fn timing(side: &Side, sheet: &mut Sheet, stage: &mut Stage) {
    const SIZE: f32 = 0.66;
    const FLOOR: f32 = 228.0;
    const TALL: f32 = 96.0;
    const PITCH: f32 = 27.5;
    const WIDE: f32 = 22.0;
    let swings: Vec<_> = side
        .turns
        .iter()
        .flat_map(|turn| &turn.pitches)
        .filter_map(|pitch| Some((pitch.off?, pitch.quality, pitch.met())))
        .collect();
    if swings.is_empty() {
        return sheet.write(
            stage,
            "timingLine",
            "NOT A SWING ALL MATCH",
            (MIDDLE, 180.0),
            1.0,
            CREAM,
        );
    }
    // For each frame off the best: the swings that met the ball sweetly,
    // those that met it less well, and those that missed.
    let bars = (2 * TIMING_REACH + 1) as usize;
    let mut counts = vec![[0u32; 3]; bars];
    for &(off, quality, met) in &swings {
        let bar = (off.clamp(-TIMING_REACH, TIMING_REACH) + TIMING_REACH) as usize;
        let kind = match (met, quality) {
            (true, Some(Quality::Good)) => 0,
            (true, _) => 1,
            (false, _) => 2,
        };
        counts[bar][kind] += 1;
    }
    let most = counts
        .iter()
        .map(|bar| bar.iter().sum::<u32>())
        .max()
        .unwrap_or(1)
        .max(1);
    let first = MIDDLE - PITCH * TIMING_REACH as f32 - WIDE / 2.0;
    for (bar, kinds) in counts.iter().enumerate() {
        let left = first + PITCH * bar as f32;
        sheet.block(stage, "timingFloor", [left, FLOOR, WIDE, 1.5], PALE, 0.7);
        let mut top = FLOOR;
        for (count, colour) in kinds.iter().zip([GREEN, GOLD, RED]) {
            let high = TALL * *count as f32 / most as f32;
            if high > 0.0 {
                top -= high;
                sheet.block(stage, "timingBar", [left, top, WIDE, high], colour, 1.0);
            }
        }
    }
    let ends = MIDDLE - PITCH * (TIMING_REACH as f32 - 1.0);
    for (across, word) in [
        (ends, "EARLY"),
        (MIDDLE, "ON TIME"),
        (2.0 * MIDDLE - ends, "LATE"),
    ] {
        sheet.write(stage, "timingAxis", word, (across, FLOOR + 5.0), SIZE, PALE);
    }
    let count = |wanted: fn(i32) -> bool| swings.iter().filter(|(off, ..)| wanted(*off)).count();
    let met = swings.iter().filter(|(.., met)| *met).count();
    let sweet = swings
        .iter()
        .filter(|(_, quality, met)| *met && *quality == Some(Quality::Good))
        .count();
    let off: i32 = swings.iter().map(|(off, ..)| *off).sum();
    let usual = off as f32 / swings.len() as f32;
    let usually = match usual {
        usual if usual <= -0.05 => format!("{:.1} FRAMES EARLY", -usual),
        usual if usual >= 0.05 => format!("{usual:.1} FRAMES LATE"),
        _ => "ON TIME".to_owned(),
    };
    let lines = [
        format!(
            "SWINGS {}   EARLY {}   ON TIME {}   LATE {}",
            swings.len(),
            count(|off| off < 0),
            count(|off| off == 0),
            count(|off| off > 0)
        ),
        format!(
            "MET THE BALL {met}: SWEETLY {sweet}, LESS WELL {}   MISSED IT {}",
            met - sweet,
            swings.len() - met
        ),
        format!("ON THE WHOLE: {usually}"),
    ];
    for (row, line) in lines.iter().enumerate() {
        let colour = if row == 2 { GOLD } else { CREAM };
        sheet.write(
            stage,
            "timingLine",
            line,
            (MIDDLE, 250.0 + 16.0 * row as f32),
            SIZE,
            colour,
        );
    }
}
