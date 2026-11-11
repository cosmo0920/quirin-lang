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

//! Card CG30-DIALECTS-FLAGS-TYPES (gateway rule 11y): structured target and
//! dialect selections in the compile-database reader and the compile-task
//! identity. A changed target or dialect is a different evidence identity.

use std::collections::BTreeMap;

use qcore_audit::compile_db::{
    CompileEntry, EffectiveInput, compile_task_identity, compiler_options,
};
use qcore_c_gateway::ClangIdentity;
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, CCompilerDialect, CFrontendFlag, CPreprocessorOption,
    validate_unrestricted_c11,
};

/// The configured Clang driver, or `None` (with the reason) when absent.
fn driver() -> Option<String> {
    match qcore_c_gateway::clang_command() {
        Ok(command) => command.get_program().to_str().map(str::to_owned),
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent CG30 control: {reason}");
            None
        }
    }
}

/// One fresh directory per entry: the tests run in parallel, and a unit file
/// another test is rewriting would not hold its declared bytes.
fn entry(driver: &str, extra: &str) -> CompileEntry {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let tag = NEXT
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        .to_string();
    entry_in(driver, extra, &tag)
}

/// An entry in the directory named by `tag` (the identity binds the
/// directory, so a repeated digest must reuse it).
fn entry_in(driver: &str, extra: &str, tag: &str) -> CompileEntry {
    let directory = std::env::temp_dir().join(format!("qcore-cg30-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let file = directory.join("unit.c");
    std::fs::write(&file, "int subject(int a) { return a; }\n").unwrap();
    CompileEntry {
        command: format!(
            "\"{driver}\" {extra} -c '{}' -o '{}'",
            file.display(),
            directory.join("unit.o").display()
        ),
        directory,
        file,
    }
}

fn targets(options: &[CPreprocessorOption]) -> Vec<String> {
    options
        .iter()
        .filter_map(|option| match option {
            CPreprocessorOption::FrontendFlag(CFrontendFlag::Target(triple)) => {
                Some(format!("target {}", triple.spelled()))
            }
            CPreprocessorOption::FrontendFlag(CFrontendFlag::Abi(abi)) => {
                Some(format!("abi {}", abi.name()))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn structured_selections_are_transported() {
    let Some(driver) = driver() else { return };
    let options = compiler_options(&entry(
        &driver,
        "-std=c23 --target=x86_64-pc-linux-gnu -mabi=sysv",
    ))
    .expect("admitted");
    assert_eq!(options.dialect, CCompilerDialect::C23);
    assert_eq!(
        targets(&options.preprocessor_options),
        vec!["target x86_64-pc-linux-gnu", "abi sysv"]
    );
    // Clang reads the unit under the transported target.
    let source = "int subject(int a) { return a; }\n";
    let unit = C11TranslationUnit {
        source,
        dialect: options.dialect,
        preprocessor_options: options.preprocessor_options,
        input: options.input,
        ..Default::default()
    };
    validate_unrestricted_c11(&unit).expect("Clang preprocesses under the target");
}

#[test]
fn selections_are_refused_by_name() {
    let Some(driver) = driver() else { return };
    for (extra, fragment) in [
        ("-std=gnu17 --target=i686-pc-linux-gnu", "data model ILP32"),
        (
            "-std=gnu17 --target=x86_64-pc-windows-msvc",
            "data model LLP64",
        ),
        (
            "-std=gnu17 --target=x86_64-pc-linux-gnux32",
            "data model ILP32",
        ),
        ("-std=gnu17 -mabi=ilp32", "data model ILP32"),
        (
            "-std=gnu17 -mabi=lp128",
            "unsupported compilation ABI `lp128`",
        ),
        (
            "-std=gnu17 --target=sparc-sun-solaris",
            "architecture `sparc`",
        ),
        (
            "-std=gnu17 --target=x86_64-pc-linux-gnu -target aarch64-unknown-linux-gnu",
            "selects a target twice",
        ),
        ("-std=gnu17 -arch i386", "data model ILP32"),
        (
            "-std=gnu17 -arch arm64 --target=aarch64-apple-darwin",
            "select a target twice",
        ),
        (
            "--target=x86_64-pc-linux-gnu",
            "explicit supported -std= dialect",
        ),
        ("-std=gnu23", "unsupported compilation dialect `gnu23`"),
    ] {
        let error = compiler_options(&entry(&driver, extra))
            .err()
            .unwrap_or_else(|| panic!("{extra} was admitted"));
        assert!(error.contains(fragment), "{extra}: {error}");
        qcore_schemas::reason::ReasonCategory::classify(&error)
            .unwrap_or_else(|_| panic!("{extra}: unclassified reason {error}"));
    }
}

/// A changed target, ABI or dialect changes the compile-task digest; the same
/// command keeps it.
#[test]
fn the_task_identity_separates_targets_and_dialects() {
    let Some(driver) = driver() else { return };
    let compiler = ClangIdentity::recorded("clang", "21.1.8", "x86_64-pc-linux-gnu").unwrap();
    let digest = |extra: &str| {
        let entry = entry_in(&driver, extra, "identity");
        let options = compiler_options(&entry).expect(extra);
        let unit = C11TranslationUnit {
            source: "int subject(int a) { return a; }\n",
            dialect: options.dialect,
            preprocessor_options: options.preprocessor_options,
            include_paths: options.include_paths,
            definitions: options.definitions,
            input: options.input,
            ..Default::default()
        };
        compile_task_identity(
            &entry,
            &unit,
            EffectiveInput::Accepted("int subject(int a) { return a; }\n"),
            &compiler,
            BTreeMap::new(),
        )
        .unwrap()
        .digest()
        .unwrap()
    };
    let rows = [
        "-std=gnu17",
        "-std=c17",
        "-std=c23",
        "-std=c2y",
        "-std=gnu17 --target=x86_64-pc-linux-gnu",
        "-std=gnu17 --target=aarch64-unknown-linux-gnu",
        "-std=gnu17 --target=x86_64-pc-linux-gnu -mabi=ms",
    ];
    let digests = rows.map(digest);
    let mut distinct = digests.to_vec();
    distinct.sort();
    distinct.dedup();
    assert_eq!(distinct.len(), rows.len(), "{digests:?}");
    assert_eq!(digest(rows[4]), digests[4]);
}
