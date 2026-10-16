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
| `stage` | The tree with its pointer, its text fields and what has happened on it, and the art it plays from |
| `app` | The runner: plays a frame and hands what happened to the rules |
| `pace` | How many frames to play for each redraw of the screen |
| `tess`, `meshes` | Cutting the art into triangles, and keeping them |
| `gpu` | Drawing a frame with the graphics card |
| `audio` | Playing sounds |
| `window` | The window, its events and the inspector |

Every part but the smallest is a folder with a file for each of its jobs.
`display` has the tree, the playing of a timeline with what a frame does
to a clip's children apart, what things cover, the list of what to draw
and the list of what is there. `stage` has the finding of things apart
from the steering of them, with its sounds and its typing. `gpu` has the
plan of a frame, the sending of it to the card, the passes that draw it,
the pipelines and the shader. `tess` has a file for each kind of art.
`window` has its own state, what it does about each event, its layout, its
keys and its redrawing. What every part's tests are built from is in
`testing.rs`.

`bb-format` is cut the same way: a file for each kind of file the art
has, a clip, a button, text, a font, a morph and the manifest that lists
them, and one each for what they share.

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

Each time, the rules are handed the stage and nothing else. The stage
carries the art it plays from, so whatever is given the stage can put a
thing on it, send a clip to a frame or ask for a sound without being given
the art as well. The few things that read the art itself, the size of a
text field, the frames of a clip, ask the stage for it
(`stage.library()`). Only what has no stage is handed the art: what sets a
game up before there is one.

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

`baseball/` is the top: which screen is showing, and what each button
does. It owns the menu (`menu/`), the match being played (`play/`), the
scores, the choice of mods and the tournament in hand (`tournament/`). Each
frame it does a short list of named steps. Its files are the screens, the
going between them, a game in progress, what a game leaves behind it, the
player's choices, the tournament in hand and a fixture of it, what is
heard, and the game's account of itself. What every screen works from, the
rules, the player's choices and the mods that are on, is the `Game` in
`game.rs`. Whatever writes words or puts the art's things on a panel, the
menu, the list of mods, the table of scores, the boards of a full match
and the tables of a tournament, does it with the one `Sheet` in `sheet.rs`.
A row of boxes to choose one thing of several by is the one `Choice` in
`choice.rs`.

The numbers the game is played by are in `rules/`, which mirrors
`data/rules.toml`: the numbers of the game, of the ball and of the mods,
the shapes a number comes in, the laying of one file of rules over
another, and the check that a set can be played by. The boards of a full
match are in `board/`, a file to a page. A tournament's tables are on a
board of the same kind, in `board/tables/`, a file to a kind of page. The
tables of batting, of figures and of innings are written from rows, by
one writer each, whichever of the two the rows come from.

### A match

`play::Match` is a game in progress. The art builds the batting view afresh
for every pitch, so the match holds whatever lasts from one pitch to the
next, and `AtBat` holds what belongs to the pitch in hand. A match is in
one `Phase` at a time, from the view arriving, through the wind-up and the
ball's flight, to the fielding and the next pitch being asked for.

- `mod.rs` has the match itself and its frame. Beside it, a file each:
  `phase.rs` and `at_bat.rs` for those two, `standing.rs` for what a run is
  worth and whether the match is over, `scoreboard.rs` for what it writes
  on the art's boards, `zingers.rs`, and `describe.rs` for its account of
  itself.
- `mode.rs`: a match is exactly one of three games, the last innings, the
  arcade game or a full match, and carries what only that kind keeps.
- `set_up/`: getting the view ready for a pitch is a list of steps, each a
  function with a name. The order is the order things have always been
  done in, and it matters (see below). The list is in `mod.rs`. The steps
  are beside it: the side at bat, the mods', and the deciding of the pitch.
- `batting/`: the frames of a pitch at the plate, a file for each part of
  it: the wind-up, the aim, the swing, the call, and the watching of a hit.
- `fielding/`: the ball in the field. `play.rs` says what kind of play it
  is, one of a walk, a foul, a steal or a fair ball, and how it stands.
  Then a file for each thing that goes on in one: the change of view, the
  loose ball, the fielder going after it, the throws, the runners, the end
  of the play, and the mods' news.
