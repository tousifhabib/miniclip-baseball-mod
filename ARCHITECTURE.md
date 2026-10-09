# How the code is laid out

This is a map for whoever changes the game next: what the parts are, which
way they lean on one another, what has to stay as it is, and where a new
thing goes. The README says what the game does. The reasons behind the
bigger choices are in `docs/decisions/`.

## The four crates

```
bb-format   the shapes of the art's files, and nothing else
    ^
bb-engine   plays the art: timelines, the pointer, drawing, sound, the window
    ^
bb-game     the baseball: screens, the menu, a match, the mods
bb-modtools tools for working on the art (beside the game, not under it)
```

Each leans only on the ones above it. `bb-format` knows nothing of how the
art is played. `bb-engine` knows nothing of baseball: it would play any art
of this kind. `bb-game` never draws, opens a window or plays a sound
itself. It steers the engine's stage and asks for sounds by name.

## Inside the engine

The engine has a middle that needs no machine, and three ends that do.

| Part | What it is |
|---|---|
| `math` | The two transforms: where a thing is, and what colour |
| `library` | The art in memory, read once and shared |
| `display` | The tree of clips, each playing its own timeline, and the list of what to draw |
| `input` | The pointer and the buttons it rolls over, and the keys |
| `stage` | The tree with its pointer, its text fields and what has happened on it |
| `app` | The runner: plays a frame and hands what happened to the rules |
| `pace` | How many frames to play for each redraw of the screen |
| `tess`, `meshes` | Cutting the art into triangles, and keeping them |
| `gpu` | Drawing a frame with the graphics card |
| `audio` | Playing sounds |
| `window` | The window, its events and the inspector |

Nothing above the line of `gpu` touches a graphics card, a sound device, a
window or the clock. That is what lets the real game be played with no
window, by written steps, in the tests and from the command line
(`--run`). Telling what is under the pointer takes only triangles, so a
game played that way opens a graphics device only if it is asked for a
picture.

The rules meet the engine at two places, both in `app` and `input`:

- `Logic` is what the engine asks of a game: to be told what happened, to
  be called once a frame, to take a key, and to say where it is.
- `Geometry` is what the pointer asks: whether a point is inside a thing.

### A frame

`Runner::tick` plays one frame, in an order that is fixed:

1. Every timeline moves on a frame. Clips are made, moved and removed, and
   sounds on the timelines are asked for.
2. The pointer is looked at again, since what is under it may have moved.
3. Everything that happened is handed to the rules, a thing at a time, and
   whatever that sets off after it, until things are quiet.
4. The rules have their turn (`Logic::tick`).
5. What that set off is handed over in the same way.
6. A click that was being kept for this frame is forgotten.

Everything the rules do to the stage is put in one queue in the order it
was done, and that order can be seen from outside: in the notes, in the
sounds, and in which free place on the stage a new thing takes.

## Inside the game

`baseball.rs` is the top: which screen is showing, and what each button
does. It owns the menu (`menu.rs`), the match being played (`play/`), the
scores and the choice of mods. Each frame it does a short list of named
steps.

### A match

`play::Match` is a game in progress. The art builds the batting view afresh
for every pitch, so the match holds whatever lasts from one pitch to the
next, and `AtBat` holds what belongs to the pitch in hand. A match is in
one `Phase` at a time, from the view arriving, through the wind-up and the
ball's flight, to the fielding and the next pitch being asked for.

- `mode.rs`: a match is exactly one of three games, the last innings, the
  arcade game or a full match, and carries what only that kind keeps.
- `set_up.rs`: getting the view ready for a pitch is a list of steps, each
  a function with a name. The order is the order things have always been
  done in, and it matters (see below).
- `batting.rs`: the frames of a pitch at the plate, a function for each
  part of it: the wait, the wind-up, the flight, the call.
- `fielding.rs`: the ball in the field, the fielders, the throws and the
  runners.
- `pitch.rs`, `field.rs`, `book.rs`, `paper.rs`, and the innings of a full
  match in `full.rs`, touch nothing on the stage. They are sums, and are
  tested as sums.
