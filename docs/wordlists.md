# Word lists

The words that come with forthling beyond the standard, each in a word list of its own so that a standard program never meets them by accident. Open one with `ALSO name`. Back to the [README](../README.md).

## Modules

Your own libraries can have a word list too. `MODULE name` starts one: the words you define
are private until `EXPORT`, and the words after `EXPORT` are public. `END-MODULE` ends it.
`USING name` (the same as `ALSO name`) makes the public words visible.

```forth
MODULE geometry
  : SQUARE  DUP * ;                    \ private: only the module sees it
  EXPORT
  : AREA  ( w h -- n )  * ;
  : DISC  ( r -- n )  SQUARE 3 * ;     \ 3 for pi, whole numbers only
END-MODULE
USING geometry
4 3 AREA .          \ 12
2 DISC .            \ 12
PREVIOUS
```

The public words keep working after `END-MODULE`, because they were compiled while the
private words were visible; but `3 SQUARE` outside the module is an undefined word (-13). While a module is open the search order is its public list, its private list and
then the order from before; `END-MODULE` puts the order and the current word list back as
they were. Modules nest, and `EXPORT` can come once. `ORDER` shows the public list under the
name of the module. For a file that other files load, use `REQUIRE`: it loads a file once, so
the module is made once. The search is relative to the including file; there is no search path.

## JSON

The word list `JSON` (`ALSO JSON`) is built in; the forthling program and the Python
module install it, a Rust program with `forth.install(&forthling::Json)`. A JSON value is
its text, a string `( c-addr u )`: the reading words give back pieces of it, so nothing is
copied. A missing key or a value of the wrong kind gives the flag 0.

```forth
ALSO JSON
: DOC  S\" {\"users\": [{\"name\": \"Ada\", \"born\": 1815}]}" ;
DOC S" users[0].name" JSON@ DROP JSON>STRING DROP TYPE   \ Ada (ALLOCATEd: FREE it)
DOC S" users[0].born" JSON@ DROP JSON>NUMBER DROP .      \ 1815
JSON{ S" sum" JSON-KEY 6 JSON-NUMBER S" ok" JSON-KEY -1 JSON-FLAG JSON}
\ {"sum":6,"ok":true}
```

| Reading | Stack |
|---|---|
| `JSON-VALID?` | `( json -- flag )` |
| `JSON-TYPE` | `( json -- n )`: 0 null, 1 boolean, 2 number, 3 string, 4 array, 5 object, -1 no JSON |
| `JSON-GET` | `( json c-addr u -- json' flag )`: the value of a key |
| `JSON-NTH` | `( json n -- json' flag )`: element n (from 0) of an array or object |
| `JSON-NAME` | `( json n -- c-addr u flag )`: key n of an object, `ALLOCATE`d |
| `JSON-LENGTH` | `( json -- n )`: elements of an array or object, else -1 |
| `JSON@` | `( json c-addr u -- json' flag )`: a path, such as `a.b[2]` or `["a.b"]` |
| `JSON>NUMBER` | `( json -- n flag )`: an integer that fits in a cell |
| `JSON>FLOAT` | `( json -- flag ) ( F: -- r )` |
| `JSON>FLAG` | `( json -- flag ok )` |
| `JSON>STRING` | `( json -- c-addr u flag )`: without its escapes, `ALLOCATE`d |

Writing goes to the output, or between `<JSON` and `JSON>` (which gives `( c-addr u )`,
`ALLOCATE`d) to a string; the commas come by themselves. `JSON{ JSON} JSON[ JSON]`,
`JSON-KEY ( c-addr u -- )`, `JSON-STRING ( c-addr u -- )`, `JSON-NUMBER ( n -- )`,
`JSON-FLOAT ( F: r -- )`, `JSON-FLAG ( flag -- )`, `JSON-NULL` and `JSON-RAW ( json -- )`.
A structure that does not fit, such as a value in an object without its key, throws -22.

## Functional words

The word list `FUNCTIONAL` (`ALSO FUNCTIONAL`) has `MAP`, `FOLD`, `RUNNING-FOLD`, `FILTER` and `RANGE`
on the data stack. The loop runs in Rust and calls the execution token once per item; the
stack effect of the token is checked after each call, and the step and time limits apply
inside it. A primitive token (`' +`, `' INVERT`, or a quotation of one primitive) is called
directly, without the inner interpreter. The forthling program, the MCP server, the Python
module and the playground install it; a Rust program does with
`forth.install(&forthling::Functional)`.

