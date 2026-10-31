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
  -b, --blocks FILE  the block file (default blocks.fb in the current directory)
  -h, --help         show this help
```

`SIZE` is a number of bytes with an optional `K`, `M` or `G` (binary units), from 64K up
to 4G. A relative block file name is relative to the current directory.

To change the defaults, put lines like these in the config file
`forthling/forthling.conf` in your config directory: `%APPDATA%` on Windows, otherwise
`$XDG_CONFIG_HOME` or `~/.config` (`--help` shows the exact path):

```
# forthling settings
memory = 16M
blocks = C:/Users/me/forth/blocks.fb
```

Lines starting with `#` are comments. The command line wins over the config file.

A program that embeds forthling picks the size with `Forth::with_memory(size)` and the
block file with `Forth::set_block_file(path)`.

## Features

- A REPL and file execution (nested with `INCLUDE`, `INCLUDED` and `REQUIRE`)
- File access: `OPEN-FILE CREATE-FILE READ-LINE READ-FILE WRITE-LINE WRITE-FILE`,
  `FILE-POSITION REPOSITION-FILE FILE-SIZE RESIZE-FILE`, `DELETE-FILE RENAME-FILE` and more
- Colon definitions compiled to threaded code, with `RECURSE`, `IMMEDIATE`,
  `POSTPONE`, `LITERAL`, `[ ]`, `' EXECUTE` and `SEE`
- `IF ELSE THEN`, `BEGIN UNTIL/AGAIN/WHILE REPEAT`, `DO ?DO LOOP +LOOP LEAVE`,
  `CASE OF ENDOF ENDCASE`
- Locals: `{: a b | c -- out :}`, `LOCALS|` and `(LOCAL)`, with `TO` for assigning
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
- Blocks: `BLOCK BUFFER UPDATE FLUSH SAVE-BUFFERS EMPTY-BUFFERS LOAD THRU LIST`, kept in
  a block file
- Floating point: `1.5E3` is a float, on a stack of its own, with `F+ F* F/ F. FS. FE.`,
  `FSQRT FSIN FEXP FLN F** FATAN2` and more, `FVARIABLE FCONSTANT FVALUE`, `>FLOAT` and
  `REPRESENT`
- Extended characters in UTF-8: `XC@+ XC!+ XCHAR+ XEMIT XKEY X-SIZE X-WIDTH XHOLD` and
  more; `CHAR`, `[CHAR]`, `'€'` and `PARSE` take whole characters
- Word lists and the search order: `WORDLIST GET-ORDER SET-ORDER ALSO ONLY PREVIOUS
  DEFINITIONS ORDER SEARCH-WORDLIST`, and `TRAVERSE-WORDLIST NAME>STRING`
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

