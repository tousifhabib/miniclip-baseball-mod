#!/usr/bin/env bash
# Shows how one of the record's games goes differently now from how it went
# at an earlier commit: everything the record takes in of each frame, from
# both, and where the two first part.
#
#     scripts/what-changed.sh COMMIT 'NAME OF THE GAME' [FROM [TO]]
#
# COMMIT is one where the game went as it should. FROM and TO are the frames
# to look at, which the failing test gives. The earlier game is built in a
# work tree of its own under target/, so nothing here is disturbed.

set -euo pipefail

if [[ $# -lt 2 ]]; then
    sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'
    exit 2
fi

then_commit=$1
game=$2
from=${3:-0}
to=${4:-600}

root=$(cd "$(dirname "$0")/.." && pwd)
out="$root/target/what-changed"
tree="$out/then"

rm -rf "$out/was" "$out/is"
mkdir -p "$out"
git -C "$root" worktree remove --force "$tree" 2>/dev/null || true
git -C "$root" worktree add --detach "$tree" "$then_commit" >/dev/null
trap 'git -C "$root" worktree remove --force "$tree" 2>/dev/null || true' EXIT

# Both are played by the record as it is now, which asks nothing of the game
# that an earlier one did not have.
rm -rf "$tree/crates/game/tests/as_it_was"
cp -R "$root/crates/game/tests/as_it_was" "$tree/crates/game/tests/"

write_out() {
    (
        cd "$1"
        BB_TRACES="$2" BB_ONLY="$game" BB_FROM="$from" BB_TO="$to" \
            CARGO_TARGET_DIR="$3" cargo test --release -p bb-game --test as_it_was \
            >"$2.log" 2>&1 || {
            cat "$2.log"
            exit 1
        }
    )
}

echo "Playing '$game' as it was at $then_commit..."
write_out "$tree" "$out/was" "$root/target"
echo "Playing it as it is now..."
write_out "$root" "$out/is" "$root/target"

if diff -ru "$out/was" "$out/is" >"$out/changed.diff"; then
    echo "Frames $from to $to of '$game' are the same in both."
else
    head -n 80 "$out/changed.diff"
    echo
    echo "The whole of it is in target/what-changed/changed.diff,"
    echo "and the two games written out are in target/what-changed/was and is."
fi
