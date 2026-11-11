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

//! Public-API matrix for the **implemented** neutral-frame recursion route
//! (card C03-NFR, units U1–U4 and U7 of
//! `docs/design/W4_RECURSIVE_SUMMARY_PROFILES.md` §9).
//!
//! Every case here runs the real gateway with **no declared contract for the
//! recursive component**, so what is measured is the derivation route itself
//! rather than the declared-contract emulation the design card probed. The
//! emulation and its one measured over-permissiveness stay pinned separately in
//! `c03_neutral_frame_recursion_probe.rs`; this file pins that the implemented
//! route does *not* inherit that permissiveness.
//!
//! The cardinal expectation is negative, as everywhere in W4: the only contract
//! this route ever publishes is the neutral one — every formal `Borrowed`,
//! result `None` — and a component that fails any obligation publishes nothing
//! and leaves its callers `Unknown`.
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, ContractTrust, LegacyDiagnosticKind,
    LegacySafetyCertificate, ResourceContracts, ResourceFunctionContract, ResourceParameterMode,
    ResourceReturnMode, ResourceSummaryRecord, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "void *q_alloc(void); void q_free(void *); void q_transfer(void *);";

fn contracts(declared: Option<&str>) -> ResourceContracts {
    let mut sources = vec![(
        "q_base.qcore".to_owned(),
        include_str!("fixtures/contracts/q_base.qv").to_owned(),
    )];
    if let Some(declared) = declared {
        sources.push(("nfr_declared.qcore".to_owned(), declared.to_owned()));
    }
    load_resource_contract_sources(&sources.into_iter().collect()).unwrap()
}

fn analyze(program: &str, declared: Option<&str>) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS}\n{program}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "subject",
        &contracts(declared),
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected by Clang: {error:?}\n{source}"))
}

/// The whole run reduced to what these tests assert about.
struct Run {
    entry: AnalysisStatus,
    functions: std::collections::BTreeMap<String, AnalysisStatus>,
    diagnostics: std::collections::BTreeMap<String, Vec<LegacyDiagnosticKind>>,
    records: std::collections::BTreeMap<String, ResourceSummaryRecord>,
    certificate: Option<LegacySafetyCertificate>,
}

impl Run {
    fn status(&self, function: &str) -> AnalysisStatus {
        *self
            .functions
            .get(function)
            .unwrap_or_else(|| panic!("no projection for `{function}`"))
    }

    fn diagnostics(&self, function: &str) -> &[LegacyDiagnosticKind] {
        self.diagnostics
            .get(function)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    fn has_unresolved_alias(&self, function: &str) -> bool {
        self.diagnostics(function)
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::UnknownAlias { .. }))
    }

    fn unsupported(&self, function: &str) -> Vec<String> {
        self.diagnostics(function)
            .iter()
            .filter_map(|kind| match kind {
                LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
                _ => None,
            })
            .collect()
    }

    fn certificate(&self) -> &LegacySafetyCertificate {
        self.certificate
            .as_ref()
            .expect("expected a Safe certificate")
    }
}

fn run(program: &str, declared: Option<&str>) -> Run {
    let (entry, summary, records, certificate) = match analyze(program, declared) {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(&certificate)
                .expect("a Safe certificate must recheck independently");
            let records = certificate
                .summaries
                .iter()
                .map(|record| (record.function.clone(), record.clone()))
                .collect();
            (
                AnalysisStatus::Safe,
                certificate.summary.clone(),
                records,
                Some(certificate),
            )
        }
        UnrestrictedLegacyResult::Unsafe(summary) => (
            AnalysisStatus::Unsafe,
            summary,
            std::collections::BTreeMap::new(),
            None,
        ),
        UnrestrictedLegacyResult::Unknown { partial, .. } => (
            AnalysisStatus::Unknown,
            partial.expect("these fixtures always project partially"),
            std::collections::BTreeMap::new(),
            None,
        ),
    };
    Run {
        entry,
        functions: summary
            .functions
            .iter()
            .map(|(name, function)| (name.clone(), function.status))
            .collect(),
        diagnostics: summary
            .functions
            .iter()
            .map(|(name, function)| {
                (
                    name.clone(),
                    function
                        .diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.kind.clone())
                        .collect(),
                )
            })
            .collect(),
        records,
        certificate,
    }
}

