# 7. A file for each job

**What was decided.** A file of code does one thing, and its first lines
say what. No file runs to more than three hundred lines, and
`scripts/check.sh` fails on one that does. Where a part of the game is more
than one file's worth it is a folder, named as the one file was, with a
file in it for each of its jobs.

**Why.** The match was once a file of two thousand lines, and the fielding
one of fourteen hundred. Everything in them was in good order line by line,
and still nobody could say where a thing would be without searching. A
name in a list of files is the cheapest map there is: the fielding now
reads as the loose ball, the chase, the throw, the runners and the end of
the play before a line of it has been opened.

**What a folder hands on.** A folder's `mod.rs` holds what the whole part
is about and hands on the names the rest of the game uses, so that nothing
outside it has to know how it is cut up. What its files share among
themselves is seen by the folder and no further.

**Tests sit beside what they test.** Short ones are at the foot of the
file. Where they would crowd the code they have a file of their own beside
it, `tests.rs` for cases written out and `properties.rs` for statements
tried against cases made up by the hundred. A long file of played tests is
a folder too, still one program, with the helpers it shares in its
`main.rs`.

**Said once.** What two files both need is in one place that both reach
for: the helpers the played tests share are in `tests/common`, each
constant is in the file that uses it, or in the folder's `mod.rs` when
several do.

**The limit is a backstop, not the rule.** A file is cut where its jobs
part, not where the count runs out. Most are well under half the limit. A
file near it that still does one thing is left alone.
