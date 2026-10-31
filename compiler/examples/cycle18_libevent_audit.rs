use std::env;
use std::path::Path;
use std::process::ExitCode;

use qcore_compiler::source_profiles::{LIBEVENT_SOURCE_SET, load_profile};
use qcore_compiler::verifier_plugins::libevent::LIBEVENT_SUBMODULE_PATH;
use qcore_compiler::verify_pinned_libevent_source;

fn main() -> ExitCode {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example cycle18_libevent_audit");
    let root = env::args_os().nth(1).map_or_else(
        || {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("compiler crate must be inside the repository")
                .join(LIBEVENT_SUBMODULE_PATH)
        },
        Into::into,
    );
    // The pin, the audited file inventory and the retained construct needles
    // are declared data (W2); an unloadable profile audits nothing.
    let profile = match load_profile(LIBEVENT_SOURCE_SET) {
        Ok(profile) => profile,
        Err(error) => {
            eprintln!("Libevent source-set profile unavailable: {error}");
            return ExitCode::FAILURE;
        }
    };
    println!("source profile assumptions: {:?}", profile.assumptions());
    let (Some(library), Some(pin)) = (
        profile.library(LIBEVENT_SOURCE_SET),
        profile.pin_for(LIBEVENT_SOURCE_SET),
    ) else {
        eprintln!("Libevent source-set profile declares no `libevent` library");
        return ExitCode::FAILURE;
    };
    match verify_pinned_libevent_source(&root, pin, library) {
        Ok(evidence) => {
            println!(
                "verified Libevent {} at {}",
                pin.description, evidence.commit
            );
            for path in evidence.checked_files {
                println!("checked {}", path.display());
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Libevent source audit failed: {error}");
            ExitCode::FAILURE
        }
    }
}