/// `N(m)` for an `arity`-ary member.
fn neutral(arity: usize) -> ResourceFunctionContract {
    ResourceFunctionContract::new(
        vec![ResourceParameterMode::Borrowed; arity],
        ResourceReturnMode::None,
    )
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent neutral-frame recursion test: {reason}");
            false
        }
    }
}

/// The corpus-shaped family the design's census bucket A describes, restricted
/// to the shapes reachable without C02-P's borrowed pointer-field reads: nine
/// distinct control shapes, none of which recurses through a pointer field.
///
/// Each entry is `(label, program, (member, arity)…)`. Every member must
/// publish exactly `N(m)`, every member and the caller must be `Safe`, and the
/// certificate must recheck — which re-runs the whole component computation.
#[allow(clippy::type_complexity)]
fn admitted_family() -> Vec<(&'static str, String, Vec<(&'static str, usize)>)> {
    vec![
        (
            "self recursion with a decreasing scalar",
            "void depth(void *p, int n) { if (n > 0) { depth(p, n - 1); } }\n\
             void subject(void) { void *p = q_alloc(); depth(p, 3); q_free(p); }"
                .to_owned(),
            vec![("depth", 2)],
        ),
        (
            "self recursion with no decreasing argument at all",
            "void depth(void *p, int n) { if (n) { depth(p, n); } }\n\
             void subject(void) { void *p = q_alloc(); depth(p, 3); q_free(p); }"
                .to_owned(),
            vec![("depth", 2)],
        ),
        (
            "unguarded self recursion with no reachable concrete exit",
            "void spin(void *p, int n) { spin(p, n); }\n\
             void subject(void) { void *p = q_alloc(); spin(p, 1); q_free(p); }"
                .to_owned(),
            vec![("spin", 2)],
        ),
        (
            "a recursive body that acquires and releases a local",
            "void depth(void *p, int n) { void *t = q_alloc(); if (n > 0) { depth(p, n - 1); } q_free(t); }\n\
             void subject(void) { void *p = q_alloc(); depth(p, 3); q_free(p); }"
                .to_owned(),
            vec![("depth", 2)],
        ),
        (
            "a mutual pair",
            "void even(void *p, int n); void odd(void *p, int n);\n\
             void even(void *p, int n) { if (n > 0) { odd(p, n - 1); } }\n\
             void odd(void *p, int n) { if (n > 0) { even(p, n - 1); } }\n\
             void subject(void) { void *p = q_alloc(); even(p, 4); q_free(p); }"
                .to_owned(),
            vec![("even", 2), ("odd", 2)],
        ),
        (
            "a three-member cycle",
            "void a(void *p, int n); void b(void *p, int n); void c(void *p, int n);\n\
             void a(void *p, int n) { if (n > 0) { b(p, n - 1); } }\n\
             void b(void *p, int n) { if (n > 0) { c(p, n - 1); } }\n\
             void c(void *p, int n) { if (n > 0) { a(p, n - 1); } }\n\
             void subject(void) { void *p = q_alloc(); a(p, 4); q_free(p); }"
                .to_owned(),
            vec![("a", 2), ("b", 2), ("c", 2)],
        ),
        (
            "a recursive scalar accumulator over a borrowed handle",
            "int weight(void *ctx, int n) { if (n <= 0) { return 0; } int r = weight(ctx, n - 1); return r + 1; }\n\
             void subject(void) { void *p = q_alloc(); int w = weight(p, 4); (void) w; q_free(p); }"
                .to_owned(),
            vec![("weight", 2)],
        ),
        (
            "two borrowed pointer formals, swapped on the recursive call",
            "void walk(void *lhs, void *rhs, int n) { if (n > 0) { walk(rhs, lhs, n - 1); } }\n\
             void subject(void) { void *p = q_alloc(); void *q = q_alloc(); walk(p, q, 3); q_free(p); q_free(q); }"
                .to_owned(),
            vec![("walk", 3)],
        ),
        (
            "a member that hands one of its own formals back, ignored by the caller",
            "void *pick(void *p, int n) { if (n > 0) { void *r = pick(p, n - 1); return r; } return p; }\n\
             void subject(void) { void *p = q_alloc(); void *q = pick(p, 3); (void) q; q_free(p); }"
                .to_owned(),
            vec![("pick", 2)],
        ),
    ]
}

