//! Batch parser-derived intent for packaging; not declaration verification.
use qcore_frontend::{frontend::parse_source_module_mode, source_extension::mode_name};
fn main() -> Result<(), String> {
    let mut rows = Vec::new();
    for path in std::env::args().skip(1) {
        let source = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
        rows.push(match parse_source_module_mode(&source) {
            Ok(mode) => serde_json::json!({"path":path,"mode":mode_name(mode)}),
            Err(error) => serde_json::json!({"path":path,"mode":null,"error":error.message}),
        });
    }
    println!(
        "{}",
        serde_json::to_string(&rows).map_err(|e| e.to_string())?
    );
    Ok(())
}
