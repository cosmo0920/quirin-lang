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
    let source = SOURCE.replace("#include <stdatomic.h>",
        "typedef enum memory_order { memory_order_relaxed = 0, memory_order_acquire = 0, memory_order_release = 0, memory_order_seq_cst = 0 } memory_order;")
        .replace("atomic_store_explicit", "__c11_atomic_store")
        .replace("atomic_load_explicit", "__c11_atomic_load");
    assert_eq!(check(&source), Err(AtomicPublicationRefusal::Source(
        qcore_c_gateway::atomic_publication_source::AtomicPublicationSourceRefusal::UnsupportedMemoryOrder
    )));
}

/// Card CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i): the production gate is
/// `Safe` only when both halves accept (`QCore.AtomicSourceRoute.
/// safe_publication_only_when_checked`), and every verdict carries the
/// single-graph assumption (`publication_verdict_carries_single_graph`).
#[test]
fn cg19_publication_verdict_gates_on_the_composed_check() {
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
    let safe = verdict(SOURCE);
    assert_eq!(safe.status(), "Safe");
    assert!(safe.assumptions().contains(&single_graph));
    // A model refusal and a source refusal are both Unknown, with the same
    // assumptions.
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
        assert_eq!(unknown.assumptions(), safe.assumptions());
    }
}
