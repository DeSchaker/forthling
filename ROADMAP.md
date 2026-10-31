# Roadmap: towards Forth-2012

Goal: make this interpreter comply with the [Forth-2012 standard](https://forth-standard.org/standard/words).
First the required CORE word set, then CORE EXT, then the optional word sets.

## Where we are

*Updated 2026-10-31: CORE, CORE EXT, EXCEPTION, TOOLS, STRING, FILE, DOUBLE, FACILITY, MEMORY, LOCALS, SEARCH, BLOCK, XCHAR and FLOATING are complete; milestone M5 is reached.*

| Word set      | Present   | Missing   |
|---------------|----------:|----------:|
| CORE          | 133 / 133 | 0         |
| CORE EXT      | 49 / 49   | 0         |
| EXCEPTION     | 2 / 2     | 0         |
| EXCEPTION EXT | 2 / 2     | 0         |
| TOOLS         | 5 / 5     | 0         |
| TOOLS EXT     | 18 / 22   | 4 (see below) |
| STRING        | 8 / 8     | 0         |
| STRING EXT    | 3 / 3     | 0         |
| FILE          | 21 / 21   | 0         |
| FILE EXT      | 8 / 8     | 0         |
| DOUBLE        | 20 / 20   | 0         |
| DOUBLE EXT    | 3 / 3     | 0         |
| FACILITY      | 3 / 3     | 0         |
| FACILITY EXT  | 37 / 37   | 0         |
| MEMORY        | 3 / 3     | 0         |
| LOCALS        | 1 / 1     | 0         |
| LOCALS EXT    | 2 / 2     | 0         |
| SEARCH        | 9 / 9     | 0         |
| SEARCH EXT    | 5 / 5     | 0         |
| BLOCK         | 8 / 8     | 0         |
| BLOCK EXT     | 6 / 6     | 0         |
| XCHAR         | 10 / 10   | 0         |
| XCHAR EXT     | 11 / 11   | 0         |
| FLOATING      | 31 / 31   | 0         |
| FLOATING EXT  | 50 / 50   | 0         |

Missing from TOOLS EXT: `;CODE ASSEMBLER CODE EDITOR` (there is no assembler and no
editor).

Test suite: in the order of `runtests.fth`, `prelimtest.fth`, `tester.fr`, `core.fr`,
`coreplustest.fth`, `utilities.fth`, `errorreport.fth`, `coreexttest.fth`, `blocktest.fth`,
`doubletest.fth`, `exceptiontest.fth`, `facilitytest.fth`, `filetest.fth`, `localstest.fth`,
`memorytest.fth`, `searchordertest.fth`, `stringtest.fth` and `toolstest.fth` run **without
errors**, and so do the nine files in `fp/` in the order of `runfptests.fth`
(`ttester.fs`, `fatan2-test.fs`, `ieee-arith-test.fs`, `ieee-fprox-test.fs`,
`fpzero-test.4th`, `fpio-test.4th`, `to-float-test.4th`, `paranoia.4th`, `ak-fp-test.fth`;
paranoia reports no failures, defects or flaws). `cargo test` runs them all from
`tests/suite.rs` (`suite_prelimtest`, `suite_core`, `suite_core_ext`, `suite_block`,
`suite_double`, `suite_exception`, `suite_facility`, `suite_file_access`, `suite_locals`,
`suite_memory`, `suite_search`, `suite_string`, `suite_tools`, `suite_xchar`,
`suite_floating`). A new word set test is one line with the helper `run_suite`, which also
takes several files (`filetest.fth` needs `coreexttest.fth`). Since SEARCH, the SEARCH parts
of `toolstest.fth` and `localstest.fth` run too; `suite_tools` and `suite_locals` check that.
The suite has no XCHAR tests; those are forthling's own, in `tests/xchartest.fth` (in the
style of the suite, with `tester.fr`), and run through `suite_xchar`.

### CI and hygiene (since 2026-10-30)

GitHub Actions (`.github/workflows/ci.yml`) runs on every push and pull request:
`cargo fmt --check`, `cargo clippy --all-targets -D warnings` and `cargo test` on Rust 1.85
(the `rust-version` in `Cargo.toml`; raised from 1.74 on 2026-10-31, see "Next session"),
plus `cargo test` on
the latest stable on Linux and Windows. There the test suite is cloned at commit `4c87e56`.
If you raise the `rust-version`, change the toolchain in `ci.yml` too. `.gitattributes`
sets all text files to LF, also in a Windows checkout.

Cleanups, done on 2026-10-31:
- `pop_str()` (in `memory.rs`) pops `( c-addr u )`, checks it and returns the index and the
  length; a negative length counts as zero. It replaces `pop2` + `check` in ten places.
  `TYPE` and `FILL` still do their own `if n > 0`, because they allow any address with
  length zero.
- The THROW codes of table 9.1 have names (`STACK_UNDERFLOW`, `INVALID_ARGUMENT`,
  `OPEN_FILE_FAILED`, …) in `throw.rs`; the message table uses the same names. New code uses
  the name, never the number. The tests keep the numbers, since those are what the standard
  fixes.
- Compile-only primitives are registered with `compile_imm` or `compile_prim`; `invoke`
  then gives `-14` (with the name of the word) when no definition is being compiled, also
  through `EXECUTE`. `SEE` reports `is a compile-only primitive`. `need_def()` remains only
  for the `Compile` instruction of `POSTPONE`.
- `R/O W/O R/W BIN` are in the prelude; `R_O` and `W_O` in `files.rs` must match them.

### Code layout (since 2026-10-30)

The interpreter is a library crate (`src/lib.rs`) with a thin binary (`src/main.rs` calls
`cli::main`; the REPL, running files and the options are in `src/cli.rs`). Every module starts with `use crate::*;`; `lib.rs` re-exports the
items of all modules, so they are available everywhere. The methods of `Forth` are spread
over the modules and are `pub(crate)`, except the public API in `lib.rs` and `host.rs`.

