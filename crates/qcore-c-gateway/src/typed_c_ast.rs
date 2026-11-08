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
///
/// Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: a launch whose executable,
/// arguments and standard input equal an earlier successful launch's on this
/// thread is answered by that launch's declarations
/// ([`crate::clang_reuse::LaunchCache`]); the source importers ask the same
/// name of the same text repeatedly (23,720 of the suite's 27,558 filtered
/// launches repeated one).
pub fn ast_declarations(
    source: &str,
    unit: &C11TranslationUnit<'_>,
    function: &str,
) -> Result<Vec<Value>, String> {
    let command = declarations_command(unit, function)?;
    FILTERED_DECLARATIONS.with(|cache| {
        cache.get_or_launch(command, source, |command| {
            launch_declarations(command, source, crate::deep_unit_ast::default_values)
        })
    })
}

/// Card **DEEP-AST-TRANSPORTS-AND-WALL-TIME** (rule 13q): the same filtered
/// launch for a consultation of the **analysed unit** (the rule-11y
/// definition fallback and the retention guard's shadow check through
/// `typed_access_path::definition_ast`, and the appended probes through
/// `typed_access_path::appended_declarations`), whose stream is read within
/// rule 13o's bound ([`crate::deep_unit_ast::values_from_slice`]). The source
/// importers keep [`ast_declarations`] and the default limit (rule 13r). The
/// two keep separate launch caches, so a stream only this one reads is never
/// answered to an importer.
pub(crate) fn unit_declarations(
    source: &str,
    unit: &C11TranslationUnit<'_>,
    filter: &str,
) -> Result<Vec<Value>, String> {
    let command = declarations_command(unit, filter)?;
    UNIT_DECLARATIONS.with(|cache| {
        cache.get_or_launch(command, source, |command| {
            launch_declarations(command, source, crate::deep_unit_ast::values_from_slice)
        })
    })
}

/// The filtered launch's command: the unit's frontend flags and Clang's
/// substring name filter.
fn declarations_command(
    unit: &C11TranslationUnit<'_>,
    function: &str,
) -> Result<std::process::Command, String> {
    let mut command = crate::clang::command()?;
    unit.append_frontend_flags(&mut command);
    let dialect = unit.dialect.clang_invocation_argument()?;
    command.args([
        "-x",
        "c",
        dialect,
        "-fsyntax-only",
        "-Xclang",
        "-ast-dump=json",
        "-Xclang",
        "-ast-dump-filter",
        "-Xclang",
        function,
        "-",
    ]);
    Ok(command)
}

thread_local! {
    static FILTERED_DECLARATIONS: crate::clang_reuse::LaunchCache<Vec<Value>> =
        const { crate::clang_reuse::LaunchCache::new() };
    /// Card DEEP-AST-TRANSPORTS-AND-WALL-TIME (rule 13q): the analysed-unit
    /// consultations' own cache.
    static UNIT_DECLARATIONS: crate::clang_reuse::LaunchCache<Vec<Value>> =
        const { crate::clang_reuse::LaunchCache::new() };
}

pub(crate) fn release_reuse_caches() {
    FILTERED_DECLARATIONS.with(crate::clang_reuse::LaunchCache::clear);
    UNIT_DECLARATIONS.with(crate::clang_reuse::LaunchCache::clear);
}

fn launch_declarations(
    mut command: std::process::Command,
    source: &str,
    parse: fn(&[u8]) -> Result<Vec<Value>, serde_json::Error>,
) -> Result<Vec<Value>, String> {
    let mut child = command
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
    parse(&output.stdout).map_err(|error| format!("unsupported Clang AST JSON: {error}"))
}

/// Parse the complete translation-unit AST.
///
/// Dedicated closed-world source judgments need this form when proving that a
/// selected object has no unexamined readers or writers.  The result is still
/// raw Clang transport and conveys no semantic authority by itself.
///
/// Card **DEEP-AST-TRANSPORTS-AND-WALL-TIME** (rule 13q): the document is read
/// as rule 13o reads the unit index ([`crate::deep_unit_ast::from_slice`]):
/// the default parse first, and only when it fails, a document nesting at most
/// 256 levels again without the limit on a dedicated stack.
pub fn translation_unit_ast(source: &str, unit: &C11TranslationUnit<'_>) -> Result<Value, String> {
    translation_unit_ast_read(source, unit, crate::deep_unit_ast::from_reader)
}