/// U1, positive direction, plus V3 and V9 over the whole admitted family: the
/// published contract is *always* the bottom element, published under
/// `InferredValidated` with no new schema field or trust label, and the caller
/// of a component that publishes becomes `Safe`.
#[test]
fn neutral_components_publish_the_bottom_element_and_unblock_their_callers() {
    if !clang_available() {
        return;
    }
    for (label, program, members) in admitted_family() {
        let outcome = run(&program, None);
        assert_eq!(outcome.entry, AnalysisStatus::Safe, "{label}");
        assert_eq!(outcome.status("subject"), AnalysisStatus::Safe, "{label}");
        for (member, arity) in members {
            assert_eq!(outcome.status(member), AnalysisStatus::Safe, "{label}");
            assert!(
                outcome.diagnostics(member).is_empty(),
                "{label}: {:?}",
                outcome.diagnostics(member)
            );
            let record = outcome
                .records
                .get(member)
                .unwrap_or_else(|| panic!("{label}: no record for `{member}`"));
            assert_eq!(record.trust, ContractTrust::InferredValidated, "{label}");
            assert_eq!(record.contract, neutral(arity), "{label}");
        }
        // V10 is not a separate code path: the recheck re-runs component
        // detection, seeding and the descent from the certificate's own
        // preprocessed source, and `run` already demanded that it agree.
        assert!(outcome.certificate().summaries.iter().all(|record| {
            record.trust == ContractTrust::InferredValidated
                && record.contract.result == ResourceReturnMode::None
                && record
                    .contract
                    .parameters
                    .iter()
                    .all(|mode| *mode == ResourceParameterMode::Borrowed)
        }));
    }
}

/// U1, negative direction. The neutral assumption certifies the *absence* of a
/// custody effect and can never certify one, so every body that carries one
/// refuses the candidate and leaves the whole component uncontracted.
///
/// Card CG27-LOOPS-RECURSION-BUDGETS (gateway rule 11x) re-pinned the guard
/// here, deliberately: with `if (n > 0)` the first two bodies now carry a
/// signed threshold rank, which the route consulted after this one publishes
/// (`loops_recursion_budgets.rs`). `if (n)` on a signed control is no rank, so
/// this test keeps exercising what it is about: the neutral refusal.
#[test]
fn a_body_that_carries_an_effect_refuses_the_whole_component() {
    if !clang_available() {
        return;
    }
    let caller = "void subject(void) { void *p = q_alloc(); depth(p, 3); q_free(p); }";
    for (label, body) in [
        (
            "releases a formal",
            "void depth(void *p, int n) { if (n) { depth(p, n - 1); return; } q_free(p); }",
        ),
        (
            "consumes a formal",
            "void depth(void *p, int n) { if (n) { depth(p, n - 1); return; } q_transfer(p); }",
        ),
        (
            "stores a formal into a global",
            "static void *g_slot;\n\
             void depth(void *p, int n) { if (n) { depth(p, n - 1); return; } g_slot = p; }",
        ),
        (
            "calls a callee the environment does not contract",
            "void q_other(void *p);\n\
             void depth(void *p, int n) { if (n) { depth(p, n - 1); return; } q_other(p); }",
        ),
    ] {
        let outcome = run(&format!("{body}\n{caller}"), None);
        assert_eq!(outcome.status("depth"), AnalysisStatus::Unknown, "{label}");
        assert!(
            outcome
                .unsupported("depth")
                .iter()
                .any(|description| description == "call to uncontracted function `depth`"),
            "{label}: the demoted member's own recursive call stays unmodelled"
        );
        assert!(outcome.records.is_empty(), "{label}");
        assert_eq!(
            outcome.status("subject"),
            AnalysisStatus::Unknown,
            "{label}"
        );
    }

    // A leaked *local* acquisition is a defect of the body, not a custody
    // effect on the caller, so it is reported as one and still publishes
    // nothing.
    let leaks =
        "void depth(void *p, int n) { void *t = q_alloc(); if (n > 0) { depth(p, n - 1); } }";
    let outcome = run(&format!("{leaks}\n{caller}"), None);
    assert_eq!(outcome.status("depth"), AnalysisStatus::Unsafe);
    assert!(
        outcome
            .diagnostics("depth")
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. }))
    );
    assert!(outcome.records.is_empty());

    // An acquiring return is refused by V6/V7 rather than downgraded to a
    // weaker mode: there is no fallback from the neutral candidate. (Card
    // CG27: `if (n > 0)` would carry a signed threshold rank and an `Owned`
    // result; the unranked `if (n)` keeps the refusal under test.)
    let acquires = "void *make(void *p, int n) { if (n) { void *r = make(p, n - 1); return r; } void *f = q_alloc(); return f; }\n\
        void subject(void) { void *p = q_alloc(); void *q = make(p, 2); q_free(p); q_free(q); }";
    let outcome = run(acquires, None);
    assert_eq!(outcome.status("make"), AnalysisStatus::Unknown);
    assert!(outcome.records.is_empty());
}

