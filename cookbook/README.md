# The forthling cookbook

Complete programs that solve a problem, and show a few words of forthling on the way.
Each part has a theme, so you learn the language by building something. The parts are
grouped in books:

| Book | Theme | Needs |
|---|---|---|
| 1. Puzzles | a maze, a sudoku, linear programming | the core only; also runs in the playground |
| 2. Pictures | Game of Life and fractals on the canvas | the core; a window needs `forthling-window` |
| 3. Data and tools | JSON, CSV to SQLite, regex, HTTP, a command-line tool | some parts need `forthling-sqlite`, `forthling-http` or the `forthling` program |
| Guide | Forth for people who know Python | the core |

The books are written part by part; this page lists the parts that exist. The plan and
the open parts are in [issue #59](https://github.com/DeSchaker/forthling/issues/59).

## Parts

**Book 1, Puzzles**

- [A maze: make it and solve it](book001_part001_maze.md): depth-first backtracking and
  breadth-first search with vectors and a small random number generator
  ([`book001_example001_maze.fs`](../examples/book001_example001_maze.fs)).
- [A sudoku: solve it by backtracking](book001_part002_sudoku.md): bitmasks for the digits
  in each row, column and box, and recursion
  ([`book001_example002_sudoku.fs`](../examples/book001_example002_sudoku.fs)).
- [Linear programming: the simplex method](book001_part003_simplex.md): a production mix
  solved on a table of fixed-point numbers, with the shadow prices of the resources
  ([`book001_example003_simplex.fs`](../examples/book001_example003_simplex.fs)).

**Book 2, Pictures**

- [Game of Life: a world on the canvas](book002_part001_life.md): two grids, live cells that
  count themselves into their neighbours, and a colour by age
  ([`book002_example002_life.fs`](../examples/book002_example002_life.fs)), and an animated
  version for the terminal
  ([`book002_example004_life_animated.fs`](../examples/book002_example004_life_animated.fs)).
- [Fractals: Mandelbrot, Julia and Sierpinski](book002_part002_fractals.md): fixed-point
  numbers for the Mandelbrot and Julia sets
  ([`book002_example001_mandelbrot.fs`](../examples/book002_example001_mandelbrot.fs)), and
  the Sierpinski triangle from Pascal's triangle and by the chaos game
  ([`book002_example003_sierpinski.fs`](../examples/book002_example003_sierpinski.fs)).
- [The Clifford attractor: a picture from one formula](book002_part003_clifford.md): a point
  that jumps on the float stack, a count of hits per pixel, and a colour by the logarithm of
  the count, so the picture looks like smoke
  ([`book002_example005_clifford.fs`](../examples/book002_example005_clifford.fs)).
- [The De Jong attractor: four numbers, many pictures](book002_part004_dejong.md): the same
  counting with another jump, the four numbers given to each picture on the float stack, and
  what one changed sign does
  ([`book002_example006_dejong.fs`](../examples/book002_example006_dejong.fs)).
- [A fractal of spheres, by sphere tracing](book002_part005_apollonian.md): a picture of
  something that is not stored, only a word that says how far a point is from it; rays walk
  in steps of that distance, and the shade is the number of steps. Heavy: it does not run in
  the playground
  ([`book002_example007_apollonian.fs`](../examples/book002_example007_apollonian.fs)).
- [Domain-warped noise: small words that compose](book002_part006_warp.md): a hash, noise
  from it, octaves of noise, and `WARP`, a word that moves a point by the noise and so can be
  applied to its own result; three pictures, from plain noise to ink in water. Heavy: it does
  not run in the playground
  ([`book002_example008_warp.fs`](../examples/book002_example008_warp.fs)).
- [Truchet tiles, cut into smaller tiles](book002_part007_truchet.md): a square is one tile or
  is cut into four and each quarter is treated the same way (`RECURSE`); the tile is two quarter
  circles drawn with the midpoint algorithm and clipped to the square
  ([`book002_example009_truchet.fs`](../examples/book002_example009_truchet.fs)).

**Book 3, Data and tools**

- [JSON: read a document, write a summary](book003_part001_json_orders.md): the orders of a
  shop read with `JSON-GET`, `JSON-NTH` and `JSON@`, totals in whole cents, and a summary
  written with `<JSON ... JSON>`
  ([`book003_example001_json_orders.fs`](../examples/book003_example001_json_orders.fs)).
- [Word count: a dictionary and a sort](book003_part002_word_count.md): the words of a text
  counted in a dictionary, then put in order of count with `SORT-CELLS`
  ([`book003_example002_word_count.fs`](../examples/book003_example002_word_count.fs)).
- [Template: fill in a letter with SUBSTITUTE](book003_part003_template.md): one template
  and two letters with `REPLACES` and `SUBSTITUTE`, prices with `<# ... #>`, and `UNESCAPE`
  for text from a user
  ([`book003_example003_template.fs`](../examples/book003_example003_template.fs)).
- [A command-line tool: arguments with NEXT-ARG](book003_part004_cli_tool.md): a small
  `lines` tool with options and file names, the same `MAIN` for the real command line and a
  fake one (`DEFER`), and `READ-LINE`
  ([`book003_example004_cli_tool.fs`](../examples/book003_example004_cli_tool.fs)).
- [A benchmark: two ways to count primes](book003_part005_benchmark.md): trial division
  against the sieve, timed with `BENCH-BEST`, and a test that does not depend on the clock
  ([`book003_example005_benchmark.fs`](../examples/book003_example005_benchmark.fs)).
- [CSV to SQLite: load a table and ask questions](book003_part006_csv_to_sqlite.md): the
  lines of a CSV cut with `$SPLIT` and inserted with a prepared statement in one transaction,
  bad rows skipped and counted, then `GROUP BY` for the sums; needs `forthling-sqlite`
  ([`book003_example006_csv_to_sqlite.fs`](../sqlite/examples/book003_example006_csv_to_sqlite.fs)).
- [An HTTP call: ask a JSON API a question](book003_part007_http_json.md): `HTTP-GET` with
  headers, the status as a number, fields read with `JSON@`, a header of the answer, and a
  host that is not allowed; needs `forthling-http` and a network, so no test runs it
  ([`book003_example007_http_json.fs`](../http/examples/book003_example007_http_json.fs)).

**Programs without a chapter yet**, which also run in the tests, are in
[`examples/`](../examples/):

- [`book000_example001_demo.fs`](../examples/book000_example001_demo.fs): a few classic Forth examples.

## Running an example

```
cargo run --release -- examples/book000_example001_demo.fs
```

Every example is a file in `examples/` with its expected output in a file of the same
name that ends in `.out`. `cargo test` runs every program there and compares what it
prints with that file (`tests/examples.rs`), so the text of a book cannot drift from
the code. An example that needs an extension says so at the top of the file and is run by
the tests of that extension: such an example lives in the `examples/` of that crate (for
SQLite, `sqlite/examples/`), with its `.out` beside it.

## Adding an example

1. Write `examples/book001_example001_maze.fs`. The name says the book, the number of the
   example in that book and the theme: `book<NNN>_example<MMM>_<theme>`, for instance
   `book001_example003_queens.fs`. Start the file with a comment that says what it does, how
   to run it and what it needs.
2. Run it and check the output by eye. Save it as `examples/book001_example001_maze.out`.
3. Run `cargo test --test examples`.

`book000` holds the programs that belong to no book, such as the demo. Three digits leave
room for many books and many examples.

The chapters are in English. A number or timing in the text is measured, never made up.
