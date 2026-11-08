# The De Jong attractor: four numbers, many pictures

*Book 2, Pictures. Needs the core only (floating point and the word list `IMAGE`). Read
[the Clifford chapter](book002_part003_clifford.md) first: this one builds on it.*

The program: [`examples/book002_example006_dejong.fs`](../examples/book002_example006_dejong.fs).
Its output is beside it, in `book002_example006_dejong.out`. Run it with:

```
cargo run --release -- examples/book002_example006_dejong.fs
```

It writes `dejong1.png`, `dejong2.png` and `dejong3.png` in the current directory and takes
about 0.5 seconds for the three (a release build, three runs: 0.491, 0.491 and 0.490). Add
`SHOW` after `DRAW` in `DRAW-ONE` to see a picture in a terminal that shows 24-bit colour.

## The same program, another jump

Peter de Jong's attractor is a cousin of the Clifford attractor. The point `(x, y)` jumps by

```
x' = sin(a·y) - cos(b·x)
y' = sin(c·x) - cos(d·y)
```

Only the jump changed, so `STEP` is the only word that is new. The counting of hits, the
logarithmic brightness and the smoke colours are copied from the Clifford program (`HIT`,
`BRIGHT`, `SMOKE`, `DRAW`). Both terms of each sum are between -1 and 1, so `x` and `y` are
always between -2 and 2: `>PIXEL` maps that square onto the picture.

```forth
: STEP  ( F: x y -- x' y' )
  FOVER FOVER                                        \ x y x y
  A F@ F* FSIN  FSWAP  B F@ F* FCOS  F-              \ x y x'      x' = sin(a y) - cos(b x)
  FROT FROT                                          \ x' x y
  FSWAP C F@ F* FSIN  FSWAP D F@ F* FCOS  F- ;       \ x' y'       y' = sin(c x) - cos(d y)
```

The stack work is the same as in `STEP` of the Clifford program, so the comments on the right
tell the same story. What differs is the minus where Clifford has a plus, and where `a b c d`
come from.

## Four numbers, given to each picture

In the Clifford program `a b c d` were constants. Here the program draws several pictures, so
the numbers are variables, and `DRAW-ONE` takes them from the float stack:

```forth
FVARIABLE A   FVARIABLE B   FVARIABLE C   FVARIABLE D

: DRAW-ONE  ( c-addr u -- ) ( F: a b c d -- )
  D F!  C F!  B F!  A F!
  HITS SIZE SIZE * CELLS ERASE
  ITERATE
  ." jumps: " JUMPS . ." counted: " TOTAL . CR
  NUMBERS
  DRAW
  2DUP TYPE ." : ior " SAVE-PNG . CR ;
```

`F!` takes the top number, so the four are stored from `d` back to `a`. The array of hits is
cleared first, so each picture starts empty, and the name of the file comes in as a string.
Making a picture is now one line, the four numbers and a name:

```forth
1.4e -2.3e 2.4e -2.1e      S" dejong1.png" DRAW-ONE
2.01e -2.53e 1.61e -0.33e  S" dejong2.png" DRAW-ONE
1.4e -2.3e -2.4e -2.1e     S" dejong3.png" DRAW-ONE
```

## What the numbers do

The first picture, `1.4 -2.3 2.4 -2.1`, is a bowl of fine, soft curves. The second,
`2.01 -2.53 1.61 -0.33`, is a quite different shape: a wing with sharp folds and a thin
edge. The third has the numbers of the first, with one change: the sign of `c`. The picture
turns into another one, with a loop in the middle and thinner lines.

That is the point of this attractor. There is no way to tell from the numbers what the picture
will look like, and a small change in one number can change it a lot. You find the good ones
by trying, which is why the program is built so that a try is one line.

Two of the signs do not matter. `cos` is an even function, so `cos(b·x)` and `cos(-b·x)` are
the same: the sign of `b` changes nothing, and neither does the sign of `d`. The signs of `a`
and `c`, inside a `sin`, do matter.

## What is checked

As in the Clifford chapter, the output holds only what is the same on every machine: each
picture counts all 100000 jumps. `sin` and `cos` differ in the last bits from one machine to
another and the jumps magnify that, so the exact counts are left out. Set `SHOW-NUMBERS` to
`TRUE` to print them. On the author's Windows machine that gives:

| picture | most hits in one pixel | pixels hit (of 25600) |
|---|---|---|
| `dejong1.png` | 332 | 15807 |
| `dejong2.png` | 310 | 5052 |
| `dejong3.png` | 122 | 17404 |

The second picture hits far fewer pixels than the others: its lines are thin and lie close
together, which is what makes it look sharp. The pictures themselves may differ slightly on
another machine.

## Try it

- Change one number by a little, such as `2.01` to `2.05` in the second picture, and look at
  what happens.
- Take `SIZE` 320 and `JUMPS` 1000000 for a larger picture and more smoke (the Clifford
  chapter has timings for that size).
- Try numbers of your own. The picture always fits (`x` and `y` stay between -2 and 2), so
  any four numbers give a picture, and you have to look to see which ones are worth keeping.
