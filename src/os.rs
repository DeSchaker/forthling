//! The operating-system words of Gforth: the word list OPERATING-SYSTEM,
//! installed as the extension `OperatingSystem`. A script opens it with
//! `ALSO OPERATING-SYSTEM`.
//!
//! What reaches the machine needs `Permission::System`, which
//! `Forth::sandboxed()` and `--sandbox` deny: the words then throw -259 (or
//! give it as their ior). That is `SYSTEM SH GETENV`, the command line
//! (`ARG` ...), the directories, pipes and `STDERR`. The words that only work on
//! text (`BASENAME`, `DIRNAME`, `FILENAME-MATCH`), `$?`, and `STDIN`,
//! `STDOUT` (the input and output of the Forth system itself, so the host
//! decides where they go) need nothing.

use crate::*;
use std::cell::Cell as Counter;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::rc::Rc;

/// The extension: `forth.install(&OperatingSystem)`; ENVIRONMENT? knows it as
/// FORTHLING-OPERATING-SYSTEM.
pub struct OperatingSystem;

/// A standard stream, as a file id (STDIN, STDOUT, STDERR).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Std {
    In,
    Out,
    Err,
}

/// What the words keep between calls.
#[derive(Default)]
#[cfg_attr(
    target_arch = "wasm32",
    allow(clippy::zero_sized_map_values, reason = "a Child is empty on wasm")
)]
pub(crate) struct Os {
    args: Vec<String>,                             // args[0] is the program, as in C
    cells: Option<(usize, usize)>,                 // where the variables ARGC and ARGV are
    block: Option<usize>,                          // the ALLOCATEd vector and strings behind ARGV
    addrs: Vec<usize>,                             // where each argument is in that block
    scratch: Option<usize>,                        // the block of the last string given back
    status: Cell,                                  // $?
    dirs: Vec<Option<fs::ReadDir>>,                // open directories, indexed by id - 1
    pipes: std::collections::HashMap<Cell, Child>, // by file id
    std: [Option<Cell>; 3],                        // the file ids of STDIN, STDOUT, STDERR
}

impl Extension for OperatingSystem {
    fn name(&self) -> &'static str {
        "FORTHLING-OPERATING-SYSTEM"
    }

    fn install(&self, f: &mut Forth) {
        let wid = f.wordlist("OPERATING-SYSTEM");
        let words: &[(&str, Prim)] = &[
            ("SYSTEM", system),
            ("SH", sh),
            ("$?", |f| {
                let status = f.state::<Os>().status;
                f.push(status);
                Ok(())
            }),
            ("GETENV", getenv),
            ("ARG", arg),
            ("NEXT-ARG", next_arg),
            ("SHIFT-ARGS", shift_args),
            ("OPEN-PIPE", open_pipe),
            ("CLOSE-PIPE", close_pipe),
            ("OPEN-DIR", open_dir),
            ("READ-DIR", read_dir),
            ("CLOSE-DIR", close_dir),
            ("GET-DIR", get_dir),
            ("SET-DIR", set_dir),
            ("MKDIR-PARENTS", mkdir_parents),
            ("BASENAME", basename),
            ("DIRNAME", dirname),
            ("FILENAME-MATCH", filename_match),
            ("STDIN", |f| std_word(f, Std::In)),
            ("STDOUT", |f| std_word(f, Std::Out)),
            ("STDERR", |f| std_word(f, Std::Err)),
            ("OUTFILE-EXECUTE", outfile_execute),
            ("INFILE-EXECUTE", infile_execute),
        ];
        for &(name, word) in words {
            f.add_word(wid, name, word).expect("OPERATING-SYSTEM is a word list");
        }
        // ARGC and ARGV are variables, so `ARGC @` works as in Gforth; they
        // are compiled into the word list, and the search order and the
        // compilation word list are left as they were.
        let (order, current) = (f.order.clone(), f.current_wid);
        let made = f
            .interpret("ALSO OPERATING-SYSTEM DEFINITIONS VARIABLE ARGC VARIABLE ARGV ARGC ARGV")
            .and_then(|()| Ok((f.pop()?, f.pop()?)));
        f.order = order;
        f.current_wid = current;
        let (vector_var, count_var) = made.expect("ARGC and ARGV must compile");
        f.state::<Os>().cells = Some((to_index(count_var), to_index(vector_var)));
        sync_args(f);
    }
}

