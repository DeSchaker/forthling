# forthling in the browser

forthling compiled to WebAssembly, with a playground page: Forth-2012 in the browser,
without installing anything. It runs on the visitor's own computer, in forthling's
sandbox; the server only serves static files.

Try it at **https://deschaker.github.io/forthling/**. The workflow
`.github/workflows/pages.yml` builds and publishes it on every push to `main` that
changes forthling.

## Building

With Rust and its WebAssembly target (`rustup target add wasm32-unknown-unknown`):

```
cargo build --release --manifest-path web/Cargo.toml --target wasm32-unknown-unknown
cp web/target/wasm32-unknown-unknown/release/forthling_web.wasm web/www/forthling.wasm
```

`web/www/` then holds the whole site: `index.html` (the playground), `worker.js`,
`forthling.js` and `forthling.wasm` (some 450 KB). Serve it with any web server, for
example `python -m http.server` in `web/www`, and open http://localhost:8000/. Opening
`index.html` as a file does not work: the browser loads the module and the worker only
over http(s). The server must give `.wasm` the type `application/wasm`; most do.

There is no JavaScript build step and no dependency: the module exports a few plain
functions, and `forthling.js` wraps them.

## The playground

`index.html` has a transcript, the stack, an editor (Ctrl+Enter runs), examples, the
words defined so far and "Start over". forthling runs in a Web Worker, so the page never
freezes; should a run still not end, the page stops the worker after 15 seconds and
starts a new one.

## Using it from JavaScript

```js
import { load } from "./forthling.js";
const forth = await load("forthling.wasm");
forth.eval(": SQUARE DUP * ; 7 SQUARE .");
// { output: "49 ", stack: [], fstack: [], compiling: false, error: null, steps: 73, max_depth: 5 }
forth.words();   // ["SQUARE"]
forth.reset();
```

An error comes back as `error: { code, message, detail, hint, text, stack, fstack, words,
source, line, column, token }`: besides what went wrong, the stacks as they were at the
error (which then empties them), where in the code it happened, and the words that were
running, such as `["MAIN", "STEP", "/"]`. `hint` says how to put it right, such as the
`ALSO DEBUG` that `TRACE-ON` needs, or is null. `steps` and `max_depth` tell what the run took. One module is one
session; the words stay until `reset()`.

`forth.eval(code, { trace: n })` also records the first `n` words that ran: `trace` is a
list of `{ word, depth, stack, stack_depth }` (the stack when the word started, at most
its top 8 cells) and `trace_dropped` counts the rest. The playground's **Step through**
plays such a record back, word by word, forwards and back, with the stack before and after
each word and jumps to each `BREAK`; WebAssembly cannot wait in the middle of a run, but
the sandbox makes a run come out the same each time, so the record is what happened.

The session's files are in memory. `INCLUDE`, the FILE words and the block words use
them, and the page can too:

```js
forth.writeFile("square.fs", ": SQUARE DUP * ;");
forth.eval("INCLUDE square.fs 7 SQUARE .");   // output: "49 "
forth.readFile("square.fs");                    // ": SQUARE DUP * ;", or null for no such file
forth.files();                                  // [{ name: "square.fs", size: 16 }]
```

`writeFile` throws when the files would pass 4 MiB in all; `reset()` starts with no
files.

## In WebAssembly

- The sandbox: files only in memory (at most 4 MiB, gone at `reset()`), no terminal input, at most 10 million steps and 1 MiB of
  output per run and 4 MiB of definitions and the like in all. WebAssembly has no clock
  for a time limit, so the steps are the limit there.
- `MS` does nothing (WebAssembly cannot sleep); `TIME&DATE` has the time of the page
  (`Forth::set_clock`).
- The memory is 1 MiB, as natively; the most it can be is 1 GiB (`MAX_MEMORY` on a
  32-bit target).
- The JSON words are installed (`ALSO JSON`), and the debug words (`ALSO DEBUG`,
  `TRACE-ON`, `TRACE-OFF`, `BREAK`, which does nothing there but shows in a record);
  SQLite is not.

## Testing

```
node --test web/tests/forthling.test.mjs
```

after building; the tests load the module from `web/target` with Node 22.
