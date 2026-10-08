//! How the original's art is put together: which symbol is which screen,
//! and what each button says.
//!
//! Everything here describes the extracted files. None of it is game logic.

use std::collections::HashMap;

use bb_engine::display::{Content, Path};
use bb_engine::library::Library;
use bb_engine::stage::Stage;
use bb_format::{Op, PlaceAction, SymbolId, SymbolInfo};

use crate::play::field::{Ground, reach};
use crate::rules::Rules;

/// The clip that holds every screen, one per labelled frame.
pub const SHELL: SymbolId = 2027;
/// The opening animation, on the shell's `intro` frame.
pub const INTRO: SymbolId = 165;
/// The menu, on the shell's `menu` frame. Its labelled sections are the
/// pages of the menu.
pub const MENU: SymbolId = 489;
/// The line on the menu's first page that named the game's author, which
/// this game's menu no longer shows.
pub const MENU_CREDIT: SymbolId = 230;
/// The "are you sure?" panel shown over a game.
pub const QUIT_PROMPT: SymbolId = 1675;
/// The three versions of the instructions, each a clip that stops at the end
/// of every page.
pub const INSTRUCTIONS: [SymbolId; 3] = [2026, 1988, 2000];

/// The strips of colour on the setup pages that a click picks from.
pub const CLOTHES_STRIP: SymbolId = 247;
pub const SKIN_STRIP: SymbolId = 439;
/// The unseen buttons lying over those strips.
pub const CLOTHES_STRIP_BUTTONS: [SymbolId; 2] = [248, 449];
pub const SKIN_STRIP_BUTTONS: [SymbolId; 1] = [450];
/// Buttons that choose one set colour: the button, and the colour.
pub const CLOTHES_BUTTON: (SymbolId, [u8; 3]) = (445, [0x00, 0x33, 0x66]);
pub const SKIN_BUTTON: (SymbolId, [u8; 3]) = (446, [0xbb, 0xa6, 0x74]);
/// Buttons that choose the logo on the bat: the button, and the frame of the
/// art's logo clip it shows.
pub const LOGO_BUTTONS: [(SymbolId, &str); 6] = [
    (447, "fire"),
    (448, "eagle"),
    (451, "miniclip"),
    (452, "sweetspot"),
    (453, "bigbat"),
    (454, "warclub"),
];

/// The panel on the high-score page, and the parts of it that tell the
/// player the scores are kept elsewhere: one drawing that holds both the
/// panel's heading and that notice, and a link to the web site.
pub const SCORE_PANEL: SymbolId = 422;
pub const SCORE_PANEL_NOTICE: [SymbolId; 2] = [421, 420];
/// A text field in the game's display lettering, with letters and figures,
/// which the table is written in.
pub const TABLE_FIELD: SymbolId = 1762;

/// The art the mods add: none of it was in the original.
///
/// A plain white square ten pixels a side, to be stretched and tinted into
/// whatever bar or band is wanted.
pub const BLOCK: SymbolId = 2031;
/// The side of [`BLOCK`], in pixels.
pub const BLOCK_SIDE: f32 = 10.0;
/// A text field in the table's lettering that starts from its left edge.
pub const LABEL_FIELD: SymbolId = 2032;
/// The backing and the publisher's mark the score table's panel is drawn
/// with. The mods' page takes them off and puts on a backing of its own,
/// which has no tabs for the table's days, weeks and months, under the
/// panel's border.
pub const SCORE_PANEL_TABLE: [SymbolId; 2] = [413, 414];
pub const MODS_PANEL: SymbolId = 2038;
pub const MODS_PANEL_DEPTH: u16 = 18;
/// The box beside each mod on that page, and the tick that shows in it.
pub const MOD_BOX: SymbolId = 2041;
pub const MOD_TICK: SymbolId = 2040;
/// A small box for one level of a mod's setting. The rules fill it with a
/// [`BLOCK`] when the setting is at that level or above.
pub const MOD_PIP: SymbolId = 2043;
/// An arrow, pointing right, that turns the page of the list of mods.
pub const PAGE_TURN: SymbolId = 2045;
/// An empty clip, for the rules to put things in and move as one.
pub const HOLDER: SymbolId = 2042;