type Prim = fn(&mut Forth) -> Res;

impl Forth {
    /// Sets the command line that `ARGC`, `ARGV`, `ARG` and `NEXT-ARG` of the
    /// word list OPERATING-SYSTEM show: the program name, then the arguments
    /// that are not used yet. Without it the list holds an empty command line.
    pub fn set_arguments(&mut self, args: Vec<String>) {
        self.state::<Os>().args = args;
        sync_args(self);
    }

    /// Takes the first argument after the program name, as `NEXT-ARG` does.
    pub(crate) fn take_argument(&mut self) -> Option<String> {
        let args = &mut self.state::<Os>().args;
        let arg = (args.len() > 1).then(|| args.remove(1));
        sync_args(self);
        arg
    }
}

fn allowed(f: &Forth, what: impl FnOnce() -> String) -> Res {
    f.need(Permission::System, what)
}

/// Gives a string back to the program: a copy in an allocated block that is
/// valid until the next word of this list gives one back.
fn give(f: &mut Forth, bytes: &[u8]) -> Res {
    if let Some(old) = f.state::<Os>().scratch.take() {
        f.heap.free(old);
    }
    let a = f.heap.allocate(bytes.len(), f.here).ok_or_else(|| Throw::new(ALLOCATE_FAILED))?;
    f.mem[a..a + bytes.len()].copy_from_slice(bytes);
    f.state::<Os>().scratch = Some(a);
    f.push(to_cell(a));
    f.push(to_cell(bytes.len()));
    Ok(())
}

/// Makes ARGC and ARGV tell the truth: ARGV points at a vector of pointers
/// to zero-terminated strings, ended by a zero, as in C.
fn sync_args(f: &mut Forth) {
    let os = f.state::<Os>();
    let (args, old, vars) = (os.args.clone(), os.block.take(), os.cells);
    os.addrs.clear();
    if let Some(old) = old {
        f.heap.free(old);
    }
    let Some((count_var, vector_var)) = vars else { return };
    let vector = (args.len() + 1) * CELL;
    let total = vector + args.iter().map(|a| a.len() + 1).sum::<usize>();
    let block = f.heap.allocate(total, f.here);
    let mut addrs = Vec::new();
    if let Some(b) = block {
        let mut at = b + vector;
        for (k, a) in args.iter().enumerate() {
            f.set(b + k * CELL, to_cell(at));
            f.mem[at..at + a.len()].copy_from_slice(a.as_bytes());
            f.mem[at + a.len()] = 0;
            addrs.push(at);
            at += a.len() + 1;
        }
        f.set(b + args.len() * CELL, 0);
    }
    f.set(count_var, to_cell(args.len()));
    f.set(vector_var, block.map_or(0, to_cell));
    let os = f.state::<Os>();
    (os.block, os.addrs) = (block, addrs);
}

fn pop_xt(f: &mut Forth, word: &str) -> Res<usize> {
    let xt = f.pop()?;
    usize::try_from(xt).ok().filter(|&i| i < f.dict.len()).ok_or_else(|| {
        Throw::with(INVALID_ARGUMENT, format!("{word}: invalid execution token {xt}"))
    })
}

// ---- Processes and the environment --------------------------------------

