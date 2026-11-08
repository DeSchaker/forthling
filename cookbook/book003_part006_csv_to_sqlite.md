# CSV to SQLite: load a table and ask questions

*Book 3, Data and tools. Needs the program `forthling-sqlite` (the word list `SQLITE`, with
the text words `DYNAMIC-STRINGS` and `STRINGS`). It does not run in the playground.*

The program: [`sqlite/examples/book003_example006_csv_to_sqlite.fs`](../sqlite/examples/book003_example006_csv_to_sqlite.fs).
Its output is beside it, in `book003_example006_csv_to_sqlite.out`. It lives in `sqlite/`
because the tests of that crate run it; the core does not have the SQLite words. Run it with:

```
cargo run --release --manifest-path sqlite/Cargo.toml -- sqlite/examples/book003_example006_csv_to_sqlite.fs
```

It loads nine lines of CSV, the sales of a market stall, into a table, skips what is not
data, and asks SQLite two questions. It takes under 12 000 steps (found by lowering
`--max-steps` until it stopped).

```
7 rows loaded, 1 skipped

Per product: pieces and money
cherry  3  18.75
apple  39  18.50
pear  15  11.50

Per day: rows and money
2026-11-01  2  10.00
2026-11-02  2  16.00
2026-11-03  2  13.75
2026-11-04  1  9.00
```

The sums can be checked by hand: 12 + 7 + 20 apples is 39, and 12 × 0.50 + 7 × 0.50 + 20 × 0.45
is 18.50.

## The lines

A file would come in with `$[]SLURP-FILE`, which makes each line an element of a `$[]` array.
The program puts the lines in the same kind of array by hand, so that it needs no file:

```forth
$[]VARIABLE LINES
: LINE ( c-addr u -- )  LINES $[]+! ;

S" date,product,quantity,price" LINE
S" 2026-11-01,apple,12,0.50" LINE
```

Each `S"` string is one line of source, which is why the CSV is a list of `LINE` calls. To
read a real file, replace them with `fid LINES $[]SLURP` after `OPEN-FILE`, or
`S" sales.csv" LINES $[]SLURP-FILE`. The CSV here has two lines that are not data rows: an
empty line and a row with only two fields.

## The table

```forth
S" CREATE TABLE sales (day TEXT, product TEXT, quantity INTEGER, price REAL)" DB DB-EXEC ?DB
S" INSERT INTO sales VALUES (?1, ?2, ?3, ?4)" DB DB-PREPARE ?DB VALUE INSERT
```

`?DB` is the helper from `people.fs`: it shows the message of SQLite for a nonzero `ior` and
`THROW`s it. The `INSERT` is prepared once, with four parameters, and used for every row.

## One row

`$SPLIT` cuts a text at the first comma and gives the part before it and the part after it, so
the four fields come out one by one. Each is bound as text:

```forth
: BIND-ROW ( c-addr u -- )
  5 1 DO
    [CHAR] , $SPLIT  2SWAP          ( rest u field u )
    I INSERT DB-BIND-TEXT ?DB
  LOOP 2DROP ;
```

The program never converts a number. The columns are declared `INTEGER` and `REAL`, and SQLite
turns the text `12` into the integer 12 and `0.50` into the float 0.5 when it stores it. A
column of a type that cannot be read as a number would keep the text.

## What is not data

`LOAD-LINE` is called for each line by `$[]MAP`:

```forth
: LOAD-LINE ( c-addr u -- )
  1 LINE# +!
  TRIM  LINE# @ 1 =  OVER 0= OR  IF 2DROP EXIT THEN
  2DUP COMMAS 3 = IF  INSERT-ROW  ELSE  2DROP 1 SKIPPED +!  THEN ;
```

The first line is the header, and an empty line (after `TRIM`) has nothing in it; both are
dropped without a word. A row with any other number of commas than three is skipped and
counted, so the program says `7 rows loaded, 1 skipped` and does not stop at the first bad
row, and does not load half a row. `COMMAS` counts them with a plain loop.

## One transaction

```forth
S" BEGIN" DB DB-EXEC ?DB
LINES ' LOAD-LINE $[]MAP
S" COMMIT" DB DB-EXEC ?DB
```

Without it SQLite makes every `INSERT` a transaction of its own, which for a database in a
file means a write to the disk for each row. With nine rows it makes no difference; with a
large file it is the first thing to do.

## The questions

`REPORT` runs a `SELECT` with three columns and prints the rows. `DB-COLUMN` gives each
column as an `ALLOCATE`d text, so `COL` prints it and frees it:

```forth
: COL ( n stmt -- )  DB-COLUMN 2DUP TYPE DROP FREE THROW ;
```

SQL does the sums. `GROUP BY product` gives one row for each product, and
`printf('%.2f', ...)` writes the money with two decimals, so the program has no float output
to format:

```sql
SELECT product, sum(quantity), printf('%.2f', sum(quantity * price))
FROM sales GROUP BY product ORDER BY sum(quantity * price) DESC
```

## Limits

- A field cannot contain a comma. Real CSV puts such a field in double quotes, and this
  program does not read those. The `CSV` words of `forthling-extras` do (`CSV-FIELD` reads
  quoted fields; see [extras/README.md](../extras/README.md)); they are not installed in
  `forthling-sqlite`, so this program keeps its own `$SPLIT`.
- The database is `:memory:`, so it is gone at the end. Give `DB-OPEN` a file name to keep
  it; that needs file permission, so it does not work in a sandbox.

## Try it

- Add a third report: the day with the most money, with `ORDER BY ... LIMIT 1`.
- Read the CSV from a file with `$[]SLURP-FILE`.
- Print the skipped lines, with their line numbers, instead of only counting them.
- Change one quantity to `12 pieces` and add `SELECT typeof(quantity), quantity FROM sales`.
  SQLite keeps the text in the column, and `sum` still reads it as 12. Decide whether the
  program should skip such a row.