```forth
ALSO FUNCTIONAL
1 6 RANGE [: DUP * ;] MAP 5 ' + FOLD .        \ 55
1 10 RANGE [: 2 MOD 0= ;] FILTER .S           \ <5> 2 4 6 8 4
```

| Word | Stack |
|---|---|
| `MAP` | `( x1 .. xn n xt -- y1 .. yn )`, `xt` is `( x -- y )` |
| `FOLD` | `( x1 .. xn n xt -- r )`, `xt` is `( acc x -- acc' )`; `x1` is the first `acc`, so `n` is at least 1 |
| `RUNNING-FOLD` | `( x1 .. xn n xt -- r1 .. rn )`: like `FOLD`, with every partial result |
| `FILTER` | `( x1 .. xn n xt -- y1 .. ym m )`, `xt` is `( x -- flag )` |
| `RANGE` | `( a b -- a .. b-1 n )` |
| `ANY?` `ALL?` | `( x1 .. xn n xt -- flag )`, `xt` is `( x -- flag )`; they stop at the answer; `ALL?` is true for `n` = 0 |
| `FIND` | `( x1 .. xn n xt -- x true \| false )`, the first item that passes |
| `COUNT-IF` | `( x1 .. xn n xt -- m )` |
| `FLAT-MAP` | `( x1 .. xn n xt -- y1 .. ym m )`, `xt` is `( x -- y1 .. yk k )`; the results are joined |
| `ITERATE` | `( x n xt -- x1 .. xn )`: `x1` is `x`, every next item is `xt` of the one before |
| `REVERSE` | `( x1 .. xn n -- xn .. x1 )` |
| `TAKE` `DROP-N` | `( x1 .. xm m n -- .. k )`: the first `n` items, or without them; `k` is the new count |
| `ZIP` | `( a1 .. an b1 .. bn n -- a1 b1 .. an bn 2n )` |
| `SUM` `PRODUCT` | `( x1 .. xn n -- r )`, wrapping; 0 and 1 for `n` = 0 |
| `MIN-OF` `MAX-OF` | `( x1 .. xn n -- r )`; `n` is at least 1 |

A word that changes the length of the list leaves the new count on top (`FILTER`, `FLAT-MAP`,
`TAKE`, `DROP-N`, `ZIP`); `MAP`, `ITERATE` and `REVERSE` do not.

```forth
5 8 12 3 [: 6 > ;] FIND .S                    \ <2> 8 -1
1 2 3 10 20 30 3 ZIP .S                       \ <7> 1 10 2 20 3 30 6
```

The same on a block of cells in memory, for large amounts of data, with the suffix `-CELLS`. The
results go back into the block; the address range is checked as for other memory words.

```forth
CREATE A 3 , 1 , 4 , 1 , 5 ,
A 5 ' MAX FOLD-CELLS .                        5
A 5 [: DUP * ;] MAP-CELLS  A 5 ' + FOLD-CELLS .    52
```

| Word | Stack |
|---|---|
| `MAP-CELLS` | `( addr n xt -- )`, in place; `xt` is `( x -- y )` |
| `FOLD-CELLS` | `( addr n xt -- r )`; the first cell is the first `acc`, so `n` is at least 1 |
| `RUNNING-FOLD-CELLS` | `( addr n xt -- )`, in place: cell i becomes the partial result over cells 0 to i |
| `FILTER-CELLS` | `( addr n xt -- m )`, the cells that pass are moved to the front; `m` is how many |
| `ANY?-CELLS` `ALL?-CELLS` | `( addr n xt -- flag )`; the block stays as it is |
| `FIND-CELLS` | `( addr n xt -- x true \| false )` |
| `COUNT-IF-CELLS` | `( addr n xt -- m )`; unlike `FILTER-CELLS` it does not change the block |
| `REVERSE-CELLS` | `( addr n -- )`, in place |
| `SUM-CELLS` `PRODUCT-CELLS` | `( addr n -- r )`, wrapping; 0 and 1 for `n` = 0 |

`TAKE`, `DROP-N`, `MIN-OF` and `MAX-OF` have no `-CELLS` form: on a block, `TAKE` and `DROP-N`
are a change of `addr n`, and `' MIN FOLD-CELLS` does the other two.

