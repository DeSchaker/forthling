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

The suite tests need the Forth-2012 test suite at a fixed commit; see
[Testing](README.md#testing) in the README. CI runs the same checks on Rust 1.85 and on the
latest stable Rust, on Linux and Windows.

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
- `regex/`: the regular expression words, likewise a crate of its own (regex-lite), with
  the program `forthling-regex`; check and test it with `--manifest-path regex/Cargo.toml`.
  The MCP server, the web module and the Python module install it.
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
- `src/cli.rs`: the forthling program (options, config file, REPL), in the library so
  that a program with extensions calls `forthling::cli::main(&[&Extension])`.
- `src/tests.rs`: the unit tests; `tests/suite.rs` runs the files of the test suite.
- `COMPLIANCE.md`: the system documentation that the standard asks for (section 4.1).

### Conventions

- Keep the core small. The core never gets a dependency or `unsafe` code. So the rule is:
  - Something that needs a dependency, or `unsafe` code, gets a crate of its own, as
    `python/`, `sqlite/`, `regex/`, `http/` and `mcp/` do.
  - Something that needs neither goes in the core, as the sandbox, the JSON words and the
    functional words (`src/functional.rs`) do.

- THROW codes are used by their names from `src/throw.rs` (`STACK_UNDERFLOW`,
  `INVALID_ARGUMENT`, …), never as bare numbers. The tests do use the numbers, since those
  are what the standard fixes.
- A word that may only be used inside a definition is registered with `compile_imm` or
  `compile_prim`; the interpreter then throws -14 when it runs outside one.
- `pop_str()` pops and checks a string ( c-addr u ).
- A length, index or address becomes a cell with `to_cell(usize)`, and a cell becomes an
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
- Keep a change small and give it tests. If it changes how forthling behaves, update
  `COMPLIANCE.md` and the README where they describe that behaviour. `VERKLARING.md` is
  its Dutch translation; the maintainer keeps it in step, so you need not.
- The open work is in the [issues on GitHub](https://github.com/DeSchaker/forthling/issues).
  Open an issue before a larger change.
- Commit messages start with a verb in the imperative, such as "Add the XCHAR word set".

## Releasing

The version is the same for `forthling`, `forthling-sqlite`, `forthling-regex`, `forthling-http` and
`forthling-mcp`, and in `mcp/server.json` (a test in `mcp/tests` checks the last two) and
`mcp/mcpb/manifest.json`. To
release: raise the version in the five `Cargo.toml`, the path dependencies, `server.json` and `manifest.json`, and push a
tag such as `v0.2.0`. `.github/workflows/release.yml` then publishes the crates to
crates.io and the server to the MCP Registry, both through GitHub OIDC, without secrets.
With a new minor version (0.x), also change the `forthling = "0.x"` examples in `README.md`:
`"0.1"` does not match 0.2.0.

## License

By contributing, you agree that your contribution is licensed under the
[MIT license](LICENSE) of this project.
