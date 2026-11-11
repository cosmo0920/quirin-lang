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

//! Typed direct scalar-field access classification (card C02,
//! `docs/reports/remaining-c02/README.md`).
//!
//! A held-out generic C fixture family — a text-editor-shaped `struct doc` and
//! its neighbours, deliberately unlike the CMetrics unit the capability was
//! selected from — exercised through the public gateway entry points only.
//!
//! The cases are the card's three acceptance classes:
//!
//! * **positive** — an owned lifetime whose scalar-field stores and reads no
//!   longer poison the enclosing function, so the resource analysis completes;
//! * **negative** — a real leak and a real invalid transfer that the analysis
//!   previously hid behind an `unsupported` construct and now *reports*;
//! * **ambiguous** — union, `volatile`, pointer-field, call-bearing,
//!   address-of-escape, chained, subscripted and cast accesses, each of which
//!   keeps its existing `Unknown` verdict and its existing diagnostic.
//!
//! Plus the independence requirement: a certificate resting on the
//! classification is re-derived by [`check_legacy_safety_certificate`], which
//! repeats the typed-access decision from the preprocessed source alone.
//!
//! No verdict is promoted here. `Safe` in these fixtures is `Safe` for the
//! declared `q_*` custody projection over synthetic test assumptions, not a
//! claim about any real library.

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    LegacySafetyCertificate, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const Q_BASE: (&str, &str) = ("q_base.qcore", include_str!("fixtures/contracts/q_base.qv"));

/// The reason a certificate whose custody summaries cannot be re-derived from
/// the preprocessed source is refused.
const SUMMARY_MISMATCH: &str = "inferred custody summaries do not match the preprocessed source";
/// The existing rejection every ambiguous access must keep.
const INDIRECT: &str = "indirect memory access is outside the resource projection";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&std::collections::BTreeMap::from([(
        Q_BASE.0.to_owned(),
        Q_BASE.1.to_owned(),
    )]))
    .unwrap()
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent typed-field-access test: {reason}");
            false
        }
    }
}

/// The held-out fixture family. Records and fields are deliberately unrelated
/// to the CMetrics unit that motivated the capability; every rule under test is
/// stated over declared C types, so no name here can matter.
const DECLARATIONS: &str = "void *q_alloc(void); void q_free(void *); void q_transfer(void *); \
                            int q_width(void);";

const RECORDS: &str = "\
struct doc { int cursor; int dirty; long revision; struct pane *pane; }; \
struct pane { int rows; struct doc *doc; }; \
struct volatile_doc { volatile int cursor; int dirty; }; \
union overlay { int cursor; void *handle; }; \
struct bitfield_doc { unsigned cursor : 4; int dirty; }; \
struct nested_doc { int cursor; union { int tag; void *handle; } view; }; \
struct callback_doc { int cursor; void (*on_change)(int); }; \
struct array_doc { int cursor; char name[8]; };";

fn source(body: &str) -> String {
    format!("{DECLARATIONS}\n{RECORDS}\n{body}")
}

fn analyze(body: &str) -> UnrestrictedLegacyResult {
    let text = source(body);
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected by Clang: {error:?}\n{text}"))
}

#[track_caller]
fn certified(body: &str) -> LegacySafetyCertificate {
    let result = analyze(body);
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("expected a completed resource analysis, got: {result:?}");
    };
    check_legacy_safety_certificate(&certificate).unwrap();
    certificate
}

#[track_caller]
fn unsafe_summary(body: &str) -> LegacyProgramSummary {
    match analyze(body) {
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        other => panic!("expected a reported violation, got: {other:?}"),
    }
}

#[track_caller]
fn unknown_reasons(body: &str) -> Vec<String> {
    match analyze(body) {
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary
            .functions
            .values()
            .flat_map(|function| &function.diagnostics)
            .filter_map(|diagnostic| match &diagnostic.kind {
                LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
                _ => None,
            })
            .collect(),
        other => panic!("expected an Unknown projection, got: {other:?}"),
    }
}

