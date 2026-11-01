//! forthling-regex: forthling with the regular expression words installed.

fn main() {
    forthling::cli::main(&[&forthling::Json, &forthling_regex::Regex]);
}