Measured on 60000 numbers, 100 times, release build: summing with `RANGE ' + FOLD` takes 93 ms,
against 471 ms for a plain `DO` loop; with a colon word instead of `+` it takes 250 ms. A
quotation that is not one primitive (`[: DUP * ;] MAP`) goes through the inner interpreter
and is not faster than a loop (460 ms against 361 ms for a loop with the same work, `MAP`
and a `FOLD` to add up the result included).

## Dynamic strings

The word list `DYNAMIC-STRINGS` (`ALSO DYNAMIC-STRINGS`) has the Gforth `$` words: strings that grow and
shrink, and arrays of them. A dynamic string lives in one cell that holds 0 (the empty
string) or the address of a block from `ALLOCATE`: a cell with the length, then the
characters. So the memory counts towards the limits of the sandbox, and `$FREE` gives it back.
The words are in their own word list because names such as `$@` and `$!` could clash with code
of your own. The forthling program, the MCP server, the Python module and the playground
install it; a Rust program does with `forth.install(&forthling::Strings)`. The behaviour is
built from the Gforth manual, not from the Gforth source.

```forth
ALSO DYNAMIC-STRINGS
$VARIABLE S
S" Hello" S $!   S" , world" S $+!
S $@ TYPE                                     \ Hello, world
: PART ( c-addr u -- )  ." <" TYPE ." >" ;
S" a,bb,c" S $!   S 44 ' PART $ITER           \ <a><bb><c>
$[]VARIABLE A
S" one" A $[]+!   S" two" A $[]+!
A ' PART $[]MAP                               \ <one><two>
```

| Word | Stack |
|---|---|
| `$VARIABLE` | `( "name" -- )`, a variable that holds an empty string |
| `$!` `$@` | `( c-addr u a -- )`, `( a -- c-addr u )`: store a copy, fetch (the empty string is `0 0`) |
| `$+!` | `( c-addr u a -- )`, append; the text may be part of the string itself |
| `$@LEN` | `( a -- u )` |
| `$!LEN` | `( u a -- )`, set the length; new characters are zero |
| `$+!LEN` | `( u a -- c-addr )`, make room for `u` more characters, give their address |
| `$INS` | `( c-addr u a pos -- )`, insert at offset `pos` (clamped to the string) |
| `$DEL` | `( a pos u -- )`, delete `u` characters at `pos` (clamped) |
| `$SPLIT` | `( c-addr u char -- c-addr u1 c-addr2 u2 )`, at the first `char`; nothing is allocated; without `char`, `u1` = `u` and `u2` = 0 |
| `$ITER` | `( .. a char xt -- .. )`, `xt` is `( .. c-addr u -- .. )`, once for each part between the `char`s; the empty string has no parts |
| `$INIT` `$FREE` | `( a -- )`: set to empty without freeing; free and set to empty |
| `$TMP` | `( xt -- c-addr u )`: run `xt` and give what it printed; valid until the next `$TMP` |
| `$EXEC` | `( xt a -- )`: run `xt` and append what it printed to the string |
| `$SLURP` `$SLURP-FILE` | `( fid a -- )`, `( c-addr u a -- )`: the whole file into the string |
| `$[]VARIABLE` | `( "name" -- )`, an empty array |
| `$[]` | `( n a -- a' )`: the cell of string `n`; the array grows to hold it |
| `$[]!` `$[]@` | `( c-addr u n a -- )`, `( n a -- c-addr u )`; `$[]@` past the end gives `0 0` and does not grow the array |
| `$[]#` | `( a -- n )` |
| `$[]+!` | `( c-addr u a -- )`: add the string as a new last element |
| `$[]MAP` | `( a xt -- )`, `xt` is `( c-addr u -- )` for each element |
| `$[]SLURP` `$[]SLURP-FILE` | `( fid a -- )`, `( c-addr u a -- )`: the lines of a file (`CR LF` too) become the elements; old elements are freed |
| `$[]FREE` | `( a -- )`: free the elements and the array |
| `$[]SORT` | `( a -- )`: put the elements in order, by the bytes of the text; stable |

A string that `$ITER` or `$[]MAP` is going through must not be changed by `xt`.

## Text words

The word list `STRINGS` (`ALSO STRINGS`) has the everyday words for a text on `( c-addr u )`. It comes
with the dynamic strings (same extension, same hosts), and keeps a standard program's dictionary
clean. Only `JOIN` allocates. `UPPER` and `LOWER` change the text in place, so give them a buffer
you own, not a literal; they work on ASCII letters and leave other bytes (UTF-8 included) alone.
White space here is every character up to the blank.

