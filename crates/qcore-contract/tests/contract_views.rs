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
    PremiseClass, REGISTERED_RELATIONS, RegisteredRelation, RelationCheck, check_registry,
    entails, read_corpus,
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
    assert!(text.contains(from), "{module} does not contain {from:?}");
    *text = text.replacen(from, to, 1);
    modules
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
    assert_eq!(report.census.len(), 10);
    assert_eq!(report.relations.len(), 12);
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
            ("lifecycle_profiles", 1),
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
            "fluent_bit/memory.qcore",
            "ensures normal live_after == (result_null == 0);",
            "ensures normal live_after == 1;",
        ),
        "resource_return_owned(nullable)` is not implied",
    );
    // The backend-result clause, removed.
    refused(
        &mutate(
            "fluent_bit/memory.qcore",
            "ensures normal size == 0 or result_is_backend == 1;",
            "ensures normal size == 0 or backend_calls == 1;",
        ),
        "is not implied",
    );
    // `flb_free` leaving an allocation live.
    refused(
        &mutate(
            "fluent_bit/memory.qcore",
            "ensures normal live_after == 0;",
            "ensures normal live_after == 1;",
        ),
        "resource_released(ptr, nullable)` is not implied",
    );
    // A new detailed premise the view does not discharge: the view would
    // claim more than the detailed contract supports.
    refused(
        &mutate(
            "fluent_bit/memory.qcore",
            "extern fn flb_malloc(size: usize) -> ptr<mut, u8>\ncontract {\n    requires ordinary_allocator_profile == 1;",
            "extern fn flb_malloc(size: usize) -> ptr<mut, u8>\ncontract {\n    requires ordinary_allocator_profile == 1;\n    requires size_is_small == 1;",
        ),
        "the view assumes",
    );
    // A signature disagreement.
    refused(
        &mutate(
            "fluent_bit/memory.qcore",
            "flb_calloc(n: usize, size: usize)",
            "flb_calloc(n: i32, size: usize)",
        ),
        "parameter 0 disagrees",
    );
    // A stronger view than the detailed contract supports: a non-null result.
    refused(
        &mutate(
            "fluent_bit/resource_memory.qcore",
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
            "chunkio/memory_close.qcore",
            "ensures normal not chunk_was_nonnull or chunk_released;",
            "ensures normal not chunk_was_nonnull or name_released;",
        ),
        "resource_released(chunk, nonnull)` is not implied",
    );
    refused(
        &mutate(
            "chunkio/chunk_destructor.qcore",
            "ensures normal chunk_name_and_object_are_released;",
            "ensures normal chunk_name_is_released;",
        ),
        "is not implied",
    );
    // The ABI disagreement this card repaired is refused if it returns.
    refused(
        &mutate(
            "chunkio/chunk_destructor.qcore",
            "delete_backing: i32",
            "delete_backing: bool",
        ),
        "parameter 1 disagrees",
    );
    // The language lifecycle carries custody in its type: a borrow does not
    // release.
    refused(
        &mutate(
            "chunkio/chunk_destructor.qcore",
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
            "chunkio/error_state.qcore",
            "ensures normal lock_is_preserved;",
            "ensures normal writes_only_declared_fields;",
        ),
        "not token-equal under the registered renaming",
    );
    refused(
        &mutate(
            "chunkio/error_frame.qcore",
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
    let chunk = evaluate_chunk_frame_contract(&summary, &modules["chunkio/error_state.qcore"]);
    let lock_only = BTreeSet::from(["lock".to_owned()]);
    let generic = evaluate_boolean_frame_contract(
        &summary,
        &modules["chunkio/error_frame.qcore"],
        "cio_chunk",
        &lock_only,
    );
    assert!(chunk.is_ok() && generic.is_ok());
    let covering = BTreeSet::from(["error_n".to_owned(), "lock".to_owned()]);
    assert!(
        evaluate_boolean_frame_contract(
            &summary,
            &modules["chunkio/error_frame.qcore"],
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
            "monkey/wait_source_policy.qcore",
            "signature == \"void *(int)\"",
            "signature == \"void *(int, int)\"",
        ),
        "has 2 parameters, the typed declaration 1",
    );
    refused(
        &mutate(
            "monkey/wait_source_policy.qcore",
            "signature == \"void (struct mk_event_ctx *)\"",
            "signature == \"void (struct mk_event_loop *)\"",
        ),
        "`struct mk_event_loop *` is EventLoop",
    );
    refused(
        &mutate(
            "monkey/wait_source_policy.qcore",
            "signature == \"int (struct mk_event_loop *, int)\"",
            "signature == \"int (struct mk_event_loop *, long)\"",
        ),
        "disagrees with i32",
    );
    refused(
        &mutate(
            "monkey/wrapper_paths.qcore",
            "_mk_event_wait_with_flags(\n    loop: borrow<'call, mut, EventLoop> use one, flags: i32\n) -> i32",
            "_mk_event_wait_with_flags(\n    loop: borrow<'call, mut, EventLoop> use one, flags: i64\n) -> i32",
        ),
        "_mk_event_wait_with_flags",
    );
    refused(
        &mutate(
            "monkey/wait_source_policy.qcore",
            "ensures normal parameter_types == \"i32\";",
            "ensures normal parameter_types == \"i64\";",
        ),
        "contract__mk_event_loop_create",
    );
    // The callback projection binds the C callback's third parameter.
    refused(
        &mutate(
            "monkey/wait_source_policy.qcore",
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
            "monkey/timeout_teardown.qcore",
            "extern fn cb_timeout_source() -> unit",
            "extern fn source_identity() -> unit\ncontract {\n    ensures normal source_revision == \"66910c10a4d7eafa810b84229cf2dcf7b0f26f97\";\n    ensures normal library == \"monkey\";\n    ensures normal source_set == \"fluent-bit\";\n    ensures normal profile_digest != \"\";\n};\n\nextern fn cb_timeout_source() -> unit",
        ),
        "unregistered multi-module declaration: `source_identity`",
    );
    // A declaration copied into an unrelated module.
    refused(
        &mutate(
            "fluent_bit/logging.qcore",
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
        "monkey/timeout_teardown.qcore",
        "extern fn cb_timeout_source() -> unit",
        "extern fn source_identity() -> unit\ncontract {\n    ensures normal source_revision == \"66910c10a4d7eafa810b84229cf2dcf7b0f26f97\";\n    ensures normal library == \"monkey\";\n    ensures normal source_set == \"fluent-bit\";\n    ensures normal profile_digest != \"\";\n};\n\nextern fn cb_timeout_source() -> unit",
    );
    let mut registry: Vec<RegisteredRelation> = REGISTERED_RELATIONS.to_vec();
    let mut duplicate = registry[0];
    duplicate.function = "source_identity";
    duplicate.detailed = "monkey/wait_source_policy.qcore";
    duplicate.view = "monkey/timeout_teardown.qcore";
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
    let premises: Vec<String> = (0..20).map(|index| format!("x{index} == y{index}")).collect();
    let premises: Vec<&str> = premises.iter().map(String::as_str).collect();
    let goal = (0..20)
        .map(|index| format!("x{index} == y{index}"))
        .collect::<Vec<_>>()
        .join(" and ");
    assert!(entails(&premises, &goal).is_err());
}
