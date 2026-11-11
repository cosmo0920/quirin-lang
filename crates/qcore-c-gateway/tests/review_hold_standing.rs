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

//! Card W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL — a call held for a human
//! act (a declaration awaiting review, C06.1, or withheld for a pin mismatch,
//! C06.2) never masks a reason that would stand without the hold
//! (`docs/reports/w45-review-hold-preempts-statement-refusal/README.md`).
//!
//! Before the card the held call's `Pending` refusal replaced its statement's
//! other refusal and the held call had no effect, so a function whose accepted
//! declaration would still leave it `Unknown` (or alleging a violation) was
//! published `Pending`, below its real standing. A `Pending` row is now
//! joined with every reason its function carries with the held declarations
//! in force; only a row whose sole non-`Safe` reason is the hold stays
//! `Pending`.
//!
//! **The family is held out.** A `k_*` API unrelated to every production
//! library, declared inline: no fixture file and no production contract moves.
//! `k_input_new` has the shape W47-LOOP-OWNER-INSTANCE-FLOW reported
//! (`flb_input` over a review-held `flb_input_new`), with an actual the
//! projection refuses even under the declaration; the real `flb_input` is
//! `Safe` with its declaration accepted, so it stays `Pending` (report §6).
//! Every `Safe` below is `Safe` for a synthetic declared custody
//! projection under the premises the fixture names, and every `Unsafe` is
//! provisional.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::review_standing::{STANDING_PREFIX, is_standing_reason};
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyProgramSummary, ResourceContracts, UnrestrictedLegacyResult, analyze_legacy_c,
    analyze_legacy_c_translation_unit, analyze_unrestricted_c11, held_call, pin_hold,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract_revision::PinMismatchReason;
use qcore_schemas::verdict::Verdict;

const BARE: &str = "module contract;

extern fn k_open() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nullable);
};

extern fn k_close(h: ptr<mut, u8>, n: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(h, nonnull);
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};

extern fn k_input_new(config: ptr<mut, u8>, input: ptr<const, u8>, data: ptr<mut, u8>, public_only: i32) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(config);
    ensures normal resource_borrowed(input);
    ensures normal resource_borrowed(data);
    ensures normal resource_borrowed(public_only);
    ensures normal resource_return_none();
};
";

const PRELUDE: &str = "void *k_open(void);\nvoid k_close(void *h, int n);\n\
     struct k_config;\nstruct k_ctx { struct k_config *config; struct k_ctx *next; };\n\
     void *k_input_new(struct k_config *config, const char *input, void *data, int public_only);\n";

/// Open, test, close: `Safe` under the bare contracts.
const BALANCED: &str = "void w(void) { void *h; h = k_open(); if (!h) { return; } k_close(h, 0); }";
/// Close twice: a provisional double release under the bare contracts.
const TWICE: &str =
    "void w(void) { void *h; h = k_open(); if (!h) { return; } k_close(h, 0); k_close(h, 0); }";
/// Open and forget: a provisional leak under the bare contracts.
const LEAKED: &str = "void w(void) { void *h; h = k_open(); if (!h) { return; } }";
/// The observed shape: the held call's own statement reads its first actual
/// through a pointer the projection refuses (`(*ctx)->config`; the observed
/// `ctx->config` refuses there as `indirect memory access is outside the
/// resource projection`, which a complete fixture type would admit).
const ARGUMENT_ACCESS: &str = "int w(struct k_ctx **ctx, const char *input, void *data) \
     { void *i; i = k_input_new((*ctx)->config, input, data, 1); if (!i) { return -1; } return 0; }";
/// The same call with a plain formal: nothing but the hold is open.
const PLAIN_ACTUAL: &str = "int w(struct k_config *config, const char *input, void *data) \
     { void *i; i = k_input_new(config, input, data, 1); if (!i) { return -1; } return 0; }";
/// A refusal later on the path, which the held analysis records itself.
const LATER_REFUSAL: &str = "int w(struct k_config *config, const char *input, void *data, struct k_ctx *x) \
     { void *i; i = k_input_new(config, input, data, 1); x->next->next = x; if (!i) { return -1; } return 0; }";

fn load(text: &str) -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([("k.qcore".to_owned(), text.to_owned())]))
        .expect("the fixture module loads")
}

/// `BARE` with `callee`'s whole contract awaiting review.
fn deferred(callee: &str) -> ResourceContracts {
    let marker = format!("extern fn {callee}(");
    let start = BARE.find(&marker).expect("the callee is declared");
    let requires = BARE[start..]
        .find("    requires resource_contract_v1;\n")
        .map(|offset| start + offset + "    requires resource_contract_v1;\n".len())
        .expect("the contract opens");
    let mut text = BARE.to_owned();
    text.insert_str(
        requires,
        "    requires resource_review_pending(\"under review\");\n",
    );
    let contracts = load(&text);
    assert!(contracts.pending_review().contains_key(callee), "{callee}");
    contracts
}