fn diagnostic_kinds(summary: &LegacyProgramSummary) -> Vec<LegacyDiagnosticKind> {
    summary
        .functions
        .values()
        .flat_map(|function| &function.diagnostics)
        .map(|diagnostic| diagnostic.kind.clone())
        .collect()
}

// --- Acceptance 1a: the positive case --------------------------------------

#[test]
fn scalar_field_stores_and_reads_no_longer_poison_an_owned_lifetime() {
    // Every one of these statements was rejected by the lexical gate before
    // this card, which left the whole function `Unknown` and its `q_alloc`
    // obligation unexamined. Their declared types make them custody-neutral,
    // so the analysis now runs to completion and the release is checked.
    if !clang_available() {
        return;
    }
    let certificate = certified(
        "void subject(struct doc *d) { \
             void *p = q_alloc(); \
             d->cursor = 1; \
             d->revision = d->revision + 1; \
             d->dirty = d->cursor; \
             q_free(p); \
         }",
    );
    assert_eq!(certificate.summary.status, AnalysisStatus::Safe);
    let subject = &certificate.summary.functions["subject"];
    assert!(
        subject.diagnostics.is_empty(),
        "a custody-neutral scalar access must leave no diagnostic: {:?}",
        subject.diagnostics
    );
}

#[test]
fn a_record_valued_local_and_a_compound_assignment_are_also_admitted() {
    if !clang_available() {
        return;
    }
    let certificate = certified(
        "void subject(void) { \
             struct doc local; \
             void *p = q_alloc(); \
             local.cursor = 0; \
             local.cursor += 2; \
             local.dirty = local.cursor; \
             q_free(p); \
         }",
    );
    assert_eq!(certificate.summary.status, AnalysisStatus::Safe);
}

// --- Acceptance 1b: the negative cases, which must now be reported ----------

#[test]
fn a_leak_that_hid_behind_a_scalar_field_store_is_now_reported() {
    // Before this card the store poisoned the state, so the missing release
    // was invisible and the function was `Unknown` — a withdrawn result, not
    // an absence of a violation. It is now a reported leak.
    if !clang_available() {
        return;
    }
    let summary = unsafe_summary(
        "void subject(struct doc *d) { \
             void *p = q_alloc(); \
             d->cursor = 1; \
         }",
    );
    let kinds = diagnostic_kinds(&summary);
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. })),
        "expected the surfaced leak, got: {kinds:?}"
    );
    assert!(
        !kinds
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::Unsupported { .. })),
        "the classified access must no longer be reported as unsupported: {kinds:?}"
    );
}

#[test]
fn a_double_release_that_hid_behind_a_scalar_field_store_is_now_reported() {
    if !clang_available() {
        return;
    }
    let summary = unsafe_summary(
        "void subject(struct doc *d) { \
             void *p = q_alloc(); \
             q_free(p); \
             d->dirty = 1; \
             q_free(p); \
         }",
    );
    let kinds = diagnostic_kinds(&summary);
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::DoubleRelease { .. })),
        "expected the surfaced double release, got: {kinds:?}"
    );
}

#[test]
fn a_release_after_transfer_that_hid_behind_a_scalar_field_read_is_now_reported() {
    if !clang_available() {
        return;
    }
    let summary = unsafe_summary(
        "void subject(struct doc *d) { \
             void *p = q_alloc(); \
             q_transfer(p); \
             d->cursor = d->revision; \
             q_free(p); \
         }",
    );
    let kinds = diagnostic_kinds(&summary);
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::ReleaseAfterEscape { .. })),
        "expected the surfaced release after escape, got: {kinds:?}"
    );
}

