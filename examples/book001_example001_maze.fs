\ book001_example001_maze.fs - make a maze with depth-first backtracking and solve it with
\ breadth-first search.   Run: cargo run --release -- examples/book001_example001_maze.fs
\ Needs the core only (COLLECTIONS); it also runs in the playground.
\
\ How it works:
\   1. The maze is a grid of squares. Rooms sit on the squares with odd coordinates; the squares
\      between them are walls, and breaking a wall joins two rooms.
\   2. MAKE-MAZE walks from room to room, always to a room not visited yet, breaking the wall in
\      between. At a dead end it steps back along the way it came (a stack). When it is back at
\      the start, every room has been visited, and there is exactly one way between two rooms.
\   3. SOLVE searches from the start in rings: all squares one step away, then two steps away,
\      and so on (a queue). The first time it reaches the goal, that is by a shortest way.
\   4. .MAZE prints the grid. S is the start, G is the goal, dots are the way found.
\ Stack comments like ( x y -- i ) show what a word takes and leaves; the last item is the top.

ALSO COLLECTIONS    \ VEC-VARIABLE, VEC+!, VEC@, VEC#, VEC-POP, VEC-CLEAR, VEC-FREE

\ --- sizes: the maze is CW x CH rooms; walls sit between them, so the grid is bigger ---
12 CONSTANT CW   6 CONSTANT CH
CW 2* 1+ CONSTANT GW   CH 2* 1+ CONSTANT GH   GW GH * CONSTANT GN   \ width, height, squares

\ Tables in memory. CREATE ... ALLOT reserves room: GN bytes, or GN cells (a cell holds a number).
CREATE GRID    GN ALLOT                \ one byte per square: 0 open, 1 wall, 2 on the path
CREATE DIST    GN CELLS ALLOT          \ steps from the start, -1 where we have not been
CREATE PARENT  GN CELLS ALLOT          \ the square we came from, to walk the way back
VEC-VARIABLE STACK                     \ the way back for the depth-first search
VEC-VARIABLE Q                         \ the queue for the breadth-first search
VARIABLE HEAD                          \ the front of Q

\ --- a small random number generator (a linear congruential one), so the maze is always the same ---
\ forthling has RANDOM too, but a generator of our own with a fixed seed gives the same maze on
\ every machine, so that the output can be checked by the tests.
VARIABLE SEED   2024 SEED !            \ change the seed for another maze
: RND  ( n -- r )  \ a number from 0 to n-1
  SEED @ 1103515245 * 12345 + 2147483647 AND   \ next seed: multiply, add, keep 31 bits
  DUP SEED !
  16 RSHIFT SWAP MOD ;                 \ the high bits are the better ones; MOD cuts to 0..n-1

\ --- squares ---
\ A square (x, y) is kept as one number i = y * GW + x, the place in the tables.
: POS    ( x y -- i )  GW * + ;
: UNPOS  ( i -- x y )  GW /MOD ;       \ /MOD gives the remainder (x) and then the quotient (y)
: CELL@  ( x y -- c )  POS GRID + C@ ;
: OPEN!  ( x y -- )    POS GRID + 0 SWAP C! ;   \ break down the wall on (x, y)

\ The four directions: 0 up, 1 right, 2 down, 3 left. A step is two squares: from room to room.
CREATE DXS   0 , 2 , 0 , -2 ,          \ the change in x for each direction
CREATE DYS  -2 , 0 , 2 , 0 ,           \ the change in y for each direction
: DX  ( d -- dx )  CELLS DXS + @ ;
: DY  ( d -- dy )  CELLS DYS + @ ;
: STEP  ( x y d -- x' y' )  >R R@ DY + SWAP R> DX + SWAP ;   \ the room next door in direction d

\ --- generate: depth-first backtracking ---
: INSIDE?    ( x y -- f )  1 GH 1- WITHIN SWAP 1 GW 1- WITHIN AND ;   \ not on the outer wall
: UNCARVED?  ( x y -- f )  2DUP INSIDE? IF CELL@ ELSE 2DROP 0 THEN ;  \ a room not visited yet

