//! One set of figures added to another: what a side did in one match and
//! in the next, or what each of its batters did, put together.

use super::figures::Figures;

impl std::ops::AddAssign for Figures {
    /// Counts `more` in with these. Everything in a set of figures is how
    /// many of something there were, and adds, but for the longest ball,
    /// which is the longer of the two.
    fn add_assign(&mut self, more: Figures) {
        // Taken apart by name, so that a count added to the figures has to
        // be added here before this will build.
        let Figures {
            turns,
            at_bats,
            runs,
            hits,
            singles,
            doubles,
            triples,
            home_runs,
            total_bases,
            runs_in,
            walks,
            strikeouts,
            sacrifices,
            double_plays,
            left,
            chances,
            chances_taken,
            two_out_runs,
            pitches,
            strikes,
            called,
            swinging,
            fouls,
            swings,
            outside,
            chases,
            in_play,
            flies,
            grounders,
            thirds,
            feet,
            longest,
            stolen,
            caught,
        } = more;
        self.turns += turns;
        self.at_bats += at_bats;
        self.runs += runs;
        self.hits += hits;
        self.singles += singles;
        self.doubles += doubles;
        self.triples += triples;
        self.home_runs += home_runs;
        self.total_bases += total_bases;
        self.runs_in += runs_in;
        self.walks += walks;
        self.strikeouts += strikeouts;
        self.sacrifices += sacrifices;
        self.double_plays += double_plays;
        self.left += left;
        self.chances += chances;
        self.chances_taken += chances_taken;
        self.two_out_runs += two_out_runs;
        self.pitches += pitches;
        self.strikes += strikes;
        self.called += called;
        self.swinging += swinging;
        self.fouls += fouls;
        self.swings += swings;
        self.outside += outside;
        self.chases += chases;
        self.in_play += in_play;
        self.flies += flies;
        self.grounders += grounders;
        for (third, more) in self.thirds.iter_mut().zip(thirds) {
            *third += more;
        }
        self.feet += feet;
        self.longest = self.longest.max(longest);
        self.stolen += stolen;
        self.caught += caught;
    }
}

impl std::iter::Sum for Figures {
    /// Every one of these sets of figures, added together.
    fn sum<I: Iterator<Item = Figures>>(each: I) -> Figures {
        let mut all = Figures::default();
        for figures in each {
            all += figures;
        }
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adding_nothing_leaves_figures_as_they_were() {
        let figures = Figures {
            turns: 4,
            hits: 2,
            thirds: [1, 0, 1],
            feet: 500,
            longest: 310,
            ..Figures::default()
        };
        let mut added = figures;
        added += Figures::default();
        assert_eq!(added, figures);
        assert_eq!(
            Vec::<Figures>::new().into_iter().sum::<Figures>(),
            Figures::default()
        );
    }

    #[test]
    fn counts_add_and_the_longest_ball_is_the_longer() {
        let one = Figures {
            turns: 4,
            thirds: [1, 0, 2],
            feet: 500,
            longest: 310,
            ..Figures::default()
        };
        let other = Figures {
            turns: 3,
            thirds: [0, 2, 1],
            feet: 420,
            longest: 220,
            ..Figures::default()
        };
        let both: Figures = [one, other].into_iter().sum();
        assert_eq!(both.turns, 7);
        assert_eq!(both.thirds, [1, 2, 3]);
        assert_eq!(both.feet, 920);
        assert_eq!(both.longest, 310);
    }
}
