# 2. A record of whole games

**What was decided.** About two hundred seeded games are played by written
steps as part of the tests. Every frame, three running sums are taken: of
what the game says of itself, of what is heard, and of everything on the
stage. Every six hundred frames the sums are compared with the ones written
down in `crates/game/tests/as_it_was`. `cargo test` plays the start of each
game, and `scripts/as-it-was.sh` plays each to its end.

**Why sums and not the games written out.** Written out in full the games
come to hundreds of megabytes. A sum of each stretch says as surely whether
anything changed, and a script (`scripts/what-changed.sh`) writes out the
frames in question from two versions and shows where they part.

**Why its own sum.** The standard library's are free to change from one
version of Rust to the next. This one is a few lines in `sums.rs`, pinned
by values worked out by hand.

**Why per kind of machine.** A few of the game's sums use the machine's own
sines and powers, which may differ in the last place between one kind of
machine and another. The record is kept for the kind it was written on.
Elsewhere each game is played twice and the two are compared.

**When it may be written down afresh.** Only for a change that is meant to
alter how the game plays, in a commit of its own that says why, with
`BB_WRITE_DOWN=1 scripts/as-it-was.sh`. Never to make a failing check pass.

**What it cannot see.** How the stage comes out as pixels
(`scripts/pictures.sh` is for that), how sounds are mixed, and the window.
It was tried against deliberate breakages before it was trusted, and one of
the six changed nothing that can be seen of this game and passed.
