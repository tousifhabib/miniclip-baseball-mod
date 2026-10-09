#!/usr/bin/env bash
# Everything a change has to pass before it is handed over: the code laid
# out as `cargo fmt` lays it and in files of a readable length, nothing for
# clippy to say, the tests, the docs and the art.
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

# A file does one job, and is short enough to be read through. The limit is
# a backstop: a file is cut where its jobs part, long before it gets here.
MOST_LINES=300

no_long_files() {
    local long
    long=$(find crates -name '*.rs' -exec wc -l {} + |
        awk -v most="$MOST_LINES" '$2 != "total" && $1 > most { print "  " $2 ": " $1 " lines" }')
    if [ -n "$long" ]; then
        printf 'Over %s lines, and wanting to be cut into their parts:\n%s\n' "$MOST_LINES" "$long"
        return 1
    fi
}

step "The code is laid out as cargo fmt lays it" cargo fmt --all --check
step "No file of code is over $MOST_LINES lines" no_long_files
step "Clippy has nothing to say" cargo lint
step "The tests pass" cargo test
step "The docs build" env RUSTDOCFLAGS="-D warnings" cargo docs
step "The art reads" cargo art-check

printf '\nAll is well.\n'
