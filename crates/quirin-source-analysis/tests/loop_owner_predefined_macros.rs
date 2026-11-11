// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS — reserved macros decided from the
//! recorded compiler for the configured census reading. Negative controls
//! first: a macro the compiler defines keeps its arm; a `-U` undefines it; a
//! compiler that is unavailable or not the recorded one decides nothing; an
//! arithmetic comparison keeps both arms; a file's own `#define` wins; a
//! failed derivation leaves its file (and every file without a command)
//! undecided. The fixed-test shape is in `loop_owner_instance_flow.rs`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use qcore_c_gateway::thread_entry_census::predefined_macros::{
    ExtraUnit, FactsRequest, PredefinedMacros, condition_names, derive_facts, fixture_command,
    fixture_database, fixture_facts, identify, is_reserved, read_preprocessed,
};
use qcore_c_gateway::thread_entry_census::{
    BuildConfiguration, CensusFile, FLUENT_BIT_HOPS, read_program_configured,
};

const ROOT: &str = "/fixture";

fn view(
    commands: Vec<qcore_c_gateway::thread_entry_census::predefined_macros::CommandFacts>,
    candidates: &[&str],
    covered: &[&str],
) -> PredefinedMacros {
    let files = commands.iter().map(|c| c.key.clone()).collect::<Vec<_>>();
    let files = files.iter().map(String::as_str).collect::<Vec<_>>();
    let database = fixture_database(ROOT, &files);
    let facts = fixture_facts(&database, commands, candidates, covered);
    PredefinedMacros::derive(&facts, &database, Path::new(ROOT)).expect("the facts match")
}

/// The functions a configured reading of `files` defines.
fn functions(files: &[(&str, &str)], predefined: Option<PredefinedMacros>) -> BTreeSet<String> {
    let files = files
        .iter()
        .map(|(path, text)| CensusFile {
            path: (*path).to_owned(),
            text: (*text).to_owned(),
        })
        .collect::<Vec<_>>();
    let configuration = BuildConfiguration {
        units: files
            .iter()
            .filter(|f| f.path.ends_with(".c"))
            .map(|f| (f.path.clone(), BTreeSet::new()))
            .collect(),
        owned_prefixes: vec!["FLB_".to_owned()],
        defined_elsewhere: BTreeSet::new(),
        predefined,
        region_macros: None,
    };
    read_program_configured(&files, &FLUENT_BIT_HOPS, Some(&configuration))
        .functions
        .into_iter()
        .collect()
}

const PLATFORM: &str = r#"
#if defined (__linux__)
int linux_arm(void) { return 0; }
#elif defined (_WIN32)
int windows_arm(void) { return 1; }
#else
int other_arm(void) { return 2; }
#endif
"#;

#[test]
fn nc_a_macro_the_compiler_defines_keeps_its_arm() {
    let linux = view(
        vec![fixture_command("a.c", &["__linux__"], &[], &[])],
        &["__linux__", "_WIN32"],
        &["a.c"],
    );
    let read = functions(&[("a.c", PLATFORM)], Some(linux));
    assert!(read.contains("linux_arm"), "{read:?}");
    assert!(
        !read.contains("windows_arm") && !read.contains("other_arm"),
        "{read:?}"
    );
    // The same file under a derived set that defines `_WIN32` only.
    let windows = view(
        vec![fixture_command("a.c", &["_WIN32"], &[], &[])],
        &["__linux__", "_WIN32"],
        &["a.c"],
    );
    let read = functions(&[("a.c", PLATFORM)], Some(windows));
    assert_eq!(read, BTreeSet::from(["windows_arm".to_owned()]));
}

#[test]
fn nc_without_derived_facts_every_arm_is_read() {
    let read = functions(&[("a.c", PLATFORM)], None);
    for name in ["linux_arm", "windows_arm", "other_arm"] {
        assert!(read.contains(name), "{name}: {read:?}");
    }
}

