//! Starts the real game with no window, for tests that drive it with
//! written steps.
//!
//! These tests need the extracted art. Where it is missing they pass
//! without checking anything, and say so.
//!
//! Starting a game is in `start`, playing a pitch of one in `play`, and
//! reading what it says in `read`. A test asks for any of them from here.

#![allow(
    dead_code,
    unused_imports,
    reason = "each file of tests uses some of these and not the rest"
)]

mod play;
mod read;
mod start;

pub use play::*;
pub use read::*;
pub use start::*;
