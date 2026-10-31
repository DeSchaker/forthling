//! SQLite for forthling: the word list SQLITE, in the style of the FILE word
//! set. Databases and statements are handles (cells), errors are iors on
//! the stack, and strings that a word returns are ALLOCATEd, for the program
//! to FREE.
//!
//! ```
//! use forthling::Forth;
//!
//! let mut f = Forth::new();
//! f.install(&forthling_sqlite::Sqlite);
//! f.interpret("ALSO SQLITE  S\" :memory:\" DB-OPEN THROW VALUE DB")?;
//! f.interpret("S\" SELECT 6 * 7\" DB DB-PREPARE THROW VALUE ST")?;
//! f.interpret("ST DB-STEP THROW DROP  0 ST DB-COLUMN-INT .")?;
//! assert_eq!(f.output(), b"42 ");
//! # Ok::<(), forthling::Throw>(())
//! ```
//!
//! A statement runs at its first DB-STEP, which reads all its rows; the
//! next DB-STEPs hand them out one by one.

use forthling::{Cell, Extension, Forth, NOT_PERMITTED, Permission, Res, Throw};
use rusqlite::limits::Limit;
use rusqlite::types::Value;
use rusqlite::{Connection, params_from_iter};
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

/// The ior of an error that SQLite reports; DB-ERROR gives its message.
pub const SQLITE_ERROR: Cell = -260;
/// The ior of a handle that is no open database or statement of the right
/// kind; words without an ior throw it.
pub const INVALID_HANDLE: Cell = -261;

/// The extension: `forth.install(&Sqlite)`.
pub struct Sqlite;

/// The open databases and statements, by handle; one counter numbers both,
/// so a database is never mistaken for a statement.
#[derive(Default)]
struct State {
    last: Cell,
    dbs: HashMap<Cell, Connection>,
    stmts: HashMap<Cell, Stmt>,
    error: String, // the message of the latest nonzero ior
}

struct Stmt {
    db: Cell,
    sql: String,
    params: Vec<Value>,                 // ?1 is params[0]; unbound ones are NULL
    rows: Option<VecDeque<Vec<Value>>>, // the rows still to come, once it ran
    row: Option<Vec<Value>>,            // the current row
}

/// Why a word failed: its ior and the message for DB-ERROR.
struct Failure(Cell, String);

impl From<rusqlite::Error> for Failure {
    fn from(e: rusqlite::Error) -> Self {
        Failure(SQLITE_ERROR, e.to_string())
    }
}

/// What running SQL may take: the time of the evaluation, and results no
/// larger than the Forth memory.
#[derive(Clone, Copy)]
struct Bounds {
    deadline: Option<Instant>,
    room: usize,
}

fn bounds(f: &Forth) -> Bounds {
    Bounds { deadline: f.deadline(), room: f.memory().len() }
}

/// Lets SQLite stop a statement when the time is up; a query can run long,
/// even forever (WITH RECURSIVE), inside one Forth step.
fn guard(conn: &Connection, deadline: Option<Instant>) -> Result<(), Failure> {
    match deadline {
        Some(d) => conn.progress_handler(1000, Some(move || Instant::now() >= d))?,
        None => conn.progress_handler(1000, None::<fn() -> bool>)?,
    }
    Ok(())
}

/// What a value takes, roughly, for the bounds and for the work it is.
fn size(v: &Value) -> usize {
    16 + match v {
        Value::Text(s) => s.len(),
        Value::Blob(b) => b.len(),
        _ => 8,
    }
}

fn invalid(handle: Cell) -> Failure {
    Failure(INVALID_HANDLE, format!("{handle} is not an open handle"))
}

impl State {
    fn handle(&mut self) -> Cell {
        self.last += 1;
        self.last
    }

    fn db(&self, db: Cell) -> Result<&Connection, Failure> {
        self.dbs.get(&db).ok_or_else(|| invalid(db))
    }

    fn stmt(&mut self, stmt: Cell) -> Result<&mut Stmt, Failure> {
        self.stmts.get_mut(&stmt).ok_or_else(|| invalid(stmt))
    }