#[test]
fn nc_an_arithmetic_comparison_keeps_both_arms() {
    let text = r#"
#if __GNUC__ >= 4
int new_gcc(void) { return 0; }
#else
int old_gcc(void) { return 1; }
#endif
#if defined (__GNUC__) && __GNUC__ >= 4
int both_new(void) { return 0; }
#else
int both_old(void) { return 1; }
#endif
"#;
    let gnuc = view(
        vec![fixture_command("a.c", &["__GNUC__"], &[], &[])],
        &["__GNUC__"],
        &["a.c"],
    );
    let read = functions(&[("a.c", text)], Some(gnuc));
    for name in ["new_gcc", "old_gcc", "both_new", "both_old"] {
        assert!(read.contains(name), "{name}: {read:?}");
    }
}

#[test]
fn nc_a_read_files_own_define_wins() {
    let header = "#define _WIN32 1\n";
    let facts = view(
        vec![fixture_command("a.c", &["__linux__"], &[], &[])],
        &["__linux__", "_WIN32"],
        &["a.c", "win.h"],
    );
    let text = "#ifndef _WIN32\nint posix(void) { return 0; }\n#else\nint win(void) { return 1; }\n#endif\n";
    let read = functions(&[("a.c", text), ("win.h", header)], Some(facts));
    assert!(read.contains("posix") && read.contains("win"), "{read:?}");
}

#[test]
fn nc_a_preprocessed_files_define_or_undef_keeps_the_name_undecided() {
    let facts = view(
        vec![
            fixture_command("a.c", &["__linux__"], &["_WIN32"], &[]),
            fixture_command("b.c", &["__linux__"], &[], &["__linux__"]),
        ],
        &["__linux__", "_WIN32"],
        &["a.c", "b.c"],
    );
    // a.c's unit `#define`s `_WIN32` somewhere: not undefined there.
    assert_eq!(facts.decide("a.c", "_WIN32"), None);
    assert_eq!(facts.decide("a.c", "__linux__"), Some(true));
    // b.c's unit `#undef`s `__linux__` somewhere: not defined there.
    assert_eq!(facts.decide("b.c", "__linux__"), None);
    assert_eq!(facts.decide("b.c", "_WIN32"), Some(false));
}

#[test]
fn nc_a_file_compiled_twice_is_decided_only_where_both_agree() {
    let facts = view(
        vec![
            fixture_command("a.c", &["__linux__", "__SHARED__"], &[], &[]),
            fixture_command("a.c", &["__linux__"], &[], &[]),
        ],
        &["__linux__", "__SHARED__", "_WIN32"],
        &["a.c"],
    );
    assert_eq!(facts.decide("a.c", "__linux__"), Some(true));
    assert_eq!(facts.decide("a.c", "__SHARED__"), None);
    assert_eq!(facts.decide("a.c", "_WIN32"), Some(false));
}

#[test]
fn nc_a_header_is_decided_over_every_command_and_only_when_read() {
    let facts = view(
        vec![
            fixture_command("a.c", &["__linux__", "__A__"], &[], &[]),
            fixture_command("b.c", &["__linux__"], &[], &[]),
        ],
        &["__linux__", "__A__", "_WIN32"],
        &["a.c", "b.c", "common.h"],
    );
    assert_eq!(facts.decide("common.h", "__linux__"), Some(true));
    assert_eq!(facts.decide("common.h", "_WIN32"), Some(false));
    assert_eq!(facts.decide("common.h", "__A__"), None);
    // A file no preprocessed unit read (a plugin the build does not compile).
    assert_eq!(facts.decide("plugins/in_winlog/winlog.c", "_WIN32"), None);
}

