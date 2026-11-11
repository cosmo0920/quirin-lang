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

//! Source-derived metadata helpers for unrelated declared Boolean record models.
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_c_gateway::boolean_source::{
    ChunkFrameSummary, ChunkSourceResult, import_chunk_frame_helper,
    import_chunk_frame_helper_with_profile, import_chunk_source_with_profile,
};
use qcore_c_gateway::legacy_c::{C11TranslationUnit, CCompilerDialect};
use qcore_contract::profile_loader::{LoadedProfile, load_profile_directory};
use serde_json::json;

const REVISION: &str = "1234567890abcdef1234567890abcdef12345678";
const RULES: &str = include_str!(
    "../../../providers/quirin-provider-shared/contracts/library/source_projection.qv"
);
const LAYOUT: &str = "struct sample_record { int ready; int other_model; int metadata; };";
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-generic-frames-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }
    fn profile(
        &self,
        record: &str,
        field: &str,
        extra_model: bool,
        changed: bool,
    ) -> LoadedProfile {
        let library = |selected_field: &str| json!({"pin":"sample","source_roots":[],"audited_files":[],"source_witnesses":[],"permitted_mechanisms":[],"operation_summaries":{},"api_dispositions":{},"accepted_builds":[],"required_evidence":[],"typed_import":null,"callback_summaries":{},"boolean_field_models":[{"record":record,"field":selected_field,"set_operation":"lock","clear_operation":"unlock"}],"provider":null});
        let mut libraries = json!({"sample":library(field)});
        if extra_model {
            libraries["unselected"] = library("other_model");
        }
        let profile = json!({"schema_version":1,"source_set":self.0.file_name().unwrap().to_str().unwrap(),"pins":{"sample":{"git_commit":REVISION,"clean_paths":[],"description":if changed {"changed declared premise"} else {"declared test revision"}}},"libraries":libraries,"resource_contract_manifest":null,"provider_manifests":[]});
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
            false
        }
    }
}
fn helper(profile: &LoadedProfile, source: &str, rules: &str) -> Result<ChunkFrameSummary, String> {
    import_chunk_frame_helper_with_profile(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        REVISION,
        "reset",
        profile,
        "sample",
        "sample_record",
        rules,
    )
}
fn caller(
    profile: &LoadedProfile,
    source: &str,
    helpers: &[ChunkFrameSummary],
    rules: &str,
) -> Result<Vec<ChunkSourceResult>, String> {
    import_chunk_source_with_profile(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        REVISION,
        profile,
        &["sample"],
        helpers,
        rules,
    )
}
fn caller_source() -> String {
    format!(
        "{LAYOUT} void reset(struct sample_record *); int run(struct sample_record *p) {{ if (!p->ready) {{ return -1; }} reset(p); p->ready = 0; return 0; }}"
    )
}
#[test]
fn unrelated_record_helper_preserves_ordered_writes_and_both_source_prestates() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    let profile = fixture.profile("sample_record", "ready", false, false);
    let source = format!(
        "{LAYOUT} void reset(struct sample_record *p) {{ p->metadata = -3; p->metadata = 0; }}"
    );
    let helper = helper(&profile, &source, RULES).unwrap();
    assert_eq!(helper.record(), "sample_record");
    assert_eq!(
        helper
            .protected_fields()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["ready"]
    );
    let results = caller(&profile, &caller_source(), &[helper], RULES).unwrap();
    let summary = results
        .iter()
        .find_map(|result| match result {
            ChunkSourceResult::Summary(summary) if summary.function() == "run" => Some(summary),
            _ => None,
        })
        .unwrap();
    assert_eq!(summary.paths()[0].frame_writes, []);
    assert_eq!(summary.paths()[0].return_value, -1);
    assert_eq!(
        summary.paths()[1].frame_writes,
        [("metadata".into(), -3), ("metadata".into(), 0)]
    );
    assert_eq!(summary.paths()[1].writes, [false]);
    assert_eq!(summary.dependencies().len(), 1);
    assert!(
        summary.source_input().program
            != qcore_schemas::chunk_summary::BooleanFieldStatement::Block(vec![])
    );
}
#[test]
fn every_modeled_field_is_protected_even_with_permissive_policy() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    let profile = fixture.profile("sample_record", "ready", true, false);
    let permissive =
        "module contract; extern fn frame_write() -> unit contract { ensures normal 1; };";
    for field in ["ready", "other_model"] {
        let source = format!("{LAYOUT} void reset(struct sample_record *p) {{ p->{field} = 0; }}");
        assert!(helper(&profile, &source, permissive).is_err());
    }
    let source = format!("{LAYOUT} void reset(struct sample_record *p) {{ p->metadata = 0; }}");
    assert_eq!(
        helper(&profile, &source, RULES)
            .unwrap()
            .protected_fields()
            .len(),
        2
    );
}
#[test]
fn helper_grammar_never_erases_nonconstant_effects_or_aliases() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    let profile = fixture.profile("sample_record", "ready", false, false);
    for body in [
        "p->metadata = p->other_model;",
        "p->metadata++;",
        "if (p->ready) { p->metadata = 0; }",
        "struct sample_record *q = p; q->metadata = 0;",
        "p->metadata = 0; return;",
        "p->metadata = 2147483648;",
        "(p + 1)->metadata = 0;",
    ] {
        let source = format!("{LAYOUT} void reset(struct sample_record *p) {{ {body} }}");
        assert!(helper(&profile, &source, RULES).is_err(), "{body}");
    }
    for declaration in [
        "volatile int metadata",
        "unsigned int metadata",
        "int metadata : 2",
    ] {
        let source = format!(
            "struct sample_record {{ int ready; {declaration}; }}; void reset(struct sample_record *p) {{ p->metadata = 0; }}"
        );
        assert!(helper(&profile, &source, RULES).is_err(), "{declaration}");
    }
}
#[test]
fn composition_rejects_profile_build_rules_layout_and_declaration_substitution() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    let profile = fixture.profile("sample_record", "ready", false, false);
    let source = format!("{LAYOUT} void reset(struct sample_record *p) {{ p->metadata = 0; }}");
    let helper = helper(&profile, &source, RULES).unwrap();
    let changed = fixture.profile("sample_record", "ready", false, true);
    assert!(
        caller(
            &changed,
            &caller_source(),
            std::slice::from_ref(&helper),
            RULES
        )
        .is_err()
    );
    let unit = C11TranslationUnit {
        source: &caller_source(),
        dialect: CCompilerDialect::Gnu11,
        ..Default::default()
    };
    assert!(
        import_chunk_source_with_profile(
            &unit,
            REVISION,
            &profile,
            &["sample"],
            std::slice::from_ref(&helper),
            RULES
        )
        .is_err()
    );
    assert!(
        caller(
            &profile,
            &caller_source(),
            std::slice::from_ref(&helper),
            &format!("{RULES}\n")
        )
        .is_err()
    );
    assert!(
        caller(
            &profile,
            &caller_source(),
            &[helper.clone(), helper.clone()],
            RULES
        )
        .is_err()
    );
    for source in [
        caller_source().replace("int ready; int other_model;", "int other_model; int ready;"),
        caller_source().replace(
            "void reset(struct sample_record *);",
            "static void reset(struct sample_record *p) { p->metadata = 0; }",
        ),
        caller_source()
            .replace(
                "void reset(struct sample_record *);",
                "void reset(struct sample_record *, int);",
            )
            .replace("reset(p);", "reset(p, 1);"),
    ] {
        assert!(caller(&profile, &source, std::slice::from_ref(&helper), RULES).is_err());
    }
}
#[test]
fn legacy_and_generic_production_shape_serialize_identically_but_legacy_cannot_write_a_new_model() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    let profile = fixture.profile("cio_chunk", "lock", false, false);
    let source = "struct cio_chunk { int lock; int error_n; }; void reset(struct cio_chunk *p) { p->error_n = 0; }";
    let unit = C11TranslationUnit {
        source,
        ..Default::default()
    };
    let legacy = import_chunk_frame_helper(&unit, REVISION, REVISION, "reset", RULES).unwrap();
    let generic = import_chunk_frame_helper_with_profile(
        &unit,
        REVISION,
        "reset",
        &profile,
        "sample",
        "cio_chunk",
        RULES,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&legacy).unwrap(),
        serde_json::to_value(&generic).unwrap()
    );
    assert_eq!(
        serde_json::to_value(legacy.input_identity()).unwrap(),
        serde_json::to_value(generic.input_identity()).unwrap()
    );
    let changed = fixture.profile("cio_chunk", "error_n", false, false);
    let source = "struct cio_chunk { int lock; int error_n; }; void reset(struct cio_chunk *); int run(struct cio_chunk *p) { reset(p); return p->error_n; }";
    assert!(caller(&changed, source, &[legacy], RULES).is_err());
}

/// The compiler description is now shared with the configured audit's
/// compile-task identity. Its serialized shape is embedded in every published
/// `import_identity`, so its field set and order are a pinned expectation.
#[test]
fn the_shared_clang_identity_keeps_its_recorded_serialized_shape() {
    let Ok(identity) = qcore_c_gateway::ClangIdentity::capture() else {
        return;
    };
    let quoted = |value: &str| serde_json::to_string(value).unwrap();
    assert_eq!(
        serde_json::to_string(&identity).unwrap(),
        format!(
            "{{\"program\":{},\"version\":{},\"target\":{}}}",
            quoted(identity.program()),
            quoted(identity.version()),
            quoted(identity.target())
        )
    );
    assert!(identity.version().contains("clang version"));
    assert!(!identity.target().is_empty());
}
