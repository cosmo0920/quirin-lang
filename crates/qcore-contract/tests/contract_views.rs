//! Card CONTRACT-VIEW-PROJECTION-CHECK: every checked view relation over the
//! real corpus holds, and each check refuses a mutated contract (negative
//! controls). The corpus guard itself lives beside the corpus test
//! (`compiler/tests/contract_view_census.rs`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use qcore_contract::chunk_contract::{
    evaluate_boolean_frame_contract, evaluate_chunk_frame_contract,
};
use qcore_contract::contract_views::{
    PremiseClass, REGISTERED_RELATIONS, RegisteredRelation, RelationCheck, check_registry, entails,
    read_corpus,
};
use qcore_schemas::chunk_summary::ChunkFrameSummaryRecord;

fn modules() -> BTreeMap<String, String> {
    let index = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../contract/corpus.json");
    read_corpus(&index).expect("the corpus reads")
}

/// Replace exactly the first occurrence of `from` in `module`, asserting it
/// was there.
fn mutate(module: &str, from: &str, to: &str) -> BTreeMap<String, String> {
    let mut modules = modules();
    let text = modules.get_mut(module).expect("module exists");
    mutate_text(module, text, from, to);
    modules
}

fn mutate_text(module: &str, text: &mut String, from: &str, to: &str) {
    // Mutation snippets use LF, while a Windows checkout may contain CRLF.
    // Adapt the snippets, preserving the module's bytes outside the mutation.
    let (from, to) = if text.contains(from) {
        (from.to_owned(), to.to_owned())
    } else {
        (from.replace('\n', "\r\n"), to.replace('\n', "\r\n"))
    };
    assert!(text.contains(&from), "{module} does not contain {from:?}");
    *text = text.replacen(&from, &to, 1);
}

#[test]
fn multiline_mutations_preserve_lf_and_crlf_and_replace_only_the_first_match() {
    for ending in ["\n", "\r\n"] {
        let mut text =
            "before\nrequires old;\nensures kept;\nrequires old;\nensures kept;\nafter\n"
                .replace('\n', ending);
        mutate_text(
            "fixture",
            &mut text,
            "requires old;\nensures kept;",
            "requires new;\nensures kept;",
        );
        assert_eq!(
            text,
            "before\nrequires new;\nensures kept;\nrequires old;\nensures kept;\nafter\n"
                .replace('\n', ending)
        );
    }
}

fn refused(modules: &BTreeMap<String, String>, expected: &str) {
    let errors = check_registry(modules, REGISTERED_RELATIONS)
        .expect_err("the mutated corpus must be refused");
    assert!(
        errors.iter().any(|error| error.contains(expected)),
        "expected a refusal containing {expected:?}, got {errors:#?}"
    );
}

#[test]
fn every_registered_relation_holds_on_the_corpus() {
    let report = check_registry(&modules(), REGISTERED_RELATIONS).unwrap();
    // NULL-SAFETY-PROPERTY: +5 (`null_deref_beside`); NULL-CANDIDATE-TRIAGE
    // +2 (a member and a result declaration beside resource views);
    // NULL-MEMBER-INVARIANT-SCOPE +1 (a function-scope entry);
    // W45-CALLEE-CASCADE-RESIDUAL round 3 +1 (`cfl_list_size`'s resource-v1
    // declaration beside its result range); CG22-OP-CALLBACKS-AND-SERVED-QUEUES
    // +1 (`rd_kafka_op_destroy`'s resource-v1 declaration beside its check);
    // FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE +2 (`flb_input_new`'s stable
    // descriptor and `flb_input_thread_instance_exit`'s scope entry);
    // CELL-GUARD-RESULT-RANGE-CHECKED +2 (`open`'s C library result set and
    // `mk_event_channel_create`'s derived range beside their resource views).
    assert_eq!(report.census.len(), 25);
    assert_eq!(report.relations.len(), 27);
    let kinds: BTreeMap<&str, usize> =
        report
            .relations
            .iter()
            .fold(BTreeMap::new(), |mut counts, relation| {
                *counts.entry(relation.relation).or_default() += 1;
                counts
            });
    assert_eq!(
        kinds,
        BTreeMap::from([
            ("lifecycle_profiles", 2),
            ("null_deref_beside", 14),
            ("projection", 5),
            ("renamed_equivalent", 1),
            ("source_shape", 5),
        ])
    );
    // No premise is silently dropped from any entailment.
    assert!(
        report
            .relations
            .iter()
            .all(|relation| relation.dropped_premises.is_empty())
    );
}

