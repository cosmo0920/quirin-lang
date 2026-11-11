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

//! Independent public-API tests for context-decreasing recursive custody.
//! Every positive case passes through Clang and certificate rederivation.
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, ContractTrust, LegacyDiagnosticKind, LegacyInstruction,
    LegacySafetyCertificate, ResourceContracts, ResourceFunctionContract, ResourceParameterMode,
    ResourceReturnMode, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "void *q_alloc(void); void q_free(void *); void q_transfer(void *);";
const RELEASE: &str =
    "void release(void *p, int flag) { if (flag) { release(p, 0); return; } q_free(p); }";
const CALL_RELEASE: &str = "void subject(void) { void *p = q_alloc(); release(p, 1); }";

fn contracts(extra: Option<&str>) -> ResourceContracts {
    let mut sources = vec![(
        "q_base.qcore".to_owned(),
        include_str!("fixtures/contracts/q_base.qv").to_owned(),
    )];
    if let Some(extra) = extra {
        sources.push(("declared.qcore".to_owned(), extra.to_owned()));
    }
    load_resource_contract_sources(&sources.into_iter().collect()).unwrap()
}

fn analyze(body: &str, extra: Option<&str>) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS}\n{body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "subject",
        &contracts(extra),
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected by Clang: {error:?}\n{source}"))
}

fn certified(body: &str) -> LegacySafetyCertificate {
    let result = analyze(body, None);
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("expected validated recursive custody: {result:?}");
    };
    check_legacy_safety_certificate(&certificate).unwrap();
    certificate
}

fn assert_contract(
    certificate: &LegacySafetyCertificate,
    function: &str,
    expected: ResourceFunctionContract,
) {
    let summary = certificate
        .summaries
        .iter()
        .find(|summary| summary.function == function)
        .unwrap_or_else(|| panic!("missing summary for {function}"));
    assert_eq!(summary.trust, ContractTrust::InferredValidated);
    assert_eq!(summary.contract, expected);
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent recursion test: {reason}");
            false
        }
    }
}

#[test]
fn nonzero_to_zero_release_has_a_rechecked_summary_for_both_contexts() {
    if !clang_available() {
        return;
    }
    for caller_flag in ["0", "1", "-7"] {
        let caller = CALL_RELEASE.replace("release(p, 1)", &format!("release(p, {caller_flag})"));
        let certificate = certified(&format!("{RELEASE}\n{caller}"));
        assert_contract(
            &certificate,
            "release",
            ResourceFunctionContract::new(
                [
                    ResourceParameterMode::Released { nullable: false },
                    ResourceParameterMode::Borrowed,
                ],
                ResourceReturnMode::None,
            ),
        );
    }
    let negated =
        "void release(void *p, int flag) { if (!flag) { q_free(p); return; } release(p, 0); }";
    certified(&format!("{negated}\n{CALL_RELEASE}"));
}

#[test]
fn recursive_owned_result_preserves_caller_custody() {
    if !clang_available() {
        return;
    }
    let make = "void *make(int flag) { if (flag) { void *r = make(0); return r; } void *p = q_alloc(); return p; }";
    let certificate = certified(&format!(
        "{make} void subject(void) {{ void *p = make(1); q_free(p); }}"
    ));
    assert_contract(
        &certificate,
        "make",
        ResourceFunctionContract::new(
            [ResourceParameterMode::Borrowed],
            ResourceReturnMode::Owned { nullable: false },
        ),
    );
    for leaked_caller in [
        "void subject(void) { void *p = make(1); }",
        "void subject(void) { make(1); }",
    ] {
        assert!(!matches!(
            analyze(&format!("{make} {leaked_caller}"), None),
            UnrestrictedLegacyResult::Safe(_)
        ));
    }
}

#[test]
fn bounded_mutual_recursion_validates_both_members_without_self_assumptions() {
    if !clang_available() {
        return;
    }
    let body = "void right(void *p, int flag); \
        void left(void *p, int flag) { if (flag) { right(p, 0); return; } q_free(p); } \
        void right(void *p, int flag) { if (flag) { left(p, 0); return; } q_free(p); } \
        void subject(void) { void *p = q_alloc(); left(p, 1); void *q = q_alloc(); right(q, -1); }";
    let certificate = certified(body);
    for function in ["left", "right"] {
        assert_contract(
            &certificate,
            function,
            ResourceFunctionContract::new(
                [
                    ResourceParameterMode::Released { nullable: false },
                    ResourceParameterMode::Borrowed,
                ],
                ResourceReturnMode::None,
            ),
        );
    }
}