```forth
ALSO DYNAMIC-STRINGS ALSO STRINGS
S"   Hello, world  " TRIM TYPE                 Hello, world
S" forthling" S" forth" STRING-PREFIX? .       -1
$[]VARIABLE A
S" a" A $[]+!   S" b" A $[]+!   S" c" A $[]+!
A S" , " JOIN 2DUP TYPE DROP FREE THROW        a, b, c
```

| Word | Stack |
|---|---|
| `TRIM` `-LEADING` | `( c-addr u -- c-addr' u' )`: drop white space at both ends, or at the start (`-TRAILING` is standard); nothing is copied |
| `UPPER` `LOWER` | `( c-addr u -- c-addr u )`: change ASCII letters in place |
| `STRING-PREFIX?` `STRING-SUFFIX?` | `( c-addr u c-addr2 u2 -- flag )`: the first text starts, or ends, with the second |
| `CONTAINS?` | `( c-addr u c-addr2 u2 -- flag )`: the second text occurs in the first; an empty one always does |
| `JOIN` | `( array c-addr u -- c-addr' u' )`: the elements of a `$[]` array with the text between them, in an `ALLOCATE`d string; `FREE` it |

For one split at a character or a walk over all parts, use `$SPLIT` and `$ITER`; for templates with
named values, `SUBSTITUTE`.

## Collections

The word list `COLLECTIONS` (`ALSO COLLECTIONS`) has vectors, dictionaries and a sort. A vector is a list of
cells that grows; a dictionary maps text keys to cells and keeps the keys in the order they were added, so a
program prints the same output every time. Both live in `ALLOCATE`d blocks, like the dynamic strings, so they
use the heap and count towards the memory limit, and `VEC-FREE` and `DICT-FREE` give the memory back. The
forthling program, the MCP server, the Python module and the playground install it; a Rust program does with
`forth.install(&forthling::Collections)` (extension `FORTHLING-COLLECTIONS`). The names start with `VEC` and
`DICT`, so they do not clash with `MAP` and `FIND` of `FUNCTIONAL`.

```forth
ALSO COLLECTIONS ALSO JSON
DICT-VARIABLE D
3 S" apples" D DICT!   5 S" pears" D DICT!   2 S" figs" D DICT!
S" pears" D DICT@ . .                           \ -1 5
: ENTRY ( c-addr u n -- )  >R JSON-KEY R> JSON-NUMBER ;
JSON{ D ' ENTRY DICT-EACH JSON}                 \ {"apples":3,"pears":5,"figs":2}
VEC-VARIABLE V
: FILL  10 0 DO I 7 * 11 MOD V VEC+! LOOP ;
FILL  V ' < VEC-SORT
: SHOW  V VEC# 0 DO I V VEC@ . LOOP ;
SHOW                                            \ 0 1 2 3 5 6 7 8 9 10
```

| Word | Stack |
|---|---|
| `VEC-VARIABLE` | `( "name" -- )`, an empty vector; it uses no memory until the first `VEC+!` |
| `VEC+!` | `( x v -- )`: add `x` as the new last element |
| `VEC#` | `( v -- n )` |
| `VEC@` `VEC!` | `( i v -- x )`, `( x i v -- )`: element `i`, counted from 0; -24 when there is none |
| `VEC-POP` | `( v -- x )`: remove and give the last element; -24 when the vector is empty |
| `VEC-CLEAR` | `( v -- )`: remove all elements, keep the memory |
| `VEC-DATA` | `( v -- addr n )`: the elements as a block of `n` cells, for `SUM-CELLS`, `MAP-CELLS` and the other `-CELLS` words; the address is good until the next `VEC+!` |
| `VEC-SORT` | `( v xt -- )`: `SORT-CELLS` on the elements |
| `VEC-FREE` | `( v -- )`: free the memory; the vector can be used again |
| `DICT-VARIABLE` | `( "name" -- )`, an empty dictionary |
| `DICT!` | `( x c-addr u d -- )`: store `x` under the key; the key is copied; an old value of the key is replaced |
| `DICT@` | `( c-addr u d -- x true \| false )` |
| `DICT-DELETE` | `( c-addr u d -- flag )`: true when the key was there |
| `DICT#` | `( d -- n )`, the number of keys |
| `DICT-EACH` | `( d xt -- )`, `xt` is `( c-addr u x -- )`, once for each key in the order they were added; `xt` must not add or delete keys |
| `DICT-FREE` | `( d -- )`: free the keys and the tables; the dictionary can be used again |
| `SORT-CELLS` | `( addr n xt -- )`: sort `n` cells in place; `xt` is `( a b -- flag )`, true when `a` goes before `b`. The sort is stable. When `xt` throws, the cells stay as they were |

