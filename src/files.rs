//! Files: file ids, INCLUDED and REQUIRED, and helpers for the FILE words.

use crate::*;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

// File access methods, as R/O and W/O in the prelude; R/W is 2 and BIN adds 4,
// which changes nothing.
pub(crate) const R_O: Cell = 0;
pub(crate) const W_O: Cell = 1;

/// A file opened by OPEN-FILE, CREATE-FILE or INCLUDED; file id n is `files[n - 1]`.
pub(crate) struct OpenFile {
    pub(crate) file: fs::File,
    pub(crate) name: String,
}

/// The name under which REQUIRED remembers a file.
pub(crate) fn canonical(path: &str) -> String {
    fs::canonicalize(path).map_or_else(|_| path.to_string(), |p| p.to_string_lossy().into_owned())
}

/// The ior of a file word: 0 on success, else the word's THROW code.
pub(crate) fn ior<T>(result: Option<T>, code: Cell) -> Cell {
    if result.is_some() { 0 } else { code }
}

/// The core of READ-LINE: reads at most `max` characters of the next line,
/// without its line ending. Returns None at the end of the file. A line
/// longer than `max` is continued by the next read.
pub(crate) fn read_line_from(file: &mut fs::File, max: usize) -> io::Result<Option<Vec<u8>>> {
    let mut chunk = Vec::new();
    file.take(max as u64 + 2).read_to_end(&mut chunk)?;
    if chunk.is_empty() {
        return Ok(None);
    }
    let limit = chunk.len().min(max);
    let (len, used) = match chunk[..chunk.len().min(max + 1)].iter().position(|&c| c == b'\n') {
        Some(i) if i > 0 && chunk[i - 1] == b'\r' => (i - 1, i + 1),
        Some(i) => (i, i + 1),
        // A CR LF just past a full buffer still ends the line.
        None if chunk[limit..].starts_with(b"\r\n") => (limit, limit + 2),
        None => (limit, limit),
    };
    file.seek(SeekFrom::Current(used as i64 - chunk.len() as i64))?;
    chunk.truncate(len);
    Ok(Some(chunk))
}

impl Forth {
    /// INCLUDED: opens a file, remembers it for REQUIRED and interprets it.
    pub fn include(&mut self, path: &str) -> Res {
        let path = self.resolve(path);
        let file =
            fs::File::open(&path).map_err(|e| Throw::with(NO_SUCH_FILE, format!("{path}: {e}")))?;
        self.included.insert(canonical(&path));
        let fid = self.add_file(file, &path);
        self.include_file(fid)
    }

    /// REQUIRED: includes a file unless it was included before.
    pub(crate) fn require(&mut self, path: &str) -> Res {
        let path = self.resolve(path);
        if self.included.contains(&canonical(&path)) {
            return Ok(());
        }
        self.include(&path)
    }

    /// A relative path that does not exist is looked up next to the file
    /// being included, so a file can include its neighbours.
    pub(crate) fn resolve(&self, path: &str) -> String {
        if Path::new(path).exists() {
            return path.to_string();
        }
        let including = self.sources.iter().rev().find_map(|s| match s.origin {
            Origin::File { fid, .. } => self.open_file(fid),
            _ => None,
        });
        let beside = including.and_then(|f| Path::new(&f.name).parent()).map(|dir| dir.join(path));
        match beside {
            Some(p) if p.exists() => p.to_string_lossy().into_owned(),
            _ => path.to_string(),
        }
    }

    /// INCLUDE-FILE: interprets the rest of an open file, then closes it.
    pub(crate) fn include_file(&mut self, fid: Cell) -> Res {
        let Some(f) = self.open_file_mut(fid) else {
            return Err(Throw::with(FILE_IO, "invalid file id"));
        };
        let name = f.name.clone();
        let mut bytes = Vec::new();
        let result = match f.file.read_to_end(&mut bytes) {
            Ok(_) => self.include_lines(fid, &name, &bytes),
            Err(e) => Err(Throw::with(FILE_IO, format!("{name}: {e}"))),
        };
        self.close_file(fid);
        result
    }

    /// Interprets the lines of a file as a nested input source with its own
    /// line buffer.
    pub(crate) fn include_lines(&mut self, fid: Cell, path: &str, bytes: &[u8]) -> Res {
        let lines = String::from_utf8_lossy(bytes).lines().map(String::from).collect();
        let buf = SOURCE_AREA + self.sources.len() * LINE_MAX;
        self.push_source(Origin::File { fid, lines: Rc::new(lines) }, buf, 0)?;
        // Read the line number before interpreting: a nested source may be
        // on top of the stack when an error comes back.
        let line = |f: &Forth| f.sources.last().unwrap().line;
        while self.refill().map_err(|e| e.at(path, line(self)))? {
            let n = line(self);
            self.interpret_buffer().map_err(|e| e.at(path, n))?;
            if self.bye {
                break;
            }
        }
        self.pop_source();
        Ok(())
    }

    /// Registers an open file and returns its file id, reusing a free one.
    pub(crate) fn add_file(&mut self, file: fs::File, name: &str) -> Cell {
        let entry = Some(OpenFile { file, name: name.to_string() });
        match self.files.iter().position(Option::is_none) {
            Some(i) => {
                self.files[i] = entry;
                i as Cell + 1
            }
            None => {
                self.files.push(entry);
                self.files.len() as Cell
            }
        }
    }

    pub(crate) fn open_file(&self, fid: Cell) -> Option<&OpenFile> {
        let i = usize::try_from(fid.checked_sub(1)?).ok()?;
        self.files.get(i)?.as_ref()
    }

    pub(crate) fn open_file_mut(&mut self, fid: Cell) -> Option<&mut OpenFile> {
        let i = usize::try_from(fid.checked_sub(1)?).ok()?;
        self.files.get_mut(i)?.as_mut()
    }

    /// Closes a file; false if the file id is not open.
    pub(crate) fn close_file(&mut self, fid: Cell) -> bool {
        let open = self.open_file(fid).is_some();
        if open {
            self.files[fid as usize - 1] = None;
        }
        open
    }

    /// Runs a file operation; None for an unknown file id or an I/O error.
    pub(crate) fn with_file<T>(
        &mut self,
        fid: Cell,
        op: impl FnOnce(&mut fs::File) -> io::Result<T>,
    ) -> Option<T> {
        op(&mut self.open_file_mut(fid)?.file).ok()
    }

    /// Pushes a file position or size as ( ud ior ).
    pub(crate) fn push_size(&mut self, size: Option<u64>, code: Cell) {
        self.push_ud(size.unwrap_or(0) as u128);
        self.push(ior(size, code));
    }

    /// Pops a file name ( c-addr u ).
    pub(crate) fn pop_name(&mut self) -> Res<String> {
        let (i, n) = self.pop_str()?;
        Ok(self.text(i, n))
    }

    /// Opens a file for OPEN-FILE and CREATE-FILE and pushes ( fileid ior ).
    pub(crate) fn open_with(&mut self, name: &str, fam: Cell, create: bool, code: Cell) {
        let mut options = fs::OpenOptions::new();
        let access = fam & 3;
        options.read(access != W_O).write(access != R_O || create);
        if create {
            options.create(true).truncate(true);
        }
        match options.open(name) {
            Ok(file) => {
                let fid = self.add_file(file, name);
                self.push(fid);
                self.push(0);
            }
            Err(_) => {
                self.push(0);
                self.push(code);
            }
        }
    }
}
