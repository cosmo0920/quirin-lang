use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    let Some(path) = arguments.next() else {
        eprintln!("usage: qcore-audit CONFIG.json");
        return ExitCode::FAILURE;
    };
    if arguments.next().is_some() {
        eprintln!("usage: qcore-audit CONFIG.json");
        return ExitCode::FAILURE;
    }
    match qcore_audit::run_config(&PathBuf::from(path)) {
        Ok(counts) => {
            println!(
                "{} units: {} Safe / {} provisional Unsafe / {} Unknown / {} data-only / {} gateway failures",
                counts.units,
                counts.safe_units,
                counts.unsafe_units,
                counts.unknown_units,
                counts.data_only_units,
                counts.gateway_failures
            );
            if counts.gateway_failures == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            eprintln!("audit rejected: {error}");
            ExitCode::FAILURE
        }
    }
}