#[test]
fn the_allocator_view_follows_from_the_native_evaluated_contract_and_mutations_refuse() {
    // `flb_malloc`'s custody link, removed: a live allocation no longer tracks
    // the null test.
    refused(
        &mutate(
            "fluent_bit/memory.qv",
            "ensures normal live_after == (result_null == 0);",
            "ensures normal live_after == 1;",
        ),
        "resource_return_owned(nullable)` is not implied",
    );
    // The backend-result clause, removed.
    refused(
        &mutate(
            "fluent_bit/memory.qv",
            "ensures normal size == 0 or result_is_backend == 1;",
            "ensures normal size == 0 or backend_calls == 1;",
        ),
        "is not implied",
    );
    // `flb_free` leaving an allocation live.
    refused(
        &mutate(
            "fluent_bit/memory.qv",
            "ensures normal live_after == 0;",
            "ensures normal live_after == 1;",
        ),
        "resource_released(ptr, nullable)` is not implied",
    );
    // A new detailed premise the view does not discharge: the view would
    // claim more than the detailed contract supports.
    refused(
        &mutate(
            "fluent_bit/memory.qv",
            "extern fn flb_malloc(size: usize) -> ptr<mut, u8>\ncontract {\n    requires ordinary_allocator_profile == 1;",
            "extern fn flb_malloc(size: usize) -> ptr<mut, u8>\ncontract {\n    requires ordinary_allocator_profile == 1;\n    requires size_is_small == 1;",
        ),
        "the view assumes",
    );
    // A signature disagreement.
    refused(
        &mutate(
            "fluent_bit/memory.qv",
            "flb_calloc(n: usize, size: usize)",
            "flb_calloc(n: i32, size: usize)",
        ),
        "parameter 0 disagrees",
    );
    // A stronger view than the detailed contract supports: a non-null result.
    refused(
        &mutate(
            "fluent_bit/resource_memory.qv",
            "resource_return_owned(nullable)",
            "resource_return_owned(nonnull)",
        ),
        "has no meaning in the allocator-observation bridge",
    );
}

#[test]
fn the_chunk_close_views_are_checked_and_mutations_refuse() {
    refused(
        &mutate(
            "chunkio/memory_close.qv",
            "ensures normal not chunk_was_nonnull or chunk_released;",
            "ensures normal not chunk_was_nonnull or name_released;",
        ),
        "resource_released(chunk, nonnull)` is not implied",
    );
    refused(
        &mutate(
            "chunkio/chunk_destructor.qv",
            "ensures normal chunk_name_and_object_are_released;",
            "ensures normal chunk_name_is_released;",
        ),
        "is not implied",
    );
    // The ABI disagreement this card repaired is refused if it returns.
    refused(
        &mutate(
            "chunkio/chunk_destructor.qv",
            "delete_backing: i32",
            "delete_backing: bool",
        ),
        "parameter 1 disagrees",
    );
    // The language lifecycle carries custody in its type: a borrow does not
    // release.
    refused(
        &mutate(
            "chunkio/chunk_destructor.qv",
            "chunk: own<Chunk> use one,",
            "chunk: borrow<'call, mut, Chunk> use one,",
        ),
        "needs a OwnedConsumed parameter",
    );
}

#[test]
fn the_error_reset_frames_are_renamed_equivalents_and_mutations_refuse() {
    refused(
        &mutate(
            "chunkio/error_state.qv",
            "ensures normal lock_is_preserved;",
            "ensures normal writes_only_declared_fields;",
        ),
        "not token-equal under the registered renaming",
    );
    refused(
        &mutate(
            "chunkio/error_frame.qv",
            "requires exclusive_object_access;",
            "requires object_is_live;",
        ),
        "not token-equal",
    );
}

