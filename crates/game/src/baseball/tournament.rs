//! The tournament in hand: the drawing of one, and its tables on the
//! board.

use bb_engine::stage::Stage;

use super::Baseball;
use super::screen::Screen;
use crate::art;
use crate::board;
use crate::look::Rgb;
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
        // A length of match the rules do not give is the first they do.
        let lengths = &rules.tournament.innings;
        let innings = if lengths.contains(&settings.innings) {
            settings.innings
        } else {
            lengths.first().copied().unwrap_or(settings.innings)
        };
        let setup = Setup {
            format: settings.format,
            innings,
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
