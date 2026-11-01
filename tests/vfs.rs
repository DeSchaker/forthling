//! The file system behind the FILE words (vfs.rs), through the public API.

use forthling::{Access, FileSystem, Forth, MemoryFs, Permission, Throw};
use std::{env, fs, io};

fn output(f: &Forth) -> String {
    String::from_utf8_lossy(f.output()).into_owned()
}

fn fails(f: &mut Forth, text: &str) -> Throw {
    f.interpret(text).expect_err(text)
}

/// A system on a file system in memory, and a clone of it for the test.
fn in_memory(limit: usize) -> (Forth, MemoryFs) {
    let vfs = MemoryFs::with_limit(limit);
    let mut f = Forth::new();
    f.set_input(Box::new(io::Cursor::new(Vec::new())));
    f.set_file_system(Box::new(vfs.clone())).unwrap();
    (f, vfs)
}

#[test]
fn a_program_writes_and_reads_files_in_memory() {
    let (mut f, vfs) = in_memory(usize::MAX);
    f.interpret(
        "VARIABLE FID \
         S\" notes/today.txt\" W/O CREATE-FILE . FID ! \
         S\" first line\" FID @ WRITE-LINE . \
         S\" second\" FID @ WRITE-FILE . \
         FID @ CLOSE-FILE .",
    )
    .unwrap();
    assert_eq!(output(&f), "0 0 0 0 ");
    assert_eq!(vfs.get("notes/today.txt").unwrap(), b"first line\nsecond");
    // The host sees it under every spelling of the path.
    assert_eq!(vfs.get("./notes/x/../today.txt"), vfs.get("notes\\today.txt"));
    assert_eq!(vfs.paths(), ["notes/today.txt"]);
    assert_eq!(vfs.used(), 17);
    f.interpret(
        "CREATE BUF 80 ALLOT \
         S\" /notes/today.txt\" R/O OPEN-FILE . FID ! \
         BUF 80 FID @ READ-LINE . . BUF SWAP TYPE \
         FID @ FILE-SIZE . D. \
         FID @ CLOSE-FILE .",
    )
    .unwrap();
    assert_eq!(output(&f)[8..], *"0 0 -1 first line0 17 0 ");
}

#[test]
fn rename_delete_and_status() {
    let (mut f, vfs) = in_memory(usize::MAX);
    vfs.insert("a.txt", "hello").unwrap();
    f.interpret("S\" a.txt\" FILE-STATUS . DROP S\" b.txt\" FILE-STATUS . DROP").unwrap();
    assert_eq!(output(&f), "0 -67 ");
    f.interpret("S\" a.txt\" S\" b.txt\" RENAME-FILE .").unwrap();
    assert_eq!(vfs.paths(), ["b.txt"]);
    f.interpret("S\" b.txt\" DELETE-FILE . S\" b.txt\" DELETE-FILE .").unwrap();
    assert!(output(&f).ends_with("0 0 -64 "), "{}", output(&f));
    assert!(vfs.paths().is_empty());
    assert_eq!(vfs.used(), 0);
    // A file opened for reading cannot be written.
    vfs.insert("c.txt", "x").unwrap();
    f.interpret("S\" c.txt\" R/O OPEN-FILE DROP S\" y\" ROT WRITE-FILE .").unwrap();
    assert!(output(&f).ends_with("-75 "), "{}", output(&f));
}

#[test]
fn include_and_require_find_files_in_memory() {
    let (mut f, vfs) = in_memory(usize::MAX);
    vfs.insert("lib/square.fs", ": SQUARE DUP * ;  .( square loaded) CR").unwrap();
    // A file includes its neighbour by a relative name.
    vfs.insert("lib/main.fs", "REQUIRE square.fs  7 SQUARE .").unwrap();
    f.interpret("INCLUDE lib/main.fs").unwrap();
    f.interpret("S\" lib/./square.fs\" REQUIRED  S\" lib/main.fs\" INCLUDED").unwrap();
    assert_eq!(output(&f), "square loaded\n49 49 ");
    // The host's include reads the same files.
    f.include("lib/main.fs").unwrap();
    let e = f.include("missing.fs").unwrap_err();
    assert_eq!(e.to_string(), "error -38: non-existent file: missing.fs: no such file");
    // An error in a file in memory gives its name and line.
    vfs.insert("bad.fs", "1 2 +\n: X NOPE ;").unwrap();
    let e = fails(&mut f, "INCLUDE bad.fs");
    let place = e.context().unwrap();
    assert_eq!((place.source.as_str(), place.line, place.token.as_str()), ("bad.fs", 2, "NOPE"));
}

#[test]
fn blocks_live_in_the_file_system() {
    let (mut f, vfs) = in_memory(usize::MAX);
    f.interpret("2 BLOCK 1024 BLANK S\" 6 7 * .\" 2 BLOCK SWAP MOVE UPDATE SAVE-BUFFERS").unwrap();
    assert_eq!(vfs.get("blocks.fb").unwrap().len(), 2048);
    f.interpret("EMPTY-BUFFERS 2 LOAD").unwrap();
    assert_eq!(output(&f), "42 ");
}

