# Regular expressions for forthling

The word list `REGEX` for [forthling](https://github.com/DeSchaker/forthling), through
[regex-lite](https://github.com/rust-lang/regex/tree/master/regex-lite). It matches in
time linear in the text, so a pattern such as `(a+)+$` cannot make a program hang, and it
reaches nothing outside Forth: no permission is needed, so the words work in the sandbox,
in the MCP server, in the Python module and in the playground, where they are built in.

Texts and patterns are strings `( c-addr u )`, and must be UTF-8. The text comes first,
then the pattern. The syntax is Perl-like (`\d`, `\s`, `[a-z]+`, `(…)`, `(?i)`, `^`, `$`);
regex-lite has no Unicode classes such as `\p{L}`. Strings that a word returns are
`ALLOCATE`d, for the program to `FREE`; positions are byte offsets into the text.

## Running

`forthling-regex` is forthling with these words installed:

```
cargo run --release --manifest-path regex/Cargo.toml -- regex/examples/dates.fs
```

In a Rust program: `forth.install(&forthling_regex::Regex)`, or
`forthling::cli::main(&[&forthling_regex::Regex])` for the whole program.

`S" FORTHLING-REGEX" ENVIRONMENT?` gives `1 true` when the words are there. They are in
the word list `REGEX`, so a program starts with `ALSO REGEX`.

## Words

| Word | Stack | |
|---|---|---|
| `RE-MATCH?` | `( c-addr u re-addr re-u -- flag )` | is there a match anywhere? Anchor with `^` and `$` to match all of the text |
| `RE-FIND` | `( c-addr u re-addr re-u -- start len true \| false )` | the first match, as an offset in the text and a length |
| `RE-GROUP` | `( c-addr u re-addr re-u n -- start len true \| false )` | group n of the first match (0 is all of it); false when there is no match or the group took no part |
| `RE-COUNT` | `( c-addr u re-addr re-u -- n )` | the matches that do not overlap |
| `RE-REPLACE` | `( c-addr u re-addr re-u rep-addr rep-u -- c-addr' u' )` | replaces every match; `$1` is group 1, `${name}` a named group, `$$` a dollar; the new text is `ALLOCATE`d |

A word throws -262 for a pattern that does not compile (the message says why), a pattern
larger than 1 MiB when compiled or nested deeper than 100 groups, a text or pattern that
is not UTF-8, a group the pattern does not have, and a replacement larger than the Forth
memory. The compiled patterns of the latest uses are kept, so using one again costs no
compiling. The text is charged as work against the limits of the sandbox (one step per
64 bytes), as the JSON words do.
