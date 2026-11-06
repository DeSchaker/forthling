//! The programs in `examples/` (the cookbook's examples too), run as a process, each with the
//! output it is expected to give: a file `name.out` beside `name.fs`. A program can
//! only be added together with its output, so no example can go stale.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The `.fs` files of a directory, in name order.
fn programs(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "fs"))
        .collect();
    files.sort();
    files
}

/// Runs `program` with the forthling program, in a directory of its own (some
/// examples write files), and returns what it prints on stdout and stderr.
fn run(program: &Path) -> String {
    let dir = std::env::temp_dir().join(format!(
        "forthling-examples-{}-{}",
        std::process::id(),
        program.file_stem().unwrap().to_string_lossy()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_forthling"))
        .arg(program)
        .current_dir(&dir)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    std::fs::remove_dir_all(&dir).ok();
    assert!(out.status.success(), "{}: {}", program.display(), out.status);
    String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr)
}

#[test]
fn every_example_gives_the_output_beside_it() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut seen = 0;
    for program in programs(&root.join("examples")) {
        let expected =
            std::fs::read_to_string(program.with_extension("out")).unwrap_or_else(|_| {
                panic!("{} has no .out file with its expected output", program.display())
            });
        assert_eq!(run(&program), expected, "{}", program.display());
        seen += 1;
    }
    assert!(seen >= 2, "only {seen} examples found");
}
