//! forthling-http: forthling with the HTTP words installed. The hosts a
//! program may reach are those of `--allow-host`; with none, no request goes
//! out. `--sandbox` denies the network altogether.

use forthling::cli::ExtraOption;
use forthling::{Extension, Forth};
use forthling_http::Http;
use std::sync::Mutex;

static HOSTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn allow_host(host: &str) -> Result<(), String> {
    if host.is_empty() {
        return Err("--allow-host needs a host".into());
    }
    HOSTS.lock().expect("no thread panics holding the list").push(host.to_string());
    Ok(())
}

/// Installs the HTTP words with the hosts of the command line, which are
/// known only after the options are read.
struct Program;

impl Extension for Program {
    fn name(&self) -> &str {
        "FORTHLING-HTTP"
    }

    fn install(&self, f: &mut Forth) {
        let hosts = HOSTS.lock().expect("no thread panics holding the list");
        f.install(&Http::new().allow_all(hosts.iter().map(String::as_str)));
    }
}

fn main() {
    let allow = ExtraOption {
        name: "--allow-host",
        value: "HOST",
        help: "a host that requests may go to (repeat; *.example.com or *)",
        apply: allow_host,
    };
    forthling::cli::main_with(&[&forthling::Json, &Program], &[allow]);
}
