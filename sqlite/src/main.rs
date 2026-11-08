//! forthling-sqlite: forthling with the `SQLite` words installed, and the text and
//! collection words that a program with a database tends to need.

fn main() {
    forthling::cli::main(&[
        &forthling::Json,
        &forthling::Strings,
        &forthling::Collections,
        &forthling_sqlite::Sqlite,
    ]);
}
