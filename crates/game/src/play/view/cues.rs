//! Sounds, and the things that have to be done to a clip when it reaches
//! a frame, where the art's own script did them.

use bb_engine::display::Path;
use bb_engine::library::Library;
use bb_engine::stage::Stage;

use crate::play::Match;

/// Something to do to a clip when it reaches a frame, where the art has
/// nothing to do it.
#[derive(Clone, Debug)]
pub(crate) struct Cue {
    pub path: Path,
    pub frame: u16,
    /// Go back to the first frame and wait there. Otherwise just stop.
    pub rewind: bool,
}

impl Match {
    pub(crate) fn sound(stage: &mut Stage, library: &Library, name: &str) {
        stage.play_sound(name, 1, library);
    }

    /// Sets a clip playing from a label, to be sent back to its first frame
    /// when it reaches `end`, where the art's own script did that.
    pub(crate) fn play_section(
        &mut self,
        path: &[u16],
        label: &str,
        end: u16,
        stage: &mut Stage,
        library: &Library,
    ) {
        self.cues.retain(|cue| cue.path != path);
        if stage.goto_label(path, label, true, library) {
            self.cues.push(Cue {
                path: path.to_vec(),
                frame: end,
                rewind: true,
            });
        }
    }

    pub(crate) fn run_cues(&mut self, stage: &mut Stage, library: &Library) {
        self.put_away.retain_mut(|(path, left)| {
            if *left > 0 {
                *left -= 1;
                return true;
            }
            stage.goto_clip(path, 1, library);
            if let Some(clip) = stage.clip_mut(path) {
                clip.playing = false;
            }
            false
        });
        let mut due = Vec::new();
        self.cues.retain(|cue| match stage.clip(&cue.path) {
            Some(clip) if clip.frame >= cue.frame => {
                due.push(cue.clone());
                false
            }
            Some(_) => true,
            None => false,
        });
        for cue in due {
            if cue.rewind {
                stage.goto_clip(&cue.path, 1, library);
            }
            if let Some(clip) = stage.clip_mut(&cue.path) {
                clip.playing = false;
            }
        }
    }
}