#[test]
fn a_scalar_struct_field_stays_independent_of_a_distinct_union_member() {
    if !clang_available() {
        return;
    }
    let certificate = certified(
        "void subject(struct nested_doc *d) { void *p = q_alloc(); d->cursor = 1; q_free(p); }",
    );
    check_legacy_safety_certificate(&certificate).unwrap();
    let certificate = certified(
        "void subject(struct bitfield_doc *d) { void *p = q_alloc(); d->dirty = 1; q_free(p); }",
    );
    check_legacy_safety_certificate(&certificate).unwrap();
    let certificate = certified(
        "void subject(struct callback_doc *d) { void *p = q_alloc(); d->cursor = 1; q_free(p); }",
    );
    check_legacy_safety_certificate(&certificate).unwrap();
    let certificate = certified(
        "void subject(struct array_doc *d) { void *p = q_alloc(); d->name[0] = 65; q_free(p); }",
    );
    check_legacy_safety_certificate(&certificate).unwrap();
}

// --- Acceptance 1c: everything ambiguous keeps its existing Unknown ---------

#[test]
fn ambiguous_accesses_keep_their_existing_unknown_and_diagnostic() {
    if !clang_available() {
        return;
    }
    // Each body differs from the admitted positive case in exactly one typed
    // or structural premise of the whitelist.
    let cases: &[(&str, &str)] = &[
        // A pointer-typed field is not custody-neutral: it can hold, alias or
        // drop a tracked identity. This is C02-P's territory, not C02's.
        (
            "pointer field",
            "void subject(struct doc *d, struct pane *n) { void *p = q_alloc(); \
             d->pane = n; q_free(p); }",
        ),
        // A distinct scalar field on a callback record certifies above;
        // pointer-valued stores remain refused by the pointer-field control.
        // `volatile` carries side effects the projection does not model.
        (
            "volatile field",
            "void subject(struct volatile_doc *d) { void *p = q_alloc(); \
             d->cursor = 1; q_free(p); }",
        ),
        // A scalar element of an array member is a typed indexed store,
        // covered by the positive control above.
        // An address escapes a sub-object the projection cannot name.
        (
            "address-of escape",
            "void subject(struct doc *d, int **o) { void *p = q_alloc(); \
             *o = &d->cursor; q_free(p); }",
        ),
        // Deliberate pin update, card W44-INDIRECT-CHAIN-ACCESS: the row
        // `n->doc->cursor = 1` over a never-bound formal moved out of this
        // table. Its link `doc` is exactly `struct doc *` and its final member
        // a scalar, so the typed route now admits it with each link's
        // obligation; it certifies in
        // `a_typed_member_chain_store_is_admitted_by_the_chain_route` below.
        // Card CG08-CASTS-AND-TYPE-QUERIES (rule 11b) moves the "cast
        // operand" row (`d->revision = (long) h;`) out of this table
        // deliberately: Clang's `PointerToIntegral` of a formal the closure
        // proves never bound is data, so the elaborator stores it as a typed
        // scalar store; the same cast of a tracked pointer is refused below.
        // A call in the statement keeps the call's own rules.
        (
            "call-bearing statement",
            "void subject(struct doc *d) { void *p = q_alloc(); \
             d->cursor = q_width(); q_free(p); }",
        ),
    ];
    // Card CG13-UNION-AND-BITFIELD-PLACES moves the "union base" and
    // "bitfield member" rows out of this table deliberately (rule 10w): a
    // scalar store through a member of a union that can overlap a pointer is
    // admitted with the refused-write frame, and a bit-field store is a
    // read-modify-write of its storage unit. BEFORE: both refused.
    for body in [
        "void subject(union overlay *o) { void *p = q_alloc(); o->cursor = 1; q_free(p); }",
        "void subject(struct bitfield_doc *d) { void *p = q_alloc(); d->cursor = 1; q_free(p); }",
        "void subject(struct doc *d, void *h) { void *p = q_alloc(); \
         d->revision = (long) h; q_free(p); }",
    ] {
        certified(body);
    }
    // Card CG08: the tracked pointer's conversion is an escape, refused.
    let reasons = unknown_reasons(
        "void subject(struct doc *d) { void *p = q_alloc(); \
         d->revision = (long) p; q_free(p); }",
    );
    assert!(
        !reasons.is_empty(),
        "a tracked pointer's integer must refuse"
    );
    for (case, body) in cases {
        let reasons = unknown_reasons(body);
        assert!(
            !reasons.is_empty(),
            "{case}: an ambiguous access must retain an unsupported diagnostic"
        );
        // Card MEMBER-EFFECT-SUMMARY (rule 38i) re-pins this deliberately: the pointer field's store into a formal's
        // member names its premise; still refused.
        let expected = if *case == "pointer field" {
            "store of a custody-carrying value into `d->pane`, a member of the formal `d` \
             (member-store-through-formal)"
        } else {
            INDIRECT
        };
        assert!(
            reasons.iter().any(|reason| reason == expected),
            "{case}: expected the existing indirect-access diagnostic, got {reasons:?}"
        );
    }
}

