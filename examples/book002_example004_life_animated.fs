\ book002_example004_life_animated.fs - Conway's Game of Life, animated in the terminal.
\ Run: cargo run --release -- examples/book002_example004_life_animated.fs
\ Needs the core only (IMAGE, the canvas), and a terminal with 24-bit colour (Windows Terminal has it).
\
\ The same world and rules as book002_example002_life.fs (read that first). The difference is
\ at the end: instead of one picture after 100 generations, every generation is drawn on the
\ canvas, shown with SHOW, and the cursor goes back to the top left (AT-XY) so the next
\ picture is drawn over the last one. MS waits a little between the pictures.
\ SHOW prints two pixels in each character, so a 48 x 30 canvas takes 15 lines.

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

\ --- the animation ---
60 CONSTANT FRAMES                                  \ how many pictures
50 CONSTANT PAUSE                                   \ milliseconds between two pictures

: FRAME  ( -- )  DRAW  0 0 AT-XY  SHOW ;

: ANIMATE  ( -- )
  W H CANVAS  SEED  PAGE
  FRAMES 0 DO  FRAME  PAUSE MS  GENERATION  LOOP
  FRAME ;

ANIMATE

PREVIOUS
