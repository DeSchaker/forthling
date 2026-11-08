# Domain-warped noise: small words that compose

*Book 2, Pictures. Needs the core only (floating point and the word list `IMAGE`).*

The program: [`examples/book002_example008_warp.fs`](../examples/book002_example008_warp.fs).
Its output is beside it, in `book002_example008_warp.out`. Run it with:

```
cargo run --release -- examples/book002_example008_warp.fs
```

It writes `noise.png`, `warp1.png` and `warp2.png` (128 by 128 pixels) in the current
directory and takes about 1.9 seconds (a release build, three runs: 1.893, 1.892 and 1.902).
That is between 100 and 150 million interpreter steps, so it does not run in the playground,
which stops at 10 million. Add `SHOW` after `DRAW` in `DRAW-ONE` to see a picture in a terminal
that shows 24-bit colour.

The pictures have no source image and no random numbers. Each one is a formula, and the
formula is built from five small words, each using the one before it.

## Noise from a hash

`HASH` turns two whole numbers into a number from 0 to 255. It only mixes bits: multiply,
shift the high bits down, multiply again.

```forth
: HASH  ( ix iy -- n )
  668265263 *  SWAP 374761393 * +
  DUP 13 RSHIFT XOR  1274126177 *
  DUP 16 RSHIFT XOR  255 AND ;
```

The same two numbers always give the same answer, and a small change in the input gives an
unrelated answer. That is all a lattice of random values needs. (The big numbers only mix the
bits; the products wrap around, which is what is wanted here.)

`NOISE` gives a value between 0 and 1 at any point `x y`. It cuts the point into a whole part
and a fraction. The whole parts name a square of the lattice; the hash at its four corners
gives four values. The fractions say where in the square the point is, and the four values are
blended with `LERP`, a straight blend `a + t (b - a)`, after the fraction has been bent by
`SMOOTH`, `t·t·(3 - 2t)`. Without the bend the picture shows the squares.

```forth
: SMOOTH  ( F: t -- s )  FDUP FDUP F* FSWAP 2e F* 3e FSWAP F- F* ;
: LERP  ( F: a b t -- v )  FROT FROT FOVER F- FROT F* F+ ;
```

Both words are short stack puzzles. `LERP` starts as `a b t`; two `FROT` give `t a b`, then
`FOVER F-` gives `t a (b-a)`, `FROT F*` multiplies that by `t`, and `F+` adds `a`.
The comment after the name is the proof: write the stack after each word and see that you
get `a + t (b - a)`.

## Octaves

One layer of noise looks like soft blobs of one size. `FBM` ("fractal Brownian motion", a
grand name for a sum) adds `NOISE` four times, each time with the point twice as far from the
origin (so the blobs are half as big) and the layer half as strong. `noise.png` is that sum,
before any warp. The sum is divided by 0.9375, which is
0.5 + 0.25 + 0.125 + 0.0625, so the result is again between 0 and 1.

```forth
: FBM  ( F: x y -- v )
  PY F!  PX F!  0e SUM F!  0.5e AMP F!
  OCTAVES 0 DO
    PX F@ PY F@ NOISE  AMP F@ F*  SUM F@ F+  SUM F!
    PX F@ 2e F* PX F!   PY F@ 2e F* PY F!   AMP F@ 2e F/ AMP F!
  LOOP
  SUM F@  0.9375e F/ ;
```

Float values live in `FVARIABLE`s here, as in the Clifford and De Jong programs: forthling has
no float locals, and a loop that keeps four floats on the float stack is harder to read than
four names.

## Warp

Now the idea of the chapter. To draw the picture at point `p`, do not ask `FBM` at `p`. Ask it
at a point that has been moved by another noise:

```
g(p) = fbm(p + k * (fbm(p), fbm(p + offset)))
```

The point is moved right by one noise and up by another. The second one needs a different
noise, but we have only one `FBM`, so we ask it at another place: `p + offset`, with the
offset `5.2, 1.3`. Any offset that is not a multiple of the lattice will do.

```forth
: WARP  ( F: x y -- x' y' )
  WY F!  WX F!
  WX F@ WY F@ FBM                                    \ qx
  WX F@ 5.2e F+  WY F@ 1.3e F+  FBM                  \ qx qy
  STRENGTH F* WY F@ F+                               \ qx y'
  FSWAP STRENGTH F* WX F@ F+                         \ y' x'
  FSWAP ;                                            \ x' y'
```

`WARP` takes a point and leaves a point, so it can be applied to its own result. That is the
second stage, and it is one word in front of `FBM`:

```forth
: VALUE-AT  ( warps -- ) ( F: x y -- v )
  0 ?DO WARP LOOP  FBM ;
```

With 0 warps this is the plain noise. With 1 it is `warp1.png`: the blobs are pulled into
folds. With 2 it is `warp2.png`: the folds are pulled again, and the picture looks like ink
in water. The program makes all three, so you can see what each step adds. Nothing in the
program is a swirl; a swirl is what comes out when a field is bent by itself.

## The colours

`FILL` computes one value per pixel and keeps them in an array, with the lowest and the highest
value. The values lie in the middle of 0 to 1, so `>LEVEL` stretches them to 0 to 255 over
the range that was found. `INK` makes a colour from a level: red grows with the square of the
level, green is in between, and blue is already 60 at level 0, so the dark is blue and the
light a pale yellow.

## What is checked

The program uses only `+ - * /` and `FLOOR` on floats. They give the same result on every
machine, unlike `FSIN` and `FEXP` (see the Clifford chapter), so here the output can hold
exact numbers: the lowest and the highest value of each picture, in thousandths, and a
checksum of its levels. The test compares them with a run of the program.

| picture | lowest | highest |
|---|---|---|
| `noise.png` | 0 | 899 |
| `warp1.png` | 119 | 856 |
| `warp2.png` | 120 | 907 |

The lowest of the plain noise is exactly 0. The corner `0 0` of the lattice has the hash 0,
and at the point `0 0` every octave gives that corner, so that is the lowest value. A warp
moves that point away, and the lowest value of the warped pictures is higher.

## Try it

- Change `STRENGTH` from 2 to 0.5 or to 4. Small gives a gentle bend, large tears the folds
  apart.
- Change `OCTAVES` to 1 or to 6. Remember to change 0.9375 to the sum of the amplitudes
  (for one octave 0.5; for six octave 0.984375), or the values go outside 0 to 1.
- Warp three times: `3 S" warp3.png" DRAW-ONE`. It costs about one more `FBM` pair per pixel
  for each stage.
- Use the first noise for the colour and the warped one for the position, or the other way
  round. Each is one more call of a word that is already there.
