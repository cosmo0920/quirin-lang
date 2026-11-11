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

//! Design probe for card C03 — "neutral-frame recursion" (NFR), the candidate
//! recursive-summary profile specified in
//! `docs/design/W4_RECURSIVE_SUMMARY_PROFILES.md`.
//!
//! **This file changes no production behaviour and asks for none.** It pins
//! what the gateway does through the *declared-contract emulation*, so the
//! design document's feasibility and rejection arguments cite measurements
//! rather than expectations. The route is emulated by putting the candidate
//! contract in the declared environment: a declared contract is what the
//! recursive call site sees, and the defining body is then checked against that
//! same contract by the ordinary analyzer — exactly the assume-then-validate
//! step the route runs. The emulation is deliberately *more* permissive than
//! the route at one point, and
//! [`declared_emulation_does_not_reproduce_component_withdrawal`] pins that
//! difference instead of hiding it.
//!
//! Declared contracts are trusted assumptions. Nothing here reports a defect in
//! production code, promotes any verdict, or claims a proof.
//!
//! # Updated by card C03-NFR (the implemented route)
//!
//! The neutral-frame route specified by this card's design document is now
//! implemented in `crates/qcore-c-gateway/src/legacy_neutral.rs`, and the
//! implemented route's own matrix lives in
//! `crates/qcore-c-gateway/tests/neutral_frame_recursion.rs`. Three
//! expectations here were therefore updated deliberately rather than dropped:
//!
//! * the motivating gap — an undeclared neutral component and its callers both
//!   `Unknown` — is closed for neutral bodies, so that case now pins the
//!   *residual* gap, an undeclared component that carries an effect;
//! * the two emulation fixtures that used to read an `InferredValidated` record
//!   back out of a declared contract no longer do, because unit U7 withholds a
//!   record derived inside a recursive component one of whose members carries a
//!   declaration. The declaration still governs every call site, so the verdicts
//!   those fixtures measure are unchanged; only the published record is gone,
//!   which is exactly the presentational risk U7 names.
//!
//! Everything else in this file is byte-identical to the design-card baseline.
use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "void *q_alloc(void); void q_free(void *); void q_transfer(void *);";

/// The verdict of one probe run, reduced to what the design document cites.
struct Probe {
    entry: AnalysisStatus,
    functions: BTreeMap<String, AnalysisStatus>,
    diagnostics: BTreeMap<String, Vec<LegacyDiagnosticKind>>,
    /// Published `InferredValidated` summaries, `Safe` runs only.
    summaries: BTreeMap<String, String>,
}

impl Probe {
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
}

fn contracts(declared: Option<&str>) -> ResourceContracts {
    let mut sources = vec![(
        "q_base.qcore".to_owned(),
        include_str!("fixtures/contracts/q_base.qv").to_owned(),
    )];
    if let Some(declared) = declared {
        sources.push(("c03_probe.qcore".to_owned(), declared.to_owned()));
    }
    load_resource_contract_sources(&sources.into_iter().collect()).unwrap()
}

/// A candidate contract in the neutral shape: every formal `Borrowed`, no
/// result custody. This is the only assumption the proposed profile makes.
fn neutral(function: &str, parameters: &[(&str, &str)], result: &str) -> String {
    let signature = parameters
        .iter()
        .map(|(name, ty)| format!("{name}: {ty}"))
        .collect::<Vec<_>>()
        .join(", ");
    let borrowed = parameters
        .iter()
        .map(|(name, _)| format!("    ensures normal resource_borrowed({name});\n"))
        .collect::<String>();
    format!(
        "extern fn {function}({signature}) -> {result}\ncontract {{\n\
         \x20   requires resource_contract_v1;\n{borrowed}\
         \x20   ensures normal resource_return_none();\n}};\n"
    )
}

fn module(bodies: &[String]) -> String {
    format!("module contract;\n{}", bodies.concat())
}

