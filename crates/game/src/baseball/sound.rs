//! What is heard on each screen: the music of the menu, and the crowd
//! under a game.

use bb_engine::stage::Stage;

use super::Baseball;
use super::screen::Screen;

impl Baseball {
    /// Starts and stops the music and the crowd for the screen being shown.
    /// The music belongs to the menu and the screens a game ends on. The
    /// crowd is heard under a game.
    pub(super) fn sound_for(&mut self, screen: Screen, stage: &mut Stage) {
        let sound = &self.game.rules.sound;
        for (name, level) in &sound.levels {
            stage.set_sound_level(name, *level);
        }
        let in_game = screen.is_game();
        let wants_music = screen == Screen::Menu;
        let stops_music = in_game || screen == Screen::Instructions;
        if wants_music && !self.music_on {
            self.music_on = stage.play_sound(&sound.music, 999);
        } else if stops_music && self.music_on {
            stage.stop_sound(&sound.music);
            self.music_on = false;
        }
        if in_game && !self.crowd_on {
            self.crowd_on = stage.play_sound(&sound.crowd, 999);
        } else if screen == Screen::Menu && self.crowd_on {
            stage.stop_sound(&sound.crowd);
            self.crowd_on = false;
        }
    }
}
