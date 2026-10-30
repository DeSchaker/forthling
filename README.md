# forthling

> **forthling** *(noun)*
>
> 1. A diminutive creature thriving on a two-stack diet.
> 2. A lean, extensible Forth system built from the ground up.

A small Forth interpreter in Rust, with no dependencies, working towards the
[Forth-2012 standard](https://forth-standard.org/).

```
$ cargo run --release
forthling — WORDS shows the dictionary, BYE quits.
: SQUARE  DUP * ;
 ok
7 SQUARE .
49  ok
```

Run a file: `cargo run --release -- examples/demo.fs`

## Features

- A REPL and file execution (nested with `INCLUDE`)
- Colon definitions compiled to threaded code, with `RECURSE`, `IMMEDIATE`,
  `POSTPONE`, `LITERAL`, `[ ]`, `' EXECUTE` and `SEE`
- `IF ELSE THEN`, `BEGIN UNTIL/AGAIN/WHILE REPEAT`, `DO ?DO LOOP +LOOP LEAVE`,
  `CASE OF ENDOF ENDCASE`
- `VARIABLE`, `CONSTANT`, `VALUE`, `DEFER`, `BUFFER:`, `MARKER`, `:NONAME`, `CREATE … DOES>`
- Byte-addressed memory with 64-bit cells
- `SOURCE`, `>IN`, `STATE`, `WORD`, `PARSE`, `REFILL`, `SAVE-INPUT`, `FIND` and `EVALUATE`,
  working on input buffers in Forth memory
- Errors as standard THROW codes, e.g. `demo.fs:3: error -13: undefined word: FOO`,
  which `CATCH` can catch
- Programming tools: `.S ? DUMP SEE WORDS`, `[IF] [ELSE] [THEN]`, `[DEFINED] [UNDEFINED]`,
  `AHEAD CS-PICK CS-ROLL`, `N>R NR>` and `SYNONYM`

Part of the dictionary is written in Forth itself (`PRELUDE` in `src/main.rs`).

## Standard compliance

[`STANDAARD.md`](STANDAARD.md) (in Dutch) holds the roadmap to Forth-2012 and the
current status. The CORE, CORE EXT, EXCEPTION and TOOLS word sets are complete, and their
tests in the [Forth-2012 test suite](https://github.com/gerryjackson/forth2012-test-suite)
pass (`core.fr`, `coreplustest.fth`, `coreexttest.fth`, `exceptiontest.fth` and
`toolstest.fth`). Of TOOLS EXT, everything is present except `;CODE ASSEMBLER CODE EDITOR`
(there is no assembler or editor), the obsolescent `FORGET` (use `MARKER`), and
`NAME>COMPILE NAME>INTERPRET NAME>STRING TRAVERSE-WORDLIST`, which will come with the
SEARCH word set. Next up is STRING.

### Implementation-defined choices

- Cells are 64 bits; an address unit and a character are 8 bits.
- Division is floored: `-7 2 /` gives `-4` and `-7 2 MOD` gives `1`. `/ MOD /MOD */ */MOD`
  are defined via `FM/MOD`; `SM/REM` is available for symmetric division.
- A double cell on the stack has its high cell on top. Division by zero throws `-10`,
  a quotient that does not fit throws `-11`.
- The pictured numeric output buffer (`<# … #>`) holds 256 characters; overflowing it
  throws `-17`. `.` and `U.` use the same buffer.
- `ACCEPT` and `KEY` read from standard input, which the terminal buffers per line: `KEY`
  returns once a line is entered, and the line ending is a character too. `ACCEPT` keeps
  at most the requested number of characters and drops the rest of the line. At the end
  of input `ACCEPT` returns 0 and `KEY` throws `-39`.
- The data stack and the return stack hold 65536 cells each; overflowing them throws `-3`
  and `-5`. `ENVIRONMENT?` reports these sizes and the other standard queries.
- `ABORT` throws `-1` and `ABORT"` throws `-2`; both empty the stacks and return to the
  terminal with a message. `QUIT` keeps the data stack and prints nothing. When running
  files, `QUIT` ends the run, since there is no terminal to return to.
- `SOURCE-ID` is 0 for the terminal and -1 for `EVALUATE`. Without the FILE word set there
  are no file ids, so an `INCLUDE`d file gives its (positive) nesting depth.
- `RESTORE-INPUT` can return to any earlier line of a file, but at the terminal and in
  `EVALUATE` only within the current line; otherwise it returns true.
- `S\"` translates `\n` to a single line feed (10). Running a word made by `DEFER` before
  an action is set with `IS` throws `-21`.
- Every error is a THROW code that `CATCH` can catch, including errors the system itself
  detects (such as `-13` for an undefined word). `QUIT` is not an exception and is not
  caught. `CATCH` can be nested 1000 deep; one more throws `-5`. When a `THROW` unwinds a
  definition that was started inside the `CATCH`, that definition is dropped.
- `[IF]` and `[ELSE]` skip words across lines, reading further lines from the file or the
  terminal; if the input ends before the matching `[ELSE]` or `[THEN]`, skipping just stops.
  `DUMP` prints the address and 16 bytes per line in hexadecimal, plus their characters.

## License

[MIT](LICENSE)

## Testing

```
cargo test
```

The tests from the Forth-2012 test suite need the suite to be cloned first:

```
git clone https://github.com/gerryjackson/forth2012-test-suite.git tests/forth2012-test-suite
git -C tests/forth2012-test-suite checkout 4c87e56
```