#[test]
fn nc_a_failed_derivation_leaves_its_file_and_every_header_undecided() {
    let mut failed = fixture_command("b.c", &["__linux__"], &[], &[]);
    failed.failure = Some("`-E -dD`: exit 1: 'generated.h' file not found".to_owned());
    let facts = view(
        vec![fixture_command("a.c", &["__linux__"], &[], &[]), failed],
        &["__linux__", "_WIN32"],
        &["a.c", "common.h"],
    );
    assert_eq!(facts.decide("a.c", "_WIN32"), Some(false));
    assert_eq!(facts.decide("b.c", "_WIN32"), None);
    assert_eq!(facts.decide("common.h", "_WIN32"), None);
    assert!(!facts.decides_uncommanded_files());
    assert_eq!(facts.unit_counts(), (1, 1));
}

#[test]
fn nc_only_reserved_candidates_are_decided() {
    let facts = view(
        vec![fixture_command("a.c", &["linux"], &[], &[])],
        &["_WIN32"],
        &["a.c"],
    );
    // `linux` is predefined in GNU modes but not reserved: never decided.
    assert_eq!(facts.decide("a.c", "linux"), None);
    // `__APPLE__` is reserved but no read condition names it.
    assert_eq!(facts.decide("a.c", "__APPLE__"), None);
    assert_eq!(facts.decide("a.c", "_WIN32"), Some(false));
    assert!(is_reserved("_WIN32") && is_reserved("__linux__"));
    assert!(!is_reserved("_win32") && !is_reserved("WIN32") && !is_reserved("_"));
}

#[test]
fn nc_a_compiler_identity_mismatch_decides_nothing() {
    let database = fixture_database(ROOT, &["a.c"]);
    let mut facts = fixture_facts(
        &database,
        vec![fixture_command("a.c", &["__linux__"], &[], &[])],
        &["_WIN32"],
        &["a.c"],
    );
    // The recorded build's compiler is another version.
    facts.recorded_compiler_version = "cc version 2".to_owned();
    let error = PredefinedMacros::derive(&facts, &database, Path::new(ROOT)).unwrap_err();
    assert!(error.contains("not the recorded build's"), "{error}");
    // An unavailable compiler.
    let mut facts = fixture_facts(
        &database,
        vec![fixture_command("a.c", &["__linux__"], &[], &[])],
        &["_WIN32"],
        &["a.c"],
    );
    facts.compilers.get_mut("cc").expect("cc").failure = Some("unavailable".to_owned());
    let error = PredefinedMacros::derive(&facts, &database, Path::new(ROOT)).unwrap_err();
    assert!(error.contains("unavailable"), "{error}");
}

#[test]
fn nc_facts_of_another_database_or_command_decide_nothing() {
    let database = fixture_database(ROOT, &["a.c"]);
    let facts = fixture_facts(
        &database,
        vec![fixture_command("a.c", &["__linux__"], &[], &[])],
        &["_WIN32"],
        &["a.c"],
    );
    let other = fixture_database(ROOT, &["b.c"]);
    let error = PredefinedMacros::derive(&facts, &other, Path::new(ROOT)).unwrap_err();
    assert!(error.contains("another compile database"), "{error}");
    let mut renamed = facts.clone();
    renamed.commands[0].key = "b.c".to_owned();
    let error = PredefinedMacros::derive(&renamed, &database, Path::new(ROOT)).unwrap_err();
    assert!(error.contains("not the database's"), "{error}");
}

#[test]
fn condition_names_reads_only_conditional_directives() {
    let text = "#define __NOT_A_CONDITION__ 1\nint __also_not__;\n#if defined (_WIN32) \\\n    || defined(__APPLE__) /* __IN_A_COMMENT__ */\n#elif __GNUC__ >= 4 // __TRAILING__\n#ifndef _MSC_VER\n#endif\n#endif\n";
    let names = condition_names(text);
    assert_eq!(
        names,
        ["_MSC_VER", "_WIN32", "__APPLE__", "__GNUC__"]
            .into_iter()
            .map(str::to_owned)
            .collect::<BTreeSet<_>>()
    );
}

