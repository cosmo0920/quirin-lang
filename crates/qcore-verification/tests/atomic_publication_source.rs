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

use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_verification::atomic_publication_source::{
    AtomicPublicationExecutionBounds, AtomicPublicationRefusal, AtomicPublicationSourceProfile,
    check_atomic_publication_source,
};

const SOURCE: &str = r#"
#include <stdatomic.h>
static _Atomic int done = 0;
static int payload;
void publish(void) {
    payload = 7;
    atomic_store_explicit(&done, 1, memory_order_release);
}
int consume(void) {
    int seen = atomic_load_explicit(&done, memory_order_acquire);
    if (seen != 1) return 0;
    return payload;
}
"#;

fn clang_atomic_ast_available() -> bool {
    match qcore_c_gateway::clang_version() {
        Ok(version) if version.supports_cg19_atomic_ast() => true,
        Ok(version) => {
            eprintln!(
                "skipping atomic-publication AST observation on Clang {}.{}.{}: CG19 requires Clang 18 or newer",
                version.major, version.minor, version.patch
            );
            false
        }
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "invalid explicit Clang: {reason}"
            );
            eprintln!("skipping Clang-dependent atomic publication test: {reason}");
            false
        }
    }
}

fn profile() -> AtomicPublicationSourceProfile<'static> {
    AtomicPublicationSourceProfile {
        publisher: "publish",
        consumer: "consume",
        flag: "done",
        payload: "payload",
    }
}

fn check(
    source: &str,
) -> Result<
    qcore_verification::atomic_publication_source::AtomicPublicationCertificate,
    AtomicPublicationRefusal,
> {
    check_atomic_publication_source(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "fixture-revision",
        &profile(),
        AtomicPublicationExecutionBounds {
            publisher_invocations: 1,
            successful_consumer_invocations: 1,
        },
    )
}

#[test]
fn extracted_release_acquire_source_is_checked_by_the_core_model() {
    if !clang_atomic_ast_available() {
        return;
    }
    let certificate = check(SOURCE).unwrap();
    assert_eq!(
        certificate.semantic_profile,
        "c11-bounded-atomic-publication-v1"
    );
    assert!(
        certificate
            .model
            .execution
            .happens_before
            .contains(&certificate.model.payload_happens_before)
    );
}

#[test]
fn extracted_controls_do_not_acquire_a_model_certificate() {
    if !clang_atomic_ast_available() {
        return;
    }
    let relaxed_store = SOURCE.replace("memory_order_release", "memory_order_relaxed");
    assert_eq!(
        check(&relaxed_store),
        Err(AtomicPublicationRefusal::Model(
            qcore_core::AtomicPublicationRefusal::StoreIsNotRelease
        ))
    );
    let relaxed_load = SOURCE.replace("memory_order_acquire", "memory_order_relaxed");
    assert_eq!(
        check(&relaxed_load),
        Err(AtomicPublicationRefusal::Model(
            qcore_core::AtomicPublicationRefusal::LoadIsNotAcquire
        ))
    );
    let ambiguous = SOURCE.replace("done = 0", "done = 1");
    assert_eq!(
        check(&ambiguous),
        Err(AtomicPublicationRefusal::Model(
            qcore_core::AtomicPublicationRefusal::AmbiguousPublishedValue
        ))
    );
}

#[test]
fn misleading_enum_names_cannot_publish_an_ordering_certificate() {
    if !clang_atomic_ast_available() {
        return;
    }
    let source = SOURCE.replace("#include <stdatomic.h>",
        "typedef enum memory_order { memory_order_relaxed = 0, memory_order_acquire = 0, memory_order_release = 0, memory_order_seq_cst = 0 } memory_order;")
        .replace("atomic_store_explicit", "__c11_atomic_store")
        .replace("atomic_load_explicit", "__c11_atomic_load");
    assert_eq!(check(&source), Err(AtomicPublicationRefusal::Source(
        qcore_c_gateway::atomic_publication_source::AtomicPublicationSourceRefusal::UnsupportedMemoryOrder
    )));
}

