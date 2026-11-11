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

//! P06: the measured rejection boundary between CTraces and the implemented
//! source-contract capabilities, plus the designed replacement contract.
//!
//! This file establishes NOTHING about CTraces safety. Every assertion here is
//! either a refusal (`ChunkSourceResult::Unknown` with its exact reason) or a
//! structural pin on the designed contract text. There is deliberately no
//! positive path: no CTraces function reaches a checker today, so a passing
//! `Summary` assertion would be fabricated progress.
//!
//! Record: `docs/reports/remaining-p06/README.md`.
//! Design: `docs/design/CTRACES_PROVIDER_CAPABILITY_V1.md`.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_c_gateway::boolean_source::{ChunkSourceResult, import_chunk_source_with_profile};
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_contract::profile_loader::{LoadedProfile, load_profile_directory};
use qcore_frontend::frontend::{
    DeclarationKind, SourceContractClauseKind, SourceContractExitSelector, parse_source_program,
};
use serde_json::json;

/// Reduced excerpt of the real upstream bodies; see its header for what it is
/// not. The declared revision is the one the excerpt was taken from.
const EXCERPT: &str = include_str!("../../../docs/reports/remaining-p06/ctraces_span_excerpt.c");
const REVISION: &str = "9f0fa5730fba1761ebaf14bcd31220a3a9be9d6e";
const RULES: &str = include_str!(
    "../../../providers/quirin-provider-shared/contracts/library/source_projection.qv"
);
const DESIGNED_CONTRACT: &str =
    include_str!("../../../docs/reports/remaining-p06/ctraces_span_dropped_counts.qv");

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-p06-ctraces-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    /// A hermetic profile declaring one Boolean field model per record. The
    /// declaration is what a CTraces adapter would have to write today; the
    /// point of the test is that declaring it changes nothing.
    fn profile(&self, models: &[(&str, &str)]) -> LoadedProfile {
        let mut libraries = serde_json::Map::new();
        for (index, (record, field)) in models.iter().enumerate() {
            libraries.insert(
                format!("ctraces{index}"),
                json!({
                    "pin": "ctraces",
                    "source_roots": [],
                    "audited_files": [],
                    "source_witnesses": [],
                    "permitted_mechanisms": [],
                    "operation_summaries": {},
                    "api_dispositions": {},
                    "accepted_builds": [],
                    "required_evidence": [],
                    "typed_import": null,
                    "callback_summaries": {},
                    "boolean_field_models": [{
                        "record": record,
                        "field": field,
                        "set_operation": "lock",
                        "clear_operation": "unlock",
                    }],
                    "provider": null,
                }),
            );
        }
        let profile = json!({
            "schema_version": 1,
            "source_set": self.0.file_name().unwrap().to_str().unwrap(),
            "pins": {"ctraces": {
                "git_commit": REVISION,
                "clean_paths": [],
                "description": "declared CTraces excerpt revision",
            }},
            "libraries": serde_json::Value::Object(libraries),
            "resource_contract_manifest": null,
            "provider_manifests": [],
        });
        fs::write(self.0.join("profile.json"), profile.to_string()).unwrap();
        load_profile_directory(&self.0).unwrap()
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
            eprintln!("skipping Clang-dependent CTraces boundary test: {reason}");
            false
        }
    }
}

fn import(profile: &LoadedProfile, libraries: &[&str]) -> Vec<ChunkSourceResult> {
    import_chunk_source_with_profile(
        &C11TranslationUnit {
            source: EXCERPT,
            ..Default::default()
        },
        REVISION,
        profile,
        libraries,
        &[],
        RULES,
    )
    .unwrap()
}

fn reason<'a>(results: &'a [ChunkSourceResult], name: &str) -> &'a str {
    for result in results {
        match result {
            ChunkSourceResult::Unknown {
                function, reason, ..
            } if function == name => return reason,
            ChunkSourceResult::Summary(summary) => panic!(
                "no CTraces function may produce a Boolean-field summary; got one for `{}`",
                summary.function()
            ),
            _ => {}
        }
    }
    panic!("function `{name}` was not enumerated");
}

/// P06-B1. The CTraces span setters carry a second scalar formal, so they never
/// reach the Boolean-field statement grammar: the ABI selector rejects first.
/// `ctr_span_set_flags` is included because its `int` result would otherwise
/// look like the i32 Boolean profile.
#[test]
fn ctraces_span_setters_are_refused_by_the_boolean_field_abi() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    let profile = fixture.profile(&[("ctrace_span", "kind")]);
    let results = import(&profile, &["ctraces0"]);
    const ABI: &str = "outside single borrowed chunk / int-or-void-result source profile";
    for function in [
        "ctr_span_set_flags",
        "ctr_span_set_dropped_link_count",
        "ctr_span_set_dropped_events_count",
        "ctr_span_set_dropped_links_count",
        "ctr_span_set_dropped_attributes_count",
    ] {
        assert_eq!(reason(&results, function), ABI, "{function}");
    }
}

