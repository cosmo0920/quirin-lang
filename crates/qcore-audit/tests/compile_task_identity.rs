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

//! The compile-task identity contract for the supported C import configuration.
//!
//! Every bound component must invalidate the identity on its own, including the
//! ones that leave the selected `.c` file and even the preprocessed token stream
//! untouched. Ambiguous compilation databases must never resolve themselves by
//! choosing a configuration.

use qcore_audit::compile_db::{
    CompileEntry, EffectiveInput, compile_entries, compile_task_identity, compiler_options,
    select_unique_configuration,
};
use qcore_c_gateway::ClangIdentity;
use qcore_c_gateway::legacy_c::{C11TranslationUnit, validate_unrestricted_c11};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{fs, path::Path};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "qcore-task-identity-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for name in ["src", "one", "two"] {
            fs::create_dir_all(path.join(name)).unwrap();
        }
        Self(path)
    }

    fn header(&self, directory: &str, value: i32) {
        fs::write(
            self.0.join(directory).join("choice.h"),
            format!("#define CHOICE {value}\n"),
        )
        .unwrap();
    }

    fn entry(&self, options: &[&str]) -> CompileEntry {
        self.entry_with_compiler("clang", options)
    }

    fn clang_entry(&self, options: &[&str]) -> CompileEntry {
        let command = qcore_c_gateway::clang_command().unwrap();
        let compiler = command.get_program().to_str().unwrap();
        self.entry_with_compiler(compiler, options)
    }

    fn entry_with_compiler(&self, compiler: &str, options: &[&str]) -> CompileEntry {
        let mut arguments = vec![compiler, "-std=c11"];
        arguments.extend_from_slice(options);
        arguments.extend(["-c", "src/unit.c"]);
        compile_entries(
            &serde_json::json!([{"directory": self.0, "file": "src/unit.c", "arguments": arguments}])
                .to_string(),
        )
        .unwrap()
        .remove(0)
    }

    /// Build the identity exactly as the configured driver does, including the
    /// declared-file transport: the unit's text is published at the path the
    /// compile record names, because that is the input Clang is handed.
    fn identity(&self, source: &str, options: &[&str], dependencies: &[(&str, &str)]) -> Value {
        fs::write(self.0.join("src/unit.c"), source).unwrap();
        let entry = self.clang_entry(options);
        let resolved = compiler_options(&entry).unwrap();
        let unit = C11TranslationUnit {
            source,
            include_paths: resolved.include_paths,
            definitions: resolved.definitions,
            preprocessor_options: resolved.preprocessor_options,
            dialect: resolved.dialect,
            input: resolved.input,
            ..Default::default()
        };
        let effective = validate_unrestricted_c11(&unit).map_err(|error| error.to_string());
        let identity = compile_task_identity(
            &entry,
            &unit,
            match &effective {
                Ok(text) => EffectiveInput::Accepted(text),
                Err(error) => EffectiveInput::Rejected(error),
            },
            &ClangIdentity::capture().unwrap(),
            dependencies
                .iter()
                .map(|(name, digest)| ((*name).to_owned(), (*digest).to_owned()))
                .collect::<BTreeMap<_, _>>(),
        )
        .unwrap();
        let mut value = serde_json::to_value(&identity).unwrap();
        value["digest"] = identity.digest().unwrap().into();
        value
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            false
        }
    }
}

const SOURCE: &str = "#include <choice.h>\nint observed(void) { return CHOICE; }";