/// The alias obligation is empty, and the consequences of that are fail-closed
/// in both directions: the assumption states no precondition, so a null actual
/// is fine; and the handback carries no custody, so a caller that releases it
/// reports an unresolved alias instead of silently double-freeing.
#[test]
fn a_neutral_contract_states_no_precondition_and_hands_back_no_custody() {
    if !clang_available() {
        return;
    }
    let outcome = run(
        "void depth(void *p, int n) { if (n > 0) { depth(p, n - 1); } }\n\
         void subject(void) { depth(0, 3); void *p = q_alloc(); q_free(p); }",
        None,
    );
    assert_eq!(outcome.entry, AnalysisStatus::Safe);
    assert_eq!(outcome.records["depth"].contract, neutral(2));

    let handback = "void *pick(void *p, int n) { if (n > 0) { void *r = pick(p, n - 1); return r; } return p; }";
    let outcome = run(
        &format!(
            "{handback}\nvoid subject(void) {{ void *p = q_alloc(); void *q = pick(p, 3); q_free(q); q_free(p); }}"
        ),
        None,
    );
    assert_eq!(outcome.status("pick"), AnalysisStatus::Safe);
    assert_eq!(outcome.status("subject"), AnalysisStatus::Unknown);
    assert!(outcome.has_unresolved_alias("subject"));
}

