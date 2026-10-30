//! forthling: runs the Forth files given as arguments, or else a REPL.

use forthling::{Forth, QUIT};
use std::env;

fn main() {
    // The main thread's stack is small (1 MiB on Windows); nested CATCHes
    // need more, up to 1000 deep.
    let interpreter = std::thread::Builder::new().stack_size(64 << 20).spawn(run_forth);
    let code = interpreter.expect("cannot start the interpreter thread").join().unwrap_or(101);
    std::process::exit(code);
}

/// Runs the files given as arguments, or else the REPL; returns the exit code.
fn run_forth() -> i32 {
    let mut forth = Forth::new();
    forth.set_interactive(true);
    let files: Vec<String> = env::args().skip(1).collect();
    if !files.is_empty() {
        for path in &files {
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