/// The command in the shell of the system.
fn shell(command: &str) -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut c = Command::new("cmd");
        // The quotes make cmd take the whole command as it is given.
        c.args(["/S", "/C"]).raw_arg(format!("\"{command}\""));
        c
    }
    #[cfg(not(windows))]
    {
        let mut c = Command::new("sh");
        c.arg("-c").arg(command);
        c
    }
}

/// Runs a command to its end; its exit status is `$?`.
fn run_command(f: &mut Forth, command: &str, word: &str) -> Res {
    allowed(f, || format!("{word} {command}"))?;
    if f.interactive {
        f.flush(); // what the program printed comes before what the command prints
    }
    let mut child = shell(command)
        .spawn()
        .map_err(|e| Throw::with(FILE_IO, format!("{word} {command}: {e}")))?;
    let status = f.wait_child(&mut child)?;
    f.state::<Os>().status = status.code().map_or(-1, Cell::from);
    Ok(())
}

/// `SYSTEM ( c-addr u -- )`
fn system(f: &mut Forth) -> Res {
    let command = f.pop_name()?;
    run_command(f, &command, "SYSTEM")
}

/// `SH ( "rest-of-line" -- )`
fn sh(f: &mut Forth) -> Res {
    let (a, n) = f.parse(b'\n');
    let command = f.text(a, n);
    run_command(f, command.trim(), "SH")
}

/// `GETENV ( c-addr1 u1 -- c-addr2 u2 )`: the value, or an empty string.
fn getenv(f: &mut Forth) -> Res {
    let (i, n) = f.pop_str()?;
    let name = f.text(i, n);
    allowed(f, || format!("GETENV {name}"))?;
    let value =
        if name.is_empty() || name.contains(['=', '\0']) { None } else { std::env::var_os(&name) };
    if let Some(v) = value {
        return give(f, v.to_string_lossy().as_bytes());
    }
    f.push(to_cell(i));
    f.push(0);
    Ok(())
}

/// `ARG ( u -- c-addr u2 )`: argument u, `0 ARG` is the program; `0 0` past the last.
fn arg(f: &mut Forth) -> Res {
    allowed(f, || "ARG".into())?;
    let k = to_index(f.pop()?);
    let os = f.state::<Os>();
    let found = os.args.get(k).map(String::len).zip(os.addrs.get(k).copied());
    if let Some((len, at)) = found {
        f.push(to_cell(at));
        f.push(to_cell(len));
    } else if let Some(text) = os.args.get(k).cloned() {
        give(f, text.as_bytes())?; // no room for the vector, but the argument is there
    } else {
        f.push(0);
        f.push(0);
    }
    Ok(())
}

/// `NEXT-ARG ( -- c-addr u )`: takes `1 ARG` away and gives it; `0 0` if none.
fn next_arg(f: &mut Forth) -> Res {
    allowed(f, || "NEXT-ARG".into())?;
    if let Some(a) = f.take_argument() {
        return give(f, a.as_bytes());
    }
    f.push(0);
    f.push(0);
    Ok(())
}

/// `SHIFT-ARGS ( -- )`
fn shift_args(f: &mut Forth) -> Res {
    allowed(f, || "SHIFT-ARGS".into())?;
    f.take_argument();
    Ok(())
}

/// A command started by OPEN-PIPE, as a file.
enum Pipe {
    Read(BufReader<ChildStdout>),
    Write(ChildStdin),
}

fn not_for(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, format!("a pipe cannot be {what}"))
}

impl Read for Pipe {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        match self {
            Pipe::Read(r) => r.read(out),
            Pipe::Write(_) => Err(not_for("read here")),
        }
    }
}

impl Write for Pipe {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        match self {
            Pipe::Write(w) => w.write(data),
            Pipe::Read(_) => Err(not_for("written here")),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Pipe::Write(w) => w.flush(),
            Pipe::Read(_) => Ok(()),
        }
    }
}

impl Seek for Pipe {
    fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
        Err(not_for("repositioned"))
    }
}

