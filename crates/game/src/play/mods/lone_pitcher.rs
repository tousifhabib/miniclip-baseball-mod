//! Lone pitcher: only the pitcher goes after a ball that has been hit.
//!
//! The fielder nearest the ball no longer goes for it: the pitcher does,
//! from the mound, however far off it is. He catches it or picks it up and
//! throws to a base as any fielder would, and a runner his throw beats is
//! out. Nobody else moves, and the fielder at the base does not throw the
//! ball on, so the play ends there and anyone still running is given his
//! base.
//!
//! The mod keeps nothing and has no numbers. That it is in play is all the
//! fielding needs to know.

use crate::mods::About;

/// What the menu and the files know this mod by.
pub(crate) const ABOUT: About = About {
    key: "lone_pitcher",
    name: "LONE PITCHER",
    does: "ONLY THE PITCHER GOES AFTER THE BALL",
    setting: None,
};

pub(crate) struct LonePitcher;