fn run(program: &str, declared: Option<&str>) -> Probe {
    let source = format!("{DECLARATIONS}\n{program}");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "subject",
        &contracts(declared),
    )
    .unwrap_or_else(|error| panic!("probe fixture rejected by Clang: {error:?}\n{source}"));

    let (entry, summary, summaries) = match result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(&certificate)
                .expect("a Safe probe certificate must recheck");
            let records = certificate
                .summaries
                .iter()
                .map(|record| (record.function.clone(), format!("{:?}", record.contract)))
                .collect();
            (AnalysisStatus::Safe, certificate.summary, records)
        }
        UnrestrictedLegacyResult::Unsafe(summary) => {
            (AnalysisStatus::Unsafe, summary, BTreeMap::new())
        }
        UnrestrictedLegacyResult::Unknown { partial, .. } => (
            AnalysisStatus::Unknown,
            partial.expect("probe fixtures always project partially"),
            BTreeMap::new(),
        ),
    };

    Probe {
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
        summaries,
    }
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent C03 design probe: {reason}");
            false
        }
    }
}

const NEUTRAL_BORROW: &str = "void depth(void *p, int n) { if (n > 0) { depth(p, n - 1); } }";
const CALLER: &str = "void subject(void) { void *p = q_alloc(); depth(p, 3); q_free(p); }";

fn depth_neutral() -> String {
    module(&[neutral(
        "depth",
        &[("p", "ptr<mut, u8>"), ("n", "i32")],
        "unit",
    )])
}

/// The gap the profile addressed, and the part of it that remains.
///
/// When this card was written, *any* undeclared recursive component had no
/// summary and every caller of it was `Unknown` — not because the caller did
/// anything unsupported, but because the call was uncontracted. The implemented
/// neutral-frame route closes that for components whose bodies carry no custody
/// effect (pinned in `neutral_frame_recursion.rs`). What remains, and is pinned
/// here, is the same shape with an effect in it: the neutral candidate is
/// refused, no candidate above it is ever seeded, and the component and its
/// callers stay `Unknown`.
///
/// Card CG27-LOOPS-RECURSION-BUDGETS (gateway rule 11x) re-pinned the guard:
/// `if (n > 0)` now carries a signed threshold rank and its component is
/// published by that route. The unranked `if (n)` keeps the probe's shape.
#[test]
fn undeclared_neutral_recursion_leaves_the_component_and_its_callers_unknown() {
    if !clang_available() {
        return;
    }
    let effectful =
        "void depth(void *p, int n) { if (n) { depth(p, n - 1); return; } q_transfer(p); }";
    let probe = run(&format!("{effectful}\n{CALLER}"), None);
    assert_eq!(probe.entry, AnalysisStatus::Unknown);
    assert_eq!(probe.status("depth"), AnalysisStatus::Unknown);
    assert_eq!(
        probe.unsupported("depth"),
        vec!["call to uncontracted function `depth`".to_owned()]
    );
    assert_eq!(probe.status("subject"), AnalysisStatus::Unknown);
    // Card AOM-BINDING-POISON re-pinned the *mechanism* here, deliberately, and
    // not the verdict: `depth(p, 3)` passes `p` by value and cannot write that
    // slot, whose address `subject` never takes, so `p` survives the refusal
    // and `q_free(p)` resolves instead of reporting an unresolved alias. What
    // this probe is about is unchanged and re-asserted below: the caller stays
    // `Unknown` on the uncontracted call itself, and — because every path
    // through a refusal carries `unmodeled` — every exit of it still publishes
    // `ExitCustody::Unknown`, so no summary can be derived from it.
    assert!(!probe.has_unresolved_alias("subject"));
    assert_eq!(
        probe.unsupported("subject"),
        vec!["call to uncontracted function `depth`".to_owned()]
    );
}