/// **The counterexample that fixes the assumption at the bottom element.**
///
/// `purge` is consistent with `resource_released(p)` and `mint` with an owned
/// result, but only vacuously: every concrete execution of both diverges, so
/// "if the call returns, the effect happened" has nothing to object to. The
/// design card measured that a *declared* effectful contract is therefore
/// validated by re-analysis (probe
/// `an_assumed_effectful_contract_validates_vacuously_under_divergence`).
/// The implemented route must never reach that state, and this pins why: the
/// only candidate it ever seeds for a member of its own component is the
/// neutral one, so the published contract for both bodies is `N(m)` and the
/// caller's obligation survives instead of being explained away.
///
/// Any future recursive family that assumes an effect must carry a rank, as the
/// contextual and unsigned-countdown routes do.
#[test]
fn a_divergent_effectful_candidate_is_never_seeded_by_the_route() {
    if !clang_available() {
        return;
    }
    let purge = "void purge(void *p, int n) { if (n > 0) { purge(p, n - 1); return; } purge(p, 1); }\n\
        void subject(void) { void *p = q_alloc(); purge(p, 2); }";
    let outcome = run(purge, None);
    // Newly surfaced by this route: with a *neutral* summary in force the
    // caller's acquisition is visibly undischarged. Under the declared
    // `Released` emulation the same caller was `Safe`.
    assert_eq!(outcome.entry, AnalysisStatus::Unsafe);
    assert_eq!(outcome.status("purge"), AnalysisStatus::Safe);
    assert!(
        outcome
            .diagnostics("subject")
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. })),
        "the caller's leak is reported, not explained away"
    );

    let mint = "void *mint(int n) { void *r = mint(n); return r; }\n\
        void subject(void) { void *p = mint(1); q_free(p); }";
    let outcome = run(mint, None);
    assert_eq!(outcome.status("mint"), AnalysisStatus::Safe);
    assert_eq!(outcome.status("subject"), AnalysisStatus::Unknown);
    assert!(
        outcome.has_unresolved_alias("subject"),
        "no owned result is minted from nothing; the caller stays fail-closed"
    );

    // Both bodies, seen through the route rather than through a declaration,
    // publish the bottom element and nothing else. Measured without the leaking
    // caller so the published vector is visible in a certificate.
    for (definition, member, arity) in [
        (
            "void purge(void *p, int n) { if (n > 0) { purge(p, n - 1); return; } purge(p, 1); }",
            "purge",
            2,
        ),
        (
            "void *mint(int n) { void *r = mint(n); return r; }",
            "mint",
            1,
        ),
    ] {
        let outcome = run(&format!("{definition}\nvoid subject(void) {{ }}"), None);
        assert_eq!(outcome.entry, AnalysisStatus::Safe, "{member}");
        assert_eq!(outcome.records[member].contract, neutral(arity), "{member}");
    }
}

/// The permissiveness gap the design card measured, pinned as a *divergence*
/// rather than adopted.
///
/// A declared contract outlives its own member's refusal, so under the
/// emulation `even` stayed `Safe` while `odd` was refused. The implemented
/// route has no declaration to outlive anything: withdrawing `odd` makes
/// `even`'s call unmodelled, and the descent withdraws the whole component.
/// Both halves are asserted in the same test so the divergence cannot silently
/// close in either direction.
#[test]
fn the_route_withdraws_a_component_the_declared_emulation_keeps() {
    if !clang_available() {
        return;
    }
    let mixed = "void even(void *p, int n); void odd(void *p, int n);\n\
        void even(void *p, int n) { if (n > 0) { odd(p, n - 1); } }\n\
        void odd(void *p, int n) { if (n > 0) { even(p, n - 1); return; } q_free(p); }\n\
        void subject(void) { void *p = q_alloc(); even(p, 4); q_free(p); }";

    let route = run(mixed, None);
    assert_eq!(route.status("odd"), AnalysisStatus::Unknown);
    assert_eq!(
        route.status("even"),
        AnalysisStatus::Unknown,
        "the route withdraws the whole component"
    );
    assert!(
        route
            .unsupported("even")
            .iter()
            .any(|description| description == "call to uncontracted function `odd`")
    );
    assert!(route.records.is_empty());

    let declared = "module contract;\n\
        extern fn even(p: ptr<mut, u8>, n: i32) -> unit\ncontract {\n\
        \x20   requires resource_contract_v1;\n\
        \x20   ensures normal resource_borrowed(p);\n\
        \x20   ensures normal resource_borrowed(n);\n\
        \x20   ensures normal resource_return_none();\n};\n\
        extern fn odd(p: ptr<mut, u8>, n: i32) -> unit\ncontract {\n\
        \x20   requires resource_contract_v1;\n\
        \x20   ensures normal resource_borrowed(p);\n\
        \x20   ensures normal resource_borrowed(n);\n\
        \x20   ensures normal resource_return_none();\n};\n";
    let emulation = run(mixed, Some(declared));
    assert_eq!(emulation.status("odd"), AnalysisStatus::Unknown);
    assert_eq!(
        emulation.status("even"),
        AnalysisStatus::Safe,
        "emulation artefact: the declaration outlives the refused member"
    );
}