/// The bare environment with `callee` withheld for a pin mismatch.
fn pin_withheld(callee: &str) -> ResourceContracts {
    let mut contracts = load(BARE);
    contracts
        .withhold_for_pin_mismatch(
            callee,
            "k.qcore".into(),
            PinMismatchReason::SourceChanged,
            Some("contracts.lock".into()),
            "the audited source's sites differ from the lock".into(),
        )
        .unwrap();
    contracts
}

fn analyse(body: &str, contracts: &ResourceContracts) -> LegacyProgramSummary {
    analyze_legacy_c_translation_unit(&format!("{PRELUDE}{body}"), "w", contracts)
        .expect("the fixture tokenizes")
}

fn w(summary: &LegacyProgramSummary) -> &LegacyFunctionSummary {
    summary.functions.get("w").expect("w is analysed")
}

fn descriptions(row: &LegacyFunctionSummary) -> Vec<String> {
    row.diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
            _ => None,
        })
        .collect()
}

fn standing(row: &LegacyFunctionSummary) -> Vec<String> {
    descriptions(row)
        .into_iter()
        .filter(|description| is_standing_reason(description))
        .collect()
}

fn held(row: &LegacyFunctionSummary) -> Vec<String> {
    descriptions(row)
        .iter()
        .filter_map(|description| {
            held_call(description)
                .or_else(|| pin_hold::held_call(description))
                .map(str::to_owned)
        })
        .collect()
}

fn no_violation(row: &LegacyFunctionSummary) -> bool {
    row.diagnostics.iter().all(|diagnostic| {
        matches!(
            diagnostic.kind,
            LegacyDiagnosticKind::Unsupported { .. } | LegacyDiagnosticKind::UnknownAlias { .. }
        )
    })
}

// ---------------------------------------------------------------------------
// Negative controls: the reasons that stand with the declaration in force.
// ---------------------------------------------------------------------------

#[test]
fn controls_with_every_declaration_in_force() {
    let bare = load(BARE);
    assert_eq!(w(&analyse(BALANCED, &bare)).status, AnalysisStatus::Safe);
    assert_eq!(
        w(&analyse(PLAIN_ACTUAL, &bare)).status,
        AnalysisStatus::Safe
    );
    assert_eq!(w(&analyse(TWICE, &bare)).status, AnalysisStatus::Unsafe);
    assert_eq!(w(&analyse(LEAKED, &bare)).status, AnalysisStatus::Unsafe);
    // The argument access and the later store refuse on their own.
    for body in [ARGUMENT_ACCESS, LATER_REFUSAL] {
        let row = analyse(body, &bare);
        assert_eq!(w(&row).status, AnalysisStatus::Unknown, "{body}");
        assert!(standing(w(&row)).is_empty());
    }
}

// ---------------------------------------------------------------------------
// The defect's shapes: each was `Pending` before the card.
// ---------------------------------------------------------------------------

#[test]
fn a_held_call_does_not_mask_its_statements_argument_refusal() {
    for contracts in [deferred("k_input_new"), pin_withheld("k_input_new")] {
        let summary = analyse(ARGUMENT_ACCESS, &contracts);
        let row = w(&summary);
        assert_eq!(row.status, AnalysisStatus::Unknown, "{:?}", row.diagnostics);
        assert_eq!(summary.status, AnalysisStatus::Unknown);
        // The held call is still named, and the refusal the bare environment
        // reports stands beside it.
        assert_eq!(held(row), ["k_input_new"]);
        let bare = analyse(ARGUMENT_ACCESS, &load(BARE));
        let expected = descriptions(w(&bare));
        assert!(!expected.is_empty());
        for description in expected {
            assert!(
                standing(row)
                    .iter()
                    .any(|standing| standing.ends_with(&description)),
                "{description} missing from {:?}",
                row.diagnostics
            );
        }
        assert!(no_violation(row));
    }
}

/// Control: a refusal of a later statement is recorded beside the held call
/// by the held analysis itself (the refused statement's writes are havocked
/// and the path goes on), so the row was `Unknown` before the card and needs
/// no standing reason. What the hold masked is its own statement's refusal
/// and what the held declaration's effect would make of the path.
#[test]
fn a_later_refusal_on_the_path_is_recorded_by_the_held_analysis() {
    for contracts in [deferred("k_input_new"), pin_withheld("k_input_new")] {
        let row = analyse(LATER_REFUSAL, &contracts);
        assert_eq!(
            w(&row).status,
            AnalysisStatus::Unknown,
            "{:?}",
            w(&row).diagnostics
        );
        assert!(standing(w(&row)).is_empty(), "{:?}", w(&row).diagnostics);
        assert_eq!(held(w(&row)), ["k_input_new"]);
    }
}

