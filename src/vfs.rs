//! The file system behind the FILE words, INCLUDE and the block file: a
//! trait the host may implement. `NativeFs` is the disk (the default);
//! `MemoryFs` keeps files in memory, for the sandbox.

use crate::*;
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

/// How a file is opened.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools, reason = "independent flags, as in std::fs::OpenOptions")]
pub struct Access {
    pub read: bool,
    pub write: bool,
    /// Create the file if it does not exist.
    pub create: bool,
    /// Empty the file if it exists.
    pub truncate: bool,
}

/// An open file.
pub trait FileHandle: Read + Write + Seek {
    /// The size of the file in bytes.
    fn size(&mut self) -> io::Result<u64>;
    /// Makes the file `size` bytes long, cutting it off or adding zeros.
    fn set_size(&mut self, size: u64) -> io::Result<()>;
}

/// Where the files of a Forth program are. Paths are the text the program
/// gives; what they mean is up to the file system.
pub trait FileSystem {
    fn open(&mut self, path: &str, access: Access) -> io::Result<Box<dyn FileHandle>>;
    fn delete(&mut self, path: &str) -> io::Result<()>;
    fn rename(&mut self, from: &str, to: &str) -> io::Result<()>;
    fn exists(&self, path: &str) -> bool;
    /// The name under which REQUIRED remembers a file, the same for every
    /// path to it.
    fn canonical(&self, path: &str) -> String {
        path.to_string()
    }
    /// Whether these are the files of the computer itself. Extensions that
    /// reach files without the file system, such as `SQLite`, use the disk
    /// only when it is (see `Forth::disk_files`).
    fn is_disk(&self) -> bool {
        false
    }
}

impl FileHandle for fs::File {
    fn size(&mut self) -> io::Result<u64> {
        Ok(self.metadata()?.len())
    }

    fn set_size(&mut self, size: u64) -> io::Result<()> {
        self.set_len(size)
    }
}

/// A file that is only bytes in memory, for a file system of the host's
/// own.
impl FileHandle for io::Cursor<Vec<u8>> {
    fn size(&mut self) -> io::Result<u64> {
        Ok(self.get_ref().len() as u64)
    }

    fn set_size(&mut self, size: u64) -> io::Result<()> {
        let size = usize::try_from(size).map_err(|_| io::Error::other("file too large"))?;
        self.get_mut().resize(size, 0);
        Ok(())
    }
}

/// The files of the computer, through `std::fs`. In WebAssembly every
/// operation fails.
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeFs;

impl FileSystem for NativeFs {
    fn open(&mut self, path: &str, access: Access) -> io::Result<Box<dyn FileHandle>> {
        let file = fs::OpenOptions::new()
            .read(access.read)
            .write(access.write)
            .create(access.create)
            .truncate(access.truncate)
            .open(path)?;
        Ok(Box::new(file))
    }

    fn delete(&mut self, path: &str) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn rename(&mut self, from: &str, to: &str) -> io::Result<()> {
        fs::rename(from, to)
    }

    fn exists(&self, path: &str) -> bool {
        Path::new(path).exists()
    }

    fn is_disk(&self) -> bool {
        true
    }

    fn canonical(&self, path: &str) -> String {
        fs::canonicalize(path)
            .map_or_else(|_| path.to_string(), |p| p.to_string_lossy().into_owned())
    }
}

type Contents = Rc<RefCell<Vec<u8>>>;

struct Store {
    files: BTreeMap<String, Contents>,
    used: usize, // bytes of all files, and of deleted ones still open
    limit: usize,
}

impl Store {
    /// Takes `more` bytes of the limit.
    fn grow(&mut self, more: usize) -> io::Result<()> {
        match self.used.checked_add(more) {
            Some(used) if used <= self.limit => {
                self.used = used;
                Ok(())
            }
            _ => Err(io::Error::other("the file system is full")),
        }
    }
}