/// U2 and its measured limit.
///
/// The descent demotes only failing members and re-validates the survivors, so
/// a member whose every call edge into a demoted member is graph-unreachable
/// could in principle survive. It does not, and the reason is a containment
/// gate rather than the lattice: a call into a demoted member compiles to an
/// `Unknown` instruction, which raises no diagnostic while it sits on an
/// unreachable path but which the certificate firewall refuses outright. The
/// route therefore refuses such a member itself, instead of publishing a
/// summary whose own containment check would later reject it, and publication
/// collapses to the design's default all-or-nothing (V4). This deviation from
/// unit U2's optimism is deliberate and recorded in
/// `docs/reports/remaining-c03-nfr/README.md`.
#[test]
fn a_graph_unreachable_edge_into_a_demoted_member_does_not_survive_the_descent() {
    if !clang_available() {
        return;
    }
    let unreachable_edge = "void a(void *p, int n); void b(void *p, int n);\n\
        void a(void *p, int n) { if (n > 0) { a(p, n - 1); } return; b(p, n); }\n\
        void b(void *p, int n) { if (n > 0) { a(p, n - 1); return; } q_free(p); }\n\
        void subject(void) { void *p = q_alloc(); a(p, 4); q_free(p); }";
    let outcome = run(unreachable_edge, None);
    assert_eq!(outcome.status("b"), AnalysisStatus::Unknown);
    assert_eq!(outcome.status("a"), AnalysisStatus::Unknown);
    assert!(outcome.records.is_empty());

    // The same component without the dead edge publishes both members, so the
    // refusal above is the retained `Unknown` instruction and not the shape of
    // the component.
    let live = unreachable_edge.replace("return; b(p, n);", "").replace(
        "if (n > 0) { a(p, n - 1); return; } q_free(p);",
        "if (n > 0) { a(p, n - 1); }",
    );
    let outcome = run(&live, None);
    for member in ["a", "b"] {
        assert_eq!(outcome.status(member), AnalysisStatus::Safe);
        assert_eq!(outcome.records[member].contract, neutral(2));
    }
}

/// V1 and V2. A declared contract keeps its component on the existing route,
/// and both ranked recursive routes still run first: a component the contextual
/// route can rank keeps its effectful contract instead of being flattened to
/// the neutral one.
#[test]
fn declared_and_ranked_routes_keep_precedence_over_the_neutral_route() {
    if !clang_available() {
        return;
    }
    // The contextual zero/nonzero route publishes `Released`, which the neutral
    // route can never propose. Precedence, not competition.
    let ranked = "void release(void *p, int flag) { if (flag) { release(p, 0); return; } q_free(p); }\n\
        void subject(void) { void *p = q_alloc(); release(p, 1); }";
    let outcome = run(ranked, None);
    assert_eq!(outcome.entry, AnalysisStatus::Safe);
    assert_eq!(
        outcome.records["release"].contract,
        ResourceFunctionContract::new(
            [
                ResourceParameterMode::Released { nullable: false },
                ResourceParameterMode::Borrowed
            ],
            ResourceReturnMode::None
        )
    );

    // The unsigned countdown route likewise keeps its own contract.
    let countdown = "void release(void *p, unsigned n) { if (n) { release(p, n - 1); return; } q_free(p); }\n\
        void subject(void) { void *p = q_alloc(); release(p, 3u); }";
    let outcome = run(countdown, None);
    assert_eq!(outcome.entry, AnalysisStatus::Safe);
    assert_eq!(
        outcome.records["release"].contract.parameters[0],
        ResourceParameterMode::Released { nullable: false }
    );
}

