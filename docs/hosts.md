# Where forthling runs

forthling runs as a program, in the browser, behind an MCP server, in Python and in Rust. Each extension lives in a crate of its own. Back to the [README](../README.md).

## In the browser

`web/` compiles forthling to WebAssembly (about 490 KB, no dependencies, no wasm-bindgen)
and has a playground page: Forth in the browser, on the visitor's own computer, in the
sandbox. Try it at **https://deschaker.github.io/forthling/**; see
[web/README.md](../web/README.md).

## For AI agents (MCP)

`mcp/` holds `forthling-mcp`, an [MCP](https://modelcontextprotocol.io/) server: AI agents
run Forth through it, in a session that keeps their definitions and files, always in the
sandbox, with files only in memory. Install it with `cargo install forthling-mcp`, or
without Rust as the bundle `forthling.mcpb` from the
[latest release](https://github.com/DeSchaker/forthling/releases/latest), which Claude
Desktop opens; with Claude Code: `claude mcp add forthling -- forthling-mcp`. It is in the
[MCP Registry](https://registry.modelcontextprotocol.io) as `io.github.DeSchaker/forthling`. Its
tools are `forth_eval`, `forth_stack`, `forth_words`, `forth_help` and `forth_reset`; each result has
the output and the stacks as text and as JSON. See [mcp/README.md](../mcp/README.md), also
for what enforces the sandbox when a user asks the AI to get around it.
[`llms.txt`](../llms.txt) (also at https://deschaker.github.io/forthling/llms.txt) sums up
for a language model how to write Forth for forthling: the idioms, locals rather than
`ROT` and `PICK`, the JSON words and the limits.

## From Python

`python/` holds a Python module, built with [PyO3](https://pyo3.rs/) and
[maturin](https://www.maturin.rs/); see [python/README.md](../python/README.md).

```python
import math, forthling

f = forthling.Forth()
f.add_word("HYPOT", math.hypot, inputs=2)   # a word written in Python
print(f.interpret("3 4 HYPOT F."))         # 5.
```

## SQLite

`sqlite/` adds the word list `SQLITE`, in the style of the FILE words: handles as cells,
errors as an `ior`. Its program `forthling-sqlite` is forthling with these words; see
[sqlite/README.md](../sqlite/README.md) and [the example](../sqlite/examples/people.fs).

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

## Regular expressions

`regex/` adds the word list `REGEX`: `RE-MATCH?`, `RE-FIND`, `RE-GROUP`, `RE-COUNT` and
`RE-REPLACE`. Matching takes time linear in the text, so no pattern can hang a program, and
the words reach nothing outside Forth, so they need no permission: they are in the MCP
server, the Python module and the playground, and `forthling-regex` is the program with
them. See [regex/README.md](../regex/README.md) and [the example](../regex/examples/dates.fs).

```
ALSO REGEX
: SWAP-DATE  S" 2026-11-01" S" (\d+)-(\d+)-(\d+)" S" $3/$2/$1" RE-REPLACE ;
SWAP-DATE TYPE
```

## HTTP

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