/// Feasibility, positive direction. Under the neutral assumption every member
/// of the component validates with no formal bound, the published contract is
/// exactly the neutral one, and the caller becomes `Safe`. The recursion shape
/// is unconstrained: a self call with a decreasing scalar, a self call with no
/// decreasing argument at all, a mutual pair, and a body that acquires and
/// releases a local all validate, because the profile constrains *effects*,
/// not the control shape.
#[test]
fn assumed_neutral_components_validate_and_unblock_their_callers() {
    if !clang_available() {
        return;
    }
    let unranked = "void depth(void *p, int n) { if (n) { depth(p, n); } }";
    let local = "void depth(void *p, int n) { void *t = q_alloc(); if (n > 0) { depth(p, n - 1); } q_free(t); }";
    for body in [NEUTRAL_BORROW, unranked, local] {
        let probe = run(&format!("{body}\n{CALLER}"), Some(&depth_neutral()));
        assert_eq!(probe.entry, AnalysisStatus::Safe, "body: {body}");
        assert_eq!(probe.status("depth"), AnalysisStatus::Safe, "body: {body}");
        assert_eq!(
            probe.status("subject"),
            AnalysisStatus::Safe,
            "body: {body}"
        );
        // U7: the record is *withheld*, because a declaration of this same
        // recursive component is what the recursive call site read. The
        // declaration still governs, which is what the three verdicts above
        // measure.
        assert_eq!(probe.summaries.get("depth"), None, "body: {body}");
    }

    let mutual = "void even(void *p, int n); void odd(void *p, int n);\n\
        void even(void *p, int n) { if (n > 0) { odd(p, n - 1); } }\n\
        void odd(void *p, int n) { if (n > 0) { even(p, n - 1); } }";
    let declared = module(&[
        neutral("even", &[("p", "ptr<mut, u8>"), ("n", "i32")], "unit"),
        neutral("odd", &[("p", "ptr<mut, u8>"), ("n", "i32")], "unit"),
    ]);
    let probe = run(
        &format!("{mutual}\nvoid subject(void) {{ void *p = q_alloc(); even(p, 4); q_free(p); }}"),
        Some(&declared),
    );
    assert_eq!(probe.entry, AnalysisStatus::Safe);
    for member in ["even", "odd"] {
        assert_eq!(probe.status(member), AnalysisStatus::Safe);
        assert_eq!(probe.summaries.get(member), None);
    }
}

