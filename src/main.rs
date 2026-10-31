//! forthling: runs the Forth files given as arguments, or else a REPL.

use forthling::{Forth, DEFAULT_MEMORY, MAX_MEMORY, MIN_MEMORY, QUIT};
use std::path::PathBuf;
use std::{env, fs};

const USAGE: &str = "\
usage: forthling [options] [file ...]

Runs the files in order, or else starts a REPL.

options:
  -m, --memory SIZE  memory for data space and heap, such as 4M (default 1M)
  -h, --help         show this help

SIZE is a number of bytes with an optional K, M or G (binary units).
A config file can set the memory with a line such as `memory = 4M`:
";

fn main() {
    let (memory, files) = match options(env::args().skip(1)) {
        Ok(Some(options)) => options,
        Ok(None) => return,
        Err(e) => {
            eprintln!("forthling: {e}");
            std::process::exit(2);
        }
    };
    // The main thread's stack is small (1 MiB on Windows); nested CATCHes
    // need more, up to 1000 deep.
    let interpreter =
        std::thread::Builder::new().stack_size(64 << 20).spawn(move || run_forth(memory, &files));
    let code = interpreter.expect("cannot start the interpreter thread").join().unwrap_or(101);
    std::process::exit(code);
}

/// The memory size and the files to run. The size comes from the command
/// line, else the config file, else the default. None after --help.
fn options(mut args: impl Iterator<Item = String>) -> Result<Option<(usize, Vec<String>)>, String> {
    let (mut memory, mut files) = (None, Vec::new());
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-m" | "--memory" => memory = Some(args.next().ok_or(format!("{arg} needs a size"))?),
            a if a.starts_with("--memory=") => memory = Some(a["--memory=".len()..].to_string()),
            "-h" | "--help" => {
                print!("{USAGE}");
                println!(
                    "  {}",
                    config_path().map_or("(none)".into(), |p| p.display().to_string())
                );
                return Ok(None);
            }
            "--" => files.extend(args.by_ref()),
            a if a.starts_with('-') && a.len() > 1 => {
                return Err(format!("unknown option {a} (see --help)"));
            }
            _ => files.push(arg),
        }
    }
    let memory = match memory {
        Some(size) => parse_size(&size)?,
        None => match config_memory()? {
            Some(size) => parse_size(&size)?,
            None => DEFAULT_MEMORY,
        },
    };
    Ok(Some((memory, files)))
}

/// The config file: forthling/forthling.conf in the user's config directory
/// (%APPDATA% on Windows, else $XDG_CONFIG_HOME or ~/.config).
fn config_path() -> Option<PathBuf> {
    let dir = if cfg!(windows) {
        PathBuf::from(env::var_os("APPDATA")?)
    } else if let Some(dir) = env::var_os("XDG_CONFIG_HOME").filter(|d| !d.is_empty()) {
        PathBuf::from(dir)
    } else {
        PathBuf::from(env::var_os("HOME")?).join(".config")
    };
    Some(dir.join("forthling").join("forthling.conf"))
}

/// The memory setting of the config file, if there is one.
fn config_memory() -> Result<Option<String>, String> {
    let Some(path) = config_path() else { return Ok(None) };
    let Ok(text) = fs::read_to_string(&path) else { return Ok(None) };
    parse_config(&text).map_err(|e| format!("{}:{e}", path.display()))
}

/// Reads `key = value` lines; `#` starts a comment. Returns the memory
/// value, or the line number and the problem.
fn parse_config(text: &str) -> Result<Option<String>, String> {
    let mut memory = None;
    for (i, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap().trim();
        if line.is_empty() {
            continue;
        }
        match line.split_once('=').map(|(k, v)| (k.trim(), v.trim())) {
            Some(("memory", value)) => memory = Some(value.to_string()),
            Some((key, _)) => return Err(format!("{}: unknown setting {key}", i + 1)),
            None => return Err(format!("{}: expected `name = value`", i + 1)),
        }
    }
    Ok(memory)
}