/// Card W43-INDIRECT-ACCESS-GENERAL moved one case out of the table above,
/// deliberately: `d->cursor = v[0]` over two unbound formals is a typed scalar
/// access statement (`v` is declared `int *`, `cursor` a scalar member), so it
/// is now admitted and certifies. The certificate rechecks from source.
#[test]
fn a_subscripted_scalar_read_into_a_scalar_member_is_admitted_by_the_general_route() {
    if !clang_available() {
        return;
    }
    certified(
        "void subject(struct doc *d, int *v) { void *p = q_alloc(); \
         d->cursor = v[0]; q_free(p); }",
    );
}

/// Card W44-INDIRECT-CHAIN-ACCESS moved the `chained access` row out of the
/// ambiguous table, deliberately: `n->doc->cursor = 1` reads through a
/// never-bound formal `n` (untracked) and the link `n->doc`, a member of an
/// object no reaching path bound (untracked), and stores a scalar member. It
/// is admitted by the chain route and certifies; the certificate rechecks
/// from source.
#[test]
fn a_typed_member_chain_store_is_admitted_by_the_chain_route() {
    if !clang_available() {
        return;
    }
    certified(
        "void subject(struct pane *n) { void *p = q_alloc(); n->doc->cursor = 1; q_free(p); }",
    );
}

#[test]
fn an_unreadable_declaration_refuses_every_access_in_the_same_function() {
    // The reader's completeness premise: it refuses the whole function rather
    // than let a declaration it could not read shadow one it could.
    if !clang_available() {
        return;
    }
    let reasons = unknown_reasons(
        "void subject(struct doc *d) { \
             void (*cb)(int) = 0; \
             void *p = q_alloc(); \
             d->cursor = 1; \
             q_free(p); \
         }",
    );
    assert!(
        reasons.iter().any(|reason| reason == INDIRECT),
        "expected the whole function to keep the lexical gate, got {reasons:?}"
    );
}

#[test]
fn a_shadowing_redeclaration_of_a_different_record_refuses_the_name() {
    if !clang_available() {
        return;
    }
    let reasons = unknown_reasons(
        "void subject(struct doc *d) { \
             void *p = q_alloc(); \
             { struct volatile_doc *d; d->cursor = 1; } \
             q_free(p); \
         }",
    );
    assert!(
        reasons.iter().any(|reason| reason == INDIRECT),
        "expected the shadowed name to be unusable, got {reasons:?}"
    );
}

// --- Acceptance 2: the classification is repeated independently -------------

