use std::env;
use std::path::Path;
use std::process::ExitCode;

use qcore_compiler::verifier_plugins::libevent::LIBEVENT_SUBMODULE_PATH;
use qcore_compiler::{PINNED_LIBEVENT_DESCRIPTION, verify_pinned_libevent_source};

fn main() -> ExitCode {
    let root = env::args_os().nth(1).map_or_else(
        || {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("compiler crate must be inside the repository")
                .join(LIBEVENT_SUBMODULE_PATH)
        },
        Into::into,
    );
    match verify_pinned_libevent_source(&root) {
        Ok(evidence) => {
            println!(
                "verified Libevent {} at {}",
                PINNED_LIBEVENT_DESCRIPTION, evidence.commit
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
