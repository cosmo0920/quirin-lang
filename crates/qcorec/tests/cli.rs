//! CLI process tests relocated with the binary (W0 of
//! `docs/GENERALIZATION_PLAN.md`); library-level toolchain and module-mode
//! tests remain in `qcore-compiler`.

use std::process::Command;

#[path = "../../../tests/support/diagnostic_sink.rs"]
mod diagnostic_sink;

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn instrument_file(path: &std::path::Path) {
    let generated = std::fs::read_to_string(path).unwrap();
    std::fs::write(path, diagnostic_sink::instrument(&generated)).unwrap();
}

#[cfg(target_os = "windows")]
#[path = "../../../tests/support/windows_c.rs"]
mod windows_c;

#[test]
#[cfg(target_os = "windows")]
fn cli_windows_emission_recovers_from_second_creation_failure() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../compiler/tests/fixtures/ct/CT13");
    let directory = std::env::temp_dir().join(format!("qcore-cli-windows-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let c = directory.join("recovery.c");
    let checked = Command::new(env!("CARGO_BIN_EXE_quirin"))
        .arg("check")
        .arg(fixture.join("case.qr"))
        .output()
        .unwrap();
    assert!(checked.status.success(), "{checked:?}");
    assert!(String::from_utf8_lossy(&checked.stdout).contains("Linux/macOS/Windows threads"));
    let emitted = Command::new(env!("CARGO_BIN_EXE_quirin"))
        .arg("emit")
        .arg(fixture.join("case.qr"))
        .arg("-o")
        .arg(&c)
        .output()
        .unwrap();
    assert!(emitted.status.success(), "{emitted:?}");
    let output = windows_c::run(
        &diagnostic_sink::instrument(&std::fs::read_to_string(&c).unwrap()),
        Some(&std::fs::read_to_string(fixture.join("create_failure.h")).unwrap()),
        &["-DQCORE_CT_FAIL_CREATE=2".to_owned()],
    );
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, b"QCORE_TASK_SPAWN 1 0\nQCORE_TASK_CREATE_FAILED 0\nQCORE_TASK_RESULT 1 I32 42\nQCORE_TASK_JOIN 1\nQCORE_TASK_RESULT 0 I32 42\n");
    assert_eq!(
        output.stderr,
        b"CREATE_OK:1\nCREATE_FAILED:2\nFREE\nJOIN_OK:1\nFREE\n"
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn cli_names_owned_profile_and_emits_real_buffer_transfer() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = root.join("compiler/tests/fixtures/ct/CT01/case.qr");
    let directory = std::env::temp_dir().join(format!("qcore-cli-owned-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let c = directory.join("module.c");
    let executable = directory.join("module");
    let checked = Command::new(env!("CARGO_BIN_EXE_quirin"))
        .arg("check")
        .arg(&source)
        .output()
        .unwrap();
    assert!(checked.status.success(), "{checked:?}");
    assert!(
        String::from_utf8_lossy(&checked.stdout)
            .contains("scoped owned-buffer tasks (Linux/macOS/Windows threads)")
    );
    let emitted = Command::new(env!("CARGO_BIN_EXE_quirin"))
        .arg("emit")
        .arg(&source)
        .arg("-o")
        .arg(&c)
        .output()
        .unwrap();
    assert!(emitted.status.success(), "{emitted:?}");
    instrument_file(&c);
    let compiled = Command::new(std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".into()))
        .args(["-std=c11", "-pthread", "-Wall", "-Wextra", "-Werror"])
        .arg(&c)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(compiled.status.success(), "{compiled:?}");
    let output = Command::new(executable).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let trace = String::from_utf8_lossy(&output.stdout);
    assert!(trace.contains("QCORE_TASK_RESULT 1 BUFFER 1\nQCORE_TASK_JOIN 1\n"));
    assert!(trace.contains("QCORE_BUFFER_DROP 0 1\nQCORE_TASK_RESULT 0 I32 42\n"));
    std::fs::remove_dir_all(directory).unwrap();
}

/// Card RULE-SWITCH-ENTRY-POINTS: `quirin` records no gateway rule set, so
/// it refuses a non-empty `QCORE_GATEWAY_RULES_OFF` before it reads anything
/// (and a refused list as everywhere); with nothing off it runs as before.
#[test]
fn quirin_refuses_a_gateway_switch_list_it_cannot_record() {
    let directory = std::env::temp_dir().join(format!("qcore-rule-switch-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let input = directory.join("main.qcore");
    std::fs::write(&input, "fn main() -> i32 { return 17; }").unwrap();
    let check = |list: Option<&str>| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_quirin"));
        command.arg("check").arg(&input);
        match list {
            Some(list) => command.env("QCORE_GATEWAY_RULES_OFF", list),
            None => command.env_remove("QCORE_GATEWAY_RULES_OFF"),
        };
        command.output().unwrap()
    };
    for (list, expected) in [
        (
            "35",
            "quirin does not record the gateway rule set it runs under",
        ),
        ("35,no-such-rule", "no-such-rule"),
    ] {
        let output = check(Some(list));
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(output.stdout.is_empty(), "{output:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "{output:?}"
        );
    }
    for list in [None, Some("")] {
        let output = check(list);
        assert!(output.status.success(), "{list:?}: {output:?}");
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn core_cli_works_without_clang() {
    let directory = std::env::temp_dir().join(format!("qcore-no-clang-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let input = directory.join("main.qcore");
    std::fs::write(&input, "fn main() -> i32 { return 17; }").unwrap();
    for operation in ["check", "emit"] {
        let output = Command::new(env!("CARGO_BIN_EXE_quirin"))
            .env("QCORE_CLANG", directory.join("missing-clang.exe"))
            .env("PATH", "")
            .arg(operation)
            .arg(&input)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!output.stdout.is_empty());
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn cli_checks_contracts_and_refuses_standalone_emit_without_overwriting_output() {
    let directory = std::env::temp_dir().join(format!("qcore-module-mode-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let source = directory.join("api.qcore");
    let output = directory.join("output.c");
    std::fs::write(&source, "module contract; extern fn f() -> unit;").unwrap();
    std::fs::write(&output, "keep existing output").unwrap();
    let checked = Command::new(env!("CARGO_BIN_EXE_quirin"))
        .arg("check")
        .arg(&source)
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    assert!(String::from_utf8_lossy(&checked.stdout).contains("no entry point"));
    let emitted = Command::new(env!("CARGO_BIN_EXE_quirin"))
        .arg("emit")
        .arg(&source)
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!emitted.status.success());
    assert_eq!(
        std::fs::read_to_string(&output).unwrap(),
        "keep existing output"
    );
    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn plugin_discovery_errors_are_reported_without_builtin_fallback() {
    let missing =
        std::env::temp_dir().join(format!("qcore-absent-profiles-{}", std::process::id()));
    for value in [missing.as_os_str(), std::ffi::OsStr::new("")] {
        let output = Command::new(env!("CARGO_BIN_EXE_quirin"))
            .env("QCORE_PROFILES", value)
            .args(["plugins", "list"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("provider discovery failed"));
    }
}

#[test]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn cli_checks_emits_and_executes_native_source_tasks() {
    let directory =
        std::env::temp_dir().join(format!("qcore-cli-source-tasks-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let input = directory.join("tasks.qcore");
    let c = directory.join("tasks.c");
    let exe = directory.join("tasks");
    std::fs::write(
        &input,
        r#"
#import std.concurrency; concurrency checked;
fn worker(x: i32) -> i32 { return x; }
fn main() -> i32 effects { throws<SpawnError>, spawn, join, may_block } {
    let x = 42;
    thread scope s {
        let h = spawn s worker(copy x)?;
        let y = join move h?;
        return y;
    }
}
"#,
    )
    .unwrap();
    let checked = Command::new(env!("CARGO_BIN_EXE_quirin"))
        .arg("check")
        .arg(&input)
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    assert!(
        String::from_utf8_lossy(&checked.stdout)
            .contains("scoped copy tasks (Linux/macOS/Windows threads)")
    );
    let emitted = Command::new(env!("CARGO_BIN_EXE_quirin"))
        .arg("emit")
        .arg(&input)
        .arg("-o")
        .arg(&c)
        .output()
        .unwrap();
    assert!(
        emitted.status.success(),
        "{}",
        String::from_utf8_lossy(&emitted.stderr)
    );
    instrument_file(&c);
    let compiled = Command::new(std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".into()))
        .args(["-std=c11", "-pthread", "-Wall", "-Wextra", "-Werror"])
        .arg(&c)
        .arg("-o")
        .arg(&exe)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let executed = Command::new(exe).output().unwrap();
    assert!(executed.status.success());
    assert!(String::from_utf8_lossy(&executed.stdout).contains("QCORE_TASK_RESULT 0 I32 42"));
    // A source failure must not replace an existing emitted artifact.
    let original = std::fs::read(&c).unwrap();
    std::fs::write(&input, "#import std.concurrency; concurrency checked; fn main() -> unit { thread scope s { let h = spawn s main()?; } return; }").unwrap();
    let rejected = Command::new(env!("CARGO_BIN_EXE_quirin"))
        .arg("emit")
        .arg(&input)
        .arg("-o")
        .arg(&c)
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert_eq!(std::fs::read(&c).unwrap(), original);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn cli_recovery_joins_first_child_after_second_creation_fails() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../compiler/tests/fixtures/ct/CT13");
    let directory =
        std::env::temp_dir().join(format!("qcore-cli-task-recovery-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let source = fixture.join("case.qr");
    let c = directory.join("recovery.c");
    let exe = directory.join("recovery");
    let checked = Command::new(env!("CARGO_BIN_EXE_quirin"))
        .arg("check")
        .arg(&source)
        .output()
        .unwrap();
    assert!(checked.status.success(), "{checked:?}");
    assert!(
        String::from_utf8_lossy(&checked.stdout)
            .contains("scoped copy tasks (Linux/macOS/Windows threads)")
    );
    let emitted = Command::new(env!("CARGO_BIN_EXE_quirin"))
        .arg("emit")
        .arg(&source)
        .arg("-o")
        .arg(&c)
        .output()
        .unwrap();
    assert!(emitted.status.success(), "{emitted:?}");
    instrument_file(&c);
    // An explicit invalid QCORE_CLANG must fail this test, never skip it.
    let mut compiler =
        Command::new(std::env::var_os("QCORE_CLANG").unwrap_or_else(|| "clang".into()));
    compiler.args([
        "-std=c11",
        "-pthread",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-DQCORE_CT_FAIL_CREATE=2",
    ]);
    if cfg!(target_os = "macos") {
        compiler.arg("-I").arg(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../compiler/tests/fixtures/apple-threads"),
        );
    }
    let compiled = compiler
        .arg("-include")
        .arg(fixture.join("create_failure.h"))
        .arg(&c)
        .arg("-o")
        .arg(&exe)
        .output()
        .unwrap();
    assert!(compiled.status.success(), "{compiled:?}");
    let executed = Command::new(&exe).output().unwrap();
    assert!(executed.status.success(), "{executed:?}");
    assert_eq!(
        String::from_utf8_lossy(&executed.stdout),
        "QCORE_TASK_SPAWN 1 0\nQCORE_TASK_CREATE_FAILED 0\nQCORE_TASK_RESULT 1 I32 42\nQCORE_TASK_JOIN 1\nQCORE_TASK_RESULT 0 I32 42\n"
    );
    // The header identifies the exact native thread and aborts if its record
    // is freed before join. The first FREE is the failed creation's record.
    assert_eq!(
        String::from_utf8_lossy(&executed.stderr),
        "CREATE_OK:1\nCREATE_FAILED:2\nFREE\nJOIN_OK:1\nFREE\n"
    );
    std::fs::remove_dir_all(directory).unwrap();
}

/// Card `UNSAFE-LINKAGE-ARTIFACT`. A `Safe` obtained through unchecked linkage
/// is never printed bare (card `UNSAFE-LINKAGE` §4.1), and when a library
/// manifest names the waived symbol the printed premise says *which build* it
/// was waived from.
#[test]
fn the_cli_prints_the_artifact_a_waiver_was_waived_from() {
    let fixtures =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../compiler/tests/fixtures");
    let source = fixtures.join("unsafe_linkage/round_trip.qr");
    let manifest = fixtures.join("native_library/unchecked_demo.qrlib");

    // Without the flag: the bare premise, exactly as wave 18 printed it.
    let bare = Command::new(env!("CARGO_BIN_EXE_quirin"))
        .arg("check")
        .arg(&source)
        .output()
        .unwrap();
    assert!(bare.status.success(), "{bare:?}");
    let bare = String::from_utf8_lossy(&bare.stdout).into_owned();
    assert!(bare.contains("under 5 declared assumption(s):"), "{bare}");
    assert!(
        bare.contains(
            "unchecked linkage: `k_unchecked_add` is linked as `(int32_t, int32_t) -> int32_t` with no contract; nothing about its behavior is checked"
        ),
        "{bare}"
    );
    assert!(!bare.contains("artifact"), "no build was named: {bare}");

    // With the flag: the same waiver, now naming the artifact and its hash.
    let sharp = Command::new(env!("CARGO_BIN_EXE_quirin"))
        .arg("check")
        .arg("--native-library")
        .arg(&manifest)
        .arg(&source)
        .output()
        .unwrap();
    assert!(sharp.status.success(), "{sharp:?}");
    let sharp = String::from_utf8_lossy(&sharp.stdout).into_owned();
    assert!(sharp.contains("under 5 declared assumption(s):"), "{sharp}");
    assert!(
        sharp.contains(
            "from artifact `libqcore_unchecked_demo.a` of library `qcore_unchecked_demo 0.1.0` at sha256:9f2c4a1e7b3d05860f14ca9e2d7b58311c06ae4f9d2b7c8e5a0341f6d9b8c72e"
        ),
        "{sharp}"
    );
    // The stated identity is printed as what it is: declared, not verified.
    assert!(
        sharp.contains("the artifact identity is declared, not verified here"),
        "{sharp}"
    );
    assert!(
        sharp.contains(
            "under stated ABI premises (data model lp64, no unwinding across the boundary, closed allocator custody, single-threaded caller)"
        ),
        "{sharp}"
    );

    // A manifest that does not load refuses the module rather than being
    // ignored, and the CLI reports it as a diagnostic.
    let broken = std::env::temp_dir().join(format!("qcore-manifest-{}.qrlib", std::process::id()));
    std::fs::write(
        &broken,
        std::fs::read_to_string(&manifest)
            .unwrap()
            .replace("data_model lp64;", "data_model ilp32;"),
    )
    .unwrap();
    let refused = Command::new(env!("CARGO_BIN_EXE_quirin"))
        .arg("check")
        .arg("--native-library")
        .arg(&broken)
        .arg(&source)
        .output()
        .unwrap();
    assert!(!refused.status.success(), "{refused:?}");
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("Q039"),
        "{refused:?}"
    );
    std::fs::remove_file(&broken).unwrap();
}

/// Card `NATIVE-LIBRARY-HEADER`. A manifest may name its **exported header**,
/// and the CLI resolves that reference against the manifest's own directory —
/// one candidate path, no search, no fallback (`HEADER-CLI-RELATIVE`). The
/// manifest is the only anchor this binary has for a reference the manifest
/// itself spelled, and a search would make two byte-identical manifests mean
/// different things depending on where they were invoked from.
///
/// Three rows: the level reachable through the shipped binary and printing a
/// **verified** identity beside the declared ones; a header that contradicts
/// the manifest refusing rather than withdrawing; and a named header nobody can
/// open reported as the `open` failure it is rather than as a binding this
/// driver silently dropped.
#[test]
fn the_cli_reads_the_exported_header_a_manifest_names() {
    let fixtures =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../compiler/tests/fixtures");
    let source = fixtures.join("unsafe_linkage/round_trip.qr");
    let base = std::fs::read_to_string(fixtures.join("native_library/unchecked_demo.qrlib"))
        .expect("the wave-19 fixture manifest is readable");

    let directory = std::env::temp_dir().join(format!("qcore-header-cli-{}", std::process::id()));
    std::fs::create_dir_all(directory.join("include")).unwrap();
    let manifest = directory.join("demo.qrlib");
    let header = directory.join("include/demo.h");

    let declarations = "#ifndef DEMO_H\n\
                        #define DEMO_H\n\
                        #include <stdint.h>\n\
                        int32_t k_unchecked_add(int32_t left, int32_t right);\n\
                        void k_unchecked_side_effect(uint32_t value);\n\
                        uint64_t k_unchecked_measure(uint64_t value);\n\
                        #endif\n";
    let write = |text: &str| {
        std::fs::write(&header, text).unwrap();
        // The digest the manifest states is measured over exactly the bytes the
        // decision is taken on, which is why it can honestly be *verified*
        // where the artifact hash beside it is only pinned.
        let digest = qcore_compiler::digest::sha256_hex(text.as_bytes());
        std::fs::write(
            &manifest,
            base.replace(
                "thread_posture single_caller;",
                &format!(
                    "thread_posture single_caller;\nheader include/demo.h;\nheader_hash sha256:{digest};"
                ),
            ),
        )
        .unwrap();
    };
    let check = || {
        Command::new(env!("CARGO_BIN_EXE_quirin"))
            .arg("check")
            .arg("--native-library")
            .arg(&manifest)
            .arg(&source)
            .output()
            .unwrap()
    };

    // 1. The header agrees: accepted, and the premise names it as read.
    write(declarations);
    let agreed = check();
    assert!(agreed.status.success(), "{agreed:?}");
    let agreed = String::from_utf8_lossy(&agreed.stdout).into_owned();
    assert!(
        agreed.contains("its exported header `include/demo.h` was read at sha256:"),
        "{agreed}"
    );
    assert!(
        agreed.contains("that digest is verified, not declared"),
        "the two kinds of identity in one premise must be distinguishable: {agreed}"
    );
    // And the artifact identity beside it says what it is, unchanged.
    assert!(
        agreed.contains("the artifact identity is declared, not verified here"),
        "{agreed}"
    );

    // 2. One projection moved in the header: refused, not withdrawn.
    write(&declarations.replace(
        "int32_t k_unchecked_add(int32_t left, int32_t right);",
        "int64_t k_unchecked_add(int32_t left, int32_t right);",
    ));
    let refused = check();
    assert!(!refused.status.success(), "{refused:?}");
    let refused = String::from_utf8_lossy(&refused.stderr).into_owned();
    assert!(refused.contains("Q064"), "{refused}");
    assert!(refused.contains("k_unchecked_add"), "{refused}");

    // 3. A named header nobody can open is the `open` failure, reported as
    //    itself rather than as a binding this driver silently dropped.
    write(declarations);
    std::fs::remove_file(&header).unwrap();
    let missing = check();
    assert!(!missing.status.success(), "{missing:?}");
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("include/demo.h"),
        "{missing:?}"
    );

    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn the_cli_reads_the_include_texts_a_manifest_declares() {
    // Card `NATIVE-LIBRARY-HEADER-DECORATION` through the shipped binary. The
    // header below is decorated the way every real third-party header is, and
    // card `NATIVE-LIBRARY-HEADER` refuses it by name; the one `include`
    // statement — which says which text, never what it means — is the delta.
    //
    // `HEADER-CLI-RELATIVE` covers the declared texts too: one candidate path
    // under the manifest's own directory, no search and no fallback, because two
    // byte-identical manifests must not mean different things depending on where
    // they were invoked from.
    let fixtures =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../compiler/tests/fixtures");
    let source = fixtures.join("unsafe_linkage/round_trip.qr");
    let base = std::fs::read_to_string(fixtures.join("native_library/unchecked_demo.qrlib"))
        .expect("the wave-19 fixture manifest is readable");

    let directory =
        std::env::temp_dir().join(format!("qcore-decoration-cli-{}", std::process::id()));
    std::fs::create_dir_all(directory.join("include")).unwrap();
    let manifest = directory.join("demo.qrlib");
    let header = directory.join("include/demo.h");
    let visibility = directory.join("include/demo_visibility.h");

    let declarations = "#ifndef DEMO_H\n\
                        #define DEMO_H\n\
                        #include <stdint.h>\n\
                        #include \"demo_visibility.h\"\n\
                        DEMO_EXPORT int32_t k_unchecked_add(int32_t left, int32_t right);\n\
                        DEMO_EXPORT void k_unchecked_side_effect(uint32_t value);\n\
                        DEMO_EXPORT uint64_t k_unchecked_measure(uint64_t value);\n\
                        #endif\n";
    let write = |visibility_text: &str| {
        std::fs::write(&header, declarations).unwrap();
        std::fs::write(&visibility, visibility_text).unwrap();
        let header_digest = qcore_compiler::digest::sha256_hex(declarations.as_bytes());
        let visibility_digest = qcore_compiler::digest::sha256_hex(visibility_text.as_bytes());
        std::fs::write(
            &manifest,
            base.replace(
                "thread_posture single_caller;",
                &format!(
                    "thread_posture single_caller;\n\
                     header include/demo.h;\n\
                     header_hash sha256:{header_digest};\n\
                     include \"demo_visibility.h\" from include/demo_visibility.h sha256:{visibility_digest};"
                ),
            ),
        )
        .unwrap();
    };
    let check = || {
        Command::new(env!("CARGO_BIN_EXE_quirin"))
            .arg("check")
            .arg("--native-library")
            .arg(&manifest)
            .arg(&source)
            .output()
            .unwrap()
    };

    // 1. Every arm of the visibility text defines the name and every body is a
    //    decoration this reader erases: the header is read and the premise names
    //    *both* texts, because the reading rested on both.
    write(
        "#ifndef DEMO_VISIBILITY_H\n\
         #define DEMO_VISIBILITY_H\n\
         #if defined(DEMO_SHARED)\n\
         # define DEMO_EXPORT __attribute__((visibility(\"default\")))\n\
         #else\n\
         # define DEMO_EXPORT\n\
         #endif\n\
         #endif\n",
    );
    let agreed = check();
    assert!(agreed.status.success(), "{agreed:?}");
    let agreed = String::from_utf8_lossy(&agreed.stdout).into_owned();
    assert!(
        agreed.contains("reading `include/demo_visibility.h` (\"demo_visibility.h\") at sha256:"),
        "the spliced text is named in the premise: {agreed}"
    );
    assert!(
        agreed.contains("those digests are verified, not declared"),
        "{agreed}"
    );

    // 2. One arm left undefined: the derivation withdraws and the decorated
    //    declaration keeps card `NATIVE-LIBRARY-HEADER`'s refusal, naming the
    //    token.
    write(
        "#ifndef DEMO_VISIBILITY_H\n\
         #define DEMO_VISIBILITY_H\n\
         #if defined(DEMO_SHARED)\n\
         # define DEMO_EXPORT __attribute__((visibility(\"default\")))\n\
         #endif\n\
         #endif\n",
    );
    let refused = check();
    assert!(!refused.status.success(), "{refused:?}");
    let refused = String::from_utf8_lossy(&refused.stderr).into_owned();
    assert!(refused.contains("Q063"), "{refused}");
    assert!(refused.contains("DEMO_EXPORT"), "{refused}");

    std::fs::remove_dir_all(&directory).unwrap();
}

/// Card `W48-QR-C-STRUCT-IMPORT`: an FFI file's `check` line says the result
/// holds under the unsafe-FFI assumption and never plain `Safe`, and lists the
/// imported struct among its premises; without a recorded target no layout is
/// read and the file is refused.
#[test]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn cli_prints_the_unsafe_ffi_verdict_and_its_premises() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let directory = std::env::temp_dir().join(format!("qcore-cli-ffi-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::copy(
        root.join("examples/ffi/qr_header.h"),
        directory.join("qr_header.h"),
    )
    .unwrap();
    let clang = std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".into());
    let version = Command::new(&clang).arg("--version").output().unwrap();
    let triple = Command::new(&clang).arg("-dumpmachine").output().unwrap();
    let compiler = String::from_utf8(version.stdout).unwrap();
    let target = directory.join("host.target");
    std::fs::write(
        &target,
        format!(
            "quirin-recorded-target v1\npointer_width = 64\ntriple = {}\ncompiler = {}\n",
            String::from_utf8(triple.stdout).unwrap().trim(),
            compiler.lines().next().unwrap().trim()
        ),
    )
    .unwrap();
    let program = directory.join("demo.qr");
    std::fs::write(
        &program,
        "#import std.ffi.unsafe;\n#import std.fmt;\n#import std.io;\nstruct! { type Header = c\"struct qr_header\" from \"qr_header.h\"; fields { len: u32, flags: u8 } }\nfn main() -> unit effects { io } {\n    let h = Header { len: 40, flags: 255 };\n    let flags = h.flags;\n    flags |> print hex;\n    return;\n}\n",
    )
    .unwrap();
    let checked = Command::new(env!("CARGO_BIN_EXE_quirin"))
        .arg("check")
        .arg("--recorded-target")
        .arg(&target)
        .arg(&program)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&checked.stdout);
    assert!(checked.status.success(), "{checked:?}");
    assert!(
        stdout.contains("Safe under the unsafe-ffi assumption, never plain Safe"),
        "{stdout}"
    );
    assert!(stdout.contains("verdict: Safe under the "), "{stdout}");
    assert!(stdout.contains("unsafe FFI (unsafe-ffi)"), "{stdout}");
    assert!(
        stdout.contains("`Header = struct qr_header` from \"qr_header.h\""),
        "{stdout}"
    );
    let refused = Command::new(env!("CARGO_BIN_EXE_quirin"))
        .arg("check")
        .arg(&program)
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("Q063"),
        "{refused:?}"
    );
    std::fs::remove_dir_all(&directory).unwrap();
}
