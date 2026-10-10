#!/usr/bin/env bash
# Draws a set of pictures of the game into a folder: seeded moments of play
# with the mods that change how things look, the menu's pages, a
# tournament's pages, single clips with blurs and masks in them, and a
# sheet of an animation.
#
#     scripts/pictures.sh FOLDER
#
# The record of whole games sees what is on the stage, not how it comes out
# as pixels. For a change to the drawing code, draw the pictures before it
# and after it on the same machine, and compare:
#
#     scripts/pictures.sh target/pictures/before     # at the commit before
#     scripts/pictures.sh target/pictures/after      # with the change
#     scripts/pictures.sh --same target/pictures/before target/pictures/after
#
# The pictures are not kept in the repository: one graphics card does not
# draw quite as another does.
#
# A card just put to work does not draw quite the same for its first few
# draws either, a pixel on a gradient here and there coming out a step of
# one colour away. The programs that take these pictures draw each frame
# until it has come out the same several times running, so a set drawn
# twice on one machine is the same set.

set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)

if [[ ${1:-} == --same ]]; then
    before=$2
    after=$3
    differ=0
    for picture in "$before"/*.png; do
        name=$(basename "$picture")
        if ! cmp -s "$picture" "$after/$name"; then
            echo "$name is not the same"
            differ=1
        fi
    done
    count=$(find "$before" -name '*.png' | wc -l | tr -d ' ')
    [[ $differ == 0 ]] && echo "All $count pictures are the same."
    exit $differ
fi

if [[ $# -ne 1 ]]; then
    sed -n '2,21p' "$0" | sed 's/^# \{0,1\}//'
    exit 2
fi

mkdir -p "$1"
out=$(cd "$1" && pwd)
cd "$root"

cargo build --release -p bb-game -p bb-engine -p bb-modtools
game=target/release/bb-game
shot=target/release/bb-shot
sheet=target/release/clip-sheet

# One moment of the game: a name, the options to start it with, and the
# steps to play before the picture is taken.
play() {
    local name=$1 steps=$2
    shift 2
    "$game" extracted --seed 7 "$@" --run "$steps; shot $out/$name.png" >/dev/null
}

play menu "wait 90" --screen menu
play match-setup "wait 60; click 200 181; wait 60; click 440 194; wait 5" --screen menu
play mods-page "wait 60; click 330 360; wait 90; click 300 140; wait 5" --screen menu
play instructions "wait 120" --screen instructionsAll
# With this seed the first pitch crosses at 335,280 and is best swung at on
# the 217th frame.
ring="wait 60; move 335 280"
swing="$ring; wait 157; click 335 280"

play aiming "$ring; wait 20" --screen match
play pitch-in-flight "$ring; wait 140" --screen match --mod timing_indicator
play fielding "$swing; wait 90" --screen match
play arcade "wait 150" --screen arcade
play zinger "$swing; wait 130" --screen match --mod zinger_hit --mod timing_indicator
play pinball "$swing; wait 160" --screen match --mod pinball_park --mod lone_pitcher
play night "wait 150" --screen match --mod night_game
play southpaw "wait 150" --screen match --mod southpaw
play signs-and-shift "wait 150" --screen match --mod hit_the_sign --mod the_shift
play bullet-time "wait 150; hold space; wait 40" --screen match --mod bullet_time
play every-mod "wait 170" --screen match \
    --mod timing_indicator --mod heat_check --mod tired_arm --mod hot_bat --mod golden_ball \
    --mod clutch --mod rally --mod stolen_bases --mod hit_the_sign --mod bullet_time
play full-match "wait 200" --screen fullMatch --ground away
play interval "wait 200" --screen fullMatch --ground home
play twice-the-size "wait 150" --screen match --scale 2

# A tournament: its two pages of the menu, and its tables for each shape,
# with fixtures enough played on paper for there to be something in them.
tables="wait 120"
play tournament-setup "wait 60; click 200 240; wait 70; click 434 290; wait 5" --screen menu
play tournament-summary "wait 60; click 200 240; wait 70; click 490 362; wait 70" --screen menu
play tournament-league "$tables" --screen tournament --format league --played 9
play tournament-groups "$tables" --screen tournament --format groups --played 13
play tournament-cup "$tables" --screen tournament --format cup --played 5
play tournament-match "$tables; click 151 63; wait 5; click 228 323; wait 5; click 52 122; wait 5" \
    --screen tournament --format league --played 9
play tournament-side "$tables; click 255 63; wait 5; click 52 120; wait 5; click 362 323; wait 5" \
    --screen tournament --format league --played 9
play tournament-leaders "$tables; click 359 63; wait 5" --screen tournament --format league --played 9
play tournament-records "$tables; click 463 63; wait 5" --screen tournament --format league --played 9

# Clips on their own: the ones with a blur in them, and the first frames of
# the whole movie.
for clip in 161 457 1675 1988 2000 2026; do
    "$shot" extracted --clip "$clip" --ticks 20 --out "$out/clip-$clip.png" >/dev/null
done
"$shot" extracted --ticks 120 --out "$out/movie.png" >/dev/null
"$shot" extracted --ticks 120 --pointer 300,200 --press --scale 1.5 --out "$out/movie-pressed.png" >/dev/null

# The pitcher's wind-up, a frame in six.
"$sheet" extracted --clip 688 --every 6 --play --out "$out/pitcher-sheet.png" >/dev/null

echo "Drew $(find "$out" -name '*.png' | wc -l | tr -d ' ') pictures in $out"
