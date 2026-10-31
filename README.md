# forthling

[![CI](https://github.com/DeSchaker/forthling/actions/workflows/ci.yml/badge.svg)](https://github.com/DeSchaker/forthling/actions/workflows/ci.yml)

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

### Options

```
forthling [options] [file ...]
  -m, --memory SIZE  memory for data space and heap, such as 4M (default 1M)
  -h, --help         show this help
```

`SIZE` is a number of bytes with an optional `K`, `M` or `G` (binary units), from 64K up
to 4G. To change the default, put a line `memory = 16M` in the config file
`forthling/forthling.conf` in your config directory: `%APPDATA%` on Windows, otherwise
`$XDG_CONFIG_HOME` or `~/.config`. `--help` shows the exact path. Lines starting with `#`
are comments. The command line wins over the config file.

A program that embeds forthling picks the size with `Forth::with_memory(size)`.

## Features

- A REPL and file execution (nested with `INCLUDE`, `INCLUDED` and `REQUIRE`)
- File access: `OPEN-FILE CREATE-FILE READ-LINE READ-FILE WRITE-LINE WRITE-FILE`,
  `FILE-POSITION REPOSITION-FILE FILE-SIZE RESIZE-FILE`, `DELETE-FILE RENAME-FILE` and more
- Colon definitions compiled to threaded code, with `RECURSE`, `IMMEDIATE`,
  `POSTPONE`, `LITERAL`, `[ ]`, `' EXECUTE` and `SEE`
- `IF ELSE THEN`, `BEGIN UNTIL/AGAIN/WHILE REPEAT`, `DO ?DO LOOP +LOOP LEAVE`,
  `CASE OF ENDOF ENDCASE`
- `VARIABLE`, `CONSTANT`, `VALUE`, `DEFER`, `BUFFER:`, `MARKER`, `:NONAME`, `CREATE … DOES>`
- Byte-addressed memory with 64-bit cells, and a heap: `ALLOCATE FREE RESIZE`
- Double-cell numbers: `123.` is a double, with `D+ D- D. D.R D< DNEGATE DABS M+ M*/`,
  `2VALUE`, `2LITERAL` and more
- Structures (`BEGIN-STRUCTURE FIELD: CFIELD: +FIELD END-STRUCTURE`), `MS`, `TIME&DATE`,
  `KEY?`, and `AT-XY`/`PAGE` for the screen
- `SOURCE`, `>IN`, `STATE`, `WORD`, `PARSE`, `REFILL`, `SAVE-INPUT`, `FIND` and `EVALUATE`,
  working on input buffers in Forth memory
- Errors as standard THROW codes, e.g. `demo.fs:3: error -13: undefined word: FOO`,
  which `CATCH` can catch
- Strings: `COMPARE SEARCH /STRING -TRAILING BLANK CMOVE CMOVE> SLITERAL`, and text
  substitution with `REPLACES SUBSTITUTE UNESCAPE`
- Programming tools: `.S ? DUMP SEE WORDS`, `[IF] [ELSE] [THEN]`, `[DEFINED] [UNDEFINED]`,
  `AHEAD CS-PICK CS-ROLL`, `N>R NR>` and `SYNONYM`

Part of the dictionary is written in Forth itself (`src/prelude.fs`).

The interpreter is a library crate (`src/lib.rs`, with one module per part: stacks, memory,
parsing, the inner and outer interpreter, and the primitives in `src/words/`); `src/main.rs`
only holds the REPL. Another Rust program can embed it:

```rust
let mut forth = forthling::Forth::new();
forth.interpret(": SQUARE DUP * ; 7 SQUARE .")?;
assert_eq!(forth.output(), b"49 ");
```

## Standard compliance

[`STANDAARD.md`](STANDAARD.md) (in Dutch) holds the roadmap to Forth-2012 and the
current status. The CORE, CORE EXT, DOUBLE, EXCEPTION, FACILITY, FILE, MEMORY, STRING and
TOOLS word sets are complete, and their tests in the [Forth-2012 test suite](https://github.com/gerryjackson/forth2012-test-suite)
pass (`core.fr`, `coreplustest.fth`, `coreexttest.fth`, `doubletest.fth`,
`exceptiontest.fth`, `facilitytest.fth`, `filetest.fth`, `memorytest.fth`,
`stringtest.fth` and `toolstest.fth`). Of TOOLS EXT, everything is present except
`;CODE ASSEMBLER CODE EDITOR` (there is no assembler or editor), the obsolescent `FORGET`
(use `MARKER`), and `NAME>COMPILE NAME>INTERPRET NAME>STRING TRAVERSE-WORDLIST`, which will
come with the SEARCH word set. Next up is LOCALS.

### Implementation-defined choices

- Cells are 64 bits; an address unit and a character are 8 bits.
- Division is floored: `-7 2 /` gives `-4` and `-7 2 MOD` gives `1`. `/ MOD /MOD */ */MOD`
  are defined via `FM/MOD`; `SM/REM` is available for symmetric division.
- A number is a double cell only when it ends with a `.` (`123.`, `-5.`, `$FF.`); a `.`
  elsewhere does not make a number. `M*/` multiplies through a 192-bit intermediate, so
  only its quotient can be out of range (`-11`); its result is floored too.
- A double cell on the stack has its high cell on top. Division by zero throws `-10`,
  a quotient that does not fit throws `-11`.
- The pictured numeric output buffer (`<# … #>`) holds 256 characters; overflowing it
  throws `-17`. `.` and `U.` use the same buffer.
- `ACCEPT` and `KEY` read from standard input, which the terminal buffers per line: `KEY`
  returns once a line is entered, and the line ending is a character too. `ACCEPT` keeps
  at most the requested number of characters and drops the rest of the line. At the end
  of input `ACCEPT` returns 0 and `KEY` throws `-39`. `KEY?` never waits: it is true once
  a line has been entered and not yet read. `EKEY` returns characters only, like `KEY`;
  the special keys (`K-UP`, `K-F1`, …) are numbered from `$110000`, above Unicode, and
  never occur.
- `TIME&DATE` gives the time in UTC. `AT-XY` and `PAGE` write ANSI escape sequences, which
  current terminals, Windows Terminal included, understand.
- Memory is 1 MiB unless set otherwise (see Options). The input buffers, the `S"` and
  `WORD` buffers, the pictured numeric output buffer and `PAD` sit at fixed addresses at
  the bottom, followed by data space. The heap of `ALLOCATE` grows down from the top
  towards `HERE`, so data space and heap share the free space: `UNUSED` is what lies between them, and allocating lowers it.
  Allocated blocks are cell-aligned and a whole number of cells. A failing `ALLOCATE`
  returns `-59`, `FREE` of an address that `ALLOCATE` did not return gives `-60` (and
  changes nothing), and a failing `RESIZE` gives `-61` and leaves the block as it was.
- The data stack and the return stack hold 65536 cells each; overflowing them throws `-3`
  and `-5`. `ENVIRONMENT?` reports these sizes and the other standard queries.
- `ABORT` throws `-1` and `ABORT"` throws `-2`; both empty the stacks and return to the
  terminal with a message. `QUIT` keeps the data stack and prints nothing. When running
  files, `QUIT` ends the run, since there is no terminal to return to.
- `SOURCE-ID` is 0 for the terminal, -1 for `EVALUATE` and the file id for an included file.
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
- `COMPARE` and `SEARCH` compare characters exactly, so case matters. Substitution names
  (`REPLACES`, `SUBSTITUTE`) do not depend on case. A `%name%` that is not a substitution
  name is kept as it is, delimiters included. `SUBSTITUTE` returns `-78` when the result
  does not fit, and `REPLACES` throws `-79` for a name that contains `%`.
- File ids are small positive numbers. `R/O` is 0, `W/O` 1 and `R/W` 2; `BIN` adds 4 and
  changes nothing. A failing file word returns its own THROW code as ior (`-62` for
  `CLOSE-FILE` up to `-76` for `WRITE-LINE`). `WRITE-LINE` ends a line with a line feed;
  `READ-LINE` accepts both LF and CR LF. `FILE-STATUS` returns 0 as its status.
- `INCLUDED` and `REQUIRED` look for a relative file name that does not exist in the
  current directory next to the file that is including it. `REQUIRED` recognises a file by
  its full path. Inside a file, a `(` comment may continue over several lines.

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

forthling needs Rust 1.74 or newer. CI (`.github/workflows/ci.yml`) checks formatting and
clippy lints and runs all tests, the test suite included, on Rust 1.74 and on the latest
stable Rust on Linux and Windows.
