# 11. A swing begins on the frame that was seen

**What was decided.** A swing begins on the step of the pitch the player
was looking at when they clicked, not the later step the click reached the
game on. The window times every click against the frames it has handed to
the screen. The game keeps what the batting view showed on each of its last
thirty frames, and looks up the one the click was meant for. Whatever lies
between the screen and the player's hand that the window cannot see, the
screen lighting up, the mouse, the hand itself, is the player's *delay*. It
is measured on a page of its own, the last of the mods' pages, or set there
by hand, and kept between runs in `delay.toml` beside the scores.

**What it was before.** A click swung on whatever step the game had come to
when it next played a frame. A good deal lay between the picture and that:

- *The frame was played, then the window waited for the screen.* Each
  redraw played the frames that were due, then asked for a picture to draw
  them on, and waited for the screen to free one. Timed in the window, the
  wait was 13 to 16 of every 16.7 thousandths of a second. So what was
  drawn was most of a redraw old before it was even handed over, and was
  seen a redraw after that.
- *Clicks were not heard while it waited.* Nothing else happens in the
  window while it waits, so a click made then, which was most of the time,
  was taken in only once the wait and the drawing were over, and swung on
  the frame after that.
- *Then the screen and the mouse.* The screen takes a redraw or more to
  light up what it is handed, and a mouse a few thousandths of a second to
  say it was pressed. Nothing in the game can see either.

Together that came to two to four frames of sixty a second. On hard, a
swing meets the ball best from one step of the pitch only, a sixtieth of a
second, so a click made as the timing bar showed the marker on the green
almost always came too late.

**What is done now.**

- *The window waits for the screen first.* It asks for the picture, then
  plays the frames that are due and draws them at once: what is handed to
  the screen is a fraction of a millisecond old.
- *It sleeps until the screen is nearly ready.* The end of a wait for a
  picture is a moment the screen redrew, and the next is one redraw on. The
  window sleeps until two thousandths of a second before then, taking in
  clicks as they come, and waits only the last moment. Timed again, the
  wait is about one and a half thousandths of a second. A wait that ends at
  once means the window woke late and the clock is not known, and then it
  asks for the next picture at once, as before, which puts it right.
- *A click is timed.* The window knows when it handed each frame over and
  how often the screen redraws, so it knows when each frame was first to be
  seen, and tells the game when a click was made on that clock
  (`Pointer::went_down_at`). Its whole numbers are frames reaching the
  screen.
- *The swing begins where it was seen.* The game takes the frame showing
  the player's delay before the click was made, and begins the swing on
  the step that frame showed, with the ring where it was. The batter is
  played on as far into his swing as that, and if a swing begun then would
  have met the ball already, it meets it now, as well as it would have.
  The timing bar marks the swing where it was seen, and says how it was
  timed from there.

**How the delay is measured.** A marker crosses a bar at the pace the
timing bar's does, with a line where its best moment is, and the player
clicks as it reaches the line. Ten clicks make a measurement: how late each
came after the marker was shown on the line, and the middle one of those,
so that a click or two well off does not throw it out. If half of them are
more than three frames from the middle one, nothing is taken. A click made
as a frame comes up reaches the game about the delay later, wobbling by a
little either way, so the delay is set half a frame short of the middle
one, which keeps the click on that frame whichever way it wobbles.

**What stays as it was.** A click a script makes is not timed, and is
taken on the frame it reaches, as every click was before; the delay is not
used for it. So every seeded game a script plays goes as it did, and the
record of whole games did not change but for the page added to the list of
mods. Scripts can make a timed click, `click X Y late N`, to try a click
that reaches the game late.
