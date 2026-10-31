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

## Tools

| Tool | Arguments | |
|---|---|---|
| `forth_eval` | `code`, `data` | runs Forth code; returns the output, the data and float stacks, and the error if there was one |
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
`{"output": "2401 ", "stack": [1, 2], "fstack": [], "compiling": false, "error": null}`;
for an error, `"error": {"code": -13, "message": "undefined word", "detail": "FOO", …}`
and `isError` is true, so the model sees it and can put it right.

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