    /// Opens a database; without `files`, only one in memory, which cannot
    /// reach files through SQL either (ATTACH and VACUUM INTO), and which
    /// cannot grow larger than the Forth memory.
    fn open(&mut self, path: &str, files: bool, room: usize) -> Result<Cell, Failure> {
        if !files && path != ":memory:" {
            return Err(Failure(NOT_PERMITTED, format!("DB-OPEN {path}: no file access")));
        }
        let conn = Connection::open(path)?;
        if !files {
            conn.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 0)?;
            let page: i64 = conn.pragma_query_value(None, "page_size", |row| row.get(0))?;
            conn.pragma_update(None, "max_page_count", (room as i64 / page).max(1))?;
        }
        let db = self.handle();
        self.dbs.insert(db, conn);
        Ok(db)
    }

    fn close(&mut self, db: Cell) -> Result<(), Failure> {
        let conn = self.dbs.remove(&db).ok_or_else(|| invalid(db))?;
        self.stmts.retain(|_, s| s.db != db);
        conn.close().map_err(|(_, e)| e.into())
    }

    fn prepare(&mut self, sql: String, db: Cell) -> Result<Cell, Failure> {
        self.db(db)?.prepare(&sql)?; // reports a mistake in the SQL now
        let stmt = self.handle();
        self.stmts.insert(stmt, Stmt { db, sql, params: Vec::new(), rows: None, row: None });
        Ok(stmt)
    }

    /// Binds parameter n (from 1); the statement starts again from the top.
    fn bind(&mut self, value: Value, n: Cell, stmt: Cell) -> Result<(), Failure> {
        let s = self.stmt(stmt)?;
        let i = usize::try_from(n - 1)
            .map_err(|_| Failure(SQLITE_ERROR, format!("no parameter {n}: they count from 1")))?;
        if i >= s.params.len() {
            s.params.resize(i + 1, Value::Null);
        }
        s.params[i] = value;
        s.rows = None;
        s.row = None;
        Ok(())
    }

    /// The next row; the first step runs the statement and reads all its
    /// rows. Also gives the bytes it read, as work.
    fn step(&mut self, stmt: Cell, bounds: Bounds) -> Result<(bool, usize), Failure> {
        let s = self.stmts.get_mut(&stmt).ok_or_else(|| invalid(stmt))?;
        let mut read = 0;
        if s.rows.is_none() {
            let conn = self.dbs.get(&s.db).ok_or_else(|| invalid(s.db))?;
            guard(conn, bounds.deadline)?;
            let mut prepared = conn.prepare(&s.sql)?;
            let columns = prepared.column_count();
            let mut rows = prepared.query(params_from_iter(s.params.iter()))?;
            let mut all = VecDeque::new();
            while let Some(row) = rows.next()? {
                let values: Vec<Value> =
                    (0..columns).map(|i| row.get(i)).collect::<Result<_, _>>()?;
                read += values.iter().map(size).sum::<usize>();
                if read > bounds.room {
                    let message = "the result of the query is larger than the Forth memory";
                    return Err(Failure(SQLITE_ERROR, message.into()));
                }
                all.push_back(values);
            }
            s.rows = Some(all);
        }
        s.row = s.rows.as_mut().and_then(VecDeque::pop_front);
        Ok((s.row.is_some(), read))
    }

    fn exec(&self, sql: &str, db: Cell, bounds: Bounds) -> Result<(), Failure> {
        let conn = self.db(db)?;
        guard(conn, bounds.deadline)?;
        Ok(conn.execute_batch(sql)?)
    }

    fn reset(&mut self, stmt: Cell) -> Result<(), Failure> {
        let s = self.stmt(stmt)?;
        s.rows = None;
        s.row = None;
        Ok(())
    }

    fn finalize(&mut self, stmt: Cell) -> Result<(), Failure> {
        self.stmts.remove(&stmt).map(drop).ok_or_else(|| invalid(stmt))
    }
}

/// Pushes the ior of a result and keeps the message of a failure.
fn push_ior(f: &mut Forth, result: Result<(), Failure>) {
    match result {
        Ok(()) => f.push(0),
        Err(Failure(ior, message)) => {
            f.state::<State>().error = message;
            f.push(ior);
        }
    }
}

/// Pushes a value and an ior; 0 for the value when it failed.
fn push_value_ior(f: &mut Forth, result: Result<Cell, Failure>) {
    let (value, result) = match result {
        Ok(v) => (v, Ok(())),
        Err(e) => (0, Err(e)),
    };
    f.push(value);
    push_ior(f, result);
}

