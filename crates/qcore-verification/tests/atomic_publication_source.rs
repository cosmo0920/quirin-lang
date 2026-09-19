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