CREATE CAND 4 ALLOT                    \ the directions that lead to a room we have not visited
: CANDIDATES  ( x y -- n )             \ fills CAND, gives how many directions it holds
  {: x y :}  0                         \ {: x y :} takes x and y off the stack into local names
  4 0 DO                               \ for each direction I = 0 to 3 ...
    x y I STEP UNCARVED? IF  I OVER CAND + C!  1+  THEN   \ ... store it at CAND + count, count + 1
  LOOP ;

: GO  ( x y d -- )   \ break the wall in direction d and go on from the next room
  {: x y d :}
  x d DX 2/ +  y d DY 2/ +  OPEN!      \ the wall is one square away, half a step
  x y d STEP  2DUP OPEN!               \ the next room, now open
  POS STACK VEC+! ;                    \ put it on the stack: we are there now

: WALK  ( -- )
  BEGIN STACK VEC# WHILE               \ as long as the stack holds a room
    STACK VEC# 1- STACK VEC@ UNPOS     \ the room on top: x y
    2DUP CANDIDATES                    \ x y n: how many neighbours are new
    ?DUP IF                            \ at least one: take a random one
      RND CAND + C@ GO
    ELSE                               \ none: a dead end, step back
      2DROP STACK VEC-POP DROP
    THEN
  REPEAT ;

: MAKE-MAZE  ( -- )
  GRID GN 1 FILL   STACK VEC-CLEAR     \ everything is wall
  1 1 OPEN!  1 1 POS STACK VEC+!       \ start in the room at the top left
  WALK ;

\ --- solve: breadth-first search from the start ---
GW 2 - GH 2 - POS CONSTANT GOAL        \ the room at the bottom right
1 1 POS CONSTANT START
CREATE OFFS  GW NEGATE , 1 , GW , -1 ,       \ to the squares above, right, below, left (one step)

: EXPAND  ( i -- )   \ put the open squares next to i we have not seen on the queue
  4 0 DO
    DUP I CELLS OFFS + @ +                    ( i n )   \ n: the square next to i
    DUP GRID + C@ 0=                          \ open?
    OVER CELLS DIST + @ 0< AND IF             \ and not seen yet (distance -1)?
      OVER CELLS DIST + @ 1+  OVER CELLS DIST + !   \ distance of n = distance of i + 1
      2DUP CELLS PARENT + !                   \ n was reached from i
      DUP Q VEC+!                             \ n goes to the back of the queue
    THEN DROP
  LOOP DROP ;

: MARK-PATH  ( -- )  \ walk back from the goal along PARENT, writing 2 on the way
  GOAL BEGIN  DUP GRID + 2 SWAP C!  DUP START <> WHILE  CELLS PARENT + @  REPEAT DROP ;

: SOLVE  ( -- )
  GN 0 DO -1 I CELLS DIST + ! LOOP     \ no square seen yet
  Q VEC-CLEAR  0 HEAD !
  START Q VEC+!  0 START CELLS DIST + !   \ the start is 0 steps from itself
  BEGIN HEAD @ Q VEC# < WHILE          \ while there is a square at the front we have not handled
    HEAD @ Q VEC@  1 HEAD +!  EXPAND   \ take it, move the front on, look around
  REPEAT
  MARK-PATH ;

\ --- print: every square is two characters wide, so that it looks square ---
: SQUARE  ( c -- )  CASE  0 OF ."   " ENDOF  1 OF ." ##" ENDOF  ." .." ENDCASE ;
: .MAZE  ( -- )
  GH 0 DO                              \ J is the row ...
    GW 0 DO                            \ ... and I the column
      I J POS DUP START = IF DROP ." S " ELSE
        DUP GOAL = IF DROP ." G " ELSE GRID + C@ SQUARE THEN THEN
    LOOP CR
  LOOP ;

MAKE-MAZE
." A maze of " CW . ." x " CH . ." rooms:" CR CR
.MAZE CR
SOLVE
." Solved. The shortest way takes " GOAL CELLS DIST + @ . ." steps:" CR CR
.MAZE
STACK VEC-FREE  Q VEC-FREE             \ give the memory back
PREVIOUS