/// Files in memory, with no directories of their own: a path is a name, in
/// which `/` and `\` both separate parts and `.` and `..` work as usual, so
/// `lib/../a.fs`, `./a.fs` and `/a.fs` are the same file `a.fs`. A clone
/// shares the files, so the host can keep one to put files in and read
/// them back while the Forth program uses another.
#[derive(Clone)]
pub struct MemoryFs(Rc<RefCell<Store>>);

impl Default for MemoryFs {
    fn default() -> Self {
        MemoryFs::new()
    }
}

impl MemoryFs {
    /// The limit of `Forth::sandboxed`: 4 MiB in all.
    pub const SANDBOX_LIMIT: usize = 4 << 20;

    /// An empty file system without a limit.
    #[must_use]
    pub fn new() -> Self {
        MemoryFs::with_limit(usize::MAX)
    }

    /// An empty file system that holds at most `limit` bytes in all; a
    /// write past it fails.
    #[must_use]
    pub fn with_limit(limit: usize) -> Self {
        MemoryFs(Rc::new(RefCell::new(Store { files: BTreeMap::new(), used: 0, limit })))
    }

    /// Puts a file in, replacing one of the same name; fails when it does
    /// not fit.
    pub fn insert(&self, path: &str, contents: impl Into<Vec<u8>>) -> io::Result<()> {
        let contents = contents.into();
        let mut store = self.0.borrow_mut();
        let path = normalize(path)?;
        let old = store.files.get(&path).map_or(0, |c| c.borrow().len());
        store.used -= old;
        if let Err(e) = store.grow(contents.len()) {
            store.used += old;
            return Err(e);
        }
        match store.files.get(&path) {
            // A file that is open sees the new contents.
            Some(c) => *c.borrow_mut() = contents,
            None => {
                store.files.insert(path, Rc::new(RefCell::new(contents)));
            }
        }
        Ok(())
    }

    /// Removes a file, as DELETE-FILE does.
    pub fn remove(&self, path: &str) -> io::Result<()> {
        self.clone().delete(path)
    }

    /// The contents of a file.
    #[must_use]
    pub fn get(&self, path: &str) -> Option<Vec<u8>> {
        let path = normalize(path).ok()?;
        self.0.borrow().files.get(&path).map(|c| c.borrow().clone())
    }

    /// The names of the files, sorted.
    #[must_use]
    pub fn paths(&self) -> Vec<String> {
        self.0.borrow().files.keys().cloned().collect()
    }

    /// The bytes all files take, of the limit.
    #[must_use]
    pub fn used(&self) -> usize {
        self.0.borrow().used
    }
}

/// A path as the name of a file in a `MemoryFs`.
fn normalize(path: &str) -> io::Result<String> {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    if parts.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "no file name"));
    }
    Ok(parts.join("/"))
}

fn not_found() -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, "no such file")
}

impl FileSystem for MemoryFs {
    fn open(&mut self, path: &str, access: Access) -> io::Result<Box<dyn FileHandle>> {
        let path = normalize(path)?;
        let mut store = self.0.borrow_mut();
        let contents = match store.files.get(&path) {
            Some(c) => Rc::clone(c),
            None if access.create => {
                let c = Contents::default();
                store.files.insert(path, Rc::clone(&c));
                c
            }
            None => return Err(not_found()),
        };
        drop(store);
        let mut handle = MemoryFile { fs: self.clone(), contents, pos: 0, access };
        if access.truncate {
            handle.set_size(0)?;
        }
        Ok(Box::new(handle))
    }

    fn delete(&mut self, path: &str) -> io::Result<()> {
        let mut store = self.0.borrow_mut();
        let contents = store.files.remove(&normalize(path)?).ok_or_else(not_found)?;
        // A file still open keeps its bytes until it is closed.
        if Rc::strong_count(&contents) == 1 {
            store.used -= contents.borrow().len();
        }
        Ok(())
    }

    fn rename(&mut self, from: &str, to: &str) -> io::Result<()> {
        let (from, to) = (normalize(from)?, normalize(to)?);
        let mut store = self.0.borrow_mut();
        let contents = store.files.remove(&from).ok_or_else(not_found)?;
        let replaced = store.files.insert(to, contents);
        if let Some(old) = replaced.filter(|old| Rc::strong_count(old) == 1) {
            store.used -= old.borrow().len();
        }
        Ok(())
    }

