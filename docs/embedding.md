# Embedding in Rust

forthling is a library crate: another Rust program can run Forth, add words of its own, and limit what Forth code may do. Back to the [README](../README.md).

## The library

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
(see [Extensions](../COMPLIANCE.md#extensions)):

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

## Sandbox

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
opens only `:memory:` unless the program may use the disk (`Forth::disk_files()`). `Forth::sandboxed()` also denies `Permission::Network`, which the HTTP words need (see [HTTP](hosts.md#http)), and `Permission::System`, which the words of `OPERATING-SYSTEM` need (see [Operating system](wordlists.md#operating-system)); `--sandbox` of the program does the same.

## Files

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

## Error reports

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
