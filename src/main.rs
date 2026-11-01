//! forthling: runs the Forth files given as arguments, or else a REPL (see
//! cli.rs in the library).

fn main() {
    forthling::cli::main(&[
        &forthling::Json,
        &forthling::Debugger,
        &forthling::Tutorial,
        &forthling::Functional,
    ]);
}
