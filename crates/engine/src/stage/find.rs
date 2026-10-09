//! Finding things on the stage: by the path to them, by name, or by the
//! symbol they are made from.

use bb_format::SymbolId;

use super::Stage;
use crate::display::{Child, ClipState, Content, Path};
use crate::math::Matrix;

impl Stage {
    /// The object at `path`.
    pub fn child(&self, path: &[u16]) -> Option<&Child> {
        let (last, parents) = path.split_last()?;
        let mut clip = &self.root;
        for depth in parents {
            match &clip.children.get(depth)?.content {
                Content::Clip(inner) => clip = inner,
                _ => return None,
            }
        }
        clip.children.get(last)
    }

    /// The object at `path`, to move, tint, show or hide.
    pub fn child_mut(&mut self, path: &[u16]) -> Option<&mut Child> {
        self.root.child_mut(path)
    }

    /// The transform from the coordinates inside the object at `path` to the
    /// stage's, or the identity for an empty path.
    pub fn to_stage(&self, path: &[u16]) -> Option<Matrix> {
        let mut matrix = Matrix::IDENTITY;
        for end in 1..=path.len() {
            matrix = matrix.then_inner(self.child(&path[..end])?.matrix);
        }
        Some(matrix)
    }

    /// Where a point of the stage is in the coordinates inside the object at
    /// `path`: what `_xmouse` and `_ymouse` gave for the pointer.
    pub fn from_stage(&self, path: &[u16], x: f32, y: f32) -> Option<(f32, f32)> {
        Some(self.to_stage(path)?.inverse()?.apply(x, y))
    }

    /// The clip at `path`, or the top timeline for an empty path.
    pub fn clip(&self, path: &[u16]) -> Option<&ClipState> {
        if path.is_empty() {
            return Some(&self.root);
        }
        match &self.child(path)?.content {
            Content::Clip(clip) => Some(clip),
            _ => None,
        }
    }

    /// The clip at `path`, or the top timeline for an empty path.
    pub fn clip_mut(&mut self, path: &[u16]) -> Option<&mut ClipState> {
        if path.is_empty() {
            return Some(&mut self.root);
        }
        match &mut self.root.child_mut(path)?.content {
            Content::Clip(clip) => Some(clip),
            _ => None,
        }
    }

    /// Finds an object by the instance names leading to it, as an
    /// ActionScript path like `game.hitter` would: `["game", "hitter"]`.
    /// The search starts inside the clip at `from`.
    pub fn find(&self, from: &[u16], names: &[&str]) -> Option<Path> {
        let mut path = from.to_vec();
        for name in names {
            let clip = self.clip(&path)?;
            let (&depth, _) = clip
                .children
                .iter()
                .find(|(_, child)| child.name.as_deref() == Some(*name))?;
            path.push(depth);
        }
        Some(path)
    }

    /// Finds the first instance of `symbol` at or below the clip at `from`,
    /// looking through each level before going deeper.
    pub fn find_symbol(&self, from: &[u16], symbol: SymbolId) -> Option<Path> {
        self.search(from, &|child| child.symbol == symbol)
    }

    /// Finds the first object with this instance name at or below the clip
    /// at `from`, looking through each level before going deeper.
    pub fn find_named(&self, from: &[u16], name: &str) -> Option<Path> {
        self.search(from, &|child| child.name.as_deref() == Some(name))
    }

    fn search(&self, from: &[u16], wanted: &dyn Fn(&Child) -> bool) -> Option<Path> {
        let mut level = vec![from.to_vec()];
        while !level.is_empty() {
            let mut next = Vec::new();
            for path in &level {
                let Some(clip) = self.clip(path) else {
                    continue;
                };
                for (&depth, child) in &clip.children {
                    let mut here = path.clone();
                    here.push(depth);
                    if wanted(child) {
                        return Some(here);
                    }
                    if matches!(child.content, Content::Clip(_)) {
                        next.push(here);
                    }
                }
            }
            level = next;
        }
        None
    }
}