/// ( n stmt -- ): column n (from 0) of the current row.
fn column(f: &mut Forth) -> Res<Value> {
    let stmt = f.pop()?;
    let n = f.pop()?;
    let state = f.state::<State>();
    let Some(s) = state.stmts.get(&stmt) else {
        return Err(Throw::with(INVALID_HANDLE, format!("{stmt} is not an open statement")));
    };
    let Some(row) = &s.row else {
        return Err(Throw::with(-24, "the statement has no current row"));
    };
    let value = usize::try_from(n).ok().and_then(|i| row.get(i));
    value.cloned().ok_or_else(|| Throw::with(-24, format!("no column {n}")))
}

fn text(value: Value) -> Vec<u8> {
    match value {
        Value::Null => Vec::new(),
        Value::Integer(n) => n.to_string().into_bytes(),
        Value::Real(r) => r.to_string().into_bytes(),
        Value::Text(s) => s.into_bytes(),
        Value::Blob(b) => b,
    }
}

fn integer(value: Value) -> Cell {
    match value {
        Value::Integer(n) => n,
        Value::Real(r) => r as Cell,
        Value::Text(s) => s.trim().parse().unwrap_or(0),
        Value::Null | Value::Blob(_) => 0,
    }
}

fn real(value: Value) -> f64 {
    match value {
        Value::Integer(n) => n as f64,
        Value::Real(r) => r,
        Value::Text(s) => s.trim().parse().unwrap_or(0.0),
        Value::Null | Value::Blob(_) => 0.0,
    }
}

/// Pops a string as text.
fn pop_text(f: &mut Forth) -> Res<String> {
    Ok(String::from_utf8_lossy(&f.pop_bytes()?).into_owned())
}

/// A word of the SQLITE word list; the word list exists once installed.
fn word(f: &mut Forth, name: &str, word: impl Fn(&mut Forth) -> Res + 'static) {
    let wid = f.wordlist("SQLITE");
    f.add_word(wid, name, word).expect("SQLITE is a word list and names are not empty");
}

impl Extension for Sqlite {
    fn name(&self) -> &str {
        "FORTHLING-SQLITE"
    }

