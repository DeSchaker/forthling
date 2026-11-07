# A sudoku: solve it by backtracking

*Book 1, Puzzles. Needs the core only.*

The program: [`examples/book001_example002_sudoku.fs`](../examples/book001_example002_sudoku.fs).
Its output is beside it, in `book001_example002_sudoku.out`. Run it with:

```
cargo run --release -- examples/book001_example002_sudoku.fs
```

It loads a puzzle from a string of 81 characters, prints it, solves it and prints the
solution. The puzzle in the file has 21 clues. The program solves it with 10102 calls of
`SOLVE`, which took about 0.8 seconds on the machine this was written on (a release build,
measured with three runs: 0.83, 0.84 and 1.05 seconds).

## The board and the sets of digits

The board is 81 cells in one row of memory. Cell `i` is in row `i / 9` and column
`i mod 9`. A cell holds a digit from 1 to 9, or 0 when it is empty.

To know quickly which digits a cell may take, the program keeps a *set of used digits* for
every row, every column and every 3 x 3 box. A set is one number with a bit for each
digit: digit 1 is bit 0, digit 9 is bit 8. The set {1, 3} is `101` in binary, which is 5.

```forth
CREATE ROWS    9 CELLS ALLOT
CREATE COLS    9 CELLS ALLOT
CREATE BOXES   9 CELLS ALLOT
: ROW   ( i -- r )  9 / ;
: COL   ( i -- c )  9 MOD ;
: BOX   ( i -- b )  9 /MOD  3 / 3 *  SWAP 3 /  + ;
```

`BOX` uses `/MOD`, which leaves the column first and the row on top. The box is
`row / 3 * 3 + column / 3`.

## Putting a digit in and taking it out

Adding a digit to a set and taking it out again are the same operation on the bit: `XOR`
flips it. So one word, `FLIP`, serves for both:

```forth
: FLIP  ( d i -- )
  SWAP 1- 1 SWAP LSHIFT {: cell bit :}
  bit cell ROW ROWS  SLOT XOR!
  bit cell COL COLS  SLOT XOR!
  bit cell BOX BOXES SLOT XOR! ;
```

`1 SWAP LSHIFT` makes the bit of the digit. `{: cell bit :}` gives the two numbers a name,
so that the three lines below need no stack juggling. `PUT` writes a digit in the board and
flips it into the sets; `UNPUT` reads the digit back, flips it out and empties the cell.

Mind the names of the locals. Forth is not case sensitive, so a local called `i` would hide
the loop index `I` inside the loop of `SOLVE`. The first version of this program did that,
and answered "no solution" for a puzzle that has one.

## Candidates

The candidates for a cell are the digits that are in none of its three sets:

```forth
: CANDS  ( i -- m )
  DUP ROW ROWS SLOT @  OVER COL COLS SLOT @ OR  SWAP BOX BOXES SLOT @ OR
  INVERT 511 AND ;
```

`OR` joins the three sets. `INVERT` turns it into the digits that are *not* used, and
`511 AND` (nine bits) cuts off the bits above digit 9. `BITS` counts how many bits are set
by clearing the lowest one (`DUP 1- AND`) until none is left.

## Backtracking

`SOLVE` does the search:

1. Take the empty cell with the fewest candidates (`BEST`).
2. If there is no empty cell, the board is full: solved.
3. If the cell has no candidates, this is a dead end: give up (false).
4. Otherwise try each candidate: put it in, call `SOLVE` for the rest of the board
   (`RECURSE`), and if that fails, take the digit away again (`UNPUT`) and try the next.

```forth
: SOLVE  ( -- f )
  1 NODES +!
  BEST DUP 10 = IF 2DROP TRUE EXIT THEN
  0= IF DROP FALSE EXIT THEN
  DUP CANDS {: cell mask :}
  10 1 DO
    mask 1 I 1- LSHIFT AND IF
      I cell PUT
      RECURSE IF UNLOOP TRUE EXIT THEN
      cell UNPUT
    THEN
  LOOP FALSE ;
```

Always taking the cell with the fewest candidates is what keeps the search small. A cell
with one candidate is forced, and a cell with none shows at once that an earlier choice was
wrong.

`UNLOOP` is needed before `EXIT` inside a `DO` loop, because the loop keeps its counters on
the return stack. `NODES` counts the calls, so the program can say how much work it did.
Forth words that use `IF` and loops must be inside a definition, which is why the last
lines of the file are a word, `MAIN`, and `MAIN` itself.

## Try it

- Change the string in `PUZZLE` for another puzzle. Use a digit for a clue and a dot for an
  empty cell, 81 characters in all, row after row.
- A puzzle with many solutions is solved too: `SOLVE` stops at the first one it finds. An
  empty board (81 dots) is filled in after 82 calls.
- A puzzle that cannot be solved prints "There is no solution.". Try a first row
  `12345678.` and a 9 below the last cell of that row (`12345678.` followed by eight dots
  and a 9): the empty corner has no candidate, and `SOLVE` says so at once.
- `LOAD` does not check the clues. Two equal digits in one row, column or box give a
  wrong board, not an error; a program for real use would check them first.