/// A violation the held declaration would allege once accepted keeps the row
/// `Unknown`, never `Pending` and never `Unsafe`: the declaration accuses
/// nothing while it is held, and accepting it would not make the row `Safe`.
#[test]
fn a_violation_the_accepted_declaration_would_allege_keeps_the_row_unknown() {
    for (body, callee) in [(TWICE, "k_close"), (LEAKED, "k_open")] {
        for contracts in [deferred(callee), pin_withheld(callee)] {
            let row = analyse(body, &contracts);
            let row = w(&row);
            assert_eq!(
                row.status,
                AnalysisStatus::Unknown,
                "{body}: {:?}",
                row.diagnostics
            );
            assert!(no_violation(row), "{body}: {:?}", row.diagnostics);
            assert!(
                standing(row)
                    .iter()
                    .any(|standing| standing.contains("candidate, not validated")),
                "{body}: {:?}",
                row.diagnostics
            );
        }
    }
}

// ---------------------------------------------------------------------------
// What stays `Pending`: a row whose only open reason is the hold.
// ---------------------------------------------------------------------------

#[test]
fn a_row_whose_only_reason_is_the_hold_stays_pending() {
    for (body, callee) in [(BALANCED, "k_close"), (PLAIN_ACTUAL, "k_input_new")] {
        for contracts in [deferred(callee), pin_withheld(callee)] {
            let summary = analyse(body, &contracts);
            let row = w(&summary);
            assert_eq!(
                row.status,
                AnalysisStatus::Pending,
                "{body}: {:?}",
                row.diagnostics
            );
            assert_eq!(summary.status, AnalysisStatus::Pending);
            assert!(standing(row).is_empty());
            assert_eq!(held(row), [callee]);
        }
    }
}

/// A unit with no `Pending` row is analysed once and published byte for byte
/// as before: the environment holding a declaration nothing here reaches, and
/// a row that is `Unknown` for its own reasons beside a held call.
#[test]
fn rows_that_are_not_pending_are_untouched() {
    let unrelated = deferred("k_input_new");
    for body in [BALANCED, TWICE, LEAKED] {
        let with_hold = analyse(body, &unrelated);
        let without = analyse(body, &load(BARE));
        assert_eq!(format!("{with_hold:?}"), format!("{without:?}"), "{body}");
    }
    // `Unknown` beside a held call already: the second analysis adds nothing.
    let unknown = "int w(struct k_config *config, const char *input, void *data, struct k_ctx *x) \
         { void *i; x->next->next = x; i = k_input_new(config, input, data, 1); return 0; }";
    let row = analyse(unknown, &deferred("k_input_new"));
    assert_eq!(w(&row).status, AnalysisStatus::Unknown);
    assert!(standing(w(&row)).is_empty(), "{:?}", w(&row).diagnostics);
}

/// The single-function entry point settles its one row the same way.
#[test]
fn the_single_function_entry_settles_the_same_way() {
    let contracts = deferred("k_input_new");
    let row = analyze_legacy_c(&format!("{PRELUDE}{ARGUMENT_ACCESS}"), "w", &contracts).unwrap();
    assert_eq!(row.status, AnalysisStatus::Unknown, "{:?}", row.diagnostics);
    assert!(!standing(&row).is_empty());
    let row = analyze_legacy_c(&format!("{PRELUDE}{PLAIN_ACTUAL}"), "w", &contracts).unwrap();
    assert_eq!(row.status, AnalysisStatus::Pending, "{:?}", row.diagnostics);
}

#[test]
fn a_standing_reason_is_never_read_as_a_held_call() {
    assert!(STANDING_PREFIX.starts_with("reason standing"));
    let row = analyse(ARGUMENT_ACCESS, &deferred("k_input_new"));
    for description in standing(w(&row)) {
        assert!(held_call(&description).is_none());
        assert!(pin_hold::held_call(&description).is_none());
        assert!(!qcore_c_gateway::legacy_c::verdict::is_held_call(
            &description
        ));
    }
}

// ---------------------------------------------------------------------------
// The production entry point: the unit verdict.
// ---------------------------------------------------------------------------

#[test]
fn the_unit_is_unknown_not_pending_through_the_production_route() {
    if qcore_c_gateway::clang_command().is_err() {
        eprintln!("skipping Clang-dependent W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL test");
        return;
    }
    let verdict = |body: &str| {
        let source = format!("{PRELUDE}{body}");
        let unit = C11TranslationUnit {
            source: &source,
            ..Default::default()
        };
        analyze_unrestricted_c11(&unit, "w", &deferred("k_input_new"))
            .unwrap()
            .verdict()
    };
    assert_eq!(verdict(ARGUMENT_ACCESS), Verdict::Unknown);
    assert_eq!(verdict(LATER_REFUSAL), Verdict::Unknown);
    assert_eq!(verdict(PLAIN_ACTUAL), Verdict::Pending);
    let source = format!("{PRELUDE}{ARGUMENT_ACCESS}");
    let unit = C11TranslationUnit {
        source: &source,
        ..Default::default()
    };
    match analyze_unrestricted_c11(&unit, "w", &deferred("k_input_new")).unwrap() {
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => assert_eq!(summary.status, AnalysisStatus::Unknown),
        other => panic!("the held unit is Unknown: {other:?}"),
    }
}
