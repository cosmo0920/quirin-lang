use qcore_compiler::driver::{CompileOptions, SourceFile, compile_files};
use qcore_compiler::frontend::{elaborate_source, parse_source_program};

#[test]
fn resolved_spawn_and_join_cannot_bypass_missing_execution_lowering() {
    let source = "fn worker() -> unit { return; } fn main() -> unit { thread scope s { let h = spawn s worker()?; let r = join move h; } return; }";
    let parsed = parse_source_program(source).unwrap();
    assert_eq!(parsed.task_expressions().len(), 2);
    let typed = elaborate_source(&parsed).unwrap();
    let main = typed
        .declarations
        .iter()
        .find(|d| d.name == "main")
        .unwrap();
    let tasks = &main.statements[0].nested;
    assert_eq!(tasks.len(), 2);
    assert!(
        tasks
            .iter()
            .all(|s| s.binding.is_some() && s.task_expressions.len() == 1)
    );
    assert_ne!(tasks[0].binding, tasks[1].binding);

    let errors = compile_files(
        &[SourceFile::new("tasks.qcore", source)],
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert!(
        errors.iter().any(|e| e.code == "Q500"
            && e.message.contains("structured control flow")
            && e.location.path == "tasks.qcore"),
        "{errors:?}"
    );
    assert!(!errors.iter().any(|e| e.code == "Q013"), "{errors:?}");
}
