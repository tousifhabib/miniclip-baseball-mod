//! The words on the art's buttons, read from the text inside each.

use std::collections::HashMap;

use bb_engine::library::Library;
use bb_format::{Look, Op, PlaceAction, SymbolId, SymbolInfo};

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
                    if record.states.contains(&Look::Up) {
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
