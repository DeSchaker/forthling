# Small building blocks for forthling: CSV, encoding, date, URL and UUID words

The crate `forthling-extras` for [forthling](https://github.com/DeSchaker/forthling) holds
small, boring building blocks that programs need when they talk to the world. Each is a word
list of its own. The crate is small, has no dependencies and only computes on text in the
Forth memory, so it needs no permission: the words work in the sandbox, in the MCP server, in
the Python module and in the playground, where they are built in.

Now: the word lists `CSV`, `ENCODING`, `DATE`, `URL` and `UUID`.

## Running

`forthling-extras` is forthling with these words installed:

```
cargo run --release --manifest-path extras/Cargo.toml -- extras/examples/csv.fs
```

In a Rust program: `forth.install(&forthling_extras::Extras)`, or
`forthling::cli::main(&[&forthling_extras::Extras])` for the whole program.

`S" FORTHLING-EXTRAS" ENVIRONMENT?` gives `1 true` when the words are there. A program
starts with `ALSO CSV`, `ALSO ENCODING`, `ALSO DATE`, `ALSO URL` or `ALSO UUID`.

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

## ENCODING

Base64 and base64url (RFC 4648), hex, SHA-256 (FIPS 180-4) and CRC-32 (the polynomial of zip and
PNG), written by hand from the specifications. They take text in memory, like the CSV words. The
words that make a text give a new one, `ALLOCATE`d, for the program to `FREE`; text can hold any
bytes.

| Word | Effect | |
|---|---|---|
| `>BASE64` | `( c-addr u -- c-addr' u' )` | The text as base64, padded with `=`. |
| `BASE64>` | `( c-addr u -- c-addr' u' )` | The bytes of base64 text. The padding must be right. |
| `>BASE64URL` | `( c-addr u -- c-addr' u' )` | As base64url (`-` and `_` for `+` and `/`), without padding, as in a JSON Web Token. |
| `BASE64URL>` | `( c-addr u -- c-addr' u' )` | The bytes of base64url text, with or without padding. |
| `>HEX` | `( c-addr u -- c-addr' u' )` | Two lower-case hex digits for each byte. |
| `HEX>` | `( c-addr u -- c-addr' u' )` | The bytes of hex text, in upper or lower case. |
| `SHA-256` | `( c-addr u -- c-addr' u' )` | The digest: 32 bytes. `>HEX` shows them. |
| `CRC-32` | `( c-addr u -- u )` | The checksum, a number from 0 to 4294967295. |

```
ALSO ENCODING
S" abc" SHA-256 >HEX 2DUP TYPE DROP FREE DROP
```

prints `ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad`. The full example,
with the error, is [examples/encoding.fs](examples/encoding.fs) (its output is in `encoding.out`).

The decoders are strict, so that one text has one meaning: no spaces or line ends, no character
outside the alphabet, the last character with no stray bits, and for `BASE64>` the right number
of `=`. Anything else throws -265, as does hex of odd length or with a character that is not a
digit. The work counts against the step limit by the number of bytes, like the other extensions.
The checks use the test vectors of the RFCs and of FIPS 180-4, and the CRC-32 of `123456789`.

## DATE

Dates and times as a number of **seconds since 1970-01-01 00:00:00 UTC** (a signed number; a time
before 1970 is negative), in the Gregorian calendar, for the years 0000 to 9999: the four digits
of ISO 8601. The words only compute: they never read a clock. `TIME&DATE DATE>SECONDS` is the
time now (the host sets the clock, so it works in the playground too).

| Word | Effect | |
|---|---|---|
| `>ISO8601` | `( seconds -- c-addr u )` | The time as text in UTC: `2001-09-09T01:46:40Z`. `ALLOCATE`d. |
| `ISO8601>` | `( c-addr u -- seconds )` | Reads `YYYY-MM-DD` (midnight) or `YYYY-MM-DDTHH:MM:SS` with `Z`, an offset such as `+02:00`, or nothing (UTC). |
| `SECONDS>DATE` | `( seconds -- sec min hour day month year )` | The parts in UTC, in the order of `TIME&DATE`. |
| `DATE>SECONDS` | `( sec min hour day month year -- seconds )` | The time of the parts; each part is checked. |
| `DAY-OF-WEEK` | `( seconds -- n )` | 1 is Monday, 7 is Sunday, as in ISO 8601. |
| `+DAYS` | `( seconds n -- seconds' )` | Add n days of 86400 seconds; n may be negative. |
| `+MONTHS` | `( seconds n -- seconds' )` | Add n months; a day the new month lacks becomes its last day (31 January plus one month is 28 or 29 February). |
| `LEAP-YEAR?` | `( year -- flag )` | |
| `DAYS-IN-MONTH` | `( year month -- n )` | |

```
ALSO DATE
S" 2026-11-08T21:30:00+02:00" ISO8601>  30 +DAYS >ISO8601 2DUP TYPE DROP FREE THROW
```

prints `2026-12-08T19:30:00Z`: the offset is applied when reading and the answer is in UTC. The
full example, which also prints a calendar of a month, is [examples/date.fs](examples/date.fs)
(its output is in `date.out`).

UTC only, on purpose: a time zone with daylight saving needs a table that changes by law, so a
word list without data cannot know it. Convert at the edge of the program with an offset. Not
supported: leap seconds (a minute has 60 seconds, as in POSIX time), fractions of a second, week
numbers, and the forms `YYYYMMDD` or `YYYY-DDD`. Anything that is not exactly the form above
throws -265: a date that does not exist (30 February), a number in the wrong place, a lower-case
`t` or `z`, and a time outside the years 0 to 9999. Reading counts against the step limit by the
number of bytes, like the other extensions.

## URL

Percent-encoding (RFC 3986) and cutting a URL into its parts. The words only cut and convert
text: they do not fetch anything and need no permission. They read the host, port and query
as the `HTTP` words (`http/`) do: the host of an IPv6 address keeps its brackets, and a URL
without a port has none. One difference: `HTTP` sees the path of `http://example.com` as `/`,
while `URL-PATH` gives the empty text, since it cuts what is written.

| Word | Effect | |
|---|---|---|
| `>PERCENT` | `( c-addr u -- c-addr' u' )` | Every byte except `A-Z a-z 0-9 - . _ ~` becomes `%XX` (upper-case hex); a space is `%20`. `ALLOCATE`d. |
| `PERCENT>` | `( c-addr u -- c-addr' u' )` | `%XX` becomes a byte, in upper or lower case; a `+` stays a plus. `ALLOCATE`d. |
| `URL-SCHEME` | `( c-addr u -- c-addr' u' )` | `https` of `https://example.com/`. |
| `URL-USERINFO` | `( c-addr u -- c-addr' u' )` | What stands before the `@` of the authority. |
| `URL-HOST` | `( c-addr u -- c-addr' u' )` | `example.com`; `[::1]` for IPv6. |
| `URL-PORT` | `( c-addr u -- n )` | The port, or -1 when there is none. |
| `URL-PATH` | `( c-addr u -- c-addr' u' )` | `/a/b` of `https://example.com/a/b?x=1`. |
| `URL-QUERY` | `( c-addr u -- c-addr' u' )` | What stands after the `?`, without it. |
| `URL-FRAGMENT` | `( c-addr u -- c-addr' u' )` | What stands after the `#`, without it. |
| `URL-PARAM` | `( query-addr query-u name-addr name-u -- value-addr value-u true \| false )` | The value of the first parameter with that name in a query such as `a=1&b=2`; a name without `=` has an empty value. |

The words that cut a URL give a place in the text they were given, with no copy and no
allocation. A part that is not there is empty (length 0), so `?` with nothing after it and no
`?` at all look the same. Relative references work too (`/a/b?c#d` has a path, a query and a
fragment and nothing else). Only the form is checked: a port must be a number from 0 to 65535 and
a host that starts with `[` must have its `]`; both throw -265. The characters of a part are not
checked, and a part is not decoded: `URL-PARAM` finds a name as it is written, and the value
goes through `PERCENT>` when it is needed.

```
ALSO URL
S" https://example.com:8443/search?q=forth%20in%20rust" 2DUP URL-QUERY S" q" URL-PARAM DROP PERCENT> TYPE
```

prints `forth in rust`. The full example is [examples/url.fs](examples/url.fs) (its output is in
`url.out`).

Not supported: the form encoding of HTML (a `+` for a space), which `PERCENT>` leaves alone on
purpose; building a URL from parts (join the pieces with `>PERCENT` on each value); resolving a
relative reference against a base; punycode for non-ASCII host names. Reading counts against the step limit by
the number of bytes, like the other extensions.

## UUID

A random UUID of version 4 (RFC 9562).

| Word | Stack | Does |
|------|-------|------|
| `UUID4` | `( -- c-addr u )` | A UUID as 36 lower-case characters, such as `bdd73226-2feb-4e95-a8ef-e333b266f103`. The text is ALLOCATEd. |

The numbers come from the generator behind `RANDOM` and `RND`, which starts from the clock.
`SEED!` makes it repeatable, so `42 SEED!` followed by `UUID4` gives the same UUID every time
(the example [examples/uuid.fs](examples/uuid.fs) does so, with its output in `uuid.out`). That
generator is not a secure one: use these UUIDs as identifiers, not as secrets or tokens.
