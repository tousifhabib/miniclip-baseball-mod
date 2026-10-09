# 9. A hit goes as far whichever way it is sent

**What was decided.** How far a hit goes is settled by the swing: its
timing, and how far above or below the ball the ring was held. Which way
the ball is sent settles only where it comes down. The same swing covers as
much of the field on every frame towards left field, centre and right, and
so comes down as far off, or meets the wall on the same frame.

This is the first thing in which the game with no mod on is not the
original. The vanilla game keeps the original's flight.

**What it was before.** The field is drawn at a slant. The wall is four
hundred feet from home every way, and on the screen that is about 230
pixels straight up the middle, 327 down the left foul line and 431 down the
right. The game had a measure that allows for this, by which it knew when a
ball was at the wall and how many feet it had gone. The ball's flight did
not use it. Two things were counted in pixels of the screen instead:

- *Its speed.* A ball was at the row of its mark after the same number of
  frames whichever way it went. To either side that row is further off, so
  a ball sent there covered up to a third more of the field in a frame than
  one sent straight.
- *The air's hold on it.* Each frame a ball that the ring was not level
  with lost a share of its speed that grew with the square of how far from
  home it was on the screen. At the wall that share was three and a half
  times as great down the right line as straight up the middle, and one and
  three quarter times what it was down the left.

The two pulled against each other. To left field they came out about even.
To right field the second won.

**What that came to.** A swing timed as well as can be, by where the ball
was sent, in feet to where it first came down:

| Ring | Left line | A quarter across | Straight | Three quarters across | Right line |
|---|---|---|---|---|---|
| Level with the ball | over the wall | over the wall | over the wall | over the wall | over the wall |
| 10 pixels under it | 360 | 389 | 381 | 334 | 295 |
| 20 pixels under it | 285 | 321 | 321 | 267 | 231 |

And a swing a frame out, with the ring level: over the wall down either
line, against the wall a quarter of the way in from each, and 382 feet
straight.

Now every row is what its straight column was: over the wall, 381 and 321,
and 382 for the swing that was a frame out.

**Why both, and not the air alone.** Counting only the air's hold by the
field's measure leaves the speed as it was, and then either line is the
best place to send every ball: the swing that goes 381 feet straight goes
over the wall down both. That would be even between left and right and
still not what was wanted, which is that the way a ball is sent should not
decide how far it goes.

**Where the numbers are from.** A hit that goes straight flies as it did,
to within a fiftieth of a pixel on every frame, and a test holds it to
that. For the speed, a hit sent to one side is given the pace that covers
in a frame what a straight one covers. For the air, how far the ball has
gone is counted in the field's measure and the rule's divisor is changed to
suit: along the straight line a pixel is worth 3.7466 of that measure, so
the original's 1,500 becomes 1,500 x 3.7466 x 3.7466, which is 21,056.
`[field]` in `data/rules.toml` has it.

**What it does to a game.** A ball sent to right field with the ring a
little off the ball goes further than it did, by as much as eighty-six
feet. Down the left line it gains a little, and to left centre it loses a
little. A swing that was level but badly timed no longer gets over the wall
down the lines.

Taking every frame of the timing window, the ring anywhere from thirty
pixels over the ball to thirty under, and the ball sent anywhere between
the foul lines, this is how many swings in a hundred clear the wall:

| Skill | Left third | Middle third | Right third | All |
|---|---|---|---|---|
| Easy, as it was | 11.2 | 5.9 | 6.5 | 7.9 |
| Easy, now | 5.4 | 5.4 | 5.4 | 5.4 |
| Medium, as it was | 10.2 | 7.2 | 5.5 | 7.7 |
| Medium, now | 7.2 | 7.2 | 7.2 | 7.2 |
| Hard, as it was | 7.1 | 4.5 | 3.8 | 5.1 |
| Hard, now | 4.5 | 4.5 | 4.5 | 4.5 |

So there are a few fewer home runs in all, because left field has lost
what it had over the middle, and a ball sent to the right third comes down
some thirty feet further off than it did. That spread of swings is not how
anyone plays, and is only a way of putting a number to it.

The record of whole games was written down afresh with this change. Of its
208 games, 136 go differently. The 72 that go as they did are the menu's,
all but one of the monkeys', and every game with the zinger hit on.

**What was left as it was.** Fielders run so many pixels of the screen a
frame, so the right fielder still covers less of the ground than the left
fielder does. That is about what gets caught, not how far a ball goes, and
it is not changed here. A zinger was already sent by the field's measure
and is as it was.