Such a program can also add words of its own, written in Rust. A word is a closure that
works on the stacks (`push`, `pop`, `fpush`, `fpop`, `pop_bytes`, `push_bytes`); it can
keep state in `Forth::state`. An `Extension` bundles words in a word list of its own and
makes its name known to `ENVIRONMENT?`, so standard programs never meet them by accident
(see [Extensions](COMPLIANCE.md#extensions)):

```rust
let mut forth = forthling::Forth::new();
let wid = forth.wordlist("MATHS"); // also defines MATHS, for ALSO MATHS
forth.add_word(wid, "CUBE", |f| {
    let n = f.pop()?;
    f.push(n * n * n);
    Ok(())
})?;
forth.interpret("ALSO MATHS 3 CUBE .")?;
assert_eq!(forth.output(), b"27 ");
```

## Standard compliance

[`ROADMAP.md`](ROADMAP.md) holds the roadmap to Forth-2012 and the
current status. The CORE, CORE EXT, BLOCK, DOUBLE, EXCEPTION, FACILITY, FILE, LOCALS,
MEMORY, SEARCH, STRING, TOOLS, XCHAR and FLOATING word sets are complete, and their tests in the
[Forth-2012 test suite](https://github.com/gerryjackson/forth2012-test-suite) pass (`core.fr`,
`coreplustest.fth`, `coreexttest.fth`, `blocktest.fth`, `doubletest.fth`, `exceptiontest.fth`,
`facilitytest.fth`, `filetest.fth`, `localstest.fth`, `memorytest.fth`,
`searchordertest.fth`, `stringtest.fth`, `toolstest.fth` and all floating-point tests in
`fp/`, the paranoia test included). Of TOOLS EXT, everything is
present except `;CODE ASSEMBLER CODE EDITOR` (there is no assembler or editor). The test
suite has no XCHAR tests, so forthling has its own, in the same style:
[`tests/xchartest.fth`](tests/xchartest.fth).

The system documentation that section 4.1 of the standard asks for (every
implementation-defined option, how forthling acts on each ambiguous condition, and the
other facts about the system) is in [`COMPLIANCE.md`](COMPLIANCE.md), with a Dutch
translation in [`VERKLARING.md`](VERKLARING.md).

### Implementation-defined choices

The main ones; [`COMPLIANCE.md`](COMPLIANCE.md) lists them all.

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
  a line has been entered and not yet read. `EKEY` returns characters only, like `XKEY`;
  the special keys (`K-UP`, `K-F1`, …) are numbered from `$110000`, above Unicode, and
  never occur.
- Floats are IEEE 754 doubles (8 bytes, like a DFLOAT; an SFLOAT is 4 bytes), on a
  separate stack of 1024 floats; over- and underflowing it throws `-44` and `-45`.
  Arithmetic follows IEEE: dividing by zero gives an infinity or a NaN rather than an
  exception. `F>D` and `F>S` throw `-11` when the result does not fit. The text
  interpreter knows float literals only while `BASE` is decimal; they need digits before
  the point and an `E` (`1E`, `-1.5e-3`). `PRECISION` starts at 15 and `SET-PRECISION`
  keeps it between 1 and 17. `F.`, `FE.` and `FS.` write `1000.`, `333.33E-3` and
  `3.3333E0`, with trailing zeros after the point dropped by `F.` only; infinities and
  NaNs come out as `inf`, `-inf` and `nan`. `CATCH` restores the depth of the float
  stack too.
- Extended characters (xchars) are UTF-8, up to `$10FFFF` in at most 4 bytes. A character
  stays one byte (`MAX-CHAR` is 255), so `KEY`, `EMIT` and `C@` work on bytes and
  `EKEY>CHAR` is true for ASCII only. Malformed UTF-8 is read a byte at a time: a byte that
  does not start a complete xchar is one on its own, with the byte as its value. Writing an
  xchar above `$10FFFF` writes U+FFFD. `XC-WIDTH` knows the common Unicode ranges (0 for
  control characters and combining marks, 2 for wide East Asian characters and emoji),
  not the full tables.
- `TIME&DATE` gives the time in UTC. `AT-XY` and `PAGE` write ANSI escape sequences, which
  current terminals, Windows Terminal included, understand.
- Memory is 1 MiB unless set otherwise (see Options). The input buffers, the `S"` and
  `WORD` buffers, the pictured numeric output buffer and `PAD` sit at fixed addresses at
  the bottom, followed by data space. The heap of `ALLOCATE` grows down from the top
  towards `HERE`, so data space and heap share the free space: `UNUSED` is what lies between them, and allocating lowers it.
  Allocated blocks are cell-aligned and a whole number of cells. A failing `ALLOCATE`
  returns `-59`, `FREE` of an address that `ALLOCATE` did not return gives `-60` (and
  changes nothing), and a failing `RESIZE` gives `-61` and leaves the block as it was.
- Blocks are kept in the file `blocks.fb` in the current directory, unless `--blocks` or
  the config file names another (see Options). The file is created the first time a
  block is read or written.
  Block numbers run from 1 to 1048576 (so the file stays below 1 GiB); any other number
  throws `-35`. Block u is at byte offset (u - 1) × 1024. A block beyond the end of the
  file reads as spaces, and writing one fills the gap with spaces. There are 8 block
  buffers in Forth memory; when all are in use, the least recently used one is written
  back (if updated) and reused. Read and write errors throw `-33` and `-34`.
- `LOAD` copies the block into an input buffer of its own, so `SOURCE` does not return
  the block buffer itself and changing the buffer during `LOAD` has no effect on the
  text. In a block, `\` skips to the end of the current line of 64 characters, `REFILL`
  moves on to the next block, and `SAVE-INPUT`/`RESTORE-INPUT` work across blocks.
  `SOURCE-ID` is 0 while a block is loaded. `LIST` prints `Screen u` and then the 16
  lines of 64 characters, numbered 0 to 15, without trailing spaces.
- A word list id (wid) is a small number; `FORTH-WORDLIST` is 1 and `WORDLIST` hands out
  2, 3, …, without limit. The search order holds up to 16 word lists (`WORDLISTS`); more
  throws `-49`, and `PREVIOUS` or `ALSO` on an empty order throws `-50`. An invalid wid
  throws `-24`. `ONLY` and `-1 SET-ORDER` leave just `FORTH-WORDLIST`, which holds all
  words of the system, `SET-ORDER` included. `WORDS` lists the first word list of the
  search order, and `ORDER` shows unnamed word lists as `#2`, `#3`, …. `MARKER` also
  restores the search order, the compilation word list and the word lists made since.
- `FORGET name` looks for the name in the compilation word list only, deletes that word
  and every word defined after it, and puts `HERE` back to where it was before the word.
  It does not undo changes to the search order or word lists; `MARKER` does. Words of the
  system itself cannot be forgotten (`-15`).
- A name token is the same number as the execution token. `NAME>STRING` copies the name
  to the heap the first time, in upper case, since names are not case-sensitive.
  `NAME>INTERPRET` never returns 0: every word can be executed.
- Locals live on a stack of their own, not on the return stack, so `>R` and `R>` can be
  mixed freely with them. A definition can have up to 256 locals (`#LOCALS`); more
  throws `-21`. It may declare them in more than one group. The locals' names are found
  before all other words, but only while compiling, and they are forgotten at `;` and at
  `DOES>`; the `DOES>` part can declare locals of its own. A `{:` declaration may continue
  over several lines. With `{:` the last argument gets the top of the stack, with
  `LOCALS|` and `(LOCAL)` the first one. A `|` or `--` out of place, or a missing `:}`,
  throws `-22`.
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

## Contributing

Bug reports, ideas and pull requests are welcome; see [CONTRIBUTING.md](CONTRIBUTING.md)
and the [code of conduct](CODE_OF_CONDUCT.md). Security problems go through
[SECURITY.md](SECURITY.md).

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
