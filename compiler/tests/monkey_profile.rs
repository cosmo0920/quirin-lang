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

//! Data-driven typed bounds must not become optimistic when malformed or absent.
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_compiler::legacy_c::C11TranslationUnit;
use qcore_compiler::profile_loader::{LoadedProfile, load_profile_directory};
use qcore_compiler::source_profiles::{FLUENT_BIT_SOURCE_SET, load_profile, profiles_root};
use qcore_compiler::typed_source::{
    SourceOperation, SourceResult, import_source_functions_from_profile,
};
use qcore_compiler::{MonkeyCallback, MonkeyCallbackSummary};
use qcore_schemas::profile::ImportRole;
use serde_json::{Value, json};

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent Monkey profile test: {reason}");
            false
        }
    }
}

struct Scratch(PathBuf);
impl Scratch {
    fn new(edit: impl FnOnce(&mut Value)) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "qcore-monkey-profile-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let directory = root.join(FLUENT_BIT_SOURCE_SET);
        fs::create_dir_all(directory.join("providers")).unwrap();
        let original = profiles_root().join(FLUENT_BIT_SOURCE_SET);
        let mut value: Value =
            serde_json::from_str(&fs::read_to_string(original.join("profile.json")).unwrap())
                .unwrap();
        for name in value["provider_manifests"].as_array().unwrap() {
            let name = name.as_str().unwrap();
            fs::copy(original.join(name), directory.join(name)).unwrap();
        }
        edit(&mut value);
        fs::write(
            directory.join("profile.json"),
            serde_json::to_string_pretty(&value).unwrap(),
        )
        .unwrap();
        Self(root)
    }
    fn path(&self) -> PathBuf {
        self.0.join(FLUENT_BIT_SOURCE_SET)
    }
    fn load(&self) -> Result<LoadedProfile, String> {
        load_profile_directory(&self.path()).map_err(|e| e.to_string())
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn callback_signature_and_declaration_failures_reject_the_whole_profile() {
    for case in 0..8 {
        let scratch = Scratch::new(|value| {
            let library = &mut value["libraries"]["monkey"];
            match case {
                0 => {
                    library["typed_import"]["signature_constraints"]
                        .as_object_mut()
                        .unwrap()
                        .remove("cb_event");
                }
                1 => {
                    library["typed_import"]["signature_constraints"]["absent"] =
                        json!({"result":"void","parameters":[]})
                }
                2 => {
                    library["typed_import"]["signature_constraints"]["cb_event"]["result"] =
                        json!("")
                }
                3 => library["typed_import"]["records"] = json!([""]),
                4 => library["typed_import"]["functions"] = json!({}),
                5 => {
                    library["callback_summaries"]["absent"] =
                        library["callback_summaries"]["cb_event"].clone();
                }
                6 => library["typed_import"] = Value::Null,
                7 => library["typed_import"]["restricted_calls"] = json!(["send", "send"]),
                _ => unreachable!(),
            }
        });
        assert!(scratch.load().is_err(), "case {case}");
    }
}

#[test]
fn duplicate_json_keys_and_set_entries_cannot_disappear_during_deserialization() {
    for (needle, replacement) in [
        (
            "\"schema_version\": 1",
            "\"schema_version\": 1, \"schema_version\": 1",
        ),
        (
            "\"cb_event\": \"callback\"",
            "\"cb_event\": \"callback\", \"cb_event\": \"wrapper\"",
        ),
        ("\"records\": [", "\"records\": [\"mk_event\","),
        ("\"sizeof_types\": [", "\"sizeof_types\": [\"int\","),
    ] {
        let scratch = Scratch::new(|_| {});
        let path = scratch.path().join("profile.json");
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains(needle));
        fs::write(path, text.replacen(needle, replacement, 1)).unwrap();
        assert!(scratch.load().is_err(), "{needle}");
    }
    // A signature vector is positional, not a set; repeated argument types are legal.
    let scratch = Scratch::new(|v| {
        v["libraries"]["monkey"]["typed_import"]["signature_constraints"]["cb_event"]["parameters"] =
            json!(["int", "int"])
    });
    assert!(scratch.load().is_ok());
}

fn custom(role: &str, exact: bool) -> Scratch {
    Scratch::new(|v| {
        let library = &mut v["libraries"]["monkey"];
        library["typed_import"]["functions"] = json!({"callback_in_another_library":role});
        library["typed_import"]["signature_constraints"] = if exact {
            json!({"callback_in_another_library":{"result":"void","parameters":["int","short","void *"]}})
        } else {
            json!({})
        };
        library["callback_summaries"] = json!({});
    })
}

