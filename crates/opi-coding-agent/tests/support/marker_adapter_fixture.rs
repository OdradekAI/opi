//! Standalone test fixture compiled at runtime by `trust_resource_gating`.
//!
//! This file intentionally uses only the standard library so the integration
//! test can compile it directly with `rustc` without registering a Cargo
//! binary target that would be shipped by `cargo install`.

use std::io::{BufRead, Write};

fn json_string_field(line: &str, field: &str) -> Option<String> {
    let key = format!("\"{field}\"");
    let after_key = line.split_once(&key)?.1;
    let value = after_key.split_once(':')?.1.trim_start();
    let value = value.strip_prefix('"')?;
    let end = value.find('"')?;
    Some(value[..end].to_string())
}

fn main() {
    let marker = std::env::args()
        .nth(2)
        .expect("startup_marker requires a marker path");
    let stdin = std::io::stdin();
    let mut reader = std::io::BufReader::new(stdin.lock());
    let mut initialize = String::new();
    reader
        .read_line(&mut initialize)
        .expect("read initialize message");
    let id = json_string_field(&initialize, "id").expect("initialize id");

    std::fs::write(marker, "startup observed").expect("write startup marker");
    let mut stdout = std::io::stdout().lock();
    writeln!(
        stdout,
        "{{\"type\":\"capabilities\",\"id\":\"{id}\",\"tools\":[],\"commands\":[],\"hooks\":[],\"model_overrides\":[]}}"
    )
    .expect("write capabilities");
    stdout.flush().expect("flush capabilities");

    for line in reader.lines().map_while(Result::ok) {
        if line.contains("\"type\":\"shutdown\"") {
            break;
        }
    }
}
