//! The triangles of everything that has been drawn or pointed at, worked out
//! the first time each is wanted and kept.
//!
//! Nothing here needs a graphics card. The renderer keeps one of these and
//! hands its triangles to the card, and a game played with no window can
//! keep one on its own: telling what is under the pointer takes only the
//! triangles.

use std::collections::HashMap;

use bb_format::{SymbolId, SymbolInfo};

use crate::input::Geometry;
use crate::library::Library;
use crate::tess::{Draw, Image, Mesh, Paint, Ramp, Tessellator, Vertex};

/// Which set of triangles: there is one for each thing that is drawn.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum MeshKey {
    Shape(SymbolId),
    Morph(SymbolId, u16),
    Text(SymbolId),
    /// A text field saying something the game has set. The number stands
    /// for what it says.
    Field(SymbolId, u64),
    /// The square from (0, 0) to (1, 1), for drawing a layer.
    Quad,
}

impl MeshKey {
    /// Which mesh draws `symbol`, if it is something with a mesh. `text` is
    /// what a text field has been set to say.
    pub(crate) fn of(
        library: &Library,
        symbol: SymbolId,
        ratio: u16,
        text: Option<&str>,
    ) -> Option<MeshKey> {
        match (&library.manifest.symbols.get(&symbol)?.info, text) {
            (SymbolInfo::Shape { .. }, _) => Some(MeshKey::Shape(symbol)),
            (SymbolInfo::MorphShape, _) => Some(MeshKey::Morph(symbol, ratio)),
            (SymbolInfo::EditText, Some(text)) => {
                let mut hasher = std::hash::DefaultHasher::new();
                std::hash::Hash::hash(text, &mut hasher);
                Some(MeshKey::Field(symbol, std::hash::Hasher::finish(&hasher)))
            }
            (SymbolInfo::Text | SymbolInfo::EditText, None) => Some(MeshKey::Text(symbol)),
            _ => None,
        }
    }
}

/// Every mesh made so far, and the gradients and images their paints use.
#[derive(Default)]
pub struct Meshes {
    tessellator: Tessellator,
    /// `None` for a thing that came to no triangles, or could not be cut
    /// into them, so that it is not tried again.
    built: HashMap<MeshKey, Option<Mesh>>,
    problems: Vec<String>,
}

impl Meshes {
    /// Symbols that could not be cut into triangles, each reported once.
    pub fn problems(&self) -> &[String] {
        &self.problems
    }

    /// How many meshes have been asked for, the empty ones among them.
    pub(crate) fn len(&self) -> usize {
        self.built.len()
    }

    /// The gradients the paints refer to by number.
    pub(crate) fn ramps(&self) -> &[Ramp] {
        &self.tessellator.ramps
    }

    /// The images the paints refer to by number.
    pub(crate) fn images(&self) -> &[Image] {
        &self.tessellator.images
    }

    /// The mesh for `key`, if it has been built and came to anything.
    pub(crate) fn get(&self, key: MeshKey) -> Option<&Mesh> {
        self.built.get(&key)?.as_ref()
    }

    /// Builds the mesh for `key` if it is not there yet, and returns it if
    /// it came to anything. `text` is what a [`MeshKey::Field`] says.
    pub(crate) fn ensure(
        &mut self,
        library: &Library,
        key: MeshKey,
        text: Option<&str>,
    ) -> Option<&Mesh> {
        if !self.built.contains_key(&key) {
            let mesh = match self.build(library, key, text) {
                Ok(mesh) if !mesh.indices.is_empty() => Some(mesh),
                Ok(_) => None,
                Err(error) => {
                    self.problems.push(format!("{error:#}"));
                    None
                }
            };
            self.built.insert(key, mesh);
        }
        self.get(key)
    }

    fn build(
        &mut self,
        library: &Library,
        key: MeshKey,
        text: Option<&str>,
    ) -> anyhow::Result<Mesh> {
        match key {
            MeshKey::Shape(id) => {
                let symbol = &library.manifest.symbols[&id];
                let origin = match symbol.info {
                    SymbolInfo::Shape { bounds } => (bounds.x_min as f32, bounds.y_min as f32),
                    _ => (0.0, 0.0),
                };
                self.tessellator
                    .svg(&library.dir.join(&symbol.file), origin)
            }
            MeshKey::Morph(id, ratio) => self.tessellator.morph(&library.morphs[&id], ratio),
            MeshKey::Text(id) => match (library.texts.get(&id), library.edit_texts.get(&id)) {
                (Some(text), _) => self.tessellator.text(text, library),
                (None, Some(field)) => self.tessellator.edit_text(field, None, library),
                (None, None) => Ok(Mesh::default()),
            },
            MeshKey::Field(id, _) => match library.edit_texts.get(&id) {
                Some(field) => self.tessellator.edit_text(field, text, library),
                None => Ok(Mesh::default()),
            },
            MeshKey::Quad => {
                let corner = |x: f32, y: f32| Vertex {
                    position: [x, y],
                    normal: [0.0, 0.0],
                    half_width: 0.0,
                    color: [255; 4],
                };
                Ok(Mesh {
                    vertices: vec![
                        corner(0.0, 0.0),
                        corner(1.0, 0.0),
                        corner(1.0, 1.0),
                        corner(0.0, 1.0),
                    ],
                    indices: vec![0, 1, 2, 0, 2, 3],
                    draws: vec![Draw {
                        indices: 0..6,
                        paint: Paint::Solid,
                    }],
                })
            }
        }
    }

    /// Text that changes often, such as a score, leaves a mesh behind for
    /// every value it has shown. Once there are more than `most` of them
    /// they are all cleared out, and the ones still wanted are built again
    /// as they are drawn. Returns whether that was done.
    pub(crate) fn forget_fields_over(&mut self, most: usize) -> bool {
        let is_field = |key: &MeshKey| matches!(key, MeshKey::Field(..));
        let too_many = self.built.keys().filter(|key| is_field(key)).count() > most;
        if too_many {
            self.built.retain(|key, _| !is_field(key));
        }
        too_many
    }
}

impl Geometry for Meshes {
    fn contains(
        &mut self,
        library: &Library,
        symbol: SymbolId,
        ratio: u16,
        x: f32,
        y: f32,
    ) -> bool {
        MeshKey::of(library, symbol, ratio, None)
            .and_then(|key| self.ensure(library, key, None))
            .is_some_and(|mesh| mesh.contains(x, y))
    }
}
