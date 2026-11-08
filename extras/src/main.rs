//! forthling-extras: forthling with the small building blocks installed.

fn main() {
    forthling::cli::main(&[&forthling::Json, &forthling_extras::Extras]);
}
