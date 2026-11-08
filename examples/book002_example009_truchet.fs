\ book002_example009_truchet.fs - multiscale Truchet tiling, with a recursive quadtree.
\ Run: cargo run --release -- examples/book002_example009_truchet.fs
\ Needs the core only (IMAGE, the canvas). It writes truchet.png in the current directory;
\ add SHOW after DRAW to see the picture in a terminal.
\
\ How it works:
\   TILE gets a square. If it is not too deep and a random bit says so, it cuts the square
\   into four and calls itself on each quarter (RECURSE). Otherwise the square is a leaf and
\   gets one of two tiles: two quarter circles around two opposite corners, joining the
\   middles of the edges. The two choices are the two ways to put the corners.
\   ARC draws a circle with the midpoint algorithm, whole numbers only, and paints only the
\   points that fall inside the square, so a circle around a corner leaves a quarter circle.
\ Everything is whole numbers and a fixed seed, so the picture is the same on every machine.

ALSO IMAGE

256 CONSTANT SIZE                                    \ the picture is SIZE by SIZE pixels
5 CONSTANT MAX-DEPTH                                 \ the smallest square is SIZE / 2^5 = 8

VARIABLE SEED   2024 SEED !                          \ a fixed seed, so the picture is always the same
: RND  ( n -- r )  \ a number from 0 to n-1
  SEED @ 1103515245 * 12345 + 2147483647 AND  DUP SEED !  16 RSHIFT SWAP MOD ;

\ A colour for each depth: big squares dark red, small ones pale yellow.
: DEPTH-INK  ( depth -- colour )
  {: d :}  120 d 50 * + 255 MIN   40 d 40 * + 255 MIN   60 d 20 * + 255 MIN  RGB ;

\ --- a circle around (cx, cy), painted only inside the square (x0, y0, side) ---
VARIABLE X0   VARIABLE Y0   VARIABLE SIDE   VARIABLE COLOUR
: PUT  ( x y -- )                                    \ paint one point if it is in the square
  {: x y :}
  x X0 @ >=  x X0 @ SIDE @ + <  AND
  y Y0 @ >=  y Y0 @ SIDE @ + <  AND AND
  IF x y COLOUR @ PIXEL THEN ;

: 8POINTS  ( cx cy dx dy -- )                        \ the eight mirror images of one point
  {: cx cy dx dy :}
  cx dx + cy dy + PUT   cx dx - cy dy + PUT   cx dx + cy dy - PUT   cx dx - cy dy - PUT
  cx dy + cy dx + PUT   cx dy - cy dx + PUT   cx dy + cy dx - PUT   cx dy - cy dx - PUT ;

: CIRCLE  ( cx cy r -- )                             \ the midpoint circle algorithm
  {: cx cy r :}
  r 0 1 r - {: dx dy err :}                          \ start at the top of the circle
  BEGIN dx dy >= WHILE
    cx cy dx dy 8POINTS
    dy 1+ TO dy
    err 0< IF  err 2 dy * 1+ + TO err
    ELSE       dx 1- TO dx   err 2 dy dx - * 1+ + TO err
    THEN
  REPEAT ;

\ A ring: circles with radius r - t/2 ... r - t/2 + t - 1, so the line is t pixels thick.
: ARC  ( cx cy r t -- )
  {: cx cy r t :}
  t 0 DO  cx cy r t 2/ - I + CIRCLE  LOOP ;

\ --- the leaves ---
CREATE LEAVES   MAX-DEPTH 1+ CELLS ALLOT             \ how many squares of each depth
LEAVES MAX-DEPTH 1+ CELLS ERASE
VARIABLE TURNED                                      \ how many got the second tile

: LEAF  ( x y side depth -- )
  {: x y s d :}
  1 LEAVES d CELLS + +!
  x X0 !  y Y0 !  s SIDE !  d DEPTH-INK COLOUR !
  s 16 / 1+  {: t :}
  2 RND IF
    x y s 2/ t ARC   x s + y s + s 2/ t ARC          \ corners top-left and bottom-right
  ELSE
    1 TURNED +!
    x s + y s 2/ t ARC   x y s + s 2/ t ARC          \ corners top-right and bottom-left
  THEN ;

\ A square splits while it is not too deep; the first two levels always split, then a die: two out of three split.
: SPLIT?  ( depth -- f )
  {: d :}
  d MAX-DEPTH < 0= IF FALSE EXIT THEN
  d 2 < IF TRUE EXIT THEN
  3 RND 0<> ;

\ The recursion: each call keeps its own x y side depth, and the return stack remembers where
\ to go on after the four quarters.
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

\ --- the picture ---
: DRAW  ( -- )
  SIZE SIZE CANVAS
  0 0 SIZE 0 TILE ;

\ The squares of one level are SIZE / 2^depth wide; together they must cover the picture.
: AREA  ( -- n )
  0  MAX-DEPTH 1+ 0 DO
    LEAVES I CELLS + @  SIZE I RSHIFT DUP *  *  +
  LOOP ;

: LIT  ( -- n )                                       \ pixels that are not black
  0  SIZE 0 DO SIZE 0 DO  J I SWAP PIXEL@ 0<> IF 1+ THEN  LOOP LOOP ;

: REPORT  ( -- )
  MAX-DEPTH 1+ 0 DO  ." depth " I . ." : " LEAVES I CELLS + @ . ." squares" CR  LOOP
  ." the squares cover the picture: " AREA SIZE DUP * = IF ." yes" ELSE ." NO" THEN CR
  ." pixels painted: " LIT . CR ;

DRAW  REPORT
S" truchet.png" SAVE-PNG  ." truchet.png: ior " . CR

PREVIOUS
