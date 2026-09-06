//! Compatibility command for the retained bounded Cycle19 provider stages.
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

#[path = "support/cycle19_compatibility.rs"]
mod compatibility;

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let root = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("../fluent-bit"));
    let monkey_database = arguments.next().map(PathBuf::from);
    let monkey_source_report = arguments.next().map(PathBuf::from);
    let diagnostic_report = arguments.next().map(PathBuf::from);
    let binding_directory = arguments.next().map(PathBuf::from);
    let contract_manifest = arguments.next().map(PathBuf::from);
    if arguments.next().is_some() {
        eprintln!(
            "usage: cycle19_fluent_bit_libraries_audit [fluent-bit-root] \
             [monkey-libevent-compile_commands.json] [monkey-callback-source-report.json] \
             [resource-diagnostics.json] [native-observation-directory] [resource-contract-manifest.json]"
        );
        return ExitCode::FAILURE;
    }
    match compatibility::run(
        &root,
        monkey_database.as_deref(),
        monkey_source_report.as_deref(),
        diagnostic_report.as_deref(),
        binding_directory.as_deref(),
        contract_manifest.as_deref(),
    ) {
        Ok(false) => ExitCode::SUCCESS,
        Ok(true) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("Fluent Bit library audit failed: {error}");
            ExitCode::FAILURE
        }
    }
}