#[test]
fn nondecreasing_or_unsupported_recursive_contexts_never_become_safe() {
    if !clang_available() {
        return;
    }
    for (case, body) in [
        (
            "unchanged",
            "if (flag) { release(p, flag); return; } q_free(p);",
        ),
        (
            "nonzero literal",
            "if (flag) { release(p, 1); return; } q_free(p);",
        ),
        (
            "arithmetic countdown",
            "if (flag) { release(p, flag - 1); return; } q_free(p);",
        ),
        (
            "zero cycle",
            "if (!flag) { release(p, 0); return; } q_free(p);",
        ),
        (
            "control assignment",
            "flag = 1; if (flag) { release(p, 0); return; } q_free(p);",
        ),
        (
            "postfix control mutation",
            "if (flag--) { release(p, 0); return; } q_free(p);",
        ),
        (
            "address escape",
            "int *alias = &flag; *alias = 1; if (flag) { release(p, 0); return; } q_free(p);",
        ),
        (
            "control shadow",
            "if (flag) { int flag = 0; release(p, 0); return; } q_free(p);",
        ),
        (
            "inconsistent contexts",
            "if (flag) { release(p, 0); q_free(p); return; } return;",
        ),
        (
            "double release",
            "if (flag) { release(p, 0); q_free(p); return; } q_free(p);",
        ),
        (
            "consume after release",
            "if (flag) { release(p, 0); q_transfer(p); return; } q_free(p);",
        ),
    ] {
        let source = format!("void release(void *p, int flag) {{ {body} }} {CALL_RELEASE}");
        let result = analyze(&source, None);
        assert!(
            !matches!(result, UnrestrictedLegacyResult::Safe(_)),
            "{case}: {result:?}"
        );
    }
    let zero_cycle = "void right(void *p, int flag); \
        void left(void *p, int flag) { if (flag) { q_free(p); return; } right(p, 0); } \
        void right(void *p, int flag) { if (flag) { q_free(p); return; } left(p, 0); } \
        void subject(void) { void *p = q_alloc(); left(p, 0); }";
    assert!(!matches!(
        analyze(zero_cycle, None),
        UnrestrictedLegacyResult::Safe(_)
    ));
}

#[test]
fn unsupported_control_types_uses_and_recursive_result_loss_remain_unproved() {
    if !clang_available() {
        return;
    }
    for (case, body) in [
        (
            "nested control expression",
            "void release(void *p, int flag) { if (flag + 0) { release(p, 0); return; } q_free(p); } void subject(void) { void *p = q_alloc(); release(p, 1); }",
        ),
        (
            "nested control call",
            "int identity(int n) { return n; } void release(void *p, int flag) { if (identity(flag)) { release(p, 0); return; } q_free(p); } void subject(void) { void *p = q_alloc(); release(p, 1); }",
        ),
        (
            "control passed to another function",
            "void scalar(int n) { return; } void release(void *p, int flag) { scalar(flag); if (flag) { release(p, 0); return; } q_free(p); } void subject(void) { void *p = q_alloc(); release(p, 1); }",
        ),
        (
            "multiple scalar controls",
            "void release(void *p, int flag, int other) { if (flag) { release(p, 0, 0); return; } q_free(p); } void subject(void) { void *p = q_alloc(); release(p, 1, 0); }",
        ),
        (
            "floating control",
            "void release(void *p, double flag) { if (flag) { release(p, 0); return; } q_free(p); } void subject(void) { void *p = q_alloc(); release(p, 1); }",
        ),
        (
            "looped recursive body",
            "void release(void *p, int flag) { while (flag) { release(p, 0); return; } q_free(p); } void subject(void) { void *p = q_alloc(); release(p, 1); }",
        ),
        (
            "discarded recursive allocation inside wrapper",
            "void *make(int flag) { if (flag) { make(0); void *p = q_alloc(); return p; } void *p = q_alloc(); return p; } void subject(void) { void *p = make(1); q_free(p); }",
        ),
    ] {
        let result = analyze(body, None);
        assert!(
            !matches!(result, UnrestrictedLegacyResult::Safe(_)),
            "{case}: {result:?}"
        );
    }
}

#[test]
fn ordinary_proof_of_unreachable_recursion_is_preserved() {
    if !clang_available() {
        return;
    }
    // The existing scalar CFG can prove this assignment forces the base path.
    // Contextual inference may reject the mutable control, but must preserve
    // this already validated ordinary nonrecursive proof.
    let body = "void release(void *p, int flag) { flag = 0; if (flag) { release(p, 0); return; } q_free(p); }";
    certified(&format!("{body} {CALL_RELEASE}"));
}

