//! Current-revision resource audit, with explicit historical evidence replay.
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

#[path = "support/cycle19_compatibility.rs"]
mod compatibility;

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1).peekable();
    let mut pinned = false;
    let mut compile_database = None;
    let mut jobs = None;
    while let Some(argument) = arguments.peek() {
        if argument == "--pinned-evidence" {
            pinned = true;
            arguments.next();
        } else if argument == "--compile-database" {
            arguments.next();
            let Some(path) = arguments.next() else {
                eprintln!("--compile-database requires a path");
                return ExitCode::FAILURE;
            };
            compile_database = Some(PathBuf::from(path));
        } else if argument == "--jobs" {
            arguments.next();
            let Some(count) = arguments.next() else {
                eprintln!("--jobs requires a positive integer");
                return ExitCode::FAILURE;
            };
            jobs = Some(count.to_string_lossy().into_owned());
        } else {
            break;
        }
    }
    if pinned && compile_database.is_some() {
        eprintln!("--compile-database selects a current Clang build; omit --pinned-evidence");
        return ExitCode::FAILURE;
    }
    // Card W44-AUDIT-PARALLEL-UNITS: translation units are analysed on up to
    // this many threads; every output is folded in unit order, so the job
    // count changes wall time only. `--jobs 1` is the sequential loop.
    let jobs = match qcore_audit::parallel::resolve_jobs(jobs.as_deref()) {
        Ok(jobs) => qcore_audit::parallel::effective_jobs(jobs),
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
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
            "usage: cycle19_fluent_bit_libraries_audit [--pinned-evidence] [--compile-database PATH] [--jobs N] [fluent-bit-root] \
             [monkey-libevent-compile_commands.json] [monkey-callback-source-report.json] \
             [resource-diagnostics.json] [native-observation-directory] [resource-contract-manifest.json]"
        );
        return ExitCode::FAILURE;
    }
    let result = if pinned {
        compatibility::run(
            &root,
            monkey_database.as_deref(),
            monkey_source_report.as_deref(),
            diagnostic_report.as_deref(),
            binding_directory.as_deref(),
            contract_manifest.as_deref(),
            jobs,
        )
    } else {
        compatibility::run_current(
            &root,
            monkey_database.as_deref(),
            monkey_source_report.as_deref(),
            diagnostic_report.as_deref(),
            binding_directory.as_deref(),
            contract_manifest.as_deref(),
            compile_database.as_deref(),
            jobs,
        )
    };
    match result {
        Ok(false) => ExitCode::SUCCESS,
        Ok(true) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("Fluent Bit library audit failed: {error}");
            ExitCode::FAILURE
        }
    }
}
