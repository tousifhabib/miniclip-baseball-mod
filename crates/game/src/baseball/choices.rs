//! What the player chooses outside a game: the mods, and how the side
//! looks.

use bb_engine::stage::Stage;
use bb_format::SymbolId;

use super::Baseball;
use super::screen::Screen;
use crate::art;
use crate::look::{Look, Rgb, Swatch};
use crate::mods::{Asked, Mod};

impl Baseball {
    /// Switches a mod on or off for this run, without writing that down.
    pub fn switch_mod(&mut self, which: Mod, on: bool) {
        self.game.mods.set(which, on);
    }

    /// Sets a mod's setting to a level for this run, without writing that
    /// down.
    pub fn set_mod_level(&mut self, which: Mod, level: u8) {
        self.game.mods.set_level(which, level);
        self.game.mods.keep_within(&self.game.rules);
    }

    /// Acts on a click on one of the boxes on the mods' page.
    pub(super) fn choose_mod(&mut self, path: &[u16]) {
        match self.mods_page.clicked(path) {
            Some(Asked::Switch(which)) => {
                self.game.mods.toggle(which);
            }
            Some(Asked::Level(which, level)) => self.game.mods.set_level(which, level),
            None => return,
        }
        self.game.mods.keep_within(&self.game.rules);
        if let Some(file) = &self.mods_file
            && let Err(error) = self.game.mods.save(file)
        {
            eprintln!("The choice of mods could not be saved: {error:#}");
        }
    }

    /// The colour under the pointer on one of the setup pages' strips.
    fn picked(stage: &Stage, strip: Option<&Swatch>, name: &str) -> Option<Rgb> {
        let shell = art::shell(stage)?;
        let path = stage.find_named(&shell, name)?;
        let (x, y) = stage.from_stage(&path, stage.pointer.x, stage.pointer.y)?;
        strip?.at(x, y)
    }

    /// Acts on a click on one of the setup pages' colour and logo choices.
    pub(super) fn choose_look(&mut self, button: SymbolId, stage: &Stage) {
        let settings = &mut self.game.settings;
        if art::CLOTHES_STRIP_BUTTONS.contains(&button) {
            if let Some(colour) =
                Baseball::picked(stage, self.clothes_strip.as_ref(), "clothesPicker")
            {
                settings.clothes = Some(colour);
            }
        } else if art::SKIN_STRIP_BUTTONS.contains(&button) {
            if let Some(colour) = Baseball::picked(stage, self.skin_strip.as_ref(), "skinPicker") {
                settings.skin = Some(colour);
            }
        } else if button == art::CLOTHES_BUTTON.0 {
            settings.clothes = Some(art::CLOTHES_BUTTON.1);
        } else if button == art::SKIN_BUTTON.0 {
            settings.skin = Some(art::SKIN_BUTTON.1);
        } else if let Some((_, logo)) = art::LOGO_BUTTONS.iter().find(|(id, _)| *id == button) {
            settings.logo = Some((*logo).to_owned());
        }
    }

    /// How the batting side should look on the screen that is showing.
    pub(super) fn look(&self) -> Look {
        let settings = &self.game.settings;
        match (&self.play, self.screen) {
            (Some(play), Screen::Match | Screen::FullMatch) => play.look(settings.clothes),
            // The arcade game and the setup pages show what was chosen.
            _ => Look {
                clothes: settings.clothes,
                skin: settings.skin,
                logo: settings.logo.clone(),
                second_skin: None,
            },
        }
    }
}
