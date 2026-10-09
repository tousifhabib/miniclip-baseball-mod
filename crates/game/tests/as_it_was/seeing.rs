//! What is taken in of a frame: what the game said of itself, what was
//! heard, and what was on the stage.

use std::cell::RefCell;
use std::rc::Rc;

use bb_engine::app::Logic;
use bb_engine::display::{ButtonMode, Child, Children, ClipState, Command, Content, Event};
use bb_engine::input::Key;
use bb_engine::library::Library;
use bb_engine::math::{ColorTransform, Matrix};
use bb_engine::stage::Stage;
use bb_game::script::Script;

use crate::sums::Sink;

/// The sounds the game has asked for and not yet been asked about, each as
/// the stage reported it: which sound, whether to start or stop it, how
/// many times over and how loud.
pub type Sounds = Rc<RefCell<Vec<String>>>;

/// The game's rules, with every sound they are told of written down on its
/// way to them. A note of a sound says only which one it was.
pub struct Listening {
    pub game: Box<dyn Logic>,
    pub sounds: Sounds,
}

impl Logic for Listening {
    fn start(&mut self, stage: &mut Stage, library: &Library) {
        self.game.start(stage, library);
    }

    fn event(&mut self, event: &Event, stage: &mut Stage, library: &Library) {
        if let Event::Sound(start) = event {
            self.sounds.borrow_mut().push(format!("{start:?}"));
        }
        self.game.event(event, stage, library);
    }

    fn tick(&mut self, stage: &mut Stage, library: &Library) {
        self.game.tick(stage, library);
    }

    fn key(&mut self, key: &Key, stage: &mut Stage, library: &Library) -> bool {
        self.game.key(key, stage, library)
    }

    fn describe(&self) -> String {
        self.game.describe()
    }
}

/// What the game says of where it is.
pub fn said(script: &Script, into: &mut impl Sink) {
    into.words("state", &script.runner.describe());
    into.next();
}

/// The notes made since this was last asked, and the sounds in full.
pub fn heard(script: &mut Script, sounds: &Sounds, into: &mut impl Sink) {
    for note in script.runner.take_notes() {
        into.words("note", &note.to_string());
        into.next();
    }
    for sound in sounds.borrow_mut().drain(..) {
        into.words("sound", &sound);
        into.next();
    }
}

/// Everything on the stage: the pointer, what is being typed in, what the
/// text fields say, every object and how it is placed and coloured, and the
/// list of what would be drawn.
pub fn seen(script: &Script, into: &mut impl Sink) {
    let stage = &script.runner.stage;
    into.float("pointer x", stage.pointer.x);
    into.float("y", stage.pointer.y);
    into.flag("down", stage.pointer.down);
    place("went down", stage.pointer.went_down, into);
    into.flag("on a button", stage.pointer.on_button());
    into.flag("hidden", stage.hide_pointer);
    into.flag("upright text", stage.upright_text);
    into.flag("space held", stage.key_down(Key::Char(' ')));
    into.next();
    into.flag("typing", stage.focus.is_some());
    if let Some(focus) = &stage.focus {
        into.number("in", u64::from(focus.symbol));
        path(&focus.path, into);
    }
    into.next();
    let mut texts: Vec<_> = stage.texts.iter().collect();
    texts.sort();
    into.number("texts", texts.len() as u64);
    into.next();
    for (variable, says) in texts {
        into.words("text", variable);
        into.words("says", says);
        into.next();
    }
    clip(&stage.root, 0, into);
    let commands = stage.commands(Matrix::IDENTITY, &script.runner.library);
    into.number("commands", commands.len() as u64);
    into.next();
    for command in &commands {
        drawn(command, into);
    }
}

/// The list of objects as the `tree` step gives it.
pub fn tree(script: &mut Script, into: &mut impl Sink) {
    for line in script.run("tree").expect("the tree") {
        into.words("tree", &line);
        into.next();
    }
}

fn place(name: &str, at: Option<(f32, f32)>, into: &mut impl Sink) {
    into.flag(name, at.is_some());
    if let Some((x, y)) = at {
        into.float("x", x);
        into.float("y", y);
    }
}

