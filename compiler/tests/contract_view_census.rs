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
/// `source_identity` (`monkey/timeout_teardown.qv` and
/// `monkey/wait_source_policy.qv`, textually identical) was removed from
/// the timeout policy by this card, so it is absent; restoring it fails here.
const EXPECTED_MULTI_MODULE: &[(&str, &[&str])] = &[
    (
        "_mk_event_loop_create",
        &["monkey/wait_source_policy.qv", "monkey/wrapper_paths.qv"],
    ),
    (
        "_mk_event_loop_destroy",
        &["monkey/wait_source_policy.qv", "monkey/wrapper_paths.qv"],
    ),
    (
        "_mk_event_wait_2",
        &["monkey/wait_source_policy.qv", "monkey/wrapper_paths.qv"],
    ),
    (
        "_mk_event_wait_with_flags",
        &["monkey/wait_source_policy.qv", "monkey/wrapper_paths.qv"],
    ),
    (
        "cb_wait_2_timeout",
        &["monkey/composition.qv", "monkey/wait_source_policy.qv"],
    ),
    (
        "cfl_list_size",
        &["cfl/null_deref.qv", "cfl/sds_lifetime.qv"],
    ),
    (
        "cio_chunk_close",
        &[
            "chunkio/chunk_destructor.qv",
            "chunkio/memory_close.qv",
            "chunkio/resource_open_close.qv",
        ],
    ),
    (
        "cio_error_reset",
        &["chunkio/error_frame.qv", "chunkio/error_state.qv"],
    ),
    // Card NULL-SAFETY-PROPERTY: `quant null-deref` checks (bounds contract
    // v1) beside the resource-v1 declarations (relation `null_deref_beside`).
    (
        "cmt_destroy",
        &["cmetrics/context_lifetime.qv", "cmetrics/null_deref.qv"],
    ),
    (
        "flb_calloc",
        &["fluent_bit/memory.qv", "fluent_bit/resource_memory.qv"],
    ),
    (
        "flb_free",
        &["fluent_bit/memory.qv", "fluent_bit/resource_memory.qv"],
    ),
    (
        "flb_input_chunk_get_name",
        &["fluent_bit/input_chunk_name.qv", "fluent_bit/null_deref.qv"],
    ),
    // Card FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE: the stable plugin descriptor's
    // host beside its resource-v1 declaration (`null_deref_beside`).
    (
        "flb_input_new",
        &["fluent_bit/input_instance.qv", "fluent_bit/null_deref.qv"],
    ),
    // ... and the input worker's exit signal's function-scope entry.
    (
        "flb_input_thread_instance_exit",
        &[
            "fluent_bit/lib_engine_thread.qv",
            "fluent_bit/null_deref.qv",
        ],
    ),
    (
        "flb_malloc",
        &["fluent_bit/memory.qv", "fluent_bit/resource_memory.qv"],
    ),
    // Card CELL-GUARD-RESULT-RANGE-CHECKED: the channel creator's derived
    // result range beside its resource-v1 declaration.
    (
        "mk_event_channel_create",
        &["monkey/event_loop.qv", "monkey/result_range.qv"],
    ),
    // Card W45-FB-RETRY-LIFETIME: the resource-v1 view of the unlink.
    (
        "mk_list_del",
        &["chunkio/list_unlink.qv", "monkey/intrusive_list.qv"],
    ),
    // Card CELL-GUARD-RESULT-RANGE-CHECKED: `open`'s C library result set
    // (no custody) beside its resource-v1 declaration.
    ("open", &["libc/result_sets.qv", "libc/unistd.qv"]),
    // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES: the op destroyer's
    // resource-v1 declaration beside its `quant null-deref #rko`.
    (
        "rd_kafka_op_destroy",
        &["librdkafka/client.qv", "librdkafka/null_deref.qv"],
    ),
    // Card NULL-CANDIDATE-TRIAGE: the toppar constructor's member
    // declaration and rd_kafka_topic_keep's non-null result.
    (
        "rd_kafka_topic_keep",
        &["librdkafka/client.qv", "librdkafka/null_deref.qv"],
    ),
    (
        "rd_kafka_topic_partition_list_destroy",
        &["librdkafka/client.qv", "librdkafka/null_deref.qv"],
    ),
    // Card NULL-MEMBER-INVARIANT-SCOPE: a function-scope entry.
    (
        "rd_kafka_toppar_fetch_backoff",
        &["librdkafka/client.qv", "librdkafka/null_deref.qv"],
    ),
    (
        "rd_kafka_toppar_new0",
        &["librdkafka/client.qv", "librdkafka/null_deref.qv"],
    ),
    (
        "rd_list_add",
        &["librdkafka/client.qv", "librdkafka/null_deref.qv"],
    ),
    (
        "rd_list_destroy",
        &["librdkafka/client.qv", "librdkafka/null_deref.qv"],
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
    // Twenty functions, twenty-two pairs, every pair registered exactly
    // once (W45-FB-RETRY-LIFETIME added `mk_list_del`; NULL-SAFETY-PROPERTY
    // the five `quant null-deref` checks beside a resource-v1 declaration;
    // NULL-CANDIDATE-TRIAGE a member and a result declaration;
    // NULL-MEMBER-INVARIANT-SCOPE a function-scope entry;
    // W45-CALLEE-CASCADE-RESIDUAL round 3 `cfl_list_size`'s resource-v1
    // declaration beside its result range; CG22-OP-CALLBACKS-AND-SERVED-QUEUES
    // `rd_kafka_op_destroy`: twenty-one functions, twenty-three pairs;
    // FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE `flb_input_new`'s stable descriptor
    // and `flb_input_thread_instance_exit`'s scope entry: twenty-three
    // functions, twenty-five pairs; CELL-GUARD-RESULT-RANGE-CHECKED `open`'s
    // C library result set and `mk_event_channel_create`'s derived range:
    // twenty-five functions, twenty-seven pairs).
    assert_eq!(census_pairs(&report.census).len(), 27);
    assert_eq!(REGISTERED_RELATIONS.len(), 27);
    assert_eq!(report.relations.len(), 27);
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
