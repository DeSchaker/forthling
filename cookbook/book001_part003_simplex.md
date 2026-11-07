# Linear programming: the simplex method with fixed-point numbers

*Book 1, Puzzles. Needs the core only (the word list `FIXED`).*

The program: [`examples/book001_example003_simplex.fs`](../examples/book001_example003_simplex.fs).
Its output is beside it, in `book001_example003_simplex.out`. Run it with:

```
cargo run --release -- examples/book001_example003_simplex.fs
```

## The problem

A workshop makes chairs, tables and shelves. Each product uses wood, labour hours and
paint, and the workshop has a fixed amount of each per week:

| | chair | table | shelf | available |
|---|---|---|---|---|
| wood (boards) | 2 | 4 | 2 | 70 |
| labour (hours) | 4 | 2 | 2 | 70 |
| paint (litres) | 2 | 2 | 4 | 60 |
| profit | 50 | 60 | 50 | |

How many of each should it make to earn the most? This is a *linear program*: the profit
and every limit are sums of the amounts times a fixed number. The *simplex method* solves
it by walking from one corner of the possible plans to a better one, until none is better.

The program finds 10 chairs, 10 tables and 5 shelves, for a profit of 1350 a week, in three
steps and about 0.03 seconds (a release build).

## Why fixed-point numbers

The method is a lot of multiplying, dividing and subtracting of a table of numbers. With
the word list `FIXED` every number is one cell with 9 decimals, and `FX+`, `FX-`, `FX*` and
`FX/` do the sums with integers only:

```forth
ALSO FIXED
CREATE PROFIT   50fx , 60fx , 50fx ,
```

A literal such as `50fx` or `0.5fx` is a fixed-point number while `FIXED` is in the search
order. `FX+` and `FX-` are exact; `FX*` and `FX/` round to the 9th decimal. So the result
is not always exact: the table of this problem ends with a profit of 1350.000000027 and
10.000000002 chairs. The program prints them rounded to cents (`.CENTS`), which is how
money is shown anyway.

## The table

Every resource becomes a row. A *slack* column is added to each, the amount left over, so
that "at most" becomes "exactly": a row of the table says
`2 chairs + 4 tables + 2 shelves + 1 slack = 70`. The last row holds minus the profits, so
the cell at its right end will grow into the profit.

```forth
: TCELL  ( r c -- addr )  SWAP W * + CELLS TAB + ;
```

`TAB` is `H` rows of `W` cells, `W` being the products, the slacks and the available
column. `TCELL` turns a row and a column into the address. `LOAD-PROBLEM` copies the data
in, with a 1 in the slack column of each row, and `BASIS` notes which variable each row
stands for (at first its own slack).

## One step

The method repeats three small questions.

**Which product to make more of?** The most negative number in the last row. It says how
much the profit rises for one more unit of that column. When none is negative, nothing
helps any more and the table is optimal:

```forth
: ENTERING  ( -- c | -1 )
  -1 0 {: col low :}
  W 1- 0 DO
    M I TCELL @ low < IF  I TO col  M I TCELL @ TO low  THEN
  LOOP  col ;
```

Fixed-point numbers are plain cells, so `<` and `0>` compare them as they are. `{: col low :}`
gives two values a name; `TO` changes one.

**How much can we make?** Each row with a positive number in that column is a limit:
`available / use` units fit. The smallest is the one that runs out first (`LEAVING`, with
`FX/` for the quotient). If no row has a positive number, nothing limits the product and the
profit has no limit; `SOLVE` then returns -1. (Give the shelf the use 0 in all three rows
and the program says "The profit has no limit.")

**Make it so: the pivot.** Divide the row that runs out by its number in the column, so
that the number becomes 1. Then subtract a multiple of that row from every other row, the
last one too, so that the column is 0 there:

```forth
: ELIM  ( r c row -- )
  {: r c row :}
  row c TCELL @ {: f :}
  W 0 DO  row I TCELL DUP @  r I TCELL @ f FX*  FX-  SWAP !  LOOP ;
```

The other rows now describe the plan with the new product in it. `SOLVE` is a loop of the
three questions with a line of output for each step:

```
Step 1: make more tables until the wood is used up
Step 2: make more chairs until the labour is used up
Step 3: make more shelves until the paint is used up
```

## Reading the answer

When the loop stops, `BASIS` says which variable is in each row and the right-hand column
holds its amount; a variable in no row is 0. `AMOUNT` looks that up. The last row has more:
under each slack column stands what one more unit of that resource would add to the profit.
The program prints it in brackets:

```
  wood: 70.00 of 70.00 (+10.00)
  labour: 70.00 of 70.00 (+5.00)
  paint: 60.00 of 60.00 (+5.00)
```

These are the *shadow prices*. They are checked: with 71 boards of wood instead of 70 the
program gives a profit of 1360. All three resources are used up, so every one of them is
worth something; a resource with room left over would show +0.00.

`USED` adds up what the plan uses of each resource with `FX*` and `FX+`, product by product
from the original data. It is not read from the table, so it is a check on the pivoting:
the numbers match the available amounts.

## Try it

- Change `PROFIT` or the lines of `USE` for another workshop. For a different number of
  products or resources change `N` and `M`, and the names in `.PRODUCT` and `.RESOURCE`.
- Take `23fx` for the profit of the table. Tables no longer pay: the program makes 13.33
  chairs and 8.33 shelves in two steps, and the wood is not used up (43.33 of 70.00), so
  its shadow price is +0.00.
- A coefficient such as `1.5fx` is fine. One that 9 decimals cannot hold exactly, such as
  a third, is an error as a literal (-24); write `0.333333333fx`.
- The program takes the usual form: every limit is "at most" with a positive amount. A
  limit of "at least" needs a first phase that this program does not have.
- On some problems the method can go round in a circle of pivots without gaining. This
  program does not guard against that; it does not happen with the numbers here.
