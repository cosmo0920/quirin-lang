use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    let Some(path) = arguments.next() else {
        eprintln!("usage: quirin-audit CONFIG.json");
        return ExitCode::FAILURE;
    };
    if arguments.next().is_some() {
        eprintln!("usage: quirin-audit CONFIG.json");
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
            println!(
                "preparation: {} unit(s), {} Clang invocation(s); -E -P {} us / syntax {} us / line markers {} us",
                counts.prepared_units,
                counts.preparation_clang_invocations,
                counts.plain_preprocessing_elapsed_us,
                counts.syntax_validation_elapsed_us,
                counts.line_marked_preprocessing_elapsed_us,
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
