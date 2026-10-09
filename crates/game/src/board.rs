//! What a full match writes on the art's boards: between innings, what the
//! other side has just done and how both sides stand, and when the match is
//! over, pages of what the book has to say of it.

use bb_engine::display::Path;
use bb_engine::library::Library;
use bb_engine::stage::Stage;

use crate::art;
use crate::look::{self, Rgb};
use crate::play::book::{End, Figures, ORDER, Side, Turn, average, percent, tenths};
use crate::play::full::{Cell, FullMatch, hits_words, ordinal, runs_words};
use crate::play::overlay::Words;
use crate::play::paper;
use crate::play::pitch::Quality;
use crate::sheet::Sheet;

/// The colours of the board's lettering: as the art has it, for the side
/// that is the player's, and for headings. The rest are for what is drawn.
const CREAM: Rgb = look::CREAM;
const GOLD: Rgb = [0xff, 0xd2, 0x4a];
const PALE: Rgb = [0xa9, 0xdc, 0xf0];
const WHITE: Rgb = look::WHITE;
const RED: Rgb = [0xff, 0x6e, 0x5c];
const GREEN: Rgb = [0x86, 0xf0, 0x8c];
const GREY: Rgb = [0xb4, 0xc2, 0xcc];
const BACKING: Rgb = [0x05, 0x1c, 0x30];

/// How wide the columns of the innings are, with the lettering at its own
/// size: the one for the sides' names, one for each innings, and one each
/// for the runs, hits and errors in all.
const NAME_WIDTH: f32 = 70.0;
const INNINGS_WIDTH: f32 = 30.0;
const ALL_WIDTH: f32 = 34.0;

/// Where things go on the board between innings, from its middle: how far
/// down the heading is and its size, the lettering's own being 1, then the
/// line that says what the other side made, the innings, and the lines
/// after them.
const HEADING: (f32, f32) = (-126.0, 1.6);
const FIRST_LINE: f32 = -82.0;
const INNINGS: (f32, f32) = (-44.0, 0.9);
const LATER_LINES: f32 = 40.0;
const LINE_PITCH: f32 = 25.0;
/// How far apart the rows of the innings are, the lettering at its own
/// size.
const ROW_PITCH: f32 = 24.0;

/// Where things go on the boards a match ends on, from the corner of the
/// stage: the middle across, how far down the line that says who won is,
/// how far down the innings are and their size.
const MIDDLE: f32 = 295.0;
const VERDICT_TOP: f32 = 123.0;
const RESULT_INNINGS: (f32, f32) = (243.0, 0.75);
const VERDICT_SIZE: f32 = 0.8;
/// How far down the line about zingers is on a full match's board.
const ZINGER_TOP: f32 = 143.0;

/// The pages after the first have a backing of their own, under a heading:
/// its left, top, width and height, and how solid it is.
const PANEL: [f32; 4] = [28.0, 84.0, 534.0, 228.0];
const PANEL_ALPHA: f32 = 0.55;
const PAGE_HEADING: (f32, f32) = (58.0, 1.35);
/// The arrows that turn the pages, and the words between them: how far
/// down, how far either side of the middle the arrows are, and the size of
/// the words.
const PAGER_TOP: f32 = 317.0;
const PAGER_REACH: f32 = 62.0;
const PAGER_SIZE: f32 = 0.7;
/// How many turns a column of an innings' page has room for.
const TURN_ROWS: usize = 14;
/// The picture of the field on its page: its size, the art's being 1, and
/// where the corner of the field's own pixels goes. The picture is of more
/// than a match ever shows, so only that much of it is let through: its
/// left, top, width and height in the field's pixels.
const FIELD_SIZE: f32 = 0.42;
const FIELD_AT: (f32, f32) = (56.0, 114.0);
const FIELD_SEEN: [f32; 4] = [-46.0, -35.0, 678.0, 460.0];
/// How far either side of the best moment the page of timing shows, in
/// frames. Swings further off than that are counted with the furthest.
const TIMING_REACH: i32 = 8;