#[test]
fn the_limit_holds() {
    let (mut f, vfs) = in_memory(100);
    f.interpret(
        "S\" big\" W/O CREATE-FILE DROP CONSTANT FID \
         : FILL-UP 0 BEGIN DUP 200 < WHILE S\" 0123456789\" FID WRITE-FILE ?DUP IF . DROP EXIT THEN 10 + REPEAT DROP ; \
         FILL-UP  FID FILE-SIZE DROP D.  FID CLOSE-FILE .",
    )
    .unwrap();
    assert_eq!(output(&f), "-75 100 0 ");
    assert_eq!(vfs.used(), 100);
    assert!(vfs.insert("more", "x").is_err());
    // Space comes back when a file is deleted, also one that was open.
    f.interpret("S\" big\" R/O OPEN-FILE DROP S\" big\" DELETE-FILE . CONSTANT OPEN").unwrap();
    assert_eq!(vfs.used(), 100);
    f.interpret("OPEN CLOSE-FILE .").unwrap();
    assert_eq!(vfs.used(), 0);
    vfs.insert("more", "x").unwrap();
}

#[test]
fn a_sandboxed_program_has_files_only_in_memory() {
    // A file on disk that the sandbox must not see.
    let disk = env::temp_dir().join(format!("forthling-vfs-{}.txt", std::process::id()));
    fs::write(&disk, "secret").unwrap();
    let disk = disk.to_string_lossy().into_owned();
    let mut f = Forth::sandboxed();
    assert!(f.permits(Permission::Files));
    // Extensions that open files themselves (SQLite) may not.
    assert!(!f.disk_files());
    assert!(Forth::new().disk_files());
    f.interpret(&format!("S\" {disk}\" R/O OPEN-FILE . DROP")).unwrap();
    assert_eq!(output(&f), "-69 ");
    f.interpret(&format!("S\" {disk}\" DELETE-FILE .")).unwrap();
    assert!(fs::metadata(&disk).is_ok());
    assert_eq!(fails(&mut f, &format!("S\" {disk}\" INCLUDED")).code, -38);
    // What it writes stays in memory, up to the sandbox's limit.
    f.interpret("S\" out.txt\" W/O CREATE-FILE . S\" hi\" ROT WRITE-FILE .").unwrap();
    assert!(output(&f).ends_with("0 0 "), "{}", output(&f));
    let resize =
        |size: usize| format!("S\" out.txt\" R/W OPEN-FILE DROP {size} 0 ROT RESIZE-FILE .");
    f.interpret(&resize(MemoryFs::SANDBOX_LIMIT)).unwrap();
    assert!(output(&f).ends_with(" 0 "), "{}", output(&f));
    f.interpret(&resize(MemoryFs::SANDBOX_LIMIT + 1)).unwrap();
    assert!(output(&f).ends_with("-74 "), "{}", output(&f));
    fs::remove_file(&disk).unwrap();
}

#[test]
fn files_can_still_be_forbidden() {
    let (mut f, vfs) = in_memory(usize::MAX);
    vfs.insert("a.fs", "1").unwrap();
    f.permit(Permission::Files, false);
    f.interpret("S\" a.fs\" R/O OPEN-FILE . DROP").unwrap();
    assert_eq!(output(&f), "-259 ");
    assert_eq!(fails(&mut f, "INCLUDE a.fs").code, -259);
}

#[test]
fn a_host_file_system_of_its_own() {
    // Only reads, from a fixed set of files.
    struct Fixed;
    impl FileSystem for Fixed {
        fn open(
            &mut self,
            path: &str,
            access: Access,
        ) -> io::Result<Box<dyn forthling::FileHandle>> {
            match (path, access.write) {
                ("hello.fs", false) => Ok(Box::new(io::Cursor::new(b".( hello)".to_vec()))),
                _ => Err(io::Error::new(io::ErrorKind::NotFound, "only hello.fs")),
            }
        }
        fn delete(&mut self, _: &str) -> io::Result<()> {
            Err(io::Error::other("read only"))
        }
        fn rename(&mut self, _: &str, _: &str) -> io::Result<()> {
            Err(io::Error::other("read only"))
        }
        fn exists(&self, path: &str) -> bool {
            path == "hello.fs"
        }
    }
    let mut f = Forth::new();
    f.set_file_system(Box::new(Fixed)).unwrap();
    f.interpret("INCLUDE hello.fs").unwrap();
    assert_eq!(output(&f), "hello");
}

/// The example of the README's section Files.
#[test]
fn the_readme_example() {
    let files = MemoryFs::new();
    files.insert("square.fs", ": SQUARE DUP * ;").unwrap();
    let mut forth = Forth::sandboxed();
    forth.set_file_system(Box::new(files.clone())).unwrap();
    forth.interpret("INCLUDE square.fs  S\" out.txt\" W/O CREATE-FILE THROW VALUE OUT").unwrap();
    forth.interpret("7 SQUARE 0 <# #S #> OUT WRITE-FILE THROW  OUT CLOSE-FILE THROW").unwrap();
    assert_eq!(files.get("out.txt").unwrap(), b"49");
}
