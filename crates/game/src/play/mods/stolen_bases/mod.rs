//! Stolen bases: a mod that lets a runner be sent for the next base while
//! the pitcher winds up.
//!
//! The batting view shows nothing of the runners but the one on second, so
//! each is marked on the little field in its corner. A click on that field
//! during the wind-up sends the runner whose mark is nearest, if the base in
//! front of him is free. If the pitch is hit fair he is a runner like any
//! other, with a start. If it is fouled off he goes back. And if it is not
//! hit at all the catcher throws, and the view goes to the field to see
//! which of them is there first.

mod leads;

use bb_engine::library::Library;
use bb_engine::stage::Stage;

use crate::look::Rgb;
use crate::mods::About;
use crate::play::fielding::ARRIVES;
use crate::play::overlay::Says;
use crate::play::pitch::Point;
use crate::play::{Runners, frame_of};
pub(crate) use leads::Leads;

/// What the menu and the files know this mod by.
pub(crate) const ABOUT: About = About {
    key: "stolen_bases",
    name: "STOLEN BASES",
    does: "CLICK THE LITTLE FIELD IN THE WIND-UP TO SEND A RUNNER",
    setting: None,
};

/// The mod, in play: the bases stolen, the runners caught, and how the last
/// try came out.
#[derive(Default)]
pub(crate) struct StolenBases {
    /// How many bases have been stolen in this game, and how many runners
    /// caught at it.
    pub stolen: u32,
    pub caught: u32,
    /// Whether the play in the field is one on which a base can be stolen:
    /// the pitch was not hit, and a runner had gone.
    pub in_play: bool,
    /// How the last try came out, and in what colour to say so, until that
    /// has been told.
    pub to_tell: Option<(&'static str, Rgb)>,
}

/// The colours of a runner's mark: standing on his base, and going.
const STANDING: Rgb = [0xff, 0xff, 0xff];
const GOING: Rgb = [0xff, 0x8a, 0x2a];
/// What the mod's line in the corner of the batting view is named, what it
/// says while a runner may be sent, and once one has been, and what the
/// view of the field says when it is known how that came out: the words,
/// and their colours.
pub(crate) const HINT: &str = "steal";
const ASK: &str = "CLICK TO STEAL";
const SENT: &str = "RUNNER GOING";
const HINT_COLOUR: Rgb = [0xc8, 0xf0, 0xff];
const STOLEN: (&str, Rgb) = ("STOLEN BASE!", [0xff, 0xe2, 0x4a]);
const CAUGHT: (&str, Rgb) = ("CAUGHT STEALING!", [0xff, 0x8a, 0x6a]);
/// How far down the view of the field that is said.
const NEWS_TOP: f32 = 232.0;

impl StolenBases {
    /// A try has come out: it is counted, and kept to be told.
    pub fn came_out(&mut self, safe: bool) {
        if safe {
            self.stolen += 1;
        } else {
            self.caught += 1;
        }
        self.to_tell = Some(if safe { STOLEN } else { CAUGHT });
    }
}

/// What the corner of the view says while a runner may be sent. `top` is
/// where the mod has its say there.
pub(crate) fn asks(top: Point) -> Says<'static> {
    Says::line(HINT, ASK, HINT_COLOUR).at(top)
}

/// What it says once one has been.
pub(crate) fn says_one_is_going(top: Point) -> Says<'static> {
    Says::line(HINT, SENT, GOING).at(top)
}

/// What the view of the field says of how a try came out, for this many
/// frames. `centre_x` is the middle of that view.
pub(crate) fn news(told: (&'static str, Rgb), frames: u32, centre_x: f32) -> Says<'static> {
    Says::news("stealNews", told.0, told.1, frames)
        .at((centre_x, NEWS_TOP))
        .sized(1.2)
}

/// Keeps a runner who is stealing from running on past the base he is
/// making for while the batting view is still up, where nobody is watching
/// for him to get there.
pub(crate) fn hold(runners: &Runners, stage: &mut Stage) {
    for runner in runners {
        let (Some(_), Some(to), Some(path)) = (runner.stole_from, runner.running_to, &runner.path)
        else {
            continue;
        };
        if frame_of(stage, path) >= ARRIVES[usize::from(to) - 1]
            && let Some(clip) = stage.clip_mut(path)
        {
            clip.playing = false;
        }
    }
}

/// The pitch was fouled off, or the side is out: whoever was stealing goes
/// back to the base he left.
pub(crate) fn send_back(runners: &mut Runners, stage: &mut Stage, library: &Library) {
    for runner in runners {
        let Some(from) = runner.stole_from.take() else {
            continue;
        };
        runner.running_to = None;
        runner.sliding = false;
        if let Some(path) = &runner.path {
            stage.goto_label(path, &format!("base{from}"), false, library);
        }
    }
}

#[cfg(test)]
mod tests {
    use bb_engine::display::Path;

    use super::leads::{Lead, Sent};
    use super::*;
    use crate::art;
    use crate::play::Place;
    use crate::play::runners::tests::{at, runners};

    /// Where the little field is in the view these tests click on.
    const LITTLE_AT: Point = (10.0, 20.0);

    /// The marks of runners standing on these bases, in the order they
    /// came up.
    fn leads(bases: &[u8]) -> Leads {
        let leads = bases.iter().enumerate().map(|(runner, &base)| Lead {
            runner,
            base,
            marks: [Path::new(), Path::new()],
        });
        Leads {
            at: LITTLE_AT,
            area: [10.0, 20.0, 110.0, 100.0],
            leads: leads.collect(),
            hint_at: (0.0, 0.0),
            frames: 0,
        }
    }

    /// The point of the view where the little field marks a base.
    fn mark_of(base: u8) -> Point {
        let mark = art::LITTLE_BASES[usize::from(base) - 1];
        (LITTLE_AT.0 + mark.0, LITTLE_AT.1 + mark.1)
    }

    #[test]
    fn a_click_off_the_little_field_sends_nobody() {
        let half = runners(vec![at(Place::Base(1))]);
        let marks = leads(&[1]);
        assert!(marks.anyone_may_go(&half));
        assert_eq!(marks.sent_by((300.0, 200.0), &half), None);
    }

    #[test]
    fn a_click_on_the_little_field_sends_the_runner_who_may_go() {
        // Second is free, and nobody steals home.
        let half = runners(vec![at(Place::Base(1)), at(Place::Base(3))]);
        let sent = leads(&[1, 3]).sent_by(mark_of(3), &half);
        let first_to_second = Sent {
            runner: 0,
            from: 1,
            to: 2,
        };
        assert_eq!(sent, Some(first_to_second));
        // The runner on first has one standing in his way. The one on
        // second has not.
        let half = runners(vec![at(Place::Base(1)), at(Place::Base(2))]);
        let sent = leads(&[1, 2]).sent_by(mark_of(1), &half);
        let second_to_third = Sent {
            runner: 1,
            from: 2,
            to: 3,
        };
        assert_eq!(sent, Some(second_to_third));
    }

    #[test]
    fn a_click_sends_nobody_when_nobody_may_go() {
        let half = runners(vec![at(Place::Base(3))]);
        let marks = leads(&[3]);
        assert!(!marks.anyone_may_go(&half));
        assert_eq!(marks.sent_by(mark_of(3), &half), None);
    }

    #[test]
    fn a_try_is_counted_and_kept_to_be_told() {
        let mut steals = StolenBases::default();
        steals.came_out(true);
        steals.came_out(false);
        assert_eq!((steals.stolen, steals.caught), (1, 1));
        assert_eq!(steals.to_tell, Some(CAUGHT));
    }
}
