//! The page the delay is measured and set on. It is the last of the mods'
//! pages, on the same panel: a bar with a line on it, which a marker
//! crosses again and again at the timing bar's pace, what the last click
//! came to, and the delay, with arrows to set it by hand.

use bb_engine::display::Path;
use bb_engine::stage::Stage;

use super::Delay;
use super::taps::{CLICKS, LENGTH, LINE, PACE, Tap, Taps};
use crate::art;
use crate::look::{self, Rgb};
use crate::sheet::Sheet;

/// What every line of words on the page is named, as on the mods' pages.
const WORDS: &str = "modsWords";

const DARK: Rgb = look::NAVY;
const SOFT: Rgb = [0x4a, 0x6f, 0x8c];
const BAR: Rgb = [0x0b, 0x3a, 0x5e];
const ON_TIME: Rgb = [0x37, 0xd4, 0x4a];
const OFF_TIME: Rgb = [0xf2, 0x9a, 0x24];

/// Where the words start across the panel, as the mods' names do.
const LEFT: f32 = -140.0;
/// The bar's left end and its top, its height, and the marker's width and
/// how far it stands out above and below the bar.
const BAR_LEFT: f32 = -87.0;
const BAR_TOP: f32 = -46.0;
const BAR_HIGH: f32 = 8.0;
const MARKER_WIDE: f32 = 2.0;
const MARKER_REACH: f32 = 4.0;
/// A click's mark: how far under the bar, how wide and how high.
const MARK_DOWN: f32 = 13.0;
const MARK: (f32, f32) = (2.0, 5.0);
/// How far down the panel the delay is, and how far across its arrows and
/// its figure are: as the arrows that turn the page have them.
const DELAY_DOWN: f32 = 14.0;
const ACROSS: [f32; 3] = [-38.0, -30.0, 42.0];
/// How much a click on an arrow changes the delay by, in thousandths of a
/// second.
const NUDGE: i32 = 5;
/// The sizes of the words: the larger as a mod's name is, the smaller as
/// what is said of one.
const BIG: f32 = 0.85;
const SMALL: f32 = 0.6;

/// The page, while it is up.
pub struct TimingPage {
    taps: Taps,
    /// The marker, white in a dark edge so that it can be seen on the bar
    /// and off it.
    marker: [Path; 2],
    /// A mark under the bar for each click of the measurement.
    marks: Vec<Path>,
    /// What the last click came to, and the delay in figures.
    news: Path,
    figure: Path,
    /// The arrows that take the delay down and up.
    less: Path,
    more: Path,
    /// What came of the last click, until the next.
    last: Option<Tap>,
}

impl TimingPage {
    /// Puts the page on `panel`, with the marker setting off on `frame`.
    pub fn lay_out(panel: &mut Sheet, frame: u64, stage: &mut Stage) -> Option<TimingPage> {
        let says = "CLICK AS THE MARKER CROSSES THE LINE";
        panel.label(stage, WORDS, says, (LEFT, -88.0), BIG, DARK)?;
        let says = "TEN CLICKS MEASURE HOW LATE YOUR CLICKS REACH THE GAME";
        panel.label(stage, WORDS, says, (LEFT, -70.0), SMALL, SOFT)?;

        let edge = 2.0;
        let around = [
            BAR_LEFT - edge,
            BAR_TOP - edge,
            LENGTH + edge * 2.0,
            BAR_HIGH + edge * 2.0,
        ];
        panel.block(stage, "delayBar", around, DARK, 0.8);
        panel.block(
            stage,
            "delayBar",
            [BAR_LEFT, BAR_TOP, LENGTH, BAR_HIGH],
            BAR,
            1.0,
        );
        let line = [BAR_LEFT + LINE - 1.0, BAR_TOP - 3.0, 2.0, BAR_HIGH + 6.0];
        panel.block(stage, "delayLine", line, ON_TIME, 1.0);
        let mut marks = Vec::new();
        for _ in 0..CLICKS {
            let at = (BAR_LEFT, BAR_TOP + MARK_DOWN);
            marks.push(block(panel, stage, "delayMark", at, MARK)?);
        }
        let high = BAR_HIGH + MARKER_REACH * 2.0;
        let at = (BAR_LEFT - 1.0, BAR_TOP - MARKER_REACH - 1.0);
        let edge = block(
            panel,
            stage,
            "delayMarker",
            at,
            (MARKER_WIDE + 2.0, high + 2.0),
        )?;
        let at = (BAR_LEFT, BAR_TOP - MARKER_REACH);
        let white = block(panel, stage, "delayMarker", at, (MARKER_WIDE, high))?;
        for (part, colour) in [(&edge, DARK), (&white, look::WHITE)] {
            if let Some(part) = stage.child_mut(part) {
                part.set_color(look::tint(colour));
            }
        }

        let news = panel.label(stage, WORDS, "", (LEFT, -20.0), 0.7, DARK)?;
        panel.label(stage, WORDS, "DELAY", (LEFT, DELAY_DOWN), BIG, DARK)?;
        let [less, figure, more] = ACROSS;
        let less = panel.add(
            stage,
            art::PAGE_TURN,
            "delayLess",
            (less, DELAY_DOWN + 2.0),
            (1.0, 1.0),
        )?;
        // The art's arrow points on. The one down is the same, turned round.
        if let Some(arrow) = stage.child_mut(&less) {
            let mut turned = arrow.matrix;
            turned.a = -turned.a;
            arrow.set_matrix(turned);
        }
        let figure = panel.label(stage, WORDS, "", (figure, DELAY_DOWN), BIG, DARK)?;
        let more = panel.add(
            stage,
            art::PAGE_TURN,
            "delayMore",
            (more, DELAY_DOWN + 2.0),
            (1.0, 1.0),
        )?;
        let says = "EVERY SWING IS TAKEN AS MADE THIS MUCH SOONER";
        panel.label(stage, WORDS, says, (LEFT, DELAY_DOWN + 24.0), SMALL, SOFT)?;
        let says = "THE ARROWS SET IT BY HAND, FIVE AT A TIME";
        panel.label(stage, WORDS, says, (LEFT, DELAY_DOWN + 36.0), SMALL, SOFT)?;

        Some(TimingPage {
            taps: Taps::new(frame),
            marker: [edge, white],
            marks,
            news,
            figure,
            less,
            more,
            last: None,
        })
    }

