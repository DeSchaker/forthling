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

**Programs without a chapter yet**, which also run in the tests, are in
[`examples/`](../examples/):

- [`book000_example001_demo.fs`](../examples/book000_example001_demo.fs): a few classic Forth examples.
- [`book002_example001_mandelbrot.fs`](../examples/book002_example001_mandelbrot.fs): the Mandelbrot set and a Julia set in fixed-point numbers.

## Running an example

```
cargo run --release -- examples/book000_example001_demo.fs
```

Every example is a file in `examples/` with its expected output in a file of the same
name that ends in `.out`. `cargo test` runs every program there and compares what it
prints with that file (`tests/examples.rs`), so the text of a book cannot drift from
the code. An example that needs an extension says so at the top of the file and is run by
the tests of that extension.

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