impl FileHandle for Pipe {
    fn size(&mut self) -> io::Result<u64> {
        Err(not_for("measured"))
    }

    fn set_size(&mut self, _: u64) -> io::Result<()> {
        Err(not_for("resized"))
    }

    fn read_line(&mut self, max: usize) -> io::Result<Option<Vec<u8>>> {
        match self {
            Pipe::Read(r) => crate::files::read_line_buffered(r, max),
            Pipe::Write(_) => Err(not_for("read here")),
        }
    }
}

/// `OPEN-PIPE ( c-addr u wfam -- wfileid wior )`: R/O reads the output of the
/// command, W/O writes to its input.
fn open_pipe(f: &mut Forth) -> Res {
    let fam = f.pop()? & 3;
    let command = f.pop_name()?;
    if !f.permits(Permission::System) {
        f.push(0);
        f.push(NOT_PERMITTED);
        return Ok(());
    }
    let write = fam == crate::files::W_O;
    let mut cmd = shell(&command);
    if write {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdout(Stdio::piped());
    }
    if f.interactive {
        f.flush();
    }
    let spawned = (fam != 2).then(|| cmd.spawn().ok()).flatten().and_then(|mut child| {
        let pipe = if write {
            child.stdin.take().map(Pipe::Write)
        } else {
            child.stdout.take().map(|o| Pipe::Read(BufReader::new(o)))
        };
        pipe.map(|pipe| (child, pipe))
    });
    if let Some((child, pipe)) = spawned {
        let fid = f.add_file(Box::new(pipe), &command);
        f.state::<Os>().pipes.insert(fid, child);
        f.push(fid);
        f.push(0);
    } else {
        f.push(0);
        f.push(OPEN_FILE_FAILED);
    }
    Ok(())
}

/// `CLOSE-PIPE ( wfileid -- wretval wior )`: waits for the command; wretval
/// is its exit status.
fn close_pipe(f: &mut Forth) -> Res {
    let fid = f.pop()?;
    if !f.permits(Permission::System) {
        f.push(0);
        f.push(NOT_PERMITTED);
        return Ok(());
    }
    if let Some(mut child) = f.state::<Os>().pipes.remove(&fid) {
        f.close_file(fid); // the command sees the end of its input
        let status = f.wait_child(&mut child)?;
        f.push(status.code().map_or(-1, Cell::from));
        f.push(0);
    } else {
        f.push(0);
        f.push(CLOSE_FILE_FAILED);
    }
    Ok(())
}

// ---- Directories and paths ---------------------------------------------

/// `OPEN-DIR ( c-addr u -- wdirid wior )`
fn open_dir(f: &mut Forth) -> Res {
    let name = f.pop_name()?;
    if !f.permits(Permission::System) {
        f.push(0);
        f.push(NOT_PERMITTED);
        return Ok(());
    }
    if let Ok(dir) = fs::read_dir(&name) {
        let dirs = &mut f.state::<Os>().dirs;
        let slot = dirs.iter().position(Option::is_none).unwrap_or(dirs.len());
        if slot == dirs.len() {
            dirs.push(Some(dir));
        } else {
            dirs[slot] = Some(dir);
        }
        f.push(to_cell(slot) + 1);
        f.push(0);
    } else {
        f.push(0);
        f.push(OPEN_FILE_FAILED);
    }
    Ok(())
}