#[test]
fn declared_conflict_cannot_be_overridden_by_recursive_inference() {
    if !clang_available() {
        return;
    }
    let declared = "module contract; extern fn release(p: ptr<mut, u8>, flag: i32) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(p); ensures normal resource_borrowed(flag); ensures normal resource_return_none(); };";
    let result = analyze(&format!("{RELEASE}\n{CALL_RELEASE}"), Some(declared));
    let summary = match &result {
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        }
        | UnrestrictedLegacyResult::Unsafe(summary) => summary,
        _ => panic!("expected declared custody to retain an unresolved result: {result:?}"),
    };
    // The declared component stays on the ordinary analysis route. The
    // recursive call still borrows; inference must not silently replace it
    // with a release summary and erase the caller's remaining obligation.
    assert!(summary.functions["subject"].cfg.nodes.iter().any(|node| matches!(
        &node.instruction,
        LegacyInstruction::ContractCall { function, contract, .. }
            if function == "release" && contract.parameters[0] == ResourceParameterMode::Borrowed
    )));
    assert!(
        summary.functions["subject"]
            .diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind, LegacyDiagnosticKind::Leak { .. })),
        "{result:?}"
    );
}

#[test]
fn certificate_recheck_rejects_source_and_summary_context_substitution() {
    if !clang_available() {
        return;
    }
    let certificate = certified(&format!("{RELEASE}\n{CALL_RELEASE}"));
    let mut stale = certificate.clone();
    stale.preprocessed_source = stale
        .preprocessed_source
        .replace("release(p, 0)", "release(p, flag)");
    assert_ne!(stale.preprocessed_source, certificate.preprocessed_source);
    assert!(check_legacy_safety_certificate(&stale).is_err());
    let mut forged = certificate.clone();
    forged
        .summaries
        .iter_mut()
        .find(|summary| summary.function == "release")
        .unwrap()
        .contract
        .parameters[0] = ResourceParameterMode::Borrowed;
    assert!(check_legacy_safety_certificate(&forged).is_err());
}

#[test]
fn unsigned_countdown_release_checks_every_builtin_width_and_guard() {
    if !clang_available() {
        return;
    }
    for control_type in [
        "unsigned",
        "unsigned int",
        "unsigned char",
        "unsigned short",
        "unsigned short int",
        "unsigned long",
        "unsigned long int",
        "unsigned long long",
        "unsigned long long int",
    ] {
        for body in [
            "if (n) { release(p, n - 1); return; } q_free(p);",
            "if (!n) { q_free(p); return; } release(p, n - 1);",
            "if (n == 0) { q_free(p); return; } release(p, n - 1);",
            "if (n != 0) { release(p, n - 1); return; } q_free(p);",
        ] {
            let certificate = certified(&format!(
                "void release(void *p, {control_type} n) {{ {body} }} void subject(void) {{ void *p = q_alloc(); release(p, 7); }}"
            ));
            assert_contract(
                &certificate,
                "release",
                ResourceFunctionContract::new(
                    [
                        ResourceParameterMode::Released { nullable: false },
                        ResourceParameterMode::Borrowed,
                    ],
                    ResourceReturnMode::None,
                ),
            );
        }
    }
}

#[test]
fn unsigned_mutual_countdown_handles_changed_control_slots_and_widths() {
    if !clang_available() {
        return;
    }
    let body = "void right(unsigned char n, void *p); \
        void left(void *p, unsigned long n) { if (n) { right(n - 1, p); return; } q_free(p); } \
        void right(unsigned char n, void *p) { if (n) { left(p, n - 1); return; } q_free(p); } \
        void subject(void) { void *p = q_alloc(); left(p, 513); void *q = q_alloc(); right(7, q); }";
    let certificate = certified(body);
    assert_contract(
        &certificate,
        "right",
        ResourceFunctionContract::new(
            [
                ResourceParameterMode::Borrowed,
                ResourceParameterMode::Released { nullable: false },
            ],
            ResourceReturnMode::None,
        ),
    );
    let mixed_edges = body.replace("left(p, n - 1)", "left(p, 0)");
    certified(&mixed_edges);
}

