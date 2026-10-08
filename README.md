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
`arcade`, `fullMatch` and so on) starts on a screen of your choice,
`--seed N` makes every game go the same way, `--mod NAME` switches a mod on
for that run, and `--ground home` (or `away`, or `toss`) says where a full
match is played.

To build it as a Mac app:

```bash
scripts/bundle-mac.sh
open "target/app/Baseball Mod.app"
```

The app is called Baseball Mod and keeps its scores apart from the vanilla
game's, so both can be installed side by side.

## Full match

The menu's first page has a Full Match row, under Bottom of the Ninth. Where
that game is the last innings of a match already nearly lost, this one is
the whole match: nine innings a side.

Only your own innings are played. You bat until three are out, as in the
other game, and then the other side has its half of the innings on paper.
How many runs they make goes by the skill level. Once that is settled the
half is played out a pitch at a time until it comes to just that many: who
struck out, who walked, who hit what and where. A board comes up between
innings to say what they made, with every innings of both sides and how the
match stands, and its button brings your side in again with nobody out and
nobody on base. Nine batters make up the order, and come round again.

The setup page has a choice under the skill levels: Home, Away or Toss.
Away, you bat first, in the top of each innings. At home you bat second, so
the visitors have batted before your first ball. Toss leaves it to a coin,
and the page after says how it came down.

The match ends as a match does. The side at home does not bat in the bottom
of the ninth if it is already ahead, and wins the moment it goes ahead
there. A match that is level after nine goes on, an innings at a time, until
one side is in front. The board the game ends on has every innings on it.

Every pitch to every batter of both sides goes in a scorebook, and when the
match is over the board has pages of what it says, turned by the arrows
under them:

| Page | What is on it |
|---|---|
| Stats | Who won, and each side's runs, hits and errors by innings |
| Your batting, their batting | At-bats, runs, hits, doubles, triples, home runs, runs batted in, walks, strikeouts and average for each of the nine, and for the side, with the figures of the pitcher who threw to them |
| The figures | The two sides side by side: average, on-base, slugging and the two together, average on balls in play and with runners on second or third, and of the pitches the share that were strikes, were swung at, were met, were missed and were chased outside the zone |
| Where you hit it, where they hit it | The field, with a mark where each ball that was put in play came down, coloured by what came of it, and how the hits were spread |
| Your timing | How many frames early or late each of your swings began, and what became of them |
| An innings each | Every turn of both halves in a line: who, what he did, where it went, the runs it brought in and the pitches he saw |

Everything said of either side is added up from that one record by the same
sums, so the pages agree with one another and with the scores. The other
side's figures are of innings that were made up, but made up a pitch at a
time and not to fit: they can be checked turn by turn against the pages of
the innings. Scores run high in this game, so averages do too. A hit's
length is taken from the wall being four hundred feet from the plate.

Every mod is played by in a full match just as in the last innings alone.
With the zinger hit mod on, when every ball you hit is a home run, the other
side makes about three times its usual runs. The numbers are under
`[full_match]` in `data/rules.toml`, with the chances their innings are
played out by under `[full_match.their_batting]`.

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
| Knuckleball | `knuckleball` | Pitches sway from side to side, and the crossing marker is only roughly right |
| Heat check | `heat_check` | Every run makes the next pitch faster, and every strike slows them again |
| Mystery pitch | `mystery_pitch` | Each pitch is a fastball, a change-up or a curve, and you find out as it is thrown |
| Called shot | `called_shot` | Click a spot on the outfield before a pitch; a hit that lands near it is worth extra runs |
| Hot bat | `hot_bat` | Each hit in a row widens your timing window, and a strike resets it |
| Sudden death | `sudden_death` | One strike and you are out, but every run counts double |
| Golden ball | `golden_ball` | Every fifth pitch is gold: runs off it count triple, and a strike on it is an out |
| Pinball park | `pinball_park` | The ball ricochets off the wall, the ground and the foul lines, as bouncily as you set |
| Moon ball | `moon_ball` | Every hit floats to where it was going, several times slower, as you set |
| Turbo runners | `turbo_runners` | Runners are several times as fast, as you set, and can be sent on with the ball in the air |
| Night game | `night_game` | The stadium is dark with the players and ball lit, and home runs flash the lights |
| The shift | `the_shift` | The fielders stand where you have been hitting the ball, until you go the other way |
| Tired arm | `tired_arm` | The pitcher slows and misses the zone more as his pitches mount up, until a fresh one comes in |
| Stolen bases | `stolen_bases` | Click the little field as the pitcher winds up to send a runner; the catcher throws to put him out |
| Hit the sign | `hit_the_sign` | Signs on the outfield wall pay runs to a ball that strikes them, the lit one most |
| Rally | `rally` | Each batter in a row who reaches base makes every run worth one more, until somebody is out |
| Clutch | `clutch` | With two out and a runner on second or third, every run counts double |

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