/// `READ-DIR ( c-addr u1 wdirid -- u2 flag wior )`: the name of the next
/// entry (not `.` and `..`) in the buffer; flag false and u2 0 when there is
/// none. A name that does not fit in u1 characters gives ior -70.
fn read_dir(f: &mut Forth) -> Res {
    let id = f.pop()?;
    let (i, n) = f.pop_str()?;
    let denied = !f.permits(Permission::System);
    let dir = (!denied).then(|| {
        let dirs = &mut f.state::<Os>().dirs;
        dirs.get_mut(to_index(id).wrapping_sub(1)).and_then(Option::as_mut)
    });
    let entry = match dir {
        None => Err(NOT_PERMITTED),
        Some(None) => Err(READ_FILE_FAILED),
        Some(Some(dir)) => match dir.next() {
            None => Ok(None),
            Some(Ok(e)) => Ok(Some(e.file_name().to_string_lossy().into_owned())),
            Some(Err(_)) => Err(READ_FILE_FAILED),
        },
    };
    let (len, more, ior) = match entry {
        Ok(Some(name)) if name.len() <= n => {
            f.mem[i..i + name.len()].copy_from_slice(name.as_bytes());
            (name.len(), true, 0)
        }
        Ok(Some(_)) => (0, false, READ_FILE_FAILED),
        Ok(None) => (0, false, 0),
        Err(code) => (0, false, code),
    };
    f.push(to_cell(len));
    f.push(flag(more));
    f.push(ior);
    Ok(())
}

/// `CLOSE-DIR ( wdirid -- wior )`
fn close_dir(f: &mut Forth) -> Res {
    let id = f.pop()?;
    let ior = if f.permits(Permission::System) {
        let dirs = &mut f.state::<Os>().dirs;
        let open = dirs.get_mut(to_index(id).wrapping_sub(1)).and_then(Option::take);
        crate::files::ior(open.is_some(), CLOSE_FILE_FAILED)
    } else {
        NOT_PERMITTED
    };
    f.push(ior);
    Ok(())
}

/// `GET-DIR ( c-addr1 u1 -- c-addr2 u2 )`: the current directory in the
/// buffer; `0 0` if it does not fit.
fn get_dir(f: &mut Forth) -> Res {
    let (i, n) = f.pop_str()?;
    allowed(f, || "GET-DIR".into())?;
    let dir = std::env::current_dir().map(|d| d.to_string_lossy().into_owned()).unwrap_or_default();
    if dir.is_empty() || dir.len() > n {
        f.push(0);
        f.push(0);
    } else {
        f.mem[i..i + dir.len()].copy_from_slice(dir.as_bytes());
        f.push(to_cell(i));
        f.push(to_cell(dir.len()));
    }
    Ok(())
}

/// `SET-DIR ( c-addr u -- wior )`
fn set_dir(f: &mut Forth) -> Res {
    let name = f.pop_name()?;
    let ior = if f.permits(Permission::System) {
        crate::files::ior(std::env::set_current_dir(&name).is_ok(), FILE_IO)
    } else {
        NOT_PERMITTED
    };
    f.push(ior);
    Ok(())
}

/// `MKDIR-PARENTS ( c-addr u mode -- ior )`: the directory and its parents;
/// `mode` is used on Unix only. A directory that exists is not an error.
fn mkdir_parents(f: &mut Forth) -> Res {
    let mode = f.pop()?;
    let name = f.pop_name()?;
    let ior = if f.permits(Permission::System) {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(u32::try_from(mode & 0o7777).unwrap_or(0o777));
        }
        #[cfg(not(unix))]
        let _ = mode;
        crate::files::ior(builder.create(&name).is_ok(), FILE_IO)
    } else {
        NOT_PERMITTED
    };
    f.push(ior);
    Ok(())
}

fn is_separator(b: u8) -> bool {
    b == b'/' || (cfg!(windows) && b == b'\\')
}

/// `BASENAME ( c-addr1 u1 -- c-addr2 u2 )`: the file name without the
/// directories before it.
fn basename(f: &mut Forth) -> Res {
    let (i, n) = f.pop_str()?;
    let start = f.mem[i..i + n].iter().rposition(|&b| is_separator(b)).map_or(0, |k| k + 1);
    f.push(to_cell(i + start));
    f.push(to_cell(n - start));
    Ok(())
}

