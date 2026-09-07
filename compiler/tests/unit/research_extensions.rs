use super::*;
use crate::inference::{InferenceConstraint, infer_principal};
use crate::legacy_c::{
    C11TranslationUnit, LegacyCError, UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use crate::native_concurrency::{NativeOperation, NativeScheduleStep, RequiredPlatform};
use crate::resource_flow::Flow;
use crate::resource_flow::Quantity;
use crate::weak_memory::{C11Execution, MemoryEvent, MemoryEventKind};
use qcore_core::native_concurrency::{NativeProgram, PlatformRun};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

fn q(amount: u64) -> Quantity {
    Quantity::single("memory", amount)
}

#[test]
fn aggregate_certificate_contains_all_six_discharges() {
    if !crate::test_support::clang_available() {
        return;
    }
    let constraints = vec![InferenceConstraint::Owns { identity: 1 }];
    let inference = infer_principal(&constraints).unwrap();
    let loop_summary =
        summarize_general_loop(&Flow::acquire(q(1), q(1)), LoopBoundEvidence::Exact(2)).unwrap();
    // W1 literal-site migration: the fixture environment is loaded from a
    // `.qcore` module through the total-or-reject v1 loader.
    let contracts = crate::contract_loader::load_resource_contract_sources(&BTreeMap::from([(
        "q_base.qcore".to_owned(),
        include_str!("../../../crates/qcore-c-gateway/tests/fixtures/contracts/q_base.qcore")
            .to_owned(),
    )]))
    .unwrap();
    let unit = C11TranslationUnit {
        source: "void *q_alloc(void); void q_free(void *); void subject(void) { void *p = q_alloc(); q_free(p); }",
        ..C11TranslationUnit::default()
    };
    let legacy = match analyze_unrestricted_c11(&unit, "subject", &contracts) {
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => certificate,
        Err(LegacyCError::ProductionParserUnavailable(_)) => return,
        other => panic!("fixture did not produce a legacy certificate: {other:?}"),
    };
    let c11 = C11Execution {
        events: vec![MemoryEvent {
            id: 0,
            thread: 0,
            kind: MemoryEventKind::AtomicInit {
                location: 1,
                value: 0,
            },
        }],
        source_observation: vec![0],
        target_observation: vec![0],
        ..C11Execution::default()
    };
    let native = NativeProgram {
        tasks: BTreeSet::from([0]),
        initial_custody: BTreeMap::from([(8, 0)]),
        schedule: vec![NativeScheduleStep {
            task: 0,
            operation: NativeOperation::Destroy { resource: 8 },
        }],
        ..NativeProgram::default()
    };
    let runs = vec![
        PlatformRun {
            platform: RequiredPlatform::Linux,
            passed: true,
            sanitizer: true,
        },
        PlatformRun {
            platform: RequiredPlatform::Windows,
            passed: true,
            sanitizer: true,
        },
    ];
    let certificate = close_cycle17(&Cycle17Input {
        constraints: &constraints,
        inference: &inference,
        loop_summary: &loop_summary,
        legacy: &legacy,
        c11: &c11,
        native: &native,
        platform_runs: &runs,
    })
    .unwrap();
    assert!(certificate.remaining_research.is_empty());
    assert_eq!(certificate.categorical.resource_morphisms.len(), 2);
}