/// Writes what both sides made in each innings: the numbers of the innings,
/// then a row for each side, with its runs, hits and errors in all at the
/// end. `middle` is the middle of the rows across and `top` the top of the
/// first.
fn innings(
    full: &FullMatch,
    sheet: &mut Sheet<'_>,
    middle: f32,
    top: f32,
    size: f32,
    stage: &mut Stage,
) {
    let (first, count) = full.shown();
    let wide = NAME_WIDTH + INNINGS_WIDTH * count as f32 + ALL_WIDTH * 3.0;
    let left = middle - wide * size / 2.0;
    let name_at = left + NAME_WIDTH * size / 2.0;
    let innings_at =
        |column: u32| left + (NAME_WIDTH + INNINGS_WIDTH * (column as f32 + 0.5)) * size;
    let all_at = |column: f32| left + (wide - ALL_WIDTH * (2.5 - column)) * size;
    for column in 0..count {
        let number = (first + column).to_string();
        let at = (innings_at(column), top);
        sheet.write(stage, "boardInnings", &number, at, size, PALE);
    }
    for (column, letter) in ["R", "H", "E"].into_iter().enumerate() {
        let at = (all_at(column as f32), top);
        sheet.write(stage, "boardInnings", letter, at, size, PALE);
    }
    for (row, line) in full.lines().iter().enumerate() {
        let down = top + ROW_PITCH * size * (row + 1) as f32;
        let colour = if line.ours { GOLD } else { CREAM };
        sheet.write(stage, "boardSide", line.name, (name_at, down), size, colour);
        for (column, cell) in line.cells.iter().enumerate() {
            let says = match cell {
                Cell::Blank => continue,
                Cell::Runs(runs) => runs.to_string(),
                Cell::NotNeeded => "X".to_owned(),
            };
            let at = (innings_at(column as u32), down);
            sheet.write(stage, "boardCell", &says, at, size, colour);
        }
        let all = [
            ("boardRuns", line.runs),
            ("boardHits", line.hits),
            ("boardErrors", line.errors),
        ];
        for (column, (name, number)) in all.into_iter().enumerate() {
            let at = (all_at(column as f32), down);
            sheet.write(stage, name, &number.to_string(), at, size, colour);
        }
    }
}

/// Writes what the board between innings says, on the art's board at
/// `board`. Returns the clip it is all in.
pub fn interval(
    full: &FullMatch,
    board: &[u16],
    stage: &mut Stage,
    library: &Library,
) -> Option<Path> {
    let at = Stage::RULES_DEPTH + 1;
    let holder = stage.attach(board, art::HOLDER, at, "intervalBoard", library)?;
    let mut sheet = Sheet::on(holder.clone(), 1, library);
    let report = full.report();
    let (down, size) = HEADING;
    sheet.write(
        stage,
        "boardHeading",
        &report.heading,
        (0.0, down),
        size,
        CREAM,
    );
    let mut lines = report.lines.iter();
    if let Some(line) = lines.next() {
        sheet.write(stage, "boardLine", line, (0.0, FIRST_LINE), 1.0, CREAM);
    }
    let (down, size) = INNINGS;
    innings(full, &mut sheet, 0.0, down, size, stage);
    for (index, line) in lines.enumerate() {
        let top = (0.0, LATER_LINES + LINE_PITCH * index as f32);
        // The last line of more than two is the one not to miss.
        let colour = if index >= 2 { GOLD } else { CREAM };
        sheet.write(stage, "boardLine", line, top, 1.0, colour);
    }
    Some(holder)
}

/// What one of the pages a finished match has is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    /// Who won, the art's own figures, and every innings.
    Score,
    /// What each batter of one side did.
    Batting { ours: bool },
    /// The two sides' figures, side by side.
    Figures,
    /// Where one side's hits went.
    Field { ours: bool },
    /// How the player's swings were timed.
    Timing,
    /// The turns of one innings, both halves. A long innings has more than
    /// one part.
    Innings { innings: u32, part: usize },
}