/// The equivalence has a stated condition: the protected set excludes the
/// declared write footprint. Outside it the two evaluators really differ, so
/// the check's refusal of such a set is not vacuous.
#[test]
fn the_frame_equivalence_condition_is_load_bearing() {
    let modules = modules();
    let summary = ChunkFrameSummaryRecord {
        function: "cio_error_reset".into(),
        source: String::new(),
        layout: Vec::new(),
        writes: vec![("error_n".into(), 0)],
    };
    let chunk = evaluate_chunk_frame_contract(&summary, &modules["chunkio/error_state.qv"]);
    let lock_only = BTreeSet::from(["lock".to_owned()]);
    let generic = evaluate_boolean_frame_contract(
        &summary,
        &modules["chunkio/error_frame.qv"],
        "cio_chunk",
        &lock_only,
    );
    assert!(chunk.is_ok() && generic.is_ok());
    let covering = BTreeSet::from(["error_n".to_owned(), "lock".to_owned()]);
    assert!(
        evaluate_boolean_frame_contract(
            &summary,
            &modules["chunkio/error_frame.qv"],
            "cio_chunk",
            &covering,
        )
        .is_err()
    );
}

#[test]
fn the_source_shape_metadata_agrees_with_the_typed_declarations_and_mutations_refuse() {
    refused(
        &mutate(
            "monkey/wait_source_policy.qv",
            "signature == \"void *(int)\"",
            "signature == \"void *(int, int)\"",
        ),
        "has 2 parameters, the typed declaration 1",
    );
    refused(
        &mutate(
            "monkey/wait_source_policy.qv",
            "signature == \"void (struct mk_event_ctx *)\"",
            "signature == \"void (struct mk_event_loop *)\"",
        ),
        "`struct mk_event_loop *` is EventLoop",
    );
    refused(
        &mutate(
            "monkey/wait_source_policy.qv",
            "signature == \"int (struct mk_event_loop *, int)\"",
            "signature == \"int (struct mk_event_loop *, long)\"",
        ),
        "disagrees with i32",
    );
    refused(
        &mutate(
            "monkey/wrapper_paths.qv",
            "_mk_event_wait_with_flags(\n    loop: borrow<'call, mut, EventLoop> use one, flags: i32\n) -> i32",
            "_mk_event_wait_with_flags(\n    loop: borrow<'call, mut, EventLoop> use one, flags: i64\n) -> i32",
        ),
        "_mk_event_wait_with_flags",
    );
    refused(
        &mutate(
            "monkey/wait_source_policy.qv",
            "ensures normal parameter_types == \"i32\";",
            "ensures normal parameter_types == \"i64\";",
        ),
        "contract__mk_event_loop_create",
    );
    // The callback projection binds the C callback's third parameter.
    refused(
        &mutate(
            "monkey/wait_source_policy.qv",
            "(signature == \"void (evutil_socket_t, short, void *)\")",
            "(signature == \"void (evutil_socket_t, short)\")",
        ),
        "does not fit",
    );
}