| File | Contents |
|---|---|
| `main.rs` | `fn main` of the program: `forthling::cli::main(&[])` |
| `cli.rs` | the program (public, `cli::main(extensions)`): the REPL, running files, the options (`--memory`, `--blocks`) and the config file |
| `lib.rs` | `struct Forth`, `Forth::new`/`with_memory`, the public API (`interpret`, `include`, `set_input`, `output`, …) |
| `throw.rs` | `Throw`, the named THROW codes and their messages |
| `stack.rs` | data stack, return stack, helpers for double cells |
| `memory.rs` | memory map (constants), `get`/`set`/`check`/`reserve`, strings in memory |
| `dictionary.rs` | `Word`, `Kind`, `define`/`find`, `SEE` and `WORDS` |
| `parse.rs` | parsing the input buffer, numbers, pictured numeric output |
| `interpreter.rs` | the outer interpreter, input sources (`Source`, `Origin`), reading the terminal |
| `terminal.rs` | the trait `Input` and `Stdin` (standard input through a thread, for `KEY?`) |
| `blocks.rs` | the block file and the block buffers (`Blocks`), `LOAD` and `LIST` |
| `heap.rs` | the heap of `ALLOCATE FREE RESIZE` (`Heap`), `data_end()` |
| `json.rs` | the JSON words (`Json: Extension`): reading in place (`whole`, `entries`, `follow`, `unescape`), writing (`Writers`, `<JSON JSON>`) |
| `sandbox.rs` | `Limits`, `Permission`, the budget of an evaluation (`evaluation`, `step`), `emit_bytes`/`emit_memory`, `sleep` |
| `host.rs` | the host API: `Extension`, `wordlist`, `add_word`, `state`, `pop_bytes`/`push_bytes`, `memory`, `stack`, `take_output` (tested from outside in `tests/host.rs`) |
| `float.rs` | the float stack (`fpush`, `fpop`), `F@`/`F!` in memory, text to float (`to_float`) and float to text (`represent`, `fixed`, `scientific`) |
| `xchar.rs` | UTF-8: encoding, decoding and column width of xchars, `XKEY` and `PARSE` with an xchar |
| `files.rs` | file ids, `INCLUDED`/`REQUIRED`, helpers for the FILE words |
| `run.rs` | the inner interpreter: `Instr`, frames, loops, `CATCH` |
| `compiler.rs` | `Ctrl`, helpers for definitions and control flow, `TO`/`IS` |
| `words/*.rs` | the primitives by group (`stack`, `double`, `memory`, `console`, `define`, `control`, `string`, `tools`, `system`, `file`, `facility`, `heap`, `locals`, `search`, `block`, `xchar`, `float`) |
| `prelude.fs` | words written in Forth itself (`include_str!`) |
| `tests.rs` | unit tests (may use the internal state); the suites are in `tests/suite.rs` |

A new word set gets its own `words/<name>.rs` with an `install(f)`, called in
`words/mod.rs`; the order there is the order in the dictionary.

### Next session: resume here

- **Next step:** step 6 of the revised order, the MCP server. All word sets are there
  (except `;CODE ASSEMBLER CODE EDITOR`), as are the statement of section 4.1, the host
  API, the Python module, the SQLite words, the sandbox and the JSON words.
