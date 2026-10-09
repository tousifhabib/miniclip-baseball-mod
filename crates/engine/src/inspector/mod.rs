//! A panel for looking inside the running scene: the tree of objects, what
//! each timeline is doing, and what has just happened.

mod tree;

use std::collections::VecDeque;

use crate::display::{ClipState, Path};
use crate::gpu::Stats;
use crate::library::Library;
use crate::math::Matrix;
use crate::stage::Stage;

/// How many recent events the panel keeps.
const LOG_LENGTH: usize = 14;

/// Something the user asked for in the panel, to carry out after it is drawn.
pub enum Action {
    TogglePause,
    Step,
    SetPlaying(Path, bool),
    SetVisible(Path, bool),
    Goto(Path, u16),
}

pub struct Inspector {
    pub open: bool,
    /// The object outlined on the stage.
    selected: Option<Path>,
    log: VecDeque<String>,
}

/// What the panel shows this frame.
pub struct Info<'a> {
    pub stage: &'a Stage,
    pub library: &'a Library,
    pub stats: Stats,
    pub paused: bool,
    pub frames_per_second: f32,
    /// How many redraws go to each frame of the game, if the screen is in
    /// step with it.
    pub redraws_to_a_frame: Option<u32>,
    /// From stage coordinates to window pixels.
    pub base: Matrix,
}

impl Default for Inspector {
    fn default() -> Inspector {
        Inspector::new()
    }
}

impl Inspector {
    pub fn new() -> Inspector {
        Inspector {
            open: false,
            selected: None,
            log: VecDeque::new(),
        }
    }

    /// Adds a line to the list of recent events.
    pub fn note(&mut self, line: String) {
        if self.log.len() == LOG_LENGTH {
            self.log.pop_front();
        }
        self.log.push_back(line);
    }

    pub fn ui(&mut self, ctx: &egui::Context, info: &Info<'_>) -> Vec<Action> {
        let mut actions = Vec::new();
        if !self.open {
            return actions;
        }
        self.outline_selected(ctx, info);

        egui::Window::new("Inspector")
            .default_pos([8.0, 8.0])
            .default_width(300.0)
            .default_height(360.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let label = if info.paused { "Play" } else { "Pause" };
                    if ui.button(label).clicked() {
                        actions.push(Action::TogglePause);
                    }
                    if ui
                        .add_enabled(info.paused, egui::Button::new("Step"))
                        .clicked()
                    {
                        actions.push(Action::Step);
                    }
                    ui.label(match info.redraws_to_a_frame {
                        Some(1) => format!("{:.0} fps, a frame each", info.frames_per_second),
                        Some(redraws) => {
                            format!("{:.0} fps, a frame every {redraws}", info.frames_per_second)
                        }
                        None => format!("{:.0} fps, frames by the clock", info.frames_per_second),
                    });
                });
                ui.label(format!(
                    "{} draws, {} blurred layers, {} meshes built",
                    info.stats.draws, info.stats.layers, info.stats.meshes
                ));
                let pointer = &info.stage.pointer;
                ui.label(format!(
                    "pointer at ({:.0}, {:.0}){}",
                    pointer.x,
                    pointer.y,
                    if pointer.on_button() {
                        ", on a button"
                    } else {
                        ""
                    }
                ));
                ui.separator();

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::CollapsingHeader::new("Objects")
                            .default_open(true)
                            .show(ui, |ui| {
                                let root = &info.stage.root;
                                Inspector::clip_controls(
                                    ui,
                                    root,
                                    &Path::new(),
                                    info,
                                    &mut actions,
                                );
                                let mut path = Path::new();
                                self.children(
                                    ui,
                                    &root.children,
                                    &mut path,
                                    true,
                                    info,
                                    &mut actions,
                                );
                            });
                        egui::CollapsingHeader::new("Recent events").show(ui, |ui| {
                            if self.log.is_empty() {
                                ui.weak("Nothing yet.");
                            }
                            for line in &self.log {
                                ui.label(line);
                            }
                        });
                    });
            });
        actions
    }

    /// The play switch and frame slider of one clip.
    fn clip_controls(
        ui: &mut egui::Ui,
        clip: &ClipState,
        path: &Path,
        info: &Info<'_>,
        actions: &mut Vec<Action>,
    ) {
        let count = clip.frame_count(info.library);
        ui.horizontal(|ui| {
            let mut playing = clip.playing;
            if ui.checkbox(&mut playing, "playing").changed() {
                actions.push(Action::SetPlaying(path.clone(), playing));
            }
            if count > 1 {
                let mut frame = clip.frame;
                let slider = egui::Slider::new(&mut frame, 1..=count).text(format!("of {count}"));
                if ui.add(slider).changed() {
                    actions.push(Action::Goto(path.clone(), frame));
                }
            } else {
                ui.weak("one frame");
            }
        });
    }
}
