//! Batch parser-derived intent for packaging; not declaration verification.
//! JSON strings are escaped here to keep the frontend's dependency boundary.
use qcore_frontend::{frontend::parse_source_module_mode, source_extension::mode_name};
use std::fmt::Write;

fn json_string(text: &str) -> String {
    let mut result = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            c if c < ' ' => write!(&mut result, "\\u{:04x}", c as u32).unwrap(),
            c => result.push(c),
        }
    }
    result.push('"');
    result
}

fn main() -> Result<(), String> {
    let mut rows = Vec::new();
    for path in std::env::args().skip(1) {
        let source = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
        let path = json_string(&path);
        rows.push(match parse_source_module_mode(&source) {
            Ok(mode) => format!(
                "{{\"path\":{path},\"mode\":{}}}",
                json_string(mode_name(mode))
            ),
            Err(error) => format!(
                "{{\"path\":{path},\"mode\":null,\"error\":{}}}",
                json_string(&error.message)
            ),
        });
    }
    println!("[{}]", rows.join(","));
    Ok(())
}
