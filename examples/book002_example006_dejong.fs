\ book002_example006_dejong.fs - the Peter de Jong attractor, drawn three times with other numbers.
\ Run: cargo run --release -- examples/book002_example006_dejong.fs
\ Needs the core only (floating point, IMAGE). It writes dejong1.png, dejong2.png and dejong3.png
\ in the current directory; add SHOW after DRAW-ONE to see a picture in a terminal.
\
\ How it works:
\   The point (x, y) jumps again and again:
\       x' = sin(a y) - cos(b x)          y' = sin(c x) - cos(d y)
\   It is the Clifford attractor of book002_example005_clifford.fs with the same counting of
\   hits and the same smoke colours; only the jump is new, and the four numbers a b c d are now
\   given to each picture (on the float stack), not fixed in constants.

ALSO IMAGE

160 CONSTANT SIZE                                    \ the picture is SIZE by SIZE pixels
100000 CONSTANT JUMPS

FVARIABLE A   FVARIABLE B   FVARIABLE C   FVARIABLE D

CREATE HITS  SIZE SIZE * CELLS ALLOT                 \ hits per pixel, row by row

\ One jump. FOVER FOVER copies the pair; FROT FROT brings the new x to the bottom; FSWAP puts
\ the right number on top before each product.
: STEP  ( F: x y -- x' y' )
  FOVER FOVER                                        \ x y x y
  A F@ F* FSIN  FSWAP  B F@ F* FCOS  F-              \ x y x'      x' = sin(a y) - cos(b x)
  FROT FROT                                          \ x' x y
  FSWAP C F@ F* FSIN  FSWAP D F@ F* FCOS  F- ;       \ x' y'       y' = sin(c x) - cos(d y)

\ Both sin - cos are between -2 and 2, so the attractor always fits in a square from -2 to 2.
: >PIXEL  ( F: r -- ) ( -- n )  2e F+  4e F/  SIZE S>F F*  F>S ;

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
\ every jump is counted. Set SHOW-NUMBERS to TRUE to print the counts as well.
FALSE VALUE SHOW-NUMBERS
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

\ One picture from the four numbers on the float stack and a file name.
: DRAW-ONE  ( c-addr u -- ) ( F: a b c d -- )
  D F!  C F!  B F!  A F!
  HITS SIZE SIZE * CELLS ERASE
  ITERATE
  ." jumps: " JUMPS . ." counted: " TOTAL . CR
  NUMBERS
  DRAW
  2DUP TYPE ." : ior " SAVE-PNG . CR ;

1.4e -2.3e 2.4e -2.1e   S" dejong1.png" DRAW-ONE    \ soft curves
2.01e -2.53e 1.61e -0.33e  S" dejong2.png" DRAW-ONE \ a different shape, with sharp edges
1.4e -2.3e -2.4e -2.1e  S" dejong3.png" DRAW-ONE    \ the first, with the sign of c changed

PREVIOUS