/// Feasibility, negative direction: the neutral assumption certifies the
/// *absence* of custody effects and can never certify one. Releasing a formal,
/// consuming a formal, leaking a local acquisition, storing a formal in a
/// global and returning a fresh resource each refuse the candidate — as an
/// unresolved identity, a leak, or a summary conflict.
#[test]
fn assumed_neutral_refuses_every_body_that_carries_an_effect() {
    if !clang_available() {
        return;
    }
    let releases =
        "void depth(void *p, int n) { if (n > 0) { depth(p, n - 1); return; } q_free(p); }";
    let consumes =
        "void depth(void *p, int n) { if (n > 0) { depth(p, n - 1); return; } q_transfer(p); }";
    for body in [releases, consumes] {
        let probe = run(&format!("{body}\n{CALLER}"), Some(&depth_neutral()));
        assert_eq!(
            probe.status("depth"),
            AnalysisStatus::Unknown,
            "body: {body}"
        );
        assert!(probe.has_unresolved_alias("depth"), "body: {body}");
        assert!(probe.summaries.is_empty(), "body: {body}");
    }

    let leaks =
        "void depth(void *p, int n) { void *t = q_alloc(); if (n > 0) { depth(p, n - 1); } }";
    let probe = run(&format!("{leaks}\n{CALLER}"), Some(&depth_neutral()));
    assert_eq!(probe.status("depth"), AnalysisStatus::Unsafe);
    assert!(matches!(
        probe.diagnostics("depth"),
        [LegacyDiagnosticKind::Leak { .. }]
    ));

    let global = "static void *g_slot;\n\
        void depth(void *p, int n) { if (n > 0) { depth(p, n - 1); return; } g_slot = p; }";
    let probe = run(&format!("{global}\n{CALLER}"), Some(&depth_neutral()));
    assert_eq!(probe.status("depth"), AnalysisStatus::Unknown);
    assert!(probe.has_unresolved_alias("depth"));
    // Card CTWIN-RESIDUALS moved this refusal *earlier* and left its
    // load-bearing half exactly where it was. Before that card, storing a
    // formal into a static-storage object was admitted, ordinary inference read
    // the formal as `Owned`, and the candidate was refused by the resulting
    // `SummaryConflict`. The store is now refused as a publication, so no
    // reading of `p` survives to conflict with anything — and the two
    // assertions that matter are unchanged above: the candidate is still
    // refused (`Unknown`, with an unresolved identity) and no summary is
    // published for it. Asserted here rather than dropped.
    assert!(
        probe.summaries.is_empty(),
        "an assumed-neutral candidate whose body publishes a formal must not be validated"
    );
    println!("{:?}", probe.diagnostics("depth"));

    let acquires = "void *make(void *p, int n) { if (n > 0) { void *r = make(p, n - 1); return r; } void *f = q_alloc(); return f; }";
    let declared = module(&[neutral(
        "make",
        &[("p", "ptr<mut, u8>"), ("n", "i32")],
        "ptr<mut, u8>",
    )]);
    let probe = run(
        &format!(
            "{acquires}\nvoid subject(void) {{ void *p = q_alloc(); void *q = make(p, 2); q_free(p); q_free(q); }}"
        ),
        Some(&declared),
    );
    assert_eq!(probe.status("make"), AnalysisStatus::Unknown);
    assert!(
        probe
            .diagnostics("make")
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::SummaryConflict { .. })),
        "an acquiring body cannot hide behind a neutral candidate"
    );
}

/// The alias bound the profile relies on. A neutral contract states no
/// precondition, so a caller may pass a null constant; and the value a neutral
/// callee returns carries no custody, so a caller that treats it as a resource
/// is fail-closed (`UnknownAlias`) rather than silently double-freeing. A
/// caller that ignores the handback stays `Safe`.
#[test]
fn a_neutral_contract_states_no_precondition_and_hands_back_no_custody() {
    if !clang_available() {
        return;
    }
    let probe = run(
        &format!(
            "{NEUTRAL_BORROW}\nvoid subject(void) {{ depth(0, 3); void *p = q_alloc(); q_free(p); }}"
        ),
        Some(&depth_neutral()),
    );
    assert_eq!(probe.entry, AnalysisStatus::Safe);

    let handback = "void *pick(void *p, int n) { if (n > 0) { void *r = pick(p, n - 1); return r; } return p; }";
    let declared = module(&[neutral(
        "pick",
        &[("p", "ptr<mut, u8>"), ("n", "i32")],
        "ptr<mut, u8>",
    )]);
    for caller in [
        "void subject(void) { void *p = q_alloc(); void *q = pick(p, 3); q_free(q); q_free(p); }",
        "void subject(void) { void *p = q_alloc(); void *q = pick(p, 3); q_free(q); }",
    ] {
        let probe = run(&format!("{handback}\n{caller}"), Some(&declared));
        assert_eq!(
            probe.status("pick"),
            AnalysisStatus::Safe,
            "caller: {caller}"
        );
        assert_eq!(
            probe.status("subject"),
            AnalysisStatus::Unknown,
            "caller: {caller}"
        );
        assert!(probe.has_unresolved_alias("subject"), "caller: {caller}");
    }
    let ignored = "void subject(void) { void *p = q_alloc(); void *q = pick(p, 3); q_free(p); }";
    let probe = run(&format!("{handback}\n{ignored}"), Some(&declared));
    assert_eq!(probe.entry, AnalysisStatus::Safe);
}

