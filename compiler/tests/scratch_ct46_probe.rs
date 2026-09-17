//! TEMPORARY scratch probe — deleted before the card closes.
#[path = "../../tests/support/mutex_observation.rs"]
mod mutex_observation;

use qcore_compiler::{CompileOptions, SourceFile, compile_files};

#[test]
fn dump_ct46() {
    let source = std::fs::read_to_string("tests/fixtures/ct/CT46/case.qcore").unwrap();
    let module = compile_files(
        &[SourceFile {
            path: "case.qcore".into(),
            source,
        }],
        &CompileOptions::default(),
    )
    .unwrap();
    let checked = module.checked.as_checked_mutex_guards().unwrap();
    let reference = qcore_core::mutex_c_emit::interpret_mutex_program(
        &checked.lowered.program,
        &checked.certificate,
    );
    println!("REFERENCE: {reference:#?}");
    let c = module.c_source.as_deref().expect("emission");
    let instrumented = mutex_observation::instrument(c);
    std::fs::write("/tmp/claude-1000/ct46_module.c", &instrumented).unwrap();
    println!("--- EMITTED C ---\n{c}\n--- END ---");
}
