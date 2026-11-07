\ book002_example002_life.fs - Conway's Game of Life on the canvas.
\ Run: cargo run --release -- examples/book002_example002_life.fs
\ Needs the core only (IMAGE, the canvas). SHOW at the end prints the picture in a terminal, SHOW-WINDOW opens it in forthling-window.
\
\ How it works:
\   1. The world is a grid of W x H cells that wraps around at the edges (a torus). A cell
\      is alive or dead. We keep its age: 0 is dead, 1 is born in this step, and so on.
\   2. In each step the rules decide every cell from its eight neighbours: a live cell with
\      two or three live neighbours stays alive; a dead cell with exactly three is born;
\      all others are dead in the next step.
\   3. Instead of every cell looking at its neighbours, every LIVE cell adds 1 to the count of
\      each neighbour (TELL-NEIGHBOURS). Most of the board is empty, so this is quick. Then
\      one pass over the board turns count and age into the age in the next world, which
\      is built in a second grid. The two grids swap places.
\   4. DRAW paints each cell on the canvas, one pixel per cell, in a colour by age.
\ The start is the R-pentomino: five cells that make a lot of
\ trouble: on an endless board they take 1103 generations to settle down.

ALSO IMAGE

48 CONSTANT W   30 CONSTANT H   W H * CONSTANT N

CREATE GRID-A  N ALLOT   CREATE GRID-B  N ALLOT      \ one byte per cell: its age, 0 is dead
VARIABLE NOW   VARIABLE NEXT                         \ the addresses of the two grids
GRID-A NOW !   GRID-B NEXT !
: SWAP-GRIDS  NOW @ NEXT @  NOW ! NEXT ! ;

\ A cell (x, y); the edges wrap around: x = -1 is x = W-1, and x = W is x = 0. MOD is floored
\ in forthling, so -1 W MOD is W-1.
: AT      ( x y grid -- addr )  >R  H MOD W *  SWAP W MOD +  R> + ;

CREATE COUNTS  N ALLOT                               \ per cell: how many live neighbours it has

\ Step 2a: a live cell at (x, y) adds 1 to the count of each of the 8 cells around it.
: TELL-NEIGHBOURS  ( x y -- )
  {: x y :}
  2 -1 DO                                           \ J is the step in y: -1, 0, 1
    2 -1 DO                                         \ I is the step in x
      I J OR IF                                     \ (0, 0) is the cell itself
        x I +  y J +  COUNTS AT  DUP C@ 1+ SWAP C!
      THEN
    LOOP
  LOOP ;

\ Step 2b: the age of a cell in the next world, from its count and its age now.
: NEXT-AGE  ( count age -- age' )
  ?DUP IF                                           \ alive: stays with 2 or 3, otherwise dies
    SWAP DUP 2 = SWAP 3 = OR IF 1+ 254 MIN ELSE DROP 0 THEN
  ELSE                                              \ dead: born with exactly 3
    3 = IF 1 ELSE 0 THEN
  THEN ;

: GENERATION  ( -- )
  COUNTS N ERASE
  N 0 DO  NOW @ I + C@ IF  I W /MOD TELL-NEIGHBOURS  THEN  LOOP
  N 0 DO  COUNTS I + C@  NOW @ I + C@  NEXT-AGE  NEXT @ I + C!  LOOP
  SWAP-GRIDS ;

: POPULATION  ( -- n )
  0  N 0 DO NOW @ I + C@ 0<> IF 1+ THEN LOOP ;

\ --- the picture ---
: COLOUR  ( age -- colour )  DUP 0= IF EXIT THEN  4 * PALETTE@ ;
: DRAW  ( -- )
  H 0 DO W 0 DO
    I J NOW @ AT C@ COLOUR >R  I J R> PIXEL
  LOOP LOOP ;

\ --- start: the R-pentomino, near the middle ---
\    . X X
\    X X .
\    . X .
: SEED  ( -- )
  NOW @ N ERASE
  1 W 2 / 1+ H 2 / NOW @ AT C!        1 W 2 / 2 + H 2 / NOW @ AT C!
  1 W 2 / H 2 / 1+ NOW @ AT C!        1 W 2 / 1+ H 2 / 1+ NOW @ AT C!
  1 W 2 / 1+ H 2 / 2 + NOW @ AT C! ;

: .STATE  ( gen -- )  ." generation " 3 .R ."   live cells " POPULATION 3 .R CR ;

: LIFE  ( generations -- )
  W H CANVAS  SEED
  0 .STATE
  DUP 0 DO  GENERATION  I 1+ DUP 10 MOD 0= IF .STATE ELSE DROP THEN  LOOP
  DROP  DRAW ;

100 LIFE

PREVIOUS
