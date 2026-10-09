//! The sounds the game's rules ask for by name, and how loud each is to be.

use bb_format::{EnvelopePoint, SoundEvent, SoundStart};

use super::Stage;
use crate::display::Event;
use crate::library::Library;

impl Stage {
    /// Asks for a sound by its export name, as `attachSound` used: played
    /// `loops` times. Returns whether there is a sound with that name.
    pub fn play_sound(&mut self, name: &str, loops: u16, library: &Library) -> bool {
        self.sound(name, SoundEvent::Event, loops, library)
    }

    /// Sets how loud the sound with this export name is whenever the game
    /// asks for it, from 0 to 1, as `setVolume` did. Returns whether there is
    /// a sound with that name.
    pub fn set_sound_level(&mut self, name: &str, level: f32, library: &Library) -> bool {
        let Some(&sound) = library.manifest.exports.get(name) else {
            return false;
        };
        self.levels.insert(sound, level.clamp(0.0, 1.0));
        true
    }

    /// Stops every copy of the sound with this export name.
    pub fn stop_sound(&mut self, name: &str, library: &Library) -> bool {
        self.sound(name, SoundEvent::Stop, 0, library)
    }

    fn sound(&mut self, name: &str, event: SoundEvent, loops: u16, library: &Library) -> bool {
        let Some(&sound) = library.manifest.exports.get(name) else {
            return false;
        };
        // A sound the game has turned down starts at that level.
        let envelope = self
            .levels
            .get(&sound)
            .map(|&level| EnvelopePoint {
                sample: 0,
                left: level,
                right: level,
            })
            .into_iter()
            .collect();
        self.push_event(Event::Sound(SoundStart {
            sound,
            event,
            loops,
            in_sample: None,
            out_sample: None,
            envelope,
        }));
        true
    }
}

#[cfg(test)]
mod tests {
    use bb_format::SymbolId;

    use super::*;
    use crate::testing::{frame, library_with};

    /// The sound the art exports as "crowd".
    const CROWD: SymbolId = 40;

    fn with_a_sound() -> Library {
        let mut library = library_with(vec![frame(vec![])], vec![frame(vec![])]);
        library.manifest.exports.insert("crowd".to_owned(), CROWD);
        library
    }

    /// What asking for the crowd puts in the stage's events.
    fn asked(event: SoundEvent, loops: u16, envelope: Vec<EnvelopePoint>) -> Event {
        Event::Sound(SoundStart {
            sound: CROWD,
            event,
            loops,
            in_sample: None,
            out_sample: None,
            envelope,
        })
    }

    #[test]
    fn the_rules_ask_for_a_sound_and_stop_it_by_the_name_it_is_exported_under() {
        let library = with_a_sound();
        let mut stage = Stage::new(None, &library);
        assert!(stage.play_sound("crowd", 3, &library));
        assert!(stage.stop_sound("crowd", &library));
        let events = [
            asked(SoundEvent::Event, 3, Vec::new()),
            asked(SoundEvent::Stop, 0, Vec::new()),
        ];
        assert_eq!(stage.take_events(), events);

        // There is no sound of this name, so nothing is asked for.
        assert!(!stage.play_sound("organ", 1, &library));
        assert!(!stage.stop_sound("organ", &library));
        assert!(!stage.set_sound_level("organ", 0.5, &library));
        assert!(stage.take_events().is_empty());
    }

    #[test]
    fn a_sound_the_rules_have_turned_down_starts_at_that_level() {
        let library = with_a_sound();
        let mut stage = Stage::new(None, &library);
        let at = |level: f32| {
            vec![EnvelopePoint {
                sample: 0,
                left: level,
                right: level,
            }]
        };
        assert!(stage.set_sound_level("crowd", 0.25, &library));
        // Setting the level asks for nothing by itself.
        assert!(stage.take_events().is_empty());
        stage.play_sound("crowd", 1, &library);
        assert_eq!(stage.take_events(), [asked(SoundEvent::Event, 1, at(0.25))]);

        // A level is kept between nothing and full.
        stage.set_sound_level("crowd", 7.0, &library);
        stage.play_sound("crowd", 1, &library);
        stage.set_sound_level("crowd", -1.0, &library);
        stage.play_sound("crowd", 1, &library);
        let events = [
            asked(SoundEvent::Event, 1, at(1.0)),
            asked(SoundEvent::Event, 1, at(0.0)),
        ];
        assert_eq!(stage.take_events(), events);
    }
}
