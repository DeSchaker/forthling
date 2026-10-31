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
- `src/host.rs`: the public API for adding words from Rust (`Extension`, `add_word`);
  `tests/host.rs` uses it from the outside.
- `python/`: the Python module, a crate of its own on top of the host API, with its own
  `Cargo.lock`; the core has no dependencies and stays that way. Check it with
  `cargo fmt` and `cargo clippy` with `--manifest-path python/Cargo.toml`, and test it with
  `pip install ./python` and `python -m unittest discover -s python/tests`.
- `src/tests.rs`: the unit tests; `tests/suite.rs` runs the files of the test suite.
- `COMPLIANCE.md`: the system documentation that the standard asks for (section 4.1).

### Conventions

- THROW codes are used by their names from `src/throw.rs` (`STACK_UNDERFLOW`,
  `INVALID_ARGUMENT`, …), never as bare numbers. The tests do use the numbers, since those
  are what the standard fixes.
- A word that may only be used inside a definition is registered with `compile_imm` or
  `compile_prim`; the interpreter then throws -14 when it runs outside one.
- `pop_str()` pops and checks a string ( c-addr u ).
- Keep a change small and give it tests. If it changes how forthling behaves, update
  `COMPLIANCE.md` and the README where they describe that behaviour. `VERKLARING.md` is
  its Dutch translation; the maintainer keeps it in step, so you need not.
- `ROADMAP.md` is the roadmap and the status; it also records the design decisions.
- Commit messages start with a verb in the imperative, such as "Add the XCHAR word set".

## License

By contributing, you agree that your contribution is licensed under the
[MIT license](LICENSE) of this project.