/// P06-B2. Naming the field the setters actually write changes nothing: the
/// same ABI refusal fires for every definition in the excerpt, so the field
/// layout gate is never even reached. That gate is a further, independent
/// blocker — `validate_field_layout` requires the selected member's `qualType`
/// to be exactly `int`, and every CTraces count is `uint32_t` — but it is
/// established by reading that function, not by this test, because no CTraces
/// definition has the one-parameter signature that would reach it.
#[test]
fn declaring_the_written_count_field_changes_no_ctraces_outcome() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    let profile = fixture.profile(&[("ctrace_span", "dropped_links_count")]);
    let results = import(&profile, &["ctraces0"]);
    assert_eq!(results.len(), 6);
    for function in [
        "ctr_span_set_flags",
        "ctr_span_set_dropped_link_count",
        "ctr_span_set_dropped_events_count",
        "ctr_span_set_dropped_links_count",
        "ctr_span_set_dropped_attributes_count",
        "ctr_opts_exit",
    ] {
        assert_eq!(
            reason(&results, function),
            "outside single borrowed chunk / int-or-void-result source profile",
            "{function}"
        );
    }
}

/// P06-B3. `ctr_opts_exit` is the single CTraces definition whose Clang
/// signature the Boolean-field importer accepts (`void (struct ctrace_opts *)`)
/// over a record whose only member is a plain `int`. It is still refused,
/// because its body reads the parameter itself rather than the modeled field.
/// It is also already `Safe` in the resource projection, so a capability built
/// on it would discharge nothing.
#[test]
fn ctraces_opts_exit_matches_the_abi_and_is_still_refused() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    let profile = fixture.profile(&[("ctrace_opts", "_make_windows_happy")]);
    let results = import(&profile, &["ctraces0"]);
    let refusal = reason(&results, "ctr_opts_exit");
    // Measured: the `!opts` guard reads the formal itself, and a bare
    // `DeclRefExpr` is outside the Boolean-field expression vocabulary
    // (Integer, the modeled field, `!`, unary `-`, `==`, `!=`).
    assert_eq!(refusal, "unmodeled expression DeclRefExpr");
}

/// P06-B4. The designed replacement contract parses and carries exactly the
/// clause vocabulary the design specifies. It is archived under `docs/`, is
/// loaded by no production route, and none of its clauses is in any implemented
/// checker's premise set — that gap is follow-on unit P06-I1.
#[test]
fn designed_ctraces_contract_parses_with_its_specified_vocabulary() {
    let program = parse_source_program(DESIGNED_CONTRACT).unwrap();
    let functions: Vec<_> = program
        .declarations
        .iter()
        .map(|declaration| &declaration.value)
        .collect();
    assert_eq!(functions.len(), 4);
    let expected = [
        (
            "ctr_span_set_dropped_attributes_count",
            "scalar_field__dropped_attr_count",
        ),
        (
            "ctr_span_set_dropped_events_count",
            "scalar_field__dropped_events_count",
        ),
        (
            "ctr_span_set_dropped_links_count",
            "scalar_field__dropped_links_count",
        ),
        (
            // The same field as the previous entry: the upstream names differ by
            // one character and the checker must read the field from source.
            "ctr_span_set_dropped_link_count",
            "scalar_field__dropped_links_count",
        ),
    ];
    for (declaration, (name, field_premise)) in functions.iter().zip(expected) {
        assert_eq!(declaration.name, name);
        assert_eq!(declaration.kind, DeclarationKind::ExternFunction);
        assert_eq!(declaration.parameters.len(), 1);
        let clauses = &declaration.function_contract.as_ref().unwrap().clauses;
        let requires: Vec<_> = clauses
            .iter()
            .filter(|clause| clause.value.kind == SourceContractClauseKind::Requires)
            .map(|clause| clause.value.expression_source.as_str())
            .collect();
        assert_eq!(
            requires,
            [
                "object_is_live",
                "exclusive_object_access",
                "c_uint32_matches_u32",
                field_premise,
            ]
        );
        let ensures: Vec<_> = clauses
            .iter()
            .filter(|clause| {
                clause.value.kind
                    == SourceContractClauseKind::Ensures(SourceContractExitSelector::Normal)
            })
            .map(|clause| clause.value.expression_source.as_str())
            .collect();
        assert_eq!(
            ensures,
            [
                "field == assigned_count",
                "only_selected_field_written",
                "no_object_release",
            ]
        );
    }
}