/// `DIRNAME ( c-addr1 u1 -- c-addr1 u2 )`: the directory of the file name,
/// with its last `/`; empty if the name has none.
fn dirname(f: &mut Forth) -> Res {
    let (i, n) = f.pop_str()?;
    let len = f.mem[i..i + n].iter().rposition(|&b| is_separator(b)).map_or(0, |k| k + 1);
    f.push(to_cell(i));
    f.push(to_cell(len));
    Ok(())
}

/// `FILENAME-MATCH ( c-addr1 u1 c-addr2 u2 -- flag )`: whether the file name
/// matches the pattern, in which `*` matches any characters and `?` one.
fn filename_match(f: &mut Forth) -> Res {
    let pattern = f.pop_name()?;
    let name = f.pop_name()?;
    let (name, pattern): (Vec<char>, Vec<char>) =
        (name.chars().collect(), pattern.chars().collect());
    f.push(flag(wildcard(&name, &pattern)));
    Ok(())
}

fn wildcard(name: &[char], pattern: &[char]) -> bool {
    let (mut n, mut p) = (0, 0);
    let mut star: Option<(usize, usize)> = None; // the last `*`, and where it started to match
    while n < name.len() {
        if pattern.get(p) == Some(&'*') {
            star = Some((p, n));
            p += 1;
        } else if pattern.get(p).is_some_and(|&c| c == '?' || c == name[n]) {
            p += 1;
            n += 1;
        } else if let Some((sp, sn)) = star {
            (p, n) = (sp + 1, sn + 1);
            star = Some((sp, sn + 1));
        } else {
            return false;
        }
    }
    pattern[p..].iter().all(|&c| c == '*')
}

// ---- Streams ------------------------------------------------------------

/// A file that is only there to hold a file id: reading and writing are
/// done by `with_std`.
struct Placeholder;

impl Read for Placeholder {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Ok(0)
    }
}

impl Write for Placeholder {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for Placeholder {
    fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
        Err(not_for("repositioned"))
    }
}

impl FileHandle for Placeholder {
    fn size(&mut self) -> io::Result<u64> {
        Err(not_for("measured"))
    }

    fn set_size(&mut self, _: u64) -> io::Result<()> {
        Err(not_for("resized"))
    }
}

/// A standard stream for the FILE words: STDIN reads what KEY and ACCEPT
/// read, STDOUT writes what TYPE writes, STDERR writes to standard error.
struct StdHandle<'a> {
    kind: Std,
    input: &'a mut dyn Input,
    out: Vec<u8>,
}

impl Read for StdHandle<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.kind == Std::In { self.input.read(buf) } else { Err(not_for("read")) }
    }
}

impl Write for StdHandle<'_> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.kind == Std::In {
            return Err(not_for("written"));
        }
        self.out.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for StdHandle<'_> {
    fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
        Err(not_for("repositioned"))
    }
}

impl FileHandle for StdHandle<'_> {
    fn size(&mut self) -> io::Result<u64> {
        Err(not_for("measured"))
    }

    fn set_size(&mut self, _: u64) -> io::Result<()> {
        Err(not_for("resized"))
    }

    fn read_line(&mut self, max: usize) -> io::Result<Option<Vec<u8>>> {
        if self.kind == Std::In {
            crate::files::read_line_buffered(self.input, max)
        } else {
            Err(not_for("read"))
        }
    }
}

/// Runs a file operation on a standard stream (see `Forth::with_file`).
pub(crate) fn with_std<T>(
    f: &mut Forth,
    kind: Std,
    op: impl FnOnce(&mut dyn FileHandle) -> io::Result<T>,
) -> Option<T> {
    if kind == Std::In && f.interactive {
        f.flush();
    }
    let mut handle = StdHandle { kind, input: &mut *f.input, out: Vec::new() };
    let result = op(&mut handle);
    let out = handle.out;
    match kind {
        Std::Out => f.emit_bytes(&out),
        Std::Err => {
            if f.interactive {
                f.flush();
            }
            io::stderr().write_all(&out).ok();
        }
        Std::In => {}
    }
    result.ok()
}