#[test]
fn read_preprocessed_separates_the_built_in_and_command_line_segments() {
    let output = "# 1 \"/src/a.c\"\n# 1 \"<built-in>\" 1\n#define __linux__ 1\n# 1 \"<command line>\" 1\n#define __QCORE_D__ 1\n#undef __clang__\n# 1 \"<built-in>\" 2\n# 1 \"/src/a.c\" 2\n# 1 \"/usr/include/x.h\" 1 3 4\n#define __X_H 1\n#undef _WIN32\n# 3 \"/src/a.c\" 2\nint x;\n";
    let read = read_preprocessed(output);
    assert_eq!(read.file_defined, BTreeSet::from(["__X_H".to_owned()]));
    assert_eq!(read.file_undefined, BTreeSet::from(["_WIN32".to_owned()]));
    assert_eq!(
        read.files,
        BTreeSet::from(["/src/a.c".to_owned(), "/usr/include/x.h".to_owned()])
    );
}

// The recorded compiler, run for real (the gateway's Clang).

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The gateway's Clang as an absolute path (a recorded command names its
/// compiler by path; a bare name is resolved on `PATH` here, as a build
/// would have).
fn clang() -> String {
    let program = PathBuf::from(
        qcore_c_gateway::clang_command()
            .expect("Clang is available")
            .get_program(),
    );
    if program.is_absolute() {
        return program.display().to_string();
    }
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|directory| directory.join(&program))
        .find(|candidate| candidate.is_file())
        .expect("the gateway's Clang is on PATH")
        .display()
        .to_string()
}

