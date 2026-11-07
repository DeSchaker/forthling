# Game of Life: a world on the canvas

*Book 2, Pictures. Needs the core only (the word list `IMAGE`).*

The program: [`examples/book002_example002_life.fs`](../examples/book002_example002_life.fs).
Its output is beside it, in `book002_example002_life.out`. Run it with:

```
cargo run --release -- examples/book002_example002_life.fs
```

It prints the number of live cells every 10 generations and draws the last generation on the
canvas. Remove the `\` before `SHOW` at the end of the file to see the picture in the terminal, or
`ALSO WINDOW SHOW-WINDOW` with the program `forthling-window` (`cargo run --release
--manifest-path window/Cargo.toml -- examples/book002_example002_life.fs`) to see it in a
window. In the playground, pick *Life* under *Examples*: the picture shows below the
transcript. A run of 100 generations takes about 7.4 million steps and 0.12 seconds
(a release build), so it fits in the 10 million steps of the playground.

## The rules

Life is a grid of cells that are alive or dead. In each step, every cell looks at its eight
neighbours:

- a live cell with two or three live neighbours stays alive;
- a dead cell with exactly three live neighbours is born;
- every other cell is dead in the next step.

That is all. The start of this program is the *R-pentomino*, five cells:

```
. X X
X X .
. X .
```

On an endless board it takes 1103 generations to settle down. Here the board is 48 by 30
cells and wraps around at the edges, so a cell at the right edge has neighbours at the
left edge (a torus).

## The grid

A cell is one byte in memory: its *age*. 0 is dead, 1 was born in this step, 2 has lived two
steps, and so on. The age costs nothing and gives the picture its colours.

```forth
48 CONSTANT W   30 CONSTANT H   W H * CONSTANT N
CREATE GRID-A  N ALLOT   CREATE GRID-B  N ALLOT
VARIABLE NOW   VARIABLE NEXT
GRID-A NOW !   GRID-B NEXT !
: SWAP-GRIDS  NOW @ NEXT @  NOW ! NEXT ! ;
```

There are two grids. The world now is in `NOW`; the next world is built in `NEXT`. If the
program changed the grid it was reading, the first changes would be seen by the cells
after it, and the result would be wrong. When the next world is ready, `SWAP-GRIDS`
exchanges the two addresses, which costs two stores and no copying.

The edges wrap with `MOD`. In forthling `MOD` is floored, so `-1 W MOD` is `W - 1`, and `W W MOD` is 0.
`AT` turns a cell `(x, y)` into an address in a grid:

```forth
: AT  ( x y grid -- addr )  >R  H MOD W *  SWAP W MOD +  R> + ;
```

## Counting neighbours the other way round

The obvious way is that every cell looks at its eight neighbours and counts the live ones.
It works, but for each of the 1440 cells it does eight lookups with the wrapping, also where
the board is empty. The first version of this program did that, and 100 generations did
not fit in the 10 million steps of the playground.

So the program turns it round. Most of the board is empty, so it lets every *live* cell tell
its eight neighbours that it is there: it adds 1 to their count.

```forth
CREATE COUNTS  N ALLOT

: TELL-NEIGHBOURS  ( x y -- )
  {: x y :}
  2 -1 DO                                           \ J is the step in y: -1, 0, 1
    2 -1 DO                                         \ I is the step in x
      I J OR IF                                     \ (0, 0) is the cell itself
        x I +  y J +  COUNTS AT  DUP C@ 1+ SWAP C!
      THEN
    LOOP
  LOOP ;
```

`2 -1 DO` runs with `I` = -1, 0 and 1. `I J OR` is zero only for the step `(0, 0)`, the cell
itself, which must not count itself. The two loops are nested, so `J` is the index of the
outer loop. After this, `COUNTS` holds for every cell the number of its live neighbours.

## One generation

With the count and the age of a cell, the rules fit in one word:

```forth
: NEXT-AGE  ( count age -- age' )
  ?DUP IF                                           \ alive: stays with 2 or 3, otherwise dies
    SWAP DUP 2 = SWAP 3 = OR IF 1+ 254 MIN ELSE DROP 0 THEN
  ELSE                                              \ dead: born with exactly 3
    3 = IF 1 ELSE 0 THEN
  THEN ;
```

`?DUP` copies the age only if it is not 0, so the `IF` tests "alive", and in the live branch
both the count and the age are on the stack. The age stops at 254, so that it fits in a byte.

A generation is two passes. The first lets every live cell count itself in; the second
builds the next world:

```forth
: GENERATION  ( -- )
  COUNTS N ERASE
  N 0 DO  NOW @ I + C@ IF  I W /MOD TELL-NEIGHBOURS  THEN  LOOP
  N 0 DO  COUNTS I + C@  NOW @ I + C@  NEXT-AGE  NEXT @ I + C!  LOOP
  SWAP-GRIDS ;
```

`I W /MOD` turns the number of a cell back into `x` and `y`: the remainder is the column,
the quotient the row.

## The picture

`DRAW` paints one pixel per cell. The colour comes from the palette of the canvas, with the age as
index, so a new cell and an old cell have different colours:

```forth
: COLOUR  ( age -- colour )  DUP 0= IF EXIT THEN  4 * PALETTE@ ;
: DRAW  ( -- )
  H 0 DO W 0 DO
    I J NOW @ AT C@ COLOUR >R  I J R> PIXEL
  LOOP LOOP ;
```

`PALETTE@` takes any number and uses it modulo 256; entry 0 is black, which is also
what a dead cell gets.

## Checking it

The live cells counted by the program for generations 0 to 100 are the same as those of
an independent program in Python, written for the check (5, 11, 32, 27, 33, 64, 79, 53,
67, 98 and 109 for generations 0, 10, 20 and so on). That is the check that the rules and the
wrapping are right; `cargo test` compares the output with the file beside the program, so the text
here cannot drift from the code.

## Animated in the terminal

[`examples/book002_example004_life_animated.fs`](../examples/book002_example004_life_animated.fs)
has the same world and rules, but shows every generation: `DRAW` paints the canvas, `SHOW`
prints it, and `0 0 AT-XY` takes the cursor back to the top left, so the next picture is
printed over the last one. `MS` waits 50 milliseconds between two pictures. It shows 61
pictures (generation 0 to 60) and needs a terminal with 24-bit colour, such as Windows
Terminal:

```
cargo run --release -- examples/book002_example004_life_animated.fs
```

A picture of 48 by 30 pixels takes 15 lines, since `SHOW` prints two pixels in each
character. The file beside the program holds all the escape sequences, which is why it is large.
The window (`SHOW-WINDOW`) and the playground show one picture, so they cannot animate yet.

## Try it

- Make the board larger (`W` and `H`) and the run longer. The program then needs more
  than 10 million steps; use the command line, not the playground, with
  `--max-steps`.
- Change `SEED` to another pattern. A *glider* is `. X . / . . X / X X X`: five cells that
  walk over the board, and over the edge, for ever.
- Change `NEXT-AGE` so that a cell dies when it is older than some age.
