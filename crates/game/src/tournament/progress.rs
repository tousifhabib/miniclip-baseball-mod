//! Playing a tournament through: the fixture the player is to play, the
//! taking of a result, the playing of the fixtures the player has no part
//! in, and who has won when it is all done.

use super::card::Card;
use super::format::{Format, Round};
use super::on_paper::{self, OnPaper, Paper};
use super::{Tournament, seeds, strength};
use crate::play::field::Ground;
use crate::rules::{FullMatchRules, Rules};
use crate::settings::Difficulty;

/// A fixture of the player's, with what it is played by.
#[derive(Clone, Debug, PartialEq)]
pub struct ToPlay {
    /// The fixture's number, and the round it is in.
    pub fixture: usize,
    pub round: Round,
    /// The side the player is to play, by its place in the draw.
    pub against: usize,
    /// Whether the player's side is at home.
    pub at_home: bool,
    /// What the match's chances are worked out from.
    pub seed: u64,
    /// A full match's rules, with the tournament's innings and the other
    /// side's runs leant by its strength.
    pub rules: FullMatchRules,
    /// The skill level it is played at.
    pub skill: Difficulty,
}

/// Why a card was not taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Misfit {
    #[error("the card is of a fixture that is not the next to be played")]
    OutOfTurn,
    #[error("the card is of other sides than the fixture is between")]
    OtherSides,
    #[error("the card is not one a match could have left")]
    Unsound,
}

/// How a tournament ended for the player's side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    Champion,
    /// Put out in a knockout round, the final among them.
    LostIn(Round),
    /// Where it finished in its table, the top being 1, having won
    /// nothing by it.
    Placed(usize),
}

impl Tournament {
    /// How strong a side is: as the rules have it, by the key it is kept
    /// under. The player's own, which the rules do not have, is middling.
    fn strength_of(&self, side: usize, rules: &Rules) -> f32 {
        let key = self.sides.get(side).map(|side| side.key.as_str());
        key.map_or(1.0, |key| rules.tournament.strength_of(key))
    }

    /// The fixture the player is to play now, if the next fixture is one
    /// of the player's.
    pub fn to_play(&self, rules: &Rules) -> Option<ToPlay> {
        let fixture = self.next()?;
        let against = fixture.against(self.player)?;
        let strength = self.strength_of(against, rules);
        Some(ToPlay {
            fixture: fixture.number,
            round: fixture.kind,
            against,
            at_home: fixture.home == Some(self.player),
            seed: seeds::of_a_fixture(self.seed, fixture.number, self.begun),
            rules: strength::match_rules(&rules.full_match, strength, self.setup.innings),
            skill: self.setup.skill,
        })
    }

    /// The player has begun a fixture. If it is given up half way, the
    /// next go at it is so another game.
    pub fn begin(&mut self) {
        self.begun += 1;
    }

    /// Takes the card of the fixture that was next to be played.
    pub fn take(&mut self, card: Card) -> Result<(), Misfit> {
        let next = self.next().ok_or(Misfit::OutOfTurn)?;
        if card.fixture != next.number {
            return Err(Misfit::OutOfTurn);
        }
        if next.sides() != Some((card.home.side, card.away.side)) {
            return Err(Misfit::OtherSides);
        }
        if !card.is_sound() {
            return Err(Misfit::Unsound);
        }
        self.played.push(card);
        Ok(())
    }

    /// Plays on paper every fixture there is before the player's next,
    /// or to the end of the tournament if the player has none. `mods` is
    /// what the mods that are on do to a match on paper, and `ground` the
    /// field. Returns how many fixtures were played.
    pub fn play_on(&mut self, rules: &Rules, mods: OnPaper, ground: &Ground) -> usize {
        let by = Paper {
            rules,
            skill: self.setup.skill,
            innings: self.setup.innings,
            mods,
            ground,
        };
        let mut played = 0;
        while let Some(fixture) = self.next() {
            let Some((home, away)) = fixture.sides() else {
                break;
            };
            if fixture.has(self.player) {
                break;
            }
            let side = |side| (side, self.strength_of(side, rules));
            let seed = seeds::of_a_fixture(self.seed, fixture.number, 0);
            let card = on_paper::played(fixture.number, side(home), side(away), &by, seed);
            self.played.push(card);
            played += 1;
        }
        played
    }

    /// The side that has won the tournament, once it is over: the winner
    /// of the final, or the top of a league's table.
    pub fn champion(&self) -> Option<usize> {
        if !self.is_over() {
            return None;
        }
        if self.setup.format == Format::League {
            return self.table(0).first().map(|row| row.side);
        }
        let last = self.fixtures().last()?.number;
        self.card(last).map(Card::winner)
    }

    /// How the tournament ended for the player's side, once it is over.
    pub fn players_end(&self) -> Option<End> {
        let champion = self.champion()?;
        if champion == self.player {
            return Some(End::Champion);
        }
        // The knockout round it went out in, if it got that far.
        let fixtures = self.fixtures();
        let out = fixtures.iter().find(|fixture| {
            let lost = self.card(fixture.number).map(Card::loser);
            fixture.kind.is_knockout() && lost == Some(self.player)
        });
        if let Some(fixture) = out {
            return Some(End::LostIn(fixture.kind));
        }
        let group = self.group_of(self.player)?;
        let table = self.table(group);
        let place = table.iter().position(|row| row.side == self.player)?;
        Some(End::Placed(place + 1))
    }
}
