\ book002_example007_apollonian.fs - a fractal of spheres, drawn by sphere tracing (raymarching).
\ Run: cargo run --release -- examples/book002_example007_apollonian.fs
\ Needs the core only (floating point, IMAGE). It writes apollonian.png in the current directory;
\ add SHOW after DRAW to see the picture in a terminal.
\
\ How it works:
\   Nothing is stored of the fractal. A word DE says how far a point is from it (a distance
\   estimate), and each ray from the camera is walked in steps of exactly that length: no step
\   can go through the surface. A ray that gets closer than a tiny distance has hit; a ray that
\   goes too far has missed. The pixel is lighter when fewer steps were needed.
\   DE folds space: it brings the point into a cube, inverts it in a sphere (which is what makes
\   the many spheres) and repeats that, keeping track of how much the space was stretched.
\   Only + - * / FLOOR FSQRT are used, so the numbers are the same on every machine.

ALSO IMAGE

96 CONSTANT W   60 CONSTANT H                        \ the picture is W by H pixels
80 CONSTANT MAXSTEPS                                 \ a ray gets at most this many steps
6 CONSTANT FOLDS                                     \ how many times DE folds the space
1.1e FCONSTANT SPHERE                                \ the radius squared of the inversion

FVARIABLE X   FVARIABLE Y   FVARIABLE Z              \ the point that DE looks at
FVARIABLE STRETCH                                    \ how much DE stretched the space
FVARIABLE DX  FVARIABLE DY  FVARIABLE DZ             \ the direction of the ray
FVARIABLE T                                          \ how far along the ray we are
FVARIABLE EYE-Y  FVARIABLE EYE-Z                     \ where the camera stands (x is 0)

\ One coordinate into the cube from -1 to 1, repeated all over space.
: WRAP  ( F: r -- r' )  0.5e F*  0.5e F+  FDUP FLOOR F-  2e F*  1e F- ;

\ r2 = x*x + y*y + z*z, the squared distance of the point to the middle of the cube.
: R2  ( F: -- r2 )  X F@ FDUP F*  Y F@ FDUP F*  F+  Z F@ FDUP F*  F+ ;

: SCALE  ( F: k -- )                                 \ stretch the point and STRETCH by k
  FDUP X F@ F* X F!  FDUP Y F@ F* Y F!  FDUP Z F@ F* Z F!
  STRETCH F@ F* STRETCH F! ;

\ The distance estimate. A point inside the sphere of radius sqrt(SPHERE) is moved out by
\ k = SPHERE / r2 (an inversion); k is at least 1, so a point outside stays where it is.
: DE  ( F: x y z -- d )
  Z F!  Y F!  X F!  1e STRETCH F!
  FOLDS 0 DO
    X F@ WRAP X F!  Y F@ WRAP Y F!  Z F@ WRAP Z F!
    SPHERE R2 F/  1e FMAX  SCALE
  LOOP
  Y F@ FABS  0.25e F*  STRETCH F@ F/ ;

\ Walk the ray. Gives the steps taken, or MAXSTEPS when it hit nothing. A ray hits when DE is
\ under 0.002 plus 0.002 per unit of distance: far away a pixel is wider, so it may be coarser.
: MARCH  ( -- steps )
  0e T F!
  MAXSTEPS 0 DO
    DX F@ T F@ F*
    DY F@ T F@ F*  EYE-Y F@ F+
    DZ F@ T F@ F*  EYE-Z F@ F+
    DE
    FDUP  0.002e T F@ F* 0.002e F+  F< IF FDROP I UNLOOP EXIT THEN
    T F@ F+ T F!
    T F@ 20e F> IF MAXSTEPS UNLOOP EXIT THEN        \ gone too far: a miss
  LOOP MAXSTEPS ;

\ The ray through pixel (col, row): x and y from the middle of the picture, z always 1.2, and
\ then made one unit long (divided by its length).
: AIM  ( col row -- )
  >R
  W 2/ -  S>F  W 2/ S>F F/  DX F!
  H 2/ R> -  S>F  W 2/ S>F F/  DY F!
  1.2e DZ F!
  DX F@ FDUP F*  DY F@ FDUP F*  F+  DZ F@ FDUP F*  F+  FSQRT
  FDUP DX F@ FSWAP F/ DX F!  FDUP DY F@ FSWAP F/ DY F!  DZ F@ FSWAP F/ DZ F! ;

\ A warm light: the fewer the steps, the lighter.
: SHADE  ( steps -- colour )
  MAXSTEPS SWAP -  255 * MAXSTEPS /  {: v :}
  v 16 LSHIFT  v 3 * 4 / 8 LSHIFT OR  v 2/ OR ;

VARIABLE HITS   VARIABLE STEPS                       \ rays that hit, and the steps of all rays

: DRAW  ( -- )
  0 HITS !  0 STEPS !
  W H CANVAS
  H 0 DO
    W 0 DO
      I J AIM  MARCH  DUP STEPS +!
      DUP MAXSTEPS = IF DROP 0 ELSE 1 HITS +!  SHADE THEN
      I J ROT PIXEL
    LOOP
  LOOP ;

0.5e EYE-Y F!  -1e EYE-Z F!
DRAW
." rays: " W H * . ." hit: " HITS @ . ." steps: " STEPS @ . CR
S" apollonian.png" SAVE-PNG ." apollonian.png: ior " . CR

PREVIOUS
