use std::env;
use std::fs;
use std::process::ExitCode;

use qcore_compiler::{
    C11TranslationUnit, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
};

fn main() -> ExitCode {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example cycle17_legacy_audit");
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() < 2 {
        eprintln!(
            "usage: cycle17_legacy_audit <translation-unit.c> <entry> [include-directory ...]"
        );
        return ExitCode::from(64);
    }
    let source = match fs::read_to_string(&arguments[0]) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("could not read {}: {error}", arguments[0]);
            return ExitCode::FAILURE;
        }
    };
    let unit = C11TranslationUnit {
        source: &source,
        include_paths: arguments[2..].to_vec(),
        definitions: Vec::new(),
        ..C11TranslationUnit::default()
    };
    match analyze_unrestricted_c11(&unit, &arguments[1], &ResourceContracts::default()) {
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => {
            println!(
                "Safe: {} checked function(s)",
                certificate.checked_functions.len()
            );
            ExitCode::SUCCESS
        }
        Ok(UnrestrictedLegacyResult::Unsafe(summary)) => {
            println!("Unsafe: {} analyzed function(s)", summary.functions.len());
            ExitCode::SUCCESS
        }
        Ok(UnrestrictedLegacyResult::Unknown {
            reason, partial, ..
        }) => {
            println!(
                "Unknown: {reason}; {} partially analyzed function(s)",
                partial.map_or(0, |summary| summary.functions.len())
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("C11 gateway failed: {error}");
            ExitCode::FAILURE
        }
    }
}