#[test]
fn the_guard_refuses_new_stale_and_unjustified_registrations() {
    // A new multi-module declaration: the removed duplicate, restored.
    refused(
        &mutate(
            "monkey/timeout_teardown.qv",
            "extern fn cb_timeout_source() -> unit",
            "extern fn source_identity() -> unit\ncontract {\n    ensures normal source_revision == \"66910c10a4d7eafa810b84229cf2dcf7b0f26f97\";\n    ensures normal library == \"monkey\";\n    ensures normal source_set == \"fluent-bit\";\n    ensures normal profile_digest != \"\";\n};\n\nextern fn cb_timeout_source() -> unit",
        ),
        "unregistered multi-module declaration: `source_identity`",
    );
    // A declaration copied into an unrelated module.
    refused(
        &mutate(
            "fluent_bit/logging.qv",
            "module contract;",
            "module contract;\nextern fn flb_sds_destroy(ptr: ptr<mut, u8>) -> unit\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_released(ptr, nullable);\n    ensures normal resource_return_none();\n};",
        ),
        "unregistered multi-module declaration: `flb_sds_destroy`",
    );
    // A registration whose pair does not exist is stale.
    let mut registry: Vec<RegisteredRelation> = REGISTERED_RELATIONS.to_vec();
    let mut stale = registry[0];
    stale.function = "flb_realloc";
    registry.push(stale);
    let errors = check_registry(&modules(), &registry).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.contains("stale registration: flb_realloc")),
        "{errors:#?}"
    );
    // A coverage-restricting premise must carry a registered finding.
    let mut registry: Vec<RegisteredRelation> = REGISTERED_RELATIONS.to_vec();
    let entry = registry
        .iter_mut()
        .find(|relation| {
            relation
                .assumed_premises
                .iter()
                .any(|(_, class)| *class == PremiseClass::Coverage)
        })
        .expect("a coverage premise is registered");
    entry.finding = None;
    let errors = check_registry(&modules(), &registry).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.contains("needs a registered finding")),
        "{errors:#?}"
    );
    // A textual duplicate cannot be registered as a relation to keep it: it
    // must be removed, or kept as `Identical` with a registered finding.
    let restored = mutate(
        "monkey/timeout_teardown.qv",
        "extern fn cb_timeout_source() -> unit",
        "extern fn source_identity() -> unit\ncontract {\n    ensures normal source_revision == \"66910c10a4d7eafa810b84229cf2dcf7b0f26f97\";\n    ensures normal library == \"monkey\";\n    ensures normal source_set == \"fluent-bit\";\n    ensures normal profile_digest != \"\";\n};\n\nextern fn cb_timeout_source() -> unit",
    );
    let mut registry: Vec<RegisteredRelation> = REGISTERED_RELATIONS.to_vec();
    let mut duplicate = registry[0];
    duplicate.function = "source_identity";
    duplicate.detailed = "monkey/wait_source_policy.qv";
    duplicate.view = "monkey/timeout_teardown.qv";
    duplicate.assumed_premises = &[];
    duplicate.check = RelationCheck::SourceShape {
        projection: None,
        nominals: &[],
    };
    registry.push(duplicate);
    let errors = check_registry(&restored, &registry).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.contains("is a textual duplicate")),
        "{errors:#?}"
    );
    duplicate.check = RelationCheck::Identical;
    duplicate.finding = None;
    *registry.last_mut().unwrap() = duplicate;
    let errors = check_registry(&restored, &registry).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.contains("without a registered finding")),
        "{errors:#?}"
    );
    duplicate.finding = Some("A-CONSUMER-CANNOT-INCLUDE");
    *registry.last_mut().unwrap() = duplicate;
    assert!(check_registry(&restored, &registry).is_ok());
    // A registration whose pinned assumptions differ from the computed ones.
    let mut registry: Vec<RegisteredRelation> = REGISTERED_RELATIONS.to_vec();
    registry[0].assumed_premises = &[];
    assert!(check_registry(&modules(), &registry).is_err());
}

#[test]
fn entailment_refuses_a_domain_it_cannot_enumerate() {
    let premises: Vec<String> = (0..20)
        .map(|index| format!("x{index} == y{index}"))
        .collect();
    let premises: Vec<&str> = premises.iter().map(String::as_str).collect();
    let goal = (0..20)
        .map(|index| format!("x{index} == y{index}"))
        .collect::<Vec<_>>()
        .join(" and ");
    assert!(entails(&premises, &goal).is_err());
}