- **JSON (2026-10-31):** step 5 of the revised order is done, in `src/json.rs`, in the
  core at the maintainer's request (no dependency is needed), as the extension
  `forthling::Json` (word list `JSON`, `FORTHLING-JSON`). `Forth::new()` stays plain
  standard; the program (`main.rs`, also `forthling-sqlite`) and the Python module install
  it. Reading works on the text in place: a JSON value is `( c-addr u )` and the words
  return pieces of it (no handles, no `FREE`, no memory outside Forth); each lookup reads
  the text again (`whole` validates, `entries` lists an array or object), nesting up to
  `MAX_DEPTH` 512 so the Rust stack is safe. A missing key or wrong kind gives flag 0, no
  THROW; a key that occurs twice gives the last. `JSON>STRING` and `JSON-NAME` unescape
  into `ALLOCATE`d memory (surrogate pairs joined, a lone one becomes U+FFFD). Writing
  goes to the output (through `emit_bytes`, so the output limit holds) or, between
  `<JSON` and `JSON>` (like `<# #>`, nestable, at the maintainer's request), to a string
  that `JSON>` gives `ALLOCATE`d; that string may not grow past the memory size (-59).
  The writer tracks the open arrays and objects (`Writers` in `Forth::state`) and puts
  the commas; a structure that does not fit throws -22, and `reset` forgets the writers
  after an error (`forget_state`). Tests: `tests/json.rs`, `test_json` in python.
- **Sandbox (2026-10-31):** step 4 of the revised order is done, in `src/sandbox.rs`.
  `Limits { steps, time, output }` per evaluation (a call of `interpret`/`include` by the
  host; `evaluation` fills the `Budget` only at depth 0, so a host word that calls
  `interpret` shares it). `step()` is counted in the loop of `run` and in `execute`; its
  fast path is one test (`steps % 4096`), the rest is in the cold `step_slowly` (clock,
  limit reached). A reached limit sets `steps` to 0, so it stays reached: CATCH cannot go
  on. About 2% slower in the benchmark (a DO loop of 30 million and `27 FIB`). Output goes
  only through `emit_bytes`/`emit_memory` (the six places that wrote `out` directly were
  changed). `MS` goes through `sleep`, which throws at once if it would pass the time
  limit. `Permission::Files` (a set `denied`): the FILE words with a name give the ior
  -259; `INCLUDE INCLUDED REQUIRE REQUIRED` and the block file throw -259. The host's own
  `include` is not checked: the sandbox restricts the program, not the host. Codes:
  `LIMIT_EXCEEDED` -258 (detail `steps`, `time` or `output`), `NOT_PERMITTED` -259, both
  public. `Forth::sandboxed()`: `Limits::SANDBOX` (10M steps, 5 s, 1 MiB), no files,
  empty terminal input. Program: `--sandbox --max-steps --max-time --max-output`
  (`none` lifts a limit; every long option now also takes `--name=value`); the files on
  the command line are still read. Python: `sandbox=`, `max_steps=`, `max_time=`,
  `max_output=`. SQLite: without files only `:memory:`, and `SQLITE_LIMIT_ATTACHED` 0
  so that `ATTACH` and `VACUUM INTO` cannot write a file (rusqlite feature `limits`).
  Tests: `tests/sandbox.rs`, `sandbox` tests in `python/tests` and
  `the_sandbox_keeps_sql_away_from_files`.
- **Sandbox, second round (2026-10-31, with JSON):** reviewing how the sandbox treats
  JSON showed three gaps, now closed. (1) A step could be much work, and the clock is read
  only every 4096 steps: `charge(bytes)` counts one step per 64 bytes (`BYTES_PER_STEP`)
  and reads the clock when it passes a multiple of 4096; called by `MOVE CMOVE CMOVE> FILL
  COMPARE SEARCH TYPE DUMP EVALUATE REPLACES SUBSTITUTE UNESCAPE X-WIDTH`, by
  `ALLOCATE`/`RESIZE` (16 bytes per heap block, for first fit) and by the JSON words (the
  text length, times the path segments for `JSON@`). `SEARCH` was naive O(n·m) and is now
  Knuth-Morris-Pratt (`find` in `words/string.rs`). (2) Nested `<JSON` strings counted
  each on its own: now all together at most the memory size, and `<JSON` and `JSON[ JSON{`
  nest at most 512 deep. Validation no longer collects entries (`entries(…, collect)`).
  (3) Definitions, code, word lists and `REPLACES` texts live in Rust, outside Forth
  memory, and could grow without bound over many evaluations: `Limits::space` (4 MiB in
  the sandbox, `--max-space`, `max_space=`) counts them (`Word::size`, `Forth::space`,
  `system_space` for the system and extensions; `new_word` instead of `define` for the
  program's words, `count_code` at `;`, `check_space` in `WORDLIST`, `REPLACES`, the JSON
  writers and every 4096 steps for code being compiled; `forget_from` gives it back, also
  in `reset` and `catch`). The core tests of the suite take about 100 KiB. The benchmark
  stays within about 2%.
- **SQLite (2026-10-31):** step 3 of "Later: extensions" is done, in `sqlite/`: a crate
  `forthling-sqlite` (own `[workspace]`, like `python/`) with rusqlite 0.40 `bundled`
  (0.41 needs Rust 1.89). The library has `Sqlite: Extension`; the binary
  `forthling-sqlite` is `forthling::cli::main(&[&Sqlite])`. For that the program moved
  from `main.rs` to the public module `cli.rs` (`cli::main(extensions)`, which installs
  them and names them in the banner; `--help` shows the program's own name). Instead of
  the `forthling-cli` crate with Cargo features from the design, each extension crate has
  its own program for now; a joint program can follow once there are more. Words in
  `SQLITE`: `DB-OPEN DB-CLOSE DB-EXEC DB-PREPARE DB-BIND DB-BIND-TEXT DB-BIND-FLOAT
  DB-BIND-NULL DB-STEP DB-RESET DB-FINALIZE DB-COLUMNS DB-COLUMN DB-COLUMN-INT
  DB-COLUMN-FLOAT DB-NULL? DB-CHANGES DB-LAST-ROWID DB-ERROR`. A rusqlite `Statement`
  borrows its `Connection`, so a statement keeps its SQL and parameters, and its first
  `DB-STEP` runs it and reads all rows (`rows`), which the next steps hand out; binding a
  parameter or `DB-RESET` starts again. One counter numbers databases and statements, so
  a database is never taken for a statement. Codes: -260 (SQLite error, message in
  `DB-ERROR`), -261 (invalid handle). `DB-COLUMN` and `DB-ERROR` return `ALLOCATE`d
  strings (no hidden transient buffer, which a `FREE` by the program could corrupt).
  Tests in `sqlite/tests/sqlite.rs`, including `sqlite/examples/people.fs`; CI job
  `sqlite` on 1.85 and stable (Linux) and stable (Windows).
- **Minimum Rust 1.85 (2026-10-31):** raised from 1.74 (November 2023, chosen only because
  it was installed) to 1.85, the Rust of Debian stable and the first with edition 2024;
  PyO3 0.30 needs it too. The local toolchain is stable (1.99.1 on 2026-10-31); CI checks
  1.85. Clippy differs between the two (1.85 wants parentheses around `a << n | b`, 1.99
  knows lints that 1.85 lacks), so check both: `cargo clippy` and `cargo +1.85 clippy`
  (1.85 is installed locally with `--profile minimal`).
- **Edition 2024 (2026-10-31):** both crates are edition 2024. `cargo fix --edition`
  changed no code; the rustfmt style of 2024 put short `if … else` on one line (as
  `use_small_heuristics = "Max"` in `rustfmt.toml` intends) and sorts imports differently.
  `python/Cargo.toml` has `rust-version = "1.85"` too, so clippy does not suggest let
  chains (`if let … && …`, Rust 1.88) and resolver 3 picks dependencies that build on 1.85.
  Let chains are an option once the minimum is raised to 1.88.
- **Python module (2026-10-31):** step 2 of "Later: extensions" is done, in `python/`: a
  crate `forthling-py` with an empty `[workspace]` of its own, so PyO3 does not end up in
  the core's `Cargo.lock` and plain `cargo` commands in the root still see only the core
  (a joint workspace can follow once there are more extension crates). PyO3 0.30
  (first 0.27, while the minimum was Rust 1.74), which knows Python 3.14; maturin
  as build backend (`pip install ./python`). The class `forthling.Forth(memory=None,
  input="")` has `interpret`/`include` (they return the output, via the new core method
  `take_output`), `push pop fpush fpop`, `stack` (new core method `Forth::stack`),
  `finished` and `add_word(name, func, inputs=0, wordlist=None)`. Such a word pops
  `inputs` cells as arguments and pushes the result: None nothing, a bool a flag, an int
  a cell, a float on the float stack, a tuple or list each item. A Python exception in it
  is THROW -257; uncaught, `interpret` raises the original exception again (kept in
  `pending`). `ForthError` has `code` and `output`. The terminal input is a string
  (empty by default), so `KEY` never waits on stdin inside Python. ENVIRONMENT? knows
  `FORTHLING-PYTHON`. The tests are in `python/tests/test_forthling.py` (unittest, no
  pytest needed); CI has a job `python` (Python 3.14). Locally `cargo` needs
  `PYO3_PYTHON` set, since Python is not on the PATH; `pip install` does not.
- **Host API (2026-10-31):** step 1 of "Later: extensions" is done, in `src/host.rs`. A
  word added from Rust is `Kind::Host(Rc<dyn Fn(&mut Forth) -> Res>)`: `Fn`, not `FnMut`,
  so a word may run itself again (through `interpret`) without a `RefCell` panicking;
  mutable state goes in `Forth::state::<T>()` (a map from `TypeId` to `Box<dyn Any>`) or
  in the closure's own `Cell`/`RefCell`. `Forth::wordlist(name)` makes a named word list
  (or returns the existing one) and defines `NAME` in FORTH-WORDLIST, which acts like
  `FORTH`. `Forth::install(&dyn Extension)` adds the words, records name and version for
  `ENVIRONMENT?` (which then gives `version true`) and moves the FORGET fence up. Public
  now: `push pop fpush fpop pop_str`, `Throw::new`/`with` and `Res`; new: `depth
  pop_bytes push_bytes memory memory_mut`. Host words are never immediate; the THROW
  constants stay crate-private, so an extension uses the numbers.
- **Roadmap in English (2026-10-31):** this file used to be the Dutch `STANDAARD.md`; at the
  maintainer's request it is now in English only.
- **Community standards (2026-10-31):** `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1;
  reports through an issue or GitHub's "Report content", on request no e-mail address),
  `CONTRIBUTING.md`, `SECURITY.md` (reports through private vulnerability reporting), the
  issue forms in `.github/ISSUE_TEMPLATE/` and `.github/pull_request_template.md`. All in
  English. Keep the conventions in `CONTRIBUTING.md` up to date when they change.
- **Behaviour points (2026-10-31):** three ambiguous conditions resolved differently than
  before. `LSHIFT`/`RSHIFT` by 64 or more (or by a negative count) give 0 (`shift` in
  `words/double.rs`). `UNESCAPE` throws -9 when its result runs past the end of its buffer;
  `Forth::buffer_end` (in `memory.rs`) knows the fixed buffers, the data space and the heap
  blocks (`Heap::region_end`). An empty search order still finds the `ROOT_WORDS` (in
  `dictionary.rs`): `FORTH FORTH-WORDLIST GET-ORDER ONLY ORDER SET-ORDER`.
- **Statement (section 4.1):** `COMPLIANCE.md` (English) and `VERKLARING.md` (Dutch), in
  both languages on request (2026-10-31). They follow the order of the standard: the
  labeling of section 5.1.1, then for each word set the implementation-defined options
  (x.4.1.1), the ambiguous conditions (x.4.1.2) and the other documentation (x.4.1.3), and
  with EXCEPTION the table of THROW codes. Every answer was checked in the code or with a
  probe in the REPL. **Keep both up to date** when behaviour changes; the README names only
  the main choices and refers to them. Found and fixed along the way: `TRAVERSE-WORDLIST`
  gave a Rust panic when the xt shortened the dictionary (a marker); now the walk goes on
  below the new end.
- **FLOATING:** `float.rs` and `words/float.rs`; the sizes, `FCONSTANT FVARIABLE`, the
  field words and the `DF` words are in the prelude. Floats are IEEE 754 doubles on a
  separate stack (`Forth::fstack`, 1024 deep, `FSTACK_FLOATS`; errors `-44`/`-45`). A float,
  also a DFLOAT, is 8 bytes and aligned like a cell; an SFLOAT is 4 bytes. Arithmetic
  follows IEEE: dividing by zero gives `inf` or `nan`, not a THROW. `F>D` and `F>S` give
  `-11` when the result does not fit. The interpreter recognises literals (`1E`,
  `-1.5e-3`) only when BASE is 10, and they must have an `E` and digits before the point;
  `>FLOAT` follows the wider syntax (`.5`, `9d-`, `1+1`, blanks = 0). Text is converted by
  Rust's own parser, which rounds correctly. `PRECISION` defaults to 15, and
  `SET-PRECISION` keeps it between 1 and 17. `F. FE. FS.` follow gforth (`1000.`,
  `333.33E-3`, `3.3333E0`); `inf`, `-inf` and `nan` are written as such. `FROUND` rounds
  half to even. `CATCH` also restores the depth of the float stack; an error empties it.
  New instructions: `FLit` and `FStore` (`TO` of an `FVALUE`, `Kind::FValue`).
  `fpzero-test.4th` uses `F>`, which is not a standard word; `suite_floating` defines it
  first (paranoia does so itself too).
- **XCHAR:** UTF-8. The helpers (`encode`, `decode`, `last_start`, `width`, `read_xkey`,
  `parse_seq`) are in `xchar.rs`, the words in `words/xchar.rs`; `XKEY?` and `EKEY>XCHAR` in
  the prelude. A character (`CHAR`, `C@`, `KEY`, `EMIT`) stays a byte, and `MAX-CHAR` stays
  255. Malformed UTF-8 is read byte by byte: a byte that does not start a complete xchar is
  one itself, with the byte's value. A value above `MAX-XCHAR` becomes U+FFFD when written.
  `CHAR`, `[CHAR]` and `'x'` give the whole xchar; `PARSE` takes an xchar as delimiter.
  `EKEY` is now `XKEY` (whole xchars), and `EKEY>CHAR` is true only for ASCII (< `$80`).
  `XC-WIDTH` knows the common ranges of Unicode (0 for control and combining characters, 2
  for wide East Asian characters and emoji), not the full tables. `XCHAR-ENCODING` gives
  `UTF-8`, a string on the heap made the first time it is asked for
  (`Forth::encoding_name`).
- **BLOCK:** `blocks.rs` and `words/block.rs`; `THRU` is in the prelude. The file defaults
  to `blocks.fb` in the current directory; `-b`/`--blocks` or `blocks = …` in the config
  file picks another (on request, 2026-10-31; in the library `Forth::set_block_file`). It
  is opened or created only on first use. 8 buffers (`BLOCK_BUFFERS`) at `BLOCK_BUFS`,
  below `DICT_START`; least-recently-used replacement through a counter. `BLK` and `SCR`
  are system variables at 24 and 32. `LOAD` pushes a `Source` with `Origin::Block`; its
  `line` is the block number, so `SAVE-INPUT` and `RESTORE-INPUT` work across blocks by
  themselves, and `refill()` loads the next block (`load_block`). `sync_blk()` sets `BLK`
  after every change of source (push, pop, `CATCH`, `reset`). In a block, `\` skips to the
  end of the 64-character line. Errors: `-33 -34 -35`. The suites and unit tests use a
  block file in the temp directory (`run_suite`, `run_blocks`); `blocks.fb` is in
  `.gitignore`.
- **SEARCH:** `words/search.rs`, also with `TRAVERSE-WORDLIST NAME>STRING NAME>INTERPRET
  NAME>COMPILE`. Every `Word` has a `wid`; `Forth::wordlists` (name per wid - 1), `order`
  (searched first at the front) and `current_wid`. `FORTH_WID` is 1 and `MAX_ORDER` 16 (in
  `dictionary.rs`). `find()` walks the search order with `find_in(name, wid)`. An nt is the
  xt. `NAME>STRING` copies the name to the heap the first time it is asked for
  (`Word::name_addr`); `MARKER` frees those blocks and also restores the word lists, the
  search order and `current_wid`. Errors: `-49 -50`, and `-24` for an invalid wid or nt.
- **FORGET (2026-10-31, on request):** in `words/define.rs`. Every `Word` remembers `here`
  of its definition; `FORGET` looks only in the compilation word list, sets `HERE` back
  and deletes the word and everything after it (`forget_from`, shared with `MARKER`). Words
  below `Forth::fence` (system and prelude) give `-15`.
- **LOCALS:** `words/locals.rs`. Locals are on a separate stack `Forth::locals`;
  `Frame::locals` is where the frame's locals start, and `leave_frame`, `CATCH` and `reset`
  cut it back. Instructions: `Locals(take, zeroed)` takes `take` cells from the data stack
  (the deepest becomes the first local) and puts `zeroed` zeros after them; `Local(i)` and
  `ToLocal(i)` read and write local i of the frame. While compiling, the names are in
  `local_names`; `handle` and `TO` look there first (`find_local`, only while compiling).
  `;`, `DOES>`, `start_def` and errors clear them (`end_local_scope`). `(LOCAL)` and
  `LOCALS|` give the first name the top cell: `end_stack_order_locals` reverses their
  names. At most `MAX_LOCALS` (256) per definition.
- **MEMORY:** the heap (`heap.rs`) grows from the top of memory down towards `HERE`, so
  data space and heap share the free space. `Heap` keeps the blocks in a `BTreeMap` (start
  → size, whole cells); `floor()` is the lowest block and `data_end()` the limit for
  `reserve`, `ALLOT` and `UNUSED`. `ALLOCATE` takes the first gap that fits, otherwise the
  heap grows further down. `RESIZE` grows in place when the space after the block is free,
  otherwise the block moves. Iors: `-59 -60 -61`. `MARKER` does not touch the heap.
- **Memory size (2026-10-31):** 1 MiB by default (`DEFAULT_MEMORY`), set with
  `-m`/`--memory 4M` or with `memory = 4M` in `forthling/forthling.conf` in the config
  directory (`%APPDATA%`, `$XDG_CONFIG_HOME` or `~/.config`); the flag wins. Allowed: 64K
  to 4G, a multiple of 8. For this, all buffers are now at fixed addresses at the bottom
  (`SOURCE_AREA` from 64, then `SQ_BUFS WORD_BUF HOLD_START…HOLD_END PAD_ADDR`) and the data
  space starts at `DICT_START` above them; only the top of the heap depends on the size.
  Options and config are in `cli.rs` (`parse_args`, `parse_config`, `Settings`, `Run`);
  the library knows only `Forth::with_memory` and `Forth::set_block_file`.
- **Decided (2026-10-31):** `TIME&DATE` stays UTC.
- **FACILITY:** structures, `EKEY EKEY? EMIT? EKEY>CHAR EKEY>FKEY` and the `K-…` constants
  are in `prelude.fs`; `KEY? MS TIME&DATE AT-XY PAGE` in `words/facility.rs`. The terminal
  (`terminal.rs`) is a trait `Input` (a `BufRead` with `ready()`); standard input (`Stdin`)
  is read line by line by a thread, which starts only on first use. So `KEY?` does not
  block: it is true as soon as a whole line has been typed. `EKEY` gives characters only
  (since XCHAR, whole xchars), since the terminal is not in raw mode; special keys have
  numbers above Unicode (`$110000` …) but never occur. `TIME&DATE` gives UTC (std knows no
  local time zone; that would need FFI). `AT-XY` and `PAGE` write ANSI escapes.
- **DOUBLE:** the arithmetic words are primitives on `i128` (`d_unop`, `d_binop`,
  `d_compare`); `2LITERAL 2ROT D. D.R` are in the prelude. `number()` now gives
  `(u128, bool)`: a point at the end makes it a double cell (only at the end, as the
  standard says). `M*/` computes through a 192-bit product (`mul_div`), floored like the
  rest. `2VALUE` is `Kind::TwoValue`; `TO` compiles two `Instr::Store`s for it.
- **FILE:** open files are in `Forth::files` (file id n is `files[n - 1]`, free ids are
  reused). `INCLUDED` opens the file, gets a file id (which is also the `SOURCE-ID`) and
  calls `include_file`, which reads the rest of the file and closes it afterwards, also on
  an error. `REQUIRED` remembers names through `fs::canonicalize` in `Forth::included`. A
  relative path that does not exist is also looked for next to the file being included
  (that is how `filetest.fth` finds its `required-helper*.fth`). The ior of a failing file
  word is its own THROW code (`-62` … `-76`). `READ-LINE` reads `u1 + 2` bytes and then
  sets the file position back to just after the line. In a file, `(` continues over
  lines. `suite_file_access` creates `fatest*.txt` in the project directory and removes
  them again (they are in `.gitignore` in case a test fails).
- **STRING:** `COMPARE SEARCH CMOVE CMOVE> SLITERAL REPLACES SUBSTITUTE UNESCAPE` are
  primitives, `/STRING BLANK -TRAILING` are in the prelude. The texts of `REPLACES` are in
  `Forth::substitutions` (a `HashMap`, names in upper case); `MARKER` does not restore it.
  `SUBSTITUTE` builds the result in a `Vec` first, so overlapping buffers work; if it does
  not fit, it gives `-78`. A name with `%` gives `-79` with `REPLACES`.
- **TOOLS:** `[IF]` and `[ELSE]` skip words with `skip_conditional`, also across lines
  (through `refill()`), and count nested `[IF]`s. `CS-PICK`/`CS-ROLL` work on the
  control-flow stack `ctrl` (`Ctrl` is now `Copy`). `SYNONYM` copies `kind` and
  `immediate` from the old word. `N>R` puts n cells and then n on the return stack. `DUMP`
  shows hex and ASCII, 16 bytes per line. `?` is in the prelude.
- **`CATCH`:** records the depth of the data stack, return stack, loops, frames, input
  sources, `>IN`, `STATE` and the control-flow stack, and restores them on a `THROW`. A
  definition that started inside `CATCH` is deleted. `QUIT` passes through `CATCH`.
- **Rust stack:** every `CATCH` nests a `run` loop in Rust. So the nesting is limited to
  `MAX_CATCHES` (1000; deeper gives `-5`), and the interpreter runs in a thread with a
  64 MiB stack (`main` → `run_forth`). Without that thread, 5000 nested `CATCH`es crashed on
  Windows.
- **Measuring:** follow `runtests.fth`: `utilities.fth` and `errorreport.fth` come before
  the word set tests, and the `ACCEPT` test in `core.fr` wants a line of input (through a
  pipe).
- **Input sources:** every `Source` has an `Origin` (terminal, `EVALUATE`, a file with all
  its lines) and a line counter. `refill()` loads the next line; `include` is
  `while refill() { interpret_buffer() }`. In a file, `RESTORE-INPUT` can go back to an
  earlier line; with the terminal and `EVALUATE` only within the current line.
- **New kinds of words:** `Kind::Value`, `Kind::Defer` and `Kind::Marker`. `TO`, `IS` and
  `ACTION-OF` compile `Instr::Store`/`Instr::Fetch`; `CASE` uses `Ctrl::Case/Of/EndOf` and
  `Instr::Of`/`Instr::Drop`. A `:NONAME` word stays hidden (empty name).
- **Errors and `QUIT`:** `ABORT` throws `-1`, `ABORT"` throws `-2` with the text as detail
  (`Instr::Abort`), `QUIT` throws `-56` (`QUIT` in the code). `reset(clear_stack)` keeps the
  data stack on `QUIT`. In file mode, `QUIT` ends the run without a message.
- **Stack limits:** the data stack and the return stack have 65536 cells each
  (`STACK_CELLS`, `RSTACK_CELLS`); overflow gives `-3`/`-5`. `ENVIRONMENT?` reports these
  values.
- **Terminal input:** `Forth::input` is the only input channel of the terminal; the REPL,
  `ACCEPT` and `KEY` all read from it. Tests put an `io::Cursor` in it. `KEY` reads line
  buffered (the terminal is not in raw mode), see the README.
- **Decided (2026-10-30):** division is floored. `/ MOD /MOD */ */MOD` are in the prelude
  and use `FM/MOD`. Documented in the README.
- **Fixed:** `WHILE` now leaves its orig *below* the dest, so several `WHILE`s can share
  one `BEGIN` (`core.fr` GI5).
- **Measuring (in general):** run the test files in `tests/forth2012-test-suite/src` with a
  file that loads them with `INCLUDE`. Then see which line it reaches and how many
  `INCORRECT RESULT`/`WRONG NUMBER OF RESULTS` there are.
- **Formatting:** the code follows `rustfmt` (settings in `rustfmt.toml`). Run `cargo fmt`
  before every commit; `cargo fmt -- --check` must be clean. The formatting commit
  `7d16ded` is in `.git-blame-ignore-revs`, so `git blame` skips it.
- **Repo:** https://github.com/DeSchaker/forthling (public, MIT). The test suite is local in
  `tests/forth2012-test-suite` and ignored by git.
- **Windows:** PowerShell 5.1 passes arguments with `"` or `/` wrongly to `git` and `gh`.
  Use Bash for those, or a file for commit messages.

Most missing words are small. The real work is in a few architectural choices that now
live outside Forth, in Rust structures. Those come first.

---

## Phase 0: the foundation ✅ done (2026-10-29)

What was built:
- **Test suite**: cloned to `tests/forth2012-test-suite` (fixed commit `4c87e56`; the repo
  has no license file, so it does not belong in our own source). To fetch it again:
  `git clone https://github.com/gerryjackson/forth2012-test-suite.git tests/forth2012-test-suite`
  and then `git -C tests/forth2012-test-suite checkout 4c87e56`. `cargo test` runs
  `prelimtest.fth` and checks `tester.fr`. If the directory is missing, those two tests are
  skipped with a message.
- **Memory**: 1 MiB, byte addressable, cells of 8 bytes (little-endian). `CREATE` aligns.
  Memory map (`src/memory.rs`, at the top): system variables (`BASE`, `STATE`, `>IN`) from
  address 0, data space from 64, transient buffers and input buffers at the top. (Since
  2026-10-31 the buffers are at the bottom and the heap at the top; see "Memory size".)
- **Input**: every input source (the terminal, and each nested `INCLUDE`) has its own line
  buffer in memory. `>IN` is restored after a nested source. The parser reads `>IN` again
  for every word, so a program can move the position itself. After a word, the space after
  it is skipped too, as the standard prescribes.
- **New words**: `SOURCE >IN STATE WORD COUNT FIND`. These are needed for `prelimtest.fth`
  and `tester.fr`, and belong to the new input model.
- **Errors**: `Throw { code, detail, location }` with codes from table 9.1. Messages look
  like `file.fs:3: error -13: undefined word: FOO`.
- **Transient buffers**: in interpretation state, `S"` uses two alternating buffers and
  leaves `HERE` alone. The space for `PAD`, the buffer of `WORD` and `<# #>` is reserved.

The original plan for this phase:

### 0.1 Test suite as yardstick
- Fetch Gerry Jackson's [forth2012-test-suite](https://github.com/gerryjackson/forth2012-test-suite)
  (`prelimtest.fth`, `tester.fr`, `core.fr`, `coreexttest.fth`, etc.).
- Add a `cargo test` that runs the suite and counts the errors. Each phase is "done" when
  its test file runs without errors.
- Note: `tester.fr` already uses `SOURCE TYPE` to report errors, so 0.3 is needed before
  the suite gives proper error messages.

### 0.2 Byte-addressable memory
Now one address is one 64-bit cell, and a character is a cell too. Formally that is
allowed (by section 3.1.2 a character is at least one address unit, and a cell holds a whole
number of them). Still, I recommend switching:
- memory becomes `Vec<u8>`, a cell is 8 bytes, `1 CELLS` = 8 and `1 CHARS` = 1;
- `@ !` read/write 8 bytes (little-endian), `C@ C!` one byte;
- `ALIGN`/`ALIGNED` get a real meaning;
- strings become compact, and `MOVE`, `FILL`, `COUNT` and counted strings (`C"`, `WORD`)
  behave like in any other Forth and as the test suite expects.

Touches: `mem`, `here`, `addr()`, `comma()`, `TYPE`, `S"`, `CELLS`/`CELL+`/`CHARS` in the
prelude.

### 0.3 Interpreter state in Forth memory
The standard lets programs read and write the state of the interpreter:
- `STATE`: an address holding the compile flag (now the Rust field `compiling`);
- `>IN`: an address holding the parse position (now `pos`);
- `SOURCE`: the address and length of the input buffer (now a `Vec<char>` outside memory).

Plan: reserve fixed cells for `STATE` and `>IN` (like `BASE` now) and an input buffer in
memory. `word()` and `parse()` read from there from now on. That also makes `WORD`,
`PARSE`, `PARSE-NAME`, `EVALUATE`, `REFILL`, `SOURCE-ID` and `SAVE-INPUT`/`RESTORE-INPUT`
possible. Input sources (keyboard, an `EVALUATE` string, a file) become a stack, so they can
be nested.

### 0.4 Error handling with THROW codes
Errors are now `String`s. Replace them with standard THROW codes (table 9.1), for example
`-4` stack underflow, `-10` division by zero, `-13` undefined word, `-14` compile-only.
- `Res` becomes `Result<T, Throw>` with a code and an optional text.
- `ABORT` = throw `-1`, `ABORT"` = `-2`, `QUIT` = `-56` (the REPL then only clears the
  stacks).
- With that, `CATCH`/`THROW` (the EXCEPTION word set) come almost for free.

### 0.5 Transient buffers
- In interpretation state, `S"` now writes at `HERE` and moves it. That must become a
  transient buffer (at least two, alternating, as the FILE word set asks).
- Also reserve space for `PAD`, the buffer of `WORD` and the pictured output buffer
  (`<# … #>`).

---

## Phase 1: finishing CORE ✅ done (2026-10-30)

In this order, because later groups lean on earlier ones:

1. ✅ **Double cells and arithmetic**: `S>D M* UM* UM/MOD FM/MOD SM/REM */MOD 2@ 2!`.
   Division is floored; `/` and `MOD` are defined through `FM/MOD`.
2. ✅ **Pictured numeric output**: `<# # #S HOLD SIGN #>`, plus `HOLDS` from CORE EXT. `.`
   and `U.` are now written in Forth in the prelude; `.R` and `U.R` (CORE EXT) can follow
   easily.
3. ✅ **Memory and strings**: `ALIGN ALIGNED COUNT FILL MOVE`.
4. ✅ **Parsing and input**: `ACCEPT KEY` (`SOURCE >IN WORD` exist since phase 0,
   `EVALUATE` and `>NUMBER` since 2026-10-30. `EVALUATE` is a nested `Source` whose buffer
   is the string itself; `>NUMBER` and our own number conversion share `to_number`).
5. ✅ **Dictionary**: `>BODY` (`STATE` and `FIND` exist since phase 0).
6. ✅ **System**: `ABORT ABORT" QUIT ENVIRONMENT?`. `ENVIRONMENT?` answers at least
   `/COUNTED-STRING /HOLD /PAD ADDRESS-UNIT-BITS FLOORED MAX-CHAR MAX-D MAX-N MAX-U
   MAX-UD RETURN-STACK-CELLS STACK-CELLS`.

**Milestone:** `core.fr` runs without errors.

---

## Phase 2: CORE EXT ✅ done (2026-10-30)

Grouped from easy to hard:

- **Trivial (prelude)**: `0<> U> WITHIN ERASE 2>R 2R> 2R@ ROLL UNUSED PAD .R U.R`
- **Control structures**: `CASE OF ENDOF ENDCASE`, built from `IF`/`THEN`, with their own
  `Ctrl` marker.
- **Compiler**: `:NONAME` (a definition without a name that leaves an xt), `COMPILE,` and
  `[COMPILE]` (obsolescent, but cheap).
- **Values**: `VALUE TO`, and `DEFER IS DEFER! DEFER@ ACTION-OF`. Both need a new `Kind`;
  `TO` and `IS` are state-smart (when compiling they record an instruction).
- **Data**: `BUFFER:`, `C"` (counted string, needs 0.2).
- **Strings with escapes**: `S\"` (`\n \t \" \\ \x41` etc.).
- **Input**: `PARSE PARSE-NAME REFILL SOURCE-ID SAVE-INPUT RESTORE-INPUT` (needs 0.3).
- **Undoing the dictionary**: `MARKER`. It saves the length of `dict` and `HERE` and
  restores them when executed. With the `Vec<Word>` design this is easy.

**Milestone:** `coreexttest.fth` runs without errors.

---

## Phase 3: optional word sets (in order of value against effort)

| Priority | Word set  | Why / what |
|---------:|-----------|------------|
| 1 ✅ | EXCEPTION | `CATCH THROW`, plus `ABORT ABORT"` from EXCEPTION EXT. Done 2026-10-30. |
| 2 ✅ | TOOLS     | `.S ? DUMP SEE WORDS`, `[IF] [ELSE] [THEN]`, `AHEAD`, `CS-PICK`, `CS-ROLL`, `BYE`, `N>R`/`NR>`, `[DEFINED]`/`[UNDEFINED]`, `SYNONYM`. Done 2026-10-30; `NAME>…` and `TRAVERSE-WORDLIST` came with SEARCH (2026-10-31). |
| 3 ✅ | STRING    | `COMPARE SEARCH /STRING -TRAILING BLANK CMOVE CMOVE> SLITERAL`, plus `REPLACES`/`SUBSTITUTE`/`UNESCAPE`. Done 2026-10-30. |
| 4 ✅ | FILE      | `INCLUDED INCLUDE REQUIRE`, plus file I/O (`OPEN-FILE READ-LINE WRITE-FILE …`) with real file ids. Done 2026-10-30. |
| 5 ✅ | DOUBLE    | `D+ D- D. D< DNEGATE 2CONSTANT 2VARIABLE M+ M*/ …`, and numbers with a point (`123.`) as double cells. Done 2026-10-30. |
| 6 ✅ | FACILITY  | `KEY? PAGE AT-XY MS TIME&DATE`, plus `BEGIN-STRUCTURE`/`FIELD:`. Done 2026-10-30. |
| 7 ✅ | MEMORY    | `ALLOCATE FREE RESIZE`: a heap that grows down from the top of the data space. Done 2026-10-31. |
| 8 ✅ | LOCALS    | `{: a b | c -- :}`, `LOCALS|`, `(LOCAL)`, on a separate locals stack. Done 2026-10-31. |
| 9 ✅ | SEARCH    | Word lists: `WORDLIST GET-ORDER SET-ORDER DEFINITIONS …`, with the search order in `find()`. Done 2026-10-31. |
| 10 ✅ | FLOATING | A separate float stack and `F+ F. F@ …` (81 words); all `fp/` tests pass. Done 2026-10-31. |
| 11 ✅ | XCHAR    | UTF-8-aware character words, with our own tests (`tests/xchartest.fth`). Done 2026-10-31. |
| 12 ✅ | BLOCK    | Blocks in a file, 8 buffers, `LOAD` as an input source. Done 2026-10-31. |

---

## Later: assembler and editor (recorded 2026-10-30)

`;CODE ASSEMBLER CODE EDITOR` are deliberately missing from TOOLS EXT. `ASSEMBLER` and
`EDITOR` themselves only put a word list first in the search order; since SEARCH
(2026-10-31) that is possible, but without assembler or editor words those lists are empty.

- **A real assembler** (`CODE name … END-CODE` with machine code): needs executable memory
  from the OS (`VirtualAlloc`/`mmap`, hand-written FFI and `unsafe`), a calling convention
  (how machine code reaches the data stack and return stack and returns to the
  interpreter), an assembler vocabulary per platform (x86-64 encoding is hard; ARM is a
  second set), and the risk that a mistake crashes the whole process, bypassing `CATCH`.
  Much work, little gain; only for completeness.
- **Cheap alternative:** the standard leaves open what "machine code" is. A `CODE` that
  assembles forthling's own `Instr` instructions is legitimate and doable, but adds little
  over `:` definitions.
- **Classic editor:** edits *blocks* (16 × 64 characters, words like `L T P`), so it needs
  the **BLOCK** word set too. A screen editor of our own in the terminal needs raw mode
  without crates (Windows Console API and termios): a lot of platform work.
- **Pragmatic, independent of the roadmap:** an extension word `EDIT file` that starts the
  user's editor (`$EDITOR`, otherwise `notepad`/`vi`) through `std::process::Command` and
  then `INCLUDE`s the file. A few lines of code, very handy in practice.

Advice: a real assembler and block editor only after SEARCH and BLOCK, and only if
completeness is worth it; `EDIT` can come in between. Note: `SECURITY.md` calls starting
other programs a vulnerability; change that if `EDIT` arrives.

---

## Later: extensions (design, recorded 2026-10-31)

Wishes: Windows-specific words, graphics, a database, and an interface with Rust and
Python. Not to be built yet; this is the design to take up step by step.

### Principle: only add, never change

The standard does not forbid extra words. It requires that the standard words behave as
described and that the system documents what it adds. As long as extensions only add
words, forthling stays a standard system. For that:

1. **A word list of its own per extension.** A program picks one with `ALSO SQLITE` or
   `ALSO GRAPHICS`. `FORTH-WORDLIST` stays clean, so a standard program never clashes with
   an extension word.
2. **Discoverable.** `S" FORTHLING-SQLITE" ENVIRONMENT?` gives a flag and perhaps a
   version; the standard leaves strings of its own to the system. Besides that,
   `[DEFINED] DB-OPEN [IF] … [THEN]` already works without extra code.
3. **In the style of the standard:**
   - strings as `( c-addr u )`;
   - handles as cells, like file ids;
   - errors as an `ior` on the stack, as in FILE, instead of a THROW;
   - large results in `ALLOCATE`d memory;
   - THROW codes of their own below -255, a block per extension (now only -256,
     `NESTING_TOO_DEEP`, exists).
4. **Reuse existing names.** Gforth, for example, already has an FFI (`c-library`,
   `c-function`); with the same names, existing code also runs on forthling. Check the
   proposals on forth-standard.org, so no name clashes with a future standard word.
5. **Document.** `COMPLIANCE.md` and `VERKLARING.md` get a section
   "Extensions"/"Extensies". Section 5.1.1 allows naming extra capabilities besides the
   word sets.

### Structure: the core stays small

A Cargo workspace:

| Crate | Contents |
|---|---|
| `forthling` | the core: no dependencies, no `unsafe`, Rust 1.85 (as now) |
| `forthling-sqlite` | word list `SQLITE`, through rusqlite (feature `bundled`) |
| `forthling-gfx` | word list `GRAPHICS`, for example through minifb or SDL2 |
| `forthling-win` | word list `WINDOWS`, behind `#[cfg(windows)]` |
| `forthling-ffi` | FFI with libloading; `unsafe` lives here, not in the core |
| `forthling-py` | Python module through PyO3/maturin |
| `forthling-cli` | the binary; picks extensions through Cargo features or `forthling.conf` |

That keeps the promises of the core true: no dependencies, no `unsafe` (see `SECURITY.md`)
and Rust 1.85. An extension may require a newer Rust. CI tests the core on its own and
also with all extensions enabled.

### Key: a host API in the core ✅ done (2026-10-31, `src/host.rs`)

All extensions depend on a public way to add words. Now a primitive is an
`fn(&mut Forth) -> Res` without state of its own, and the methods are `pub(crate)`. Needed
are:

- **Public methods:** `push`, `pop`, `pop_str`, `fpush` and `fpop`, plus access to memory.
- **Words with state:** a closure (`Box<dyn FnMut(&mut Forth) -> Res>`), or a function
  pointer plus a place for the extension's state, such as a map from type to
  `Box<dyn Any>`. That is where SQLite, for example, keeps its open connections.
- **Word lists:** `Forth::wordlist("SQLITE")` makes a named word list; `define_in(wid, …)`
  puts words in it.
- **A trait `Extension`:** `fn install(&self, f: &mut Forth)` plus a name for
  `ENVIRONMENT?`.

This is also the interface with Rust: whoever builds forthling into a program of their own
uses the same API.

### Per wish

- **Python:**
  - Forth from Python is the easy side, with PyO3:
    `f = forthling.Forth(); f.interpret("2 3 + ."); f.pop()`.
  - Python from Forth (for example `PYTHON-EVAL`) means embedding CPython. PyO3 can do
    that too, but then the binary depends on an installed Python. Make it a separate
    feature.
- **Database:** SQLite first, since rusqlite with `bundled` needs no external installation.
  An API in the style of FILE:
  - `DB-OPEN ( c-addr u -- db ior )`
  - `DB-EXEC ( c-addr u db -- ior )`
  - `DB-PREPARE`, `DB-STEP ( stmt -- flag ior )` and `DB-COLUMN ( n stmt -- c-addr u )`
  - `DB-CLOSE`
- **Graphics:** the hard part is the event loop.
  - A window wants to handle events on the main thread, while the REPL waits for stdin.
    The clean solution: `EKEY` and `EKEY?` also deliver window events (the constants
    `K-UP` and the like are ready), plus a word that refreshes the window.
  - Start simple with a framebuffer: `WINDOW ( w h -- )`, `PIXEL ( x y rgb -- )`, `LINE`
    and `FLIP`. Turtle graphics fits nicely on top.
  - Without dependencies it can also be done in the terminal, with ANSI colours and
    Unicode blocks.
- **Windows:** two levels.
  - Fixed words behind `#[cfg(windows)]`, such as `MESSAGE-BOX`, the clipboard and the
    registry.
  - Or a general FFI right away: `LIBRARY user32.dll` and `FUNCTION: MessageBoxW`, as in
    gforth. It is more powerful but unsafe: a wrongly declared function crashes the
    process. Hence a crate of its own and an explicit feature, and a note in
    `SECURITY.md`.

### Proposed order (first version, superseded)

1. ✅ **The host API.** Everything depends on it, and it is cleanup work in the core without
   dependencies.
2. ✅ **The Python module.** Small, and it tests the host API from the outside right away.
3. ✅ **SQLite.** A clear API in FILE style and easy to test.
4. **Graphics.** The most design questions, mainly around the event loop.
5. **FFI and Windows last,** because of `unsafe`.

### Revised order (2026-10-31)

The question was which extensions would make forthling really worth using for developers
and AI agents. Graphics, Windows words and an FFI are fun, but they do not set forthling
apart: for those people reach for Python or Lua. What does set it apart is that it is
**small, predictable and safe**: no dependencies, no `unsafe`, bounded memory, a standard
language, embeddable in Rust and Python. The order builds on that.

For AI agents, forthling can be a safe environment to compute and script in: agents want
to run code that cannot harm the machine or run forever, and a full Python sandbox is
heavy and hard to close off. LLMs are weak at tracking the stack in their head; an
environment that returns the stack after every line makes up for that.

4. ✅ **Sandbox and limits, in the core** (no dependencies). The basis for everything below.
   - A budget of instructions, so that `BEGIN AGAIN` stops with a THROW instead of
     hanging; a time limit; a limit on the output.
   - Capabilities to switch on or off: files, blocks, `BYE`, reading the terminal.
     Extensions (SQLite, HTTP) fall under them too.
   - Through the host API, and as options of the program.
5. ✅ **JSON words.** The language agents and tools speak: parse JSON into Forth data, and
   give the stack and values back as JSON.
6. **An MCP server** (`forthling-mcp`): tools such as `forth_eval` (a session that keeps its
   state between calls), `forth_stack` and `forth_reset`, so any agent can use forthling
   directly. Together with 4 and 5, a safe, persistent calculator and scripting language
   for agents. With it, machine-readable output: errors, the stack and `SEE` as structured
   data, so an agent need not parse text.
7. **WebAssembly and a playground in the browser** (`forthling-web`, wasm-bindgen, built
   with wasm-pack). Forth without installing anything, and the best showcase of the
   project; it can run on a website such as robbertwesterink.nl as static files (an HTML
   page, some JavaScript and a `.wasm` of a few hundred KB), in the visitor's browser, with
   no server running Forth. The core then needs small `#[cfg(target_arch = "wasm32")]`
   adjustments: no threads (the terminal thread for `KEY?`; input comes from the page
   through `set_input`), no sleep or system clock (`MS`, `TIME&DATE`), and no file system
   (FILE and BLOCK words give an ior, later perhaps a file system in memory or
   localStorage). The sandbox of step 4 keeps a `BEGIN AGAIN` from freezing the
   visitor's tab.
8. **Debugging and tracing:** stepping with the stack after each word, breakpoints,
   `TRACE` on and off. Helps people and agents alike.
9. **Practical glue:** an HTTP client, regular expressions, starting processes. Useful,
   but dependencies and risks, so separate extensions, under the capabilities of step 4.
10. **Later, as hobby projects:** graphics, Windows words and the FFI (as designed above),
    and editor support (an LSP with stack effects on hover), once there are users.

forthling as an embedded DSL in Rust programs (a safe configuration or rule language, the
niche of Lua) needs no step of its own: the host API is there, and step 4 makes it safe.

---

## Ongoing: documentation (section 4.1 of the standard) ✅ statement done (2026-10-31)

The statement is in `COMPLIANCE.md` and `VERKLARING.md` (see "Next session"). The original
plan:

A standard system must document its *implementation-defined* options and *ambiguous
conditions*. Keep a section in the README with, among other things:
- cell size (64 bits), address unit (8 bits after 0.2), character set;
- division: symmetric or floored;
- case insensitivity of names (allowed);
- what happens on stack underflow, an undefined word, an invalid address, etc. (THROW
  codes);
- the maximum length of a name and of counted strings, the size of `PAD` and of the
  pictured output buffer;
- extensions: `."` and `S"` also work in interpretation state, plus the number prefixes
  `$ # % 'c'`.

Finally the interpreter can officially call itself: *"Forth-2012 System, providing the
Core word set, the Core Extensions word set, the Exception word set, …"*.

---

## Proposed milestones

| # | Contents | Done when |
|---|----------|-----------|
| M1 ✅ | Phase 0.1 + 0.4 (test suite + THROW codes) | `prelimtest.fth` runs, errors show a code |
| M2 ✅ | Phase 0.2 + 0.3 + 0.5 (rebuilding memory and input) | all existing tests and the demo still work |
| M3 ✅ | Phase 1 | `core.fr` without errors and all CORE words present → **CORE compliant** (2026-10-30) |
| M4 ✅ | Phase 2 | `coreexttest.fth` without errors (2026-10-30) |
| M5 ✅ | EXCEPTION + TOOLS + STRING | their test files without errors (2026-10-30) |
| M6+ | other word sets as needed (FILE, DOUBLE and FACILITY ✅ 2026-10-30, MEMORY, LOCALS, SEARCH, BLOCK, XCHAR and FLOATING ✅ 2026-10-31) | likewise |

M2 is the biggest and riskiest step. It is best done in one go, with the current unit
tests as a safety net, before new words are built on top.
