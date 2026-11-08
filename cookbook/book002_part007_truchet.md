# Truchet tiles, cut into smaller tiles

*Book 2, Pictures. Needs the core only (the word list `IMAGE`).*

The program: [`examples/book002_example009_truchet.fs`](../examples/book002_example009_truchet.fs).
Its output is beside it, in `book002_example009_truchet.out`. Run it with:

```
cargo run --release -- examples/book002_example009_truchet.fs
```

It writes `truchet.png` (256 by 256 pixels) in the current directory. It takes about 0.07
seconds (a release build, three runs) and between 3 and 4 million interpreter steps, so it also
fits in the playground's limit of 10 million: it is there as "Truchet", the same program
without the `SAVE-PNG` line, and the picture shows on the page. Add `SHOW` after `DRAW` to see the picture in a
terminal that shows 24-bit colour.

A Truchet tile is a square with two quarter circles in it, each around a corner, each joining
the middles of two edges. There are two ways to put them: around the top left and bottom right
corners, or around the top right and bottom left. Fill a floor with tiles, each turned at
random, and the arcs join into winding lines.

This picture is *multiscale*: a square is either one tile, or it is cut into four squares and
each of those is treated the same way. Big tiles and small tiles sit side by side. A program
that cuts a square into four squares is the best example of recursion there is.

## The recursion

```forth
: TILE  ( x y side depth -- )
  {: x y s d :}
  d SPLIT? IF
    s 2/ {: h :}
    x     y     h d 1+ RECURSE
    x h + y     h d 1+ RECURSE
    x     y h + h d 1+ RECURSE
    x h + y h + h d 1+ RECURSE
  ELSE
    x y s d LEAF
  THEN ;
```

`TILE` gets a square: its top left corner, its side and how deep it is (the whole picture is
depth 0). `SPLIT?` says whether to cut it. If so, `TILE` calls itself four times, once for each
quarter, with half the side and one level deeper. If not, the square is a leaf and `LEAF` draws
a tile in it.

Each call needs its own `x`, `y`, `s` and `d`. That is what the locals are for: every call to
`TILE` gets a fresh set, and when it returns, the call that was waiting finds its own numbers
again. Under the locals, the return stack keeps the way back: where in `TILE` to go on after
the call. The picture is cut at most 5 levels deep, so at most 6 calls wait at the same time.

`SPLIT?` has three rules. A square at the greatest depth (`MAX-DEPTH`, 5, a square of 8 pixels)
is never cut. The first two levels are always cut, so the picture does not become one big
tile. Below that, a throw of a three-sided die decides: two out of three cut.

```forth
: SPLIT?  ( depth -- f )
  {: d :}
  d MAX-DEPTH < 0= IF FALSE EXIT THEN
  d 2 < IF TRUE EXIT THEN
  3 RND 0<> ;
```

`RND` is a small linear generator with a fixed seed (the same one as in the Sierpinski
example), so the picture is always the same.

## A circle with whole numbers

A leaf needs two quarter circles. `CIRCLE` draws a whole circle with the midpoint algorithm: it
walks along one eighth of the circle, and each point it finds is mirrored to the other seven
(`8POINTS`). It only adds, subtracts and compares whole numbers; there is no square root and
no sine. The variable `err` says whether the next point is better one step to the side or one
step to the side and down.

`PUT` paints a point only when it lies inside the square of the leaf. So a circle around a
corner of the square leaves just a quarter of itself in the square, and no arc is drawn into
the neighbour. The circle does not know the
square; `PUT` is the one place that does.

A line one pixel thick is too thin for the big tiles. `ARC` draws a few circles with radii next to
each other, thick as the square is big (`s 16 / 1+` pixels), so a tile of 8 pixels has a line of 1
pixel and a tile of 128 pixels has one of 9.

## The leaf

```forth
2 RND IF
  x y s 2/ t ARC   x s + y s + s 2/ t ARC          \ top left and bottom right
ELSE
  x s + y s 2/ t ARC   x y s + s 2/ t ARC          \ top right and bottom left
THEN
```

Each circle has radius `s/2`, so it passes through the middle of two edges. The centres of the
circles are corners of the square: the corner `x + s` is one pixel outside the square, which is
why `PUT` has to clip. The colour follows the depth: big tiles dark red, small tiles pale
yellow (`DEPTH-INK`).

## What the program says

The output counts the leaves of each depth. There are none of depth 0 or 1 (those always
split), 7 squares of 64 pixels, 14 of 32, 18 of 16 and 280 of 8. The program adds up the areas
of all leaves and prints whether they cover the picture exactly (`yes`): a check that the
four quarters of each square are really the whole square. It also prints how many pixels were
painted (9184) and the result of `SAVE-PNG` (0).

Everything here is whole numbers and the seed is fixed, so these numbers are the same on every
machine.

## Try this

- Change `SPLIT?` to `2 RND 0<>` or to `4 RND 0<>`: a coin gives bigger tiles, a die with four
  sides gives mostly small ones.
- Change `MAX-DEPTH`. The line thickness follows the size of the tile, so deeper trees give
  a finer lace.
- Turn all tiles of one depth the same way, and see which patterns appear.
- Give `SPLIT?` a rule that depends on the place, for instance cut more near the middle of the
  picture.
