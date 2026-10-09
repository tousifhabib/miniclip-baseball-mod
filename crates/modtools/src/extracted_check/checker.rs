//! The checks themselves: every symbol's file read, and everything it
//! says of another followed.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use bb_engine::library::read_json;
use bb_format::{
    Align, Button, Clip, EditText, FieldFlag, Font, Look, Manifest, MorphShape, Op, PlaceAction,
    SoundStart, Symbol, SymbolId, SymbolInfo, Text,
};
use serde::de::DeserializeOwned;

pub(super) struct Checker {
    pub(super) dir: PathBuf,
    pub(super) manifest: Manifest,
    pub(super) glyph_counts: BTreeMap<SymbolId, usize>,
    pub(super) files: usize,
    pub(super) references: usize,
    pub(super) problems: Vec<String>,
}

impl Checker {
    pub(super) fn run(&mut self) -> Result<()> {
        let symbols = self.manifest.symbols.clone();

        // Fonts first, because texts are checked against their glyph counts.
        for (id, symbol) in &symbols {
            if let SymbolInfo::Font { .. } = symbol.info {
                let font: Font = self.read(&symbol.file)?;
                self.glyph_counts.insert(*id, font.glyphs.len());
            }
        }
        for (id, symbol) in &symbols {
            self.check_symbol(*id, symbol)?;
        }
        self.check_main_timeline()?;
        self.check_exports();
        Ok(())
    }

    /// Checks one symbol by the kind the manifest says it is.
    fn check_symbol(&mut self, id: SymbolId, symbol: &Symbol) -> Result<()> {
        match &symbol.info {
            SymbolInfo::Font { .. } => {}
            SymbolInfo::Shape { .. } | SymbolInfo::Bitmap { .. } | SymbolInfo::Sound { .. } => {
                self.check_file_is_there(&symbol.file);
            }
            SymbolInfo::MorphShape => self.check_morph(id, &symbol.file)?,
            SymbolInfo::Clip { frame_count } => {
                self.check_clip_symbol(id, &symbol.file, *frame_count)?;
            }
            SymbolInfo::Button => self.check_button(id, &symbol.file)?,
            SymbolInfo::Text => self.check_text(id, &symbol.file)?,
            SymbolInfo::EditText => self.check_field(id, &symbol.file)?,
        }
        Ok(())
    }

    /// A shape, a bitmap or a sound is not read here: its file has only to
    /// be there, with something in it.
    fn check_file_is_there(&mut self, file: &str) {
        self.files += 1;
        let path = self.dir.join(file);
        let size = fs::metadata(&path).map_or(0, |meta| meta.len());
        if size == 0 {
            self.problem(format!("{file} is missing or empty"));
        }
    }

    /// Each path of a morph shape must be made of the same commands at its
    /// two ends, or there is no blending between them.
    fn check_morph(&mut self, id: SymbolId, file: &str) -> Result<()> {
        let morph: MorphShape = self.read(file)?;
        for (index, path) in morph.paths.iter().enumerate() {
            if commands(&path.start) != commands(&path.end) {
                self.problem(format!(
                    "morph {id}: path {index} has different commands at its two ends"
                ));
            }
        }
        Ok(())
    }

    /// A clip must have as many frames as the manifest says, and its
    /// timeline must hold together.
    fn check_clip_symbol(&mut self, id: SymbolId, file: &str, frame_count: u16) -> Result<()> {
        let clip: Clip = self.read(file)?;
        if clip.frames.len() != usize::from(frame_count) {
            self.problem(format!(
                "clip {id}: the manifest says {frame_count} frames, the file has {}",
                clip.frames.len()
            ));
        }
        self.check_clip(&format!("clip {id}"), &clip);
        Ok(())
    }

    /// A button's looks must each be one a button has, made of things that
    /// can be shown, and its sounds must be sounds.
    fn check_button(&mut self, id: SymbolId, file: &str) -> Result<()> {
        let button: Button = self.read(file)?;
        for record in &button.records {
            self.check_drawable(&format!("button {id}"), record.symbol);
            for look in &record.states {
                if let Look::Other(word) = look {
                    self.problem(format!("button {id}: `{word}` is not a look a button has"));
                }
            }
        }
        if let Some(sounds) = &button.sounds {
            let starts = [
                &sounds.over_to_up,
                &sounds.up_to_over,
                &sounds.over_to_down,
                &sounds.down_to_over,
            ];
            for start in starts.into_iter().flatten() {
                self.check_sound(&format!("button {id}"), start);
            }
        }
        Ok(())
    }

