//! The side at bat, put back for the pitch: the batter at the plate, the
//! runners on their bases, and the innings said.

use bb_engine::library::Library;
use bb_engine::stage::Stage;

use super::Coming;
use crate::game::Game;
use crate::look;
use crate::play::book::ORDER;
use crate::play::overlay::Says;
use crate::play::{Match, Parts, Place, Runner, show};

impl Match {
    /// With the southpaw mod on the batter stands on the other side of the
    /// plate, turned round. The number on his shirt is not.
    pub(super) fn stand_the_batter(&mut self, parts: &mut Parts, stage: &mut Stage) {
        stage.upright_text = self.mods.southpaw.is_some();
        if let Some(southpaw) = &mut self.mods.southpaw {
            southpaw.stand(parts, stage);
        }
    }

    /// Sends a new batter to the plate, if nobody is there.
    ///
    /// Each batter in a match has his own skin, and they carry the bat
    /// logos in turn. The arcade game's one batter is as chosen. A full
    /// match's nine come round again, each as he was.
    pub(super) fn bring_up_a_batter(&mut self, game: &Game) {
        if self.runners.batter().is_some() {
            return;
        }
        let team = &game.rules.team;
        let order = match self.mode.full() {
            Some(_) => self.came_up % ORDER,
            None => self.came_up,
        };
        let (skin, logo) = if self.mode.is_arcade() {
            (game.settings.skin, game.settings.logo.clone())
        } else {
            let known = self.mode.full().and(self.line_up.get(order).copied());
            let skin = known.unwrap_or_else(|| {
                let pick = self.rng.below(team.skins.len() as u32) as usize;
                team.skins.get(pick).and_then(|skin| look::rgb(skin))
            });
            if self.mode.full().is_some() && self.line_up.len() <= order {
                self.line_up.resize(order + 1, None);
                self.line_up[order] = skin;
            }
            (
                skin,
                team.logos.get(order % team.logos.len().max(1)).cloned(),
            )
        };
        self.came_up += 1;
        self.runners.push(Runner {
            place: Place::AtBat,
            running_to: None,
            sliding: false,
            runs: 0,
            order,
            stole_from: None,
            skin,
            logo,
            path: None,
        });
        self.clear_count();
    }

    /// A full match says which half of which innings this is, and what the
    /// mods say goes under that.
    pub(super) fn say_the_innings(
        &self,
        coming: &mut Coming,
        stage: &mut Stage,
        library: &Library,
    ) {
        if let Some(full) = self.mode.full() {
            coming.notices.put(
                Says::line("innings", &full.half_words(), [0xfd, 0xf6, 0xc0])
                    .at(coming.corner.line()),
                &coming.parts,
                stage,
                library,
            );
        }
    }

    /// The ball is out of sight until it is thrown, and the strike zone is
    /// drawn only where the skill level shows it.
    pub(super) fn clear_the_plate(coming: &Coming, stage: &mut Stage) {
        let parts = &coming.parts;
        show(stage, &parts.ball, false);
        show(stage, &parts.shadow, false);
        if let Some(zone) = stage.find(&parts.main, &["strikeZone"]) {
            show(stage, &zone, coming.table.show_zone);
        }
    }

    /// Every runner still in the game stands where the last pitch left him.
    pub(super) fn stand_the_runners(
        &mut self,
        coming: &Coming,
        stage: &mut Stage,
        library: &Library,
    ) {
        self.mods.a_steal_is_in_play(false);
        for index in 0..self.runners.len() {
            self.runners[index].path = None;
            self.runners[index].running_to = None;
            self.runners[index].stole_from = None;
            self.runners[index].sliding = false;
            let label = match self.runners[index].place {
                Place::AtBat => "waiting".to_owned(),
                Place::Base(base) => format!("base{base}"),
                Place::Out | Place::Home => continue,
            };
            let Some(symbol) = self.runner_symbol else {
                continue;
            };
            let Some(holder) = &coming.parts.holder else {
                continue;
            };
            let depth = Stage::RULES_DEPTH + index as u16;
            let name = format!("runner{}", index + 1);
            if let Some(path) = stage.attach(holder, symbol, depth, &name, library) {
                stage.goto_label(&path, &label, false, library);
                self.runners[index].path = Some(path);
            }
        }
    }
}