/// What the full match adds to the menu: a text field in the menu's own
/// lettering, which centres what it says, and a small box to choose with,
/// with room beside it for a word.
pub const MENU_FIELD: SymbolId = 2051;
pub const CHOICE: SymbolId = 2052;
/// The round badge the menu's rows and headings begin with, as its two
/// drawings, and the nine that is the full match's picture in it. The nine
/// is drawn to lie on a badge this far down and to the right of it.
pub const BADGE: [SymbolId; 2] = [69, 70];
pub const NINE: SymbolId = 2048;
pub const NINE_FROM_BADGE: (f32, f32) = (-16.7, -4.4);
/// The heading and the words of the page that says what a match is to be,
/// which a full match writes its own in place of.
pub const SUMMARY_WORDS: [SymbolId; 2] = [481, 486];

/// The board that says an innings was tied. A full match shows it between
/// innings, with its own words on it in place of the art's: the heading,
/// which is a drawing, and the lines under it.
pub const BOARD: SymbolId = 1769;
pub const BOARD_WORDS: [SymbolId; 5] = [1760, 1761, 1762, 1763, 1764];
/// The frame of the board on which its words arrive.
pub const BOARD_WORDS_FRAME: u16 = 38;
/// The word over the score to beat on the game's scoreboards, as its two
/// drawings. In a full match the other side's score is shown there.
pub const TARGET_LABEL: [SymbolId; 2] = [505, 517];
/// The figures the art puts on the boards a match ends on: one clip on the
/// board for a match won, and another on the one for a match lost. A full
/// match has pages of its own figures, and these are the first of them.
pub const RESULT_FIGURES: [SymbolId; 2] = [1756, 1722];
/// The field from over it, which a match is played on, the picture of the
/// stadium that is drawn under it, and the picture of the ball over it: a
/// white disc five pixels across.
pub const FIELD: SymbolId = 1548;
pub const FIELD_PICTURE: SymbolId = 1094;
pub const DOT: SymbolId = 1545;
/// An arrow, pointing right, that turns the pages on those boards.
pub const BOARD_TURN: SymbolId = 2053;

/// The fixed points of the field as the art places them, for placing a hit
/// by when there is no match on the stage to find them on.
pub fn ground(library: &Library, rules: &Rules) -> Ground {
    let placed = |name: &str| -> Option<(f32, f32)> {
        let field = library.clips.get(&FIELD)?;
        field.frames.first()?.ops.iter().find_map(|op| match op {
            Op::Place(place) if place.name.as_deref() == Some(name) => {
                let matrix = place.matrix?;
                Some((matrix[4] as f32, matrix[5] as f32))
            }
            _ => None,
        })
    };
    let mut ground = Ground {
        wall: rules.field.wall,
        feet_each: rules.zinger.wall_feet / rules.field.wall,
        ..Ground::default()
    };
    if let Some(home) = placed("startPointMarker") {
        ground.home = home;
    }
    if let Some(mark) = placed("shadowFlyMarker") {
        ground.mark_y = mark.1;
    }
    if let (Some(left), Some(right)) = (placed("foulMarkerLeft"), placed("foulMarkerRight")) {
        ground.foul = (left.0, right.0);
    }
    if let Some(second) = placed("base2") {
        ground.infield = reach(ground.home, second);
    }
    ground
}

/// The little field in the corner of the batting view, and the mark on it
/// of each fielder who roams: which fielder, counting from 0, and the depth
/// his mark is at. They are the left fielder, the shortstop, the centre
/// fielder and the right fielder.
pub const LITTLE_FIELD: SymbolId = 1092;
pub const LITTLE_FIELDERS: [(usize, u16); 4] = [(0, 6), (1, 8), (3, 10), (4, 12)];
/// Where a runner on first, second and third is marked on the little field,
/// by the stolen bases mod: just outside the corners of its diamond.
pub const LITTLE_BASES: [(f32, f32); 3] = [(76.5, 57.5), (50.25, 33.5), (24.0, 57.5)];