    fn exists(&self, path: &str) -> bool {
        normalize(path).is_ok_and(|p| self.0.borrow().files.contains_key(&p))
    }

    fn canonical(&self, path: &str) -> String {
        normalize(path).unwrap_or_else(|_| path.to_string())
    }
}

/// An open file of a `MemoryFs`.
struct MemoryFile {
    fs: MemoryFs,
    contents: Contents,
    pos: u64,
    access: Access,
}

fn denied(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, format!("not opened for {what}"))
}

impl MemoryFile {
    /// Whether the file is still in the file system, not deleted while open.
    fn listed(&self) -> bool {
        self.fs.0.borrow().files.values().any(|c| Rc::ptr_eq(c, &self.contents))
    }
}

impl Drop for MemoryFile {
    fn drop(&mut self) {
        // The last handle of a deleted file gives its bytes back.
        if Rc::strong_count(&self.contents) == 1 && !self.listed() {
            self.fs.0.borrow_mut().used -= self.contents.borrow().len();
        }
    }
}

impl Read for MemoryFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if !self.access.read {
            return Err(denied("reading"));
        }
        let contents = self.contents.borrow();
        let start = usize::try_from(self.pos).unwrap_or(usize::MAX).min(contents.len());
        let n = buf.len().min(contents.len() - start);
        buf[..n].copy_from_slice(&contents[start..start + n]);
        self.pos += n as u64;
        Ok(n)
    }
}

impl Write for MemoryFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if !self.access.write {
            return Err(denied("writing"));
        }
        let start = usize::try_from(self.pos).map_err(|_| io::Error::other("file too large"))?;
        let end = start.checked_add(buf.len()).ok_or_else(|| io::Error::other("file too large"))?;
        let len = self.contents.borrow().len();
        if end > len {
            self.fs.0.borrow_mut().grow(end - len)?;
            self.contents.borrow_mut().resize(end, 0);
        }
        self.contents.borrow_mut()[start..end].copy_from_slice(buf);
        self.pos = end as u64;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for MemoryFile {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let (base, offset) = match to {
            SeekFrom::Start(n) => (0, i128::from(n)),
            SeekFrom::End(n) => (self.contents.borrow().len() as i128, i128::from(n)),
            SeekFrom::Current(n) => (i128::from(self.pos), i128::from(n)),
        };
        let pos = u64::try_from(base + offset)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "before the start"))?;
        self.pos = pos;
        Ok(pos)
    }
}

impl FileHandle for MemoryFile {
    fn size(&mut self) -> io::Result<u64> {
        Ok(self.contents.borrow().len() as u64)
    }

    fn set_size(&mut self, size: u64) -> io::Result<()> {
        if !self.access.write {
            return Err(denied("writing"));
        }
        let size = usize::try_from(size).map_err(|_| io::Error::other("file too large"))?;
        let len = self.contents.borrow().len();
        let mut store = self.fs.0.borrow_mut();
        if size > len {
            store.grow(size - len)?;
        } else {
            store.used -= len - size;
        }
        drop(store);
        self.contents.borrow_mut().resize(size, 0);
        Ok(())
    }
}

impl Forth {
    /// Uses another file system from now on, for the FILE words, INCLUDE,
    /// the host's `include` and the block file. The block buffers are
    /// written back to the old one first; files that are open stay open on
    /// it. `Permission::Files` still decides whether the program may use
    /// files at all.
    pub fn set_file_system(&mut self, fs: Box<dyn FileSystem>) -> Result<(), Throw> {
        self.save_buffers()?;
        self.empty_buffers();
        self.blocks.file = None;
        self.vfs = fs;
        Ok(())
    }

    /// Whether the program may use the files of the computer: it has
    /// `Permission::Files` and the file system is the disk. For extensions
    /// that open files themselves, not through the file system.
    #[must_use]
    pub fn disk_files(&self) -> bool {
        self.permits(Permission::Files) && self.vfs.is_disk()
    }
}
