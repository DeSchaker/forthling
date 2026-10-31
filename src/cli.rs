//! The forthling program: runs the Forth files given as arguments, or else
//! a REPL. It is in the library so that a program with extensions, such as
//! forthling-sqlite, is the same program with more words.

use crate::*;
use std::path::PathBuf;
use std::time::Duration;
use std::{env, fs};

/// The extensions a program installs in every Forth it starts.
pub type Extensions = &'static [&'static (dyn Extension + Sync)];

const USAGE: &str = "\
usage: forthling [options] [file ...]

Runs the files in order, or else starts a REPL.

options:
  -m, --memory SIZE  memory for data space and heap, such as 4M (default 1M)
  -b, --blocks FILE  the block file (default blocks.fb in the current directory)
      --sandbox      no file access from Forth, and the limits below: 10000000
                     steps, 5s and 1M of output per line of the REPL or per file
      --max-steps N  at most N steps (instructions and words) per evaluation
      --max-time T   at most T per evaluation, such as 5s or 500ms
      --max-output SIZE  at most SIZE bytes of output per evaluation
  -h, --help         show this help

SIZE is a number of bytes with an optional K, M or G (binary units). A limit
of `none` lifts it, also one of --sandbox.
A config file can set both, with lines such as `memory = 4M` and
`blocks = /home/me/forth/blocks.fb`:
";

/// Settings from the command line or the config file; None when not given.
#[derive(Debug, Default, PartialEq)]
struct Settings {
    memory: Option<String>,
    blocks: Option<String>,
}

/// What to run: each setting comes from the command line, else the config
/// file, else the default.
struct Run {
    memory: usize,
    blocks: Option<String>, // None: the default block file
    sandbox: bool,          // no file access from Forth
    limits: Limits,
    files: Vec<String>,
}

/// What the command line asks for.
#[derive(Debug, Default, PartialEq)]
struct Args {
    settings: Settings,
    sandbox: bool,
    max_steps: Option<String>,
    max_time: Option<String>,
    max_output: Option<String>,
    files: Vec<String>,
    help: bool,
}

/// Runs the program with these extensions installed: the main function of
/// forthling, and of every program that adds extensions to it.
///
/// ```no_run
/// forthling::cli::main(&[]); // what main.rs does
/// ```
pub fn main(extensions: Extensions) -> ! {
    let run = match options() {
        Ok(Some(run)) => run,
        Ok(None) => std::process::exit(0),
        Err(e) => {
            eprintln!("forthling: {e}");
            std::process::exit(2);
        }
    };
    // The main thread's stack is small (1 MiB on Windows); nested CATCHes
    // need more, up to 1000 deep.
    let interpreter =
        std::thread::Builder::new().stack_size(64 << 20).spawn(move || run_forth(run, extensions));
    let code = interpreter.expect("cannot start the interpreter thread").join().unwrap_or(101);
    std::process::exit(code);
}

