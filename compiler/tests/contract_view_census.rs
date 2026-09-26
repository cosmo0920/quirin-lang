//! Card CONTRACT-VIEW-PROJECTION-CHECK: the corpus guard for multi-module
//! declarations.
//!
//! A function may be declared in more than one contract module only as a
//! **checked relation** registered in
//! `qcore_contract::contract_views::REGISTERED_RELATIONS`: one hand-written
//! view per consuming checker, each pair checked (projection entailment,
//! renamed equivalence, or agreement of everything both state). This test
//! fails on any new multi-module declaration that is not registered, on a
//! registration whose pair no longer exists, and on any registered relation
//! whose check fails. The negative controls for each check are in
//! `crates/qcore-contract/tests/contract_views.rs`.

use std::collections::BTreeMap;

use qcore_contract::contract_views::{
    REGISTERED_RELATIONS, census_pairs, check_registry, read_corpus,
};

#[path = "support/contract_corpus.rs"]
mod contract_corpus;

/// Every function declared in more than one corpus module, with its modules.
/// `source_identity` (`monkey/timeout_teardown.qcore` and
/// `monkey/wait_source_policy.qcore`, textually identical) was removed from
/// the timeout policy by this card, so it is absent; restoring it fails here.
const EXPECTED_MULTI_MODULE: &[(&str, &[&str])] = &[
    (
        "_mk_event_loop_create",
        &[
            "monkey/wait_source_policy.qcore",
            "monkey/wrapper_paths.qcore",
        ],
    ),
    (
        "_mk_event_loop_destroy",
        &[
            "monkey/wait_source_policy.qcore",
            "monkey/wrapper_paths.qcore",
        ],
    ),
    (
        "_mk_event_wait_2",
        &[
            "monkey/wait_source_policy.qcore",
            "monkey/wrapper_paths.qcore",
        ],
    ),
    (
        "_mk_event_wait_with_flags",
        &[
            "monkey/wait_source_policy.qcore",
            "monkey/wrapper_paths.qcore",
        ],
    ),
    (
        "cb_wait_2_timeout",
        &[
            "monkey/composition.qcore",
            "monkey/wait_source_policy.qcore",
        ],
    ),
    (
        "cio_chunk_close",
        &[
            "chunkio/chunk_destructor.qcore",
            "chunkio/memory_close.qcore",
            "chunkio/resource_open_close.qcore",
        ],
    ),
    (
        "cio_error_reset",
        &["chunkio/error_frame.qcore", "chunkio/error_state.qcore"],
    ),
    (
        "flb_calloc",
        &[
            "fluent_bit/memory.qcore",
            "fluent_bit/resource_memory.qcore",
        ],
    ),
    (
        "flb_free",
        &[
            "fluent_bit/memory.qcore",
            "fluent_bit/resource_memory.qcore",
        ],
    ),
    (
        "flb_malloc",
        &[
            "fluent_bit/memory.qcore",
            "fluent_bit/resource_memory.qcore",
        ],
    ),
    // Card W45-FB-RETRY-LIFETIME: the resource-v1 view of the unlink.
    (
        "mk_list_del",
        &["chunkio/list_unlink.qcore", "monkey/intrusive_list.qcore"],
    ),
];

fn corpus() -> BTreeMap<String, String> {
    contract_corpus::sources()
        .into_iter()
        .map(|(logical, path)| {
            let text = contract_corpus::text(&logical, &path);
            (logical, text)
        })
        .collect()
}

#[test]
fn every_multi_module_declaration_is_a_registered_checked_relation() {
    let modules = corpus();
    let report = check_registry(&modules, REGISTERED_RELATIONS).unwrap_or_else(|errors| {
        panic!(
            "multi-module declaration guard refused the corpus:\n{}",
            errors.join("\n")
        )
    });
    let actual: Vec<(String, Vec<String>)> = report
        .census
        .iter()
        .map(|entry| {
            (
                entry.function.clone(),
                entry
                    .declarations
                    .iter()
                    .map(|(module, _)| module.clone())
                    .collect(),
            )
        })
        .collect();
    let expected: Vec<(String, Vec<String>)> = EXPECTED_MULTI_MODULE
        .iter()
        .map(|(function, modules)| {
            (
                (*function).to_owned(),
                modules.iter().map(|module| (*module).to_owned()).collect(),
            )
        })
        .collect();
    assert_eq!(actual, expected);
    // Eleven functions, thirteen pairs, every pair registered exactly once
    // (W45-FB-RETRY-LIFETIME added `mk_list_del`).
    assert_eq!(census_pairs(&report.census).len(), 13);
    assert_eq!(REGISTERED_RELATIONS.len(), 13);
    assert_eq!(report.relations.len(), 13);
}

/// The library's own corpus reader and this test's discovery agree module for
/// module, byte for byte, so the census the report publishes is the corpus the
/// corpus test pins.
#[test]
fn the_census_reads_the_corpus_the_corpus_test_reads() {
    let index =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../contract/corpus.json");
    assert_eq!(read_corpus(&index).unwrap(), corpus());
}
