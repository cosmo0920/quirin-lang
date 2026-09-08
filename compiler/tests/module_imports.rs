use qcore_compiler::{CompileOptions, SourceFile, compile_files};
use qcore_core::prelude::concurrency::task_execution::{TaskExecutionScript, execute_task_module};

const CONTEXT: &str = include_str!("../../examples/threads/core_context.qcore");

#[test]
fn source_import_enables_existing_checked_task_lowering() {
    let module = compile_files(
        &[SourceFile::new("context.qcore", CONTEXT)],
        &CompileOptions::default(),
    )
    .unwrap();
    let checked = module.checked.as_scoped_tasks().unwrap();
    for created in [true, false] {
        let observed = execute_task_module(
            checked,
            &TaskExecutionScript {
                spawn_outcomes: vec![created],
                allocation_outcomes: vec![true],
            },
        )
        .unwrap();
        assert_eq!(
            observed.root_outcome,
            qcore_compiler::prelude::concurrency::scoped_tasks::TaskOutcome::I32(42)
        );
    }
}

#[test]
fn an_import_in_another_file_cannot_enable_concurrency() {
    let missing = CONTEXT.replace("import std.concurrency;\n", "");
    assert_ne!(missing, CONTEXT);
    let helper = SourceFile::new(
        "helper.qcore",
        "module library; import std.concurrency; fn helper() -> i32 { return 0; }",
    );
    for files in [
        vec![helper.clone(), SourceFile::new("missing.qcore", &missing)],
        vec![SourceFile::new("missing.qcore", &missing), helper.clone()],
    ] {
        let errors = compile_files(&files, &CompileOptions::default()).unwrap_err();
        assert_eq!(errors[0].code, "Q015");
        assert_eq!(errors[0].location.path, "missing.qcore");
    }
    compile_files(
        &[helper, SourceFile::new("context.qcore", CONTEXT)],
        &CompileOptions::default(),
    )
    .unwrap();
}

#[test]
fn sequential_and_entry_free_modules_need_no_concurrency_import() {
    for prefix in ["", "module library; ", "module contract; "] {
        compile_files(
            &[SourceFile::new(
                "plain.qcore",
                format!("{prefix}fn main() -> i32 {{ return 0; }}"),
            )],
            &CompileOptions::default(),
        )
        .unwrap();
    }
    for source in [
        "import std.missing; fn main() -> i32 { return 0; }",
        "module library; import std.missing; fn helper() -> i32 { return 0; }",
    ] {
        let errors = compile_files(
            &[SourceFile::new("bad.qcore", source)],
            &CompileOptions::default(),
        )
        .unwrap_err();
        assert_eq!(errors[0].code, "Q014");
    }
}
