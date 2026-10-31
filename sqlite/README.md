# SQLite for forthling

The word list `SQLITE` for [forthling](https://github.com/DeSchaker/forthling), through
[rusqlite](https://github.com/rusqlite/rusqlite), which builds SQLite itself, so nothing
needs to be installed. The words follow the FILE word set of Forth-2012: databases and
statements are handles (cells), errors are an `ior` (0 when it went well), and the
strings they return are `ALLOCATE`d, for the program to `FREE`.

## Running

`forthling-sqlite` is forthling with these words installed:

```
cargo run --release --manifest-path sqlite/Cargo.toml -- sqlite/examples/people.fs
```

In a Rust program: `forth.install(&forthling_sqlite::Sqlite)`, or
`forthling::cli::main(&[&forthling_sqlite::Sqlite])` for the whole program.

`S" FORTHLING-SQLITE" ENVIRONMENT?` gives `1 true` when the words are there. They are in
the word list `SQLITE`, so a program starts with `ALSO SQLITE`.

## Words

| Word | Stack | |
|---|---|---|
| `DB-OPEN` | `( c-addr u -- db ior )` | opens or creates a database file; `:memory:` is one in memory |
| `DB-CLOSE` | `( db -- ior )` | closes it and finalizes its statements |
| `DB-EXEC` | `( c-addr u db -- ior )` | runs SQL without results, also several statements |
| `DB-PREPARE` | `( c-addr u db -- stmt ior )` | one statement, with parameters `?1`, `?2`, … |
| `DB-BIND` | `( x n stmt -- ior )` | an integer for parameter n (from 1) |
| `DB-BIND-TEXT` | `( c-addr u n stmt -- ior )` | a text |
| `DB-BIND-FLOAT` | `( n stmt -- ior ) ( F: r -- )` | a float |
| `DB-BIND-NULL` | `( n stmt -- ior )` | NULL |
| `DB-STEP` | `( stmt -- flag ior )` | the next row; false when there is none |
| `DB-RESET` | `( stmt -- ior )` | the next `DB-STEP` runs the statement again |
| `DB-FINALIZE` | `( stmt -- ior )` | done with the statement |
| `DB-COLUMNS` | `( stmt -- n )` | the columns of the current row (0 without one) |
| `DB-COLUMN` | `( n stmt -- c-addr u )` | column n (from 0) as text, `ALLOCATE`d |
| `DB-COLUMN-INT` | `( n stmt -- x )` | as an integer |
| `DB-COLUMN-FLOAT` | `( n stmt -- ) ( F: -- r )` | as a float |
| `DB-NULL?` | `( n stmt -- flag )` | whether it is NULL |
| `DB-CHANGES` | `( db -- n )` | the rows the latest statement changed |
| `DB-LAST-ROWID` | `( db -- n )` | the rowid of the latest `INSERT` |
| `DB-ERROR` | `( -- c-addr u )` | the message of the latest nonzero ior, `ALLOCATE`d |

The iors are -260, an error that SQLite reports (`DB-ERROR` tells which), and -261, a
handle that is no open database or statement. The words without an ior throw -261 for
such a handle and -24 for a row or column that is not there. Binding a parameter starts
the statement again from the top, as `DB-RESET` does.

In the sandbox (without `Permission::Files`) `DB-OPEN` opens only `:memory:`, and gives
the ior -259 for a file; SQL cannot attach a database file either (`ATTACH` and
`VACUUM INTO` fail with -260).

A statement runs at its first `DB-STEP`, which reads all its rows; the next `DB-STEP`s
hand them out one by one.

## Example

```forth
ALSO SQLITE
S" :memory:" DB-OPEN THROW VALUE DB
S" CREATE TABLE t (x); INSERT INTO t VALUES (1), (2), (3)" DB DB-EXEC THROW
S" SELECT sum(x) FROM t" DB DB-PREPARE THROW VALUE SUM
SUM DB-STEP THROW DROP  0 SUM DB-COLUMN-INT .   \ 6
SUM DB-FINALIZE THROW  DB DB-CLOSE THROW
```

## Testing

```
cargo test --manifest-path sqlite/Cargo.toml
```