/// `STDIN`, `STDOUT` and `STDERR` ( -- wfileid ). The file id is made the
/// first time it is asked for, and again if the program closed it.
fn std_word(f: &mut Forth, kind: Std) -> Res {
    if kind == Std::Err {
        allowed(f, || "STDERR".into())?;
    }
    let slot = kind as usize;
    let known = f.state::<Os>().std[slot];
    let known = known.filter(|&fid| f.open_file(fid).is_some_and(|o| o.std == Some(kind)));
    let fid = if let Some(fid) = known {
        fid
    } else {
        let name = ["stdin", "stdout", "stderr"][slot];
        let fid = f.add_file(Box::new(Placeholder), name);
        if let Some(file) = f.open_file_mut(fid) {
            file.std = Some(kind);
        }
        f.state::<Os>().std[slot] = Some(fid);
        fid
    };
    f.push(fid);
    Ok(())
}

/// `OUTFILE-EXECUTE ( ... xt wfileid -- ... )`: what xt types goes to the file.
fn outfile_execute(f: &mut Forth) -> Res {
    let fid = f.pop()?;
    let xt = pop_xt(f, "OUTFILE-EXECUTE")?;
    match f.open_file(fid) {
        None => return Err(Throw::with(FILE_IO, "OUTFILE-EXECUTE: invalid file id")),
        Some(file) if file.std == Some(Std::Out) => return f.execute(xt),
        Some(_) => {}
    }
    let start = f.out.len();
    let ran = f.execute(xt);
    let text = f.out.split_off(start.min(f.out.len()));
    let written = f.with_file(fid, |file| file.write_all(&text));
    ran?;
    written.ok_or_else(|| Throw::with(FILE_IO, "OUTFILE-EXECUTE: cannot write to the file"))
}

/// What a file gave to INFILE-EXECUTE, with a count of what was used.
struct Rest {
    data: Vec<u8>,
    used: Rc<Counter<usize>>,
}

impl Read for Rest {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let n = (&self.data[self.used.get()..]).read(out)?;
        self.used.set(self.used.get() + n);
        Ok(n)
    }
}

impl BufRead for Rest {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        Ok(&self.data[self.used.get()..])
    }

    fn consume(&mut self, n: usize) {
        self.used.set((self.used.get() + n).min(self.data.len()));
    }
}

impl Input for Rest {}

/// `INFILE-EXECUTE ( ... xt wfileid -- ... )`: what xt reads with KEY and
/// ACCEPT comes from the file. The rest of the file is read first; what xt
/// did not use is given back by repositioning the file (not possible for a pipe).
fn infile_execute(f: &mut Forth) -> Res {
    let fid = f.pop()?;
    let xt = pop_xt(f, "INFILE-EXECUTE")?;
    match f.open_file(fid) {
        None => return Err(Throw::with(FILE_IO, "INFILE-EXECUTE: invalid file id")),
        Some(file) if file.std == Some(Std::In) => return f.execute(xt),
        Some(_) => {}
    }
    let start = f.with_file(fid, |file| file.stream_position()).filter(|_| true);
    let mut data = Vec::new();
    f.with_file(fid, |file| file.read_to_end(&mut data))
        .ok_or_else(|| Throw::with(FILE_IO, "INFILE-EXECUTE: cannot read the file"))?;
    f.charge(data.len())?;
    let used = Rc::new(Counter::new(0));
    let rest = Rest { data, used: Rc::clone(&used) };
    let terminal = std::mem::replace(&mut f.input, Box::new(rest));
    let ran = f.execute(xt);
    f.input = terminal;
    if let Some(start) = start {
        let at = start + used.get() as u64;
        f.with_file(fid, |file| file.seek(SeekFrom::Start(at)));
    }
    ran
}
