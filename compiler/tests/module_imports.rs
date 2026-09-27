use qcore_compiler::{CompileOptions, SourceFile, compile_files};
use qcore_core::prelude::concurrency::task_execution::{TaskExecutionScript, execute_task_module};

const CONTEXT: &str = include_str!("../../examples/threads/core_context.qr");

#[test]
fn source_import_enables_existing_checked_task_lowering() {
    let module = compile_files(
        &[SourceFile::new("context.qcore", CONTEXT)],
        &CompileOptions::default(),
    )
    .unwrap();
    // `core_context.qcore` is on `source.std-io-tasks.v1` since card
    // `COMPOSE-CONCURRENCY`; its task half is the module the task route checked.
    let checked = module.checked.as_composed_tasks().unwrap().tasks();
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
    let helper = SourceFile::new(
        "helper.qcore",
        "module library; import std.concurrency; fn helper() -> i32 { return 0; }",
    );
    let lf = CONTEXT.replace("\r\n", "\n");
    for context in [lf.clone(), lf.replace('\n', "\r\n")] {
        // Remove only the declaration, independent of checkout line endings.
        let missing = context.replacen("import std.concurrency;", "", 1);
        assert_ne!(missing, context);
        for files in [
            vec![helper.clone(), SourceFile::new("missing.qcore", &missing)],
            vec![SourceFile::new("missing.qcore", &missing), helper.clone()],
        ] {
            let errors = compile_files(&files, &CompileOptions::default()).unwrap_err();
            assert_eq!(errors[0].code, "Q015");
            assert_eq!(errors[0].location.path, "missing.qcore");
        }
        compile_files(
            &[helper.clone(), SourceFile::new("context.qcore", &context)],
            &CompileOptions::default(),
        )
        .unwrap();
    }
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
