# Small building blocks for forthling: CSV words

The crate `forthling-extras` for [forthling](https://github.com/DeSchaker/forthling) holds
small, boring building blocks that programs need when they talk to the world. Each is a word
list of its own. The crate is small, has no dependencies and only computes on text in the
Forth memory, so it needs no permission: the words work in the sandbox, in the MCP server, in
the Python module and in the playground, where they are built in.

Now: the word list `CSV`. Encodings and hashes, dates, URLs and UUIDs are planned (issues
#99 to #102 in the repository).

## Running

`forthling-extras` is forthling with these words installed:

```
cargo run --release --manifest-path extras/Cargo.toml -- extras/examples/csv.fs
```

In a Rust program: `forth.install(&forthling_extras::Extras)`, or
`forthling::cli::main(&[&forthling_extras::Extras])` for the whole program.

`S" FORTHLING-EXTRAS" ENVIRONMENT?` gives `1 true` when the words are there. A program
starts with `ALSO CSV`.

## CSV

Comma-separated values as RFC 4180 describes them: a field may be in double quotes, and then
it may hold the separator, a line end and, doubled, a quote. A record ends at `LF`, `CR LF` or
`CR`. The words read one field at a time from text in memory, in the style of the JSON words;
nothing is allocated except where the table says so.

| Word | Effect | |
|---|---|---|
| `CSV-FIELD` | `( c-addr u -- rest-addr rest-u field-addr field-u last? )` | The first field, without its quotes, and what is left. `last?` is true when the field ended its record. An empty text gives an empty field that ends the record. |
| `CSV-SKIP` | `( c-addr u -- rest-addr rest-u )` | Skips one record, for a header line. |
| `CSV>STRING` | `( c-addr u -- c-addr' u' )` | A field with its doubled quotes made single. ALLOCATEd. |
| `CSV-QUOTE` | `( c-addr u -- c-addr' u' )` | A field as it is written: in quotes, with quotes doubled, when it holds the separator, a quote or a line end. ALLOCATEd, also when nothing changed. |
| `CSV-SEPARATOR` | `( char -- )` | The separator; a comma to begin with. `9` gives tabs, `59` semicolons. |

A field after a separator at the end of a record is an empty field (`a,` has two fields), so
a loop over a text asks first whether text is left, and then reads fields until `last?`:

```
ALSO CSV
: SHOW-RECORD ( c-addr u -- c-addr' u' )
  BEGIN  CSV-FIELD >R  CSV>STRING 2DUP TYPE DROP FREE THROW  R> 0= WHILE  ."  | "  REPEAT  CR ;
: SHOW-ALL ( c-addr u -- )  BEGIN  DUP WHILE  SHOW-RECORD  REPEAT  2DROP ;
S\" a,\"b,c\",\"say \"\"hi\"\"\"\n1,,3\n" SHOW-ALL
```

prints

```
a | b,c | say "hi"
1 |  | 3
```

The full example, with writing fields, another separator and the error, is
[examples/csv.fs](examples/csv.fs) (its output is in `csv.out`).

Errors are all -265: a quote that is never closed, text between a closing quote and the next
separator (`"a"b`), and a separator that cannot be one (a quote or a line end). A quote in the
middle of an unquoted field is kept as it is. The work counts against the step limit by the
number of bytes, like the other extensions.
