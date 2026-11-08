# Contributing to forthling

Thanks for your interest. Bug reports, questions and pull requests are all welcome. Everyone
taking part follows the [code of conduct](CODE_OF_CONDUCT.md).

## Reporting a bug or asking for a feature

Open an [issue](https://github.com/DeSchaker/forthling/issues/new/choose) and pick a
template. For a bug, the most useful thing is a few lines of Forth that show it, with what
forthling did and what you expected. If the bug is about the standard, name the word and,
if you can, the section of [Forth-2012](https://forth-standard.org/).

Security problems go through [SECURITY.md](SECURITY.md), not a public issue.

## Changing the code

forthling needs Rust 1.85 or newer, and 1.85 is also the oldest version it supports, so
please do not use features of newer Rust versions.

Before you open a pull request, run:

```
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

Clippy runs with all of `clippy::pedantic`. The setting lives in the `[lints.clippy]` table of
each `Cargo.toml`, so a local run and CI are the same, and a warning fails CI. The lints of
Rust 1.85 and of the latest stable differ, so run clippy on both (`cargo +1.85 clippy ...`).
Fix a warning in the code. Where the lint is wrong for that spot, add an
`#[allow(clippy::..., reason = "...")]` with the reason; the one allowed for the whole
project is `wildcard_imports` (`use crate::*;` is a convention here).

### Testing

`cargo test` runs the tests. The ones from the Forth-2012 test suite need the suite to be
cloned first, at a fixed commit:

```
git clone https://github.com/gerryjackson/forth2012-test-suite.git tests/forth2012-test-suite
git -C tests/forth2012-test-suite checkout 4c87e56
```

CI (`.github/workflows/ci.yml`) checks formatting and clippy lints and runs all tests, the
test suite included, on Rust 1.85 and on the latest stable Rust on Linux and Windows. It also
builds the Python module and runs its tests (`python -m unittest discover -s python/tests`,
after `pip install ./python`) and checks and tests the SQLite words
(`cargo test --manifest-path sqlite/Cargo.toml`), the regular expression words
(`cargo test --manifest-path regex/Cargo.toml`), the science words
(`cargo test --manifest-path science/Cargo.toml`), the small building blocks
(`cargo test --manifest-path extras/Cargo.toml`), the HTTP words
(`cargo test --manifest-path http/Cargo.toml`, against a small server on the machine) and the
MCP server (`cargo test --manifest-path mcp/Cargo.toml`, also with `--no-default-features`
and `--all-features`), and builds the WebAssembly module and tests it with Node
(`node --test web/tests/forthling.test.mjs`).

### Where things are

- `src/words/*.rs`: the primitives, one file per word set. Each file has an `install`
  function, called from `src/words/mod.rs`.
- `src/prelude.fs`: the words that are defined in Forth itself.
- `src/json.rs`: the JSON words, an extension that is part of the core (`forthling::Json`);
  `tests/json.rs` tests them.
- `src/host.rs`: the public API for adding words from Rust (`Extension`, `add_word`);
  `tests/host.rs` uses it from the outside.
- `python/`: the Python module, a crate of its own on top of the host API, with its own
  `Cargo.lock`; the core has no dependencies and stays that way. Check it with
  `cargo fmt` and `cargo clippy` with `--manifest-path python/Cargo.toml`, and test it with
  `pip install ./python` and `python -m unittest discover -s python/tests`.
- `sqlite/`: the SQLite words, likewise a crate of its own (rusqlite), with the program
  `forthling-sqlite`; check and test it with `--manifest-path sqlite/Cargo.toml`.
- `window/`: the native window for the canvas (`SHOW-WINDOW`), likewise a crate of its own
  (minifb), with the program `forthling-window`; check and test it with
  `--manifest-path window/Cargo.toml`. Not in the MCP server, the web module or the Python module.
- `regex/`: the regular expression words, likewise a crate of its own (regex-lite), with
  the program `forthling-regex`; check and test it with `--manifest-path regex/Cargo.toml`.
  The MCP server, the web module and the Python module install it.
- `science/`: the vector, matrix, linear algebra and special function words (no dependencies, built from the
  mathematics), likewise a crate of its own, with the program `forthling-science`; check and
  test it with `--manifest-path science/Cargo.toml`. The MCP server, the web module and the
  Python module install it.
- `extras/`: small building blocks (now the CSV words; encodings and hashes, dates, URLs and UUIDs planned),
  one word list each, no dependencies, likewise a crate of its own, with the program
  `forthling-extras`; check and test it with `--manifest-path extras/Cargo.toml`. It is for
  pure computation only: a topic that needs a dependency or a permission gets a crate of its own
  (as `sqlite/` and `http/` do). The MCP server, the web module and the Python module install it.
- `http/`: the HTTP client words, likewise a crate of its own (ureq), with the program
  `forthling-http`; check and test it with `--manifest-path http/Cargo.toml` (its tests run a
  small server on the machine). It reaches the network, so it needs `Permission::Network` and
  a list of hosts. The MCP server (feature `http`, off by default) and the Python module
  install it; the web module cannot.
- `mcp/`: the MCP server `forthling-mcp`, a crate of its own (serde_json); the protocol
  is in `src/lib.rs` (`Server::handle`, one message at a time), stdio in `src/main.rs`.
  Test it with `--manifest-path mcp/Cargo.toml`, also with `--no-default-features`. Keep
  its tools free of arguments that could change the sandbox.
- `web/`: WebAssembly and the playground (`www/`); plain exports in `src/lib.rs`, the
  JavaScript in `www/forthling.js`, tests in `tests/forthling.test.mjs` (Node 22). The
  core builds for `wasm32-unknown-unknown`: code that needs a thread, the clock or sleep
  has a `#[cfg(target_arch = "wasm32")]` alternative (see `sleep`, `now`, `deadline`).
- `benches/bench.rs`: the benchmarks of the interpreter, with no dependencies. `cargo bench`
  prints the shortest time of a few runs for a new system, the empty `DO LOOP`, `27 FIB`,
  many definitions and many lookups (`cargo bench -- fib` runs the cases with `fib` in the
  name). To check a change to the inner interpreter or the dictionary, run it in a checkout
  of the previous commit too (a `git worktree` does) and compare the two tables. Times
  depend on the machine, so compare runs on the same one. `benches/*.fs` are the same cases
  as Forth files that time themselves with the `BENCHMARK` words (`forthling benches/fib.fs`);
  `definitions.fs` uses 5000 lines, not 20000, to fit the default memory of 1 MiB.
- `src/cli.rs`: the forthling program (options, config file, REPL), in the library so
  that a program with extensions calls `forthling::cli::main(&[&Extension])`.
- `src/testing.rs`: the `TESTING` words (`T{ -> }T`) and `Forth::take_test_results`, which
  the MCP server and `forthling --test` use. Not the same as `tests/forth2012-test-suite`,
  which brings its own `T{` in the main word list.
- `src/tests.rs`: the unit tests; `tests/suite.rs` runs the files of the test suite.
- `COMPLIANCE.md`: the system documentation that the standard asks for (section 4.1).

### Conventions

- Keep the core small. The core never gets a dependency or `unsafe` code. So the rule is:
  - Something that needs a dependency, or `unsafe` code, gets a crate of its own, as
    `python/`, `sqlite/`, `regex/`, `science/`, `extras/`, `http/`, `window/` and `mcp/` do.
  - The compiler enforces the rule: the core, `mcp/`, `sqlite/`, `regex/`, `science/`, `extras/`, `http/`, `window/` and
    `python/` have `unsafe_code = "forbid"` in `[lints.rust]`. `web/` does not, since the
    exports of a WebAssembly module are `#[unsafe(no_mangle)]`.
  - Something that needs neither goes in the core, as the sandbox, the JSON words and the
    functional words (`src/functional.rs`) do.

- THROW codes are used by their names from `src/throw.rs` (`STACK_UNDERFLOW`,
  `INVALID_ARGUMENT`, …), never as bare numbers. The tests do use the numbers, since those
  are what the standard fixes.
- A word that may only be used inside a definition is registered with `compile_imm` or
  `compile_prim`; the interpreter then throws -14 when it runs outside one.
- `pop_str()` pops and checks a string ( c-addr u ).
- A length, index or address becomes a cell with `to_cell(usize)` (public as `forthling::to_cell`), and a cell becomes an
  index with `to_index(Cell)` (both in `src/memory.rs`), never with `as`. A value that does
  not fit saturates: `to_cell` at `Cell::MAX`, `to_index` at `usize::MAX`, also for a
  negative cell, so it stays outside memory and `check` refuses it. The other casts
  (`as u8`, `as u64`, `as f64`, ...) are bit-level on purpose, as the standard defines
  them; where one is needed, the module or statement has an `#[allow(clippy::cast_.., reason = "..")]`.
- Output goes through `emit_bytes`, `emit_memory` or `emit_str`, never straight into the
  output buffer, so the output limit of the sandbox holds. Code that reaches a file checks
  `Permission::Files` first (`need`, or `permits` for a word with an ior) and opens it
  through the file system (`self.vfs`, see `src/vfs.rs`), never with `std::fs`; an
  extension that opens files itself checks `disk_files()`. A word that handles much data at once calls `charge(bytes)` before the
  work, so the time limit holds. A word of the program is made with `new_word`, not
  `define`, and whatever else grows outside Forth memory is counted in `space_used` and
  checked with `check_space`. See `src/sandbox.rs`. An extension uses the public
  `charge`, `check_time` and `deadline` for work outside Forth, as the SQLite words do.
- Every word needs a line in the help text: `src/help.txt` for the core and its
  extensions, a `help.txt` in the crate for the word lists of `sqlite/`, `regex/`,
  `science/`, `extras/` and `http/` (added with `f.add_help(include_str!("help.txt"))` in `install`). A line is
  `NAME ( effect ) description ;; code => output`; the part from `;;` is an example that
  the tests run. `tests/help.rs` fails for a word without a line. Write the effect in the
  standard notation, `( x1 x2 -- x2 x1 )`, so a stack check can read it later.
- An example program is a file `name.fs` in `examples/` (or in `examples/` of a crate;
  a cookbook example is called `book<NNN>_example<MMM>_<theme>.fs`, as `book001_example002_sudoku.fs`)
  with its expected output in `name.out` beside it. `tests/examples.rs` runs every
  program there and fails for one without a `.out` file, so an example cannot go stale.
  The examples of `sqlite/`, `regex/`, `science/` and `extras/` are run the same way by the tests of those crates.
  `http/examples/` needs a server and is not run by a test. See [cookbook/](cookbook/).
- An example that is also in the playground is a copy in `web/www/index.html` (the object
  `examples`). The copy is the file without the lines about the command line (`Run: cargo run ...`
  and `Needs the core only ...`). Add it to `copies` in the last test of
  `web/tests/forthling.test.mjs`: that test checks the text against the file, and that the module
  gives the output in the `.out` file. Mandelbrot is not checked, since its picture is smaller there.
- Keep a change small and give it tests. If it changes how forthling behaves, update
  `COMPLIANCE.md` and the pages in `docs/` (and the README) where they describe that behaviour. `VERKLARING.md` is
  its Dutch translation; the maintainer keeps it in step, so you need not.
- The open work is in the [issues on GitHub](https://github.com/DeSchaker/forthling/issues).
  Open an issue before a larger change.
- Commit messages start with a verb in the imperative, such as "Add the XCHAR word set".

## Releasing

The version is the same for `forthling`, `forthling-sqlite`, `forthling-regex`, `forthling-science`,
`forthling-extras`, `forthling-http`, `forthling-window` and `forthling-mcp`, and in `mcp/server.json` (a test in `mcp/tests` checks the last two) and
`mcp/mcpb/manifest.json`. `python/` and `web/` have a version of their own (0.1.0): they
are not published through `release.yml` and are not raised with a release. To
release: raise the version in the seven `Cargo.toml`, the path dependencies, `server.json` and `manifest.json`, and push a
tag such as `v0.2.4`. `.github/workflows/release.yml` then publishes the crates to
crates.io and the server to the MCP Registry, both through GitHub OIDC, without secrets.
With every release, also change the `forthling = "0.x.y"` examples in `docs/install.md`.
They name the full version (`"0.2.1"`), so they are easy to miss.

## License

By contributing, you agree that your contribution is licensed under the
[MIT license](LICENSE) of this project.
