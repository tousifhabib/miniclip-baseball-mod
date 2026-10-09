# 6. What the code is held to

**What was decided.** One table in the root `Cargo.toml` says what the
compiler and clippy hold every crate to: no unsafe code, clippy's stricter
set, a reason on every attribute that switches a lint off, and limits on
how long, how nested and how tangled a function may be. Outside the tests,
nothing is unwrapped.

**The lints that are off.** About twenty, each with its reason beside it.
Most ask for something this code does on purpose: turning pixels into
frames and back, or laying a list of cases out a case to a line. One is off
because its fix would change the game: halving a sum the way it suggests
rounds differently (see 1).

**The limits only come down.** `clippy.toml` sets the limits at the worst
function there is today, and says which that is. When a long function is
broken up the limit comes down to the next worst. It is not to go up.

**Errors.** Where a caller can do something about what went wrong, it is a
type of its own that says what it was: a step of a script that cannot be
read is a `StepFault`. Where all that can be done is to tell the player,
it is an error with the story of what was being attempted. Where the game
should go on regardless, a shape that cannot be drawn or a sound that
cannot be played, it is noted as a problem and reported at the end.