/// A neutral member also refuses to publish while it calls anything the
/// environment does not contract: the profile inherits the gateway's
/// fail-closed treatment of uncontracted calls and unmodelled accesses instead
/// of relaxing it.
#[test]
fn a_neutral_member_still_fails_closed_on_an_uncontracted_callee() {
    if !clang_available() {
        return;
    }
    let body = "void q_other(void *p);\n\
        void depth(void *p, int n) { if (n > 0) { depth(p, n - 1); return; } q_other(p); }";
    let probe = run(&format!("{body}\n{CALLER}"), Some(&depth_neutral()));
    assert_eq!(probe.status("depth"), AnalysisStatus::Unknown);
    assert_eq!(
        probe.unsupported("depth"),
        vec!["call to uncontracted function `q_other`".to_owned()]
    );
}

/// The counterexample that fixes the profile at the neutral element: an
/// *effectful* assumption validates vacuously whenever no exit is concretely
/// reachable. `purge` never returns, so "if it returns, `p` was released" holds
/// vacuously, the candidate rechecks against its own body, and the caller's
/// leak disappears. `mint` mints an owned result the same way. Both bodies are
/// refused by production today only because ordinary derivation never seeds a
/// component member's own contract — validation-by-reanalysis alone does not
/// refuse them. A profile that assumed effects would therefore need a rank, as
/// the two implemented routes have.
#[test]
fn an_assumed_effectful_contract_validates_vacuously_under_divergence() {
    if !clang_available() {
        return;
    }
    let released = module(&[
        "extern fn purge(p: ptr<mut, u8>, n: i32) -> unit\ncontract {\n\
            \x20   requires resource_contract_v1;\n\
            \x20   ensures normal resource_released(p, nonnull);\n\
            \x20   ensures normal resource_borrowed(n);\n\
            \x20   ensures normal resource_return_none();\n};\n"
            .to_owned(),
    ]);
    let guarded_divergence =
        "void purge(void *p, int n) { if (n > 0) { purge(p, n - 1); return; } purge(p, 1); }";
    let probe = run(
        &format!(
            "{guarded_divergence}\nvoid subject(void) {{ void *p = q_alloc(); purge(p, 2); }}"
        ),
        Some(&released),
    );
    assert_eq!(probe.entry, AnalysisStatus::Safe);
    assert_eq!(
        probe.status("purge"),
        AnalysisStatus::Safe,
        "the vacuous release still validates against its own declaration"
    );
    // Before card C03-NFR this fixture also *published* the vacuous release as
    // an `InferredValidated` record. Unit U7 withholds it: a record derived
    // inside a recursive component that carries a declaration is the
    // declaration read back, not corroboration of it. The vacuity itself is
    // untouched — the caller's leak still disappears under the trusted
    // declaration, which is the measurement this test exists for.
    assert_eq!(probe.summaries.get("purge"), None);

    let mint = "void *mint(int n) { void *r = mint(n); return r; }";
    let owned = module(&["extern fn mint(n: i32) -> ptr<mut, u8>\ncontract {\n\
            \x20   requires resource_contract_v1;\n\
            \x20   ensures normal resource_borrowed(n);\n\
            \x20   ensures normal resource_return_owned(nonnull);\n};\n"
        .to_owned()]);
    let probe = run(
        &format!("{mint}\nvoid subject(void) {{ void *p = mint(1); q_free(p); }}"),
        Some(&owned),
    );
    assert_eq!(probe.entry, AnalysisStatus::Safe);

    // The same candidate over a body with one concretely reachable base exit
    // that does not perform the effect is refused, so the vacuity above is a
    // divergence artefact and not a gap in the exit comparison itself.
    let real_base_exit = "void purge(void *p, int n) { if (n > 0) { purge(p, n - 1); return; } }";
    let probe = run(
        &format!("{real_base_exit}\nvoid subject(void) {{ void *p = q_alloc(); purge(p, 2); }}"),
        Some(&released),
    );
    assert_eq!(probe.status("purge"), AnalysisStatus::Unknown);
    assert!(probe.has_unresolved_alias("purge"));
}

