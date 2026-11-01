//! The SQLITE words, used from Forth.

use forthling::{Cell, Forth};
use forthling_sqlite::Sqlite;

fn forth() -> Forth {
    let mut f = Forth::new();
    f.install(&Sqlite);
    run(&mut f, "ALSO SQLITE  S\" :memory:\" DB-OPEN THROW VALUE DB");
    f
}

fn run(f: &mut Forth, text: &str) -> String {
    let start = f.output().len();
    f.interpret(text).unwrap_or_else(|e| panic!("{text}: {e}"));
    String::from_utf8_lossy(&f.output()[start..]).into_owned()
}

fn code(f: &mut Forth, text: &str) -> Cell {
    f.interpret(text).expect_err(text).code
}

// Forth reads a line at a time, so the S" string stays on one line.
const PEOPLE: &str = concat!(
    r#"S" CREATE TABLE people (name TEXT, age INTEGER, height REAL); "#,
    r#"INSERT INTO people VALUES ('Ada', 36, 1.65), ('Alan', 41, NULL)" DB DB-EXEC THROW"#
);

#[test]
fn rows_and_columns() {
    let mut f = forth();
    run(&mut f, PEOPLE);
    run(
        &mut f,
        r#"S" SELECT name, age, height FROM people ORDER BY name" DB DB-PREPARE THROW VALUE ST"#,
    );
    let print = ": PEOPLE  BEGIN ST DB-STEP THROW WHILE  0 ST DB-COLUMN 2DUP TYPE DROP FREE THROW \
                 SPACE 1 ST DB-COLUMN-INT .  2 ST DB-NULL? .  ST DB-COLUMNS . CR  REPEAT ;";
    run(&mut f, print);
    assert_eq!(run(&mut f, "PEOPLE"), "Ada 36 0 3 \nAlan 41 -1 3 \n");
    assert_eq!(run(&mut f, "ST DB-STEP . . ST DB-COLUMNS ."), "0 0 0 ");
    assert_eq!(run(&mut f, "ST DB-RESET . PEOPLE"), "0 Ada 36 0 3 \nAlan 41 -1 3 \n");
    assert_eq!(run(&mut f, "ST DB-RESET DROP ST DB-STEP 2DROP 2 ST DB-COLUMN-FLOAT F."), "1.65 ");
    assert_eq!(run(&mut f, "1 ST DB-COLUMN TYPE"), "36");
    assert_eq!(run(&mut f, "ST DB-FINALIZE . ST DB-FINALIZE ."), "0 -261 ");
}

