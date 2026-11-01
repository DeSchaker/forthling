//! forthling-mcp: an MCP server over stdio that runs Forth in forthling's
//! sandbox. A client starts it and sends JSON-RPC messages, one per line.

use forthling::cli::{parse_bytes, parse_duration, parse_size};
use forthling_mcp::{Server, Settings};
use std::io::{self, BufRead, Write};
use std::{env, process, thread};

const USAGE: &str = "\
usage: forthling-mcp [options]

An MCP server over stdio: AI agents run Forth through it, always in the
sandbox, with files only in memory. With Claude Code:

  claude mcp add forthling -- forthling-mcp

options (each at most the value in brackets):
  -m, --memory SIZE      Forth memory (default 1M) [64M]
      --max-steps N      steps per call (default 10000000) [1000000000]
      --max-time T       time per call, such as 5s or 500ms (default 5s) [60s]
      --max-output SIZE  output per call (default 1M) [16M]
      --max-space SIZE   definitions and the like in all (default 4M) [64M]
      --allow-host HOST  a host that Forth code may send HTTP requests to (repeat
                         for more; *.example.com or *); needs a build with the
                         feature http (cargo install forthling-mcp --features http).
                         Without this option the network stays closed
  -h, --help             show this help
  -V, --version          show the version
";

fn settings() -> Result<Option<Settings>, String> {
    let mut s = Settings::default();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        let (name, inline) = match arg.split_once('=') {
            Some((name, value)) if name.starts_with("--") => (name, Some(value.to_string())),
            _ => (arg.as_str(), None),
        };
        let mut value = |what: &str| {
            inline.clone().or_else(|| args.next()).ok_or(format!("{name} needs {what}"))
        };
        match name {
            "-m" | "--memory" => s.memory = parse_size(&value("a size")?)?,
            "--max-steps" => {
                let n = value("a number")?;
                s.steps = n.parse().map_err(|_| format!("invalid number of steps {n}"))?;
            }
            "--max-time" => s.time = parse_duration(&value("a time")?)?,
            "--max-output" => s.output = parse_bytes(&value("a size")?)?,
            "--max-space" => s.space = parse_bytes(&value("a size")?)?,
            "--allow-host" => s.hosts.push(value("a host")?),
            "-h" | "--help" => {
                print!("{USAGE}");
                return Ok(None);
            }
            "-V" | "--version" => {
                println!("forthling-mcp {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            }
            _ => return Err(format!("unknown option {arg} (see --help)")),
        }
    }
    s.check()?;
    Ok(Some(s))
}

fn serve(settings: Settings) -> io::Result<()> {
    let mut server = Server::new(settings);
    let mut input = io::stdin().lock();
    let mut out = io::stdout().lock();
    let mut line = Vec::new();
    loop {
        line.clear();
        if input.read_until(b'\n', &mut line)? == 0 {
            return Ok(()); // the client closed stdin: done
        }
        let text = String::from_utf8_lossy(&line);
        if text.trim().is_empty() {
            continue;
        }
        if let Some(answer) = server.handle(&text) {
            writeln!(out, "{answer}")?;
            out.flush()?;
        }
    }
}

fn main() {
    let settings = match settings() {
        Ok(Some(s)) => s,
        Ok(None) => return,
        Err(e) => {
            eprintln!("forthling-mcp: {e}");
            process::exit(2);
        }
    };
    // As the forthling program does: a big stack for deeply nested CATCHes.
    let server = thread::Builder::new().stack_size(64 << 20).spawn(move || serve(settings));
    match server.expect("cannot start the server thread").join() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            eprintln!("forthling-mcp: {e}");
            process::exit(1);
        }
        Err(_) => process::exit(101),
    }
}
