//! forthling-sqlite: forthling with the `SQLite` words installed.

fn main() {
    forthling::cli::main(&[&forthling::Json, &forthling_sqlite::Sqlite]);
}
