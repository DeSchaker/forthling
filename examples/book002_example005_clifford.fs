\ book002_example005_clifford.fs - the Clifford strange attractor, drawn by density on the canvas.
\ Run: cargo run --release -- examples/book002_example005_clifford.fs
\ Needs the core only (floating point, IMAGE). It writes clifford.png in the current directory;
\ add SHOW after the drawing to see it in a terminal.
\
\ How it works:
\   The point (x, y) jumps again and again:
\       x' = sin(a y) + c cos(a x)        y' = sin(b x) + d cos(b y)
\   The jumps never settle, but they stay inside a small area and keep returning to the same
\   curves. Painting each spot gives a dotted picture; instead every pixel counts how often it
\   was hit, and the colour follows the logarithm of that count, so thin curves and dense
\   places both show, like smoke. Only (x, y) lives on the float stack.

ALSO IMAGE

160 CONSTANT SIZE                                    \ the picture is SIZE by SIZE pixels
100000 CONSTANT JUMPS
-1.4e FCONSTANT A   1.6e FCONSTANT B   1.0e FCONSTANT C   0.7e FCONSTANT D

CREATE HITS  SIZE SIZE * CELLS ALLOT                 \ hits per pixel, row by row
HITS SIZE SIZE * CELLS ERASE

\ One jump. FOVER FOVER copies the pair; FROT FROT brings the new x to the bottom; FSWAP puts
\ the right number on top before each product.
: STEP  ( F: x y -- x' y' )
  FOVER FOVER                                        \ x y x y
  A F* FSIN  FSWAP  A F* FCOS C F*  F+               \ x y x'      x' = sin(a y) + c cos(a x)
  FROT FROT                                          \ x' x y
  FSWAP B F* FSIN  FSWAP B F* FCOS D F*  F+ ;        \ x' y'       y' = sin(b x) + d cos(b y)

\ The attractor lies between -2.1 and 2.1 in both directions (|x| <= 1 + c, |y| <= 1 + d).
: >PIXEL  ( F: r -- ) ( -- n )  2.1e F+  4.2e F/  SIZE S>F F*  F>S ;

: HIT  ( F: x y -- x y )                             \ count one visit of the point
  FOVER FOVER  >PIXEL >PIXEL  {: row col :}          \ y first, so its pixel is under x's
  row 0 SIZE WITHIN  col 0 SIZE WITHIN AND IF
    1  HITS row SIZE * col + CELLS +  +!
  THEN ;

: ITERATE  ( -- )  0.1e 0.1e  JUMPS 0 DO STEP HIT LOOP  FDROP FDROP ;

\ --- the picture ---
: MAXHIT  ( -- n )  0  SIZE SIZE * 0 DO HITS I CELLS + @ MAX LOOP ;
: TOTAL  ( -- n )   0  SIZE SIZE * 0 DO HITS I CELLS + @ + LOOP ;
: LIT  ( -- n )     0  SIZE SIZE * 0 DO HITS I CELLS + @ 0<> - LOOP ;  \ a flag is -1

\ sin and cos differ in the last bits from one machine to another, and the jumps magnify that, so
\ the exact counts are not the same everywhere. The output below holds only what is the same:
\ every jump is counted, and the picture has a few dense places and much empty space.
FALSE VALUE SHOW-NUMBERS                             \ set it to TRUE to print the counts
: YES/NO  ( f -- )  IF ." yes" ELSE ." no" THEN CR ;
: NUMBERS  ( -- )
  SHOW-NUMBERS IF
    ." most hits in one pixel: " MAXHIT . CR
    ." pixels hit: " LIT . ." of " SIZE SIZE * . CR
  THEN ;

\ A brightness of 0 to 255 from a count and the largest count, on a logarithmic scale.
: BRIGHT  ( n max -- v )
  1+ S>F FLN  1+ S>F FLN  FSWAP F/  255e F*  F>S ;

\ A cold white: red a little less than green, green a little less than blue.
: SMOKE  ( v -- colour )
  {: v :}  v 3 * 4 /  16 LSHIFT  v 7 * 8 / 8 LSHIFT OR  v OR ;

: DRAW  ( -- )
  SIZE SIZE CANVAS  MAXHIT {: top :}
  SIZE 0 DO
    SIZE 0 DO
      HITS J SIZE * I + CELLS + @
      DUP 0= IF DROP ELSE top BRIGHT SMOKE I J ROT PIXEL THEN
    LOOP
  LOOP ;

ITERATE
." jumps: " JUMPS . ." counted: " TOTAL . CR
." some pixel was hit over 100 times: " MAXHIT 100 > YES/NO
." under half of the pixels were hit: " LIT SIZE SIZE * 2/ < YES/NO
NUMBERS
DRAW
S" clifford.png" SAVE-PNG  ." clifford.png: ior " . CR

PREVIOUS
