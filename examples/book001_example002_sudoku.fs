\ book001_example002_sudoku.fs - solve a sudoku by backtracking, with bitmasks for the candidates.
\ Run: cargo run --release -- examples/book001_example002_sudoku.fs
\ Needs the core only; it also runs in the playground.
\
\ How it works:
\   1. The board is 81 cells in a row: cell i is in row i/9 and column i mod 9. A digit from 1 to 9
\      is one bit in a number: 1 is bit 0, 9 is bit 8. A number with the bits of the digits already
\      used is a set of digits. There is one such set for each row, each column and each 3 x 3 box.
\   2. The candidates for a cell are the digits that are in none of its three sets.
\   3. SOLVE takes the empty cell with the fewest candidates, tries each candidate in turn and
\      calls itself for the rest of the board. If that fails, the digit is taken away again
\      (backtracking) and the next candidate is tried. When no cell is empty, the board is solved.
\ Stack comments like ( d i -- ) show what a word takes and leaves; the last item is the top.

\ --- the board and the sets of used digits ---
CREATE GRID   81 CELLS ALLOT           \ the digit of each cell, 0 when empty
CREATE ROWS    9 CELLS ALLOT           \ the digits used in each row, as bits
CREATE COLS    9 CELLS ALLOT           \ ... in each column
CREATE BOXES   9 CELLS ALLOT           \ ... in each 3 x 3 box
VARIABLE NODES                         \ how many times SOLVE has been called

: SLOT  ( n table -- addr )  SWAP CELLS + ;       \ the address of element n
: ROW   ( i -- r )  9 / ;
: COL   ( i -- c )  9 MOD ;
: BOX   ( i -- b )  9 /MOD  3 / 3 *  SWAP 3 /  + ;   \ /MOD gives column, then row; box = row/3*3 + column/3
: XOR!  ( b addr -- )  DUP @ ROT XOR SWAP ! ;       \ flip the bits b in the cell at addr

: FLIP  ( d i -- )   \ put digit d in the sets of cell i, or take it out: XOR does both
  SWAP 1- 1 SWAP LSHIFT {: cell bit :}       \ bit is the bit of the digit; not named i or b: i would hide the loop index I
  bit cell ROW ROWS  SLOT XOR!
  bit cell COL COLS  SLOT XOR!
  bit cell BOX BOXES SLOT XOR! ;

: PUT    ( d i -- )  2DUP FLIP  CELLS GRID + ! ;                  \ write digit d in cell i
: UNPUT  ( i -- )    DUP CELLS GRID + @  OVER FLIP  0 SWAP CELLS GRID + ! ;   \ empty cell i again

: CANDS  ( i -- m )  \ the digits that can go in cell i, as bits
  DUP ROW ROWS SLOT @  OVER COL COLS SLOT @ OR  SWAP BOX BOXES SLOT @ OR
  INVERT 511 AND ;                      \ not used, and only the nine bits of the digits

: BITS   ( m -- n )  \ how many bits are set: clear the lowest one until none is left
  0 SWAP BEGIN DUP WHILE  DUP 1- AND  SWAP 1+ SWAP  REPEAT DROP ;

\ --- solve ---
: BEST  ( -- i n )  \ the empty cell with the fewest candidates, and that number; n is 10 if no cell is empty
  -1 10
  81 0 DO
    I CELLS GRID + @ 0= IF
      I CANDS BITS  DUP 2 PICK < IF  NIP NIP I SWAP  ELSE DROP THEN
    THEN
  LOOP ;

: SOLVE  ( -- f )   \ true when the board can be completed
  1 NODES +!
  BEST DUP 10 = IF 2DROP TRUE EXIT THEN      \ nothing empty: solved
  0= IF DROP FALSE EXIT THEN                 \ an empty cell without candidates: dead end
  DUP CANDS {: cell mask :}
  10 1 DO                                    \ try the digits 1 to 9
    mask 1 I 1- LSHIFT AND IF                   \ is this digit a candidate?
      I cell PUT
      RECURSE IF UNLOOP TRUE EXIT THEN       \ the rest of the board worked out
      cell UNPUT                                \ it did not: take the digit away
    THEN
  LOOP FALSE ;

\ --- load and print ---
: CLEAR-BOARD  ( -- )  GRID 81 CELLS ERASE  ROWS 9 CELLS ERASE  COLS 9 CELLS ERASE  BOXES 9 CELLS ERASE  0 NODES ! ;

: LOAD  ( c-addr u -- )   \ 81 characters, a digit or a dot for each cell
  CLEAR-BOARD
  0 DO
    DUP I + C@ [CHAR] 0 -               \ the digit; a dot gives a negative number
    DUP 0 > IF I PUT ELSE DROP THEN
  LOOP DROP ;

: .BOARD  ( -- )
  9 0 DO
    I 3 MOD 0=  I 0<> AND IF ." ------+-------+------" CR THEN
    9 0 DO
      I 3 MOD 0=  I 0<> AND IF ." | " THEN
      J 9 * I + CELLS GRID + @  ?DUP IF . ELSE ." . " THEN
    LOOP CR
  LOOP ;

: PUZZLE  ( -- c-addr u )
  S" 8..........36......7..9.2...5...7.......457.....1...3...1....68..85...1..9....4.." ;

: MAIN  ( -- )
  PUZZLE LOAD
  ." The puzzle:" CR CR
  .BOARD CR
  SOLVE IF
    ." Solved after " NODES @ . ." calls of SOLVE:" CR CR
    .BOARD
  ELSE
    ." There is no solution." CR
  THEN ;

MAIN