#[test]
fn identity_binds_compiler_target_dialect_directory_ordered_arguments_and_effective_input() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    fixture.header("one", 17);
    let identity = fixture.identity(
        SOURCE,
        &["-Ione", "-iquote", "two"],
        &[("source_profile", "abc")],
    );
    assert_eq!(identity["schema_version"], 1);
    assert!(
        identity["compiler"]["version"]
            .as_str()
            .unwrap()
            .contains("clang version")
    );
    assert!(!identity["compiler"]["target"].as_str().unwrap().is_empty());
    assert!(!identity["compiler"]["program"].as_str().unwrap().is_empty());
    assert_eq!(identity["dialect"], "c11");
    assert_eq!(identity["function_scope"], "preprocessed_translation_unit");
    assert_eq!(
        Path::new(identity["directory"].as_str().unwrap()),
        fixture.0
    );
    assert_eq!(
        Path::new(identity["file"].as_str().unwrap()),
        fixture.0.join("src/unit.c")
    );
    // The complete ordered argument vector, not a filtered projection.
    let arguments = identity["arguments"].as_array().unwrap();
    assert_eq!(
        arguments.last().unwrap().as_str().unwrap(),
        "src/unit.c",
        "{arguments:?}"
    );
    assert!(arguments.iter().any(|word| word == "-c"));
    assert!(arguments.iter().any(|word| word == "-std=c11"));
    assert!(arguments.iter().any(|word| word == "-Ione"));
    assert_eq!(identity["declared_driver"], arguments[0]);
    assert_eq!(
        identity["raw_source_sha256"],
        qcore_schemas::sha256_hex(SOURCE.as_bytes())
    );
    assert!(
        identity["preprocessed_source_sha256"]
            .as_str()
            .unwrap()
            .len()
            == 64
    );
    assert!(identity["preprocessing_error"].is_null());
    assert_eq!(identity["dependencies"]["source_profile"], "abc");
    // The bound quote-include class survives into the recorded configuration.
    // The command's own `-iquote` is the one that must survive: the includer's
    // own directory is no longer approximated by a synthesized quote path, it
    // comes from the declared input recorded below.
    assert!(
        identity["preprocessor_options"]
            .as_array()
            .unwrap()
            .iter()
            .any(|option| option.get("QuoteInclude").is_some()),
        "{:?}",
        identity["preprocessor_options"]
    );
    // How the input actually reached Clang is part of the identity. Two runs
    // that agree on directory, file and arguments but transport the source
    // differently are different tasks, because `__FILE__` and the
    // includer-directory rule answer differently.
    assert_eq!(
        identity["source_input"]["DeclaredFile"]["spelled"], "src/unit.c",
        "{:?}",
        identity["source_input"]
    );
    assert_eq!(
        Path::new(
            identity["source_input"]["DeclaredFile"]["directory"]
                .as_str()
                .unwrap()
        ),
        fixture.0
    );
}

#[test]
fn the_input_transport_is_part_of_the_identity() {
    if !clang_available() {
        return;
    }
    // A unit fed through standard input sees `<stdin>` for its file-name macros
    // and resolves quoted includes elsewhere. Recording the compile record's
    // directory and arguments while transporting the source some other way
    // would make the recorded identity describe a compile that never ran.
    let fixture = Fixture::new();
    fixture.header("one", 17);
    let declared = fixture.identity(SOURCE, &["-Ione"], &[]);
    fs::write(fixture.0.join("src/unit.c"), SOURCE).unwrap();
    let entry = fixture.clang_entry(&["-Ione"]);
    let resolved = compiler_options(&entry).unwrap();
    let unit = C11TranslationUnit {
        source: SOURCE,
        include_paths: resolved.include_paths,
        definitions: resolved.definitions,
        preprocessor_options: resolved.preprocessor_options,
        dialect: resolved.dialect,
        input: qcore_c_gateway::legacy_c::CSourceInput::HistoricalStdin,
        ..Default::default()
    };
    let effective = validate_unrestricted_c11(&unit).unwrap();
    let historical = compile_task_identity(
        &entry,
        &unit,
        EffectiveInput::Accepted(&effective),
        &ClangIdentity::capture().unwrap(),
        BTreeMap::new(),
    )
    .unwrap();
    assert_ne!(
        historical.digest().unwrap(),
        declared["digest"].as_str().unwrap(),
        "the transport must not be invisible to the identity"
    );
}

#[test]
fn a_changed_used_header_invalidates_the_identity_with_an_unchanged_main_source() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    fixture.header("one", 17);
    let before = fixture.identity(SOURCE, &["-Ione"], &[]);
    fixture.header("one", 29);
    let after = fixture.identity(SOURCE, &["-Ione"], &[]);
    assert_eq!(before["raw_source_sha256"], after["raw_source_sha256"]);
    assert_eq!(before["arguments"], after["arguments"]);
    assert_ne!(
        before["preprocessed_source_sha256"],
        after["preprocessed_source_sha256"]
    );
    assert_ne!(before["digest"], after["digest"]);
}

