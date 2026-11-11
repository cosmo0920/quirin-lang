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

use qcore_c_gateway::atomic_publication_source::{
    AtomicPublicationExecutionBounds, AtomicPublicationOrder, AtomicPublicationSourceProfile,
    AtomicPublicationSourceRefusal, extract_atomic_publication_source,
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

fn clang_atomic_ast_available() -> bool {
    if !clang_available() {
        return false;
    }
    match qcore_c_gateway::clang_version() {
        Ok(version) if version.supports_cg19_atomic_ast() => true,
        Ok(version) => {
            eprintln!(
                "skipping atomic-publication AST observation on Clang {}.{}.{}: CG19 requires Clang 18 or newer",
                version.major, version.minor, version.patch
            );
            false
        }
        Err(reason) => panic!("Clang was resolved but its release could not be detected: {reason}"),
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
    extract_atomic_publication_source(
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
fn release_acquire_source_derives_the_closed_typed_candidate() {
    if !clang_atomic_ast_available() {
        return;
    }
    let certificate = check(SOURCE).unwrap();
    assert_eq!(certificate.initial_value, 0);
    assert_eq!(certificate.published_value, 1);
    assert_eq!(certificate.successful_observation, 1);
    assert_eq!(
        certificate.semantic_profile,
        "c11-bounded-atomic-publication-source-v1"
    );
    assert!(
        certificate
            .scope
            .contains("full preprocessed translation unit")
    );
    assert_eq!(certificate.analysis_configuration_sha256.len(), 64);
    assert_eq!(certificate.store_order, AtomicPublicationOrder::Release);
    assert_eq!(certificate.load_order, AtomicPublicationOrder::Acquire);
}

#[test]
fn integer_conversions_never_supply_publication_values() {
    if !clang_atomic_ast_available() {
        return;
    }
    // On the test target these literals undergo an IntegralCast to int. The
    // first stores 0 and the second stores 1; reading their literal tokens as
    // i64 would invent the very value distinction used to identify reads-from.
    let narrowed_initializer = SOURCE.replace("done = 0", "done = 4294967296");
    assert_eq!(
        check(&narrowed_initializer),
        Err(AtomicPublicationSourceRefusal::UnsupportedInitializer)
    );
    let narrowed_publication = SOURCE.replace(
        "atomic_store_explicit(&done, 1, memory_order_release);",
        "atomic_store_explicit(&done, 4294967297, memory_order_release);",
    );
    assert_eq!(
        check(&narrowed_publication),
        Err(AtomicPublicationSourceRefusal::UnsupportedPublisher)
    );
    let converted_ready = SOURCE.replace("seen != 1", "seen != 4294967297");
    assert_eq!(
        check(&converted_ready),
        Err(AtomicPublicationSourceRefusal::UnsupportedConsumer)
    );
    let unsigned_initializer = SOURCE.replace("done = 0", "done = 0u");
    assert_eq!(
        check(&unsigned_initializer),
        Err(AtomicPublicationSourceRefusal::UnsupportedInitializer)
    );
}

#[test]
fn declaration_and_function_attributes_are_not_ignored() {
    if !clang_available() {
        return;
    }
    // Clang accepts these two declarations even though their assembler names
    // alias. Distinct AST declaration ids are therefore insufficient proof of
    // distinct memory locations.
    let assembler_alias = SOURCE.replace(
        "static _Atomic int done = 0;",
        "static _Atomic int done __asm__(\"payload\") = 0;",
    );
    assert_eq!(
        check(&assembler_alias),
        Err(AtomicPublicationSourceRefusal::UnsupportedHiddenAccess)
    );
    let attributed_publisher = SOURCE.replace(
        "void publish(void) {",
        "__attribute__((noinline)) void publish(void) {",
    );
    assert_eq!(
        check(&attributed_publisher),
        Err(AtomicPublicationSourceRefusal::UnsupportedPublisher)
    );
}

#[test]
fn inline_assembly_cannot_bypass_the_full_translation_unit_inventory() {
    if !clang_available() {
        return;
    }
    let hidden_writer = format!(
        "{SOURCE}\nvoid hidden(void) {{ __asm__ volatile(\"movl $2, done(%%rip)\" ::: \"memory\"); }}\n"
    );
    assert_eq!(
        check(&hidden_writer),
        Err(AtomicPublicationSourceRefusal::UnsupportedHiddenAccess)
    );
    let file_scope_asm = format!("{SOURCE}\n__asm__(\".globl ct41_hidden_symbol\");\n");
    assert_eq!(
        check(&file_scope_asm),
        Err(AtomicPublicationSourceRefusal::UnsupportedHiddenAccess)
    );
}

#[test]
fn redeclarations_cannot_change_the_identity_used_by_reference_counting() {
    if !clang_atomic_ast_available() {
        return;
    }
    let flag_redeclaration = format!(
        "{SOURCE}\nvoid hidden(void) {{ extern _Atomic int done; atomic_store_explicit(&done, 2, memory_order_relaxed); }}\n"
    );
    assert_eq!(
        check(&flag_redeclaration),
        Err(AtomicPublicationSourceRefusal::RedeclaredProfileEntity(
            "flag"
        ))
    );
    let payload_redeclaration =
        format!("{SOURCE}\nint hidden(void) {{ extern int payload; return payload; }}\n");
    assert_eq!(
        check(&payload_redeclaration),
        Err(AtomicPublicationSourceRefusal::RedeclaredProfileEntity(
            "payload"
        ))
    );
    let publisher_prototype = SOURCE.replace(
        "void publish(void) {",
        "void publish(void);\nvoid publish(void) {",
    );
    assert_eq!(
        check(&publisher_prototype),
        Err(AtomicPublicationSourceRefusal::RedeclaredProfileEntity(
            "publisher"
        ))
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
int consume(void) { int seen = done; if (seen != 1) return 0; return payload; }
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
fn atomic_spelling_extracts_orders_without_claiming_synchronization() {
    if !clang_atomic_ast_available() {
        return;
    }
    let relaxed_store = SOURCE.replace("memory_order_release", "memory_order_relaxed");
    assert_eq!(
        check(&relaxed_store).unwrap().store_order,
        AtomicPublicationOrder::Relaxed
    );
    let relaxed_load = SOURCE.replace("memory_order_acquire", "memory_order_relaxed");
    assert_eq!(
        check(&relaxed_load).unwrap().load_order,
        AtomicPublicationOrder::Relaxed
    );
    // Flipped by card CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i): an
    // `_Atomic` simple assignment is a sequentially consistent store (C11
    // 6.5.16.1p2, 7.17.7), read through the atomic source route. BEFORE it
    // was refused as an unsupported publisher.
    let ordinary_store = SOURCE.replace(
        "atomic_store_explicit(&done, 1, memory_order_release);",
        "done = 1;",
    );
    assert_eq!(
        check(&ordinary_store).unwrap().store_order,
        AtomicPublicationOrder::SequentiallyConsistent
    );
    let shadowed_order = SOURCE.replace(
        "void publish(void) {",
        "void publish(int memory_order_release) {",
    );
    assert_eq!(
        check(&shadowed_order),
        Err(AtomicPublicationSourceRefusal::UnsupportedMemoryOrder)
    );
    let shadowed_load_order = SOURCE.replace(
        "int consume(void) {",
        "int consume(int memory_order_acquire) {",
    );
    assert_eq!(
        check(&shadowed_load_order),
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
    let initialized_payload = SOURCE.replace("static int payload;", "static int payload = 3;");
    assert_eq!(
        check(&initialized_payload),
        Err(AtomicPublicationSourceRefusal::PayloadMustBeStaticInt)
    );
}

#[test]
fn source_shape_refuses_bypasses_ambiguity_and_extra_accesses() {
    if !clang_atomic_ast_available() {
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
    let ambiguous = check(&same_initial).unwrap();
    assert_eq!(ambiguous.initial_value, ambiguous.published_value);
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
    let result = extract_atomic_publication_source(
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

#[test]
fn non_atomic_refusal_requires_the_actual_closed_publication_shape() {
    if !clang_available() {
        return;
    }
    let source =
        include_str!("../../../compiler/tests/fixtures/ct/CT41/case.c").replace("\r\n", "\n");
    assert_eq!(
        check(&source),
        Err(AtomicPublicationSourceRefusal::VolatileFlagHasNoSynchronization)
    );
    let bypass = source.replace("if (seen != 1) return 0;", "(void) seen;");
    assert_eq!(
        check(&bypass),
        Err(AtomicPublicationSourceRefusal::UnsupportedConsumer)
    );
    let reversed = source.replace("payload = 7;\n    done = 1;", "done = 1;\n    payload = 7;");
    assert_eq!(
        check(&reversed),
        Err(AtomicPublicationSourceRefusal::UnsupportedPublisher)
    );
    let extra_writer = format!("{source}\nvoid extra(void) {{ done = 2; }}\n");
    assert_eq!(
        check(&extra_writer),
        Err(AtomicPublicationSourceRefusal::ExtraFlagAccess)
    );
    let extra_reader = format!("{source}\nint extra(void) {{ return payload; }}\n");
    assert_eq!(
        check(&extra_reader),
        Err(AtomicPublicationSourceRefusal::ExtraPayloadAccess)
    );
    let repeated = format!("{source}\nvoid extra(void) {{ publish(); }}\n");
    assert_eq!(
        check(&repeated),
        Err(AtomicPublicationSourceRefusal::ExtraEntryCall)
    );
}

#[test]
fn memory_order_enum_values_are_semantic_not_name_only() {
    if !clang_atomic_ast_available() {
        return;
    }
    let declarations = "typedef enum memory_order { memory_order_relaxed = 0, memory_order_acquire = 2, memory_order_release = 3, memory_order_seq_cst = 5 } memory_order;";
    let direct = SOURCE
        .replace("#include <stdatomic.h>", declarations)
        .replace("atomic_store_explicit", "__c11_atomic_store")
        .replace("atomic_load_explicit", "__c11_atomic_load");
    assert_eq!(
        check(&direct).unwrap().store_order,
        AtomicPublicationOrder::Release
    );
    for (from, to) in [
        ("memory_order_release = 3", "memory_order_release = 0"),
        ("memory_order_acquire = 2", "memory_order_acquire = 0"),
        ("memory_order_seq_cst = 5", "memory_order_seq_cst = 0"),
        ("memory_order_release = 3", "memory_order_release = 2"),
        ("memory_order_acquire = 2", "memory_order_acquire = 3"),
    ] {
        assert_eq!(
            check(&direct.replace(from, to)),
            Err(AtomicPublicationSourceRefusal::UnsupportedMemoryOrder),
            "{to}"
        );
    }
}

/// Card CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i): publication beyond the
/// one `atomic_*_explicit` shape, each derived stage recorded, and the route's
/// named refusals.
#[test]
fn cg19_publication_forms_through_the_atomic_route() {
    if !clang_atomic_ast_available() {
        return;
    }
    // Clang's builtins with literal orders: no `memory_order` enumeration.
    let builtins = r#"
static _Atomic int done = 0;
static int payload;
void publish(void) { payload = 7; __c11_atomic_store(&done, 1, __ATOMIC_RELEASE); }
int consume(void) { int seen = __c11_atomic_load(&done, __ATOMIC_ACQUIRE); if (seen != 1) return 0; return payload; }
"#;
    let certificate = check(builtins).unwrap();
    assert_eq!(certificate.store_order, AtomicPublicationOrder::Release);
    assert_eq!(certificate.load_order, AtomicPublicationOrder::Acquire);
    assert_eq!(certificate.stages.initialise, "static `done` = 0");
    assert!(certificate.stages.publish.contains("__c11_atomic_store"));
    assert!(certificate.stages.observe.contains("`seen != 1`"));
    assert_eq!(
        certificate.stages.reclaim,
        "not derived: reclamation is outside this route (CG29)"
    );
    // GNU builtins on a non-`_Atomic` flag every access of which is one.
    let gnu = r#"
static int done = 0;
static int payload;
void publish(void) { payload = 7; __atomic_store_n(&done, 1, __ATOMIC_RELEASE); }
int consume(void) { int seen = __atomic_load_n(&done, __ATOMIC_ACQUIRE); if (seen == 1) return payload; return 0; }
"#;
    let certificate = check(gnu).unwrap();
    assert!(certificate.stages.observe.contains("`seen == 1`"));
    assert!(
        certificate
            .assumptions
            .iter()
            .any(|a| a.starts_with("GNU __atomic builtins on a non-_Atomic int object"))
    );
    // One GNU access and one plain access: not the GNU atomic reading; the
    // plain arm's shape reads no builtin store.
    let mixed = gnu.replace("__atomic_load_n(&done, __ATOMIC_ACQUIRE)", "done");
    assert_eq!(
        check(&mixed),
        Err(AtomicPublicationSourceRefusal::UnsupportedPublisher)
    );
    // `_Atomic` operator forms: sequentially consistent both ways.
    let qualified = SOURCE
        .replace(
            "atomic_store_explicit(&done, 1, memory_order_release);",
            "done = 1;",
        )
        .replace("atomic_load_explicit(&done, memory_order_acquire)", "done");
    let certificate = check(&qualified).unwrap();
    assert_eq!(
        (certificate.store_order, certificate.load_order),
        (
            AtomicPublicationOrder::SequentiallyConsistent,
            AtomicPublicationOrder::SequentiallyConsistent
        )
    );
    // A consume load and a read-modify-write are refused by the route's name.
    let consume = SOURCE.replace("memory_order_acquire", "memory_order_consume");
    assert_eq!(
        check(&consume),
        Err(AtomicPublicationSourceRefusal::NamedAtomicRefusal(
            "memory_order_consume load `__c11_atomic_load` is not represented by the weak-memory model (CG28)".into()
        ))
    );
    let exchange = SOURCE.replace(
        "atomic_store_explicit(&done, 1, memory_order_release);",
        "atomic_exchange_explicit(&done, 1, memory_order_release);",
    );
    assert_eq!(
        check(&exchange),
        Err(AtomicPublicationSourceRefusal::NamedAtomicRefusal(
            // Card CG28 (rule 11t): the exchange is now routed to the
            // weak-memory model, and still not a publication shape.
            "atomic `__c11_atomic_exchange` is routed to the weak-memory model but is not a publication shape this route decides (CG19-PUBLICATION-ONE-FLAG-ONE-PAYLOAD)".into()
        ))
    );
    // A test that reads the payload on the branch that did NOT observe the
    // ready value is not a publication shape.
    let inverted = gnu.replace(
        "if (seen == 1) return payload; return 0;",
        "if (seen != 1) return payload; return 0;",
    );
    assert_eq!(
        check(&inverted),
        Err(AtomicPublicationSourceRefusal::UnsupportedConsumer)
    );
}
