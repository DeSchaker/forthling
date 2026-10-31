//! A bigger stack for the WebAssembly module than the 1 MiB default, for
//! deep nesting such as CATCH inside CATCH (forthling allows 1000 of them).

fn main() {
    if std::env::var("TARGET").is_ok_and(|t| t.starts_with("wasm32")) {
        println!("cargo:rustc-link-arg=-zstack-size=16777216");
    }
}