- `runners/`: the batters of the half and where each has got to, and the
  count in a file of its own. They answer what the play asks of them, who
  is up, who has to go, which base may be stolen, without looking at the
  stage.
- `view/`: where the parts of the view are and how they are found, the few
  things done to them over and over, how the batting view lies against the
  field, and the words and notices laid over both.
- `pitch/`, `field/`, `book/`, `paper/`, and the innings of a full match
  in `full/`, touch nothing on the stage. They are sums, and are tested as
  sums. Each has its parts in files: the pitch its timing window, the
  field its ground and the way a fielder faces, the book a turn, the
  figures, the adding of figures together and the rows a board writes of
  them, an innings on paper its pitches and its balls in play. `full/`
  says once how a match ends, in `ending.rs`, which a match played wholly
  on paper asks as well.
- `snapshot/`: asked how it stands, a match sets down plain facts, and the
  facts print as one line. That printing, in `words.rs`, is the only place
  the words of the line and their order are fixed.

### The mods

Every mod has a file in `play/mods/`, named as the mod is, or a folder
where there is more to it: what it works out in one file, what it draws in
another. `ModsInPlay`, in `play/mods/in_play/`, holds the ones that are on
for the game being played. The rest of the play never asks whether a mod
is switched on. It asks a question that says what it wants to know
(`strikes_allowed`, `worth_of_a_run`, `the_pitcher_fields_alone`) or says
what has happened (`a_strike_was_called`, `the_batter_reached_base`), and
one function for each names the mods with a say in it, in the order they
have it. The questions are in files by what they are about: the score, the
pitch, the swing, the slowing of the ball, the field and the runners. A
mod that is off is not there. Which mods have no place in the arcade game
is said once, in `ModsInPlay::for_game`.

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

A mod never acts on the match. No mod's file has a function of the match
in it, and a test reads the files to keep it so. What a mod is known by on
the menu, its key, its name and its setting, is at the top of its own file
too, so the whole of a mod is in one place.

### A tournament

A tournament is made of full matches, and adds no kind of game of its own.
`tournament/` is all sums: none of it touches the stage, and a whole
tournament can be drawn, played out on paper and added up with no game
running, which is how most of its tests go.

- A `Tournament` keeps what was chosen for it, the sides in the order they
  were drawn, and the card of every fixture played. Who meets whom next,
  how the tables stand and who has won are worked out from those each time
  they are asked for. Nothing is kept twice.
- `format.rs` has the three shapes, `sides.rs` the draw, `schedule/` who
  is to meet whom, and `fixtures.rs` the fixtures as far as they are
  known. In each round the player's own comes first, so that the tables
  are level whenever the player comes to bat.
- A fixture of the player's is a full match played by other numbers:
  `progress.rs` says which, the tournament's innings, the skill level it
  was drawn at, the other side's runs leant by its strength
  (`strength.rs`), and a seed of the fixture's own. `baseball/fixture.rs`
  lays them over the game the match is played by, as the mods' numbers
  are laid over it, and nothing in `play/` knows a tournament is on.
- A fixture of other sides is a match `on_paper`: both sides doing what
  the other side of a full match does.
- What is kept of a match is its `card`: the runs innings by innings and
  the figures of the nine places of each side. `table.rs` and `stats/`
  add the cards up, as rows for a page to write, by the sums the book has
  for a full match's own pages. `brief.rs` is the few lines the menu says,
  and `describe.rs` the one a script reads.
- `kept.rs` writes a tournament out and reads it back by playing the cards
  into one begun afresh, so that a file that is no tournament is found
  out.

The menu is told of the tournament in hand and keeps no more of it than
that. Its two pages for one are in `menu/tournament/`, on the sections the
art has for a match.

## How the files are cut

A file does one job and says which in its first lines. None is over three
hundred lines, which `scripts/check.sh` sees to, and most are well under
half that. A part with more than one job is a folder named for the part:
its `mod.rs` holds what the whole is about and hands on the names the rest
of the game uses, so nothing outside has to know how it is cut up.

Tests sit beside what they test. Short ones are at the foot of the file.
Longer ones are in `tests.rs` beside it, and statements tried against
made-up cases in `properties.rs`. A long file of played tests is a folder
that is still one program of tests. What the played tests share is said
once, in `crates/game/tests/common`.

