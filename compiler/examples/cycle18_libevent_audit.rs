use std::env;
use std::process::ExitCode;

use qcore_compiler::{PINNED_LIBEVENT_DESCRIPTION, verify_pinned_libevent_source};

fn main() -> ExitCode {
    let root = env::args_os()
        .nth(1)
        .unwrap_or_else(|| "../libevent".into());
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