/// A memory size such as 65536, 512K, 4M or 1G.
fn parse_size(s: &str) -> Result<usize, String> {
    let invalid = || format!("invalid memory size {s}");
    let s = s.trim();
    let digits = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    let shift = match s[digits..].to_ascii_uppercase().as_str() {
        "" | "B" => 0,
        "K" | "KB" | "KIB" => 10,
        "M" | "MB" | "MIB" => 20,
        "G" | "GB" | "GIB" => 30,
        _ => return Err(invalid()),
    };
    let n: usize = s[..digits].parse().map_err(|_| invalid())?;
    let size = n.checked_mul(1 << shift).ok_or_else(invalid)?;
    if !(MIN_MEMORY..=MAX_MEMORY).contains(&size) {
        return Err(format!("memory size {s} is not between 64K and 4G"));
    }
    if size % 8 != 0 {
        return Err(format!("memory size {s} is not a multiple of 8 bytes"));
    }
    Ok(size)
}

/// Runs the files, or else the REPL; returns the exit code.
fn run_forth(memory: usize, files: &[String]) -> i32 {
    let mut forth = Forth::with_memory(memory);
    forth.set_interactive(true);
    if !files.is_empty() {
        for path in files {
            let result = forth.include(path);
            forth.flush();
            match result {
                // There is no terminal to return to, so QUIT ends the run.
                Err(e) if e.code == QUIT => break,
                Err(e) => {
                    eprintln!("{e}");
                    return 1;
                }
                Ok(()) => {}
            }
            if forth.finished() {
                break;
            }
        }
        return 0;
    }

    println!("forthling — WORDS shows the dictionary, BYE quits.");
    // The REPL reads through the same buffer as ACCEPT and KEY.
    while let Some(line) = forth.read_line() {
        let result = forth.interpret(&String::from_utf8_lossy(&line));
        forth.flush();
        match result {
            Ok(()) if forth.finished() => break,
            Ok(()) if forth.in_definition() => println!(" compiled"),
            Ok(()) => println!(" ok"),
            Err(e) if e.code == QUIT => println!(),
            Err(e) => println!(" {e}"),
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(parse_size("65536"), Ok(64 << 10));
        assert_eq!(parse_size("512K"), Ok(512 << 10));
        assert_eq!(parse_size("4m"), Ok(4 << 20));
        assert_eq!(parse_size("2MiB"), Ok(2 << 20));
        assert_eq!(parse_size("1G"), Ok(1 << 30));
        assert!(parse_size("4X").is_err());
        assert!(parse_size("M").is_err());
        assert!(parse_size("1K").is_err());
        assert!(parse_size("8G").is_err());
        assert!(parse_size("65537").is_err());
    }

    #[test]
    fn arguments() {
        let args = |a: &str| options(a.split_whitespace().map(String::from));
        let files = |f: &[&str]| f.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(args("-m 4M a.fs b.fs"), Ok(Some((4 << 20, files(&["a.fs", "b.fs"])))));
        assert_eq!(args("a.fs --memory=128K"), Ok(Some((128 << 10, files(&["a.fs"])))));
        assert_eq!(args("--memory 2M -- -x.fs"), Ok(Some((2 << 20, files(&["-x.fs"])))));
        assert!(args("-m").is_err() && args("-x").is_err() && args("-m 3X").is_err());
    }

    #[test]
    fn config() {
        let text = "# forthling settings\n\nmemory = 16M  # more room\n";
        assert_eq!(parse_config(text), Ok(Some("16M".to_string())));
        assert_eq!(parse_config(""), Ok(None));
        assert_eq!(parse_config("x = 1"), Err("1: unknown setting x".to_string()));
        assert_eq!(parse_config("\nmemory"), Err("2: expected `name = value`".to_string()));
    }
}
