\ book003_example006_csv_to_sqlite.fs - load the lines of a CSV file into a SQLite table and ask questions.
\ Run: cargo run --release --manifest-path sqlite/Cargo.toml -- sqlite/examples/book003_example006_csv_to_sqlite.fs
\ Needs the program forthling-sqlite (the word list SQLITE). The CSV is in the program, so it needs no file.
\
\ How it works:
\   1. The CSV lines are in a $[] array, as $[]SLURP-FILE would give them for a real file.
\   2. LOAD-LINE skips the header, empty lines and rows that do not have four fields.
\      It cuts the other rows at the commas with $SPLIT and binds each field as text.
\   3. The table has INTEGER and REAL columns, so SQLite turns the text into numbers itself.
\   4. All inserts run in one transaction, which is much faster for a large file.
\   5. SQL does the sums: GROUP BY for the totals per product, printf for the money.

ALSO DYNAMIC-STRINGS ALSO STRINGS ALSO SQLITE

\ THROWs a nonzero ior after showing the message of SQLite.
: ?DB ( ior -- )
  DUP IF  DB-ERROR 2DUP TYPE CR DROP FREE DROP  THEN THROW ;

\ ---- the CSV: one line per element, as a file would give them ----
$[]VARIABLE LINES
: LINE ( c-addr u -- )  LINES $[]+! ;

S" date,product,quantity,price" LINE
S" 2026-11-01,apple,12,0.50" LINE
S" 2026-11-01,pear,5,0.80" LINE
S" 2026-11-02,apple,7,0.50" LINE
S" 2026-11-02,cherry,2,6.25" LINE
S" " LINE
S" 2026-11-03,pear" LINE
S" 2026-11-03,cherry,1,6.25" LINE
S" 2026-11-03,pear,10,0.75" LINE
S" 2026-11-04,apple,20,0.45" LINE

\ ---- the database ----
S" :memory:" DB-OPEN ?DB VALUE DB
S" CREATE TABLE sales (day TEXT, product TEXT, quantity INTEGER, price REAL)" DB DB-EXEC ?DB
S" INSERT INTO sales VALUES (?1, ?2, ?3, ?4)" DB DB-PREPARE ?DB VALUE INSERT

VARIABLE LINE#   VARIABLE LOADED   VARIABLE SKIPPED

\ The number of commas in a text.
: COMMAS ( c-addr u -- n )
  0 -ROT  0 ?DO  DUP I + C@ [CHAR] , = IF SWAP 1+ SWAP THEN  LOOP DROP ;

\ Bind the four fields of a row to the parameters 1 to 4 of INSERT.
: BIND-ROW ( c-addr u -- )
  5 1 DO
    [CHAR] , $SPLIT  2SWAP          ( rest u field u )
    I INSERT DB-BIND-TEXT ?DB
  LOOP 2DROP ;

: INSERT-ROW ( c-addr u -- )
  BIND-ROW  INSERT DB-STEP ?DB DROP  1 LOADED +! ;

\ Called for each line: the header and empty lines are no data; a row with another
\ number of fields than four is skipped (and counted), not loaded.
: LOAD-LINE ( c-addr u -- )
  1 LINE# +!
  TRIM  LINE# @ 1 =  OVER 0= OR  IF 2DROP EXIT THEN
  2DUP COMMAS 3 = IF  INSERT-ROW  ELSE  2DROP 1 SKIPPED +!  THEN ;

S" BEGIN" DB DB-EXEC ?DB
LINES ' LOAD-LINE $[]MAP
S" COMMIT" DB DB-EXEC ?DB
INSERT DB-FINALIZE ?DB

LOADED @ . ." rows loaded, " SKIPPED @ . ." skipped" CR

\ ---- questions ----
: COL ( n stmt -- )  DB-COLUMN 2DUP TYPE DROP FREE THROW ;

: REPORT ( c-addr u -- )                \ run a SELECT with 3 columns and print the rows
  DB DB-PREPARE ?DB {: q :}
  BEGIN  q DB-STEP ?DB  WHILE
    0 q COL 2 SPACES  1 q COL 2 SPACES  2 q COL CR
  REPEAT
  q DB-FINALIZE ?DB ;

CR .( Per product: pieces and money) CR
S" SELECT product, sum(quantity), printf('%.2f', sum(quantity * price)) FROM sales GROUP BY product ORDER BY sum(quantity * price) DESC" REPORT

CR .( Per day: rows and money) CR
S" SELECT day, count(*), printf('%.2f', sum(quantity * price)) FROM sales GROUP BY day ORDER BY day" REPORT

DB DB-CLOSE ?DB
