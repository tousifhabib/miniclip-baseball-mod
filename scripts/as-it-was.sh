#!/usr/bin/env bash
# Plays every game of the record to its end, with the game built to run
# fast, and checks that each still goes as it was written down. `cargo test`
# plays only the start of each.
#
#     scripts/as-it-was.sh
#
# To write down how the games go now, in place of what was written before:
#
#     BB_WRITE_DOWN=1 scripts/as-it-was.sh
#
# Do that only for a change that is meant to alter how the game plays, and
# in a commit of its own that says why. The record is in
# crates/game/tests/as_it_was, which says more.

set -euo pipefail

cd "$(dirname "$0")/.."

BB_WHOLE=1 cargo test --release -p bb-game --test as_it_was "$@"