#[test]
fn parameters() {
    let mut f = forth();
    run(&mut f, PEOPLE);
    run(&mut f, r#"S" INSERT INTO people VALUES (?1, ?2, ?3)" DB DB-PREPARE THROW VALUE INS"#);
    let insert = r#"S" Grace" 1 INS DB-BIND-TEXT THROW 85 2 INS DB-BIND THROW
                    1.5E0 3 INS DB-BIND-FLOAT THROW INS DB-STEP THROW ."#;
    assert_eq!(run(&mut f, insert), "0 ");
    assert_eq!(run(&mut f, "DB DB-CHANGES . DB DB-LAST-ROWID ."), "1 3 ");
    run(
        &mut f,
        r#"S" SELECT count(*), max(age) FROM people WHERE age > ?1" DB DB-PREPARE THROW VALUE Q"#,
    );
    let query = ": OLDER ( n -- ) 1 Q DB-BIND THROW Q DB-STEP THROW DROP 0 Q DB-COLUMN-INT . 1 Q DB-COLUMN-INT . ;";
    run(&mut f, query);
    assert_eq!(run(&mut f, "40 OLDER 0 OLDER"), "2 85 3 85 ");
    let out = run(&mut f, "3 Q DB-BIND-NULL . Q DB-STEP NIP . DB-ERROR TYPE");
    assert!(out.starts_with("0 -260 Wrong number of parameters"), "{out}");
}

#[test]
fn errors_are_iors() {
    let mut f = forth();
    assert_eq!(run(&mut f, r#"S" SELEKT 1" DB DB-PREPARE . ."#), "-260 0 ");
    assert!(run(&mut f, "DB-ERROR TYPE").contains("syntax error"));
    assert_eq!(run(&mut f, r#"S" SELECT * FROM nowhere" DB DB-EXEC ."#), "-260 ");
    assert!(run(&mut f, "DB-ERROR TYPE").contains("no such table"));
    assert_eq!(run(&mut f, r#"S" SELECT 1" 999 DB-PREPARE . ."#), "-261 0 ");
    assert_eq!(run(&mut f, "S\" x\" 0 99 DB-BIND-TEXT ."), "-261 ");
    assert_eq!(run(&mut f, r#"S" SELECT ?1" DB DB-PREPARE THROW 5 0 ROT DB-BIND ."#), "-260 ");
    run(&mut f, r#"S" SELECT 1" DB DB-PREPARE THROW VALUE ONE"#);
    assert_eq!(code(&mut f, "0 ONE DB-COLUMN"), -24); // no current row yet
    run(&mut f, "ONE DB-STEP 2DROP");
    assert_eq!(code(&mut f, "1 ONE DB-COLUMN"), -24);
    assert_eq!(code(&mut f, "0 DB DB-COLUMN"), -261); // a database is no statement
    assert_eq!(code(&mut f, "ONE DB-CHANGES"), -261);
}

#[test]
fn closing_finalizes_the_statements() {
    let mut f = forth();
    run(&mut f, r#"S" SELECT 1" DB DB-PREPARE THROW VALUE ONE"#);
    assert_eq!(run(&mut f, "DB DB-CLOSE . DB DB-CLOSE . ONE DB-STEP . ."), "0 -261 -261 0 ");
}

#[test]
fn files_and_discovery() {
    let path = std::env::temp_dir().join(format!("forthling-sqlite-{}.db", std::process::id()));
    let name = path.to_string_lossy().into_owned();
    let mut f = Forth::new();
    assert_eq!(run(&mut f, "S\" FORTHLING-SQLITE\" ENVIRONMENT? ."), "0 ");
    f.install(&Sqlite);
    assert_eq!(run(&mut f, "S\" FORTHLING-SQLITE\" ENVIRONMENT? . ."), "-1 1 ");
    assert_eq!(code(&mut f, "DB-OPEN"), -13); // only after ALSO SQLITE
    let setup = format!(
        "ALSO SQLITE S\" {name}\" DB-OPEN THROW VALUE DB \
         S\" CREATE TABLE t (x); INSERT INTO t VALUES (7)\" DB DB-EXEC THROW DB DB-CLOSE THROW"
    );
    run(&mut f, &setup);
    let read = format!(
        "S\" {name}\" DB-OPEN THROW TO DB S\" SELECT x FROM t\" DB DB-PREPARE THROW \
         DUP DB-STEP THROW DROP 0 SWAP DB-COLUMN-INT . DB DB-CLOSE THROW"
    );
    assert_eq!(run(&mut f, &read), "7 ");
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn the_example_runs() {
    let mut f = Forth::new();
    f.install(&Sqlite);
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/people.fs");
    f.include(path).unwrap_or_else(|e| panic!("{e}"));
    let out = String::from_utf8_lossy(f.output()).into_owned();
    assert_eq!(out, "Born after 1900:\n1906 Grace Hopper\n1912 Alan Turing\n");
}

#[test]
fn the_sandbox_keeps_sql_away_from_files() {
    let path =
        std::env::temp_dir().join(format!("forthling-sqlite-sandbox-{}.db", std::process::id()));
    let name = path.to_string_lossy().into_owned();
    let mut f = Forth::sandboxed();
    f.install(&Sqlite);
    let open = format!("ALSO SQLITE S\" {name}\" DB-OPEN . . DB-ERROR TYPE");
    assert_eq!(run(&mut f, &open), format!("-259 0 DB-OPEN {name}: no files on the disk"));
    run(&mut f, "S\" :memory:\" DB-OPEN THROW VALUE DB");
    let attach = format!("S\" ATTACH '{name}' AS other\" DB DB-EXEC .");
    assert_eq!(run(&mut f, &attach), "-260 ");
    run(&mut f, "S\" CREATE TABLE t (x)\" DB DB-EXEC THROW");
    let vacuum = format!("S\" VACUUM INTO '{name}'\" DB DB-EXEC .");
    assert_eq!(run(&mut f, &vacuum), "-260 ");
    assert!(!path.exists(), "SQL wrote {name}");
}

#[test]
fn sql_keeps_to_the_limits() {
    use forthling::Limits;
    use std::time::{Duration, Instant};
    // An endless query stops when the time is up, inside its one step.
    let mut f = forth();
    f.set_limits(Limits { time: Some(Duration::from_millis(200)), ..Limits::default() });
    let endless =
        "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM c) SELECT count(*) FROM c";
    let start = Instant::now();
    let e = f.interpret(&format!("S\" {endless}\" DB DB-PREPARE THROW DB-STEP")).unwrap_err();
    assert_eq!(e.to_string(), "error -258: limit exceeded: time");
    assert!(start.elapsed() < Duration::from_secs(3), "{:?}", start.elapsed());
    let start = Instant::now();
    let e = f.interpret(&format!("S\" CREATE TABLE t AS {endless}\" DB DB-EXEC")).unwrap_err();
    assert_eq!(e.code, -258);
    assert!(start.elapsed() < Duration::from_secs(3), "{:?}", start.elapsed());
    // A result may not be larger than the Forth memory.
    let mut f = Forth::with_memory(64 << 10);
    f.install(&Sqlite);
    run(&mut f, "ALSO SQLITE  S\" :memory:\" DB-OPEN THROW VALUE DB");
    let big = "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM c LIMIT 10000) SELECT 'some text' FROM c";
    let out = run(&mut f, &format!("S\" {big}\" DB DB-PREPARE THROW DB-STEP . . DB-ERROR TYPE"));
    assert_eq!(out, "-260 0 the result of the query is larger than the Forth memory");
    // In the sandbox a database cannot grow larger than the Forth memory.
    let mut f = Forth::with_memory(64 << 10);
    f.permit(forthling::Permission::Files, false);
    f.install(&Sqlite);
    run(
        &mut f,
        "ALSO SQLITE  S\" :memory:\" DB-OPEN THROW VALUE DB  S\" CREATE TABLE t (x)\" DB DB-EXEC THROW",
    );
    let fill = "INSERT INTO t WITH RECURSIVE c(i) AS (SELECT 1 UNION ALL SELECT i+1 FROM c LIMIT 10000) SELECT randomblob(100) FROM c";
    let out = run(&mut f, &format!("S\" {fill}\" DB DB-EXEC . DB-ERROR TYPE"));
    assert!(out.starts_with("-260 ") && out.contains("full"), "{out}");
}