/// The pages of a finished full match on the board the game ends on, with
/// arrows to turn them by.
pub struct Pages {
    /// The clip it is all in, which lies over the whole stage.
    holder: Path,
    full: FullMatch,
    /// The outs the player's side had in an innings.
    our_outs: u32,
    /// What there is to say about zingers, if there were any.
    zingers: Option<String>,
    pages: Vec<Page>,
    page: usize,
    /// The clip the page that is up is written in.
    sheet: Option<Path>,
    back: Path,
    on: Path,
    count: Words,
}

impl Pages {
    /// Puts the first page up in the clip at `holder`. `our_outs` is how
    /// many outs the player's side had in an innings.
    pub fn new(
        full: &FullMatch,
        our_outs: u32,
        zingers: Option<String>,
        holder: &[u16],
        stage: &mut Stage,
        library: &Library,
    ) -> Option<Pages> {
        let mut pages = vec![
            Page::Score,
            Page::Batting { ours: true },
            Page::Batting { ours: false },
            Page::Figures,
            Page::Field { ours: true },
            Page::Field { ours: false },
            Page::Timing,
        ];
        let played = full.book.ours.turns.iter().chain(&full.book.theirs.turns);
        let last = played.map(|turn| turn.innings).max().unwrap_or(0);
        for innings in 1..=last {
            let most = [&full.book.ours, &full.book.theirs]
                .map(|side| side.told(innings).len())
                .into_iter()
                .max()
                .unwrap_or(0);
            for part in 0..most.div_ceil(TURN_ROWS).max(1) {
                pages.push(Page::Innings { innings, part });
            }
        }
        let mut sheet = Sheet::on(holder.to_vec(), 500, library);
        // The art's arrow points on. The one back is the same, turned
        // round.
        let down = PAGER_TOP;
        let back = sheet.add(
            stage,
            art::BOARD_TURN,
            "pageBack",
            (MIDDLE - PAGER_REACH, down),
            (-1.0, 1.0),
        )?;
        let on = sheet.add(
            stage,
            art::BOARD_TURN,
            "pageOn",
            (MIDDLE + PAGER_REACH, down),
            (1.0, 1.0),
        )?;
        let depth = sheet.depth;
        let top = (MIDDLE, PAGER_TOP - 1.0);
        let count = Words::new(holder, depth, "pageCount", top, PAGER_SIZE, stage, library)?;
        let mut pages = Pages {
            holder: holder.to_vec(),
            full: full.clone(),
            our_outs,
            zingers,
            pages,
            page: 0,
            sheet: None,
            back,
            on,
            count,
        };
        pages.draw(stage, library);
        Some(pages)
    }

    /// Takes in a click on the button at `path`, which may be one of the
    /// arrows. They go round: back from the first page is the last.
    pub fn clicked(&mut self, path: &[u16], stage: &mut Stage, library: &Library) {
        let pages = self.pages.len();
        if self.back == path {
            self.page = (self.page + pages - 1) % pages;
        } else if self.on == path {
            self.page = (self.page + 1) % pages;
        } else {
            return;
        }
        self.draw(stage, library);
    }

    /// Called every frame the board is up. The art's own figures are only
    /// for the first page.
    pub fn keep(&self, stage: &mut Stage) {
        let Some(shell) = art::shell(stage) else {
            return;
        };
        for figures in art::RESULT_FIGURES {
            let found = stage.find_symbol(&shell, figures);
            if let Some(figures) = found.and_then(|path| stage.child_mut(&path)) {
                figures.set_visible(self.page == 0);
            }
        }
    }

    /// Which page is up, counting from 1, and how many there are.
    pub fn at(&self) -> (usize, usize) {
        (self.page + 1, self.pages.len())
    }