    /// Each run of a text must be in a font, and of glyphs the font has.
    fn check_text(&mut self, id: SymbolId, file: &str) -> Result<()> {
        let text: Text = self.read(file)?;
        for run in &text.runs {
            self.references += 1;
            let Some(&glyphs) = self.glyph_counts.get(&run.font) else {
                self.problem(format!("text {id}: font {} is not a font", run.font));
                continue;
            };
            if run.glyphs.iter().any(|g| g.glyph as usize >= glyphs) {
                self.problem(format!(
                    "text {id}: uses a glyph font {} does not have",
                    run.font
                ));
            }
        }
        Ok(())
    }

    /// A text field's font must be a font, and what it is said to be, and
    /// the side its lines are set against, must be words the format has.
    fn check_field(&mut self, id: SymbolId, file: &str) -> Result<()> {
        let text: EditText = self.read(file)?;
        if let Some(font) = text.font {
            self.references += 1;
            if !self.glyph_counts.contains_key(&font) {
                self.problem(format!("text field {id}: font {font} is not a font"));
            }
        }
        for flag in &text.flags {
            if let FieldFlag::Other(word) = flag {
                self.problem(format!(
                    "text field {id}: `{word}` is not something a text field can be"
                ));
            }
        }
        if let Some(Align::Other(word)) = text.layout.as_ref().map(|layout| &layout.align) {
            self.problem(format!(
                "text field {id}: `{word}` is not a side to set lines against"
            ));
        }
        Ok(())
    }

    /// The main timeline must be as long as the manifest says, and hold
    /// together as any clip must.
    fn check_main_timeline(&mut self) -> Result<()> {
        let root: Clip = self.read("clips/root.json")?;
        if root.frames.len() != usize::from(self.manifest.stage.frame_count) {
            self.problem("the main timeline's length differs from the manifest".to_owned());
        }
        self.check_clip("the main timeline", &root);
        Ok(())
    }

    /// Each name the art exports must lead to a symbol that goes by it.
    fn check_exports(&mut self) {
        for (name, id) in &self.manifest.exports.clone() {
            self.references += 1;
            match self.manifest.symbols.get(id) {
                Some(symbol) if symbol.export_name.as_deref() == Some(name) => {}
                Some(_) => self.problem(format!("export {name:?}: symbol {id} is not named so")),
                None => self.problem(format!("export {name:?}: no symbol {id}")),
            }
        }
    }

    fn check_clip(&mut self, name: &str, clip: &Clip) {
        for (label, &frame) in &clip.labels {
            if frame == 0 || usize::from(frame) > clip.frames.len() {
                self.problem(format!("{name}: label {label:?} points at frame {frame}"));
            }
        }
        for frame in &clip.frames {
            for op in &frame.ops {
                if let Op::Place(place) = op
                    && let PlaceAction::Place(symbol) | PlaceAction::Replace(symbol) = place.action
                {
                    self.check_drawable(name, symbol);
                }
            }
            for start in &frame.sounds {
                self.check_sound(name, start);
            }
        }
    }

    /// Checks that `symbol` exists and is something a timeline can show.
    fn check_drawable(&mut self, name: &str, symbol: SymbolId) {
        self.references += 1;
        match self
            .manifest
            .symbols
            .get(&symbol)
            .map(|symbol| &symbol.info)
        {
            None => self.problem(format!(
                "{name}: places symbol {symbol}, which does not exist"
            )),
            Some(
                SymbolInfo::Sound { .. } | SymbolInfo::Font { .. } | SymbolInfo::Bitmap { .. },
            ) => self.problem(format!(
                "{name}: places symbol {symbol}, which cannot be shown"
            )),
            Some(_) => {}
        }
    }

    fn check_sound(&mut self, name: &str, start: &SoundStart) {
        self.references += 1;
        let info = self.manifest.symbols.get(&start.sound).map(|s| &s.info);
        if !matches!(info, Some(SymbolInfo::Sound { .. })) {
            self.problem(format!(
                "{name}: plays symbol {}, which is not a sound",
                start.sound
            ));
        }
    }

    fn read<T: DeserializeOwned>(&mut self, file: &str) -> Result<T> {
        self.files += 1;
        read_json(&self.dir.join(file))
    }

    fn problem(&mut self, message: String) {
        self.problems.push(message);
    }
}

/// The command letters of some SVG path data, without their numbers.
fn commands(path: &str) -> String {
    path.chars().filter(char::is_ascii_alphabetic).collect()
}
