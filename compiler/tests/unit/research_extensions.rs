use super::*;
use crate::inference::{InferenceConstraint, infer_principal};
use crate::legacy_c::{
    C11TranslationUnit, LegacyCError, UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use crate::native_concurrency::{NativeOperation, NativeScheduleStep, RequiredPlatform};
use crate::resource_flow::Quantity;
use crate::weak_memory::{C11Execution, MemoryEvent, MemoryEventKind};

fn q(amount: u64) -> Quantity {
    Quantity::single("memory", amount)
}

#[test]
fn positive_and_negative_net_loops_have_checked_invariants() {
    let growing = Flow::acquire(q(2), q(2));
    let grown = summarize_general_loop(&growing, LoopBoundEvidence::Terminating(3)).unwrap();
    assert_eq!(grown.summary.net().get("memory"), 6);
    assert_eq!(grown.summary.peak().get("memory"), 6);
    assert_eq!(grown.summary.total().get("memory"), 6);
    assert_eq!(grown.summary.required_entry().get("memory"), 0);
    check_loop_summary(&grown).unwrap();

    let shrinking = Flow::release(q(2), Quantity::zero());
    let shrunk = summarize_general_loop(&shrinking, LoopBoundEvidence::Widened(3)).unwrap();
    assert_eq!(shrunk.summary.net().get("memory"), -6);
    assert_eq!(shrunk.summary.required_entry().get("memory"), 6);
}

#[test]
fn unbounded_loop_refuses_a_finite_resource_claim() {
    assert_eq!(
        summarize_general_loop(&Flow::zero(), LoopBoundEvidence::Unbounded),
        Err(LoopSummaryError::Unbounded)
    );
}

#[test]
fn ledger_morphisms_compose_only_at_equal_endpoints() {
    let first = LedgerMorphism {
        name: "acquire".into(),
        source: Ledger::new(),
        target: BTreeMap::from([("memory".into(), 2)]),
        delta: BTreeMap::from([("memory".into(), 2)]),
    };
    let second = LedgerMorphism {
        name: "release".into(),
        source: first.target.clone(),
        target: Ledger::new(),
        delta: BTreeMap::from([("memory".into(), -2)]),
    };
    let composed = first.then(&second).unwrap();
    assert!(composed.source.is_empty());
    assert!(composed.target.is_empty());
    let mut broken = second;
    broken.source.insert("memory".into(), 3);
    broken.delta.insert("memory".into(), -3);
    assert!(matches!(
        first.then(&broken),
        Err(CategoryError::EndpointMismatch { .. })
    ));
}

#[test]
fn aggregate_certificate_contains_all_six_discharges() {
    if !crate::test_support::clang_available() {
        return;
    }
    let constraints = vec![InferenceConstraint::Owns { identity: 1 }];
    let inference = infer_principal(&constraints).unwrap();
    let loop_summary = summarize_general_loop(
        &Flow::acquire(q(1), q(1)),
        LoopBoundEvidence::Terminating(2),
    )
    .unwrap();
    // W1 literal-site migration: the fixture environment is loaded from a
    // `.qcore` module through the total-or-reject v1 loader.
    let contracts = crate::contract_loader::load_resource_contract_sources(&BTreeMap::from([(
        "q_base.qcore".to_owned(),
        include_str!("../fixtures/contracts/q_base.qcore").to_owned(),
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
