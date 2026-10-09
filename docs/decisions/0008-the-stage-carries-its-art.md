# 8. The stage carries its art

**What was decided.** A `Stage` holds the art it plays from. Nothing that
is given the stage is given the art as well: the engine hands the rules
the stage alone, and every function of the game that works on the stage
takes the stage alone. What reads the art itself asks the stage for it.

**What it was before.** The art was passed beside the stage to everything.
A hundred and twenty-three functions of the game took both, and all but
six of them did nothing with the art but pass it on, to the next function
or back to the stage. Every call to put a thing on the stage, send a clip
to a frame or ask for a sound named the art again.

**Why it is the stage's.** A stage is built from one set of art and can
play no other. The pair was never two things that might differ. Saying it
twice at every call was a chance to be wrong and nothing else.

**One copy, shared.** The stage holds the art by a counted reference, so
several stages can play from one copy of it: the tests read the art once
for a whole file of them, and the tool that draws a clip's frames makes a
stage for each. Nothing changes the art once it is loaded.

**What is still handed the art.** Only what has no stage: what reads the
words on the art's buttons, what picks a side's colours off a clip, and
what sets a match up before the first frame.

**The game is the same game.** The record of whole games plays out to the
frame as it did, and the pictures the engine draws are the same to the
byte.
