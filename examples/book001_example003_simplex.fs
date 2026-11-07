\ book001_example003_simplex.fs - linear programming: the simplex method in fixed-point numbers.
\ Run: cargo run --release -- examples/book001_example003_simplex.fs
\ Needs the core only (the word list FIXED); it runs in the sandbox too (--sandbox).
\
\ The problem: a workshop makes chairs, tables and shelves. Each needs wood, labour hours and
\ paint, and the workshop has so much of each per week. How many of each give the most profit?
\
\ How it works:
\   1. The problem is a table (the tableau). One row per resource: what a chair, a table and a
\      shelf use of it, then one slack column per resource (what is left over, starts as 1 for
\      its own row), then the amount available. The last row holds minus the profits.
\   2. Entering: the column of the last row with the most negative number is the product whose
\      extra unit raises the profit the most. When no number is negative, the table is optimal.
\   3. Leaving: of the rows where that column is positive, the one with the smallest
\      available / use runs out first, so it decides how many units we can make.
\   4. Pivot: divide that row by the pivot number, subtract multiples of it from all other rows
\      so that the column is 0 there, and note which variable is now in the row. Repeat from 2.
\ All numbers are fixed-point (FX+ FX- FX* FX/): the additions are exact, and a product or a
\ quotient is rounded to 9 decimals, the start of the word list FIXED.
\ Stack comments like ( d i -- ) show what a word takes and leaves; the last item is the top.

ALSO FIXED

\ --- the problem ---
3 CONSTANT N                           \ products: chairs, tables, shelves
3 CONSTANT M                           \ resources: wood, labour, paint

CREATE PROFIT   50fx , 60fx , 50fx ,   \ per chair, table, shelf
CREATE USE                             \ one line per resource: use per chair, table, shelf; available
   2fx ,  4fx ,  2fx ,  70fx ,         \ wood (boards)
   4fx ,  2fx ,  2fx ,  70fx ,         \ labour (hours)
   2fx ,  2fx ,  4fx ,  60fx ,         \ paint (litres)

\ --- the tableau ---
N M + 1+ CONSTANT W                    \ columns: products, slacks, available
M 1+ CONSTANT H                        \ rows: resources, then the profit row
CREATE TAB    H W * CELLS ALLOT
CREATE BASIS  M CELLS ALLOT            \ the variable that is in each row now
VARIABLE STEPS                         \ how many pivots

: TCELL  ( r c -- addr )  SWAP W * + CELLS TAB + ;

: LOAD-PROBLEM  ( -- )
  TAB H W * CELLS 0 FILL
  M 0 DO
    N 0 DO  J N 1+ * I + CELLS USE + @   J I TCELL !  LOOP     \ use of each product
    1fx  I N I + TCELL !                                       \ this row's own slack
    I N 1+ * N + CELLS USE + @   I W 1- TCELL !                \ available
    N I +  I CELLS BASIS + !                                   \ the slack is in the row at first
  LOOP
  N 0 DO  I CELLS PROFIT + @ FXNEGATE  M I TCELL !  LOOP ;

\ --- names, for the report ---
: .RESOURCE  ( i -- )  CASE 0 OF ." wood" ENDOF 1 OF ." labour" ENDOF ." paint" ENDCASE ;
: .PRODUCT   ( i -- )  CASE 0 OF ." chairs" ENDOF 1 OF ." tables" ENDOF ." shelves" ENDCASE ;
: .MOVE  ( j -- )   \ what it means to bring variable j into the plan
  DUP N < IF  ." make more " .PRODUCT  ELSE  ." leave some " N - .RESOURCE ."  unused"  THEN ;
: .CENTS  ( fx width -- )     \ print fx rounded to two decimals, right-aligned (for fx >= 0)
  >R 100 * FX>S  0 <# # # 46 HOLD #S #>  R> OVER - 0 MAX SPACES TYPE ;

\ --- one step of the simplex method ---
: ENTERING  ( -- c | -1 )   \ the column with the most negative number in the profit row
  -1 0 {: col low :}
  W 1- 0 DO
    M I TCELL @ low < IF  I TO col  M I TCELL @ TO low  THEN
  LOOP  col ;

: LEAVING  ( c -- r | -1 )  \ the row that runs out first, -1 when none does
  -1 -1 {: c row best :}
  M 0 DO
    I c TCELL @ DUP 0> IF
      I W 1- TCELL @ SWAP FX/                    \ available / use: how many units fit
      DUP best <  row 0< OR IF  TO best  I TO row  ELSE DROP THEN
    ELSE DROP THEN
  LOOP  row ;

: ELIM  ( r c row -- )      \ subtract a multiple of row r from row, so that column c becomes 0
  {: r c row :}
  row c TCELL @ {: f :}
  W 0 DO  row I TCELL DUP @  r I TCELL @ f FX*  FX-  SWAP !  LOOP ;

: PIVOT  ( r c -- )
  {: r c :}
  r c TCELL @ {: p :}
  W 0 DO  r I TCELL DUP @ p FX/ SWAP !  LOOP        \ the pivot number becomes 1
  H 0 DO  I r <> IF r c I ELIM THEN  LOOP
  c r CELLS BASIS + ! ;

: SOLVE  ( -- 0 | -1 )      \ 0 when the table is optimal, -1 when the profit has no limit
  BEGIN
    ENTERING DUP 0< IF DROP 0 EXIT THEN
    DUP LEAVING DUP 0< IF 2DROP -1 EXIT THEN
    1 STEPS +!
    ." Step " STEPS @ 0 .R ." : " OVER .MOVE ."  until the " DUP .RESOURCE ."  is used up" CR
    SWAP PIVOT
  AGAIN ;

\ --- the report ---
: AMOUNT  ( j -- fx )       \ the value of variable j: the available column of its row, or 0
  {: j :}
  0  M 0 DO  I CELLS BASIS + @ j = IF DROP I W 1- TCELL @ THEN  LOOP ;

: USED  ( i -- fx )         \ what the plan uses of resource i, added up product by product
  {: res :}
  0  N 0 DO  res N 1+ * I + CELLS USE + @  I AMOUNT FX*  FX+  LOOP ;

: REPORT  ( -- )
  CR ." Most profit per week: " M W 1- TCELL @ 0 .CENTS CR CR
  ." Make:" CR
  N 0 DO  ."   " I .PRODUCT ." : " I AMOUNT 0 .CENTS CR  LOOP CR
  ." Resources, used of available, and what one more unit of it would add to the profit:" CR
  M 0 DO
    ."   " I .RESOURCE ." : " I USED 0 .CENTS ."  of "
    I N 1+ * N + CELLS USE + @ 0 .CENTS
    ."  (+" M N I + TCELL @ 0 .CENTS ." )" CR
  LOOP ;

: MAIN  ( -- )
  LOAD-PROBLEM
  SOLVE IF ." The profit has no limit." CR EXIT THEN
  REPORT ;

MAIN