/// A database of `a.c` (`-D__QCORE_ONE__ -U__QCORE_ONE__ -U__clang__`) and
/// `b.c` (`-D__QCORE_ONE__`), both including `common.h`; `a.c` also
/// includes `defines.h`, which `#define`s `__QCORE_HEADER__`.
fn real_build(name: &str, compiler: &str) -> (Scratch, String, BTreeSet<String>) {
    let root = std::env::temp_dir().join(format!("qcore-predefined-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("scratch")).expect("fixture directory");
    let common = "#ifdef __QCORE_ONE__\nint one_common;\n#endif\n#ifdef __QCORE_NEVER__\nint never;\n#endif\n";
    let a = "#include \"defines.h\"\n#include \"common.h\"\n#ifdef __QCORE_ONE__\nint one;\n#endif\n#if defined (__clang__)\nint clang_a;\n#endif\n#ifdef __QCORE_HEADER__\nint header;\n#endif\n#ifdef __STDC__\nint stdc;\n#endif\n";
    let b = "#include \"common.h\"\n#ifdef __QCORE_ONE__\nint one;\n#endif\n#if defined (__clang__)\nint clang_b;\n#endif\n";
    for (file, text) in [
        ("common.h", common),
        ("defines.h", "#define __QCORE_HEADER__ 1\n"),
        ("a.c", a),
        ("b.c", b),
    ] {
        fs::write(root.join(file), text).expect("fixture file");
    }
    let entry = |file: &str, flags: &[&str]| {
        let mut arguments = vec![compiler.to_owned(), "--driver-mode=gcc".to_owned()];
        arguments.extend(flags.iter().map(|f| (*f).to_owned()));
        arguments.extend([
            "-c".to_owned(),
            root.join(file).display().to_string(),
            "-o".to_owned(),
            format!("{file}.o"),
        ]);
        serde_json::json!({
            "directory": root.display().to_string(),
            "file": root.join(file).display().to_string(),
            "arguments": arguments,
        })
    };
    let database = serde_json::to_string(&vec![
        entry(
            "a.c",
            &["-D__QCORE_ONE__", "-U__QCORE_ONE__", "-U__clang__"],
        ),
        entry("b.c", &["-D__QCORE_ONE__"]),
    ])
    .expect("database JSON");
    let mut candidates = BTreeSet::new();
    for text in [common, a, b] {
        candidates.extend(condition_names(text));
    }
    (Scratch(root), database, candidates)
}

fn request(
    root: &Path,
    database: &str,
    candidates: BTreeSet<String>,
    recorded: &str,
) -> FactsRequest {
    FactsRequest {
        database_text: database.to_owned(),
        root: root.to_owned(),
        recorded_compiler_version: recorded.to_owned(),
        candidates,
        extras: Vec::<ExtraUnit>::new(),
        scratch: root.join("scratch"),
        jobs: 2,
    }
}

#[test]
fn the_recorded_compiler_decides_per_command_and_a_minus_u_undefines() {
    let compiler = clang();
    let (scratch, database, candidates) = real_build("per-command", &compiler);
    let recorded = identify(&compiler).version;
    let facts =
        derive_facts(&request(&scratch.0, &database, candidates, &recorded)).expect("facts");
    for command in &facts.commands {
        assert_eq!(command.failure, None, "{command:?}");
    }
    let view = PredefinedMacros::derive(&facts, &database, &scratch.0).expect("the facts match");
    // `-D` then `-U`: undefined in a.c; defined in b.c.
    assert_eq!(view.decide("a.c", "__QCORE_ONE__"), Some(false));
    assert_eq!(view.decide("b.c", "__QCORE_ONE__"), Some(true));
    // A predefined macro the command undefines, and one it keeps.
    assert_eq!(view.decide("a.c", "__clang__"), Some(false));
    assert_eq!(view.decide("b.c", "__clang__"), Some(true));
    // MS-compatible Clang need not define __STDC__. Independently observe the
    // configured driver instead of assuming its native platform's macros.
    let predefined = std::process::Command::new(&compiler)
        .args(["--driver-mode=gcc", "-dM", "-E", "-x", "c", "-"])
        .stdin(std::process::Stdio::null())
        .output()
        .expect("the configured compiler's predefined macros");
    assert!(predefined.status.success());
    let stdc = String::from_utf8(predefined.stdout)
        .unwrap()
        .lines()
        .any(|line| line.split_whitespace().nth(1) == Some("__STDC__"));
    assert_eq!(view.decide("a.c", "__STDC__"), Some(stdc));
    // A header `#define`s it: undecided even though no command defines it.
    assert_eq!(view.decide("a.c", "__QCORE_HEADER__"), None);
    // The common header: one command defines `__QCORE_ONE__`, none
    // `__QCORE_NEVER__`.
    assert_eq!(view.decide("common.h", "__QCORE_ONE__"), None);
    assert_eq!(view.decide("common.h", "__QCORE_NEVER__"), Some(false));
    // The `-dM` listing agrees with the probe.
    let b = &facts.commands[1];
    assert!(b.predefined.contains("__clang__") && b.predefined.contains("__QCORE_ONE__"));
    assert!(!facts.commands[0].predefined.contains("__QCORE_ONE__"));
    let identity = &facts.compilers[&compiler];
    assert!(!identity.sha256.is_empty() && !identity.resolved.is_empty());
}

#[test]
fn nc_the_recorded_compiler_mismatch_decides_nothing_when_run() {
    let compiler = clang();
    let (scratch, database, candidates) = real_build("mismatch", &compiler);
    let facts = derive_facts(&request(
        &scratch.0,
        &database,
        candidates,
        "clang version 0.0.0 (not the recorded build's)",
    ))
    .expect("facts");
    assert!(facts.commands.iter().all(|c| {
        c.failure
            .as_deref()
            .is_some_and(|f| f.contains("not the recorded build's"))
    }));
    assert!(PredefinedMacros::derive(&facts, &database, &scratch.0).is_err());
}

#[test]
fn nc_an_unavailable_compiler_decides_nothing() {
    let (scratch, database, candidates) =
        real_build("unavailable", "/nonexistent/qcore-no-such-compiler");
    let facts =
        derive_facts(&request(&scratch.0, &database, candidates, "cc version 1")).expect("facts");
    assert!(facts.compilers.values().all(|c| c.failure.is_some()));
    let error = PredefinedMacros::derive(&facts, &database, &scratch.0).unwrap_err();
    assert!(error.contains("unavailable"), "{error}");
}
