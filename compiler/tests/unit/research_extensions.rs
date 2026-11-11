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
        include_str!("../../../crates/qcore-c-gateway/tests/fixtures/contracts/q_base.qv")
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

#[test]
fn aggregate_closure_rejects_loop_evidence_mutated_from_exact_to_maximum() {
    if !crate::test_support::clang_available() {
        return;
    }

    let constraints = vec![InferenceConstraint::Owns { identity: 1 }];
    let inference = infer_principal(&constraints).unwrap();
    let body = Flow::acquire(q(1), q(1));
    let loop_summary = summarize_general_loop(&body, LoopBoundEvidence::Exact(2)).unwrap();
    // The aggregate closure checks the loop certificate before it reaches the
    // legacy evidence. Keep that evidence real anyway, so public certificate
    // construction cannot bypass its scope invariants.
    let unit = C11TranslationUnit {
        source: "void subject(void) { return; }",
        ..C11TranslationUnit::default()
    };
    let legacy = match analyze_unrestricted_c11(
        &unit,
        "subject",
        &crate::legacy_c::ResourceContracts::default(),
    ) {
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => certificate,
        other => panic!("fixture did not produce a legacy certificate: {other:?}"),
    };
    let mut forged = loop_summary.clone();
    forged.evidence = LoopBoundEvidence::AtMost(2);
    let c11 = C11Execution::default();
    let native = NativeProgram::default();
    let input = |summary| Cycle17Input {
        constraints: &constraints,
        inference: &inference,
        loop_summary: summary,
        legacy: &legacy,
        c11: &c11,
        native: &native,
        platform_runs: &[],
    };
    assert_eq!(
        close_cycle17(&input(&forged)),
        Err(Cycle17Error::Loop(LoopSummaryError::NonExactCustody))
    );
    // The unmutated certificate passes the same replay step and only then
    // stops on the placeholder evidence that follows it.
    assert!(!matches!(
        close_cycle17(&input(&loop_summary)),
        Err(Cycle17Error::Loop(_))
    ));
}
