//! What a play in the field tells the mods that have news to give: a
//! sign struck, a base stolen.

use bb_engine::library::Library;
use bb_engine::stage::Stage;

use crate::play::book::ORDER;
use crate::play::{AtBat, Match, sign, steal};
use crate::rules::SignRules;

impl Match {
    /// A runner who was stealing has got to `base`, or has been put out on
    /// his way there: it is counted, written in a full match's book, and
    /// kept to be told.
    pub(super) fn a_steal_came_out(&mut self, runner: usize, base: u8, safe: bool) {
        self.mods.a_steal_came_out(safe);
        let order = self.runners[runner].order % ORDER;
        if let Some(full) = self.mode.full_mut() {
            let innings = full.innings();
            full.book.ours.stole(innings, order, base, safe);
        }
    }

    /// The ball is at the wall, `across` the field and this high. With the
    /// hit the sign mod on, if it has struck a sign the runs that is worth
    /// are the batter's.
    pub(in crate::play) fn strike_sign(
        &mut self,
        at_bat: &mut AtBat,
        across: f32,
        height: f32,
        rules: &SignRules,
    ) {
        let struck = at_bat
            .signs
            .as_mut()
            .and_then(|board| board.strike(across, height, rules));
        let Some((sign, runs)) = struck else {
            return;
        };
        self.score += runs;
        // The batter is the last to have come up.
        if let Some(batter) = self.runners.last_mut() {
            batter.runs += runs;
        }
        self.mods.a_sign_was_struck(sign, runs);
    }

    /// Says over the field that a sign was struck, once one has been.
    pub(super) fn tell_sign(
        &mut self,
        at_bat: &mut AtBat,
        frames: u32,
        stage: &mut Stage,
        library: &Library,
    ) {
        let Some(runs) = self.mods.news_of_a_sign() else {
            return;
        };
        self.show_numbers(stage);
        Match::sound(stage, library, "crowd_bigClap");
        Match::sound(stage, library, "baseball_organ_FX");
        let words = sign::news_words(runs);
        let says = sign::news(&words, frames, at_bat.parts.centre_x);
        at_bat.notices.put(says, &at_bat.parts, stage, library);
    }

    /// Says over the field how a steal came out, once it has.
    pub(super) fn tell_steal(
        &mut self,
        at_bat: &mut AtBat,
        frames: u32,
        stage: &mut Stage,
        library: &Library,
    ) {
        let Some(told) = self.mods.news_of_a_steal() else {
            return;
        };
        at_bat.notices.take_down(steal::HINT, stage);
        let says = steal::news(told, frames, at_bat.parts.centre_x);
        at_bat.notices.put(says, &at_bat.parts, stage, library);
    }
}