/// Seekable transport for readers that need retries or multiple declaration
/// passes. Large outputs spool to a private disposable `.qc`; small outputs
/// stay in memory. Parsing starts only after Clang exits successfully.
pub(crate) fn translation_unit_ast_read<T>(
    source: &str,
    unit: &C11TranslationUnit<'_>,
    parse: impl FnOnce(&mut crate::clang_ast_input::AstInput) -> Result<T, serde_json::Error>,
) -> Result<T, String> {
    let mut command = crate::clang::command()?;
    unit.append_frontend_flags(&mut command);
    let dialect = unit.dialect.clang_invocation_argument()?;
    command.args([
        "-x",
        "c",
        dialect,
        "-fsyntax-only",
        "-Xclang",
        "-ast-dump=json",
        "-",
    ]);
    launch_ast_read(command, source, parse)
}

fn launch_ast_read<T>(
    mut command: std::process::Command,
    source: &str,
    parse: impl FnOnce(&mut crate::clang_ast_input::AstInput) -> Result<T, serde_json::Error>,
) -> Result<T, String> {
    let child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Clang AST unavailable: {error}"))?;
    let mut input = std::thread::scope(|scope| {
        // This guard lives inside the scope callback, so unwinding or a
        // failed thread launch kills/reaps Clang before scoped threads join.
        struct ChildGuard {
            child: std::process::Child,
            armed: bool,
        }
        impl Drop for ChildGuard {
            fn drop(&mut self) {
                if self.armed {
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                }
            }
        }
        let mut child = ChildGuard { child, armed: true };
        let mut stdin = child.child.stdin.take().ok_or("Clang stdin unavailable")?;
        let stdout = child
            .child
            .stdout
            .take()
            .ok_or("Clang stdout unavailable")?;
        let mut stderr = child
            .child
            .stderr
            .take()
            .ok_or("Clang stderr unavailable")?;
        let writer = std::thread::Builder::new()
            .spawn_scoped(scope, move || stdin.write_all(source.as_bytes()))
            .map_err(|error| format!("Clang input failed: {error}"))?;
        let reader = std::thread::Builder::new()
            .spawn_scoped(scope, move || {
                crate::clang_ast_input::AstInput::capture(stdout)
            })
            .map_err(|error| format!("Clang AST failed: {error}"))?;
        let errors = std::thread::Builder::new()
            .spawn_scoped(scope, move || {
                use std::io::Read;
                let mut bytes = Vec::new();
                stderr.read_to_end(&mut bytes).map(|_| bytes)
            })
            .map_err(|error| format!("Clang AST failed: {error}"))?;
        let status = child.child.wait();
        if status.is_ok() {
            child.armed = false;
        } else {
            let _ = child.child.kill();
        }
        let written = writer
            .join()
            .map_err(|_| "Clang input writer panicked".to_owned());
        let captured = reader
            .join()
            .map_err(|_| "Clang output reader panicked".to_owned());
        let errors = errors
            .join()
            .map_err(|_| "Clang stderr reader panicked".to_owned());
        written?.map_err(|error| format!("Clang input failed: {error}"))?;
        let input = captured?.map_err(|error| format!("Clang AST failed: {error}"))?;
        let status = status.map_err(|error| format!("Clang AST failed: {error}"))?;
        let stderr = errors?.map_err(|error| format!("Clang AST failed: {error}"))?;
        if !status.success() {
            return Err(format!(
                "Clang AST rejected source: {}",
                String::from_utf8_lossy(&stderr).trim()
            ));
        }
        Ok(input)
    })?;
    parse(&mut input).map_err(|error| format!("unsupported Clang AST JSON: {error}"))
}