fn path(path: &[u16], into: &mut impl Sink) {
    into.number("deep", path.len() as u64);
    for &depth in path {
        into.number("at", u64::from(depth));
    }
}

fn matrix(matrix: Matrix, into: &mut impl Sink) {
    let Matrix { a, b, c, d, tx, ty } = matrix;
    for (name, float) in [
        ("a", a),
        ("b", b),
        ("c", c),
        ("d", d),
        ("tx", tx),
        ("ty", ty),
    ] {
        into.float(name, float);
    }
}

fn colour(colour: ColorTransform, into: &mut impl Sink) {
    for float in colour.mult {
        into.float("times", float);
    }
    for float in colour.add {
        into.float("plus", float);
    }
}

fn clip(clip: &ClipState, deep: u64, into: &mut impl Sink) {
    into.number("deep", deep);
    into.flag("a symbol", clip.symbol.is_some());
    into.number("clip", u64::from(clip.symbol.unwrap_or(0)));
    into.number("frame", u64::from(clip.frame));
    into.flag("playing", clip.playing);
    children(&clip.children, deep + 1, into);
}

fn children(all: &Children, deep: u64, into: &mut impl Sink) {
    into.number("children", all.len() as u64);
    into.next();
    for (&depth, child) in all {
        object(depth, child, deep, into);
    }
}

fn object(depth: u16, child: &Child, deep: u64, into: &mut impl Sink) {
    into.number("deep", deep);
    into.number("depth", u64::from(depth));
    into.number("symbol", u64::from(child.symbol));
    matrix(child.matrix, into);
    colour(child.color, into);
    into.number("ratio", u64::from(child.ratio));
    into.flag("a mask", child.clip_depth.is_some());
    into.number("up to", u64::from(child.clip_depth.unwrap_or(0)));
    into.flag("named", child.name.is_some());
    into.words("name", child.name.as_deref().unwrap_or(""));
    into.flag("visible", child.visible);
    into.number("filters", child.filters.len() as u64);
    for filter in &child.filters {
        into.words("filter", &format!("{filter:?}"));
    }
    into.number("placed on", u64::from(child.placed_on));
    into.flag("matrix held", child.held.matrix);
    into.flag("colour held", child.held.color);
    into.flag("showing held", child.held.visible);
    into.flag("attached", child.attached);
    into.flag("says", child.said.is_some());
    into.words("said", child.said.as_deref().unwrap_or(""));
    match &child.content {
        Content::Graphic => {
            into.number("kind", 0);
            into.next();
        }
        Content::Clip(inside) => {
            into.number("kind", 1);
            clip(inside, deep, into);
        }
        Content::Button(button) => {
            into.number("kind", 2);
            into.number(
                "look",
                match button.mode {
                    ButtonMode::Up => 0,
                    ButtonMode::Over => 1,
                    ButtonMode::Down => 2,
                },
            );
            for look in [&button.up, &button.over, &button.down, &button.hit] {
                children(look, deep + 1, into);
            }
        }
    }
}

fn drawn(command: &Command, into: &mut impl Sink) {
    match command {
        Command::Draw {
            symbol,
            ratio,
            matrix: placed,
            color,
            text,
        } => {
            into.number("draw", 0);
            into.number("symbol", u64::from(*symbol));
            into.number("ratio", u64::from(*ratio));
            matrix(*placed, into);
            colour(*color, into);
            into.flag("says", text.is_some());
            into.words("text", text.as_deref().unwrap_or(""));
        }
        Command::PushMask => into.number("mask begins", 1),
        Command::ActivateMask => into.number("mask on", 2),
        Command::DeactivateMask => into.number("mask off", 3),
        Command::PopMask => into.number("mask ends", 4),
        Command::BeginBlur {
            blur_x,
            blur_y,
            passes,
            bounds,
        } => {
            into.number("blur begins", 5);
            into.float("blur x", *blur_x);
            into.float("blur y", *blur_y);
            into.number("passes", u64::from(*passes));
            for &edge in bounds {
                into.float("edge", edge);
            }
        }
        Command::EndBlur => into.number("blur ends", 6),
    }
    into.next();
}
