//! A fixture of the tournament in hand: the full match it is played as,
//! the taking of its result into the tournament, and where the screens
//! on either side of it lead.

use bb_engine::stage::Stage;

use super::Baseball;
use super::screen::Screen;
use crate::art;
use crate::menu::MenuPage;
use crate::play::Match;
use crate::tournament::Card;

impl Baseball {
    /// Starts the fixture the player is to play next, if there is one. If
    /// there is none, the menu's summary says why not.
    pub(super) fn play_the_fixture(&mut self, stage: &mut Stage) {
        let rules = &self.game.rules;
        let to_play = self.tournament.as_ref().and_then(|all| all.to_play(rules));
        let (Some(to_play), Some(tournament)) = (to_play, &mut self.tournament) else {
            self.menu
                .open(MenuPage::TournamentSummary, &self.game, stage);
            return;
        };
        // Begun: given up half way, it is another game the next time,
        // which is kept so that it is in another run of the game too.
        tournament.begin();
        self.fixture = Some(to_play);
        self.keep_the_tournament();
        self.show(Screen::FullMatch, stage);
    }

    /// The full match that is to be played: a fixture of the tournament,
    /// if one has been started, and if not a match by itself, from `seed`.
    pub(super) fn a_full_match(&mut self, seed: u64, stage: &Stage) -> Match {
        self.playing = self.game.as_played(true);
        let Some(fixture) = &self.fixture else {
            let home = self.menu.take_home(&self.game.settings);
            return Match::new_full(&self.playing, home, seed, stage.library());
        };
        // A fixture is a full match played by the tournament's numbers:
        // its innings, its skill level, the other side's runs leant by its
        // strength, at home or away as the draw has it, from a seed of its
        // own.
        self.playing.rules.full_match = fixture.rules.clone();
        self.playing.settings.difficulty = fixture.skill;
        let (home, seed) = (fixture.at_home, fixture.seed);
        let mut play = Match::new_full(&self.playing, home, seed, stage.library());
        if let Some(them) = self
            .tournament
            .as_ref()
            .and_then(|tournament| tournament.sides().get(fixture.against))
        {
            play.call_them(&them.name, &them.short);
        }
        play
    }

    /// The fixture in hand has been played to its end. Its card goes into
    /// the tournament, every fixture there is before the player's next is
    /// played on paper, and the menu is told how things stand.
    pub(super) fn take_the_result(&mut self, stage: &Stage) {
        let mods = self.mods_on_paper();
        let our_outs = self.playing.rules.game.outs;
        let (Some(fixture), Some(full), Some(tournament)) =
            (&self.fixture, &self.finished, &mut self.tournament)
        else {
            return;
        };
        let (ours, theirs) = (tournament.player(), fixture.against);
        let Some(card) = Card::of(full, fixture.fixture, ours, theirs, our_outs) else {
            return;
        };
        if let Err(misfit) = tournament.take(card) {
            eprintln!("The result could not be taken into the tournament: {misfit}");
            return;
        }
        let rules = &self.game.rules;
        tournament.play_on(rules, mods, &art::ground(stage.library(), rules));
        self.tell_the_menu();
        self.keep_the_tournament();
    }

    /// On from the screen a game ended on: to the tournament's tables if
    /// the game was one of its fixtures, and if not to the menu.
    pub(super) fn on_from_the_result(&mut self, stage: &mut Stage) {
        if self.fixture.is_some() {
            self.show(Screen::Tournament, stage);
        } else {
            self.show(Screen::Menu, stage);
        }
    }

    /// Out of a game that has been given up: to the menu, and if the game
    /// was a fixture of the tournament, to what the menu says of that,
    /// where the fixture is still to be played.
    pub(super) fn give_the_game_up(&mut self, stage: &mut Stage) {
        let fixture = self.fixture.is_some();
        self.show(Screen::Menu, stage);
        if fixture {
            self.menu
                .open(MenuPage::TournamentSummary, &self.game, stage);
        }
    }
}
