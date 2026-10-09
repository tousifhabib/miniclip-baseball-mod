//! The page of figures: both sides' hitting and pitching, row by row.

use bb_engine::stage::Stage;

use super::{CREAM, GOLD, PALE};
use crate::play::book::{Figures, Side, average, percent, tenths};
use crate::sheet::Sheet;

/// How many innings a pitcher has got through when he has put this many
/// out, as a scorer writes it: the innings, a point, and the outs of the
/// one in hand. `an_innings` is how many outs the side he pitches to has
/// in one.
pub(super) fn innings_pitched(outs: u32, an_innings: u32) -> String {
    let an_innings = an_innings.max(1);
    format!("{}.{}", outs / an_innings, outs % an_innings)
}

/// A line of the page of figures: what it is of, and what each side has
/// of it, the player's first.
type Row = (&'static str, String, String);

/// The page of the two sides' figures, side by side.
pub(super) fn figures(ours: &Side, theirs: &Side, sheet: &mut Sheet, stage: &mut Stage) {
    const SIZE: f32 = 0.68;
    const TOP: f32 = 90.0;
    const PITCH: f32 = 16.2;
    let (us, them) = (ours.figures(), theirs.figures());
    let (hitting, pitches) = (hitting_rows(&us, &them), pitching_rows(&us, &them));
    // Each half of the page: where its words begin, and the middles of the
    // two sides' columns.
    for (rows, left, columns) in [
        (hitting, 38.0, [208.0, 258.0]),
        (pitches, 300.0, [478.0, 530.0]),
    ] {
        for (across, side, colour) in [(columns[0], "YOU", GOLD), (columns[1], "THEM", CREAM)] {
            sheet.write(stage, "figuresHead", side, (across, TOP), SIZE, colour);
        }
        for (row, (name, us, them)) in rows.into_iter().enumerate() {
            let down = TOP + PITCH * (row + 1) as f32;
            sheet.write_left(stage, "figuresName", name, (left, down), SIZE, PALE);
            sheet.write(stage, "figuresOurs", &us, (columns[0], down), SIZE, GOLD);
            sheet.write(
                stage,
                "figuresTheirs",
                &them,
                (columns[1], down),
                SIZE,
                CREAM,
            );
        }
    }
}

/// What the page of figures says of each side's hitting.
fn hitting_rows(us: &Figures, them: &Figures) -> Vec<Row> {
    let feet = |feet: u32| format!("{feet} FT");
    let whole = |value: Option<f32>| value.map_or("-".to_owned(), |value| format!("{value:.0} FT"));
    let mut hitting: Vec<Row> = vec![
        ("AVERAGE", average(us.average()), average(them.average())),
        ("ON BASE", average(us.on_base()), average(them.on_base())),
        ("SLUGGING", average(us.slugging()), average(them.slugging())),
        (
            "ON BASE + SLUGGING",
            average(us.on_base_plus_slugging()),
            average(them.on_base_plus_slugging()),
        ),
        (
            "ON BALLS IN PLAY",
            average(us.in_play_average()),
            average(them.in_play_average()),
        ),
        (
            "RUNNERS ON 2ND OR 3RD",
            average(us.chance_average()),
            average(them.chance_average()),
        ),
        (
            "TOTAL BASES",
            us.total_bases.to_string(),
            them.total_bases.to_string(),
        ),
        (
            "HOME RUNS",
            us.home_runs.to_string(),
            them.home_runs.to_string(),
        ),
        (
            "TWO-OUT RUNS",
            us.two_out_runs.to_string(),
            them.two_out_runs.to_string(),
        ),
        ("LEFT ON BASE", us.left.to_string(), them.left.to_string()),
        ("LONGEST HIT", feet(us.longest), feet(them.longest)),
        (
            "USUAL HIT",
            whole(us.usual_feet()),
            whole(them.usual_feet()),
        ),
    ];
    // Bases are only stolen with the mod for it on, and there is only a
    // line for them when somebody has tried.
    if us.stolen + us.caught + them.stolen + them.caught > 0 {
        hitting.push(("BASES STOLEN", us.stolen_of(), them.stolen_of()));
    }
    hitting
}

/// What it says of the pitches each side was thrown.
fn pitching_rows(us: &Figures, them: &Figures) -> Vec<Row> {
    vec![
        (
            "PITCHES SEEN",
            us.pitches.to_string(),
            them.pitches.to_string(),
        ),
        (
            "PITCHES A TURN",
            tenths(us.pitches_a_turn()),
            tenths(them.pitches_a_turn()),
        ),
        (
            "STRIKES",
            percent(us.strike_rate()),
            percent(them.strike_rate()),
        ),
        (
            "SWUNG AT",
            percent(us.swing_rate()),
            percent(them.swing_rate()),
        ),
        (
            "MET WHEN SWUNG AT",
            percent(us.contact_rate()),
            percent(them.contact_rate()),
        ),
        (
            "MISSED WHEN SWUNG AT",
            percent(us.miss_rate()),
            percent(them.miss_rate()),
        ),
        (
            "CHASED OUTSIDE",
            percent(us.chase_rate()),
            percent(them.chase_rate()),
        ),
        (
            "CALLED STRIKES",
            us.called.to_string(),
            them.called.to_string(),
        ),
        (
            "SWINGING STRIKES",
            us.swinging.to_string(),
            them.swinging.to_string(),
        ),
        ("FOULS", us.fouls.to_string(), them.fouls.to_string()),
        (
            "STRUCK OUT",
            percent(us.strikeout_rate()),
            percent(them.strikeout_rate()),
        ),
        ("WALKED", percent(us.walk_rate()), percent(them.walk_rate())),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pitchers_innings_are_counted_by_the_outs_the_side_he_pitches_to_has() {
        // Ten out at three to an innings: three innings and one out.
        assert_eq!(innings_pitched(10, 3), "3.1");
        assert_eq!(innings_pitched(27, 3), "9.0");
        // The same ten at five to an innings are two innings exactly.
        assert_eq!(innings_pitched(10, 5), "2.0");
        assert_eq!(innings_pitched(0, 3), "0.0");
        // Rules that give a side no outs are not divided by.
        assert_eq!(innings_pitched(4, 0), "4.0");
    }
}
