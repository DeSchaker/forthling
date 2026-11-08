# A fractal of spheres, by sphere tracing

*Book 2, Pictures. Needs the core only (floating point and the word list `IMAGE`).*

The program: [`examples/book002_example007_apollonian.fs`](../examples/book002_example007_apollonian.fs).
Its output is beside it, in `book002_example007_apollonian.out`. Run it with:

```
cargo run --release -- examples/book002_example007_apollonian.fs
```

It writes `apollonian.png` (96 by 60 pixels) in the current directory and takes about 1.9
seconds (a release build, three runs: 1.939, 1.935 and 1.940). Add `SHOW` after `DRAW` to see
the picture in a terminal that shows 24-bit colour.

This is the heaviest program of the book: it takes over 150 million steps of the interpreter,
so it does **not** run in the playground (a run there has 10 million steps), and
`forthling --sandbox` stops it at the same limit. Run it from a file, as above. To run it
without writing a file, lift the limit: `forthling --sandbox --max-steps none <file>` (the
picture is then lost). The test of the examples runs it with the others.

## A picture of something that is not stored

In the other chapters the picture came from a list of pixels, or from counting points. Here
there is no list of points and no model of the thing: only a word `DE` that is given a point
in space and says **how far that point is from the fractal**. That number is called a
*distance estimate*, and it is enough to draw a picture, by *sphere tracing*:

1. For each pixel, make a ray from the camera through that pixel.
2. Stand on the ray at distance `t = 0`. Ask `DE` how far the nearest part of the fractal is.
3. Nothing is nearer than that, so the ray can walk that far without touching anything. Step
   `t` forward by that distance and ask again.
4. When the answer is tiny, the ray has hit the fractal. When `t` is far too big, it missed.

Far from the fractal the steps are long, close by they are short, and a ray that grazes a
surface takes many small steps. That gives the picture its shading for free: the pixel is
lighter when the ray needed **fewer** steps, and darker near crevices and edges, where it
needed many. There is no light source and no normal of a surface in this program.

```forth
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
```

The point is the camera plus `t` times the direction: `x` is `DX * t`, because the camera
stands at `x = 0`; `y` and `z` also add where the camera stands (`EYE-Y` and `EYE-Z`). `DE`
leaves its answer on the float stack. If it is less than `0.002 + 0.002 t` the ray has hit,
and `MARCH` leaves the number of steps it took (`I`). The limit grows with `t` because far
away a pixel covers more space, so a coarser hit is enough. A ray that is still going after
`MAXSTEPS` steps, or whose `t` passes 20, has missed, and `MARCH` leaves `MAXSTEPS`.

## The rays

`AIM` makes the direction of the ray for a pixel: from the middle of the picture, `x` and `y`
in units of half the width, and `z` always 1.2 (a bigger `z` is a narrower view). Then it
divides the three numbers by their length, so that the direction is one unit long and `t` is a
real distance.

```forth
: AIM  ( col row -- )
  >R
  W 2/ -  S>F  W 2/ S>F F/  DX F!
  H 2/ R> -  S>F  W 2/ S>F F/  DY F!
  1.2e DZ F!
  DX F@ FDUP F*  DY F@ FDUP F*  F+  DZ F@ FDUP F*  F+  FSQRT
  FDUP DX F@ FSWAP F/ DX F!  FDUP DY F@ FSWAP F/ DY F!  DZ F@ FSWAP F/ DZ F! ;
```

Row 0 is the top of the picture, so `y` is the row measured from the middle *upwards*: the
row is taken from `H 2/`, not the other way round. The point, the ray and the stretch are
kept in `FVARIABLE`s: a point has three numbers, there is a fourth for the stretch, and a
float stack that holds that many while words are called in between is hard to read.

## The distance estimate

This is the fractal. It does three things to a point, and repeats them `FOLDS` (6) times.

**Wrap.** Space is cut into cubes of side 2 and every cube is laid on the first one, from -1 to
1 in each direction. `WRAP` does that for one coordinate: `r * 0.5 + 0.5` is brought into 0 to
1 by taking off `FLOOR`, and then doubled and moved back to -1 to 1. A whole space becomes a
single cube, repeated.

```forth
: WRAP  ( F: r -- r' )  0.5e F*  0.5e F+  FDUP FLOOR F-  2e F*  1e F- ;
```

**Invert.** In the cube, a point close to the middle is thrown out through a sphere: it is
multiplied by `k = SPHERE / r2`, where `r2` is the point's squared distance to the middle and
`SPHERE` is 1.1. A point that is already outside is not moved (`1e FMAX` makes `k` at least 1).
Pushing points out of a sphere makes the next wrap fold them into a new set of smaller and
smaller spheres. That is where the many spheres of the picture come from.

**Keep count of the stretch.** An inversion makes the space bigger around the point by `k`.
`SCALE` multiplies the point by `k` and also `STRETCH`, which starts at 1 and collects the
product of all the `k`. A distance that is measured after the folds is in the stretched
space; divided by `STRETCH` it is a distance in the real one.

```forth
: DE  ( F: x y z -- d )
  Z F!  Y F!  X F!  1e STRETCH F!
  FOLDS 0 DO
    X F@ WRAP X F!  Y F@ WRAP Y F!  Z F@ WRAP Z F!
    SPHERE R2 F/  1e FMAX  SCALE
  LOOP
  Y F@ FABS  0.25e F*  STRETCH F@ F/ ;
```

At the end the distance is that of the folded point to the plane `y = 0`, a quarter of `|y|`
(a quarter, to be safe: an estimate that is too big would let a ray step through the surface),
divided by the stretch. This is a known way to draw an *Apollonian gasket* in three
dimensions (the construction is not new, only this program is); it is an estimate, not an exact distance. If it is too big somewhere, a ray steps
through a thin part of the surface and the picture has a hole; if it is too small, the ray
takes more steps than it needs. The quarter is a safe margin chosen by trying, not proved.

## Floats, not fixed-point

The program uses floating-point numbers: `DE` divides and stretches by large factors, and the
size of the numbers changes a lot from one fold to the next. Fixed-point numbers (the `FIXED`
word list of the Mandelbrot chapter) were not tried here.

## What is checked

The program uses only `+ - * /`, `FLOOR`, `FABS`, `FMAX` and `FSQRT`, and these give the same
result on every machine (unlike `FSIN` and `FCOS`, see the Clifford chapter). So the output
can hold exact numbers:

```
rays: 5760 hit: 5138 steps: 243987
apollonian.png: ior 0
```

5760 rays (96 by 60), 5138 of them hit the fractal and 622 went by. The steps of all rays
together are 243 987, about 42 for a ray. That is the number to look at when you change
something: it measures the work of the picture, and the time follows it.

## Try it

- Move the camera: `0.5e EYE-Y F!  -1e EYE-Z F!` are the last lines before `DRAW`. Change
  them a little and the view moves through the spheres.
- Make `FOLDS` 3 (1.2 seconds): a few big, smooth spheres, and a hit count of 4936. Make it
  10 (3.0 seconds): at this size the picture looks the same as with 6 and the steps are
  nearly the same (243 874), so the extra folds only cost time. The small spheres they add
  are smaller than a pixel.
- Make `SPHERE` 1.05 or 1.3, and look at what happens to the space between the spheres.
- A larger picture: `W` 192 and `H` 120 take four times as many rays, so about 8 seconds (7.9 measured, one run; 23040 rays, 974 109 steps).