#[test]
fn include_order_and_admitted_macro_options_invalidate_a_byte_identical_analysis() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    // Only `one` supplies the header, so swapping the search order cannot change
    // the preprocessed token stream. The configuration is still different.
    fixture.header("one", 17);
    let first = fixture.identity(SOURCE, &["-Ione", "-Itwo"], &[]);
    let swapped = fixture.identity(SOURCE, &["-Itwo", "-Ione"], &[]);
    assert_eq!(
        first["preprocessed_source_sha256"],
        swapped["preprocessed_source_sha256"]
    );
    assert_ne!(
        first["preprocessor_options"],
        swapped["preprocessor_options"]
    );
    assert_ne!(first["digest"], swapped["digest"]);

    // An admitted macro option the source never reads is still part of the task.
    let unused = fixture.identity(SOURCE, &["-Ione", "-Itwo", "-DNEVER_READ=1"], &[]);
    assert_eq!(
        first["preprocessed_source_sha256"],
        unused["preprocessed_source_sha256"]
    );
    assert_ne!(first["digest"], unused["digest"]);

    // Definition and undefinition order remains observable in the identity.
    let defined = fixture.identity(SOURCE, &["-Ione", "-DCHOICE=1", "-UCHOICE"], &[]);
    let undefined = fixture.identity(SOURCE, &["-Ione", "-UCHOICE", "-DCHOICE=1"], &[]);
    assert_ne!(
        defined["preprocessor_options"],
        undefined["preprocessor_options"]
    );
    assert_ne!(defined["digest"], undefined["digest"]);

    // A different dialect is a different task even for identical output.
    let entry = fixture.entry(&["-Ione"]);
    let gnu = CompileEntry {
        command: entry.command.replace("-std=c11", "-std=gnu11"),
        ..entry
    };
    let resolved = compiler_options(&gnu).unwrap();
    let unit = C11TranslationUnit {
        source: SOURCE,
        include_paths: resolved.include_paths,
        definitions: resolved.definitions,
        preprocessor_options: resolved.preprocessor_options,
        dialect: resolved.dialect,
        ..Default::default()
    };
    let preprocessed = validate_unrestricted_c11(&unit).unwrap();
    let identity = compile_task_identity(
        &gnu,
        &unit,
        EffectiveInput::Accepted(&preprocessed),
        &ClangIdentity::capture().unwrap(),
        BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(serde_json::to_value(&identity).unwrap()["dialect"], "gnu11");
    assert_ne!(
        serde_json::Value::from(identity.digest().unwrap()),
        first["digest"]
    );
}

#[test]
fn contract_profile_and_database_dependency_changes_invalidate_the_identity() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    fixture.header("one", 17);
    let base = [
        ("compile_database", "d0"),
        ("resource_contract_manifest", "c0"),
        ("source_profile", "p0"),
    ];
    let original = fixture.identity(SOURCE, &["-Ione"], &base);
    for replaced in 0..base.len() {
        let mut changed = base;
        changed[replaced].1 = "rotated";
        let identity = fixture.identity(SOURCE, &["-Ione"], &changed);
        assert_eq!(
            original["preprocessed_source_sha256"],
            identity["preprocessed_source_sha256"]
        );
        assert_ne!(
            original["digest"], identity["digest"],
            "dependency `{}` did not invalidate the task identity",
            base[replaced].0
        );
    }
    // An added dependency is a change too; nothing silently drops out.
    let extended = fixture.identity(
        SOURCE,
        &["-Ione"],
        &[
            ("compile_database", "d0"),
            ("resource_contract_manifest", "c0"),
            ("source_profile", "p0"),
            ("source_contract_rules", "r0"),
        ],
    );
    assert_ne!(original["digest"], extended["digest"]);
}

#[test]
fn a_rejected_preprocessing_run_keeps_an_attributable_identity() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    let rejected = fixture.identity(
        "#include <absent-header.h>\nint value(void) { return 1; }",
        &[],
        &[],
    );
    assert!(rejected["preprocessed_source_sha256"].is_null());
    assert!(!rejected["preprocessing_error"].as_str().unwrap().is_empty());
    fixture.header("one", 17);
    let accepted = fixture.identity(SOURCE, &["-Ione"], &[]);
    assert_ne!(rejected["digest"], accepted["digest"]);
}

#[test]
fn duplicate_compile_records_for_one_file_are_kept_distinct_and_never_chosen_between() {
    let fixture = Fixture::new();
    let file = fixture.0.join("src/unit.c");
    let shared = fixture.entry(&["-Ione", "-DEVENT_SHARED_EXPORTS"]);
    let statik = fixture.entry(&["-Ione"]);
    let repeated = fixture.entry(&["-Ione", "-DEVENT_SHARED_EXPORTS"]);

    // A single configuration, however many times the database repeats it.
    assert_eq!(
        select_unique_configuration(&[shared.clone(), repeated.clone()], &file)
            .unwrap()
            .command,
        shared.command
    );

    // Two real configurations: reject, and say how many were found.
    let error = select_unique_configuration(&[shared.clone(), statik.clone()], &file).unwrap_err();
    assert!(error.contains("2 distinct configurations"), "{error}");
    assert!(error.contains("will not choose one"), "{error}");

    // Differing only in the working directory is still a different task.
    let relocated = CompileEntry {
        directory: fixture.0.join("one"),
        ..shared.clone()
    };
    assert!(
        select_unique_configuration(&[shared.clone(), relocated], &file)
            .unwrap_err()
            .contains("distinct configurations")
    );

    // No command at all fails closed rather than defaulting.
    assert!(
        select_unique_configuration(&[], &file)
            .unwrap_err()
            .contains("no command")
    );
}