/// What to run; None after --help.
fn options() -> Result<Option<Run>, String> {
    let args = parse_args(env::args().skip(1))?;
    if args.help {
        // Programs with extensions, such as forthling-sqlite, show their own name.
        let program = env::args()
            .next()
            .and_then(|p| Some(PathBuf::from(p).file_stem()?.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "forthling".into());
        print!("{}", USAGE.replacen("forthling", &program, 1));
        println!("  {}", config_path().map_or("(none)".into(), |p| p.display().to_string()));
        return Ok(None);
    }
    let config = config_settings()?;
    let memory = match args.settings.memory.or(config.memory) {
        Some(size) => parse_size(&size)?,
        None => DEFAULT_MEMORY,
    };
    let blocks = args.settings.blocks.or(config.blocks);
    let mut limits = if args.sandbox { Limits::SANDBOX } else { Limits::default() };
    if let Some(n) = &args.max_steps {
        limits.steps = limit(n, |n| n.parse().map_err(|_| format!("invalid number of steps {n}")))?;
    }
    if let Some(t) = &args.max_time {
        limits.time = limit(t, parse_duration)?;
    }
    if let Some(size) = &args.max_output {
        limits.output = limit(size, parse_bytes)?;
    }
    Ok(Some(Run { memory, blocks, sandbox: args.sandbox, limits, files: args.files }))
}

/// A limit: `none`, or what `parse` makes of it.
fn limit<T>(s: &str, parse: impl Fn(&str) -> Result<T, String>) -> Result<Option<T>, String> {
    if s.eq_ignore_ascii_case("none") { Ok(None) } else { parse(s.trim()).map(Some) }
}

/// A time such as 5s, 500ms or 2m; a plain number is seconds.
fn parse_duration(s: &str) -> Result<Duration, String> {
    let invalid = || format!("invalid time {s}");
    let digits = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    let n: u64 = s[..digits].parse().map_err(|_| invalid())?;
    match s[digits..].to_ascii_lowercase().as_str() {
        "" | "s" => Ok(Duration::from_secs(n)),
        "ms" => Ok(Duration::from_millis(n)),
        "m" | "min" => Ok(Duration::from_secs(n.checked_mul(60).ok_or_else(invalid)?)),
        _ => Err(invalid()),
    }
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut parsed = Args::default();
    while let Some(arg) = args.next() {
        // --name=value is the same as --name value.
        let (name, inline) = match arg.split_once('=') {
            Some((name, value)) if name.starts_with("--") => (name, Some(value.to_string())),
            _ => (arg.as_str(), None),
        };
        let mut value = |what: &str| {
            inline.clone().or_else(|| args.next()).ok_or(format!("{name} needs {what}"))
        };
        match name {
            "-m" | "--memory" => parsed.settings.memory = Some(value("a size")?),
            "-b" | "--blocks" => parsed.settings.blocks = Some(value("a file name")?),
            "--sandbox" => parsed.sandbox = true,
            "--max-steps" => parsed.max_steps = Some(value("a number")?),
            "--max-time" => parsed.max_time = Some(value("a time")?),
            "--max-output" => parsed.max_output = Some(value("a size")?),
            "-h" | "--help" => parsed.help = true,
            "--" => parsed.files.extend(args.by_ref()),
            a if a.starts_with('-') && a.len() > 1 => {
                return Err(format!("unknown option {a} (see --help)"));
            }
            _ => parsed.files.push(arg),
        }
    }
    Ok(parsed)
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

/// The settings of the config file, if there is one.
fn config_settings() -> Result<Settings, String> {
    let Some(path) = config_path() else { return Ok(Settings::default()) };
    let Ok(text) = fs::read_to_string(&path) else { return Ok(Settings::default()) };
    parse_config(&text).map_err(|e| format!("{}:{e}", path.display()))
}

/// Reads `key = value` lines; `#` starts a comment. Returns the settings,
/// or the line number and the problem.
fn parse_config(text: &str) -> Result<Settings, String> {
    let mut settings = Settings::default();
    for (i, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap().trim();
        if line.is_empty() {
            continue;
        }
        match line.split_once('=').map(|(k, v)| (k.trim(), v.trim().to_string())) {
            Some(("memory", value)) => settings.memory = Some(value),
            Some(("blocks", value)) => settings.blocks = Some(value),
            Some((key, _)) => return Err(format!("{}: unknown setting {key}", i + 1)),
            None => return Err(format!("{}: expected `name = value`", i + 1)),
        }
    }
    Ok(settings)
}

/// A memory size such as 65536, 512K, 4M or 1G.
fn parse_size(s: &str) -> Result<usize, String> {
    let size = parse_bytes(s).map_err(|_| format!("invalid memory size {s}"))?;
    if !(MIN_MEMORY..=MAX_MEMORY).contains(&size) {
        return Err(format!("memory size {s} is not between 64K and 4G"));
    }
    if size % 8 != 0 {
        return Err(format!("memory size {s} is not a multiple of 8 bytes"));
    }
    Ok(size)
}

/// A number of bytes such as 65536, 512K, 4M or 1G.
fn parse_bytes(s: &str) -> Result<usize, String> {
    let invalid = || format!("invalid size {s}");
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
    n.checked_mul(1 << shift).ok_or_else(invalid)
}

/// Runs the files, or else the REPL; returns the exit code.
fn run_forth(run: Run, extensions: Extensions) -> i32 {
    let mut forth = Forth::with_memory(run.memory);
    for &extension in extensions {
        forth.install(extension);
    }
    if let Some(path) = run.blocks {
        forth.set_block_file(path).expect("no block buffers in use yet");
    }
    // The files on the command line are the user's own; Forth code that the
    // sandbox runs cannot open others.
    forth.permit(Permission::Files, !run.sandbox);
    forth.set_limits(run.limits);
    forth.set_interactive(true);
    if !run.files.is_empty() {
        for path in &run.files {
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
    if !extensions.is_empty() {
        let names: Vec<&str> = extensions.iter().map(|e| e.name()).collect();
        println!("Extensions: {}.", names.join(", "));
    }
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
        let args = |a: &str| parse_args(a.split_whitespace().map(String::from));
        let files = |f: &[&str]| f.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let parsed = args("-m 4M a.fs -b my.fb b.fs").unwrap();
        assert_eq!(parsed.settings.memory.as_deref(), Some("4M"));
        assert_eq!(parsed.settings.blocks.as_deref(), Some("my.fb"));
        assert_eq!(parsed.files, files(&["a.fs", "b.fs"]));
        let parsed = args("a.fs --memory=128K --blocks=x.fb").unwrap();
        assert_eq!(
            parsed.settings,
            Settings { memory: Some("128K".into()), blocks: Some("x.fb".into()) }
        );
        assert_eq!(args("--memory 2M -- -x.fs").unwrap().files, files(&["-x.fs"]));
        assert!(args("--help").unwrap().help);
        assert!(args("-m").is_err() && args("-b").is_err() && args("-x").is_err());
        let parsed =
            args("--sandbox --max-steps 1000 --max-time=2s --max-output none x.fs").unwrap();
        assert!(parsed.sandbox);
        assert_eq!(parsed.max_steps.as_deref(), Some("1000"));
        assert_eq!(parsed.max_time.as_deref(), Some("2s"));
        assert_eq!(parsed.max_output.as_deref(), Some("none"));
        assert_eq!(parsed.files, files(&["x.fs"]));
        assert!(args("--max-time").is_err());
    }

    #[test]
    fn limits() {
        assert_eq!(parse_duration("5s"), Ok(Duration::from_secs(5)));
        assert_eq!(parse_duration("7"), Ok(Duration::from_secs(7)));
        assert_eq!(parse_duration("250ms"), Ok(Duration::from_millis(250)));
        assert_eq!(parse_duration("2m"), Ok(Duration::from_secs(120)));
        assert!(parse_duration("5h").is_err() && parse_duration("s").is_err());
        assert_eq!(parse_bytes("1M"), Ok(1 << 20));
        assert_eq!(parse_bytes("10"), Ok(10));
        assert_eq!(limit("NONE", parse_bytes), Ok(None));
        assert_eq!(limit("2K", parse_bytes), Ok(Some(2048)));
        assert!(limit("x", parse_bytes).is_err());
    }

    #[test]
    fn config() {
        let text = "# forthling settings\n\nmemory = 16M  # more room\nblocks = C:/forth/my.fb\n";
        let settings =
            Settings { memory: Some("16M".into()), blocks: Some("C:/forth/my.fb".into()) };
        assert_eq!(parse_config(text), Ok(settings));
        assert_eq!(parse_config(""), Ok(Settings::default()));
        assert_eq!(parse_config("x = 1"), Err("1: unknown setting x".to_string()));
        assert_eq!(parse_config("\nmemory"), Err("2: expected `name = value`".to_string()));
    }
}
