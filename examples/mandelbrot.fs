\ mandelbrot.fs - the Mandelbrot set and a Julia set as pictures, in fixed-point
\ numbers (Q32.32) on a canvas.   Run: cargo run -- examples/mandelbrot.fs
\ It writes mandelbrot.ppm and julia.png in the current directory.

ALSO FIXED  ALSO IMAGE
32 FX-BITS  TRUE FX-ROUND-LITERALS   \ literals such as -2.2fx are rounded to 2^-32

160 CONSTANT W   96 CONSTANT H   100 CONSTANT LIMIT

\ The iterations z := z*z + c takes before |z| passes 2, at most LIMIT.
: ESCAPE  ( zr zi cr ci -- n )
  0 {: zr zi cr ci n :}
  BEGIN
    n LIMIT <
    zr zr FX*  zi zi FX*  FX+  4.0fx <  AND
  WHILE
    zr zr FX*  zi zi FX*  FX-  cr FX+          \ new zr
    zr zi FX*  DUP FX+  ci FX+  TO zi          \ new zi = 2 zr zi + ci
    TO zr
    n 1+ TO n
  REPEAT n ;

\ Pixel x of a line of W pixels, as a number from lo to lo + span.
: SPREAD  ( x total lo span -- fx )
  {: total lo span :}  S>FX span FX*  total S>FX FX/  lo FX+ ;

: COLOUR  ( n -- colour )  DUP LIMIT = IF DROP 0 ELSE 1 + 2 * PALETTE@ THEN ;

: MANDELBROT  ( -- )
  W H CANVAS
  H 0 DO
    W 0 DO
      0.0fx 0.0fx
      I W -2.5fx 3.5fx SPREAD
      J H -1.2fx 2.4fx SPREAD
      ESCAPE COLOUR  >R  I J R> PIXEL
    LOOP
  LOOP ;

: JULIA  ( -- )     \ the set of c = -0.8 + 0.156i
  W H CANVAS
  H 0 DO
    W 0 DO
      I W -1.6fx 3.2fx SPREAD
      J H -0.96fx 1.92fx SPREAD
      -0.8fx 0.156fx
      ESCAPE COLOUR  >R  I J R> PIXEL
    LOOP
  LOOP ;

MANDELBROT  S" mandelbrot.ppm" SAVE-PPM  .( mandelbrot.ppm: ior ) . CR
JULIA       S" julia.png"      SAVE-PNG  .( julia.png: ior ) . CR

PREVIOUS PREVIOUS
