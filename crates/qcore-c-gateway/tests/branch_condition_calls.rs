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

//! B02-BRANCH-CONDITION — custody effects of one unconditional call in `if`.
//!
//! The call is evaluated by the existing `ContractCall` transfer before the
//! condition splits.  Short-circuit and conditional evaluation remain refused;
//! loops and switches belong to B03, and nested call arguments belong to B04.

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "void *q_alloc(void); void q_free(void *);";
const BRANCH_REFUSAL: &str = "unmodeled call, aggregate or indirect access in branch condition";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(
        &[(
            "q_base.qcore".to_owned(),
            include_str!("fixtures/contracts/q_base.qv").to_owned(),
        )]
        .into_iter()
        .collect(),
    )
    .unwrap()
}

fn probe_contracts() -> ResourceContracts {
    load_resource_contract_sources(
        &[
            (
                "q_base.qcore".to_owned(),
                include_str!("fixtures/contracts/q_base.qv").to_owned(),
            ),
            (
                "probe.qcore".to_owned(),
                "module contract;\n\
                 extern fn q_probe() -> i32\n\
                 contract {\n\
                     requires resource_contract_v1;\n\
                     ensures normal resource_return_none();\n\
                 };"
                .to_owned(),
            ),
        ]
        .into_iter()
        .collect(),
    )
    .unwrap()
}

fn analyze(body: &str) -> (AnalysisStatus, Vec<LegacyDiagnosticKind>) {
    let source = format!("{DECLARATIONS}\nvoid subject(int flag) {{ {body} }}");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected: {error:?}\n{source}"));
    let summary = match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.unwrap(),
    };
    let result = &summary.functions["subject"];
    (
        result.status,
        result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect(),
    )
}

fn clang_available() -> bool {
    qcore_c_gateway::clang_command().is_ok()
}

fn has_branch_refusal(diagnostics: &[LegacyDiagnosticKind]) -> bool {
    diagnostics.iter().any(|kind| {
        matches!(kind, LegacyDiagnosticKind::Unsupported { description } if description == BRANCH_REFUSAL)
    })
}

#[test]
fn an_unconditional_release_runs_before_the_branch() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) =
        analyze("void *p = q_alloc(); if ((q_free(p), flag)) { flag = 1; } else { flag = 2; }");
    assert_eq!(status, AnalysisStatus::Safe, "{diagnostics:?}");
    assert!(!has_branch_refusal(&diagnostics));
}

#[test]
fn a_whole_scalar_result_call_is_evaluated_once_before_the_split() {
    if !clang_available() {
        return;
    }
    let source = "int q_probe(void); void subject(int flag) { if (q_probe() == 0) { flag = 1; } }";
    let all = probe_contracts();
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "subject",
        &all,
    )
    .unwrap();
    let summary = match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => certificate.summary,
        other => panic!("expected Safe, got {other:?}"),
    };
    assert_eq!(summary.functions["subject"].status, AnalysisStatus::Safe);
}

#[test]
fn the_unknown_call_result_keeps_both_branch_arms() {
    if !clang_available() {
        return;
    }
    let source = "int q_probe(void); void *q_alloc(void); void q_free(void *);\n\
                  void subject(void) {\n\
                      void *p = q_alloc();\n\
                      if (q_probe()) { q_free(p); }\n\
                  }";
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "subject",
        &probe_contracts(),
    )
    .unwrap();
    let summary = match outcome {
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        other => panic!("the else path leaks and must stay reachable, got {other:?}"),
    };
    assert_eq!(summary.functions["subject"].status, AnalysisStatus::Unsafe);
}

/// Flipped by card CG08-CASTS-AND-TYPE-QUERIES (rule 11b). The branch-call
/// route still never applies a call inside `sizeof` (C does not evaluate
/// it); the elaborator now reads the query's operand as unevaluated, so the
/// condition is call-free, no `q_probe` node exists and nothing is refused.
#[test]
fn a_call_inside_sizeof_is_never_applied_because_c_does_not_evaluate_it() {
    if !clang_available() {
        return;
    }
    let source =
        "int q_probe(void); void subject(int flag) { if (sizeof(q_probe()) > 0) { flag = 1; } }";
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "subject",
        &probe_contracts(),
    )
    .unwrap();
    let certificate = match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => certificate,
        other => panic!("expected Safe, got {other:?}"),
    };
    let function = &certificate.summary.functions["subject"];
    assert!(!has_branch_refusal(
        &function
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect::<Vec<_>>()
    ));
    assert!(!function.cfg.nodes.iter().any(|node| matches!(
        &node.instruction,
        qcore_c_gateway::legacy_c::LegacyInstruction::ContractCall { function, .. }
            if function == "q_probe"
    )));
}

