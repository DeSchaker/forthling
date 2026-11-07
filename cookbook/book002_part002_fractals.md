# Fractals: Mandelbrot, Julia and Sierpinski

*Book 2, Pictures. Needs the core only (the word lists `FIXED` and `IMAGE`).*

Three pictures, drawn pixel by pixel on the canvas:

- [`examples/book002_example001_mandelbrot.fs`](../examples/book002_example001_mandelbrot.fs):
  the Mandelbrot set and a Julia set, with fixed-point numbers. It writes `mandelbrot.ppm` and
  `julia.png`, and takes about 0.55 seconds (a release build, three runs: 0.546, 0.542 and
  0.545).
- [`examples/book002_example003_sierpinski.fs`](../examples/book002_example003_sierpinski.fs):
  the Sierpinski triangle in two ways. It writes `pascal.png` and `chaos.png` and takes about
  0.06 seconds.

The output of each is beside it, in a file that ends in `.out`. Run them with:

```
cargo run --release -- examples/book002_example001_mandelbrot.fs
cargo run --release -- examples/book002_example003_sierpinski.fs
```

Add `SHOW` after a drawing to see it in a terminal that shows 24-bit colour. With the
program `forthling-window`, `ALSO WINDOW SHOW-WINDOW` opens the picture in a window. In the
playground, pick *Mandelbrot* under *Examples*: the picture shows below the transcript
(smaller there, to stay within the 10 million steps of a run).

## Numbers with decimals, without floating point

The Mandelbrot set needs numbers such as -2.5 and 0.156. forthling has floating-point
words, but the fixed-point words of the list `FIXED` are made for this: a number is one
cell, and the sums are integer sums, so the picture is the same on every machine.

```forth
ALSO FIXED  ALSO IMAGE
32 FX-BITS  TRUE FX-ROUND-LITERALS
```

`32 FX-BITS` says that 32 of the 64 bits of a cell are the part after the point (Q32.32). A
literal such as `-2.5fx` is a fixed-point number. `TRUE FX-ROUND-LITERALS` rounds literals such as
`-2.2fx` to the nearest number that can be stored, instead of giving an error. `FX+` and `FX-`
are exact; `FX*` and `FX/` round. `S>FX` makes a fixed-point number of an integer.

## The Mandelbrot set

Take a point `c` of the plane, a complex number `cr + ci·i`. Start with `z = 0` and repeat
`z := z² + c`. For some `c` the numbers stay small for ever; those are in the set. For
others they grow without limit, and once `|z|` is larger than 2 it is certain they will. The
number of steps it takes to get past 2 gives the colour of the pixel.

With `z = zr + zi·i`, one step is `zr' = zr² − zi² + cr` and `zi' = 2·zr·zi + ci`, and `|z| < 2`
is the same as `zr² + zi² < 4`. That is the word `ESCAPE`, which takes `z` and `c` and gives the
number of steps, at most `LIMIT`:

```forth
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
```

The `{: ... :}` takes the five numbers off the stack into local names, so the formulas read like the
formulas above. The new `zr` is worked out first, while the old `zi` is still there; then `TO zi`
stores the new `zi`, and `TO zr` the new `zr`. Both new values use the old ones, so the
order matters.

## From pixel to point

The picture is `W` by `H` pixels and shows the part of the plane from -2.5 to 1.0 across and
from -1.2 to 1.2 up and down. `SPREAD` turns the number of a pixel into its place on such a range:

```forth
: SPREAD  ( x total lo span -- fx )
  {: total lo span :}  S>FX span FX*  total S>FX FX/  lo FX+ ;
```

Pixel `x` of `total` is at `lo + span · x / total`. And the colour comes from the palette of the
canvas, with the number of steps as index; a point that never escaped (`n = LIMIT`) is black:

```forth
: COLOUR  ( n -- colour )  DUP LIMIT = IF DROP 0 ELSE 1 + 2 * PALETTE@ THEN ;
```

`MANDELBROT` then makes a canvas, and for each pixel sets `z = 0` and `c` to the point of the
pixel, and paints the colour of `ESCAPE`.

## A Julia set

For a Julia set it is the other way round: `c` is the same for all pixels, and `z` starts at the
point of the pixel. `JULIA` is `MANDELBROT` with those two changes: the call of `ESCAPE` gets the
pixel's point as `z`, and the fixed `c = -0.8 + 0.156i`:

```forth
      I W -1.6fx 3.2fx SPREAD
      J H -0.96fx 1.92fx SPREAD
      -0.8fx 0.156fx
      ESCAPE COLOUR  >R  I J R> PIXEL
```

The file ends by saving the two pictures: `S" mandelbrot.ppm" SAVE-PPM` and
`S" julia.png" SAVE-PNG` both give 0 when it worked. The files land in the current directory.
Both formats are read by most viewers; the PNG is not compressed, so it is larger.

## The Sierpinski triangle, from Pascal's triangle

The triangle of Pascal has a 1 at the top, and every number below is the sum of the two above
it. If you paint the *odd* numbers, the Sierpinski triangle appears. There is no need to
add anything: the number in row `n` at place `k` is odd exactly when every 1 bit of `k` is
also a 1 bit of `n`, that is, when `k AND n = k`.

```forth
: ODD?  ( n k -- f )  TUCK AND = ;                 \ k AND n = k
: PASCAL  ( -- count )
  SIZE SIZE CANVAS  0
  SIZE 0 DO
    I 1+ 0 DO
      J I ODD? IF  I J WHITE PIXEL  1+  THEN       \ J is the row n, I the place k in it
    LOOP
  LOOP ;
```

The 128 rows have 3⁷ = 2187 odd numbers, and the program counts 2187 and prints
both, as a check. (Each time the number of rows doubles, the number of painted spots
triples.) The picture is a right-angled triangle.

## The Sierpinski triangle, by chance

The second way is the *chaos game*. Take the three corners of a triangle and a point anywhere.
Pick a corner at random, move half way to it, paint the point, and repeat. A few
steps aside, the points always land on the Sierpinski triangle:

```forth
: CHAOS  ( jumps -- lit )
  SIZE SIZE CANVAS
  SIZE 2/ SIZE 2/ 0 {: x y c :}  0 SWAP
  0 DO
    3 RND CELLS TO c
    x CORNER-X c + @ + 2/ TO x                     \ half way to the corner
    y CORNER-Y c + @ + 2/ TO y
    x y PIXEL@ 0= IF  1+  THEN                     \ a new spot?
    x y WHITE PIXEL
  LOOP ;
```

The result of `30000 CHAOS` is the number of different pixels painted, 3380 here. The random
numbers come from a small generator of our own with a fixed seed (the same as in the maze),
so the picture and the number are always the same. All the locals (`x`, `y` and `c`) are declared once, at the start of the word; `c` starts as 0
and `TO c` sets it in each round. (A `{: c :}` in the middle of the loop did not give the
right answer when this was written, so the declaration stays at the start.)

Two ways, one picture: the first is exact and has no chance in it; the second is a
picture that grows out of noise.

## Try it

- Zoom in on the Mandelbrot set. Change the ranges `-2.5fx 3.5fx` and `-1.2fx 2.4fx` (start and
  width) in `MANDELBROT` and raise `LIMIT`.
- Try other values of `c` for the Julia set, such as `-0.4 + 0.6i`.
- Give the chaos game other corners, or jump another part of the way than a half.
