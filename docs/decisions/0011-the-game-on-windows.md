# 11. The game on Windows

**What was decided.** The game is built and played on Windows from the same
code as on a Mac. What differs between the two is where the player's things
are kept, how the app is put together, and the record of whole games.
Nothing in the play knows which it is on.

**Where things are kept.** A Mac keeps a player's things under
`~/Library/Application Support`, and Windows in the folder that `APPDATA`
names. The game's folder is named by its identifier on both, and
`locate::own_folder` is the one place that knows the difference: the
scores, the choice of mods, the tournament and a copy of the art are all
found from it.

**What the app is.** On a Mac the app is a bundle with the art inside it.
On Windows it is a folder with the program and the art side by side, which
can be put anywhere, and the program is built with the `app` feature: it
opens no console window of its own, and puts up a dialog to say why it
could not start. A plain `cargo run` is still a console program, so
`--run` and the scripts print as they always did. The icon is drawn from
`assets/icon.svg` as the app is built, on both, and on Windows it is built
into the program, whose window takes it from there.

**Why the C runtime is built in.** A Rust program on Windows otherwise
leans on Microsoft's runtime for C++, which a new machine may not have, and
on whichever version of Windows' own C runtime the machine has, whose
mathematics is some of what the record sums. Built in, the app needs
nothing put on the machine first, and the mathematics a test is played by
comes with the program and not with the machine. `.cargo/config.toml` says
so for every program built for Windows, the tests' as well.

**Why the window waits for the screen.** The pace plays a frame of the game
every so many redraws, and counts on each redraw being one of the screen's.
A Mac's card offers that first. Windows' offers first to draw as fast as it
can and show the newest picture, which drew some three hundred a second on
a screen of sixty, never in step with it, and the game juddered. The window
now asks for a picture that waits for the screen, which every card has.

**Why the dialog is a crate's.** No code here may be unsafe, and Windows'
own call for a message box is. `rfd` puts one up safely, and is built for
Windows only. The Mac's alert was already put up by a program the system
has, and was left as it was.

**Why the scripts are still bash.** Git for Windows comes with bash, and so
do GitHub's Windows machines, so `scripts/check.sh` and the rest run there
as they are. Written a second time for PowerShell, the two would drift
apart. Only the building of the app is PowerShell, since a Windows app is
put together with Windows' own tools, as the Mac's is with the Mac's.

**Why every file has plain line feeds.** Git for Windows turns the end of
each line into two characters as it checks a file out, unless it is told
not to. Bash will not run a script written so, and the record of whole
games, which sums what the game reads, would sum other bytes.
`.gitattributes` tells it not to.

**What it cost the record.** Every game of the record goes on Windows as it
went on a Mac in all that was said and heard, frame for frame, and ends on
the same frame. What was seen does not always agree: in the zinger hit's
games, in the tournaments with every mod on and in a few others, and in 29
of the 1,598 sums of the triangles the art is cut into, the machine's own
mathematics comes out differently in the last place and a thing is drawn a
hair from where it was. So the record is written down for Windows on its
own, beside the Mac's, as decision 2 allows for. A change that is meant to alter how the game plays writes both down
afresh. Whoever has only one kind of machine to hand can have GitHub's
write down both, with the workflow in `.github/workflows/write-down.yml`.
