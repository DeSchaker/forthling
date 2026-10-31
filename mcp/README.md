# forthling-mcp

An [MCP](https://modelcontextprotocol.io/) server that lets AI agents (Claude Code,
Claude Desktop and other MCP clients) run Forth in forthling, always in its sandbox. The
agent keeps one session: the words it defines and the stacks stay between calls.

## Installing

```
cargo install --path mcp
```

or build it with `cargo build --release --manifest-path mcp/Cargo.toml`; the program is
then `mcp/target/release/forthling-mcp`. With Claude Code:

```
claude mcp add forthling -- forthling-mcp
```

Other clients start it the same way: the command `forthling-mcp`, over stdio.

When a client connects, the server tells the model the basics in its instructions: the
idioms that trip models up, locals rather than `ROT`, `PICK` and `ROLL`, the JSON words and
the limits. [`llms.txt`](../llms.txt) says the same at more length, for a model that reads
it beforehand; a test keeps the limits in both the same.

## Tools

| Tool | Arguments | |
|---|---|---|
| `forth_eval` | `code`, `data`, `trace` | runs Forth code; returns the output, the data and float stacks, and the error if there was one |
| `forth_stack` | | the stacks, without running anything |
| `forth_words` | | the words defined in this session, the latest first |
| `forth_reset` | | a new session: every definition, stack value and the data are gone |

A line of code, and so a string in `S"`, is at most 1024 characters. Larger input, such
as a JSON document, goes in `data`: the code reads it with `DATA ( -- c-addr u )`, which
gives a copy in `ALLOCATE`d memory (`FREE` it). The data stays for later calls until a
call sends other data, and may be at most as large as the Forth memory:

```json
{"name": "forth_eval", "arguments": {"code": "ALSO JSON DATA JSON-LENGTH .", "data": "[[59],[73,41]]"}}
```

Each result is text the way the REPL shows it:

```
2401  ok
stack: 1 2
```

and the same as structured JSON (`structuredContent`, with an `outputSchema`):
`{"output": "2401 ", "stack": [1, 2], "fstack": [], "compiling": false, "error": null,
"steps": …, "max_depth": …}`; `steps` and `max_depth` (only for `forth_eval`) tell how
many steps the call took and how deep the data stack went, counting the words of the
system it ran (`.` is written in Forth, so it counts too).

An error empties the stacks, so the result also tells what they held at the error, where
it happened and which words were running; the model can put it right in one go:

```
error -10: division by zero
at input, line 2, column 3: MAIN
in: MAIN > STEP > /
stack at the error: 5 1
stack: empty
```

and as JSON: `"error": {"code": -10, "message": "division by zero", "stack": [5, 1],
"fstack": [], "words": ["MAIN", "STEP", "/"], "source": "input", "line": 2, "column": 3,
"token": "MAIN", …}`, with `isError` true. A word that took its arguments before it
failed, such as `/` here, has taken them.

With `"trace": true`, `forth_eval` also returns a trace: each word that ran, with how deep
it was nested and the data stack when it started (at most its top 8 cells), at most 1000
words; `trace_dropped` counts the rest. The insides of the system's own words are left
out. The text starts with it, indented the way `TRACE-ON` prints it:

```
trace:
F <0>
  SQ <1> 3
    DUP <1> 3
    * <2> 3 3
  1+ <1> 9
. <1> 10
end of trace
10  ok
stack: empty
```

and the JSON has `"trace": [{"word": "F", "depth": 0, "stack": [], "stack_depth": 0},
{"word": "SQ", "depth": 1, "stack": [3], "stack_depth": 1}, …], "trace_dropped": 0`. The
trace of a call that fails shows how it got to the error. In the code itself,
`ALSO DEBUG TRACE-ON … TRACE-OFF` prints the same lines as output.

The JSON words are installed (`ALSO JSON`), and the SQLite words for databases in memory
(`ALSO SQLITE`, only `:memory:`, which SQL stops when the time is up and which cannot
grow larger than the Forth memory); without the Cargo feature `sqlite`
(`--no-default-features`) the server is built without SQLite.

## The sandbox

Every session runs in forthling's sandbox: no file access, and each call at most
10 million steps, 5 seconds and 1 MiB of output; the session's definitions and the like at
most 4 MiB in all. The options change these limits, each up to a ceiling the program
fixes:

```
  -m, --memory SIZE      Forth memory (default 1M) [64M]
      --max-steps N      steps per call (default 10000000) [1000000000]
      --max-time T       time per call, such as 5s or 500ms (default 5s) [60s]
      --max-output SIZE  output per call (default 1M) [16M]
      --max-space SIZE   definitions and the like in all (default 4M) [64M]
```

There is no option to switch the sandbox off or to permit files, so changing how the
server is started does not open them either.

**What enforces it.** The sandbox is enforced by the server, not by what the model is
told or promises: the tools take only Forth code (other arguments are refused), and no
Forth word can change a limit or a permission. A user who asks the AI to get around the
sandbox cannot do it through forthling. An agent that also has other tools, such as a
shell or editing files, can however go around it: change the MCP configuration, or start
another program. Against that, forthling cannot help; use the permissions of the agent
instead. In Claude Code: deny or limit the shell, deny edits to the MCP configuration
(`.mcp.json`, `~/.claude.json`), and allow only the MCP servers you want through managed
settings. For extra safety against a bug in forthling, run the server in a container or
as a user with few rights.

## Testing

```
cargo test --manifest-path mcp/Cargo.toml
cargo test --manifest-path mcp/Cargo.toml --no-default-features
```
