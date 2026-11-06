\ book001_example001_maze.fs - make a maze with depth-first backtracking and solve it with
\ breadth-first search.   Run: cargo run --release -- examples/book001_example001_maze.fs
\ Needs the core only (COLLECTIONS); it also runs in the playground.

ALSO COLLECTIONS

\ --- sizes: the maze is CW x CH rooms; walls sit between them, so the grid is bigger ---
12 CONSTANT CW   6 CONSTANT CH
CW 2* 1+ CONSTANT GW   CH 2* 1+ CONSTANT GH   GW GH * CONSTANT GN

CREATE GRID    GN ALLOT                \ one byte per square: 0 open, 1 wall, 2 on the path
CREATE DIST    GN CELLS ALLOT          \ steps from the start, -1 where we have not been
CREATE PARENT  GN CELLS ALLOT          \ the square we came from
VEC-VARIABLE STACK                     \ the way back for the depth-first search
VEC-VARIABLE Q                         \ the queue for the breadth-first search
VARIABLE HEAD                          \ the front of Q

\ --- a small random number generator (a linear congruential one), so the maze is always the same ---
VARIABLE SEED   2024 SEED !
: RND  ( n -- r )  \ a number from 0 to n-1
  SEED @ 1103515245 * 12345 + 2147483647 AND DUP SEED !  16 RSHIFT SWAP MOD ;

\ --- squares ---
: POS    ( x y -- i )  GW * + ;
: UNPOS  ( i -- x y )  GW /MOD ;
: CELL@  ( x y -- c )  POS GRID + C@ ;
: OPEN!  ( x y -- )    POS GRID + 0 SWAP C! ;

\ The four directions: up, right, down, left; two squares at a time while we carve.
CREATE DXS   0 , 2 , 0 , -2 ,
CREATE DYS  -2 , 0 , 2 , 0 ,
: DX  ( d -- dx )  CELLS DXS + @ ;
: DY  ( d -- dy )  CELLS DYS + @ ;
: STEP  ( x y d -- x' y' )  >R R@ DY + SWAP R> DX + SWAP ;

\ --- generate: depth-first backtracking ---
: INSIDE?    ( x y -- f )  1 GH 1- WITHIN SWAP 1 GW 1- WITHIN AND ;
: UNCARVED?  ( x y -- f )  2DUP INSIDE? IF CELL@ ELSE 2DROP 0 THEN ;

CREATE CAND 4 ALLOT                    \ the directions that lead to a room we have not visited
: CANDIDATES  ( x y -- n )
  {: x y :}  0
  4 0 DO  x y I STEP UNCARVED? IF  I OVER CAND + C!  1+  THEN  LOOP ;

: GO  ( x y d -- )   \ break the wall in direction d and go on from the next room
  {: x y d :}
  x d DX 2/ +  y d DY 2/ +  OPEN!
  x y d STEP  2DUP OPEN!  POS STACK VEC+! ;

: WALK  ( -- )
  BEGIN STACK VEC# WHILE
    STACK VEC# 1- STACK VEC@ UNPOS  2DUP CANDIDATES
    ?DUP IF  RND CAND + C@ GO  ELSE  2DROP STACK VEC-POP DROP  THEN
  REPEAT ;

: MAKE-MAZE  ( -- )
  GRID GN 1 FILL   STACK VEC-CLEAR
  1 1 OPEN!  1 1 POS STACK VEC+!  WALK ;

\ --- solve: breadth-first search from the start ---
GW 2 - GH 2 - POS CONSTANT GOAL
1 1 POS CONSTANT START
CREATE OFFS  GW NEGATE , 1 , GW , -1 ,       \ to the squares above, right, below, left

: EXPAND  ( i -- )   \ put the open squares next to i we have not seen on the queue
  4 0 DO
    DUP I CELLS OFFS + @ +                    ( i n )
    DUP GRID + C@ 0=  OVER CELLS DIST + @ 0< AND IF
      OVER CELLS DIST + @ 1+  OVER CELLS DIST + !
      2DUP CELLS PARENT + !
      DUP Q VEC+!
    THEN DROP
  LOOP DROP ;

: MARK-PATH  ( -- )  \ walk back from the goal along PARENT
  GOAL BEGIN  DUP GRID + 2 SWAP C!  DUP START <> WHILE  CELLS PARENT + @  REPEAT DROP ;

: SOLVE  ( -- )
  GN 0 DO -1 I CELLS DIST + ! LOOP
  Q VEC-CLEAR  0 HEAD !
  START Q VEC+!  0 START CELLS DIST + !
  BEGIN HEAD @ Q VEC# < WHILE
    HEAD @ Q VEC@  1 HEAD +!  EXPAND
  REPEAT
  MARK-PATH ;

\ --- print: every square is two characters wide, so that it looks square ---
: SQUARE  ( c -- )  CASE  0 OF ."   " ENDOF  1 OF ." ##" ENDOF  ." .." ENDCASE ;
: .MAZE  ( -- )
  GH 0 DO
    GW 0 DO
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
STACK VEC-FREE  Q VEC-FREE
PREVIOUS
