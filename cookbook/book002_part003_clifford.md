# The Clifford attractor: a picture from one formula

*Book 2, Pictures. Needs the core only (floating point and the word list `IMAGE`).*

The program: [`examples/book002_example005_clifford.fs`](../examples/book002_example005_clifford.fs).
Its output is beside it, in `book002_example005_clifford.out`. Run it with:

```
cargo run --release -- examples/book002_example005_clifford.fs
```

It writes `clifford.png` in the current directory and takes about 0.2 seconds (a release
build, three runs: 0.199, 0.198 and 0.202). Add `SHOW` after `DRAW` to see the picture in a
terminal that shows 24-bit colour; with `forthling-window`, `ALSO WINDOW SHOW-WINDOW` opens
it in a window. In the playground, pick *Clifford* under *Examples*: the picture shows below
the transcript (smaller there, 128 by 128 pixels and 40 000 jumps, to stay within the 10
million steps of a run).

## One point, jumping

Take a point `(x, y)` and move it by this rule, again and again:

```
x' = sin(a·y) + c·cos(a·x)
y' = sin(b·x) + d·cos(b·y)
```

With `a = -1.4`, `b = 1.6`, `c = 1.0` and `d = 0.7` the point never settles and never leaves a
small area. It keeps coming back to the same curves, and together they make a shape like a
folded cloth: a strange attractor. Other values of `a`, `b`, `c` and `d` give other shapes.
Since `sin` and `cos` are never more than 1, `|x| <= 1 + c` and `|y| <= 1 + d`, so the picture
always fits in a square from -2.1 to 2.1.

The numbers are floating point: `FSIN` and `FCOS` are standard words, and the constants are
made with `FCONSTANT`:

```forth
-1.4e FCONSTANT A   1.6e FCONSTANT B   1.0e FCONSTANT C   0.7e FCONSTANT D
```

## A jump on the float stack

The whole state of the system is two numbers, so they are all the stack holds. `STEP` takes
`x` and `y` and leaves `x'` and `y'`:

```forth
: STEP  ( F: x y -- x' y' )
  FOVER FOVER                                        \ x y x y
  A F* FSIN  FSWAP  A F* FCOS C F*  F+               \ x y x'      x' = sin(a y) + c cos(a x)
  FROT FROT                                          \ x' x y
  FSWAP B F* FSIN  FSWAP B F* FCOS D F*  F+ ;        \ x' y'       y' = sin(b x) + d cos(b y)
```

The new `x'` needs both old numbers, and so does `y'`. So the word first copies the pair
(`FOVER FOVER` does for floats what `2DUP` does for cells: there is no `F2DUP`). The top copy
gives `x'`, and it sits on top of the original pair: `x y x'`. Then `FROT FROT` turns the stack
round: `FROT` brings the third number to the top, and doing it twice makes `x' x y` of it. Now `y'` is worked out from the top two, and `FSWAP` puts the right number on top
before each product (`b·x`, then `b·y`). The stack comment after each line shows where
things are; with the stack in front of you, there are no names to keep track of.

`FOVER FOVER`, `FROT FROT` and `FSWAP` are the floating-point twins of `2DUP`, `-ROT` and
`SWAP`. The Mandelbrot program takes its numbers by name with locals; that works there because
fixed-point numbers are cells. There are no locals for floats, and a jump this size is easy to
follow on the stack.

## Counting visits

If every point were painted once, the picture would be a haze of dots, and most of it would
look the same. Instead each pixel counts how often the point landed in it, and the colour
follows that count. A cell array `HITS` has a count for each pixel:

```forth
: >PIXEL  ( F: r -- ) ( -- n )  2.1e F+  4.2e F/  SIZE S>F F*  F>S ;

: HIT  ( F: x y -- x y )                             \ count one visit of the point
  FOVER FOVER  >PIXEL >PIXEL  {: row col :}          \ y first, so its pixel is under x's
  row 0 SIZE WITHIN  col 0 SIZE WITHIN AND IF
    1  HITS row SIZE * col + CELLS +  +!
  THEN ;

: ITERATE  ( -- )  0.1e 0.1e  JUMPS 0 DO STEP HIT LOOP  FDROP FDROP ;
```

`>PIXEL` maps a number in -2.1 to 2.1 onto 0 to `SIZE - 1`. `HIT` copies the pair, makes the
two pixel numbers (the data stack and the float stack are separate, so they do not get in each
other's way), and adds 1 to the cell of that pixel. The test with `WITHIN` keeps a point
outside the square from writing outside the array; here none does, and the program prints
the total as a check: `jumps: 100000 counted: 100000`.

The starting point `(0.1, 0.1)` is arbitrary: the first jumps already land on the attractor.

## Smoke

A pixel that was hit 311 times (the most, on the author's Windows machine) is not 311 times
brighter than one that was hit once; the eye would see only the dense places. The colour follows the *logarithm* of the count:

```forth
: BRIGHT  ( n max -- v )
  1+ S>F FLN  1+ S>F FLN  FSWAP F/  255e F*  F>S ;
```

The `1+` is there since `ln 1` is 0: without it a pixel with one hit would stay black. A pixel
with no hit is skipped by `DRAW`, and stays black. `SMOKE` then turns a
brightness `v` from 0 to 255 into a colour, a cold white with the blue part strongest:

```forth
: SMOKE  ( v -- colour )
  {: v :}  v 3 * 4 /  16 LSHIFT  v 7 * 8 / 8 LSHIFT OR  v OR ;
```

A colour on the canvas is one number, `0xRRGGBB`: the red part is moved 16 bits up, the green
part 8. `DRAW` goes over the array, finds the colour for each pixel that was hit and paints it.

The program prints two facts about the picture: some pixel was hit over 100 times, and under
half of the pixels were hit at all. The picture is mostly empty, which is why the smoke shows
against black. The exact counts are left out of the output on purpose. `sin` and `cos` differ in
the last bits from one machine to another, and the jumps magnify that: the picture looks the
same, but the counts do not (the most hits in one pixel was 311 on Windows and 344 on Linux).
Set `SHOW-NUMBERS` to `TRUE` to print them: here that gives 7038 pixels hit of 25600.

## Try it

- Change `A B C D`. `-1.7 1.3 -0.1 -1.2` gives a quite different shape. Its `y` can reach
  `1 + 1.2 = 2.2`, a little outside the square, so the program counts fewer than the 100000
  jumps (97434 on Windows): that is what the check line is for. Widen the range in `>PIXEL` (2.1 and 4.2) to
  2.3 and 4.6, and all jumps are counted.
- Raise `JUMPS` to 1 000 000 for a smoother picture (it took about 1.5 seconds here, one run).
  With more jumps the dense places have far larger counts (3261 at most on Windows), so the
  logarithm matters more.
- Take `SIZE` 320 for a larger picture. The cell array and the canvas grow with it.
