//! Shared Clang typed-AST transport after the unrestricted-C gateway.
//!
//! Input is expected to have passed [`crate::legacy_c::validate_unrestricted_c11`].
//! These helpers expose raw JSON for source-profile interpreters, not trusted
//! semantic evidence. Exact shape matching and fail-closed handling of absent
//! fields remain the caller's obligation.

use crate::legacy_c::C11TranslationUnit;
use serde_json::Value;
use std::io::Write;
use std::process::Stdio;

/// Parse Clang declarations selected by its substring name filter.
pub fn ast_declarations(
    source: &str,
    unit: &C11TranslationUnit<'_>,
    function: &str,
) -> Result<Vec<Value>, String> {
    let mut command = crate::clang::command()?;
    unit.append_frontend_flags(&mut command);
    let mut child = command
        .args([
            "-x",
            "c",
            unit.dialect.clang_argument(),
            "-fsyntax-only",
            "-Xclang",
            "-ast-dump=json",
            "-Xclang",
            "-ast-dump-filter",
            "-Xclang",
            function,
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Clang AST unavailable: {error}"))?;
    // Drain stdout concurrently with feeding input: filtered AST output can be
    // emitted before Clang has consumed a large preprocessed translation unit.
    let mut stdin = child.stdin.take().ok_or("Clang stdin unavailable")?;
    let output = std::thread::scope(|scope| {
        let writer = scope.spawn(move || stdin.write_all(source.as_bytes()));
        let output = child.wait_with_output();
        let written = writer
            .join()
            .map_err(|_| "Clang input writer panicked".to_owned())?;
        written.map_err(|error| format!("Clang input failed: {error}"))?;
        output.map_err(|error| format!("Clang AST failed: {error}"))
    })?;
    if !output.status.success() {
        return Err(format!(
            "Clang AST rejected source: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter::<Value>()
        .map(|value| value.map_err(|error| format!("unsupported Clang AST JSON: {error}")))
        .collect()
}

/// Require exactly one body-bearing definition with the exact requested name.
pub fn function_ast(
    source: &str,
    unit: &C11TranslationUnit<'_>,
    function: &str,
) -> Result<Value, String> {
    // Clang's name filter is a substring filter. Require one exact definition.
    let mut definitions = ast_declarations(source, unit, function)?
        .into_iter()
        .filter(|value| {
            kind(value) == "FunctionDecl"
                && value["name"].as_str() == Some(function)
                && children(value)
                    .iter()
                    .any(|node| kind(node) == "CompoundStmt")
        })
        .collect::<Vec<_>>();
    if definitions.len() != 1 {
        return Err(format!(
            "expected one selected definition after preprocessing, found {}",
            definitions.len()
        ));
    }
    Ok(definitions.remove(0))
}

/// Read a raw node kind; absent or malformed kinds yield the empty string.
pub fn kind(node: &Value) -> &str {
    node["kind"].as_str().unwrap_or("")
}

/// Read raw child nodes; absent or malformed arrays yield an empty slice.
pub fn children(node: &Value) -> &[Value] {
    node["inner"].as_array().map_or(&[], Vec::as_slice)
}
