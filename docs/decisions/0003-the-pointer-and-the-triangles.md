# 3. What is under the pointer is told from triangles, with no graphics card

**What was decided.** The meshes the art is cut into are kept by
`engine/src/meshes.rs`, which has nothing of the graphics card in it, and
which answers whether a point is inside a thing. The renderer keeps one and
sends a mesh to the card when it is first drawn. A game played with no
window keeps one on its own, and opens a graphics device only when a
picture is asked for.

**Why.** Every played test needs to know what its pointer is on, and none
needs a picture. Before, each had to open a graphics device to be told. Now
they run on a machine with no graphics card at all, and faster.

**What had to hold.** The answers had to be the same to the last bit, so
the cutting is the same code and the same triangles. The triangles of
everything in the art are part of the record (see 2).
