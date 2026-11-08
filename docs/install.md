# Install and options

How to install forthling, and the options of the `forthling` program. Back to the [README](../README.md).

## What to install

forthling comes in parts, called crates in Rust: the core (`forthling`) and extensions
such as `forthling-sqlite` and `forthling-mcp`. Which ones you need depends on how you use
it.

- **To use forthling,** install one program; its extensions are built in.
  - In the browser: nothing to install, open the
    [playground](https://deschaker.github.io/forthling/).
  - `cargo install forthling`: the program, the core alone, without SQLite.
  - `cargo install forthling-sqlite`: the same program with the SQLite words.
  - `cargo install forthling-mcp`, or the bundle `forthling.mcpb` (see
    [For AI agents](hosts.md#for-ai-agents-mcp)): the MCP server, with the core and SQLite.
- **To embed forthling in a Rust program,** name the core and the extensions you want in
  your `Cargo.toml`. An extension brings the core along, and Cargo builds it once, shared
  by all extensions, so they can work in the same system. An extension you do not name,
  such as SQLite with its C library, stays out of your program.

  ```toml
  [dependencies]
  forthling = "0.2.3"
  forthling-sqlite = "0.2.3"
  ```

  ```rust
  let mut forth = forthling::Forth::new();
  forth.install(&forthling_sqlite::Sqlite);
  ```

## Options

```
forthling [options] [file ...]
  -m, --memory SIZE  memory for data space and heap, such as 4M (default 1M)
  -b, --blocks FILE  the block file (default blocks.fb in the current directory)
      --test         run the files as tests (see [Tests](tools.md#tests)): print how many
                     T{ -> }T tests passed and failed, and exit with 1 if one
                     failed or an error stopped the run
      --sandbox      files only in memory (the files given here and the block
                     file are copied in; what Forth writes is gone at the end),
                     and the limits below: 10000000 steps, 5s and 1M of output
                     per line of the REPL or per file, and 4M of space for
                     definitions and the like in all
      --max-steps N  at most N steps (instructions and words) per evaluation
      --max-time T   at most T per evaluation, such as 5s or 500ms
      --max-output SIZE  at most SIZE bytes of output per evaluation
      --max-space SIZE   at most SIZE bytes outside Forth memory for definitions,
                     code, word lists and the like
  -h, --help         show this help
```

A limit of `none` lifts it, also one of `--sandbox`. See [Sandbox](embedding.md#sandbox).

`SIZE` is a number of bytes with an optional `K`, `M` or `G` (binary units), from 64K up
to 4G. A relative block file name is relative to the current directory.

To change the defaults, put lines like these in the config file
`forthling/forthling.conf` in your config directory: `%APPDATA%` on Windows, otherwise
`$XDG_CONFIG_HOME` or `~/.config` (`--help` shows the exact path):

```
# forthling settings
memory = 16M
blocks = C:/Users/me/forth/blocks.fb
```

Lines starting with `#` are comments. The command line wins over the config file.

A program that embeds forthling picks the size with `Forth::with_memory(size)` and the
block file with `Forth::set_block_file(path)`.