#[test]
fn unsigned_countdown_preserves_owned_returns_and_caller_leaks() {
    if !clang_available() {
        return;
    }
    let make = "void *make(unsigned n) { if (n) { void *r = make(n - 1); return r; } void *p = q_alloc(); return p; }";
    let certificate = certified(&format!(
        "{make} void subject(void) {{ void *p = make(7); q_free(p); }}"
    ));
    assert_contract(
        &certificate,
        "make",
        ResourceFunctionContract::new(
            [ResourceParameterMode::Borrowed],
            ResourceReturnMode::Owned { nullable: false },
        ),
    );
    for caller in [
        "void subject(void) { void *p = make(7); }",
        "void subject(void) { make(7); }",
    ] {
        assert!(!matches!(
            analyze(&format!("{make} {caller}"), None),
            UnrestrictedLegacyResult::Safe(_)
        ));
    }
}

#[test]
fn unsupported_countdowns_and_invalid_resource_steps_never_publish() {
    if !clang_available() {
        return;
    }
    for body in [
        "if (n) { release(p, n); return; } q_free(p);",
        "if (n) { release(p, n + 1); return; } q_free(p);",
        "if (n) { release(p, n - 2); return; } q_free(p);",
        "if (n) { release(p, 1 - n); return; } q_free(p);",
        "if (n) { release(p, n--); return; } q_free(p);",
        "if (n) { release(p, --n); return; } q_free(p);",
        "if (n) { release(p, n - 1); return; } release(p, n - 1);",
        "if (!n) { release(p, n - 1); return; } q_free(p);",
        "if (n) { release(p, n - 1); return; } release(p, 0);",
        "n = 7; if (n) { release(p, n - 1); return; } q_free(p);",
        "unsigned *alias = &n; if (n) { release(p, n - 1); return; } q_free(p);",
        "if (n) { unsigned n = 7; release(p, n - 1); return; } q_free(p);",
        "if (n) { release(p, n - 1); q_free(p); return; } q_free(p);",
        "if (n) { q_free(p); release(p, n - 1); return; } q_free(p);",
        "if (n) { release(p, n - 1); q_transfer(p); return; } q_free(p);",
        "if (n) { release(p, n - 1); return; } return;",
        "if (n) { release(p, n - 1); release(p, n - 1); return; } q_free(p);",
    ] {
        let source = format!("void release(void *p, unsigned n) {{ {body} }} {CALL_RELEASE}");
        let result = analyze(&source, None);
        assert!(
            !matches!(result, UnrestrictedLegacyResult::Safe(_)),
            "{source}: {result:?}"
        );
    }
    for control_type in ["int", "signed char", "short", "long", "_Bool"] {
        let source = format!(
            "void release(void *p, {control_type} n) {{ if (n) {{ release(p, n - 1); return; }} q_free(p); }} {CALL_RELEASE}"
        );
        assert!(
            !matches!(analyze(&source, None), UnrestrictedLegacyResult::Safe(_)),
            "{control_type}"
        );
    }
    let mixed_signed = "void right(void *p, int n); void left(void *p, unsigned n) { if (n) { right(p, n - 1); return; } q_free(p); } void right(void *p, int n) { if (n) { left(p, 0); return; } q_free(p); } void subject(void) { void *p = q_alloc(); left(p, 7); }";
    assert!(!matches!(
        analyze(mixed_signed, None),
        UnrestrictedLegacyResult::Safe(_)
    ));
}

#[test]
fn countdown_certificate_rechecks_rank_guards_and_summary_identity() {
    if !clang_available() {
        return;
    }
    let body =
        "void release(void *p, unsigned n) { if (n) { release(p, n - 1); return; } q_free(p); }";
    let certificate = certified(&format!("{body} {CALL_RELEASE}"));
    for (old, new) in [
        ("n - 1", "n + 1"),
        ("n - 1", "n - 2"),
        ("n - 1", "n"),
        ("unsigned n", "int n"),
        ("if (n)", "if (!n)"),
    ] {
        let mut stale = certificate.clone();
        stale.preprocessed_source = stale.preprocessed_source.replace(old, new);
        assert_ne!(stale.preprocessed_source, certificate.preprocessed_source);
        assert!(
            check_legacy_safety_certificate(&stale).is_err(),
            "{old} -> {new}"
        );
    }
    let mut forged = certificate.clone();
    forged
        .summaries
        .iter_mut()
        .find(|summary| summary.function == "release")
        .unwrap()
        .contract
        .parameters[1] = ResourceParameterMode::Owned { nullable: false };
    assert!(check_legacy_safety_certificate(&forged).is_err());
}

#[test]
fn countdown_native_fixture_has_independently_rechecked_custody() {
    if !clang_available() {
        return;
    }
    let certificate = certified(include_str!("fixtures/unsigned_countdown.c"));
    for function in ["countdown_release", "countdown_make", "wide", "narrow"] {
        assert!(
            certificate
                .summaries
                .iter()
                .any(|summary| summary.function == function)
        );
    }
}
