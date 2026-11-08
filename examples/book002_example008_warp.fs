\ book002_example008_warp.fs - value noise, summed over octaves, then domain warped.
\ Run: cargo run --release -- examples/book002_example008_warp.fs
\ Needs the core only (floating point, IMAGE). It writes noise.png, warp1.png and warp2.png
\ in the current directory; add SHOW after DRAW in DRAW-ONE to see a picture in a terminal.
\
\ How it works:
\   HASH turns a pair of whole numbers into a number from 0 to 255: a small, fixed mixing of bits.
\   NOISE gives a smooth value between 0 and 1 at any point: the hash at the four corners of the
\   square the point is in, blended with a smooth curve.
\   FBM adds NOISE at 4 sizes (octaves), each half as strong and twice as fine.
\   WARP moves a point by an amount that FBM gives, then the picture asks FBM at the moved
\   point:  g(p) = fbm(p + k * (fbm(p), fbm(p + offset))).  Do it twice for the swirls.
\ Only + - * / and FLOOR are used on floats, so every number is the same on every machine.

ALSO IMAGE

128 CONSTANT SIZE                                    \ the picture is SIZE by SIZE pixels
4 CONSTANT OCTAVES
2e FCONSTANT STRENGTH                                \ how far WARP moves a point (k)
4e FCONSTANT EXTENT                                  \ the picture shows 0..EXTENT in x and y

\ --- noise ---
\ The odd numbers are only there to mix the bits. The shifts bring the high bits down, so the
\ low 8 bits that we keep depend on all of the input.
: HASH  ( ix iy -- n )
  668265263 *  SWAP 374761393 * +
  DUP 13 RSHIFT XOR  1274126177 *
  DUP 16 RSHIFT XOR  255 AND ;

: LATTICE  ( ix iy -- ) ( F: -- v )  HASH S>F 255e F/ ;   \ a value 0..1 at a corner

: SMOOTH  ( F: t -- s )  FDUP FDUP F* FSWAP 2e F* 3e FSWAP F- F* ;   \ t*t*(3 - 2t)
: LERP  ( F: a b t -- v )  FROT FROT FOVER F- FROT F* F+ ;            \ a + t (b - a)

FVARIABLE FX   FVARIABLE FY

: NOISE  ( F: x y -- v )                             \ x and y are not negative
  FDUP FLOOR FDUP F>S {: iy :} F- SMOOTH FY F!
  FDUP FLOOR FDUP F>S {: ix :} F- SMOOTH FX F!
  ix iy LATTICE      ix 1+ iy LATTICE      FX F@ LERP
  ix iy 1+ LATTICE   ix 1+ iy 1+ LATTICE   FX F@ LERP
  FY F@ LERP ;

FVARIABLE PX   FVARIABLE PY   FVARIABLE SUM   FVARIABLE AMP

: FBM  ( F: x y -- v )                               \ octaves of noise, between 0 and 1
  PY F!  PX F!  0e SUM F!  0.5e AMP F!
  OCTAVES 0 DO
    PX F@ PY F@ NOISE  AMP F@ F*  SUM F@ F+  SUM F!
    PX F@ 2e F* PX F!   PY F@ 2e F* PY F!   AMP F@ 2e F/ AMP F!
  LOOP
  SUM F@  0.9375e F/ ;                               \ 0.5 + 0.25 + 0.125 + 0.0625

\ --- warp ---
FVARIABLE WX   FVARIABLE WY

: WARP  ( F: x y -- x' y' )
  WY F!  WX F!
  WX F@ WY F@ FBM                                    \ qx
  WX F@ 5.2e F+  WY F@ 1.3e F+  FBM                  \ qx qy   the second noise, found elsewhere
  STRENGTH F* WY F@ F+                               \ qx y'
  FSWAP STRENGTH F* WX F@ F+                         \ y' x'
  FSWAP ;                                            \ x' y'

\ How many warps the picture gets: 0, 1 or 2.
: VALUE-AT  ( warps -- ) ( F: x y -- v )
  0 ?DO WARP LOOP  FBM ;

\ --- the picture ---
CREATE VALUES  SIZE SIZE * FLOATS ALLOT              \ one value per pixel, row by row
FVARIABLE LO   FVARIABLE HI

: >COORD  ( n -- ) ( F: -- r )  S>F SIZE S>F F/ EXTENT F* ;

: FILL  ( warps -- )
  1e LO F!  0e HI F!
  SIZE 0 DO
    SIZE 0 DO
      I >COORD  J >COORD  DUP VALUE-AT
      FDUP LO F@ FMIN LO F!   FDUP HI F@ FMAX HI F!
      VALUES J SIZE * I + FLOATS +  F!
    LOOP
  LOOP DROP ;

\ The values are mostly in the middle of 0..1, so stretch them to use the whole range.
: >LEVEL  ( F: v -- ) ( -- n )  LO F@ F-  HI F@ LO F@ F-  F/  255e F*  F>S ;

\ From dark blue, through teal, to a pale yellow: levels 0 to 255.
: INK  ( v -- colour )
  {: v :}
  v v * 255 /  16 LSHIFT                             \ red comes late
  v 255 + v * 510 /  8 LSHIFT OR                     \ green in between
  60 v 195 * 255 / + OR ;                            \ blue from the start

: DRAW  ( -- )
  SIZE SIZE CANVAS
  SIZE 0 DO
    SIZE 0 DO
      VALUES J SIZE * I + FLOATS + F@ >LEVEL INK  I J ROT PIXEL
    LOOP
  LOOP ;

\ A checksum of the levels, so the output says something about the picture itself.
: CHECKSUM  ( -- n )
  0  SIZE SIZE * 0 DO  VALUES I FLOATS + F@ >LEVEL  I 1+ * +  LOOP ;

: DRAW-ONE  ( warps c-addr u -- )
  2>R FILL
  ." lowest " LO F@ 1000e F* F>S .  ." highest " HI F@ 1000e F* F>S . CR
  ." checksum " CHECKSUM . CR
  DRAW
  2R> 2DUP TYPE ." : ior " SAVE-PNG . CR ;

0 S" noise.png" DRAW-ONE                             \ the noise as it is
1 S" warp1.png" DRAW-ONE                             \ warped once
2 S" warp2.png" DRAW-ONE                             \ warped twice: swirls

PREVIOUS
