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

//! Old in-process counter route versus the new process-transported route.
//!
//! Both routes import the same typed paths and evaluate the same pinned
//! contract in the trusted core. The archived parity is certificate equality
//! on accepted functions and refusal agreement on rejected ones; the adapter
//! is retained, not replaced, by this increment.
use qcore_compiler::counter_contract::CounterCertificate;
use qcore_compiler::counter_source::{
    COUNTER_SEMANTIC_PROFILE, CounterSourceResult, CounterSourceTask, import_counter_sources,
};
use qcore_compiler::legacy_c::C11TranslationUnit;
use qcore_compiler::verifier_plugins::chunkio::{
    PINNED_FLUENT_BIT_REVISION,
    teardown_source::{COUNTER_CONTRACT, check_counter_sources},
};
use qcore_schemas::counter_summary::{CounterContractWitness, CounterUpdateRecord};
use qcore_schemas::{SemanticWitness, TargetProfile};
use qcore_verifier_host::WitnessChecker;

const FUNCTIONS: [&str; 4] = [
    "cio_chunk_counter_total_add",
    "cio_chunk_counter_total_sub",
    "cio_chunk_counter_total_up_add",
    "cio_chunk_counter_total_up_sub",
];
const ROOT_C_TYPE: &str = "struct cio_ctx *";
const ROOT_QCORE_TYPE: &str = "ChunkContext";
// Model unsigned counters without redefining the host's size_t.
const SOURCE: &str = "typedef unsigned long qcore_test_size_t; \
struct cio_ctx { qcore_test_size_t total_chunks; qcore_test_size_t total_chunks_up; }; \
qcore_test_size_t cio_chunk_counter_total_add(struct cio_ctx *ctx) { ctx->total_chunks++; return ctx->total_chunks; } \
qcore_test_size_t cio_chunk_counter_total_sub(struct cio_ctx *ctx) { ctx->total_chunks--; return ctx->total_chunks; } \
qcore_test_size_t cio_chunk_counter_total_up_add(struct cio_ctx *ctx) { ctx->total_chunks_up++; return ctx->total_chunks_up; } \
qcore_test_size_t cio_chunk_counter_total_up_sub(struct cio_ctx *ctx) { ctx->total_chunks_up--; return ctx->total_chunks_up; }";

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent counter parity test: {reason}");
            false
        }
    }
}

fn unit(source: &str) -> C11TranslationUnit<'_> {
    C11TranslationUnit {
        source,
        ..Default::default()
    }
}

fn target() -> TargetProfile {
    TargetProfile {
        library: "chunkio".into(),
        revision: PINNED_FLUENT_BIT_REVISION.into(),
        profile: COUNTER_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    }
}

fn new_route(source: &str) -> Vec<CounterSourceResult> {
    import_counter_sources(
        &unit(source),
        PINNED_FLUENT_BIT_REVISION,
        &FUNCTIONS,
        &["cio_ctx"],
        ROOT_C_TYPE,
    )
    .unwrap()
}

#[test]
fn both_routes_produce_identical_certificates_for_every_counter() {
    if !clang_available() {
        return;
    }
    let old: Vec<CounterCertificate> =
        check_counter_sources(&unit(SOURCE), PINNED_FLUENT_BIT_REVISION).unwrap();
    assert_eq!(old.len(), FUNCTIONS.len());
    let new = new_route(SOURCE);
    assert_eq!(new.len(), FUNCTIONS.len());
    for (index, function) in FUNCTIONS.iter().enumerate() {
        let CounterSourceResult::Import(import) = &new[index] else {
            panic!("{function} lost its typed import on the new route")
        };
        let replay = CounterSourceTask::derive(
            import,
            "cio_chunk.c",
            COUNTER_CONTRACT,
            ROOT_QCORE_TYPE,
            target(),
        )
        .unwrap();
        let recomputed = replay.evaluate().unwrap();
        assert_eq!(
            serde_json::to_value(&old[index]).unwrap(),
            serde_json::to_value(&recomputed).unwrap(),
            "{function}"
        );
        let expected_field = if function.contains("_up_") {
            "total_chunks_up"
        } else {
            "total_chunks"
        };
        let expected_delta = if function.ends_with("_add") { 1 } else { -1 };
        assert_eq!(recomputed.field(), expected_field);
        assert_eq!(recomputed.delta(), expected_delta);

        // The reading a process must propose, written out independently here.
        let witness = SemanticWitness::CounterContract(Box::new(CounterContractWitness {
            task_digest: replay.task().task_digest(),
            summary: CounterUpdateRecord {
                function: (*function).into(),
                source_revision: PINNED_FLUENT_BIT_REVISION.into(),
                root_c_type: ROOT_C_TYPE.into(),
                counter_c_type: "unsigned long".into(),
                field: expected_field.into(),
                delta: expected_delta,
                result_is_updated_counter: true,
                only_selected_field_written: true,
                profile_assumptions: Default::default(),
            },
        }));
        assert_eq!(
            replay.check(replay.task(), &witness).unwrap(),
            replay.task().coverage_for(replay.capability())
        );
    }
}

#[test]
fn both_routes_refuse_the_same_changed_sources() {
    if !clang_available() {
        return;
    }
    for (old_text, new_text) in [
        ("ctx->total_chunks--;", "ctx->total_chunks_up--;"),
        ("return ctx->total_chunks;", "return 0;"),
        (
            "ctx->total_chunks--;",
            "ctx->total_chunks--; ctx->total_chunks--;",
        ),
        ("qcore_test_size_t total_chunks;", "long total_chunks;"),
    ] {
        let changed = SOURCE.replace(old_text, new_text);
        assert!(
            check_counter_sources(&unit(&changed), PINNED_FLUENT_BIT_REVISION).is_err(),
            "old route accepted {new_text}"
        );
        // The new route either loses the projection at import or refuses the
        // contract at replay; in neither case can a witness discharge it.
        let results = new_route(&changed);
        let sub = &results[1];
        let refused = match sub {
            CounterSourceResult::Unknown { .. } => true,
            CounterSourceResult::Import(import) => match CounterSourceTask::derive(
                import,
                "cio_chunk.c",
                COUNTER_CONTRACT,
                ROOT_QCORE_TYPE,
                target(),
            ) {
                Ok(replay) => replay.evaluate().is_err(),
                Err(_) => true,
            },
        };
        assert!(refused, "new route accepted {new_text}");
    }
}
