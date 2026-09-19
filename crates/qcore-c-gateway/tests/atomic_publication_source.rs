use qcore_c_gateway::atomic_publication_source::{
    AtomicPublicationExecutionBounds, AtomicPublicationSourceProfile,
    AtomicPublicationSourceRefusal, check_atomic_publication_source,
};
use qcore_c_gateway::legacy_c::C11TranslationUnit;

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

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
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
    qcore_c_gateway::atomic_publication_source::AtomicPublicationSourceCertificate,
    AtomicPublicationSourceRefusal,
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
fn release_acquire_source_derives_the_conditional_publication_edge() {
    if !clang_available() {
        return;
    }
    let certificate = check(SOURCE).unwrap();
    assert_eq!(certificate.initial_value, 0);
    assert_eq!(certificate.published_value, 1);
    assert_eq!(certificate.successful_observation, 1);
    assert!(
        certificate
            .model
            .execution
            .happens_before
            .contains(&certificate.model.payload_happens_before)
    );
}

#[test]
fn volatile_and_plain_flags_establish_no_synchronization() {
    if !clang_available() {
        return;
    }
    let ordinary = r#"
static int done = 0;
static int payload;
void publish(void) { payload = 7; done = 1; }
int consume(void) { if (done != 1) return 0; return payload; }
"#;
    assert_eq!(
        check(ordinary),
        Err(AtomicPublicationSourceRefusal::PlainFlagHasNoSynchronization)
    );
    assert_eq!(
        check(&ordinary.replace("static int done", "static volatile int done")),
        Err(AtomicPublicationSourceRefusal::VolatileFlagHasNoSynchronization)
    );
}

#[test]
fn atomic_spelling_without_release_acquire_is_refused() {
    if !clang_available() {
        return;
    }
    let relaxed_store = SOURCE.replace("memory_order_release", "memory_order_relaxed");
    assert_eq!(
        check(&relaxed_store),
        Err(AtomicPublicationSourceRefusal::Model(
            qcore_core::AtomicPublicationRefusal::StoreIsNotRelease
        ))
    );
    let relaxed_load = SOURCE.replace("memory_order_acquire", "memory_order_relaxed");
    assert_eq!(
        check(&relaxed_load),
        Err(AtomicPublicationSourceRefusal::Model(
            qcore_core::AtomicPublicationRefusal::LoadIsNotAcquire
        ))
    );
    let ordinary_store = SOURCE.replace(
        "atomic_store_explicit(&done, 1, memory_order_release);",
        "done = 1;",
    );
    assert_eq!(
        check(&ordinary_store),
        Err(AtomicPublicationSourceRefusal::UnsupportedPublisher)
    );
    let shadowed_order = SOURCE.replace(
        "void publish(void) {",
        "void publish(int memory_order_release) {",
    );
    assert_eq!(
        check(&shadowed_order),
        Err(AtomicPublicationSourceRefusal::UnsupportedMemoryOrder)
    );
    let effectful_payload = SOURCE
        .replace(
            "static int payload;",
            "static int payload;\nint calculate(void) { return 7; }",
        )
        .replace("payload = 7;", "payload = calculate();");
    assert_eq!(
        check(&effectful_payload),
        Err(AtomicPublicationSourceRefusal::UnsupportedPublisher)
    );
}

#[test]
fn source_shape_refuses_bypasses_ambiguity_and_extra_accesses() {
    if !clang_available() {
        return;
    }
    let reversed = SOURCE.replace(
        "payload = 7;\n    atomic_store_explicit(&done, 1, memory_order_release);",
        "atomic_store_explicit(&done, 1, memory_order_release);\n    payload = 7;",
    );
    assert_eq!(
        check(&reversed),
        Err(AtomicPublicationSourceRefusal::UnsupportedPublisher)
    );
    let bypass = SOURCE.replace("if (seen != 1) return 0;", "(void) seen;");
    assert_eq!(
        check(&bypass),
        Err(AtomicPublicationSourceRefusal::UnsupportedConsumer)
    );
    let same_initial = SOURCE.replace("done = 0", "done = 1");
    assert_eq!(
        check(&same_initial),
        Err(AtomicPublicationSourceRefusal::Model(
            qcore_core::AtomicPublicationRefusal::AmbiguousPublishedValue
        ))
    );
    let extra_store = SOURCE.replace(
        "payload = 7;",
        "atomic_store_explicit(&done, 2, memory_order_relaxed);\n    payload = 7;",
    );
    assert_eq!(
        check(&extra_store),
        Err(AtomicPublicationSourceRefusal::UnsupportedPublisher)
    );
    let hidden_store = format!(
        "{SOURCE}\nvoid rewrite(void) {{ atomic_store_explicit(&done, 2, memory_order_relaxed); }}\n"
    );
    assert_eq!(
        check(&hidden_store),
        Err(AtomicPublicationSourceRefusal::ExtraFlagAccess)
    );
    let extra_read = SOURCE.replace("return payload;", "(void) payload; return payload;");
    assert_eq!(
        check(&extra_read),
        Err(AtomicPublicationSourceRefusal::UnsupportedConsumer)
    );
    let hidden_read = format!("{SOURCE}\nint inspect(void) {{ return payload; }}\n");
    assert_eq!(
        check(&hidden_read),
        Err(AtomicPublicationSourceRefusal::ExtraPayloadAccess)
    );
    let repeated = format!("{SOURCE}\nvoid twice(void) {{ publish(); publish(); }}\n");
    assert_eq!(
        check(&repeated),
        Err(AtomicPublicationSourceRefusal::ExtraEntryCall)
    );
}

#[test]
fn invocation_counts_are_explicit_bounded_execution_premises() {
    if !clang_available() {
        return;
    }
    let result = check_atomic_publication_source(
        &C11TranslationUnit {
            source: SOURCE,
            ..Default::default()
        },
        "fixture-revision",
        &profile(),
        AtomicPublicationExecutionBounds {
            publisher_invocations: 2,
            successful_consumer_invocations: 1,
        },
    );
    assert_eq!(
        result,
        Err(AtomicPublicationSourceRefusal::UnsupportedExecutionBounds)
    );
}