    fn install(&self, f: &mut Forth) {
        // DB-OPEN ( c-addr u -- db ior ): opens or creates a database file;
        // :memory: is a database in memory, the only kind without
        // Permission::Files.
        word(f, "DB-OPEN", |f| {
            let path = pop_text(f)?;
            let (files, room) = (f.permits(Permission::Files), f.memory().len());
            let result = f.state::<State>().open(&path, files, room);
            push_value_ior(f, result);
            Ok(())
        });
        // DB-CLOSE ( db -- ior ), which also finalizes its statements.
        word(f, "DB-CLOSE", |f| {
            let db = f.pop()?;
            let result = f.state::<State>().close(db);
            push_ior(f, result);
            Ok(())
        });
        // DB-EXEC ( c-addr u db -- ior ): runs SQL without results, which
        // may be several statements separated by semicolons.
        word(f, "DB-EXEC", |f| {
            let db = f.pop()?;
            let sql = pop_text(f)?;
            f.charge(sql.len())?;
            let bounds = bounds(f);
            let result = f.state::<State>().exec(&sql, db, bounds);
            f.check_time()?; // SQLite was stopped because the time is up
            push_ior(f, result);
            Ok(())
        });
        // DB-PREPARE ( c-addr u db -- stmt ior ): one statement, which may
        // have parameters ?1, ?2, …
        word(f, "DB-PREPARE", |f| {
            let db = f.pop()?;
            let sql = pop_text(f)?;
            let result = f.state::<State>().prepare(sql, db);
            push_value_ior(f, result);
            Ok(())
        });
        // DB-BIND ( x n stmt -- ior ): an integer for parameter n.
        word(f, "DB-BIND", |f| {
            let (stmt, n, x) = (f.pop()?, f.pop()?, f.pop()?);
            let result = f.state::<State>().bind(Value::Integer(x), n, stmt);
            push_ior(f, result);
            Ok(())
        });
        // DB-BIND-TEXT ( c-addr u n stmt -- ior )
        word(f, "DB-BIND-TEXT", |f| {
            let (stmt, n) = (f.pop()?, f.pop()?);
            let text = pop_text(f)?;
            let result = f.state::<State>().bind(Value::Text(text), n, stmt);
            push_ior(f, result);
            Ok(())
        });
        // DB-BIND-FLOAT ( n stmt -- ior ) ( F: r -- )
        word(f, "DB-BIND-FLOAT", |f| {
            let (stmt, n) = (f.pop()?, f.pop()?);
            let r = f.fpop()?;
            let result = f.state::<State>().bind(Value::Real(r), n, stmt);
            push_ior(f, result);
            Ok(())
        });
        // DB-BIND-NULL ( n stmt -- ior )
        word(f, "DB-BIND-NULL", |f| {
            let (stmt, n) = (f.pop()?, f.pop()?);
            let result = f.state::<State>().bind(Value::Null, n, stmt);
            push_ior(f, result);
            Ok(())
        });
        // DB-STEP ( stmt -- flag ior ): true when there is a next row.
        word(f, "DB-STEP", |f| {
            let stmt = f.pop()?;
            let bounds = bounds(f);
            let result = f.state::<State>().step(stmt, bounds);
            f.check_time()?; // SQLite was stopped because the time is up
            let result = result.map(|(row, read)| (if row { -1 } else { 0 }, read));
            f.charge(result.as_ref().map_or(0, |&(_, read)| read))?;
            push_value_ior(f, result.map(|(flag, _)| flag));
            Ok(())
        });
        // DB-RESET ( stmt -- ior ): the next DB-STEP runs it again.
        word(f, "DB-RESET", |f| {
            let stmt = f.pop()?;
            let result = f.state::<State>().reset(stmt);
            push_ior(f, result);
            Ok(())
        });
        // DB-FINALIZE ( stmt -- ior )
        word(f, "DB-FINALIZE", |f| {
            let stmt = f.pop()?;
            let result = f.state::<State>().finalize(stmt);
            push_ior(f, result);
            Ok(())
        });
        // DB-COLUMNS ( stmt -- n ): the columns of the current row.
        word(f, "DB-COLUMNS", |f| {
            let stmt = f.pop()?;
            let state = f.state::<State>();
            let s = state.stmt(stmt).map_err(|Failure(code, m)| Throw::with(code, m))?;
            let n = s.row.as_ref().map_or(0, Vec::len);
            f.push(n as Cell);
            Ok(())
        });
        // DB-COLUMN ( n stmt -- c-addr u ): column n as text, ALLOCATEd.
        word(f, "DB-COLUMN", |f| {
            let value = column(f)?;
            f.push_bytes(&text(value))
        });
        // DB-COLUMN-INT ( n stmt -- x )
        word(f, "DB-COLUMN-INT", |f| {
            let value = column(f)?;
            f.push(integer(value));
            Ok(())
        });
        // DB-COLUMN-FLOAT ( n stmt -- ) ( F: -- r )
        word(f, "DB-COLUMN-FLOAT", |f| {
            let value = column(f)?;
            f.fpush(real(value))
        });
        // DB-NULL? ( n stmt -- flag )
        word(f, "DB-NULL?", |f| {
            let value = column(f)?;
            f.push(if value == Value::Null { -1 } else { 0 });
            Ok(())
        });
        // DB-CHANGES ( db -- n ): the rows the latest statement changed.
        word(f, "DB-CHANGES", |f| {
            let db = f.pop()?;
            let state = f.state::<State>();
            let conn = state.db(db).map_err(|Failure(code, m)| Throw::with(code, m))?;
            let n = conn.changes() as Cell;
            f.push(n);
            Ok(())
        });
        // DB-LAST-ROWID ( db -- n ): the rowid of the latest INSERT.
        word(f, "DB-LAST-ROWID", |f| {
            let db = f.pop()?;
            let state = f.state::<State>();
            let conn = state.db(db).map_err(|Failure(code, m)| Throw::with(code, m))?;
            let n = conn.last_insert_rowid();
            f.push(n);
            Ok(())
        });
        // DB-ERROR ( -- c-addr u ): the message of the latest nonzero ior,
        // ALLOCATEd.
        word(f, "DB-ERROR", |f| {
            let message = f.state::<State>().error.clone();
            f.push_bytes(message.as_bytes())
        });
    }
}
