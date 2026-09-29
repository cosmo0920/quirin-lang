//! Card CG06-VALUE-CONNECTIVES-AND-CONDITIONAL (gateway rule 10z): scratch.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const S: &str = "struct S { int a; int b; unsigned bf : 3; void *p; void *v; \
struct I { void *p; int x; } in; int arr[4]; void *slots[4]; void (*cb)(void); };\n";

fn gs(source: &str) -> G {
    g(&format!("{S}{source}"))
}

#[test]
fn scratch() {
    if !clang_available() {
        return;
    }
    let Ok(path) = std::env::var("CG06_SCRATCH") else {
        return;
    };
    for source in std::fs::read_to_string(path).unwrap().lines() {
        if source.trim().is_empty() {
            continue;
        }
        let run = gs(source);
        eprintln!("{source}\n  => {:?} {:?}\n", run.status, run.diagnostics);
    }
}