    /// Writes the page that is up, in place of the one that was.
    fn draw(&mut self, stage: &mut Stage, library: &Library) {
        if let Some(old) = self.sheet.take() {
            stage.remove(&old);
        }
        let says = format!("PAGE {} OF {}", self.page + 1, self.pages.len());
        self.count.say(&says, CREAM, stage);
        let Some(holder) = stage.attach(&self.holder, art::HOLDER, 10, "resultPage", library)
        else {
            return;
        };
        self.sheet = Some(holder.clone());
        let mut sheet = Sheet::on(holder, 1, library);
        let full = &self.full;
        let page = self.pages[self.page];
        if page == Page::Score {
            let top = (MIDDLE, VERDICT_TOP);
            sheet.write(
                stage,
                "boardVerdict",
                &full.verdict(),
                top,
                VERDICT_SIZE,
                CREAM,
            );
            if let Some(zingers) = &self.zingers {
                let top = (MIDDLE, ZINGER_TOP);
                sheet.write(stage, "zingerLine", zingers, top, VERDICT_SIZE, CREAM);
            }
            let (down, size) = RESULT_INNINGS;
            return innings(full, &mut sheet, MIDDLE, down, size, stage);
        }
        sheet.block(stage, "pagePanel", PANEL, BACKING, PANEL_ALPHA);
        let side = |ours: bool| {
            if ours {
                &full.book.ours
            } else {
                &full.book.theirs
            }
        };
        let heading = match page {
            Page::Score => String::new(),
            Page::Batting { ours: true } => "YOUR BATTING".to_owned(),
            Page::Batting { ours: false } => "THEIR BATTING".to_owned(),
            Page::Figures => "THE FIGURES".to_owned(),
            Page::Field { ours: true } => "WHERE YOU HIT IT".to_owned(),
            Page::Field { ours: false } => "WHERE THEY HIT IT".to_owned(),
            Page::Timing => "YOUR TIMING".to_owned(),
            Page::Innings { innings, part: 0 } => format!("THE {} INNINGS", ordinal(innings)),
            Page::Innings { innings, .. } => format!("THE {} INNINGS, GOING ON", ordinal(innings)),
        };
        let (down, size) = PAGE_HEADING;
        sheet.write(stage, "pageHeading", &heading, (MIDDLE, down), size, CREAM);
        match page {
            Page::Score => {}
            Page::Batting { ours } => {
                // The other side's innings are played on paper, by the
                // game's own old rules.
                let outs = if ours { self.our_outs } else { paper::OUTS };
                batting(side(ours), side(!ours), (ours, outs), &mut sheet, stage);
            }
            Page::Figures => figures(&full.book.ours, &full.book.theirs, &mut sheet, stage),
            Page::Field { ours } => field(side(ours), &mut sheet, stage),
            Page::Timing => timing(&full.book.ours, &mut sheet, stage),
            Page::Innings { innings, part } => turns(full, innings, part, &mut sheet, stage),
        }
    }
}

/// How many innings a pitcher has got through when he has put this many
/// out, as a scorer writes it: the innings, a point, and the outs of the
/// one in hand. `an_innings` is how many outs the side he pitches to has
/// in one.
fn innings_pitched(outs: u32, an_innings: u32) -> String {
    let an_innings = an_innings.max(1);
    format!("{}.{}", outs / an_innings, outs % an_innings)
}