#[test]
fn a_certificate_resting_on_the_classification_is_rederived_from_source() {
    // `check_legacy_safety_certificate` re-analyzes the preprocessed source,
    // which repeats the record reading, the variable typing and the statement
    // classification. Substituting a record definition so that the same
    // statement is no longer custody-neutral must be refused, not accepted on
    // the certificate's stored summary.
    if !clang_available() {
        return;
    }
    let mut certificate = certified(
        "void subject(struct doc *d) { \
             void *p = q_alloc(); \
             d->cursor = 1; \
             q_free(p); \
         }",
    );
    let original = certificate.preprocessed_source.clone();

    // Each substitution changes only the *declaration* the classification
    // rests on; the statement is byte-identical. Each rejection reason was
    // measured before being pinned, so a case cannot pass by failing for an
    // unrelated cause. All three land on the summary-containment gate, which
    // runs before the program-summary gate: re-deriving `subject`'s own
    // custody summary already repeats the typed-access decision.
    let cases: &[(&str, &str, &str)] = &[
        (
            "the accessed field becomes a pointer",
            "struct doc { int cursor; int dirty;",
            "struct doc { void *cursor; int dirty;",
        ),
        (
            "the accessed record becomes a union",
            "struct doc { int cursor;",
            "union doc { int cursor;",
        ),
        (
            "the accessed field becomes volatile",
            "struct doc { int cursor;",
            "struct doc { volatile int cursor;",
        ),
        (
            "the record gains a bitfield the reader refuses",
            "struct doc { int cursor; int dirty;",
            "struct doc { int cursor; unsigned flags : 3; int dirty;",
        ),
    ];
    for (case, from, to) in cases {
        let substituted = original.replace(from, to);
        assert_ne!(substituted, original, "{case}: substitution did not apply");
        certificate.preprocessed_source = if *case == "the accessed record becomes a union" {
            substituted.replace("struct doc *d", "union doc *d")
        } else {
            substituted
        };
        let result = check_legacy_safety_certificate(&certificate);
        assert!(
            result == Err(SUMMARY_MISMATCH.to_owned())
                // Card CG13 (rule 10w): the union version is now read too (a
                // framed store), so its re-derived program differs.
                || (matches!(
                    *case,
                    "the record gains a bitfield the reader refuses"
                        | "the accessed record becomes a union"
                ) && result
                        == Err("program summary does not match the preprocessed source".to_owned())),
            "{case}: the certificate must not survive the substitution: {result:?}"
        );
    }

    // The unmodified certificate still passes, so the three refusals above are
    // caused by the substitution and not by an unrelated failure.
    certificate.preprocessed_source = original;
    check_legacy_safety_certificate(&certificate).unwrap();
}

#[test]
fn a_custody_summary_derived_through_a_classified_body_is_validated_and_rederived() {
    // W4 publishes a summary only for a callee whose body re-analyzes clean.
    // `release` is such a callee only because its scalar-field store is
    // classified; the caller's obligation is then discharged through that
    // validated summary, and the recheck re-derives both independently.
    if !clang_available() {
        return;
    }
    let certificate = certified(
        "void release(struct doc *d, void *p) { d->dirty = 0; q_free(p); } \
         void subject(struct doc *d) { void *p = q_alloc(); release(d, p); }",
    );
    let record = certificate
        .summaries
        .iter()
        .find(|record| record.function == "release")
        .expect("expected a validated custody summary for the classified wrapper");
    assert_eq!(
        record.trust,
        qcore_c_gateway::legacy_c::ContractTrust::InferredValidated
    );
    // The entry requirement and the per-exit disposition are the summary's
    // content: the second formal is released on every exit, the first is not.
    assert_eq!(
        record.contract.parameters.len(),
        2,
        "the wrapper's formal arity must survive classification: {:?}",
        record.contract
    );
    assert!(
        matches!(
            record.contract.parameters[1],
            qcore_c_gateway::legacy_c::ResourceParameterMode::Released { .. }
        ),
        "expected the released formal, got {:?}",
        record.contract.parameters
    );
    assert!(
        matches!(
            record.contract.parameters[0],
            qcore_c_gateway::legacy_c::ResourceParameterMode::Borrowed
        ),
        "the record formal must stay borrowed, got {:?}",
        record.contract.parameters
    );
    check_legacy_safety_certificate(&certificate).unwrap();
}

#[test]
fn a_classified_wrapper_that_stops_releasing_loses_its_caller_certificate() {
    // The complement of the previous case: if the classified callee no longer
    // discharges, the caller is a reported leak rather than a retained `Safe`.
    if !clang_available() {
        return;
    }
    let summary = unsafe_summary(
        "void release(struct doc *d, void *p) { d->dirty = 0; } \
         void subject(struct doc *d) { void *p = q_alloc(); release(d, p); }",
    );
    let kinds = diagnostic_kinds(&summary);
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. })),
        "expected the caller's leak to be reported, got: {kinds:?}"
    );
}