## What has to stay as it is

A seeded game plays out the same every time, frame for frame, and a change
that is not meant to alter the game must leave that so. In practice:

- **Numbers by chance are drawn in the same order.** A match has one
  generator for the play, and separate ones for the lit sign, the other
  side's runs and their innings on paper. A tournament has one for its
  draw and a seed for each fixture, from which a fixture on paper has a
  generator for each of its sides. Asking for a number sooner,
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
2. **The record of whole games** (`crates/game/tests/as_it_was`). Over two
   hundred seeded games, with sums of everything said, heard and seen every
   frame. It was tried against deliberate breakages before it was trusted.
3. **Pictures before and after** (`scripts/pictures.sh`), for changes to
   the drawing, which the record cannot see.

Clippy holds the code to more than its defaults. What, and why not the
rest, is in the root `Cargo.toml`. The limits on how long and how tangled a
function may be are in `clippy.toml`, set at the worst there is today, and
are meant only to come down. How long a file may be is in the check
itself.

## Where a new thing goes

| To add | Go to |
|---|---|
| A mod | `play/mods/`, and the README's "A mod" |
| A question to ask the mods | `play/mods/in_play/`, in the file for what it is about |
| Something to ask of the runners or the count | `play/runners/`, with a test that needs no game |
| Words or a drawing on a panel | A `Sheet`, from `sheet.rs` |
| A number the game is played by | `data/rules.toml`, and its field in the file of `rules/` for what it is a number of |
| Something for a match to say of itself | A field of `Snapshot`, and its place in the line in `snapshot/words.rs` |
| A step in getting the view ready | `set_up/`, called from `mod.rs` at the place in the order it belongs |
| A page of a full match's boards | A file in `board/`, and its name in `Page` |
| A page of a tournament's tables | A file in `board/tables/`, and its name in `Page` there |
| Something a tournament adds up | `tournament/stats/`, as rows, with a test that needs no game |
| A shape of tournament | `Format` in `tournament/format.rs`, and its ties in `tournament/schedule/` |
| A side to play against | `[tournament.sides]` in `data/rules.toml` |
| One thing to be chosen of several | A `Choice`, from `choice.rs` |
| A row of the menu's first page | New art, placed in the menu's clip, and its words in `Menu::clicked` |
| A word the art's files may hold | Its type in `bb-format`, made with `words!` |
| A step of the script language | `Step` in `script/step.rs` |
| Something kept between runs | `kept.rs` reads and writes it |
| A helper two files of played tests both want | `crates/game/tests/common`, once |

## What is not finished

Everything the plan for this shape set out to do is done. What is left is
smaller, and of the same kind as what has been done already.

The limits in `clippy.toml` are at a hundred lines and six deep, and could
come down further: the pages of a full match's boards are the longest
functions left, and the test that reads those pages is the most tangled.
The record of whole games is kept for one kind of machine, and on any
other it only checks that a game plays the same twice. The workflow in
`.github/` runs everything on each push, on one of GitHub's Macs, whose
sums have agreed with the record every time so far.

## Found on the way

Reshaping the code turned up things that looked like slips. They were left
alone while the code was being reshaped, so that the game stayed the game,
and put right afterwards, each in a commit of its own that says what it
was. Three of them changed how seeded games go, and the record was written
down afresh for those: a miss now cools a hot bat in the arcade game, a
steal is told when its play ends, and the lit sign no longer draws the
numbers the coin does.

Three things that looked like slips are as they are meant to be.

- **Every game started in one run with a seed is the same game.** That is
  what `--seed` is for. Without it each game has a seed of its own from
  the clock.
- **The other side has three outs, three strikes and four balls, whatever
  the rules say.** Its innings are played on paper by the game's own old
  rules, and only the player's side is played by the numbers in
  `data/rules.toml`.
- **A picture is drawn several times before it is kept.** A graphics card
  just put to work draws a pixel here and there on a gradient a step of
  one colour out for its first few draws, and then settles. Nothing sent
  to it differs. `Renderer::capture` draws until the frame has come out
  the same six times running. In the window the same thing happens in the
  first hundredth of a second, where nobody could see it.
