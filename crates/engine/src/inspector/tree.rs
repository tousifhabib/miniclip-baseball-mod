//! The inspector's tree of what is on the stage, and the outline round
//! the one that is picked.

use bb_format::SymbolInfo;

use super::{Action, Info, Inspector};
use crate::display::{ButtonMode, Child, Children, Content, Path, bounds_of, child_bounds};
use crate::library::Library;

impl Inspector {
    /// Lists `children`, topmost first. Objects inside a button cannot be
    /// reached by a path, so they are listed without controls.
    pub(super) fn children(
        &mut self,
        ui: &mut egui::Ui,
        children: &Children,
        path: &mut Path,
        reachable: bool,
        info: &Info<'_>,
        actions: &mut Vec<Action>,
    ) {
        for (&depth, child) in children.iter().rev() {
            path.push(depth);
            let title = title(depth, child, info.stage.library());
            match &child.content {
                Content::Graphic => {
                    ui.horizontal(|ui| self.row(ui, &title, child, path, reachable, actions));
                }
                Content::Clip(clip) => {
                    egui::CollapsingHeader::new(format!("{title}, frame {}", clip.frame))
                        .id_salt(("clip", path.as_slice(), reachable))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                self.row(ui, "this clip", child, path, reachable, actions);
                            });
                            if reachable {
                                Inspector::clip_controls(ui, clip, path, info, actions);
                            }
                            self.children(ui, &clip.children, path, reachable, info, actions);
                        });
                }
                Content::Button(button) => {
                    let mode = match button.mode {
                        ButtonMode::Up => "up",
                        ButtonMode::Over => "over",
                        ButtonMode::Down => "down",
                    };
                    egui::CollapsingHeader::new(format!("{title}, {mode}"))
                        .id_salt(("button", path.as_slice(), reachable))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                self.row(ui, "this button", child, path, reachable, actions);
                            });
                            self.children(ui, button.shown(), path, false, info, actions);
                        });
                }
            }
            path.pop();
        }
    }

    /// One object's line: a switch to hide it, and its name, which selects
    /// it when clicked.
    fn row(
        &mut self,
        ui: &mut egui::Ui,
        title: &str,
        child: &Child,
        path: &Path,
        reachable: bool,
        actions: &mut Vec<Action>,
    ) {
        if !reachable {
            ui.label(title);
            return;
        }
        let mut visible = child.visible;
        if ui
            .checkbox(&mut visible, "")
            .on_hover_text("Shown")
            .changed()
        {
            actions.push(Action::SetVisible(path.clone(), visible));
        }
        let selected = self.selected.as_ref() == Some(path);
        if ui.selectable_label(selected, title).clicked() {
            self.selected = (!selected).then(|| path.clone());
        }
    }

    /// Draws a box on the stage around the selected object.
    pub(super) fn outline_selected(&mut self, ctx: &egui::Context, info: &Info<'_>) {
        let Some(path) = &self.selected else {
            return;
        };
        // Walk down to the object, collecting its parents' transforms.
        let mut matrix = info.base;
        let mut children = &info.stage.root.children;
        let mut found = None;
        for (index, depth) in path.iter().enumerate() {
            let Some(child) = children.get(depth) else {
                break;
            };
            if index + 1 == path.len() {
                found = Some(child);
                break;
            }
            let Content::Clip(clip) = &child.content else {
                break;
            };
            matrix = matrix.then_inner(child.matrix);
            children = &clip.children;
        }
        let Some(child) = found else {
            // The timeline has moved on and the object is gone.
            self.selected = None;
            return;
        };
        let bounds =
            child_bounds(child, matrix, info.stage.library()).or_else(|| match &child.content {
                Content::Clip(clip) => bounds_of(
                    &clip.children,
                    matrix.then_inner(child.matrix),
                    info.stage.library(),
                ),
                _ => None,
            });
        if let Some([left, top, right, bottom]) = bounds {
            let scale = ctx.pixels_per_point();
            let rect = egui::Rect::from_min_max(
                egui::pos2(left / scale, top / scale),
                egui::pos2(right / scale, bottom / scale),
            );
            ctx.debug_painter().rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 80, 200)),
                egui::StrokeKind::Outside,
            );
        }
    }
}

/// A short description of an object: its depth, what it is, and its name.
fn title(depth: u16, child: &Child, library: &Library) -> String {
    let kind = match library.manifest.symbols.get(&child.symbol).map(|s| &s.info) {
        Some(SymbolInfo::Shape { .. }) => "shape",
        Some(SymbolInfo::MorphShape) => "morph",
        Some(SymbolInfo::Clip { .. }) => "clip",
        Some(SymbolInfo::Button) => "button",
        Some(SymbolInfo::Text) => "text",
        Some(SymbolInfo::EditText) => "text field",
        _ => "object",
    };
    let name = child
        .name
        .as_ref()
        .map_or(String::new(), |name| format!(" \"{name}\""));
    let mut title = format!("{depth}: {kind} {}{name}", child.symbol);
    if child.clip_depth.is_some() {
        title.push_str(" (mask)");
    }
    if !child.filters.is_empty() {
        title.push_str(" (blurred)");
    }
    title
}
