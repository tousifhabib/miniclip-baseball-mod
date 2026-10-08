# miniclip-baseball-mod

*Miniclip Baseball*, rebuilt as a native Rust application, for changing.

This is the project for mods. The game as it was is kept in
[miniclip-baseball-vanilla](https://github.com/tousifhabib/miniclip-baseball-vanilla),
which this one started from. That project stays true to the original. This
one is free not to.

The game's art and sound are published here by the project's owner, who
holds the rights to them for this game under an agreement with Miniclip.
Those rights are for this game: they do not make the art free to use
anywhere else. Miniclip does not run or support this project.

## What is here

| Path | Purpose |
|---|---|
| `extracted/` | The game's art, sound and animation, as open files: SVG, PNG, MP3 and JSON |
| `data/rules.toml` | The numbers the game is played by |
| `crates/game` | The game: `bb-game` |
| `crates/engine` | Plays the animation and draws it; `bb-player` and `bb-shot` |
| `crates/format` | The types the art's files are read into |
| `crates/modtools` | Tools for working on the art |
| `scripts/bundle-mac.sh` | Builds the Mac app |
| `assets/icon.svg` | The app's icon |

There is nothing here from a decompiler, and no copy of the original Flash
file. The art in `extracted/` is what the game loads, and is the thing to
edit.

## Playing

Needs a current stable Rust toolchain.

```bash
cargo run --release -p bb-game
```

F1 opens the inspector, F2 pauses, F3 steps one frame while paused, and
Escape quits. `--mute` turns the sound off, `--screen match` (or `menu`,
`arcade` and so on) starts on a screen of your choice, `--seed N` makes
every game go the same way, and `--mod NAME` switches a mod on for that run.

To build it as a Mac app:

```bash
scripts/bundle-mac.sh
open "target/app/Baseball Mod.app"
```

The app is called Baseball Mod and keeps its scores apart from the vanilla
game's, so both can be installed side by side.

## Mods

The menu's first page has a Mods row. It leads to a list of the mods, each
with a box to tick. The list has pages of its own, turned by the arrows under
it. What is ticked is kept from one run to the next, beside
the scores. Every mod starts off, so the game plays as it did until one is
switched on.

| Mod | Name for `--mod` | What it does |
|---|---|---|
| Timing indicator | `timing_indicator` | A bar under the plate that shows when to swing |
| Lone pitcher | `lone_pitcher` | Only the pitcher goes after a ball that has been hit |
| Zinger hit | `zinger_hit` | Every hit is a home run, bigger the better it was timed |
| Butterfingers | `butterfingers` | Fielders drop and fumble the ball, as often as you set |

The timing bar lays out the frames of the pitch from left to right. The
frames on which a swing would meet the ball are coloured by how well: green
for the best, then yellow, orange and red. A white marker runs along the bar
as the frames go by, and reaches the green at the moment to swing. It always
moves at the same pace and the green is always at the same place, so only
the width of the colours changes with the pitch and the skill level. A swing
stops the marker where it was made, and the bar says how it was timed:
too early, early, perfect, late or too late.

With the lone pitcher, the fielder nearest the ball no longer goes for it:
the pitcher does, from the mound, however far off it is. He catches it or
picks it up and throws to a base as any fielder would, and a runner his
throw beats is out. Nobody else moves, and the fielder at the base does not
throw the ball on, so the play ends there and anyone still running is given
his base.

With the zinger hit, every ball the bat meets goes over the wall. A swing
still has to be timed to meet the ball, but how well it was timed now
decides only how far the ball goes:

- The worst-timed swing that still meets the ball sends it 440 feet, just
  over the wall. The best sends it 800 feet on easy, 850 on medium and 900
  on hard. The best moment is the middle of the timing bar's green.
- Sixty of those feet are for holding the ring on the ball, and are lost
  little by little as it is held further off. That is always less than a
  frame of timing is worth.
- Holding the ring below the ball skies it: it goes far higher and hangs in
  the air up to two and a half times as long before it drops. Holding the
  ring above the ball drives it low and gets it there sooner.
- To one side or the other still sends the ball that way, but a hit that
  would have gone foul stays just inside the line.

The home run is called when the ball comes down, not when it crosses the
wall. Until then the ball is drawn large with a trail behind it, a gold mark
beats where it will land, and the distance it has gone is counted up in the
middle of the field. When it lands the count stops, and the place it went is
named: into the stands, off the scoreboard, or out of the park. The crowd
makes more of a better-timed hit and of a longer one, and the outfielders go
back to the wall to watch it over.

The longest zinger there has been is kept beside the high scores. One that
beats it is called a new record as it lands, and the result screen gives the
longest of the game just played with the longest ever. With the timing bar
on as well, a figure over each of the bar's colours says how far a swing on
that colour sends the ball at the most.

Because every hit scores, a match with the mod on starts further behind: 3
runs on easy, 5 on medium and 8 on hard. In the arcade game there is no
target: a hit scores the feet it goes.

The mod's numbers are under `[zinger]` in `data/rules.toml`.

With butterfingers, fielders let the ball go. A fielder under a fly ball
drops it, so the batter is not out and the ball is on the ground. A fielder
bending for a ball on the ground fumbles it, and has to go after it again:
he has it at the second go. And the fielder at a base fails to hold a throw,
so the runner it would have beaten is safe, and he has to gather the ball
before it can go anywhere else. Whoever did it has DROPPED! or FUMBLED! over
him for a moment. The arcade game has no fielders, and plays as it did.

How often they let go is set on the Mods page, on the row of five boxes
under the mod: from 20 goes in a hundred to all of them, starting at 60. The
level is kept with the choice of mods, and stays as it was set while the mod
is off. On the command line it goes after the mod's name, as in
`--mod butterfingers=5`. What each level comes to, how far a dropped ball
rolls and how long a fielder is at a loss are under `[butterfingers]` in
`data/rules.toml`.

## Changing the game

### The numbers

Pitch speeds, timing windows, how the ball flies, the count, the arcade
target's rings and points, how far a zinger goes, sound levels, skin tones
and bat logos are all in `data/rules.toml`, with a note on each. Change them
there.

The game can also lay other files of the same shape over that one, each
holding only the numbers it changes, and refuses a number it does not have
or one of the wrong kind, naming the file. Loading such files from mod
folders, and picking up changes while the game runs, is not written yet.

### The art

Everything in `extracted/` is an ordinary file. Shapes are SVG and open in
any vector editor. Bitmaps are PNG, sounds are MP3, and the animation is
JSON: what each frame of each clip places, moves and removes. `manifest.json`
lists every symbol and its file. The types in `crates/format` document every
field.

After changing any of it, check that every file still reads and every
reference between files still leads somewhere:

```bash
cargo run --release -p bb-modtools --bin extracted-check -- extracted
```

To see what an animation does without stepping through it in a window,
draw many of its frames side by side, each marked with its number:

```bash
cargo run --release -p bb-modtools --bin clip-sheet -- extracted --clip 688 --every 6 --play --out pitcher.png
```

### The rules

The rules are in `crates/game/src`. `baseball.rs` decides which screen is
showing, `menu.rs` is the menu, `play/` is a game in progress, `look.rs`
dresses the batting side, `scores.rs` keeps the high scores, `mods.rs` is
the mods and their page of the menu, and `art.rs` describes how the art is
put together: which clip is which, and what each button is.

The art only knows how to play its animations. The rules are told about
every button the pointer touches and every key pressed, and are called once
a frame. In return they steer the stage: jump a clip to a labelled frame,
move, tint or hide an object, add one of their own, set what a text field
says, or ask for a sound.

### A mod

A mod is a change the player can switch on and off. To add one, give it a
name in `Mod` in `crates/game/src/mods.rs` and add it to `Mod::ALL`: the
menu lists whatever is there. Then have the rules ask
`game.mods.is_on(Mod::YourMod)` wherever the game should go differently.
The timing indicator, in `crates/game/src/play/timing.rs`, is one to copy
from.

A mod that draws something can build it from the art's `BLOCK`, a plain
white square to stretch and tint, `LABEL_FIELD`, a text field, and `HOLDER`,
an empty clip to keep its parts in. All three are described in `art.rs`.

A mod can have a setting as well as being on or off: a level, counted from
1. Give it a name in `Mod::setting`, say how many levels it has and what
each comes to in `Mod::levels` and `Mod::level_words`, and the Mods page
puts a row of boxes under the mod to set it by. The rules read it with
`game.mods.level(Mod::YourMod)`. Butterfingers is the one to copy from.

## Checking a change

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

The tests in `crates/game/tests` play the real game with no window, by
written steps: whole matches and arcade games with batters of different
skill, the menus, the colour picker, the score table and the mods. The same
steps drive the game from the command line:

```bash
cargo run --release -p bb-game -- --screen menu \
  --run "wait 60; click 200 192; wait 60; state; shot setup.png"
```

`wait N` plays N frames, `click X Y` clicks at a stage position, `move`,
`press` and `release` work the pointer by hand, `type TEXT` types, `key NAME`
presses a key, `state` prints where the game is, `events` prints the buttons
touched and sounds asked for, `tree` prints every object on the stage, and
`shot FILE` saves a picture. The stage is 590 by 400.

## Licence

No licence has been chosen yet for the code. The art and sound are not
covered by whatever licence the code is given: see the top of this page.