/// Where the profile's corpus value is capped today: recursion that walks a
/// pointer field is refused before any recursion rule is consulted. Both
/// blockers are independent; the design document records this as the dependency
/// ordering for the follow-up units.
///
/// **Pin updated by card O04.** When C03 recorded this, the statement-level
/// indirect-access gate decided the case and the expectation named its
/// diagnostic. O04 implements C02-P, so the *call* statement `seek(n->next,
/// key)` is now a modeled borrowed actual — but the branch condition
/// `if (n->key == key)` still is not, and branch conditions are out of scope
/// for both C02 (§7) and O04. The load-bearing assertion is unchanged:
/// `seek` stays `Unknown`, refused before any recursion rule applies. Only the
/// identity of the gate that refuses it moved, and the expectation was updated
/// deliberately rather than dropped.
#[test]
fn a_pointer_field_search_certifies_once_its_branch_read_is_modeled() {
    if !clang_available() {
        return;
    }
    let chain = "struct node { struct node *next; int key; };\n\
        int seek(struct node *n, int key) { if (!n) { return 0; } if (n->key == key) { return 1; } int r = seek(n->next, key); return r; }";
    let declared = module(&[neutral(
        "seek",
        &[("n", "ptr<mut, u8>"), ("key", "i32")],
        "i32",
    )]);
    let probe = run(
        &format!("{chain}\nvoid subject(void) {{ void *p = q_alloc(); seek(p, 1); q_free(p); }}"),
        Some(&declared),
    );
    // Deliberate pin update, card W43-INDEX-TRAVERSAL. The branch condition
    // `n->key == key` is now the typed read of a scalar member through `n`,
    // whose dereference obligation holds (an unbound formal, after its null
    // test). That was the only blocker: the recursive call was already a
    // modeled borrowed actual, so the search now certifies against its
    // declared neutral contract. This is the dependency ordering the comment
    // above and `docs/design/W4_RECURSIVE_SUMMARY_PROFILES.md` §6.4 record:
    // the pointer-field read was the gate, and the recursion rule never
    // refused it. The heap-link teardown stays refused (see
    // `neutral_frame_recursion.rs`).
    assert_eq!(probe.status("seek"), AnalysisStatus::Safe);
    assert!(probe.unsupported("seek").is_empty());
}

/// The one place the declared-contract emulation is more permissive than the
/// proposed route: a declaration survives its own member's refusal, so `even`
/// stays `Safe` here. The route has no such declaration — withdrawing `odd`'s
/// candidate makes `even`'s call uncontracted — which is why the design fixes
/// publication at the component level rather than the member level.
#[test]
fn declared_emulation_does_not_reproduce_component_withdrawal() {
    if !clang_available() {
        return;
    }
    let mixed = "void even(void *p, int n); void odd(void *p, int n);\n\
        void even(void *p, int n) { if (n > 0) { odd(p, n - 1); } }\n\
        void odd(void *p, int n) { if (n > 0) { even(p, n - 1); return; } q_free(p); }";
    let declared = module(&[
        neutral("even", &[("p", "ptr<mut, u8>"), ("n", "i32")], "unit"),
        neutral("odd", &[("p", "ptr<mut, u8>"), ("n", "i32")], "unit"),
    ]);
    let probe = run(
        &format!("{mixed}\nvoid subject(void) {{ void *p = q_alloc(); even(p, 4); q_free(p); }}"),
        Some(&declared),
    );
    assert_eq!(probe.status("odd"), AnalysisStatus::Unknown);
    assert!(probe.has_unresolved_alias("odd"));
    assert_eq!(
        probe.status("even"),
        AnalysisStatus::Safe,
        "emulation artefact: the declaration outlives the refused member"
    );
}