#[test]
fn explicit_profile_controls_names_and_callbacks_keep_their_call_allowlist() {
    if !clang_available() {
        return;
    }
    let source = "void outside(void); static void callback_in_another_library(int fd, short flags, void *data) { outside(); }";
    let scratch = custom("callback", true);
    let profile = scratch.load().unwrap();
    let unit = C11TranslationUnit {
        source,
        ..Default::default()
    };
    let revision = &profile.manifest().pin_for("monkey").unwrap().git_commit;
    let results = import_source_functions_from_profile(
        &unit,
        revision,
        &profile,
        "monkey",
        ImportRole::Callback,
    )
    .unwrap();
    assert!(matches!(&results[0], SourceResult::Unknown { .. }));
    assert!(custom("callback", false).load().is_err());
    let no_call = source.replace("outside();", "return;");
    let results = import_source_functions_from_profile(
        &C11TranslationUnit {
            source: &no_call,
            ..Default::default()
        },
        revision,
        &profile,
        "monkey",
        ImportRole::Callback,
    )
    .unwrap();
    let SourceResult::Summary(summary) = &results[0] else {
        panic!("{results:?}")
    };
    assert_eq!(summary.function(), "callback_in_another_library");
    assert_eq!(
        summary.profile_assumptions()["profile_digest"],
        profile.manifest_digest()
    );
    assert!(
        import_source_functions_from_profile(
            &unit,
            "wrong-revision",
            &profile,
            "monkey",
            ImportRole::Callback
        )
        .is_err()
    );
    assert!(
        import_source_functions_from_profile(
            &unit,
            revision,
            &profile,
            "missing",
            ImportRole::Callback
        )
        .is_err()
    );
}

#[test]
fn wrapper_roles_explicitly_preserve_opaque_calls_without_signature_constraints() {
    if !clang_available() {
        return;
    }
    let scratch = custom("wrapper", false);
    let profile = scratch.load().unwrap();
    let unit = C11TranslationUnit {
        source: "void outside(void); static void callback_in_another_library(int fd, int flags, void *data) { outside(); }",
        ..Default::default()
    };
    let results = import_source_functions_from_profile(
        &unit,
        &profile.manifest().pin_for("monkey").unwrap().git_commit,
        &profile,
        "monkey",
        ImportRole::Wrapper,
    )
    .unwrap();
    let SourceResult::Summary(summary) = &results[0] else {
        panic!("{results:?}")
    };
    assert!(summary.paths().iter().any(|p| {
        p.operations
            .iter()
            .any(|op| matches!(op,SourceOperation::Call{function,..} if function=="outside"))
    }));
    assert_eq!(
        summary.profile_assumptions()["typed_import_role"],
        "Wrapper"
    );
}

#[test]
fn declared_callback_effects_are_data_and_missing_entries_are_errors() {
    let profile = load_profile(FLUENT_BIT_SOURCE_SET).unwrap();
    assert!(
        MonkeyCallbackSummary::audited_from_profile(&profile, MonkeyCallback::Event)
            .unwrap()
            .writes_fired_array
    );
    let scratch = Scratch::new(|v| {
        v["libraries"]["monkey"]["callback_summaries"]["cb_event"]["writes_fired_array"] =
            json!(false)
    });
    assert!(
        !MonkeyCallbackSummary::audited_from_profile(
            &scratch.load().unwrap(),
            MonkeyCallback::Event
        )
        .unwrap()
        .writes_fired_array
    );
    let scratch = Scratch::new(|v| {
        v["libraries"]["monkey"]["callback_summaries"]
            .as_object_mut()
            .unwrap()
            .remove("cb_event");
    });
    assert!(
        MonkeyCallbackSummary::audited_from_profile(
            &scratch.load().unwrap(),
            MonkeyCallback::Event
        )
        .is_err()
    );
    let declared = profile
        .library("monkey")
        .unwrap()
        .typed_import
        .as_ref()
        .unwrap()
        .functions
        .iter()
        .filter_map(|(n, r)| (*r == ImportRole::Wrapper).then_some(n.as_str()))
        .collect::<BTreeSet<_>>();
    let text =
        include_str!("../../providers/qcore-provider-libevent/contracts/monkey/wrapper_paths.qv");
    let parsed = qcore_compiler::frontend::parse_source_program(text).unwrap();
    assert_eq!(
        declared,
        parsed
            .declarations
            .iter()
            .map(|d| d.value.name.as_str())
            .collect()
    );
}
