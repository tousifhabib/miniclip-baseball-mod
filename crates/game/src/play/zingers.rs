//! The match's zingers: the longest, the record there is to beat, and
//! what is told when one comes down.

use bb_engine::library::Library;
use bb_engine::stage::Stage;

use super::Match;
use crate::play::mods::zinger_hit as zinger;

impl Match {
    /// The longest zinger of this game, in feet. Nought if there was none.
    pub fn longest_zinger(&self) -> u32 {
        self.mods.longest_zinger()
    }

    /// The longest zinger there has ever been, as far as this game knows.
    pub fn zinger_record(&self) -> u32 {
        self.mods.zinger_record()
    }

    /// Tells the game the record its zingers have to beat.
    pub fn set_zinger_record(&mut self, feet: u32) {
        self.mods.set_zinger_record(feet);
    }

    /// A zinger has come down: the player is told how far it went and
    /// where, and the crowd is heard.
    pub(crate) fn zinger_down(
        &mut self,
        show: &mut zinger::Show,
        stage: &mut Stage,
        library: &Library,
    ) {
        let record = self.mods.a_zinger_went(show.zinger.feet);
        show.landed(record, stage);
        self.mods.a_home_run_was_hit();
        let mut sounds = show.place.cheers().to_vec();
        if record && !sounds.contains(&"baseball_organ_FX") {
            sounds.push("baseball_organ_FX");
        }
        for name in sounds {
            Match::sound(stage, library, name);
        }
    }
}