/// The page of what each batter of a side did. `fielding` is the side that
/// was in the field, whose errors and whose pitcher's figures these are
/// too. `whose` is whether the side batting is the player's, and how many
/// outs it had in an innings.
fn batting(
    side: &Side,
    fielding: &Side,
    whose: (bool, u32),
    sheet: &mut Sheet<'_>,
    stage: &mut Stage,
) {
    let (ours, outs_an_innings) = whose;
    const ACROSS: [f32; 11] = [
        62.0, 108.0, 150.0, 192.0, 234.0, 276.0, 318.0, 364.0, 408.0, 450.0, 506.0,
    ];
    const HEADS: [&str; 11] = [
        "BAT", "AB", "R", "H", "2B", "3B", "HR", "RBI", "BB", "SO", "AVG",
    ];
    const SIZE: f32 = 0.7;
    const TOP: f32 = 90.0;
    const PITCH: f32 = 15.5;
    for (across, head) in ACROSS.into_iter().zip(HEADS) {
        sheet.write(stage, "battingHead", head, (across, TOP), SIZE, PALE);
    }
    let all = side.figures();
    let rows = (0..ORDER)
        .map(|order| ((order + 1).to_string(), side.figures_of(order)))
        .chain([("ALL".to_owned(), all)]);
    for (row, (who, figures)) in rows.enumerate() {
        // The row for the whole side stands a little apart.
        let last = row == ORDER;
        let down = TOP + 17.0 + PITCH * row as f32 + if last { 6.0 } else { 0.0 };
        let colour = if last { GOLD } else { CREAM };
        let cells = [
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
        ];
        let name = if last { "battingAll" } else { "battingCell" };
        for (across, cell) in ACROSS.into_iter().zip(cells) {
            sheet.write(stage, name, &cell, (across, down), SIZE, colour);
        }
    }
    let left = format!(
        "LEFT ON BASE {}   DOUBLE PLAYS {}   SACRIFICE FLIES {}   TWO-OUT RUNS {}   ERRORS {}",
        all.left, all.double_plays, all.sacrifices, all.two_out_runs, fielding.errors
    );
    sheet.write(stage, "battingLine", &left, (MIDDLE, 276.0), 0.6, PALE);
    let whose = if ours { "THEIR" } else { "YOUR" };
    let pitcher = format!(
        "{whose} PITCHER: {} INNINGS, {} PITCHES, {} STRIKES, {} STRIKEOUTS, {} WALKS",
        innings_pitched(side.outs(), outs_an_innings),
        all.pitches,
        percent(all.strike_rate()),
        all.strikeouts,
        all.walks
    );
    sheet.write(stage, "pitcherLine", &pitcher, (MIDDLE, 292.0), 0.6, PALE);
}

/// A line of the page of figures: what it is of, and what each side has
/// of it, the player's first.
type Row = (&'static str, String, String);

/// The page of the two sides' figures, side by side.
fn figures(ours: &Side, theirs: &Side, sheet: &mut Sheet<'_>, stage: &mut Stage) {
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
fn field(side: &Side, sheet: &mut Sheet<'_>, stage: &mut Stage) {
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

/// The page of how the player's swings were timed: a bar for each frame
/// early or late, as tall as the swings that began on it are many, and what
/// they come to under it.
fn timing(side: &Side, sheet: &mut Sheet<'_>, stage: &mut Stage) {
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

/// A page of an innings: the turns of the visitors' half down the left,
/// and of the home side's down the right.
fn turns(full: &FullMatch, innings: u32, part: usize, sheet: &mut Sheet<'_>, stage: &mut Stage) {
    const SIZE: f32 = 0.62;
    const TOP: f32 = 90.0;
    const PITCH: f32 = 13.6;
    let home = full.at_home();
    // The visitors bat in the top of the innings.
    let halves = [(!home, "TOP", 38.0), (home, "BOTTOM", 302.0)];
    for (ours, half, left) in halves {
        let side = if ours {
            &full.book.ours
        } else {
            &full.book.theirs
        };
        let runs: u32 = side.innings(innings).map(|turn| turn.runs_in).sum();
        let hits = side.hits_in(innings);
        // Every turn, and every try at stealing a base, in the order they
        // came.
        let all = side.told(innings);
        let who = if ours { "YOU" } else { "THEM" };
        let head = if all.is_empty() {
            format!("{half}: {who}, NOT BATTED")
        } else {
            format!(
                "{half}: {who}, {} ON {}",
                runs_words(runs),
                hits_words(hits)
            )
        };
        let colour = if ours { GOLD } else { CREAM };
        sheet.write_left(stage, "turnsHead", &head, (left, TOP), 0.66, colour);
        let shown = all.iter().skip(part * TURN_ROWS).take(TURN_ROWS);
        for (row, (line, scored)) in shown.enumerate() {
            let down = TOP + 18.0 + PITCH * row as f32;
            // A turn that brought a run in stands out.
            let colour = if *scored { GOLD } else { CREAM };
            sheet.write_left(stage, "turnLine", line, (left, down), SIZE, colour);
        }
    }
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