/// Run the same Clang transport while letting an internal reader deserialize
/// the buffered JSON without retaining the entire translation-unit `Value`.
/// The output buffer and Clang's own peak memory still exist during parsing.
pub(crate) fn translation_unit_ast_with<T>(
    source: &str,
    unit: &C11TranslationUnit<'_>,
    parse: impl FnOnce(&[u8]) -> Result<T, serde_json::Error>,
) -> Result<T, String> {
    let mut command = crate::clang::command()?;
    unit.append_frontend_flags(&mut command);
    let dialect = unit.dialect.clang_invocation_argument()?;
    let mut child = command
        .args([
            "-x",
            "c",
            dialect,
            "-fsyntax-only",
            "-Xclang",
            "-ast-dump=json",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Clang AST unavailable: {error}"))?;
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
    parse(&output.stdout).map_err(|error| format!("unsupported Clang AST JSON: {error}"))
}

/// Card CG13-UNION-AND-BITFIELD-PLACES (rule 10w): Clang's own record
/// layouts of the translation unit (`-fdump-record-layouts-complete`), as
/// text. Raw transport, like the AST: the caller parses it fail-closed.
pub(crate) fn record_layout_dump(
    source: &str,
    unit: &C11TranslationUnit<'_>,
) -> Result<String, String> {
    layout_dump(source, unit, &[])
}

fn layout_dump(
    source: &str,
    unit: &C11TranslationUnit<'_>,
    extra: &[&str],
) -> Result<String, String> {
    let mut command = crate::clang::command()?;
    unit.append_frontend_flags(&mut command);
    let dialect = unit.dialect.clang_invocation_argument()?;
    let mut child = command
        .args([
            "-x",
            "c",
            dialect,
            "-fsyntax-only",
            "-Xclang",
            "-fdump-record-layouts-complete",
        ])
        .args(extra.iter().flat_map(|flag| ["-Xclang", flag]))
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Clang layout unavailable: {error}"))?;
    let mut stdin = child.stdin.take().ok_or("Clang stdin unavailable")?;
    let output = std::thread::scope(|scope| {
        let writer = scope.spawn(move || stdin.write_all(source.as_bytes()));
        let output = child.wait_with_output();
        let written = writer
            .join()
            .map_err(|_| "Clang input writer panicked".to_owned())?;
        written.map_err(|error| format!("Clang input failed: {error}"))?;
        output.map_err(|error| format!("Clang layout failed: {error}"))
    })?;
    if !output.status.success() {
        return Err(format!(
            "Clang layout rejected source: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| format!("Clang layout is not text: {error}"))
}

/// Card **LINK-STANDING** (rule 40): Clang's name-filtered declarations of a
/// probe source, launched without the reuse caches (each probe text is read
/// once) and read under rule 13q's bound.
pub(crate) fn filtered_declarations_uncached(
    source: &str,
    unit: &C11TranslationUnit<'_>,
    filter: &str,
) -> Result<Vec<Value>, String> {
    launch_declarations(
        declarations_command(unit, filter)?,
        source,
        crate::deep_unit_ast::values_from_slice,
    )
}

/// Card **LINK-STANDING** (rule 40): every complete record's layout with
/// canonical member types (`-fdump-record-layouts-complete
/// -fdump-record-layouts-canonical`), as text. Raw transport.
pub(crate) fn canonical_record_layout_dump(
    source: &str,
    unit: &C11TranslationUnit<'_>,
) -> Result<String, String> {
    layout_dump(source, unit, &["-fdump-record-layouts-canonical"])
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

#[cfg(test)]
mod audit_lifetime_tests {
    use super::*;

    #[test]
    fn filtered_ast_ownership_is_released_on_error_and_nested_exit() {
        let command = || std::process::Command::new("audit-lifetime-test-clang");
        let mut launches = 0;
        let result: Result<(), ()> = crate::with_audit_unit_lifetimes(|| {
            FILTERED_DECLARATIONS
                .with(|cache| {
                    cache.get_or_launch(command(), "int x;", |_| {
                        launches += 1;
                        Ok(vec![serde_json::json!({"kind":"VarDecl"})])
                    })
                })
                .unwrap();
            crate::with_audit_unit_lifetimes(|| {
                FILTERED_DECLARATIONS
                    .with(|cache| {
                        cache.get_or_launch(command(), "int x;", |_| {
                            launches += 1;
                            Ok(vec![])
                        })
                    })
                    .unwrap();
            });
            Err(())
        });
        assert!(result.is_err());
        FILTERED_DECLARATIONS
            .with(|cache| {
                cache.get_or_launch(command(), "int x;", |_| {
                    launches += 1;
                    Ok(vec![])
                })
            })
            .unwrap();
        assert_eq!(launches, 3);
        release_reuse_caches();
    }
}

#[cfg(all(test, unix))]
mod transport_tests {
    use super::*;

    fn shell(script: &str) -> std::process::Command {
        let mut command = std::process::Command::new("sh");
        command.args(["-c", script]);
        command
    }

    #[test]
    fn refuses_successful_prefix_with_malformed_suffix() {
        let error = launch_ast_read(
            shell("cat >/dev/null; printf '{} garbage'"),
            "input",
            crate::deep_unit_ast::from_reader::<Value>,
        )
        .unwrap_err();
        assert!(error.starts_with("unsupported Clang AST JSON: trailing characters"));
    }

    #[test]
    fn child_rejection_precedes_json_and_preserves_stderr() {
        let error = launch_ast_read(
            shell("cat >/dev/null; printf 'not JSON'; printf 'rejected details' >&2; exit 7"),
            "input",
            crate::deep_unit_ast::from_reader::<Value>,
        )
        .unwrap_err();
        assert_eq!(error, "Clang AST rejected source: rejected details");
    }

    #[test]
    fn drains_both_outputs_while_feeding_large_input() {
        let script = "printf '{\"padding\":\"'; head -c 2097152 /dev/zero | tr '\\000' x; printf '\"}'; head -c 131072 /dev/zero >&2; cat >/dev/null";
        let value = launch_ast_read(
            shell(script),
            &"x".repeat(2 * 1024 * 1024),
            crate::deep_unit_ast::from_reader::<Value>,
        )
        .unwrap();
        assert_eq!(value["padding"].as_str().unwrap().len(), 2097152);
    }
}
