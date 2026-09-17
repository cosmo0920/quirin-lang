//! TEMPORARY scratch probe — deleted before the card closes.
use qcore_compiler::{CompileOptions, SourceFile, compile_files};

fn compile(source: &str) -> Result<qcore_compiler::CompiledModule, Vec<String>> {
    compile_files(
        &[SourceFile {
            path: "guarded.qcore".into(),
            source: source.into(),
        }],
        &CompileOptions::default(),
    )
    .map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|d| format!("{}@{}", d.code, d.location.line))
            .collect()
    })
}

#[test]
fn probe_guard_local() {
    // A guard-local child: `thread scope c under g`, lowering to GuardLocalJoin.
    let guard_local = "import std.concurrency; fn main() -> unit {\n\
         thread scope s {\n\
         let buf = protected 4;\n\
         let m = mutex guards buf;\n\
         with lock m as g {\n\
         thread scope c under g { }\n\
         read buf with g;\n\
         }\n\
         }\n\
         }\n";
    match compile(guard_local) {
        Ok(module) => {
            let checked = module.checked.as_checked_mutex_guards().unwrap();
            println!("GUARD_LOCAL EVENTS:");
            for (i, e) in checked.lowered.program.events.iter().enumerate() {
                println!("  {i}: {e:?}");
            }
            println!(
                "GUARD_LOCAL shape: {:?}",
                qcore_core::mutex_c_emit::classify_trace(&checked.lowered.program)
            );
            println!("GUARD_LOCAL c_source some: {}", module.c_source.is_some());
        }
        Err(e) => println!("GUARD_LOCAL REJECTED: {e:?}"),
    }

    // Two sequential children — the multi-child fork/join shape.
    let two = "import std.concurrency; fn main() -> unit {\n\
         thread scope s {\n\
         let buf = protected 4;\n\
         let m = mutex guards buf;\n\
         thread scope c share buf under m { with lock m as g1 { open g1; write buf with g1; close g1; } }\n\
         thread scope d share buf under m { with lock m as g2 { open g2; write buf with g2; close g2; } }\n\
         with lock m as g { read buf with g; }\n\
         }\n\
         }\n";
    match compile(two) {
        Ok(module) => {
            let checked = module.checked.as_checked_mutex_guards().unwrap();
            println!(
                "TWO shape: {:?}",
                qcore_core::mutex_c_emit::classify_trace(&checked.lowered.program)
            );
            println!("TWO c_source some: {}", module.c_source.is_some());
            println!(
                "TWO reference: {:?}",
                qcore_core::mutex_c_emit::interpret_mutex_program(
                    &checked.lowered.program,
                    &checked.certificate
                )
            );
        }
        Err(e) => println!("TWO REJECTED: {e:?}"),
    }
}
