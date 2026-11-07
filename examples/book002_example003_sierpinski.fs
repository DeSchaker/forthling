\ book002_example003_sierpinski.fs - the Sierpinski triangle, made in two ways on the canvas.
\ Run: cargo run --release -- examples/book002_example003_sierpinski.fs
\ Needs the core only (IMAGE, the canvas). It writes pascal.png and chaos.png in the current
\ directory; add SHOW after a drawing to see it in a terminal.
\
\ How it works:
\   1. PASCAL: row n of Pascal's triangle has the numbers C(n,0) ... C(n,n). C(n,k) is odd
\      exactly when every 1 bit of k is also a 1 bit of n, that is, when k AND n = k. Paint
\      the odd numbers and the Sierpinski triangle appears. 128 rows have 3^7 = 2187 of them.
\   2. CHAOS: start anywhere. Pick one of three corners at random, jump half way to it,
\      paint the spot, and do it again. The spots end up on the triangle.

ALSO IMAGE

128 CONSTANT SIZE   16777215 CONSTANT WHITE        \ WHITE is 0xFFFFFF

\ --- 1. Pascal's triangle modulo 2 ---
: ODD?  ( n k -- f )  TUCK AND = ;                 \ k AND n = k
: PASCAL  ( -- count )
  SIZE SIZE CANVAS  0
  SIZE 0 DO
    I 1+ 0 DO
      J I ODD? IF  I J WHITE PIXEL  1+  THEN       \ J is the row n, I the place k in it
    LOOP
  LOOP ;

\ --- 2. the chaos game ---
VARIABLE SEED   2024 SEED !                        \ a fixed seed, so the picture is always the same
: RND  ( n -- r )  \ a number from 0 to n-1
  SEED @ 1103515245 * 12345 + 2147483647 AND  DUP SEED !  16 RSHIFT SWAP MOD ;

CREATE CORNER-X   0 , SIZE 1- , SIZE 2/ ,          \ the three corners of the triangle
CREATE CORNER-Y   SIZE 1- , SIZE 1- , 0 ,

: CHAOS  ( jumps -- lit )
  SIZE SIZE CANVAS
  SIZE 2/ SIZE 2/ 0 {: x y c :}  0 SWAP            \ lit pixels so far, then the jump counter
  0 DO
    3 RND CELLS TO c
    x CORNER-X c + @ + 2/ TO x                     \ half way to the corner
    y CORNER-Y c + @ + 2/ TO y
    x y PIXEL@ 0= IF  1+  THEN                     \ a new spot?
    x y WHITE PIXEL
  LOOP ;

PASCAL  ." pascal: " . ." odd numbers (3^7 = " 3 3 * 3 * 3 * 3 * 3 * 3 * . ." )" CR
S" pascal.png" SAVE-PNG  ." pascal.png: ior " . CR

30000 CHAOS  ." chaos: " . ." lit pixels" CR
S" chaos.png" SAVE-PNG  ." chaos.png: ior " . CR

PREVIOUS