    /// How much a click on the button at `path` changes the delay by, if
    /// it is one of the arrows.
    pub fn nudge(&self, path: &[u16]) -> Option<i32> {
        if self.less == path {
            Some(-NUDGE)
        } else if self.more == path {
            Some(NUDGE)
        } else {
            None
        }
    }

    /// Keeps the page true to this frame for a player whose delay is
    /// `delay`, and takes in a click made on it. Returns the delay a
    /// measurement has found, when one has.
    pub fn show(&mut self, delay: Delay, stage: &mut Stage) -> Option<Delay> {
        let frame = stage.frames_played();
        let frame_rate = stage.library().manifest.stage.frame_rate;
        let mut found = None;
        if stage.pointer.went_down.is_some() {
            // A click the window did not time was made on the frame
            // before this one, which was the one on the screen.
            let at = stage.pointer.went_down_at.unwrap_or(frame as f64 - 1.0);
            let tap = self.taps.tap(at, frame_rate);
            if let Tap::Measured(measured) = tap {
                found = Some(measured);
            }
            self.last = Some(tap);
        }
        let delay = found.unwrap_or(delay);

        let marker = self.taps.marker(frame);
        for (part, wider) in self.marker.iter().zip([2.0, 0.0]) {
            if let Some(child) = stage.child_mut(part) {
                child.set_visible(marker.is_some());
                let mut matrix = child.matrix;
                matrix.tx = BAR_LEFT + marker.unwrap_or(0.0) - (MARKER_WIDE + wider) / 2.0;
                child.set_matrix(matrix);
            }
        }
        self.show_marks(delay.frames(frame_rate), stage);
        let news = self.news(frame_rate);
        say(stage, &self.news, &news);
        say(stage, &self.figure, &format!("{} MS", delay.ms()));
        found
    }

    /// Marks under the bar where the marker was, as the game takes it, for
    /// each click of the measurement: in green on the line, and in orange
    /// off it.
    fn show_marks(&self, delay: f64, stage: &mut Stage) {
        let late = self.taps.late();
        for (index, mark) in self.marks.iter().enumerate() {
            let Some(child) = stage.child_mut(mark) else {
                continue;
            };
            let Some(&late) = late.get(index) else {
                child.set_visible(false);
                continue;
            };
            // A click taken as made on the frame the marker was on the
            // line is at the line. Each frame either side is a step of
            // the marker's either side of it.
            let off = (late - delay).floor();
            let along = (LINE + off as f32 * PACE).clamp(0.0, LENGTH);
            let mut matrix = child.matrix;
            matrix.tx = BAR_LEFT + along - MARK.0 / 2.0;
            child.set_matrix(matrix);
            child.set_color(look::tint(if off == 0.0 { ON_TIME } else { OFF_TIME }));
            child.set_visible(true);
        }
    }

    /// What is said of the last click.
    fn news(&self, frame_rate: f64) -> String {
        let ms = |frames: f64| (frames * 1000.0 / frame_rate).round() as i64;
        match self.last {
            None => String::new(),
            Some(Tap::Counted(late)) => {
                let (by, which) = if late < 0.0 {
                    (-late, "EARLY")
                } else {
                    (late, "LATE")
                };
                let count = self.taps.counted();
                format!("CLICK {count} OF {CLICKS}: {} MS {which}", ms(by))
            }
            Some(Tap::TooFar) => "TOO FAR FROM THE LINE TO COUNT".to_owned(),
            Some(Tap::Measured(delay)) => format!("MEASURED: THE DELAY IS NOW {} MS", delay.ms()),
            Some(Tap::TooSpread) => "TOO SPREAD OUT TO TRUST: TRY TEN MORE".to_owned(),
        }
    }
}

/// A plain block of colour on `panel`: where its top left corner is, and
/// how wide and how high.
fn block(
    panel: &mut Sheet,
    stage: &mut Stage,
    name: &str,
    at: (f32, f32),
    size: (f32, f32),
) -> Option<Path> {
    let size = (size.0 / art::BLOCK_SIDE, size.1 / art::BLOCK_SIDE);
    panel.add(stage, art::BLOCK, name, at, size)
}

/// Has the words at `path` say `says`, if they do not already.
fn say(stage: &mut Stage, path: &[u16], says: &str) {
    if let Some(words) = stage.child_mut(path)
        && words.said.as_deref() != Some(says)
    {
        words.said = Some(says.to_owned());
    }
}