With the knuckleball, every pitch sways from side to side on its way in, a
little while it is far off and up to 16 pixels either way as it comes by. The
marker still shows where the pitch was going before it began to sway, so the
ball crosses somewhere near it and not always on it, and the red pointer on
the outfield answers to the marker. The hit goes by where the ball really
was: a ring held on the marker sends it off to one side by as much as the
ball was out. A pitch that sways out of the strike zone is a ball. The
numbers are under `[knuckleball]` in `data/rules.toml`.

With the heat check, every run you score takes six hundredths off the time
the next pitch takes, up to eight runs' worth, which is nearly twice as
fast. Every strike, a foul that counts as one included, puts one run's worth
back. How much heat is on is written under the little field in the corner of
the batting view. The arcade game has no runs, and plays as it did. The
numbers are under `[heat]` in `data/rules.toml`.

With the mystery pitch, each pitch is one of three kinds, by chance: a
fastball, which takes seven tenths of the usual time, a change-up, which
takes half as long again, or a curve, which swings a pixel and a half more
each frame to one side or the other and drops more as well. Nothing gives it
away beforehand: the pitcher stands as long before one as before another,
and the marker of where the pitch will cross is not shown until the ball has
left his hand. Then the pitch is named over him. The numbers are under
`[mystery]` in `data/rules.toml`.

With the called shot, a click on the outfield while the pitcher stands and
waits puts a target there: the arcade game's, drawn smaller. It lies behind
the players in the batting view and is seen again on the field once the ball
is hit. Another click moves it, until the wind-up starts. A hit that first
comes down on the target is worth runs on top of whatever it earns the usual
way: three in the middle, two in the next ring, and one in either of the
outer two. A ball that is caught, or that goes over the wall, never comes
down on it. Each pitch is called afresh, and need not be called at all. The
arcade game has a target of its own, and plays as it did. The numbers are
under `[called_shot]` in `data/rules.toml`.

With the hot bat, every swing in a row that meets the ball adds a frame to
each end of the timing window for the next, up to three, each as good as the
frame that was the end. A strike that is not a foul takes the window back to
what it was. How hot the bat is is written in the corner of the batting
view, and the mark on the bat glows, redder the hotter. The number is under
`[hot_bat]` in `data/rules.toml`.

With sudden death, one strike puts a batter out, and every run counts for
two. A foul is still never the last strike, so with only one to give it is
no strike at all. Runs a called shot is worth are not doubled. The numbers
are under `[sudden_death]` in `data/rules.toml`.

With the golden ball, every fifth pitch of a match is gold, and says so in
the corner of the batting view. Every run scored off it counts for three: on
a home run, each runner's and the batter's. A strike on it, swung at or
called, puts the batter out whatever the count, though a foul is a foul and
a ball is a ball. The arcade game has no runs or outs, and plays as it did.
The numbers are under `[golden]` in `data/rules.toml`.

