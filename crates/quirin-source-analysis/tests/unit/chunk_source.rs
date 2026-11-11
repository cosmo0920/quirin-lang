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

use super::*;
use crate::chunk_lifecycle::{
    ChunkBackend, ChunkProgram, ChunkResidency, check_chunk_program, execute_chunk_prefix,
};

fn import(body: &str) -> Vec<ChunkSourceResult> {
    import_chunk_source_with_helpers(
        &C11TranslationUnit {
            source: &format!(
                "struct flb_input_chunk {{ int busy; int other; }};\n\
                 struct cio_chunk {{ int lock; }};\n{body}"
            ),
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &[],
    )
    .unwrap()
}

fn release_summary() -> ChunkSourceSummary {
    let results = import(
        "int release(struct flb_input_chunk *ic) {\n\
         if (ic->busy == 0) { return -1; }\n\
         ic->busy = 0; return 0; }",
    );
    let [ChunkSourceResult::Summary(summary)] = results.as_slice() else {
        panic!("expected source summary: {results:?}")
    };
    summary.as_ref().clone()
}

fn prefix() -> Vec<ChunkOperation> {
    vec![
        ChunkOperation::CreateContext { context: 3 },
        ChunkOperation::CreateStream {
            context: 3,
            stream: 4,
        },
        ChunkOperation::OpenChunk {
            context: 3,
            stream: 4,
            chunk: 7,
            backend: ChunkBackend::File,
            residency: ChunkResidency::Up,
        },
    ]
}

#[test]
fn derives_both_returns_and_instantiates_the_actual_identity() {
    if !crate::test_support::clang_available() {
        return;
    }
    let summary = release_summary();
    assert_eq!(summary.field(), ChunkSourceField::Busy);
    assert_eq!(summary.paths()[0].return_value, -1);
    assert_eq!(summary.paths()[0].branches, [true]);
    assert!(summary.paths()[0].writes.is_empty());
    assert_eq!(summary.paths()[1].return_value, 0);
    assert_eq!(summary.paths()[1].branches, [false]);
    assert_eq!(summary.paths()[1].writes, [false]);

    let mut operations = prefix();
    let state = execute_chunk_prefix(&operations).unwrap().final_state;
    assert_eq!(summary.instantiate(&state, 7).unwrap(), (-1, vec![]));
    assert!(summary.instantiate(&state, 8).is_err());
    operations.push(ChunkOperation::SetBusy {
        chunk: 7,
        busy: true,
    });
    let state = execute_chunk_prefix(&operations).unwrap().final_state;
    let (result, effects) = summary.instantiate(&state, 7).unwrap();
    assert_eq!(result, 0);
    assert_eq!(
        effects,
        [ChunkOperation::SetBusy {
            chunk: 7,
            busy: false
        }]
    );
    operations.extend(effects);
    operations.push(ChunkOperation::DestroyContext { context: 3 });
    let certificate = check_chunk_program(&ChunkProgram { operations }).unwrap();
    assert!(!certificate.final_state.chunks[&7].live);
    assert!(summary.instantiate(&certificate.final_state, 7).is_err());
}

#[test]
fn releasing_wrapper_busy_does_not_discharge_chunkio_lock() {
    if !crate::test_support::clang_available() {
        return;
    }
    let summary = release_summary();
    let mut operations = prefix();
    operations.push(ChunkOperation::Lock { chunk: 7 });
    operations.push(ChunkOperation::SetBusy {
        chunk: 7,
        busy: true,
    });
    let state = execute_chunk_prefix(&operations).unwrap().final_state;
    operations.extend(summary.instantiate(&state, 7).unwrap().1);
    let state = execute_chunk_prefix(&operations).unwrap().final_state;
    assert!(state.chunks[&7].locked);
    assert!(!state.chunks[&7].busy);
    operations.push(ChunkOperation::DestroyContext { context: 3 });
    assert!(check_chunk_program(&ChunkProgram { operations }).is_err());
}

#[test]
fn typed_ast_preserves_else_and_ordered_writes() {
    if !crate::test_support::clang_available() {
        return;
    }
    let results = import(
        "int toggle(struct cio_chunk *ch) {\n\
         if (ch->lock != 0) { ch->lock = 0; } else { ch->lock = !0; }\n\
         if (!ch->lock) { ch->lock = 1; ch->lock = 0; }\n\
         return ch->lock; }",
    );
    let [ChunkSourceResult::Summary(summary)] = results.as_slice() else {
        panic!("{results:?}")
    };
    assert_eq!(summary.paths()[0].writes, [true]);
    assert_eq!(summary.paths()[0].return_value, 1);
    assert_eq!(summary.paths()[1].writes, [false, true, false]);
    assert_eq!(summary.paths()[1].return_value, 0);
}

#[test]
fn rejects_unmodeled_effects_even_on_unexecuted_paths() {
    if !crate::test_support::clang_available() {
        return;
    }
    for body in [
        "extern void opaque(void); int f(struct flb_input_chunk *p) { opaque(); return p->busy; }",
        "extern void opaque(void); int f(struct flb_input_chunk *p) { if (0) opaque(); return p->busy; }",
        "extern void opaque(void); int f(struct flb_input_chunk *p) { return p->busy; opaque(); }",
        "int f(struct flb_input_chunk *p) { p->other = 0; return p->busy; }",
        "int f(struct flb_input_chunk *p) { struct flb_input_chunk *q = p; q->busy = 0; return 0; }",
        "int f(struct flb_input_chunk *p) { while(p->busy) p->busy = 0; return 0; }",
        "int f(struct flb_input_chunk *p) { p->busy = 2; return 0; }",
        "int f(struct flb_input_chunk *p) { if(p->busy) return 0; }",
        "int f(struct flb_input_chunk *p) { return (p->busy++, 0); }",
    ] {
        assert!(
            matches!(import(body).as_slice(), [ChunkSourceResult::Unknown { .. }]),
            "unexpected acceptance: {body}"
        );
    }
}

#[test]
fn source_configuration_and_parameter_qualifiers_are_not_erased() {
    if !crate::test_support::clang_available() {
        return;
    }
    let source = "struct flb_input_chunk { int busy; };\n\
        #ifdef INCLUDE_F\n\
        int f(struct flb_input_chunk *p) { p->busy = 0; return 0; }\n\
        #endif\n";
    let mut unit = C11TranslationUnit {
        source,
        ..Default::default()
    };
    assert!(matches!(
        import_chunk_source_with_helpers(&unit, PINNED_FLUENT_BIT_REVISION, &[])
            .unwrap()
            .as_slice(),
        [ChunkSourceResult::Unknown { .. }]
    ));
    unit.definitions.push("INCLUDE_F".into());
    assert!(matches!(
        import_chunk_source_with_helpers(&unit, PINNED_FLUENT_BIT_REVISION, &[])
            .unwrap()
            .as_slice(),
        [ChunkSourceResult::Summary(_)]
    ));
    assert!(import_chunk_source_with_helpers(&unit, "other revision", &[]).is_err());
    assert!(matches!(
        import("int f(volatile struct flb_input_chunk *p) { return p->busy; }").as_slice(),
        [ChunkSourceResult::Unknown { .. }]
    ));
}

#[test]
fn exact_ast_definition_selection_does_not_confuse_name_prefixes() {
    if !crate::test_support::clang_available() {
        return;
    }
    let results = import(
        "int f(struct flb_input_chunk *p) { return p->busy; }\n\
         int f_extra(struct flb_input_chunk *p) { return !p->busy; }",
    );
    let [
        ChunkSourceResult::Summary(first),
        ChunkSourceResult::Summary(second),
    ] = results.as_slice()
    else {
        panic!("{results:?}")
    };
    assert_eq!(first.function(), "f");
    assert_eq!(first.paths()[1].return_value, 1);
    assert_eq!(second.function(), "f_extra");
    assert_eq!(second.paths()[1].return_value, 0);
}

#[test]
fn bitfields_and_volatile_members_cannot_receive_boolean_field_summaries() {
    if !crate::test_support::clang_available() {
        return;
    }
    for declaration in ["int busy : 1", "volatile int busy", "_Atomic int busy"] {
        let source = format!(
            "struct flb_input_chunk {{ {declaration}; }};\n\
             int f(struct flb_input_chunk *p) {{ p->busy = 1; return p->busy; }}"
        );
        let results = import_chunk_source_with_helpers(
            &C11TranslationUnit {
                source: &source,
                ..Default::default()
            },
            PINNED_FLUENT_BIT_REVISION,
            &[],
        )
        .unwrap();
        assert!(
            matches!(results.as_slice(), [ChunkSourceResult::Unknown { .. }]),
            "{declaration}"
        );
    }
}

const HELPER_LAYOUT: &str = "struct cio_chunk { int lock; int error_n; };";

fn frame_helper(body: &str) -> Result<ChunkFrameSummary, String> {
    import_chunk_frame_helper(
        &C11TranslationUnit {
            source: &format!("{HELPER_LAYOUT} {body}"),
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        "reset",
    )
}

fn unlock_with(helper: &ChunkFrameSummary) -> ChunkSourceSummary {
    let source = format!(
        "{HELPER_LAYOUT} void reset(struct cio_chunk *); int cio_chunk_unlock(struct cio_chunk *ch) {{ reset(ch); if (ch->lock == 0) return -1; ch->lock = 0; return 0; }}"
    );
    let unit = C11TranslationUnit {
        source: &source,
        ..Default::default()
    };
    assert!(matches!(
        import_chunk_source_with_helpers(&unit, PINNED_FLUENT_BIT_REVISION, &[])
            .unwrap()
            .as_slice(),
        [ChunkSourceResult::Unknown { .. }]
    ));
    let result = import_chunk_source_with_helpers(
        &unit,
        PINNED_FLUENT_BIT_REVISION,
        std::slice::from_ref(helper),
    )
    .unwrap();
    let [ChunkSourceResult::Summary(summary)] = result.as_slice() else {
        panic!("{result:?}");
    };
    summary.as_ref().clone()
}

#[test]
fn validated_frame_helper_connects_unlock_to_identity_and_native_contract() {
    if !crate::test_support::clang_available() {
        return;
    }
    let helper = frame_helper("void reset(struct cio_chunk *ch) { ch->error_n = 0; }").unwrap();
    let summary = unlock_with(&helper);
    assert_eq!(summary.paths()[0].return_value, -1);
    assert_eq!(summary.paths()[1].return_value, 0);
    for path in summary.paths() {
        assert_eq!(path.frame_writes, [("error_n".into(), 0)]);
    }
    let contract = include_str!(
        "../../../../providers/qcore-provider-chunkio/contracts/chunkio/lock_state.qv"
    );
    let certificate =
        crate::chunk_contract::check_chunk_source_contract(&summary, contract).unwrap();
    assert_eq!(certificate.dependency_sources()["reset"], helper.source);
    let mut operations = prefix();
    operations.push(ChunkOperation::Lock { chunk: 7 });
    let state = execute_chunk_prefix(&operations).unwrap().final_state;
    assert_eq!(
        summary.instantiate(&state, 7).unwrap(),
        (0, vec![ChunkOperation::Unlock { chunk: 7 }])
    );
    operations.extend(summary.instantiate(&state, 7).unwrap().1);
    operations.push(ChunkOperation::DestroyContext { context: 3 });
    assert!(check_chunk_program(&ChunkProgram { operations }).is_ok());
    for changed in [
        contract.replace("#field_error_n == 0", "#field_error_n == 1"),
        contract.replace(
            "quant condition normal #exit_7(#field_error_n: integer) = (#field_error_n == 0);",
            "",
        ),
        contract.replace("mut, Chunk", "const, Chunk"),
    ] {
        assert_ne!(
            changed, contract,
            "mutation did not change the typed contract"
        );
        assert!(crate::chunk_contract::check_chunk_source_contract(&summary, &changed).is_err());
    }
    let changed = frame_helper("void reset(struct cio_chunk *ch) { ch->error_n = 1; }").unwrap();
    assert!(
        crate::chunk_contract::check_chunk_source_contract(&unlock_with(&changed), contract)
            .is_err()
    );
}

#[test]
fn helper_import_rejects_hidden_effects_aliasing_and_nonconstant_writes() {
    if !crate::test_support::clang_available() {
        return;
    }
    for body in [
        "void reset(struct cio_chunk *ch) { ch->lock = 0; }",
        "void reset(struct cio_chunk *ch) { ch->error_n = ch->error_n; }",
        "void opaque(void); void reset(struct cio_chunk *ch) { ch->error_n = 0; if (0) opaque(); }",
        "void reset(struct cio_chunk *ch) { struct cio_chunk *alias = ch; alias->error_n = 0; }",
        "void reset(struct cio_chunk *ch) { ch->error_n = 0; return; ch->lock = 0; }",
        "struct cio_chunk *global; void reset(struct cio_chunk *ch) { global->error_n = 0; }",
        "void reset(struct cio_chunk *ch) { }",
    ] {
        assert!(frame_helper(body).is_err(), "accepted {body}");
    }
    let source = "struct cio_chunk { int lock; volatile int error_n; }; void reset(struct cio_chunk *ch) { ch->error_n = 0; }";
    assert!(
        import_chunk_frame_helper(
            &C11TranslationUnit {
                source,
                ..Default::default()
            },
            PINNED_FLUENT_BIT_REVISION,
            "reset"
        )
        .is_err()
    );
}

#[test]
fn helper_dependencies_reject_layout_duplicates_and_wrong_pointer_binding() {
    if !crate::test_support::clang_available() {
        return;
    }
    let helper = frame_helper("void reset(struct cio_chunk *ch) { ch->error_n = 0; }").unwrap();
    let source = format!(
        "{HELPER_LAYOUT} void reset(struct cio_chunk *); int f(struct cio_chunk *ch) {{ reset(ch); return ch->lock; }}"
    );
    let unit = C11TranslationUnit {
        source: &source,
        ..Default::default()
    };
    assert!(
        import_chunk_source_with_helpers(
            &unit,
            PINNED_FLUENT_BIT_REVISION,
            &[helper.clone(), helper.clone()]
        )
        .is_err()
    );
    let changed = source.replace("int error_n;", "long error_n;");
    assert!(
        import_chunk_source_with_helpers(
            &C11TranslationUnit {
                source: &changed,
                ..Default::default()
            },
            PINNED_FLUENT_BIT_REVISION,
            std::slice::from_ref(&helper)
        )
        .is_err()
    );
    for declaration in [
        "static void reset(struct cio_chunk *ch) { ch->lock = 1; }",
        "void reset(struct cio_chunk *ch) { ch->lock = 1; }",
    ] {
        let changed = source.replace("void reset(struct cio_chunk *);", declaration);
        assert!(
            import_chunk_source_with_helpers(
                &C11TranslationUnit {
                    source: &changed,
                    ..Default::default()
                },
                PINNED_FLUENT_BIT_REVISION,
                std::slice::from_ref(&helper)
            )
            .is_err()
        );
    }
    let changed = source.replace("reset(ch);", "reset((struct cio_chunk *)0);");
    let results = import_chunk_source_with_helpers(
        &C11TranslationUnit {
            source: &changed,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &[helper],
    )
    .unwrap();
    assert!(matches!(
        results.as_slice(),
        [ChunkSourceResult::Unknown { .. }]
    ));
}

#[test]
fn native_helper_contract_checks_values_footprint_and_signature() {
    if !crate::test_support::clang_available() {
        return;
    }
    let contract = include_str!(
        "../../../../providers/qcore-provider-chunkio/contracts/chunkio/error_state.qv"
    )
    .replace("cio_error_reset", "reset");
    let helper = frame_helper("void reset(struct cio_chunk *ch) { ch->error_n = 0; }").unwrap();
    assert_eq!(
        crate::chunk_contract::check_chunk_frame_contract(&helper, &contract).unwrap(),
        3
    );
    for (old, replacement) in [
        ("field_error_n == 0", "field_error_n == 1"),
        ("field_error_n == 0", "unknown_predicate"),
        ("field_error_n == 0", "field_missing == 0"),
        ("requires chunk_is_live;", ""),
        (
            "requires exclusive_chunk_access;",
            "requires invented_assumption;",
        ),
        ("ensures normal writes_only_declared_fields;", ""),
        ("mut, Chunk", "const, Chunk"),
        ("-> unit", "-> i32"),
        (" use one", ""),
        ("ensures normal", "ensures any"),
        ("reset(", "different("),
    ] {
        assert!(
            crate::chunk_contract::check_chunk_frame_contract(
                &helper,
                &contract.replace(old, replacement)
            )
            .is_err(),
            "accepted {replacement}"
        );
    }
    let helper = frame_helper("void reset(struct cio_chunk *ch) { ch->error_n = 1; }").unwrap();
    assert!(crate::chunk_contract::check_chunk_frame_contract(&helper, &contract).is_err());
    let source = "struct cio_chunk { int lock; int error_n; int extra; }; void reset(struct cio_chunk *ch) { ch->error_n = 0; ch->extra = 0; }";
    let helper = import_chunk_frame_helper(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        "reset",
    )
    .unwrap();
    assert!(crate::chunk_contract::check_chunk_frame_contract(&helper, &contract).is_err());
}

fn field_profile(models: serde_json::Value) -> Result<LoadedProfile, String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let original =
        crate::source_profiles::load_profile(crate::source_profiles::FLUENT_BIT_SOURCE_SET)
            .unwrap();
    let mut value = serde_json::to_value(original.manifest()).unwrap();
    let mut library = value["libraries"]["chunkio"].clone();
    library["provider"] = serde_json::Value::Null;
    library["audited_files"] = serde_json::json!([]);
    library["source_witnesses"] = serde_json::json!([]);
    library["boolean_field_models"] = models;
    value["source_set"] = "field-fixture".into();
    value["libraries"] = serde_json::json!({ "custom": library });
    value["provider_manifests"] = serde_json::json!([]);
    value["resource_contract_manifest"] = serde_json::Value::Null;
    value["pins"]["fluent-bit"]["git_commit"] = "b".repeat(40).into();
    let root = std::env::temp_dir().join(format!(
        "qcore-field-profile-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(root.join("field-fixture")).unwrap();
    std::fs::write(
        root.join("field-fixture/profile.json"),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    let loaded = qcore_contract::profile_loader::load_source_set_profile(&root, "field-fixture")
        .map_err(|error| error.to_string());
    std::fs::remove_dir_all(root).unwrap();
    loaded
}

fn field_model(record: &str, field: &str, set: &str, clear: &str) -> serde_json::Value {
    serde_json::json!({"record":record,"field":field,"set_operation":set,"clear_operation":clear})
}

#[test]
fn profile_models_route_unrelated_records_and_retain_their_assumptions() {
    if !crate::test_support::clang_available() {
        return;
    }
    let source =
        "struct widget { int held; }; int clear(struct widget *p) { p->held = 0; return 0; }";
    let unit = C11TranslationUnit {
        source,
        ..Default::default()
    };
    for (set, clear, expected) in [
        ("lock", "unlock", ChunkSourceField::Locked),
        ("set_busy", "set_busy", ChunkSourceField::Busy),
    ] {
        let profile = field_profile(serde_json::json!([field_model(
            "widget", "held", set, clear
        )]))
        .unwrap();
        let results =
            import_chunk_source_with_profile(&unit, &"b".repeat(40), &profile, &["custom"], &[])
                .unwrap();
        let [ChunkSourceResult::Summary(summary)] = results.as_slice() else {
            panic!("{results:?}");
        };
        assert_eq!(summary.field(), expected);
        assert_eq!(summary.paths()[0].writes, [false]);
        assert_eq!(summary.paths()[1].writes, [false]);
        assert_eq!(summary.profile_assumptions(), &profile.assumptions());
        assert!(!summary.profile_assumptions().is_empty());
        let serialized = serde_json::to_value(summary).unwrap();
        assert!(serialized["profile_assumptions"].is_object());
        assert!(
            import_chunk_source_with_profile(&unit, &"a".repeat(40), &profile, &["custom"], &[])
                .is_err()
        );
        assert!(
            import_chunk_source_with_profile(&unit, &"b".repeat(40), &profile, &["missing"], &[])
                .is_err()
        );
    }
}

#[test]
fn absent_ambiguous_or_unsupported_field_models_never_gain_fallback_routes() {
    if !crate::test_support::clang_available() {
        return;
    }
    let unit = C11TranslationUnit {
        source: "struct widget { int held; }; int clear(struct widget *p) { p->held = 0; return 0; }",
        ..Default::default()
    };
    for models in [
        serde_json::json!([]),
        serde_json::json!([field_model("widget", "other", "lock", "unlock")]),
        serde_json::json!([field_model("other", "held", "lock", "unlock")]),
    ] {
        let profile = field_profile(models).unwrap();
        let results =
            import_chunk_source_with_profile(&unit, &"b".repeat(40), &profile, &["custom"], &[])
                .unwrap();
        assert!(
            matches!(results.as_slice(), [ChunkSourceResult::Unknown { .. }]),
            "{results:?}"
        );
    }
    for models in [
        serde_json::json!([field_model("widget", "held", "lock", "lock")]),
        serde_json::json!([field_model("widget", "held", "destroy_context", "unlock")]),
        serde_json::json!([
            field_model("widget", "held", "lock", "unlock"),
            field_model("widget", "other", "set_busy", "set_busy")
        ]),
    ] {
        let profile = field_profile(models).unwrap();
        assert!(
            import_chunk_source_with_profile(&unit, &"b".repeat(40), &profile, &["custom"], &[])
                .is_err()
        );
    }
    let model = field_model("widget", "held", "lock", "unlock");
    assert!(field_profile(serde_json::json!([model, model])).is_err());
}

#[test]
fn data_selected_field_does_not_weaken_typed_source_validation() {
    if !crate::test_support::clang_available() {
        return;
    }
    let profile = field_profile(serde_json::json!([field_model(
        "widget", "held", "lock", "unlock"
    )]))
    .unwrap();
    for source in [
        "struct widget { int held:1; }; int clear(struct widget *p) { p->held = 0; return 0; }",
        "struct widget { volatile int held; }; int clear(struct widget *p) { p->held = 0; return 0; }",
        "struct widget { int held; }; int clear(struct widget *p) { p->held = 2; return 0; }",
        "struct widget { int held; }; int clear(struct widget *p) { struct widget *q=p; q->held = 0; return 0; }",
    ] {
        let unit = C11TranslationUnit {
            source,
            ..Default::default()
        };
        let results =
            import_chunk_source_with_profile(&unit, &"b".repeat(40), &profile, &["custom"], &[])
                .unwrap();
        assert!(
            matches!(results.as_slice(), [ChunkSourceResult::Unknown { .. }]),
            "{results:?}"
        );
    }
}

#[test]
fn checked_source_identity_binds_inputs_configuration_profile_and_policy() {
    if !crate::test_support::clang_available() {
        return;
    }
    use crate::legacy_c::{CCompilerDialect, LegacyFunctionScope};
    use qcore_c_gateway::boolean_source as gateway;
    let profile = field_profile(serde_json::json!([field_model(
        "widget", "held", "lock", "unlock"
    )]))
    .unwrap();
    let source =
        "struct widget { int held; }; int clear(struct widget *p) { p->held = 0; return 0; }";
    let unit = C11TranslationUnit {
        source,
        ..Default::default()
    };
    let import = |unit: &C11TranslationUnit<'_>, profile: &LoadedProfile, rules: &str| {
        let mut results = gateway::import_chunk_source_with_profile(
            unit,
            &"b".repeat(40),
            profile,
            &["custom"],
            &[],
            rules,
        )
        .unwrap();
        let gateway::ChunkSourceResult::Summary(summary) = results.remove(0) else {
            panic!("expected checked source summary");
        };
        summary
    };
    let original = import(&unit, &profile, crate::source_contract::RULES);
    let identity = original.input_identity();
    assert_eq!(
        identity.raw_source_digest(),
        qcore_schemas::sha256_hex(source.as_bytes())
    );
    assert_eq!(
        identity.preprocessed_source_digest(),
        qcore_schemas::sha256_hex(
            crate::legacy_c::validate_unrestricted_c11(&unit)
                .unwrap()
                .as_bytes()
        )
    );
    assert!(identity.compiler_version().contains("clang version"));
    assert_eq!(identity.dialect(), "c11");
    assert!(!identity.compiler_target().is_empty());
    assert!(identity.profile_digest().is_some());
    assert_eq!(
        serde_json::to_value(&original).unwrap(),
        serde_json::to_value(&**original).unwrap(),
        "checked provenance must not alter the compatibility record serialization"
    );
    let commented = format!("{source}\n// another source snapshot\n");
    let changed = import(
        &C11TranslationUnit {
            source: &commented,
            ..unit.clone()
        },
        &profile,
        crate::source_contract::RULES,
    );
    assert_ne!(
        identity.raw_source_digest(),
        changed.input_identity().raw_source_digest()
    );
    assert_eq!(
        identity.preprocessed_source_digest(),
        changed.input_identity().preprocessed_source_digest()
    );
    for changed_unit in [
        C11TranslationUnit {
            definitions: vec!["UNUSED=1".into()],
            ..unit.clone()
        },
        C11TranslationUnit {
            include_paths: vec![".".into()],
            ..unit.clone()
        },
        C11TranslationUnit {
            dialect: CCompilerDialect::Gnu11,
            ..unit.clone()
        },
        C11TranslationUnit {
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            ..unit.clone()
        },
    ] {
        let changed = import(&changed_unit, &profile, crate::source_contract::RULES);
        assert_eq!(
            serde_json::to_value(&original).unwrap(),
            serde_json::to_value(&changed).unwrap()
        );
        assert_ne!(
            identity.configuration_digest(),
            changed.input_identity().configuration_digest()
        );
    }
    let changed_rules = format!("{}\n// identity change\n", crate::source_contract::RULES);
    let changed = import(&unit, &profile, &changed_rules);
    assert_ne!(
        identity.source_contract_digest(),
        changed.input_identity().source_contract_digest()
    );
    let changed_profile = field_profile(serde_json::json!([field_model(
        "widget", "held", "set_busy", "set_busy"
    )]))
    .unwrap();
    let changed = import(&unit, &changed_profile, crate::source_contract::RULES);
    assert_ne!(
        identity.profile_digest(),
        changed.input_identity().profile_digest()
    );
}

#[test]
fn checked_helper_build_identity_survives_source_import_and_raw_record_parity() {
    if !crate::test_support::clang_available() {
        return;
    }
    use qcore_c_gateway::boolean_source as gateway;
    let helper_source = "struct cio_chunk { int lock; int error; }; void reset(struct cio_chunk *p) { p->error = 0; }";
    let helper_unit = C11TranslationUnit {
        source: helper_source,
        ..Default::default()
    };
    let helper = gateway::import_chunk_frame_helper(
        &helper_unit,
        PINNED_FLUENT_BIT_REVISION,
        PINNED_FLUENT_BIT_REVISION,
        "reset",
        crate::source_contract::RULES,
    )
    .unwrap();
    let changed_helper = gateway::import_chunk_frame_helper(
        &C11TranslationUnit {
            definitions: vec!["UNUSED=1".into()],
            ..helper_unit
        },
        PINNED_FLUENT_BIT_REVISION,
        PINNED_FLUENT_BIT_REVISION,
        "reset",
        crate::source_contract::RULES,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&helper).unwrap(),
        serde_json::to_value(&changed_helper).unwrap()
    );
    assert_ne!(helper.input_identity(), changed_helper.input_identity());
    let unit = C11TranslationUnit {
        source: "struct cio_chunk { int lock; int error; }; void reset(struct cio_chunk *); int clear(struct cio_chunk *p) { reset(p); p->lock = 0; return 0; }",
        ..Default::default()
    };
    let checked = |helper: &ChunkFrameSummary| {
        let mut results = import_chunk_source_with_helpers(
            &unit,
            PINNED_FLUENT_BIT_REVISION,
            std::slice::from_ref(helper),
        )
        .unwrap();
        let ChunkSourceResult::Summary(summary) = results.remove(0) else {
            panic!("expected checked source")
        };
        summary
    };
    let first = checked(&helper);
    let second = checked(&changed_helper);
    assert_eq!(
        serde_json::to_value(&first).unwrap(),
        serde_json::to_value(&second).unwrap()
    );
    assert_eq!(
        first.source_checked().input_identity().helpers()["reset"],
        *helper.input_identity()
    );
    assert_ne!(
        first.source_checked().input_identity(),
        second.source_checked().input_identity()
    );
}