Keys are compared byte for byte, so the case counts, and the empty key is a key. Keys are text only; to
use numbers as keys, write them as text first. `$[]SORT ( a -- )` in `DYNAMIC-STRINGS` puts a `$[]` array in
order by the bytes of its strings (the sort is stable).

## Fixed-point numbers

The word list `FIXED` (`ALSO FIXED`) has numbers with a fraction that live in one cell, with integer
arithmetic only. It is for exact sums (money, percentages: `0.1E 0.2E F+ 0.3E F=` is false, and Python
gives `0.1 + 0.2` = `0.30000000000000004`) and for fractals, where speed and the same result everywhere
count. It comes with every host (extension `FORTHLING-FIXED`).

A number is written `1.5fx`, `-0.25fx` or `3fx`, while `FIXED` is in the search order. Without any setting
it counts in 9 decimals (about ±9.2 billion), so `0.10fx 0.20fx FX+` is exactly `0.30fx`. A literal that the
scale cannot hold exactly is an error (−24), never a quiet rounding: `0.0000000001fx` has too many decimals,
and so has `0.1fx` in binary.

You can choose the scale, once, before you define words:

- `n FX-DECIMALS` counts in decimals (0 to 18). The start is 9.
- `n FX-BITS` counts in binary (0 to 32 fraction bits). 32 is Q32.32, from about −2 billion to +2 billion in
  steps of 2⁻³². Binary is fast and fits fractals, but most decimal fractions (`0.1`) cannot be exact in it.
- `TRUE FX-ROUND-LITERALS` lets a literal that does not fit exactly be rounded (`FALSE` forbids it again).
  Fractals need it in binary: `32 FX-BITS TRUE FX-ROUND-LITERALS`.

```forth
ALSO FIXED
0.10fx 0.20fx FX+ 0.30fx = .   -1
19.99fx 3 S>FX FX* FX.         59.970000000
2 FX-DECIMALS
10fx 3fx FX/ FX.               3.33
32 FX-BITS TRUE FX-ROUND-LITERALS
2fx FXSQRT FX.                 1.4142135624
```

`FX+` and `FX-` are always exact. `FX*` and `FX/` calculate in 128 bits and then round back to the scale, as
you choose (`FX-NEAREST`, the default, half away from zero; `FX-FLOOR`; `FX-TRUNC`): `19.99fx × 0.15fx` at
2 decimals is 2.9985 and becomes 3.00. A result that does not fit a cell throws −11, so a sum never wraps
around quietly. Comparing, `MIN`, `MAX`, `0<` and the like work on the cells as they are; so does `*` with a
whole number.

| Word | Stack |
|---|---|
| `FX-DECIMALS` `FX-BITS` | `( n -- )`: the scale; `FX-SCALE ( -- n )` is the cell for 1.0 |
| `FX-ROUND-LITERALS` | `( flag -- )`: may a literal be rounded (default false) |
| `FX-NEAREST` `FX-FLOOR` `FX-TRUNC` | `( -- )`: the rounding |
| `FX+` `FX-` `FX*` `FX/` | `( fx1 fx2 -- fx3 )`; −11 when the result does not fit, −10 for `FX/` by zero |
| `FXNEGATE` `FXABS` `FXSQRT` | `( fx -- fx' )`; `FXSQRT` of a negative number is −24 |
| `S>FX` `FX>S` | `( n -- fx )` and `( fx -- n )`, the whole number rounded as set |
| `F>FX` `FX>F` | between the float stack and a fixed-point number |
| `>FX` | `( c-addr u -- fx true \| 0 false )`: read text such as `12.34` (it must fit exactly) |
| `FX.` `FX>STRING` | print with all places, or give `( c-addr u )` in the pictured-output buffer |

`FX.` prints all the places: the decimals, or for a binary scale enough digits (10 at Q32.32) to tell
neighbouring numbers apart. A word that is compiled keeps the cells it was compiled with, so set the scale
before you define words.

