# 12. Batting practice is no kind of game

**What was decided.** A session of batting practice adds no kind of game. It
is a last innings or an arcade game, whichever view was chosen, with a
`Session` beside it on the match. The session keeps the pitches, and a few
named places ask whether there is one: the match then has no outcome, each
pitch begins afresh, the boards and the target come off the view, and a
list of mods of its own is in play.

**What it might have been.** A fourth `Mode`. But the point of practice is
that the batting is the game's own, and a session on the match's view wants
everything a last innings does, the strike zone, the calls and the fielders,
and on the arcade game's view everything that does, the ball over the field
and the next pitch on offer while it flies. A fourth kind would have had to
answer as one or the other in every place that asks, which is what the
tournament's decision (0010) turned down for the same reason.

**Why a pitch is written down where it is decided.** The two views end a play
in different places, and the arcade game's may never end one: the next pitch
can be called while the ball is up, and the view that was following it goes.
So what came of a pitch is worked out once, as the bat meets the ball or as
the ball goes by, with the sums the fielding itself goes by: `Ball::first_comes_to`
for where the ball is going and `Parts::is_foul` for whether it is fair. A
session therefore says nothing of what a fielder did with a ball. A fly that
was caught is told as how far it went, which is what is being practised.

**Why the view's boards come off.** They would say what the view's game has
come to, and a session has come to nothing of the kind. A last innings has a
target to show, and the one a session has is too large to mean anything,
being there so that no number of runs ends a play.

**What it costs.** Every game with no session goes as it did, which the record
of whole games was played to show before the menu changed. The price is the
few places that ask, each of them one line, and a row more on the menu's first
page, whose rows had been squeezed once already. Eight are about as many as
that page can take.