#[test]
fn an_indirect_residual_cannot_be_masked_by_the_modeled_call() {
    if !clang_available() {
        return;
    }
    let source = "int q_probe(void);\n\
                  void subject(int flag, int *table) {\n\
                      if (q_probe() + table[flag]) { }\n\
                  }";
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "subject",
        &probe_contracts(),
    )
    .unwrap();
    let partial = match outcome {
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.unwrap(),
        other => panic!("expected Unknown, got {other:?}"),
    };
    assert!(has_branch_refusal(
        &partial.functions["subject"]
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect::<Vec<_>>()
    ));
}

#[test]
fn a_residual_read_through_the_released_pointer_keeps_the_whole_condition_refused() {
    if !clang_available() {
        return;
    }
    let source = "struct item { int flag; };\n\
                  void *q_alloc(void); void q_free(void *);\n\
                  void subject(void) {\n\
                      struct item *p = (struct item *) q_alloc();\n\
                      if ((q_free(p), p->flag)) { }\n\
                  }";
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap();
    let partial = match outcome {
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.unwrap(),
        other => panic!("expected Unknown, got {other:?}"),
    };
    let diagnostics = partial.functions["subject"]
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.kind.clone())
        .collect::<Vec<_>>();
    assert!(has_branch_refusal(&diagnostics), "{diagnostics:?}");
}

/// Card W44-BRANCH-CONDITION-CALLS replaces this pin's refusal assertion,
/// deliberately: the short-circuited release is now desugared onto the edge
/// on which C evaluates it (`legacy_condition_desugar.rs`). What the pin
/// protected is kept — the skipped-call path still retains the live resource,
/// so the leak is reported and the function is not `Safe`.
#[test]
fn a_short_circuit_release_is_desugared_and_cannot_hide_the_live_resource() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze("void *p = q_alloc(); if (flag && (q_free(p), 1)) { }");
    assert_eq!(status, AnalysisStatus::Unsafe);
    assert!(!has_branch_refusal(&diagnostics), "{diagnostics:?}");
    assert!(
        diagnostics
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. })),
        "the skipped-call path must retain the live resource: {diagnostics:?}"
    );
}

/// Card W44-BRANCH-CONDITION-CALLS replaces this pin's refusal assertion:
/// the release runs only on `flag`'s true edge, so the false edge leaks.
#[test]
fn a_conditional_release_is_desugared_and_the_other_arm_leaks() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze("void *p = q_alloc(); if (flag ? (q_free(p), 1) : 0) { }");
    assert_eq!(status, AnalysisStatus::Unsafe);
    assert!(!has_branch_refusal(&diagnostics), "{diagnostics:?}");
    assert!(
        diagnostics
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. })),
        "{diagnostics:?}"
    );
}

#[test]
fn two_calls_stay_refused_because_the_bounded_rule_models_one_call() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) =
        analyze("void *p = q_alloc(); void *q = q_alloc(); if ((q_free(p), q_free(q), flag)) { }");
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(has_branch_refusal(&diagnostics), "{diagnostics:?}");
}

#[test]
fn a_call_with_an_owned_result_stays_refused() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze("if (q_alloc() != 0) { }");
    assert_eq!(status, AnalysisStatus::Unknown);
    assert!(has_branch_refusal(&diagnostics), "{diagnostics:?}");
}

#[test]
fn an_uncontracted_call_stays_refused() {
    if !clang_available() {
        return;
    }
    let source = "int external(int); void subject(int flag) { if (external(flag)) { } }";
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap();
    let partial = match outcome {
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.unwrap(),
        other => panic!("expected Unknown, got {other:?}"),
    };
    assert!(has_branch_refusal(
        &partial.functions["subject"]
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect::<Vec<_>>()
    ));
}
