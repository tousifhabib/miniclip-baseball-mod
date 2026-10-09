# 4. The play asks the mods in play, and never whether a mod is on

**What was decided.** Each mod has a file of its own with what it keeps,
its rules and what it draws. A match holds the mods that are on for it in
`ModsInPlay`. The rest of the play asks a question named for what it wants
to know, or says what has happened, and one function for each names the
mods with a say in it. A mod that is off is not there.

**Why.** There were forty-odd places that asked whether this mod or that
was switched on, and thirty of the match's fields that belonged to one mod
or another. To know what a mod did, the whole of the play had to be read,
and no rule of a mod could be tested without playing a game.

**Why not a list of mods called in turn.** That is the usual shape for
things that plug in, and it was looked at first. It needs one order that
suits every question, and there is none: what a run is worth, the lines in
the corner of the view and the line that says how the game stands each take
the mods in an order of their own (see 1 for why the order cannot be
changed to suit). So each question lists its mods itself. It costs a line
per question when a mod is added, and the order can be read where it
matters.

**Why a mod's rules hand back small things.** A mod says what line to
write, what colour to use or what the pitch's numbers become, and the
caller puts it on the stage there and then (see 5). The rules can then be
tested as sums, and are.