/// The pictures of the stadium: behind the batter, and from over the field
/// in a match and in the arcade game. The match's has the stands nearest
/// the eye as a picture of their own, drawn over the players, and the
/// arcade game's has the top of its wall as one.
pub const BACKDROPS: [SymbolId; 5] = [501, 1094, 1544, 1796, 1802];

/// The arcade game's target: rings lying on the grass, `ring1` the outermost,
/// each a clip whose second frame is the ring lit.
pub const TARGET: SymbolId = 1794;

/// Buttons whose words are drawn as artwork, not text, so they have to be
/// known by number: the ones on the result screens that lead back to the
/// menu, such as the "restart game" badge.
pub const CONTINUE_BUTTONS: [SymbolId; 4] = [493, 1725, 1767, 1847];

/// Where the shell is in the tree, once the art has reached it.
pub fn shell(stage: &Stage) -> Option<Path> {
    stage.find_symbol(&[], SHELL)
}

/// A clip that sits directly inside the shell: the screen that is showing.
pub fn in_shell(stage: &Stage, symbol: SymbolId) -> Option<Path> {
    stage.find_symbol(&shell(stage)?, symbol)
}

/// Every object with this instance name at or below the clip at `from`.
pub fn all_named(stage: &Stage, from: &[u16], name: &str) -> Vec<Path> {
    let mut found = Vec::new();
    let mut to_search = vec![from.to_vec()];
    while let Some(path) = to_search.pop() {
        let Some(clip) = stage.clip(&path) else {
            continue;
        };
        for (&depth, child) in &clip.children {
            let mut here = path.clone();
            here.push(depth);
            if child.name.as_deref() == Some(name) {
                found.push(here.clone());
            }
            if matches!(child.content, Content::Clip(_)) {
                to_search.push(here);
            }
        }
    }
    found
}

/// The words on each button, in capitals, for telling buttons apart.
pub struct ButtonLabels(HashMap<SymbolId, String>);

impl ButtonLabels {
    pub fn read(library: &Library) -> ButtonLabels {
        let labels = library
            .buttons
            .iter()
            .map(|(&id, button)| {
                let mut words = Vec::new();
                for record in &button.records {
                    if record.states.iter().any(|state| state == "up") {
                        collect_words(record.symbol, library, 0, &mut words);
                    }
                }
                (id, words.join(" ").to_uppercase())
            })
            .filter(|(_, label)| !label.is_empty())
            .collect();
        ButtonLabels(labels)
    }

    pub fn get(&self, button: SymbolId) -> Option<&str> {
        self.0.get(&button).map(String::as_str)
    }
}

/// Gathers the fixed text inside a symbol: its own, and that of anything on
/// the first frame of a clip inside it.
fn collect_words(symbol: SymbolId, library: &Library, depth: usize, words: &mut Vec<String>) {
    if depth > 5 {
        return;
    }
    match library.manifest.symbols.get(&symbol).map(|s| &s.info) {
        Some(SymbolInfo::Text) => {
            let Some(text) = library.texts.get(&symbol) else {
                return;
            };
            words.extend(
                text.runs
                    .iter()
                    .map(|run| run.text.trim().to_owned())
                    .filter(|run| !run.is_empty()),
            );
        }
        Some(SymbolInfo::Clip { .. }) => {
            let first = library
                .clips
                .get(&symbol)
                .and_then(|clip| clip.frames.first());
            for op in first.map(|frame| frame.ops.as_slice()).unwrap_or_default() {
                if let Op::Place(place) = op
                    && let PlaceAction::Place(child) = place.action
                {
                    collect_words(child, library, depth + 1, words);
                }
            }
        }
        _ => {}
    }
}
