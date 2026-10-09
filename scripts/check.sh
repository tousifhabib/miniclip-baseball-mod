#!/usr/bin/env bash
# Everything a change has to pass before it is handed over: the code laid
# out as `cargo fmt` lays it, nothing for clippy to say, the tests, the docs
# and the art.
#
#     scripts/check.sh
#
# The tests include the start of every game in the record of whole games.
# `scripts/as-it-was.sh` plays each of them to its end, which is worth doing
# as well after a change to how the game is put together.

set -euo pipefail

cd "$(dirname "$0")/.."

step() {
    printf '\n== %s\n' "$1"
    shift
    "$@"
}

step "The code is laid out as cargo fmt lays it" cargo fmt --all --check
step "Clippy has nothing to say" cargo lint
step "The tests pass" cargo test
step "The docs build" env RUSTDOCFLAGS="-D warnings" cargo docs
step "The art reads" cargo art-check

printf '\nAll is well.\n'
