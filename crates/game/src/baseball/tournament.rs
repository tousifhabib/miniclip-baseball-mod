//! The tournament in hand: the drawing of one, and its tables on the
//! board.

use bb_engine::stage::Stage;

use super::Baseball;
use super::screen::Screen;
use crate::art;
use crate::board;
use crate::look::Rgb;
use crate::menu::{Leave, MenuPage, innings_chosen};
use crate::mods::Mod;
use crate::rng::Rng;
use crate::tournament::{Entrant, Format, OnPaper, Setup, Tournament};

/// The colour the player's own side has in a tournament's tables when none
/// has been picked for its shirts: the gold the boards write the player's
/// lines in.
const OURS: Rgb = [0xff, 0xd2, 0x4a];

impl Baseball {
    /// For this run, the shape of the tournament to be drawn and how many
    /// innings its matches have. Whichever is not given is left as it is.
    pub fn choose_tournament(&mut self, format: Option<Format>, innings: Option<u32>) {
        let settings = &mut self.game.settings;
        settings.format = format.unwrap_or(settings.format);
        settings.innings = innings.unwrap_or(settings.innings);
    }

    /// For trying a tournament out: as soon as one is drawn, this many of
    /// its fixtures are played on paper, the player's own among them.
    pub fn play_on_paper(&mut self, fixtures: usize) {
        self.on_paper = fixtures;
    }

    /// Keeps the tournament in this file, starting from the one it holds
    /// if it holds one.
    pub fn keep_tournament_in(&mut self, file: std::path::PathBuf) {
        self.tournament = Tournament::load(&file);
        self.tournament_file = Some(file);
        self.tell_the_menu();
    }

    /// Writes the tournament in hand to where it is kept, or takes away
    /// what is kept there if there is none in hand.
    pub(super) fn keep_the_tournament(&self) {
        let Some(file) = &self.tournament_file else {
            return;
        };
        let kept = match &self.tournament {
            Some(tournament) => tournament.save(file),
            None => Tournament::forget(file),
        };
        if let Err(error) = kept {
            eprintln!("The tournament could not be kept: {error:#}");
        }
    }

    /// What the mods that are on do to a match played on paper.
    pub(super) fn mods_on_paper(&self) -> OnPaper {
        let on = |which| self.game.mods.is_on(which);
        OnPaper {
            every_hit_is_a_home_run: on(Mod::ZingerHit),
            runners_steal: on(Mod::StolenBases),
        }
    }

    /// Draws a tournament as the player has chosen it, in place of any
    /// there was. The player's side is called what has been typed for the
    /// team's name, if anything has.
    pub(super) fn draw_tournament(&mut self, stage: &Stage) {
        let (settings, rules) = (&self.game.settings, &self.game.rules);
        let setup = Setup {
            format: settings.format,
            innings: innings_chosen(&self.game),
            skill: settings.difficulty,
        };
        let typed = stage.text("teamName").unwrap_or_default();
        let player = Entrant::the_players(typed, settings.clothes.unwrap_or(OURS));
        let seed = self.seed.unwrap_or_else(Rng::seed_from_clock);
        let mut tournament = Tournament::new(setup, player, &rules.tournament, seed);
        if let Some(tournament) = &mut tournament {
            let (mods, ground) = (self.mods_on_paper(), art::ground(stage.library(), rules));
            for _ in 0..self.on_paper {
                tournament.play_one_on_paper(rules, mods, &ground);
            }
        }
        self.tournament = tournament;
    }

    /// Tells the menu of the tournament in hand, as it stands now.
    pub(super) fn tell_the_menu(&mut self) {
        let told = self.tournament.as_ref().map(Tournament::brief);
        self.menu.tell(told);
    }

    /// Does what the menu has asked for about a tournament: draws one as
    /// the setup page has it and goes on to its summary, or gives up the
    /// one in hand and goes back to the menu's first page.
    pub(super) fn about_the_tournament(&mut self, asked: Leave, stage: &mut Stage) {
        let page = match asked {
            Leave::Draw => {
                self.draw_tournament(stage);
                self.keep_the_tournament();
                MenuPage::TournamentSummary
            }
            Leave::GiveUp => {
                self.tournament = None;
                self.keep_the_tournament();
                MenuPage::Main
            }
            _ => return,
        };
        self.tell_the_menu();
        self.menu.open(page, &self.game, stage);
    }

    /// While a tournament's tables are showing, keeps the art's own words
    /// off the board, and puts the tables on once the board has arrived.
    pub(super) fn show_tables(&mut self, stage: &mut Stage) {
        if self.screen != Screen::Tournament {
            return;
        }
        let Some((_, arrived)) = Baseball::clear_the_board(stage) else {
            return;
        };
        // The board's button asks for the next innings, of which there is
        // none here. The tables say what it does in its place.
        let words = stage.find_symbol(&[], art::NEXT_INNINGS_WORDS);
        if let Some(words) = words.and_then(|path| stage.child_mut(&path)) {
            words.set_visible(false);
        }
        if !arrived || self.tables.is_some() {
            return;
        }
        let (Some(tournament), Some(shell)) = (&self.tournament, art::shell(stage)) else {
            return;
        };
        let depth = Stage::RULES_DEPTH + 400;
        let Some(holder) = stage.attach(&shell, art::HOLDER, depth, "tournamentTables") else {
            return;
        };
        let rules = &self.game.rules.tournament;
        self.tables = board::Tables::new(tournament, rules, &holder, stage);
    }
}