/// U7: a record derived inside a recursive component that carries a declaration
/// is the declaration read back, not corroboration of it, and is withheld.
///
/// The declaration keeps governing every call site — the verdicts below are
/// unchanged by the withholding — so nothing is lost except the ability of a
/// certificate consumer to mistake a declaration for an inference. A
/// *non*-recursive declared function is untouched, which is what keeps this
/// change confined to the component the derivation actually reads back.
#[test]
fn a_recursive_component_carrying_a_declaration_publishes_no_inferred_record() {
    if !clang_available() {
        return;
    }
    let declared_released = "module contract;\n\
        extern fn purge(p: ptr<mut, u8>, n: i32) -> unit\ncontract {\n\
        \x20   requires resource_contract_v1;\n\
        \x20   ensures normal resource_released(p, nonnull);\n\
        \x20   ensures normal resource_borrowed(n);\n\
        \x20   ensures normal resource_return_none();\n};\n";
    let divergent = "void purge(void *p, int n) { if (n > 0) { purge(p, n - 1); return; } purge(p, 1); }\n\
         void subject(void) { void *p = q_alloc(); purge(p, 2); }";
    let outcome = run(divergent, Some(declared_released));
    // The declaration still governs: the body validates against it and the
    // caller's obligation is discharged by the trusted assumption.
    assert_eq!(outcome.entry, AnalysisStatus::Safe);
    assert_eq!(outcome.status("purge"), AnalysisStatus::Safe);
    // But nothing is republished as an inference.
    assert!(
        !outcome.records.contains_key("purge"),
        "a declaration must not be readable back as an inference"
    );
    assert!(
        outcome
            .certificate()
            .contracts
            .contract("purge")
            .is_some_and(|contract| contract.parameters[0]
                == ResourceParameterMode::Released { nullable: false }),
        "the declaration itself still travels with the certificate"
    );

    // A non-recursive declared function keeps publishing its independent
    // corroboration: its derivation never read its own declaration back.
    let straight_line = "void wrap(void *p) { q_free(p); }\n\
        void subject(void) { void *p = q_alloc(); wrap(p); }";
    let declared_wrap = "module contract;\n\
        extern fn wrap(p: ptr<mut, u8>) -> unit\ncontract {\n\
        \x20   requires resource_contract_v1;\n\
        \x20   ensures normal resource_released(p, nonnull);\n\
        \x20   ensures normal resource_return_none();\n};\n";
    let outcome = run(straight_line, Some(declared_wrap));
    assert_eq!(outcome.entry, AnalysisStatus::Safe);
    assert_eq!(
        outcome.records["wrap"].contract.parameters[0],
        ResourceParameterMode::Released { nullable: false }
    );
}

/// §8's rejection boundary, kept closed. Recursion that walks a pointer field —
/// and the heap-link chain teardown the design rejects architecturally — are
/// refused before any recursion rule is consulted, so the neutral route neither
/// admits them nor makes them reachable.
#[test]
fn heap_link_chain_teardown_stays_refused_and_a_pointer_field_search_certifies() {
    if !clang_available() {
        return;
    }
    let teardown = "struct node { struct node *next; };\n\
        void purge(struct node *n) { if (!n) { return; } purge(n->next); q_free(n); }\n\
        void subject(void) { void *p = q_alloc(); purge(p); }";
    let outcome = run(teardown, None);
    assert_eq!(outcome.status("purge"), AnalysisStatus::Unknown);
    // Deliberate pin update, card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (rule
    // 12k): `n->next` is an actual W44 reads at a `Borrowed` position, so the
    // recursive call's callee, which has no contract or summary, is what
    // decides, and the refusal names it. The teardown stays refused and no
    // summary is recorded.
    assert!(
        outcome
            .unsupported("purge")
            .iter()
            .any(|description| description == "call to uncontracted function `purge`")
    );
    assert!(outcome.records.is_empty());

    let seek = "struct node { struct node *next; int key; };\n\
        int seek(struct node *n, int key) { if (!n) { return 0; } if (n->key == key) { return 1; } int r = seek(n->next, key); return r; }\n\
        void subject(void) { void *p = q_alloc(); seek(p, 1); q_free(p); }";
    let outcome = run(seek, None);
    // Deliberate pin update, card W43-INDEX-TRAVERSAL. Pointer-field
    // *recursion* was refused here only because its branch condition
    // `n->key == key` was an unmodeled read; the recursion rule never refused
    // it (design §6.4). That read is now the typed condition read, with its
    // dereference obligation on `n` (an unbound formal after its null test),
    // so the search is a neutral recursive component and certifies. The
    // heap-link *teardown* above, which releases through the chain, keeps its
    // refusal: that boundary is architectural (§8) and untouched.
    assert_eq!(outcome.status("seek"), AnalysisStatus::Safe);
    assert_eq!(
        outcome
            .records
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["seek", "subject"],
        "{:?}",
        outcome.records
    );
    assert!(
        outcome.records["seek"]
            .contract
            .parameters
            .iter()
            .all(|mode| *mode == ResourceParameterMode::Borrowed)
            && outcome.records["seek"].contract.result == ResourceReturnMode::None,
        "{:?}",
        outcome.records
    );
}

