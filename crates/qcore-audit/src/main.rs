use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    let Some(path) = arguments.next() else {
        eprintln!("usage: quirin-audit CONFIG.json");
        return ExitCode::FAILURE;
    };
    if path == "timeout-teardown" {
        let Some(config) = arguments.next() else {
            eprintln!("usage: quirin-audit timeout-teardown CONFIG.json");
            return ExitCode::FAILURE;
        };
        if arguments.next().is_some() {
            eprintln!("usage: quirin-audit timeout-teardown CONFIG.json");
            return ExitCode::FAILURE;
        }
        return match qcore_audit::timeout_teardown::run_timeout_teardown_config(&PathBuf::from(
            config,
        )) {
            Ok(()) => {
                println!("Bounded cb_timeout self-teardown checked; report written.");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("timeout-teardown audit rejected (Unknown): {error}");
                ExitCode::FAILURE
            }
        };
    }
    if path == "event-payload" {
        let Some(config) = arguments.next() else {
            eprintln!("usage: quirin-audit event-payload CONFIG.json");
            return ExitCode::FAILURE;
        };
        if arguments.next().is_some() {
            eprintln!("usage: quirin-audit event-payload CONFIG.json");
            return ExitCode::FAILURE;
        }
        return match qcore_audit::event_payload::run_event_payload_config(&PathBuf::from(config)) {
            Ok(()) => {
                println!("Bounded cb_event payload checked; report written.");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("event-payload audit rejected (Unknown): {error}");
                ExitCode::FAILURE
            }
        };
    }
    if path == "observed-wait" {
        let Some(config) = arguments.next() else {
            eprintln!("usage: quirin-audit observed-wait CONFIG.json");
            return ExitCode::FAILURE;
        };
        if arguments.next().is_some() {
            eprintln!("usage: quirin-audit observed-wait CONFIG.json");
            return ExitCode::FAILURE;
        }
        return match qcore_audit::observed_wait::run_observed_wait_config(&PathBuf::from(config)) {
            Ok(()) => {
                println!("Observed timed-wait composition checked; bounded report written.");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("observed-wait audit rejected (Unknown): {error}");
                ExitCode::FAILURE
            }
        };
    }
    if path == "wait-failures" {
        let Some(config) = arguments.next() else {
            eprintln!("usage: quirin-audit wait-failures CONFIG.json");
            return ExitCode::FAILURE;
        };
        if arguments.next().is_some() {
            eprintln!("usage: quirin-audit wait-failures CONFIG.json");
            return ExitCode::FAILURE;
        }
        return match qcore_audit::wait_failures::run_wait_failures_config(&PathBuf::from(config)) {
            Ok(()) => {
                println!("Timed-wait failure paths checked; bounded report written.");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("wait-failures audit rejected (Unknown): {error}");
                ExitCode::FAILURE
            }
        };
    }
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
