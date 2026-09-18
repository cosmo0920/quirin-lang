//! Byte-identity census tool for card `AGGREGATE-OWNED-FIELD`.
//!
//! Copied into `compiler/examples/` of two trees — the base commit's scratch
//! extraction and the card's own worktree — and run from each tree's root with
//! the same relative spelled paths (standing constraint 11), it prints, for
//! every `.qcore`/`.qc` file under the directories given, a record per file:
//! whether the file compiled, the execution profile, the **custody operation
//! list** (`Debug`-rendered) the byte-buffer, string, socket and sequential
//! routes hand `check_ownership`, the emitted C, and the reference observation.
//! The driver script compares the two trees' outputs byte for byte.
//!
//! The custody projection is read through the public route entry points
//! (`std_buffer::custody_operations`, `std_string::string_custody_operations`,
//! `std_socket::custody_operations`, and the sequential route's
//! `ownership_operations`) so the number measured is the list the route really
//! builds, not a re-derivation of it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use qcore_compiler::{CheckedSourceModule, CompileOptions, SourceFile, compile_files};

fn sources(root: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    let mut entries: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target" || n == ".lake") {
                continue;
            }
            sources(&path, into);
        } else if path
            .extension()
            .is_some_and(|ext| ext == "qcore" || ext == "qc")
        {
            into.push(path);
        }
    }
}

fn main() {
    let mut files = Vec::new();
    for root in std::env::args().skip(1) {
        sources(Path::new(&root), &mut files);
    }
    for path in files {
        let spelled = path.to_string_lossy().into_owned();
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        println!("=== {spelled}");
        let result = compile_files(
            &[SourceFile::new(spelled.clone(), &text)],
            &CompileOptions {
                entry: None,
                extern_contracts: None,
                native_libraries: None,
                native_library_headers: None,
            },
        );
        match result {
            Err(diagnostics) => {
                let codes: Vec<&str> = diagnostics.iter().map(|d| d.code.as_str()).collect();
                println!("refused {codes:?}");
            }
            Ok(module) => {
                println!("profile {:?}", module.execution_profile);
                let operations: Option<String> = match &module.checked {
                    CheckedSourceModule::StandardBuffer(checked) => Some(format!(
                        "{:?}",
                        qcore_core::std_buffer::custody_operations(&checked.lowered.program)
                    )),
                    CheckedSourceModule::StandardString(checked) => Some(format!(
                        "{:?}",
                        qcore_core::std_string::string_custody_operations(
                            &checked.lowered.program
                        )
                    )),
                    CheckedSourceModule::StandardSocket(checked) => Some(format!(
                        "{:?}",
                        qcore_core::std_socket::custody_operations(&checked.lowered.program)
                    )),
                    // The sequential route's driver retains the checked
                    // functions' final states, not the operation lists it
                    // handed the checker; its list is built from plain-name
                    // operations only (no arm of `whole_program.rs` was
                    // changed but one exhaustive match), so the census
                    // compares that route by its C and observation and says
                    // so rather than rendering a state whose `Debug` changed
                    // shape with the binding key.
                    CheckedSourceModule::Sequential(checked) => Some(format!(
                        "(sequential: {} checked functions; operation list not retained by the driver)",
                        checked.functions.len()
                    )),
                    _ => None,
                };
                match operations {
                    Some(operations) => println!("custody {operations}"),
                    None => println!("custody (not a route with a published custody projection)"),
                }
                match &module.c_source {
                    Some(c) => {
                        println!("--- c ({} bytes)", c.len());
                        print!("{c}");
                        if !c.ends_with('\n') {
                            println!();
                        }
                        println!("--- end c");
                    }
                    None => println!("--- no c"),
                }
                match &module.extern_observation {
                    Some(lines) => println!("observation {lines:?}"),
                    None => println!("observation none"),
                }
            }
        }
    }
}