/// Card CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i), amended by card
/// CG29-EXECUTION-COVERAGE-AND-RECLAMATION (rule 11z): the production gate is
/// `Safe` only when both halves accept and the extracted shape is race-free
/// over every candidate execution (`QCore.AtomicSourceRoute.
/// safe_publication_only_when_checked`). `Safe` carries the all-executions
/// assumption and never the single-graph one
/// (`safe_publication_covers_every_execution`); `Unknown` carries the
/// single-graph one (`unknown_publication_carries_single_graph`).
#[test]
fn cg19_publication_verdict_gates_on_the_composed_check() {
    if !clang_atomic_ast_available() {
        return;
    }
    use qcore_verification::atomic_publication_source::{
        AtomicPublicationVerdict, atomic_publication_verdict,
    };
    let verdict = |source: &str| {
        atomic_publication_verdict(
            &C11TranslationUnit {
                source,
                ..Default::default()
            },
            "fixture-revision",
            &profile(),
            AtomicPublicationExecutionBounds {
                publisher_invocations: 1,
                successful_consumer_invocations: 1,
            },
        )
    };
    let single_graph = qcore_c_gateway::atomic_source_route::SINGLE_GRAPH_ASSUMPTION;
    let all_executions = qcore_c_gateway::atomic_source_route::ALL_EXECUTIONS_ASSUMPTION;
    let safe = verdict(SOURCE);
    assert_eq!(safe.status(), "Safe");
    assert!(safe.assumptions().contains(&all_executions));
    assert!(!safe.assumptions().contains(&single_graph));
    // A model refusal and a source refusal are both Unknown, with the
    // single-graph assumption.
    for control in [
        SOURCE.replace("memory_order_release", "memory_order_relaxed"),
        SOURCE.replace("static _Atomic int done", "static volatile int done"),
        SOURCE.replace("memory_order_acquire", "memory_order_consume"),
    ] {
        let unknown = verdict(&control);
        assert!(
            matches!(unknown, AtomicPublicationVerdict::Unknown { .. }),
            "{control}"
        );
        assert_eq!(unknown.status(), "Unknown");
        assert!(unknown.assumptions().contains(&single_graph));
        assert!(!unknown.assumptions().contains(&all_executions));
    }
}

/// Card CG29 (rule 11z): the extracted shape's fragment and its coverage
/// within the stated bounds, for every order pair the extractor admits. The
/// same fragment with relaxed orders (which the core model refuses first)
/// races at the program level.
#[test]
fn cg29_extracted_shape_is_race_free_over_every_execution() {
    if !clang_atomic_ast_available() {
        return;
    }
    use qcore_core::execution_coverage::{Instruction, ProgramVerdict, check_program_executions};
    use qcore_verification::atomic_publication_source::{
        PUBLICATION_COVERAGE_BOUNDS, publication_fragment,
    };
    for (store, load) in [
        ("memory_order_release", "memory_order_acquire"),
        ("memory_order_seq_cst", "memory_order_acquire"),
        ("memory_order_release", "memory_order_seq_cst"),
        ("memory_order_seq_cst", "memory_order_seq_cst"),
    ] {
        let source = SOURCE
            .replace("memory_order_release", store)
            .replace("memory_order_acquire", load);
        let certificate =
            check(&source).unwrap_or_else(|refusal| panic!("{store} {load}: {refusal:?}"));
        let evidence = &certificate.execution_coverage;
        assert_eq!(evidence.bounds, PUBLICATION_COVERAGE_BOUNDS);
        assert_eq!(evidence.threads, 2);
        assert_eq!(evidence.event_bound, 7);
        assert!(evidence.candidates <= 16, "{store} {load}: {evidence:?}");
        assert!(
            certificate
                .assumptions
                .contains(&qcore_c_gateway::atomic_source_route::ALL_EXECUTIONS_ASSUMPTION)
        );
        let mut relaxed = publication_fragment(&certificate.source);
        for instruction in relaxed.threads.iter_mut().flatten() {
            if let Instruction::Store { order, .. } | Instruction::Load { order, .. } =
                &mut instruction.instruction
            {
                *order = qcore_core::MemoryOrder::Relaxed;
            }
        }
        assert!(matches!(
            check_program_executions(&relaxed, PUBLICATION_COVERAGE_BOUNDS),
            ProgramVerdict::Race { .. }
        ));
    }
}
