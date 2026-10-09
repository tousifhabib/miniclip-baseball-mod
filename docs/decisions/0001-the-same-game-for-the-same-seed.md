# 1. The same game for the same seed

**What was decided.** A game started with a seed plays out the same every
time: the same pitches, the same bounces, the same sounds on the same
frames. A change that is not meant to alter the game has to keep that so,
to the frame.

**Why.** It is what lets the game be tested by playing it. A test can swing
at the third pitch of seed 4 and say what must come of it. It is also what
lets a large change be checked without anyone having to say in advance
everything that might go wrong: if two hundred seeded games still go just
as they went, nothing went wrong.

**What it costs.** Order becomes part of what the code does. Numbers by
chance have to be asked for in the same order, the stage touched in the
same order, and sums done in the same order, since each rounds. Two of the
numbers drawn only choose which of three shouts the umpire gives, and they
still have to be drawn. `ARCHITECTURE.md` lists what has to stay as it is.

**What it does not mean.** It does not mean the game may never change. A
change that is meant to alter how the game plays says so, and writes the
record down afresh (see 2).