In the pinball park, a ball keeps most of its speed when it bounces, and
the air takes nothing from it once it has been down. The wall sends it back
as a cushion would, at the angle it came in at, and a ball that has bounced
never goes over the wall however high it hops: only a hit that clears it on
the fly is a home run. The foul lines are cushions too once the ball has
been down, so it cannot get out. A fielder can only get hold of a ball that
is low, so one that is hopping goes by over his head, and each time it is
sent back whoever is nearest takes up the chase. How bouncy it all is is
set on the Mods page: the ball keeps from six tenths of its speed at each
bounce to nine. The arcade game is left as it was. The numbers are under
`[pinball]` in `data/rules.toml`.

With the moon ball, a hit floats. It goes the way it would have gone and
comes down where it would have come down, and takes several times as long
over it: how many is set on the Mods page, from one and a half to five, and
starts at two. A home run is still a home run. What changes is what the
time is worth: a fielder who could not have got under a ball has it, and
the runners have longer to go round. A batter is out to a catch wherever he
has got to by then. A ball a fielder has thrown is as fast as ever. The arcade game is left as it was. The numbers are under `[moon]`
in `data/rules.toml`.

With turbo runners, runners go round the bases several times as fast: how
many is set on the Mods page, from one and a half to four, and starts at
two. A runner on a base can be sent on at any time the ball is in play, in
the air or not, where as the game was he had to wait for it to come down or
be caught. The arcade game has no runners. The numbers are under `[turbo]`
in `data/rules.toml`.

With the night game, the picture of the stadium behind everything is
darkened, in the batting view and from over the field, and the players, the
ball and the scoreboards are left as they are, so they stand out lit. A home
run flashes the lights: the stadium goes from dark to brighter than day and
back several times in under a second. It changes nothing about how the game
is played, and the arcade game has it too. The numbers are under `[night]`
in `data/rules.toml`.

With the shift, the fielders stand where you have been hitting the ball.
Every fair ball is remembered by how far across the field it came down,
caught or not. Once there have been three, the outfielders and the shortstop
move over before each pitch: the middle of the field is taken to be where
the last eight went on the whole, and they stand to either side of that as
they stood of the real middle, squeezed up on the side it has moved to and
spread out on the other. The pitcher and the men at the bases stay where
they are. The little field in the corner of the batting view shows where
everyone is standing, with SHIFT LEFT or SHIFT RIGHT under it, and hitting
the other way brings them back. The arcade game has no fielders. The numbers
are under `[shift]` in `data/rules.toml`.

With the tired arm, the pitcher wears out. His first ten pitches are as they
always were. From there to his thirtieth each takes a little longer than the
last, up to three tenths longer, and he aims at a wider and wider area about
the same middle, up to half as wide and as high again, so that more and more
of his pitches miss the strike zone. How many he has thrown is written in the
corner of the batting view, from white through yellow to red, and he grows
flushed. So it pays to make him throw: take the balls, and he tires sooner.
After thirty-six a new pitcher comes in for him, as fresh as the first was,
and the count starts again. In a full match the count goes on from one
innings to the next. The arcade game is over before any arm tires. The
numbers are under `[tired_arm]` in `data/rules.toml`.

With stolen bases, a runner can be sent for the next base while the pitcher
winds up. Each runner on base is marked on the little field in the corner of
the batting view, and once the wind-up starts the marks of those who may go
beat, with CLICK TO STEAL under them. A click on the little field then sends
the one whose mark is nearest. He may go from first or from second, if the
base in front of him is free or the runner on it has been sent too, and
nobody steals home. The ring is where the pointer is, so sending him costs
you your aim for a moment.

