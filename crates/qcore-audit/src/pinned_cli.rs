//! Explicit replay of the historical Cycle 19 Fluent Bit evidence.
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

pub const USAGE: &str = "usage: quirin-audit pinned [--jobs N] [fluent-bit-root] \
[monkey-libevent-compile_commands.json] [monkey-callback-source-report.json] \
[resource-diagnostics.json] [native-observation-directory] [resource-contract-manifest.json]";

pub fn main(arguments: Vec<OsString>) -> ExitCode {
    let mut arguments = arguments.into_iter().peekable();
    let mut jobs = None;
    while let Some(argument) = arguments.peek() {
        if argument == "--jobs" {
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
    let jobs = match crate::parallel::resolve_jobs(jobs.as_deref()) {
        Ok(jobs) => crate::parallel::effective_jobs(jobs),
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
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    }
    match crate::pinned_route::run(
        &root,
        monkey_database.as_deref(),
        monkey_source_report.as_deref(),
        diagnostic_report.as_deref(),
        binding_directory.as_deref(),
        contract_manifest.as_deref(),
        jobs,
    ) {
        Ok(false) => ExitCode::SUCCESS,
        Ok(true) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("Fluent Bit library audit failed: {error}");
            ExitCode::FAILURE
        }
    }
}
