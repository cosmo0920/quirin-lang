#![cfg(unix)]

// The wrapper is written by a separate process
// (`tests/support/process_fixture.rs`). This binary has one test, so no
// sibling spawns while it is written; the shared writer keeps it that way
// if a test is added.
#[path = "../../../tests/support/process_fixture.rs"]
mod process_fixture;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyFunctionScope, ResourceContracts, analyze_prepared_translation_unit,
    prepare_translation_unit,
};

#[test]
fn a_rejected_preparation_is_not_retried_into_a_verdict() {
    let real = match qcore_c_gateway::clang_command() {
        Ok(command) => command.get_program().to_owned(),
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping prepared-TU failure test: {reason}");
            return;
        }
    };
    let root = std::env::temp_dir().join(format!("qcore-prepared-failure-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let wrapper = root.join("clang-wrapper");
    let log = root.join("invocations.log");
    let failed = root.join("failed-once");
    process_fixture::write_executable_mode(
        &wrapper,
        b"#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$QCORE_TEST_LOG\"\nsaw_e=\nsaw_p=\nfor arg in \"$@\"; do\n  [ \"$arg\" = -E ] && saw_e=1\n  [ \"$arg\" = -P ] && saw_p=1\ndone\nif [ -n \"$saw_e\" ] && [ -n \"$saw_p\" ] && [ ! -e \"$QCORE_TEST_FAILED\" ]; then\n  : > \"$QCORE_TEST_FAILED\"\n  echo intentional-first-preprocessing-failure >&2\n  exit 1\nfi\nexec \"$QCORE_TEST_REAL_CLANG\" \"$@\"\n",
        0o755,
    );

    let prior = std::env::var_os("QCORE_CLANG");
    unsafe {
        std::env::set_var("QCORE_CLANG", &wrapper);
        std::env::set_var("QCORE_TEST_LOG", &log);
        std::env::set_var("QCORE_TEST_FAILED", &failed);
        std::env::set_var("QCORE_TEST_REAL_CLANG", &real);
    }
    let source = "void subject(void) { return; }\n";
    let unit = C11TranslationUnit {
        source,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..C11TranslationUnit::default()
    };
    let prepared = prepare_translation_unit(&unit);
    assert!(prepared.validation_error().is_some());
    let result =
        analyze_prepared_translation_unit(&prepared, "subject", &ResourceContracts::default());
    assert!(
        result.is_err(),
        "a rejected snapshot cannot publish a verdict"
    );

    let invocations = std::fs::read_to_string(&log).unwrap();
    let plain = invocations
        .lines()
        .filter(|line| line.contains(" -E ") && line.contains(" -P "))
        .count();
    assert_eq!(
        plain, 1,
        "the rejected -E -P stage was retried:\n{invocations}"
    );

    unsafe {
        match prior {
            Some(value) => std::env::set_var("QCORE_CLANG", value),
            None => std::env::remove_var("QCORE_CLANG"),
        }
        std::env::remove_var("QCORE_TEST_LOG");
        std::env::remove_var("QCORE_TEST_FAILED");
        std::env::remove_var("QCORE_TEST_REAL_CLANG");
    }
    let _ = std::fs::remove_dir_all(root);
}
