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
- `IF ELSE THEN`, `BEGIN UNTIL/AGAIN/WHILE REPEAT`, `DO ?DO LOOP +LOOP LEAVE`
- `VARIABLE`, `CONSTANT`, `CREATE … DOES>`
- Byte-addressed memory with 64-bit cells
- `SOURCE`, `>IN`, `STATE`, `WORD` and `FIND`, working on input buffers in Forth memory
- Errors as standard THROW codes, e.g. `demo.fs:3: error -13: undefined word: FOO`

Part of the dictionary is written in Forth itself (`PRELUDE` in `src/main.rs`).

## Standard compliance

[`STANDAARD.md`](STANDAARD.md) (in Dutch) holds the roadmap to Forth-2012 and the
current status. Phase 0, the groundwork, is done. Of the CORE word set, 105 of the
133 words are present.

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