## Images

The word list `IMAGE` (`ALSO IMAGE`) is a canvas with no window and no dependencies: draw pixels and
lines, then write the picture to a file. So the program, MCP, the Python module and CI can all make
pictures. It comes with every host (extension `FORTHLING-CANVAS`).

`w h CANVAS` makes the canvas, all black, at most 4096 by 4096 pixels. A colour is one cell,
`0xRRGGBB`; `r g b RGB` makes it. (0, 0) is the top left. Drawing outside the canvas is ignored; reading a
pixel outside it, a bad size or drawing before there is a canvas is −24.

| Word | Stack |
|---|---|
| `CANVAS` `CANVAS-SIZE` | `( w h -- )` and `( -- w h )` |
| `RGB` | `( r g b -- colour )`, each 0 to 255 |
| `CLEAR` | `( colour -- )` fill the canvas |
| `PIXEL` `PIXEL@` | `( x y colour -- )` and `( x y -- colour )` |
| `LINE` | `( x1 y1 x2 y2 colour -- )`, both end points included |
| `PALETTE!` `PALETTE@` | `( colour i -- )` and `( i -- colour )`: 256 entries; the index is taken modulo 256, so an iteration count goes straight in. Entry 0 is black, the rest go round the colour wheel |
| `SHOW` | `( -- )`: print the picture in the terminal, two pixels in each character (`▀` with 24-bit colour; shades with `NO_COLOR` set), as wide as `COLUMNS` says (80 if not set); a wider canvas is averaged down |
| `SAVE-PPM` `SAVE-PNG` | `( c-addr u -- ior )`: write the picture; 0 when it worked |

