//! The figures set out in rows, as a board writes them: a batter's line,
//! the lines under a side's batting, and two sides' hitting and pitching
//! side by side.
//!
//! Whatever page they are written on, of one match or of many, these are
//! what it says.

use super::figures::{Figures, average, percent, tenths};

/// A line of a page of figures: what it is of, and what each of two sides
/// has of it.
pub type Row = (&'static str, String, String);

/// What stands at the head of each column of a side's batting.
pub const BATTING_HEADS: [&str; 11] = [
    "BAT", "AB", "R", "H", "2B", "3B", "HR", "RBI", "BB", "SO", "AVG",
];

/// A batter's line under those heads. `who` is what he is known by: his
/// place in the order, or ALL for the whole side.
pub fn batting_cells(who: String, figures: &Figures) -> [String; 11] {
    [
        who,
        figures.at_bats.to_string(),
        figures.runs.to_string(),
        figures.hits.to_string(),
        figures.doubles.to_string(),
        figures.triples.to_string(),
        figures.home_runs.to_string(),
        figures.runs_in.to_string(),
        figures.walks.to_string(),
        figures.strikeouts.to_string(),
        average(figures.average()),
    ]
}

/// The line under a side's batting that says what else its batters did.
/// `errors` are those of the side in the field.
pub fn besides(all: &Figures, errors: u32) -> String {
    format!(
        "LEFT ON BASE {}   DOUBLE PLAYS {}   SACRIFICE FLIES {}   TWO-OUT RUNS {}   ERRORS {}",
        all.left, all.double_plays, all.sacrifices, all.two_out_runs, errors
    )
}

/// The line that tells of the pitcher who threw to a side: `whose` he is,
/// how many of the side he put out, and how many outs it has in an
/// innings. `all` are the figures of the side he threw to.
pub fn pitcher(whose: &str, outs: u32, an_innings: u32, all: &Figures) -> String {
    format!(
        "{whose} PITCHER: {} INNINGS, {} PITCHES, {} STRIKES, {} STRIKEOUTS, {} WALKS",
        innings_pitched(outs, an_innings),
        all.pitches,
        percent(all.strike_rate()),
        all.strikeouts,
        all.walks
    )
}

/// How many innings a pitcher has got through when he has put this many
/// out, as a scorer writes it: the innings, a point, and the outs of the
/// one in hand. `an_innings` is how many outs the side he pitches to has
/// in one.
pub fn innings_pitched(outs: u32, an_innings: u32) -> String {
    let an_innings = an_innings.max(1);
    format!("{}.{}", outs / an_innings, outs % an_innings)
}

/// What a page of figures says of two sides' hitting, the first side's
/// first.
pub fn hitting_rows(one: &Figures, other: &Figures) -> Vec<Row> {
    let feet = |feet: u32| format!("{feet} FT");
    let whole = |value: Option<f32>| value.map_or("-".to_owned(), |value| format!("{value:.0} FT"));
    let mut hitting: Vec<Row> = vec![
        ("AVERAGE", average(one.average()), average(other.average())),
        ("ON BASE", average(one.on_base()), average(other.on_base())),
        (
            "SLUGGING",
            average(one.slugging()),
            average(other.slugging()),
        ),
        (
            "ON BASE + SLUGGING",
            average(one.on_base_plus_slugging()),
            average(other.on_base_plus_slugging()),
        ),
        (
            "ON BALLS IN PLAY",
            average(one.in_play_average()),
            average(other.in_play_average()),
        ),
        (
            "RUNNERS ON 2ND OR 3RD",
            average(one.chance_average()),
            average(other.chance_average()),
        ),
        (
            "TOTAL BASES",
            one.total_bases.to_string(),
            other.total_bases.to_string(),
        ),
        (
            "HOME RUNS",
            one.home_runs.to_string(),
            other.home_runs.to_string(),
        ),
        (
            "TWO-OUT RUNS",
            one.two_out_runs.to_string(),
            other.two_out_runs.to_string(),
        ),
        ("LEFT ON BASE", one.left.to_string(), other.left.to_string()),
        ("LONGEST HIT", feet(one.longest), feet(other.longest)),
        (
            "USUAL HIT",
            whole(one.usual_feet()),
            whole(other.usual_feet()),
        ),
    ];
    // Bases are only stolen with the mod for it on, and there is only a
    // line for them when somebody has tried.
    if one.stolen + one.caught + other.stolen + other.caught > 0 {
        hitting.push(("BASES STOLEN", one.stolen_of(), other.stolen_of()));
    }
    hitting
}

/// What it says of the pitches each side was thrown.
pub fn pitching_rows(one: &Figures, other: &Figures) -> Vec<Row> {
    vec![
        (
            "PITCHES SEEN",
            one.pitches.to_string(),
            other.pitches.to_string(),
        ),
        (
            "PITCHES A TURN",
            tenths(one.pitches_a_turn()),
            tenths(other.pitches_a_turn()),
        ),
        (
            "STRIKES",
            percent(one.strike_rate()),
            percent(other.strike_rate()),
        ),
        (
            "SWUNG AT",
            percent(one.swing_rate()),
            percent(other.swing_rate()),
        ),
        (
            "MET WHEN SWUNG AT",
            percent(one.contact_rate()),
            percent(other.contact_rate()),
        ),
        (
            "MISSED WHEN SWUNG AT",
            percent(one.miss_rate()),
            percent(other.miss_rate()),
        ),
        (
            "CHASED OUTSIDE",
            percent(one.chase_rate()),
            percent(other.chase_rate()),
        ),
        (
            "CALLED STRIKES",
            one.called.to_string(),
            other.called.to_string(),
        ),
        (
            "SWINGING STRIKES",
            one.swinging.to_string(),
            other.swinging.to_string(),
        ),
        ("FOULS", one.fouls.to_string(), other.fouls.to_string()),
        (
            "STRUCK OUT",
            percent(one.strikeout_rate()),
            percent(other.strikeout_rate()),
        ),
        (
            "WALKED",
            percent(one.walk_rate()),
            percent(other.walk_rate()),
        ),
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