/// U3 / V10. The recheck repeats component detection, seeding and the descent
/// from the certificate's own preprocessed source, so a substituted member, a
/// widened component, a forged record and a withdrawn record each refuse.
#[test]
fn certificate_rederivation_refuses_substitution_widening_and_forgery() {
    if !clang_available() {
        return;
    }
    let mutual = "void even(void *p, int n); void odd(void *p, int n);\n\
        void even(void *p, int n) { if (n > 0) { odd(p, n - 1); } }\n\
        void odd(void *p, int n) { if (n > 0) { even(p, n - 1); } }\n\
        void subject(void) { void *p = q_alloc(); even(p, 4); q_free(p); }";
    let certificate = run(mutual, None).certificate().clone();
    check_legacy_safety_certificate(&certificate).unwrap();

    // A substituted member: `odd` now releases the formal, so the component
    // withdraws and the recorded vector cannot be reproduced.
    let mut substituted = certificate.clone();
    substituted.preprocessed_source = substituted
        .preprocessed_source
        .replace("if (n > 0) { even(p, n - 1); }", "q_free(p);");
    assert_ne!(
        substituted.preprocessed_source,
        certificate.preprocessed_source
    );
    assert!(check_legacy_safety_certificate(&substituted).is_err());

    // A widened component: the caller joins the cycle, which is a different
    // component computation and a different published vector.
    let mut widened = certificate.clone();
    widened.preprocessed_source = widened.preprocessed_source.replace(
        "void odd(void *p, int n) { if (n > 0) { even(p, n - 1); } }",
        "void odd(void *p, int n) { if (n > 0) { even(p, n - 1); subject(); } }",
    );
    assert_ne!(widened.preprocessed_source, certificate.preprocessed_source);
    assert!(check_legacy_safety_certificate(&widened).is_err());

    // A forged neutral record for a function the route never published.
    let mut forged = certificate.clone();
    forged.summaries.push(ResourceSummaryRecord::new(
        "q_free".to_owned(),
        ContractTrust::InferredValidated,
        neutral(1),
    ));
    assert!(check_legacy_safety_certificate(&forged).is_err());

    // A strengthened record for a member the route did publish.
    let mut strengthened = certificate.clone();
    strengthened
        .summaries
        .iter_mut()
        .find(|record| record.function == "odd")
        .unwrap()
        .contract
        .parameters[0] = ResourceParameterMode::Released { nullable: false };
    assert!(check_legacy_safety_certificate(&strengthened).is_err());

    // A withdrawn record: the recheck demands exact equality of the vector, so
    // dropping one is refused as well as adding one.
    let mut withdrawn = certificate.clone();
    withdrawn
        .summaries
        .retain(|record| record.function != "even");
    assert!(check_legacy_safety_certificate(&withdrawn).is_err());

    // A relabelled record cannot enter under a different trust label either.
    let mut relabelled = certificate.clone();
    relabelled
        .summaries
        .iter_mut()
        .find(|record| record.function == "even")
        .unwrap()
        .trust = ContractTrust::Declared;
    assert!(check_legacy_safety_certificate(&relabelled).is_err());
}