The files go through the file system of the host (see [Files](embedding.md#files)), so they need the permission for files:
`-259` without it. On the command line they land on the disk; in MCP they are files in memory. The PNG is
not compressed (stored deflate blocks), so it is larger than a viewer's own export, but every viewer reads
it. `examples/book002_example001_mandelbrot.fs` draws the Mandelbrot set and a Julia set with the fixed-point words of the
previous section (`32 FX-BITS`), and writes `mandelbrot.ppm` and `julia.png`:
`cargo run --release -- examples/book002_example001_mandelbrot.fs`. Add `SHOW` to see a picture in a terminal that shows
24-bit colour. For a native window, `window/` adds the word `SHOW-WINDOW` in the word list `WINDOW`
(`ALSO WINDOW`): it opens a window with the picture and waits until the window is closed or Esc is
pressed. It needs the permission `System`, and `forthling-window` is the program with it; see
[window/README.md](../window/README.md).

In the [playground](https://deschaker.github.io/forthling/) the picture shows below the transcript as soon as a
run has drawn on the canvas, with a button to save it as a PNG; the Mandelbrot example there is a smaller
version of the same program (128 by 80 pixels, 50 iterations at most, about 5.9 million steps, to stay within
the limit of 10 million steps of a run). A Rust host that wants to show the picture itself calls
`forth.take_canvas()`: it gives `(width, height, rgba)` once after the canvas was drawn on, and `None` when
nothing changed.

## Operating system

The word list `OPERATING-SYSTEM` (`ALSO OPERATING-SYSTEM`) has the operating-system words of
Gforth: start a program, read the environment and the command line, walk directories, and use
the standard streams. The forthling program installs it; the MCP server, the Python module and
the playground install it too, but keep the machine closed (see below); a Rust program does with
`forth.install(&forthling::OperatingSystem)`. The behaviour is built from the Gforth manual, not
from the Gforth source. A script can start with a `#!` line, which is skipped.

```forth
#! /usr/bin/env forthling
ALSO OPERATING-SYSTEM
S" echo hello" R/O OPEN-PIPE THROW  DUP >R
PAD 80 ROT READ-LINE THROW DROP PAD SWAP TYPE CR     \ hello
R> CLOSE-PIPE THROW .                                \ 0, the exit status
S" src/lib.rs" BASENAME TYPE SPACE
S" src/lib.rs" DIRNAME TYPE                          \ lib.rs src/
```

| Word | Stack |
|---|---|
| `SYSTEM` | `( c-addr u -- )`: run the command in the shell of the system and wait; its exit status is `$?` |
| `SH` | `( "rest-of-line" -- )`, the same for the rest of the line |
| `$?` | `( -- n )`, the exit status of the last `SYSTEM`, `SH` or `CLOSE-PIPE` |
| `OPEN-PIPE` `CLOSE-PIPE` | `( c-addr u fam -- fileid ior )`, `( fileid -- status ior )`: `R/O` reads the output of the command, `W/O` writes to its input; the file words work on it (a pipe cannot be repositioned) |
| `GETENV` | `( c-addr u -- c-addr' u' )`: the value of an environment variable; `u'` is 0 if it is not set |
| `ARGC` `ARGV` | variables: the number of arguments (the program name included) and the address of a vector of zero-terminated strings, as in C |
| `ARG` | `( u -- c-addr u' )`: argument `u`; `0 ARG` is the program; `0 0` past the last |
| `NEXT-ARG` `SHIFT-ARGS` | `( -- c-addr u )`, `( -- )`: take the first argument after the program away; `0 0` if none |
| `OPEN-DIR` `READ-DIR` `CLOSE-DIR` | `( c-addr u -- dirid ior )`, `( c-addr u1 dirid -- u2 flag ior )`, `( dirid -- ior )`: `READ-DIR` gives the next name (not `.` and `..`) in the buffer, flag false at the end |
| `GET-DIR` `SET-DIR` | `( c-addr u -- c-addr u' )`, `( c-addr u -- ior )`: the current directory |
| `MKDIR-PARENTS` | `( c-addr u mode -- ior )`: the directory and the ones above it; one that exists is no error |
| `BASENAME` `DIRNAME` | `( c-addr u -- c-addr' u' )`: the file name, or the directory with its last `/` (empty if none) |
| `FILENAME-MATCH` | `( c-addr u c-addr2 u2 -- flag )`: does the name match the pattern (`*` and `?`) |
| `STDIN` `STDOUT` `STDERR` | `( -- fileid )`: the standard streams as files; `STDIN` and `STDOUT` are the input and output of the Forth system itself, `STDERR` is standard error |
| `OUTFILE-EXECUTE` `INFILE-EXECUTE` | `( xt fileid -- )`: run `xt` with `TYPE` writing to the file, or with `KEY` and `ACCEPT` reading from it |

The arguments of the forthling program are the files it still has to include: a script that
calls `NEXT-ARG` takes the next file for itself, so `forthling script.fs data.txt` lets the
script read `data.txt` without including it. A command started by `SYSTEM` or a pipe writes to
the real standard output, not through the Forth output. The words that touch the machine need
`Permission::System`, which `Forth::sandboxed()` and `--sandbox` deny: they then throw -259, or
give it as the ior. The MCP server always denies it, and the Python module when `sandbox=True`.
`BASENAME`, `DIRNAME`, `FILENAME-MATCH`, `$?`, `STDIN` and `STDOUT` need no permission.

## Benchmarks

The word list `BENCHMARK` (`ALSO BENCHMARK`) times code from Forth, with the clock that
`UTIME` and `CPUTIME` use, so a program, or an agent, can measure its own words. The forthling
program, the MCP server, the Python module and the playground install it; a Rust program does
with `forth.install(&forthling::Benchmark)`. In the playground the clock is
`performance.now()`, which a browser rounds (how much depends on the browser), so time a word that runs for
milliseconds, or use `BENCH-BEST`. The words need no permission.

```forth
ALSO BENCHMARK
: SQUARES  1000000 0 DO I DUP * DROP LOOP ;
' SQUARES BENCH .                \ the time of one run, in microseconds
5 ' SQUARES BENCH-BEST .         \ the shortest time of 5 runs
```

| Word | Stack |
|---|---|
| `BENCH` | `( xt -- microseconds )`: run `xt` once, which must have the effect `( -- )`, and give the time it took |
| `BENCH-BEST` | `( n xt -- microseconds )`: run `xt` `n` times (at least 1) and give the shortest time, which is the least disturbed by the machine |

An error in `xt` comes through, and so do the limits of the sandbox. The time is a wall-clock
time, so a `MS` in `xt` counts.

The programs of `cargo bench` (`benches/bench.rs`) are also Forth files that time themselves:
`forthling benches/loop.fs` (the empty `DO LOOP`, 30 million trips), `benches/fib.fs` (`27 FIB`)
and `benches/definitions.fs` (5000 definitions and constants, and lookups among them). Each
prints the name of the case and the time in ms. The start of a system stays with `cargo bench`,
since a Forth program cannot time the start of its own system.