- `snapshot.rs`: asked how it stands, a match sets down plain facts, and
  the facts print as one line. That printing is the only place the words of
  the line and their order are fixed.

### The mods

Every mod has a file in `play/mods/`, named as the mod is. `ModsInPlay`
holds the ones that are on for the game being played. The rest of the play
never asks whether a mod is switched on. It asks a question that says what
it wants to know (`strikes_allowed`, `worth_of_a_run`,
`the_pitcher_fields_alone`) or says what has happened
(`a_strike_was_called`, `the_batter_reached_base`), and one function for
each names the mods with a say in it, in the order they have it. A mod that
is off is not there. Which mods have no place in the arcade game is said
once, in `ModsInPlay::for_game`.

There is no list of mods that are all called in turn, on purpose: no one
order suits every question. What a run is worth is settled by the golden
ball, sudden death, the rally and the clutch. The lines in the corner of
the view come heat, pitches, hot bat, golden ball, clutch, rally. The line
that says how the game stands has an order of its own again.

A mod's rules are functions that touch nothing on the stage and hand back a
small thing, a `Line` for the corner, a colour, a changed table of numbers,
which the caller puts on the stage there and then. So the rules can be
tested without a game, and the stage is still touched in the order it
always was.

## What has to stay as it is

A seeded game plays out the same every time, frame for frame, and a change
that is not meant to alter the game must leave that so. In practice:

- **Numbers by chance are drawn in the same order.** A match has one
  generator for the play, and separate ones for the lit sign, the other
  side's runs and their innings on paper. Asking for a number sooner,
  later, or not at all changes every pitch after it. Two of the draws only
  choose a sound, and still count.
- **The stage is touched in the same order.** See "A frame" above.
- **Sums are done in the same order.** Two mods that each change how long
  a pitch takes do so one after the other, and each rounds.
- **The line a match prints stays to the letter**, since the tests read it,
  and so do the notes a runner keeps.
- **The art is the clock.** How far a runner has got is the frame his clip
  is on. The ball leaves the pitcher's hand on a frame of his wind-up. Where
  a fielder stands is read back from the stage. None of this is kept twice.

## How a change is checked

`scripts/check.sh` runs everything a change has to pass. The README says
what is in it and how to read a failure. In short there are three nets:

1. **The tests**, most of which play the real game by written steps. Each
   mod's rules also have tests of their own that need no game, and the
   sums are checked against cases made up by the hundred (`proptest`).
2. **The record of whole games** (`crates/game/tests/as_it_was`). About two
   hundred seeded games, with sums of everything said, heard and seen every
   frame. It was tried against deliberate breakages before it was trusted.
3. **Pictures before and after** (`scripts/pictures.sh`), for changes to
   the drawing, which the record cannot see.

Clippy holds the code to more than its defaults. What, and why not the
rest, is in the root `Cargo.toml`. The limits on how long and how tangled a
function may be are in `clippy.toml`, set at the worst there is today, and
are meant only to come down.

## Where a new thing goes

| To add | Go to |
|---|---|
| A mod | `play/mods/`, and the README's "A mod" |
| A number the game is played by | `data/rules.toml`, and its field in `rules.rs` |
| Something for a match to say of itself | A field of `Snapshot`, and its place in the line |
| A step in getting the view ready | `set_up.rs`, at the place in the order it belongs |
| A word the art's files may hold | Its type in `bb-format`, made with `words!` |
| A step of the script language | `Step` in `script.rs` |
| Something kept between runs | `kept.rs` reads and writes it |

## What is not finished

The fielding (`fielding.rs`) is in four named parts, but a fielder's job is
still one function with an arm for each thing he may be doing, and what
kind of play it is, and how it stands, is a handful of flags that would
read better as one thing that says how the play began and how it ended.
Three mods, hit the sign, stolen bases and the zinger hit, have their files
and are asked like the rest, but still keep some of what they need with
the match. The menu, the mods' page and the boards of a full
match each have their own few lines for writing words on a panel, where
one would do. Each of these is a change of the same kind as the ones
already made, with the same nets under it.