What comes of it goes by the pitch. Hit fair, he is a runner like any other
with a start on the ball, and has stolen nothing. Fouled off, he goes back.
On ball four he has the base anyway. On any other pitch the catcher has the
ball: the view goes to the field, he draws back and throws, and whichever of
the ball and the runner is at the base first settles it. A runner takes 211
frames from base to base, the wind-up is 78 of them, and the catcher takes
from 20 to 50 to let go, so the sooner in the wind-up he is sent the better
his chance, and on the harder levels, where pitches are quicker, he needs
the best of starts. Thrown out, he is out and the batter's count is as it
was.

In a full match steals go in the book. The other side's runners steal too,
on paper: now and then one goes before a batter's turn, and gets there
seven times in ten. The page of figures has a line of bases stolen for both
sides once anyone has tried, and each try is told among the turns of its
innings. The numbers are under `[steal]` in `data/rules.toml`.

With hit the sign, the outfield wall has five signs on it, side by side,
each with what it is worth written on it. One of them is lit and beats: a
ball that strikes it is worth three runs on top of whatever it earns the
usual way, and one that strikes any of the others is worth one. A different
sign is lit each innings, never the one that was lit the innings before, so
in the last innings alone it is the same sign throughout. The signs are on
the wall in the batting view too, where the red pointer shows a hit going,
so the pointer on a sign is a hit at it.

A ball strikes a sign if it comes to the wall there no higher than the sign
is tall, on the fly or on the bounce. Too low to clear the wall, it comes
back off the sign as it would off the wall anywhere else. A little higher,
it goes off the top of the sign and out, and is a home run all the same,
with the sign's runs on top. Only the first sign a hit strikes counts, the
runs are the batter's, and like a called shot's they are not multiplied by
anything. The arcade game has no runs to add to. The numbers are under
`[sign]` in `data/rules.toml`.

With the rally, batters who reach base one after another make runs worth
more. Each one who gets on, by a hit, a walk or a fielder's slip, adds one to
what a run counts for from the next pitch on: after three in a row a run is
worth four, and so is each of the four a home run would then bring in. It
stops at five. An out of any kind, at the plate or on the bases, takes it
back to one. What a run is worth is settled when the pitch is thrown, so the
batter who keeps the rally going does not raise the worth of his own hit.
The corner of the batting view says what runs are worth while a rally is
on. Whatever else multiplies runs, sudden death or a golden ball,
multiplies these. The arcade game has no runs. The number is under `[rally]`
in `data/rules.toml`.

With the clutch, a pitch thrown with two out and a runner standing on second
or third is one on which every run counts for two: the runs of everyone it
brings home, the batter's own included. The corner of the batting view says
so, and the organ plays as the batter comes up. A runner on first alone is
not enough. It goes with the other mods that change what a run is worth, so
a golden ball in the clutch is worth six. The arcade game has no runs. The
number is under `[clutch]` in `data/rules.toml`.

## Changing the game

### The numbers

Pitch speeds, timing windows, how the ball flies, the count, how many
innings a full match has and how the other side scores in them, the arcade
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
showing, `menu.rs` is the menu, `play/` is a game in progress, with a full
match's innings in `play/full.rs`, its scorebook in `play/book.rs`, the
other side's innings in `play/paper.rs` and what is written on the boards
in `board.rs`, `look.rs` dresses the batting side, `scores.rs` keeps the high
scores, `mods.rs` is the mods and their page of the menu, and `art.rs`
describes how the art is put together: which clip is which, and what each
button is.

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
  --run "wait 60; click 200 181; wait 60; state; shot setup.png"
```

`wait N` plays N frames, `click X Y` clicks at a stage position, `move`,
`press` and `release` work the pointer by hand, `type TEXT` types, `key NAME`
presses a key, `state` prints where the game is, `events` prints the buttons
touched and sounds asked for, `tree` prints every object on the stage, and
`shot FILE` saves a picture. The stage is 590 by 400.

## Licence

No licence has been chosen yet for the code. The art and sound are not
covered by whatever licence the code is given: see the top of this page.