#[test]
fn null_resource_overlap_accepts_only_replayed_matching_neutral_metadata() {
    let baseline = modules();
    check_registry(&baseline, REGISTERED_RELATIONS).unwrap();
    let name = "rd_kafka_toppar_new0";
    let bounds_module = "librdkafka/null_deref.qv";
    let source = &baseline[bounds_module];
    let start = source.find(&format!("extern fn {name}(")).unwrap();
    let end = start + source[start..].find("\n};").unwrap() + "\n};".len();
    let declaration = &source[start..end];
    let atom = "#loader_enabled(#env,#family) == 1";
    for (from, to) in [
        (atom, "#loader_enabled(#env,#family) == 0"),
        (
            "resource_premise_decomposition(rk_plugin_callback_closure,",
            "resource_premise_decomposition(missing_legacy_premise,",
        ),
        (
            "requires bounds_contract_v1;",
            "requires resource_contract_v1;",
        ),
        (
            "requires bounds_contract_v1;",
            "requires bounds_contract_v1;\n    ensures normal resource_return_none();",
        ),
    ] {
        assert!(declaration.contains(from));
        let changed = declaration.replacen(from, to, 1);
        let mut input = baseline.clone();
        input.insert(
            bounds_module.into(),
            source.replacen(declaration, &changed, 1),
        );
        assert!(
            check_registry(&input, REGISTERED_RELATIONS).is_err(),
            "accepted {to}"
        );
    }
    // A valid bounds graph cannot borrow a missing resource-side attachment.
    let resource_module = "librdkafka/client.qv";
    let source = &baseline[resource_module];
    let start = source.find(&format!("extern fn {name}(")).unwrap();
    let end = start + source[start..].find("\n};").unwrap() + "\n};".len();
    let declaration = &source[start..end];
    let changed = declaration
        .lines()
        .filter(|line| {
            !line
                .trim()
                .starts_with("requires resource_premise_decomposition(rk_plugin_callback_closure,")
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert_ne!(changed, declaration);
    let mut input = baseline.clone();
    input.insert(
        resource_module.into(),
        source.replacen(declaration, &changed, 1),
    );
    assert!(check_registry(&input, REGISTERED_RELATIONS).is_err());
    check_registry(&baseline, REGISTERED_RELATIONS).unwrap();
}

fn symbolic_overlap_pair(name: &str) -> BTreeMap<String, String> {
    let actual = modules();
    ["librdkafka/null_deref.qv", "librdkafka/client.qv"]
        .into_iter()
        .map(|module| {
            let source = &actual[module];
            let start = source.find(&format!("extern fn {name}(")).unwrap();
            let end = start + source[start..].find("\n};").unwrap() + "\n};".len();
            (
                module.into(),
                format!("module contract;\n{}\n", &source[start..end]),
            )
        })
        .collect()
}
fn overlap_relation(name: &str) -> &RegisteredRelation {
    REGISTERED_RELATIONS
        .iter()
        .find(|r| r.function == name && matches!(r.check, RelationCheck::NullDerefBeside))
        .unwrap()
}
#[test]
fn both_actual_symbolic_overlaps_retain_complete_original_relation_evidence() {
    for name in ["rd_kafka_toppar_new0", "rd_kafka_toppar_fetch_backoff"] {
        let pair = symbolic_overlap_pair(name);
        let original = pair
            .iter()
            .map(|(module, source)| {
                let text = source
                    .split_inclusive('\n')
                    .filter(|line| {
                        let line = line.trim_start();
                        !line.starts_with("requires resource_lifecycle_v1(")
                            && !line.starts_with("requires resource_premise_flow_v1(")
                    })
                    .collect::<String>();
                (module.clone(), text)
            })
            .collect();
        let run = |input: &BTreeMap<String, String>| {
            qcore_contract::contract_views::check_relation(overlap_relation(name), input).unwrap()
        };
        assert_eq!(
            serde_json::to_value(run(&pair)).unwrap(),
            serde_json::to_value(run(&original)).unwrap()
        );
    }
}
#[test]
fn symbolic_overlap_compares_complete_maps_and_rejects_invalid_entries_on_either_peer() {
    let name = "rd_kafka_toppar_new0";
    let pair = symbolic_overlap_pair(name);
    for module in ["librdkafka/null_deref.qv", "librdkafka/client.qv"] {
        let source = &pair[module];
        let flow = source
            .lines()
            .find(|l| {
                l.trim_start()
                    .starts_with("requires resource_premise_flow_v1(")
            })
            .unwrap();
        let life = source
            .lines()
            .find(|l| {
                l.trim_start()
                    .starts_with("requires resource_lifecycle_v1(")
            })
            .unwrap();
        let changes = [
            (flow, String::new()),
            (flow, flow.replacen("decompose_", "other_", 1)),
            (flow, flow.replacen("|->", "||->", 1)),
            (flow, format!("{flow}\n{flow}")),
            (
                flow,
                flow.replacen("resource_premise_flow_v1", "resource_premise_flow_v2", 1),
            ),
            (
                flow,
                "    requires resource_premise_flow_v1(extra, missing);".into(),
            ),
            (
                flow,
                format!("{flow}\n    requires resource_premise_flow_v1(extra, flow(extra));"),
            ),
            (flow, flow.replacen("requires ", "ensures normal ", 1)),
            (life, String::new()),
            (life, life.replacen("introduce_", "other_", 1)),
            (life, life.replacen("|^+", "|^-", 1)),
            (life, life.replacen("hypothesis(", "resource(", 1)),
            (life, format!("{life}\n{life}")),
            (
                life,
                life.replacen("resource_lifecycle_v1", "resource_lifecycle_v2", 1),
            ),
            (
                life,
                "    requires resource_lifecycle_v1(extra, |^+ hypothesis(missing));".into(),
            ),
            (life, life.replacen("requires ", "ensures normal ", 1)),
            (
                life,
                format!("{life}\n    requires unknown_metadata_v1(h);"),
            ),
            (
                life,
                format!("{life}\n    ensures normal resource_borrowed(missing);"),
            ),
        ];
        for (from, to) in changes {
            let mut changed = pair.clone();
            changed.insert(module.into(), source.replacen(from, &to, 1));
            assert!(
                qcore_contract::contract_views::check_relation(overlap_relation(name), &changed)
                    .is_err(),
                "accepted {module}: {to}"
            );
        }
    }
    qcore_contract::contract_views::check_relation(overlap_relation(name), &pair).unwrap();
}
#[test]
fn resource_only_symbolic_metadata_cannot_bypass_matching_an_absent_null_map() {
    let name = "rd_kafka_toppar_new0";
    let mut pair = symbolic_overlap_pair(name);
    let source = pair.get_mut("librdkafka/null_deref.qv").unwrap();
    *source = source
        .split_inclusive('\n')
        .filter(|line| {
            let line = line.trim_start();
            !(line.starts_with("quant proposition #plugin_")
                || line.starts_with("requires resource_hypothesis(")
                || line.starts_with("requires resource_premise_decomposition(")
                || line.starts_with("requires resource_lifecycle_v1(")
                || line.starts_with("requires resource_premise_flow_v1("))
        })
        .collect();
    let standalone = BTreeMap::from([("null.qv".into(), source.clone())]);
    qcore_contract::bounds_hypotheses::replay(&standalone).unwrap();
    assert!(!source.contains("resource_hypothesis("));
    let error =
        qcore_contract::contract_views::check_relation(overlap_relation(name), &pair).unwrap_err();
    assert!(
        error.contains("neutral symbolic metadata differs"),
        "{error}"
    );
}

#[test]
fn original_pure_null_pairs_keep_resource_metadata_independent_and_validate_it() {
    for name in [
        "rd_kafka_topic_partition_list_destroy",
        "rd_kafka_op_destroy",
    ] {
        let pair = symbolic_overlap_pair(name);
        let original = pair
            .iter()
            .map(|(module, source)| {
                let text = source
                    .split_inclusive('\n')
                    .filter(|line| {
                        let line = line.trim_start();
                        !line.starts_with("requires resource_lifecycle_v1(")
                            && !line.starts_with("requires resource_premise_flow_v1(")
                    })
                    .collect::<String>();
                (module.clone(), text)
            })
            .collect();
        let run = |input: &BTreeMap<String, String>| {
            qcore_contract::contract_views::check_relation(overlap_relation(name), input).unwrap()
        };
        assert_eq!(
            serde_json::to_value(run(&pair)).unwrap(),
            serde_json::to_value(run(&original)).unwrap()
        );
        // Neither null graph nor metadata introductions can enter the independent case.
        for clause in [
            "requires resource_hypothesis(injected, \"An independently unbound proposition\");",
            "requires resource_lifecycle_v1(injected, |^+ resource(x));",
            "requires resource_premise_flow_v1(injected, missing);",
        ] {
            let mut changed = pair.clone();
            let null = changed.get_mut("librdkafka/null_deref.qv").unwrap();
            *null = null.replacen(
                "requires bounds_contract_v1;",
                &format!("requires bounds_contract_v1;\n    {clause}"),
                1,
            );
            assert!(
                qcore_contract::contract_views::check_relation(overlap_relation(name), &changed)
                    .is_err(),
                "accepted null introduction {name}: {clause}"
            );
        }
        // Resource-only metadata is validated rather than ignored.
        for (from, to) in [
            ("resource_premise_flow_v1", "resource_premise_flow_v2"),
            (
                "|^+ hypothesis(plugin_artifact_symbol_abi)",
                "|^+ hypothesis(missing)",
            ),
            (
                "requires resource_lifecycle_v1",
                "ensures normal resource_lifecycle_v1",
            ),
        ] {
            let mut changed = pair.clone();
            let resource = changed.get_mut("librdkafka/client.qv").unwrap();
            assert!(resource.contains(from));
            *resource = resource.replacen(from, to, 1);
            assert!(
                qcore_contract::contract_views::check_relation(overlap_relation(name), &changed)
                    .is_err(),
                "accepted invalid resource-only metadata {name}: {to}"
            );
        }
    }
}

#[test]
fn guarded_source_metadata_compares_complete_bounds_resource_maps_without_truth_transport() {
    let name = "rd_kafka_toppar_new0";
    let original = symbolic_overlap_pair(name);
    let mut pair = original.clone();
    let metadata =
        "\nrequires resource_guarded_symbol_v1(env, environment, \"Proposed environment\");
requires resource_guarded_symbol_v1(pre, prerequisite, \"Proposed prerequisite\");
requires resource_guarded_symbol_v1(op, operation, \"Proposed admission\");
requires resource_guarded_symbol_v1(ok_case, case, \"Proposed success case\");
requires resource_guarded_symbol_v1(err_case, case, \"Proposed failure case\");
requires resource_guarded_symbol_v1(ok_post, postcondition, \"Proposed success postcondition\");
requires resource_guarded_symbol_v1(err_post, postcondition, \"Proposed failure postcondition\");
requires resource_guarded_decomposition_v1(guarded, \"guarded_decomposition_v1;
(env ∧ pre) |-> op;
op ||-> ok_case / err_case;
ok_case ->| ok_post;
err_case ->| err_post;
\");\n";
    for source in pair.values_mut() {
        *source = source.replace("\n};", &format!("{metadata}\n}};"));
    }
    let checked =
        qcore_contract::contract_views::check_relation(overlap_relation(name), &pair).unwrap();
    let old =
        qcore_contract::contract_views::check_relation(overlap_relation(name), &original).unwrap();
    assert_eq!(
        serde_json::to_value(checked).unwrap(),
        serde_json::to_value(old).unwrap()
    );
    for module in ["librdkafka/null_deref.qv", "librdkafka/client.qv"] {
        for (from, to) in [
            (metadata, ""),
            (
                "Proposed failure postcondition",
                "Changed failure postcondition",
            ),
            ("err_post, postcondition", "err_post, operation"),
            (
                "resource_guarded_decomposition_v1",
                "resource_guarded_decomposition_v2",
            ),
            ("err_case ->| err_post;", "err_case ->| missing;"),
        ] {
            let mut changed = pair.clone();
            let s = changed.get_mut(module).unwrap();
            let edited = s.replacen(from, to, 1);
            assert_ne!(*s, edited);
            *s = edited;
            assert!(
                qcore_contract::contract_views::check_relation(overlap_relation(name), &changed)
                    .is_err()
            );
        }
    }
    qcore_contract::contract_views::check_relation(overlap_relation(name), &pair).unwrap();
}

#[test]
fn typed_entry_assumptions_cannot_be_erased_or_changed_by_a_legacy_bridge() {
    let report = check_registry(&modules(), REGISTERED_RELATIONS).unwrap();
    let projection = report
        .relations
        .iter()
        .find(|r| r.function == "cio_chunk_close" && r.relation == "projection")
        .unwrap();
    assert_eq!(projection.assumed_premises.len(), 3);
    assert!(
        projection
            .assumed_premises
            .iter()
            .all(|p| p.starts_with("quant-condition-entry:"))
    );
    let changed = mutate(
        "chunkio/chunk_destructor.qv",
        "#number(#env,#chunk,#busy_field) == 0",
        "#number(#env,#chunk,#busy_field) == 1",
    );
    assert!(check_registry(&changed, REGISTERED_RELATIONS).is_err());
}
