# forthling

[![CI](https://github.com/DeSchaker/forthling/actions/workflows/ci.yml/badge.svg)](https://github.com/DeSchaker/forthling/actions/workflows/ci.yml)

> **forthling** *(noun)*
>
> 1. A diminutive creature thriving on a two-stack diet.
> 2. A lean, extensible Forth system built from the ground up.

Forth is a tiny language with a big idea: you build your program out of small words, and
you can try each one the moment you write it. forthling brings that to today's world: a
complete [Forth-2012](https://forth-standard.org/) system in Rust that you can run on the
command line, build into your own program, hand to an AI agent, or
**[try right now in your browser](https://deschaker.github.io/forthling/)**, with nothing
to install.

![The tutorial in the playground: lesson 4 asks for a word CUBE; a wrong answer gets
"Not yet" with what it left, the right one "Well done!"](docs/tutorial.gif)

- **The whole standard.** All word sets of Forth-2012, from CORE to FLOATING, pass the
  Forth-2012 test suite, so standard code from books and tutorials runs as written. See
  [Standard compliance](#standard-compliance).
- **Safe to play with.** A sandbox puts limits on steps, time and output, and keeps the
  disk out of reach: files live in memory there, so a program can still write, read and
  `INCLUDE` them. A runaway loop stops with an error instead of hanging, and gives the same
  result on every machine. See [Sandbox](#sandbox).
- **Small and clean.** The core has no dependencies and no `unsafe`, and its memory is
  bounded.
- **Talks to the world.** JSON words in the core; SQLite as an extension.
- **Made for AI agents.** An [MCP server](#for-ai-agents-mcp) gives an agent a Forth
  session that remembers its definitions and shows the stacks after every call. When
  something goes wrong, it also says where, which words were running and what the stacks
  held, so an agent can often fix its code in one round. [`llms.txt`](llms.txt) tells a
  language model how to use it.
- **Learn as you go.** `ALSO TUTORIAL` gives 20 short lessons, ten for beginners and ten
  beyond, and `CHECK` tests your answer. See [Tutorial](#tutorial).
- **Runs anywhere.** As a program, embedded in Rust or [Python](#from-python), or
  [in the browser](https://deschaker.github.io/forthling/) through WebAssembly.

The extensions (Python, SQLite, MCP, WebAssembly) live in crates of their own. MIT
licence.

Install the program from [crates.io](https://crates.io/crates/forthling) with Rust 1.85 or
newer, and run `forthling`:

```
cargo install forthling
```

From a clone of the repository, `cargo run --release` does the same:

```
$ cargo run --release
forthling — WORDS shows the dictionary, BYE quits.
: SQUARE  DUP * ;
 ok
7 SQUARE .
49  ok
```

Run a file: `cargo run --release -- examples/demo.fs`

### What to install

forthling comes in parts, called crates in Rust: the core (`forthling`) and extensions
such as `forthling-sqlite` and `forthling-mcp`. Which ones you need depends on how you use
it.

- **To use forthling,** install one program; its extensions are built in.
  - In the browser: nothing to install, open the
    [playground](https://deschaker.github.io/forthling/).
  - `cargo install forthling`: the program, the core alone, without SQLite.
  - `cargo install forthling-sqlite`: the same program with the SQLite words.
  - `cargo install forthling-mcp`, or the bundle `forthling.mcpb` (see
    [For AI agents](#for-ai-agents-mcp)): the MCP server, with the core and SQLite.
- **To embed forthling in a Rust program,** name the core and the extensions you want in
  your `Cargo.toml`. An extension brings the core along, and Cargo builds it once, shared
  by all extensions, so they can work in the same system. An extension you do not name,
  such as SQLite with its C library, stays out of your program.

  ```toml
  [dependencies]
  forthling = "0.2"
  forthling-sqlite = "0.2"
  ```

  ```rust
  let mut forth = forthling::Forth::new();
  forth.install(&forthling_sqlite::Sqlite);
  ```

### Options

```
forthling [options] [file ...]
  -m, --memory SIZE  memory for data space and heap, such as 4M (default 1M)
  -b, --blocks FILE  the block file (default blocks.fb in the current directory)
      --sandbox      files only in memory (the files given here and the block
                     file are copied in; what Forth writes is gone at the end),
                     and the limits below: 10000000 steps, 5s and 1M of output
                     per line of the REPL or per file, and 4M of space for
                     definitions and the like in all
      --max-steps N  at most N steps (instructions and words) per evaluation
      --max-time T   at most T per evaluation, such as 5s or 500ms
      --max-output SIZE  at most SIZE bytes of output per evaluation
      --max-space SIZE   at most SIZE bytes outside Forth memory for definitions,
                     code, word lists and the like
  -h, --help         show this help
```

A limit of `none` lifts it, also one of `--sandbox`. See [Sandbox](#sandbox).

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
- Quotations (Forth 2019): `[: … ;]` makes a nameless word and leaves its xt, inside a
  definition or at the prompt: `3 [: 2 * ;] EXECUTE .` prints `6`
- Byte-addressed memory with 64-bit cells, and a heap: `ALLOCATE FREE RESIZE`
- Double-cell numbers: `123.` is a double, with `D+ D- D. D.R D< DNEGATE DABS M+ M*/`,
  `2VALUE`, `2LITERAL` and more
- Structures (`BEGIN-STRUCTURE FIELD: CFIELD: +FIELD END-STRUCTURE`), `MS`, `TIME&DATE`,
  `KEY?`, and `AT-XY`/`PAGE` for the screen
- `SOURCE`, `>IN`, `STATE`, `WORD`, `PARSE`, `REFILL`, `SAVE-INPUT`, `FIND` and `EVALUATE`,
  working on input buffers in Forth memory
- JSON: read it in place (`JSON@` with a path such as `users[0].name`) and write it to the
  output or to a string (see [JSON](#json))
- A sandbox for code you do not trust: limits on steps, time and output, and no file
  access (see [Sandbox](#sandbox))
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
- Words inspired by other Forths: the loops `+DO -DO U+DO U-DO FOR NEXT -LOOP ?LEAVE ?EXIT`, strings (`BOUNDS PLACE STR=
  SCAN SKIP S+`), stack and comparison words (`2NIP THIRD <= U<= UMAX`), `ROL ROR CTZ LOG2`,
  `[IFDEF] [IFUNDEF]`, `ALIAS`, `VOCABULARY`, `RANDOM`, `UTIME`, `SLURP-FILE` and more (see
  [Words inspired by other Forths](COMPLIANCE.md#words-inspired-by-other-forths))

Part of the dictionary is written in Forth itself (`src/prelude.fs`).

The interpreter is a library crate (`src/lib.rs`, with one module per part: stacks, memory,
parsing, the inner and outer interpreter, and the primitives in `src/words/`); `src/main.rs`
only holds the REPL. Another Rust program can embed it (`cargo add forthling`):

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

The macro `forth_word!` writes the `pop` and `push` for you: the word is a Rust function
with typed arguments, taken in stack order (the last is the top), and a typed result:

```rust
use forthling::{forth_word, Forth, Res, Throw};

let mut forth = Forth::new();
let wid = forth.wordlist("MATHS");
forth_word!(forth, wid, "CUBE", (n: i64) -> i64 { n * n * n })?;
forth_word!(forth, wid, "HYPOT", (a: f64, b: f64) -> f64 { a.hypot(b) })?;
forth_word!(forth, wid, "SHOUT", (s: String) -> String { s.to_uppercase() })?;
forth_word!(forth, wid, "DIVMOD", (a: i64, b: i64) -> Res<(i64, i64)> {
    if b == 0 {
        return Err(Throw::new(-10));
    }
    Ok((a % b, a / b))
})?;
forth.interpret("ALSO MATHS 3 CUBE . 3E0 4E0 HYPOT F. S\" hi\" SHOUT TYPE 7 2 DIVMOD . .")?;
assert_eq!(forth.output(), b"27 5. HI3 1 ");
```

`i64` and `bool` (a flag) come from the data stack, `f64` from the float stack, `String`
and `Vec<u8>` from a string ( c-addr u ). A result of those types (or `&str`) goes back
the same way, a string in `ALLOCATE`d memory that the program frees; a tuple gives
several, `Res<T>` lets the word throw, and without `-> T` the word gives nothing back.
A word that gets too few arguments takes none and throws -4 (or -45 for floats). For
types of your own, implement `FromForth` and `IntoForth`.

### JSON

The word list `JSON` (`ALSO JSON`) is built in; the forthling program and the Python
module install it, a Rust program with `forth.install(&forthling::Json)`. A JSON value is
its text, a string `( c-addr u )`: the reading words give back pieces of it, so nothing is
copied. A missing key or a value of the wrong kind gives the flag 0.

```forth
ALSO JSON
: DOC  S\" {\"users\": [{\"name\": \"Ada\", \"born\": 1815}]}" ;
DOC S" users[0].name" JSON@ DROP JSON>STRING DROP TYPE   \ Ada (ALLOCATEd: FREE it)
DOC S" users[0].born" JSON@ DROP JSON>NUMBER DROP .      \ 1815
JSON{ S" sum" JSON-KEY 6 JSON-NUMBER S" ok" JSON-KEY -1 JSON-FLAG JSON}
\ {"sum":6,"ok":true}
```

| Reading | Stack |
|---|---|
| `JSON-VALID?` | `( json -- flag )` |
| `JSON-TYPE` | `( json -- n )`: 0 null, 1 boolean, 2 number, 3 string, 4 array, 5 object, -1 no JSON |
| `JSON-GET` | `( json c-addr u -- json' flag )`: the value of a key |
| `JSON-NTH` | `( json n -- json' flag )`: element n (from 0) of an array or object |
| `JSON-NAME` | `( json n -- c-addr u flag )`: key n of an object, `ALLOCATE`d |
| `JSON-LENGTH` | `( json -- n )`: elements of an array or object, else -1 |
| `JSON@` | `( json c-addr u -- json' flag )`: a path, such as `a.b[2]` or `["a.b"]` |
| `JSON>NUMBER` | `( json -- n flag )`: an integer that fits in a cell |
| `JSON>FLOAT` | `( json -- flag ) ( F: -- r )` |
| `JSON>FLAG` | `( json -- flag ok )` |
| `JSON>STRING` | `( json -- c-addr u flag )`: without its escapes, `ALLOCATE`d |

Writing goes to the output, or between `<JSON` and `JSON>` (which gives `( c-addr u )`,
`ALLOCATE`d) to a string; the commas come by themselves. `JSON{ JSON} JSON[ JSON]`,
`JSON-KEY ( c-addr u -- )`, `JSON-STRING ( c-addr u -- )`, `JSON-NUMBER ( n -- )`,
`JSON-FLOAT ( F: r -- )`, `JSON-FLAG ( flag -- )`, `JSON-NULL` and `JSON-RAW ( json -- )`.
A structure that does not fit, such as a value in an object without its key, throws -22.

### Functional words

The word list `FUNCTIONAL` (`ALSO FUNCTIONAL`) has `MAP`, `FOLD`, `RUNNING-FOLD`, `FILTER` and `RANGE`
on the data stack. The loop runs in Rust and calls the execution token once per item; the
stack effect of the token is checked after each call, and the step and time limits apply
inside it. A primitive token (`' +`, `' INVERT`, or a quotation of one primitive) is called
directly, without the inner interpreter. The forthling program, the MCP server, the Python
module and the playground install it; a Rust program does with
`forth.install(&forthling::Functional)`.

```forth
ALSO FUNCTIONAL
1 6 RANGE [: DUP * ;] MAP 5 ' + FOLD .        \ 55
1 10 RANGE [: 2 MOD 0= ;] FILTER .S           \ <5> 2 4 6 8 4
```

| Word | Stack |
|---|---|
| `MAP` | `( x1 .. xn n xt -- y1 .. yn )`, `xt` is `( x -- y )` |
| `FOLD` | `( x1 .. xn n xt -- r )`, `xt` is `( acc x -- acc' )`; `x1` is the first `acc`, so `n` is at least 1 |
| `RUNNING-FOLD` | `( x1 .. xn n xt -- r1 .. rn )`: like `FOLD`, with every partial result |
| `FILTER` | `( x1 .. xn n xt -- y1 .. ym m )`, `xt` is `( x -- flag )` |
| `RANGE` | `( a b -- a .. b-1 n )` |
| `ANY?` `ALL?` | `( x1 .. xn n xt -- flag )`, `xt` is `( x -- flag )`; they stop at the answer; `ALL?` is true for `n` = 0 |
| `FIND` | `( x1 .. xn n xt -- x true \| false )`, the first item that passes |
| `COUNT-IF` | `( x1 .. xn n xt -- m )` |
| `FLAT-MAP` | `( x1 .. xn n xt -- y1 .. ym m )`, `xt` is `( x -- y1 .. yk k )`; the results are joined |
| `ITERATE` | `( x n xt -- x1 .. xn )`: `x1` is `x`, every next item is `xt` of the one before |
| `REVERSE` | `( x1 .. xn n -- xn .. x1 )` |
| `TAKE` `DROP-N` | `( x1 .. xm m n -- .. k )`: the first `n` items, or without them; `k` is the new count |
| `ZIP` | `( a1 .. an b1 .. bn n -- a1 b1 .. an bn 2n )` |
| `SUM` `PRODUCT` | `( x1 .. xn n -- r )`, wrapping; 0 and 1 for `n` = 0 |
| `MIN-OF` `MAX-OF` | `( x1 .. xn n -- r )`; `n` is at least 1 |

A word that changes the length of the list leaves the new count on top (`FILTER`, `FLAT-MAP`,
`TAKE`, `DROP-N`, `ZIP`); `MAP`, `ITERATE` and `REVERSE` do not.

```forth
5 8 12 3 [: 6 > ;] FIND .S                    \ <2> 8 -1
1 2 3 10 20 30 3 ZIP .S                       \ <7> 1 10 2 20 3 30 6
```

The same on a block of cells in memory, for large amounts of data, with the suffix `-CELLS`. The
results go back into the block; the address range is checked as for other memory words.

```forth
CREATE A 3 , 1 , 4 , 1 , 5 ,
A 5 ' MAX FOLD-CELLS .                        5
A 5 [: DUP * ;] MAP-CELLS  A 5 ' + FOLD-CELLS .    52
```

| Word | Stack |
|---|---|
| `MAP-CELLS` | `( addr n xt -- )`, in place; `xt` is `( x -- y )` |
| `FOLD-CELLS` | `( addr n xt -- r )`; the first cell is the first `acc`, so `n` is at least 1 |
| `RUNNING-FOLD-CELLS` | `( addr n xt -- )`, in place: cell i becomes the partial result over cells 0 to i |
| `FILTER-CELLS` | `( addr n xt -- m )`, the cells that pass are moved to the front; `m` is how many |
| `ANY?-CELLS` `ALL?-CELLS` | `( addr n xt -- flag )`; the block stays as it is |
| `FIND-CELLS` | `( addr n xt -- x true \| false )` |
| `COUNT-IF-CELLS` | `( addr n xt -- m )`; unlike `FILTER-CELLS` it does not change the block |
| `REVERSE-CELLS` | `( addr n -- )`, in place |
| `SUM-CELLS` `PRODUCT-CELLS` | `( addr n -- r )`, wrapping; 0 and 1 for `n` = 0 |

`TAKE`, `DROP-N`, `MIN-OF` and `MAX-OF` have no `-CELLS` form: on a block, `TAKE` and `DROP-N`
are a change of `addr n`, and `' MIN FOLD-CELLS` does the other two.

Measured on 60000 numbers, 100 times, release build: summing with `RANGE ' + FOLD` takes 93 ms,
against 471 ms for a plain `DO` loop; with a colon word instead of `+` it takes 250 ms. A
quotation that is not one primitive (`[: DUP * ;] MAP`) goes through the inner interpreter
and is not faster than a loop (460 ms against 361 ms for a loop with the same work, `MAP`
and a `FOLD` to add up the result included).

### Sandbox

Code that is not trusted, from a user or an AI agent, runs in a sandbox:

```rust
let mut forth = forthling::Forth::sandboxed();
let error = forth.interpret(": FOREVER BEGIN AGAIN ; FOREVER").unwrap_err();
assert_eq!(error.to_string(), "error -258: limit exceeded: steps");
```

`Forth::sandboxed()` has files only in memory (see [Files](#files)), no terminal input, and limits each evaluation (each call of
`interpret` or `include`) to 10 million steps, 5 seconds and 1 MiB of output. A step is an
instruction or a word, and words that handle much data at once (`MOVE`, `SEARCH`, the JSON
words and the like) count one step per 64 bytes, so the time limit holds for them too.
Forth memory has its fixed size; what a program makes outside it (definitions and their
code, word lists, `REPLACES` texts, JSON strings) is limited to 4 MiB in all, its
`space`. `set_limits(Limits { … })` and `permit(Permission::Files, …)` set them one by
one. A limit that is reached throws -258, and every step after it throws again, so
`CATCH` cannot go on. `permit(Permission::Files, false)` takes files away altogether (the
FILE words give the ior -259, `INCLUDE` and the block words throw -259). The sandbox restricts the Forth program, not the host: `include`
called by the host still reads its file. Extensions follow the permissions too; SQLite
opens only `:memory:` unless the program may use the disk (`Forth::disk_files()`). `Forth::sandboxed()` also denies `Permission::Network`, which the HTTP words need (see [HTTP](#http)); `--sandbox` of the program does the same.

### Files

The FILE words, `INCLUDE` and the like, the block file and the host's `include` go
through a file system that the host chooses with `set_file_system`: `NativeFs`, the disk
(the default), `MemoryFs`, files in memory, or a `FileSystem` of its own, such as files
in a database or the browser's storage. A `MemoryFs` and its clones share their files, so
the host can put files in and read back what the program wrote:

```rust
use forthling::{Forth, MemoryFs};

let files = MemoryFs::new();
files.insert("square.fs", ": SQUARE DUP * ;").unwrap();
let mut forth = Forth::sandboxed();
forth.set_file_system(Box::new(files.clone())).unwrap();
forth.interpret("INCLUDE square.fs  S\" out.txt\" W/O CREATE-FILE THROW VALUE OUT").unwrap();
forth.interpret("7 SQUARE 0 <# #S #> OUT WRITE-FILE THROW  OUT CLOSE-FILE THROW").unwrap();
assert_eq!(files.get("out.txt").unwrap(), b"49");
```

In a `MemoryFs` a name is a path in which `/` and `\` both separate the parts, so
`lib/../square.fs` is `square.fs`; `MemoryFs::with_limit(bytes)` caps the files in all,
and a write past it fails with the word's ior. `Forth::sandboxed()` starts with an empty
one of 4 MiB (`MemoryFs::SANDBOX_LIMIT`), and so does each session of the MCP server and
the playground, which keeps its files in the browser. With `--sandbox` the program puts
copies of the files on its command line (and of the `--blocks` file) in one, and Python's
`sandbox=True` has one too (`write_file`, `read_file`, `delete_file`, `files`); what the
Forth code writes there never reaches the disk.

### Error reports

An error empties the stacks, but the `Throw` that `interpret` or `include` returns keeps
what the system looked like at that moment, so a person or an agent sees what went wrong
without running the code again:

```rust
let mut forth = forthling::Forth::new();
forth.interpret(": STEP 0 / ;\n: MAIN 1 2 STEP ;").unwrap();
let error = forth.interpret("5 MAIN").unwrap_err();
let c = error.context().unwrap();
assert_eq!(c.words, ["MAIN", "STEP", "/"]);   // the words that were running
assert_eq!(c.stack, [5, 1]);                  // `/` has taken its two arguments
assert_eq!((c.line, c.column, c.token.as_str()), (1, 3, "MAIN"));
```

`ErrorContext` has the data and float stacks at the error, the words that were running
(from the outermost; the chain ends at a word of the system, such as `/`, which is
written in Forth itself), and where in the input: `source` (`input`, `evaluate`, the path
of a file or `block n`), `line`, `column` (in characters) and `token`, the last word read.
A word that takes its arguments before it fails has taken them; a stack underflow takes
nothing. `steps_used()` and `max_depth()` tell how many steps the last evaluation took and
how deep the data stack went, the words of the system it ran (such as `.`, written in
Forth) included.

An undefined word that is in a word list outside the search order gets a hint, which
`Throw::hint()` gives and the message ends with:
`error -13: undefined word: JSON@ (it is in the word list JSON; ALSO JSON finds it)`. The
MCP server, the playground and the Python module pass all of this on.

### Tracing

To see what code does word by word, switch on a trace. `TRACE-ON` and `TRACE-OFF` are in
the word list `DEBUG` (`ALSO DEBUG`), installed by the forthling program, the MCP server
and the playground. Each word that runs gets a line: indented by how deep it is nested,
then its name and the data stack when it starts, as `.S` shows it (at most the top 8
cells):

```
ALSO DEBUG
: SQ  DUP * ;
: F  3 SQ 1+ ;
TRACE-ON F . TRACE-OFF
```

```
F <0>
  SQ <1> 3
    DUP <1> 3
    * <2> 3 3
  1+ <1> 9
. <1> 10
10
```

The insides of the system's own words (such as `/` and `.`, written in Forth) are not
traced, nor are the words that run while a definition is compiled, nor the system's words
that read the input, such as `\`, `(` and `[IF]`. The lines are output,
so the output limit of the sandbox holds for them. A host records the same as data:
`record_trace(Some(max))` and then `take_trace()`, a `TraceEntry` per word (`word`,
`depth`, `stack`, `stack_depth`); `forth_eval` of the MCP server does this with
`trace: true`. When no trace is on, it costs one test per word.

### Debugging

`BREAK`, also in the word list `DEBUG`, stops the program where it stands. The forthling
program then shows the words that are running and the stack, and asks how to go on: `s`
(or an empty line) steps to the next word, `n` steps over the words this one calls, `f`
runs on until the word this one is in has finished, `c` continues to the next `BREAK`, and
`q` quits with -28 (user interrupt). A file `debug.fs`:

```
ALSO DEBUG
: SQ  DUP * ;
: F  3 SQ 1+ ;
: MAIN  BREAK F 2 SQ + ;
MAIN .
```

and `forthling debug.fs` in a terminal, with the answers typed after the prompts:

```
break: MAIN > BREAK  <0>
s step, n over, f out, c continue, q quit> s
step: MAIN > F  <0>
s step, n over, f out, c continue, q quit> s
step: MAIN > F > SQ  <1> 3
s step, n over, f out, c continue, q quit> f
step: MAIN > SQ  <2> 10 2
s step, n over, f out, c continue, q quit> c
14
```

A host decides what a pause does with `set_debugger`: a function that gets the system
and a `Pause` (why it stopped, the word, its depth, and the stacks, the running words and
the place in the input, as for an error) and returns a `Resume` (`Step`, `Over`, `Out`,
`Continue`) or an error to throw. The time spent in it does not count towards the time
limit. Without a debugger, `BREAK` does nothing.

The MCP server and the playground cannot wait in the middle of a run, as the program
waits for the terminal: `CATCH`, `EVALUATE` and `INCLUDE` run Forth inside Rust calls, so
a run can only stop while its host function runs. So they do it differently. In
`forth_eval`, `BREAK` does not stop: the result shows the stacks and the running words at
each `BREAK`. In the playground, **Step through** runs the code while it records each
word, then plays the run back: forwards and back, step, over, out and on to the next
`BREAK`, with the stack before and after each word. Since the sandbox makes a run come out
the same each time, the record is what happened.

### Tutorial

New to Forth? The tutorial teaches it inside forthling, in 20 short lessons: ten for
beginners (the stack, arithmetic, defining words, decisions, loops, variables, strings)
and ten beyond (recursion, locals, tables, execution tokens, exceptions, floating point,
formatting numbers, JSON and finding a bug with a trace). Each lesson explains a little,
gives code to try and a task; `CHECK` tests your answer and says what is still wrong.

```
ALSO TUTORIAL
LESSONS          \ the list, with the lessons you have done
4 LESSON         \ lesson 4: Defining words
: CUBE DUP DUP * * ;
CHECK            \ Well done! Lesson 4 is done. NEXT goes on to: Printing.
HINT  NEXT       \ a hint, and the next lesson
```

The program, the MCP server and the playground have it; in the playground the
**Tutorial** button opens the lessons beside the editor and keeps your progress in the
browser. `Forth::new()` leaves it out, so a standard program sees no extra words; a Rust
host installs it with `forth.install(&forthling::Tutorial)`, and `Forth::tutorial(word)`
runs its words also without `ALSO TUTORIAL`, for buttons.

### In the browser

`web/` compiles forthling to WebAssembly (about 490 KB, no dependencies, no wasm-bindgen)
and has a playground page: Forth in the browser, on the visitor's own computer, in the
sandbox. Try it at **https://deschaker.github.io/forthling/**; see
[web/README.md](web/README.md).

### For AI agents (MCP)

`mcp/` holds `forthling-mcp`, an [MCP](https://modelcontextprotocol.io/) server: AI agents
run Forth through it, in a session that keeps their definitions and files, always in the
sandbox, with files only in memory. Install it with `cargo install forthling-mcp`, or
without Rust as the bundle `forthling.mcpb` from the
[latest release](https://github.com/DeSchaker/forthling/releases/latest), which Claude
Desktop opens; with Claude Code: `claude mcp add forthling -- forthling-mcp`. It is in the
[MCP Registry](https://registry.modelcontextprotocol.io) as `io.github.DeSchaker/forthling`. Its
tools are `forth_eval`, `forth_stack`, `forth_words` and `forth_reset`; each result has
the output and the stacks as text and as JSON. See [mcp/README.md](mcp/README.md), also
for what enforces the sandbox when a user asks the AI to get around it.
[`llms.txt`](llms.txt) (also at https://deschaker.github.io/forthling/llms.txt) sums up
for a language model how to write Forth for forthling: the idioms, locals rather than
`ROT` and `PICK`, the JSON words and the limits.

### From Python

`python/` holds a Python module, built with [PyO3](https://pyo3.rs/) and
[maturin](https://www.maturin.rs/); see [python/README.md](python/README.md).

```python
import math, forthling

f = forthling.Forth()
f.add_word("HYPOT", math.hypot, inputs=2)   # a word written in Python
print(f.interpret("3 4 HYPOT F."))         # 5.
```

### SQLite

`sqlite/` adds the word list `SQLITE`, in the style of the FILE words: handles as cells,
errors as an `ior`. Its program `forthling-sqlite` is forthling with these words; see
[sqlite/README.md](sqlite/README.md) and [the example](sqlite/examples/people.fs).

```
$ cargo run --release --manifest-path sqlite/Cargo.toml
forthling — WORDS shows the dictionary, BYE quits.
Extensions: FORTHLING-SQLITE.
ALSO SQLITE  S" :memory:" DB-OPEN THROW VALUE DB
 ok
S" SELECT sqlite_version()" DB DB-PREPARE THROW VALUE V
 ok
V DB-STEP THROW DROP  0 V DB-COLUMN TYPE
3.53.2 ok
```

### Regular expressions

`regex/` adds the word list `REGEX`: `RE-MATCH?`, `RE-FIND`, `RE-GROUP`, `RE-COUNT` and
`RE-REPLACE`. Matching takes time linear in the text, so no pattern can hang a program, and
the words reach nothing outside Forth, so they need no permission: they are in the MCP
server, the Python module and the playground, and `forthling-regex` is the program with
them. See [regex/README.md](regex/README.md) and [the example](regex/examples/dates.fs).

```
ALSO REGEX
: SWAP-DATE  S" 2026-11-01" S" (\d+)-(\d+)-(\d+)" S" $3/$2/$1" RE-REPLACE ;
SWAP-DATE TYPE
```

### HTTP

`http/` adds the word list `HTTP`: `HTTP-GET`, `HTTP-POST`, `HTTP-HEADER` and
`HTTP-RESPONSE-HEADER`, through ureq with rustls. A program fetches a page or posts to a
JSON API, and reads the answer with the JSON words. It reaches other machines, so it is
closed by default: it needs `Permission::Network` (which `Forth::sandboxed()` and
`--sandbox` deny) and a list of the hosts it may reach, which the host program gives
(`Http::new().allow("api.example.com")`; `--allow-host` for the program
`forthling-http` and the MCP server, the `allow_hosts` argument of the Python module).
With no list, no request goes out. The time left in the budget bounds a request, a
response is at most 1 MiB, and redirects only go to allowed hosts. The playground has none.
See [http/README.md](http/README.md) and [the examples](http/examples).

```
ALSO HTTP
: SHOW-PAGE ( -- )
  S" https://example.com/" HTTP-GET ( body u status )
  . . ." bytes, "
  S" content-type" HTTP-RESPONSE-HEADER IF 2DUP TYPE DROP FREE THROW THEN
  CR  FREE THROW ;
SHOW-PAGE
```

`forthling-http --allow-host example.com` printed `200 577 bytes, text/html; charset=utf-8`.

## Standard compliance

The open work is in the
[issues on GitHub](https://github.com/DeSchaker/forthling/issues). The CORE, CORE EXT, BLOCK, DOUBLE, EXCEPTION, FACILITY, FILE, LOCALS,
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

forthling needs Rust 1.85 or newer. CI (`.github/workflows/ci.yml`) checks formatting and
clippy lints and runs all tests, the test suite included, on Rust 1.85 and on the latest
stable Rust on Linux and Windows. It also builds the Python module and runs its tests
(`python -m unittest discover -s python/tests`, after `pip install ./python`) and checks
and tests the SQLite words (`cargo test --manifest-path sqlite/Cargo.toml`), the regular
expression words (`cargo test --manifest-path regex/Cargo.toml`), the HTTP words
(`cargo test --manifest-path http/Cargo.toml`, against a small server on the machine) and the MCP
server (`cargo test --manifest-path mcp/Cargo.toml`, also with `--no-default-features` and `--all-features`),
and builds the WebAssembly module and tests it with Node (`node --test
web/tests/forthling.test.mjs`).
