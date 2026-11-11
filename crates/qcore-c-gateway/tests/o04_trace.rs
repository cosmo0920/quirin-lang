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

//! O04-TRACE — declared variadic data tails and one level of nested calls
//! (`docs/reports/remaining-o04-trace/README.md`).
//!
//! Card O04-B09B10 measured the constructor's success exit **C6** down to
//! exactly one refused statement and proved, with a labelled control, that the
//! `return ic;` residue is that statement's shadow. The statement is
//!
//! ```c
//! flb_trace("… '%s' … '%s'", flb_input_chunk_get_name(ic), tag);
//! ```
//!
//! and it needs **both** of this card's families: a declared variadic tail
//! (`flb_trace` is `void flb_trace(const char *, ...)`, card O04-ARMS's
//! **O04-ARMS-VARIADIC**) *and* nested-call modeling (the second argument is a
//! call, card O04-B09B10's **O04-B09B10-NESTED**, which that card turned into
//! an explicit refusal because nothing modeled it).
//!
//! Three legs, all through the public gateway entry points only.
//!
//! **Held-out generic family.** A `v_*` API deliberately unrelated to ChunkIO,
//! to Fluent Bit **and** to the `g_*` (O04), `h_*` (O04-ARMS), `k_*` (O04-B08),
//! `r_*` (O04-READ-BOUND) and `m_*` (O04-B09B10) families. Every rule is stated
//! over declared C types and contract modes, so no name in it can matter.
//!
//! **Real source.** The same byte-verbatim upstream regions the five earlier
//! cards pinned, under their byte-unchanged premise sets plus two declared
//! callees.
//!
//! **The vocabulary boundary.** What the loader accepts and refuses, and what a
//! declaration that is not there cannot buy.
//!
//! Nothing here promotes a verdict. `Safe` in the generic family is `Safe` for
//! a synthetic declared custody projection under premises this card names, and
//! the real function's `Safe` rests on every premise §4 of the report lists —
//! the declared contracts are assumptions, not facts any body here discharges.
//! No raw Cycle 19 unit is promoted.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, ExitCustody, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyProgramSummary, LegacySafetyCertificate, ResourceContracts, SiteDisposition,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::{
    declared_variadic_data_tails, load_resource_contract_sources,
};

#[path = "support/text.rs"]
mod text;
use text::normalize_newlines;

const PRELUDE: &str = include_str!("fixtures/o04/prelude.c");
const STORAGE_INPUT: &str = include_str!("fixtures/o04/flb_storage_input.h.txt");
const CALLER: &str = include_str!("fixtures/o04/flb_input_chunk_create.c.txt");
const CONSTRUCTOR: &str = include_str!("fixtures/o04/cio_chunk_open.c.txt");
const PATH_CONTRACTS: &str = include_str!("fixtures/contracts/o04_path.qv");
const ARMS_CALLEES: &str = include_str!("fixtures/contracts/o04_arms_callees.qr");
const B08_CALLEES: &str = include_str!("fixtures/contracts/o04_b08_callees.qr");
const B09B10_CALLEES: &str = include_str!("fixtures/contracts/o04_b09b10_callees.qr");
/// The two declared callee premises this card adds; see the fixture header.
const TRACE_CALLEES: &str = include_str!("fixtures/contracts/o04_trace_callees.qr");
const GENERIC_CONTRACTS: &str = include_str!("fixtures/contracts/o04_trace_generic.qv");

/// The refusal card O04-B09B10 installed, which every nested call outside this
/// card's premises must keep, verbatim.
fn nested_refusal(callee: &str) -> String {
    format!("nested call `{callee}` inside a modeled call is outside the resource projection")
}

/// The refusal this card installs for an actual beyond the declared parameter
/// vector with no declared tail to cover it.
fn excess_refusal(callee: &str) -> String {
    format!(
        "arguments beyond the declared parameters of `{callee}` are outside the resource projection"
    )
}

fn load(name: &str, source: &str) -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(name.to_owned(), source.to_owned())])).unwrap()
}

fn path_contracts() -> String {
    normalize_newlines(&format!(
        "{}\n{ARMS_CALLEES}\n{B08_CALLEES}\n{B09B10_CALLEES}\n{TRACE_CALLEES}",
        normalize_newlines(PATH_CONTRACTS)
    ))
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent O04-TRACE test: {reason}");
            false
        }
    }
}

fn program(result: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match result {
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        }
        | UnrestrictedLegacyResult::Unsafe(summary) => summary,
        other => panic!("expected an analyzable partial summary: {other:?}"),
    }
}

fn function<'a>(summary: &'a LegacyProgramSummary, name: &str) -> &'a LegacyFunctionSummary {
    summary
        .functions
        .get(name)
        .unwrap_or_else(|| panic!("missing function `{name}`"))
}

fn unsupported(summary: &LegacyFunctionSummary) -> Vec<&str> {
    summary
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => Some(description.as_str()),
            _ => None,
        })
        .collect()
}

fn leaks(summary: &LegacyFunctionSummary) -> Vec<usize> {
    summary
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match diagnostic.kind {
            LegacyDiagnosticKind::Leak { acquisition_line } => Some(acquisition_line),
            _ => None,
        })
        .collect()
}

fn exit_shape(summary: &LegacyFunctionSummary) -> Vec<(ExitCustody, Vec<SiteDisposition>)> {
    summary
        .exits
        .iter()
        .map(|exit| {
            (
                exit.custody,
                exit.sites.iter().map(|site| site.disposition).collect(),
            )
        })
        .collect()
}

// --------------------------------------------------------------------------
// Held-out generic family
// --------------------------------------------------------------------------

/// `v_box` carries the shapes the rules separate. `note` is a declared scalar,
/// `label` an opaque pointer this body never binds; `item` is a tracked child.
const DECLARATIONS: &str = "\
struct v_item { int weight; }; \
struct v_box { int flag; struct v_item *item; }; \
struct v_box *v_make(int n); \
struct v_item *v_take(int n); \
void v_drop(struct v_box *b); \
void v_free(struct v_item *i); \
char *v_name(struct v_box *b); \
struct v_item *v_grab(struct v_box *b); \
void v_eat(struct v_item *i); \
int v_size(struct v_box *b); \
void v_log(const char *fmt, ...); \
void v_plain(const char *fmt, ...);";

fn analyze_generic(body: &str, entry: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS} {body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        entry,
        &load("o04_trace_generic.qcore", GENERIC_CONTRACTS),
    )
    .unwrap_or_else(|error| panic!("generic fixture rejected by Clang: {error:?}\n{source}"))
}

fn generic_summary(body: &str, entry: &str) -> LegacyFunctionSummary {
    let result = analyze_generic(body, entry);
    match &result {
        UnrestrictedLegacyResult::Safe(certificate) => certificate
            .summary
            .functions
            .get(entry)
            .expect("entry present")
            .clone(),
        other => function(program(other), entry).clone(),
    }
}

// --------------------------------------------------------------------------
// Real source — the headline
// --------------------------------------------------------------------------

fn real_unit() -> String {
    format!("{PRELUDE}\n{STORAGE_INPUT}\n{CONSTRUCTOR}\n{CALLER}\n")
}

fn analyze_real(source: &str) -> UnrestrictedLegacyResult {
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "flb_input_chunk_create",
        &load("o04_trace_path.qcore", &path_contracts()),
    )
    .unwrap_or_else(|error| panic!("O04-TRACE fixture rejected by Clang: {error:?}"))
}

fn real_caller(source: &str) -> LegacyFunctionSummary {
    let result = analyze_real(source);
    function(program(&result), "flb_input_chunk_create").clone()
}

/// **The headline.** On the byte-verbatim upstream body, with the two declared
/// callee premises added and every earlier card's premise set unchanged, the
/// success exit **C6** discharges and `flb_input_chunk_create` reaches
/// function-level `Safe`.
///
/// **Composed-tree reconciliation (LOADER-ABI × NEIGHBOURS-CHAINED-TAIL).** This
/// headline was withdrawn by card LOADER-ABI measured alone, and is restored
/// here on a *better* premise than the one it originally rested on. The fixture
/// used to declare `flb_error` with three named parameters beside a C prelude
/// declaring the real `void flb_error(const char *format, ...)`, so the chained
/// tail actual `storage->stream->name` sat at a declared `Borrowed` position;
/// LOADER-ABI refused that arity — a declaration may not name modes for
/// parameters C never typed — and corrected it to one parameter plus
/// `resource_variadic_data_tail()`. That correction is exactly what puts the
/// actual at a **declared data tail**, which is the position clause **T5** of
/// card NEIGHBOURS-CHAINED-TAIL admits a bounded typed access chain at. The two
/// grounds compose rather than collide: T5's premise is satisfied *because* the
/// declaration is faithful.
///
/// The result is therefore the same six-exit ledger O04-TRACE measured, now
/// resting on a premise C's own `...` supports instead of on two parameters
/// that do not exist. What is **not** restored is any claim about production:
/// `O04-TRACE-REACH` stands, and card NEIGHBOURS-CHAINED-TAIL's own corpus
/// measurement is where the production effect is recorded.
///
/// **Retired by name:** the probe card LOADER-ABI added here (substituting the
/// chained actual and asserting the whole-function result returned) is removed
/// — with the baseline itself `Safe` it can no longer discriminate and would
/// pass vacuously. Its claim lives where it is still live, in
/// `o04_arms.rs::the_declaration_decides_the_same_chain_in_a_variadic_tail`.
///
/// This is a claim about *this projection under the premises §4 of the report
/// names*, most of them trusted declarations. It is not a proof that the C is
/// correct, and it promotes no raw Cycle 19 unit.
#[test]
fn the_success_exit_discharges_and_the_caller_reaches_safe() {
    if !clang_available() {
        return;
    }
    let caller = real_caller(&real_unit());
    assert!(
        caller.diagnostics.is_empty(),
        "no refusal remains: {:?}",
        caller.diagnostics
    );
    assert_eq!(caller.status, AnalysisStatus::Safe);
    assert_eq!(caller.exits.len(), 6);
    assert!(
        caller
            .exits
            .iter()
            .all(|exit| exit.custody == ExitCustody::Discharged),
        "every exit settles: {:?}",
        exit_shape(&caller)
    );
    // C6, the success exit, escapes all three identities: the chunk and the
    // routes mask as live linked children of the escaping aggregate (premise
    // P-B08-SHALLOW), and `ic` itself as the returned value. That is exactly
    // what card O04-B09B10's labelled control predicted, now measured on the
    // verbatim source rather than on a substitution.
    let success = caller
        .exits
        .iter()
        .max_by_key(|exit| exit.line)
        .expect("present");
    assert_eq!(
        success.custody,
        ExitCustody::Discharged,
        "C6 still discharges: {:?}",
        exit_shape(&caller)
    );
    assert_eq!(success.sites.len(), 3);
    assert!(
        success
            .sites
            .iter()
            .all(|site| site.disposition == SiteDisposition::Escaped),
        "the success exit escapes all three identities: {:?}",
        success.sites
    );
    assert!(leaks(&caller).is_empty());
}

/// The **whole** per-exit ledger, pinned exactly: every exit line, its custody
/// disposition, and every site with its acquisition line. The five failure arms
/// keep the exact shape cards O04-ARMS, O04-B08, O04-READ-BOUND and O04-B09B10
/// measured — each releasing the chunk once, all naming acquisition site 206 —
/// and C6 escapes all three identities. A silent change in either direction
/// fails here.
#[test]
fn the_whole_per_exit_ledger_is_pinned() {
    if !clang_available() {
        return;
    }
    let caller = real_caller(&real_unit());
    let ledger = caller
        .exits
        .iter()
        .map(|exit| {
            (
                exit.line,
                exit.custody,
                exit.sites
                    .iter()
                    .map(|site| (site.acquisition_line, site.disposition))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    use ExitCustody::Discharged;
    use SiteDisposition::{Escaped, Released};
    assert_eq!(
        ledger,
        vec![
            (
                277,
                Discharged,
                vec![(206, Escaped), (227, Escaped), (244, Escaped)]
            ),
            (251, Discharged, vec![(206, Released), (227, Released)]),
            (231, Discharged, vec![(206, Released)]),
            (225, Discharged, vec![(206, Released)]),
            (218, Discharged, vec![(206, Released)]),
            // Composed-tree reconciliation: the logging arm at line 211
            // discharges again. Card LOADER-ABI's correction of `flb_error`
            // to `(const char *, ...)` put its chained actual at a declared
            // data tail, and clause T5 of card NEIGHBOURS-CHAINED-TAIL admits
            // it there; see
            // `the_success_exit_discharges_and_the_caller_reaches_safe`.
            (211, Discharged, vec![]),
        ],
        "the whole per-exit ledger: {:?}",
        exit_shape(&caller)
    );
}

/// **The premises are load-bearing, on the real subject.** Withdrawing either
/// declared callee restores the exact refusal card O04-B09B10 measured, and the
/// success exit goes back to `Unknown`.
#[test]
fn withdrawing_either_declared_callee_restores_the_refusal() {
    if !clang_available() {
        return;
    }
    let unit = real_unit();
    for (label, contracts, expected) in [
        (
            "no flb_trace declaration at all",
            format!(
                "{}\n{ARMS_CALLEES}\n{B08_CALLEES}\n{B09B10_CALLEES}",
                PATH_CONTRACTS.replace("\r\n", "\n")
            ),
            "call to uncontracted function `flb_trace`".to_owned(),
        ),
        // Without the tail, the nested position is not a data position, so
        // premise N8 fails and card O04-B09B10's nested refusal is what
        // remains — the tail is load-bearing for the *nested* admission too,
        // which is exactly the "both halves must compose" claim. The
        // excess-actual refusal this card adds is exercised separately, in the
        // generic family, where no nested call confounds it.
        (
            "flb_trace declared without its variadic data tail",
            path_contracts().replace("    ensures normal resource_variadic_data_tail();\n", ""),
            nested_refusal("flb_input_chunk_get_name"),
        ),
        (
            "the nested accessor left uncontracted",
            path_contracts().replace(
                "extern fn flb_input_chunk_get_name(ic: ptr<mut, u8>) -> ptr<mut, u8>",
                "extern fn flb_input_chunk_get_name_unused(ic: ptr<mut, u8>) -> ptr<mut, u8>",
            ),
            nested_refusal("flb_input_chunk_get_name"),
        ),
    ] {
        let result = analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: &unit,
                ..Default::default()
            },
            "flb_input_chunk_create",
            &load("o04_trace_path.qcore", &contracts),
        )
        .unwrap_or_else(|error| panic!("{label}: rejected by Clang: {error:?}"));
        let caller = function(program(&result), "flb_input_chunk_create");
        assert!(
            unsupported(caller).contains(&expected.as_str()),
            "{label}: expected `{expected}`, got {:?}",
            caller.diagnostics
        );
        assert_eq!(caller.status, AnalysisStatus::Unknown, "{label}");
        assert!(
            caller
                .exits
                .iter()
                .any(|exit| exit.custody != ExitCustody::Discharged),
            "{label}: the success exit is not settled"
        );
    }
}

/// The constructor's own boundary is untouched: four Discharged exits, two
/// Unknown, and the seven `INDIRECT` refusals card O04-B09B10 pinned.
#[test]
fn the_constructor_boundary_is_unchanged() {
    if !clang_available() {
        return;
    }
    let result = analyze_real(&real_unit());
    let constructor = function(program(&result), "cio_chunk_open");
    assert_eq!(
        constructor
            .exits
            .iter()
            .filter(|exit| exit.custody == ExitCustody::Discharged)
            .count(),
        4,
        "{:?}",
        exit_shape(constructor)
    );
    assert_eq!(
        constructor
            .exits
            .iter()
            .filter(|exit| exit.custody == ExitCustody::Unknown)
            .count(),
        2,
        "{:?}",
        exit_shape(constructor)
    );
    assert!(leaks(constructor).is_empty());
}

/// The whole-program result stays `Unknown`, because `cio_chunk_open` in the
/// same unit keeps its own refusals. A function reaching `Safe` is not a unit
/// reaching `Safe`.
#[test]
fn the_program_stays_unknown() {
    if !clang_available() {
        return;
    }
    assert!(matches!(
        analyze_real(&real_unit()),
        UnrestrictedLegacyResult::Unknown { .. }
    ));
}

// --------------------------------------------------------------------------
// The vocabulary boundary
// --------------------------------------------------------------------------

/// The loader accepts the clause, rederives it from the retained trusted
/// sources, and refuses every malformed spelling — total-or-reject, with no
/// partial environment.
#[test]
fn the_variadic_tail_clause_is_total_or_reject() {
    // The callee fixture is a concatenation piece, so it carries no module
    // header of its own; the loader needs one.
    let module = normalize_newlines(&format!("module contract;\n{TRACE_CALLEES}"));
    let tails = declared_variadic_data_tails(&load("o04_trace_callees.qcore", &module))
        .expect("the fixture loads");
    assert_eq!(
        tails,
        ["flb_trace".to_owned()].into_iter().collect(),
        "exactly the declaration that carries the clause"
    );
    // An environment with no declared sources has no tails: the tail is a
    // declaration, never an inference.
    assert!(
        declared_variadic_data_tails(&ResourceContracts::default())
            .expect("an empty environment")
            .is_empty()
    );
    for (label, source) in [
        (
            "an argument the tail has no index to point at",
            module.replace(
                "resource_variadic_data_tail()",
                "resource_variadic_data_tail(format)",
            ),
        ),
        (
            "the clause twice",
            module.replace(
                "    ensures normal resource_variadic_data_tail();\n",
                "    ensures normal resource_variadic_data_tail();\n    ensures normal resource_variadic_data_tail();\n",
            ),
        ),
        (
            "the clause as a requirement rather than an exit fact",
            module.replace(
                "ensures normal resource_variadic_data_tail();",
                "requires resource_variadic_data_tail();",
            ),
        ),
        (
            "the clause on an error exit",
            module.replace(
                "ensures normal resource_variadic_data_tail();",
                "ensures error(E) resource_variadic_data_tail();",
            ),
        ),
    ] {
        assert_ne!(source, module, "{label}: the substitution applied");
        let loaded = load_resource_contract_sources(&BTreeMap::from([(
            "mutated.qcore".to_owned(),
            source,
        )]));
        assert!(loaded.is_err(), "{label}: accepted");
    }
    // The clause does not replace a mode: a parameter still needs one.
    let missing = module.replace("    ensures normal resource_borrowed(format);\n", "");
    assert_ne!(missing, module);
    assert!(
        load_resource_contract_sources(&BTreeMap::from([("m.qcore".to_owned(), missing)])).is_err()
    );
}

// --------------------------------------------------------------------------
// Held-out generic family — the admitted shapes
// --------------------------------------------------------------------------

/// The real shape, synthetically: a nested contracted accessor inside a
/// declared data tail, with the owned child released on every exit.
#[test]
fn a_nested_accessor_in_a_declared_tail_discharges_every_exit() {
    if !clang_available() {
        return;
    }
    let summary = generic_summary(
        "int v_use(int n) { \
           struct v_box *b = v_make(n); \
           if (b == 0) { return -1; } \
           v_log(\"%s %d\", v_name(b), n); \
           v_drop(b); \
           return 0; }",
        "v_use",
    );
    assert!(
        summary.diagnostics.is_empty(),
        "no refusal: {:?}",
        summary.diagnostics
    );
    assert_eq!(summary.status, AnalysisStatus::Safe);
    assert!(
        summary
            .exits
            .iter()
            .all(|exit| exit.custody == ExitCustody::Discharged),
        "{:?}",
        exit_shape(&summary)
    );
}

/// Precision in the defect direction: the same body without the release is
/// `Unsafe` with exactly one leak. The admission does not hide anything.
#[test]
fn an_admitted_nested_call_does_not_hide_a_leak() {
    if !clang_available() {
        return;
    }
    let summary = generic_summary(
        "int v_use(int n) { \
           struct v_box *b = v_make(n); \
           if (b == 0) { return -1; } \
           v_log(\"%s %d\", v_name(b), n); \
           return 0; }",
        "v_use",
    );
    assert_eq!(leaks(&summary).len(), 1, "{:?}", summary.diagnostics);
}

/// A tail actual that is a plain declared scalar, an enumeration constant, a
/// string literal or a name this body never binds is data; the whole family of
/// admitted tails is exercised in one accepted body.
#[test]
fn the_admitted_tail_actual_shapes_are_data() {
    if !clang_available() {
        return;
    }
    let summary = generic_summary(
        "enum v_kind { V_ONE = 1, V_TWO = 2 }; \
         int v_use(int n) { \
           struct v_box *b = v_make(n); \
           int count = 3; \
           if (b == 0) { return -1; } \
           v_log(\"%s %d %d %d\", \"x\", count, V_TWO, 7); \
           v_drop(b); \
           return 0; }",
        "v_use",
    );
    assert!(
        summary.diagnostics.is_empty(),
        "every admitted tail shape is data: {:?}",
        summary.diagnostics
    );
    assert_eq!(summary.status, AnalysisStatus::Safe);
}

// --------------------------------------------------------------------------
// Held-out generic family — the adversarial rows
// --------------------------------------------------------------------------

/// **Adversarial row 1 — a tail smuggling a custody pointer.** The declaration
/// covers what it covers: a tail actual that is a tracked resource identity is
/// refused *under* the declaration, exactly as it would be without one.
#[test]
fn a_tail_smuggling_a_tracked_identity_is_never_admitted() {
    if !clang_available() {
        return;
    }
    for body in [
        // The tracked aggregate itself.
        "int v_use(int n) { struct v_box *b = v_make(n); if (b == 0) { return -1; } \
           v_log(\"%p\", b); v_drop(b); return 0; }",
        // An owned child.
        "int v_use(int n) { struct v_item *i = v_take(n); if (i == 0) { return -1; } \
           v_log(\"%p\", i); v_free(i); return 0; }",
        // Its address, which is how a callee would be handed a handle it keeps.
        "int v_use(int n) { struct v_box *b = v_make(n); if (b == 0) { return -1; } \
           v_log(\"%p\", &b); v_drop(b); return 0; }",
        // A member access, which this rule does not read. Card
        // W44-INDIRECT-CALL-ARGUMENT-ACCESS moved the live-base row
        // (`v_log("%d", b->flag)` before `v_drop(b)`): a scalar member of a live
        // identity is data, not the identity, and its call argument rule admits
        // it under the base's obligation. The row now reads the member after
        // the release, which that rule refuses and accuses.
        "int v_use(int n) { struct v_box *b = v_make(n); if (b == 0) { return -1; } \
           v_drop(b); v_log(\"%d\", b->flag); return 0; }",
    ] {
        let summary = generic_summary(body, "v_use");
        assert_ne!(
            summary.status,
            AnalysisStatus::Safe,
            "a tail actual carrying an identity must refuse: {body}"
        );
        assert!(
            !summary.diagnostics.is_empty(),
            "the refusal is reported, never silent: {body}"
        );
    }
}

/// **Adversarial row 2 — a nested call whose result is owned.** The result mode
/// is the premise that makes the nested result safe to leave unbound; a call
/// that produces custody in an argument position is refused, and card
/// O04-B09B10's diagnostic is what says so.
#[test]
fn a_nested_call_producing_custody_is_never_admitted() {
    if !clang_available() {
        return;
    }
    let summary = generic_summary(
        "int v_use(int n) { struct v_box *b = v_make(n); if (b == 0) { return -1; } \
           v_log(\"%p\", v_grab(b)); v_drop(b); return 0; }",
        "v_use",
    );
    // Re-pinned by CG05-NESTED-CALL-EFFECTS (gateway rule 10y): the owned
    // result is bound to a reserved place and handed to the data tail, which
    // takes no custody, so it is reported as a leak rather than refused.
    assert!(
        unsupported(&summary).contains(&nested_refusal("v_grab").as_str())
            || summary
                .diagnostics
                .iter()
                .any(|diagnostic| matches!(diagnostic.kind, LegacyDiagnosticKind::Leak { .. })),
        "an owned nested result is refused or reported: {:?}",
        summary.diagnostics
    );
    assert_ne!(summary.status, AnalysisStatus::Safe);
}

/// **Adversarial row 3 — two-level nesting.** Each admitted nested call must
/// itself contain no call, so `f(g(h(x)))` still refuses.
#[test]
fn two_level_nesting_is_never_admitted() {
    if !clang_available() {
        return;
    }
    let body = "int v_use(int n) { struct v_box *b = v_make(n); if (b == 0) { return -1; } \
        v_log(\"%d\", v_size(v_make(n))); v_drop(b); return 0; }";
    let summary = generic_summary(body, "v_use");
    // Re-pinned by CG05-NESTED-CALL-EFFECTS (gateway rule 10y): two levels are
    // composed inside-out; `v_make`'s owned result, only borrowed by
    // `v_size`, is reported as a leak.
    assert!(
        unsupported(&summary)
            .iter()
            .any(|description| description.starts_with("nested call"))
            || summary
                .diagnostics
                .iter()
                .any(|diagnostic| matches!(diagnostic.kind, LegacyDiagnosticKind::Leak { .. })),
        "a call inside a nested call is refused or reported: {:?}",
        summary.diagnostics
    );
    assert_ne!(summary.status, AnalysisStatus::Safe);
}

/// **Sibling-neutral calls compose.** C leaves their evaluation order
/// unspecified, so this is admitted only because both callees are independently
/// modeled as effect-neutral borrowed reads. The retained certificate must
/// rederive the same result.
#[test]
fn sibling_neutral_nested_calls_are_admitted_and_recheck() {
    if !clang_available() {
        return;
    }
    let body = "int v_use(int n) { struct v_box *b = v_make(n); if (b == 0) { return -1; } \
        v_log(\"%s %d\", v_name(b), v_size(b)); v_drop(b); return 0; }";
    let certificate = generic_certificate(body, "v_use");
    let summary = function(&certificate.summary, "v_use");
    assert_eq!(summary.status, AnalysisStatus::Safe);
    assert!(
        unsupported(summary).is_empty(),
        "neutral siblings need no unsupported diagnostic: {:?}",
        summary.diagnostics
    );
    check_legacy_safety_certificate(&certificate).expect("sibling certificate rechecks");
}

/// **Adversarial row 4 — an inner call with no contract in force.** Nothing
/// models its effects, which is the whole reason the statement was refused.
#[test]
fn a_nested_call_without_a_contract_is_never_admitted() {
    if !clang_available() {
        return;
    }
    let summary = generic_summary(
        "int v_other(int n); \
         int v_use(int n) { struct v_box *b = v_make(n); if (b == 0) { return -1; } \
           v_log(\"%d\", v_other(n)); v_drop(b); return 0; }",
        "v_use",
    );
    assert!(
        unsupported(&summary).contains(&nested_refusal("v_other").as_str()),
        "{:?}",
        summary.diagnostics
    );
    assert_ne!(summary.status, AnalysisStatus::Safe);
}

/// A nested call that is only *part* of an argument, or that is the whole
/// statement's outer call with a nested call inside a non-tail position it does
/// not admit, keeps the refusal.
#[test]
fn a_partial_nested_call_expression_is_never_admitted() {
    if !clang_available() {
        return;
    }
    // Flipped by CG05-NESTED-CALL-EFFECTS (gateway rule 10y): arithmetic
    // around a borrowed `None` call in a data tail is a data actual, composed.
    let summary = generic_summary(
        "int v_use(int n) { struct v_box *b = v_make(n); if (b == 0) { return -1; } \
           v_log(\"%d\", v_size(b) + 1); v_drop(b); return 0; }",
        "v_use",
    );
    assert_eq!(
        summary.status,
        AnalysisStatus::Safe,
        "{:?}",
        summary.diagnostics
    );
    // A nested call at a *custody* position of the outer contract.
    let body = "int v_use(int n) { struct v_box *b = v_make(n); if (b == 0) { return -1; } \
           v_eat(v_grab(b)); v_drop(b); return 0; }";
    let summary = generic_summary(body, "v_use");
    assert!(
        !summary.diagnostics.is_empty(),
        "the shape keeps its refusal: {body}"
    );
    assert_ne!(summary.status, AnalysisStatus::Safe, "{body}");
}

/// **Adversarial row 5 — an address inside the nested call's own arguments.**
/// This is a fail-open the card **found and closed** during implementation:
/// the enclosing `&`-gate is exempted for the admitted nested argument, and
/// `contains_unmodeled_access` does not read `&`, so without premise N9
/// `v_log("%s", v_name(&b))` was admitted with the escape modeled by nobody.
#[test]
fn an_address_inside_the_nested_arguments_is_never_admitted() {
    if !clang_available() {
        return;
    }
    let summary = generic_summary(
        "int v_use(int n) { struct v_box *b = v_make(n); if (b == 0) { return -1; } \
           v_log(\"%s\", v_name(*&b)); v_drop(b); return 0; }",
        "v_use",
    );
    assert_ne!(
        summary.status,
        AnalysisStatus::Safe,
        "an undenotable inner actual must refuse: {:?}",
        summary.diagnostics
    );
    assert!(!summary.diagnostics.is_empty());
}

/// **The arity repair.** An actual beyond the declared parameter vector, with
/// no declared tail, is refused rather than silently dropped — a fail-closed
/// repair of the same class card O04-B09B10 found for a nested call.
#[test]
fn an_excess_actual_without_a_declared_tail_is_refused() {
    if !clang_available() {
        return;
    }
    let summary = generic_summary(
        "int v_use(int n) { struct v_box *b = v_make(n); if (b == 0) { return -1; } \
           v_plain(\"%d %d\", n, n); v_drop(b); return 0; }",
        "v_use",
    );
    assert!(
        unsupported(&summary).contains(&excess_refusal("v_plain").as_str()),
        "{:?}",
        summary.diagnostics
    );
    assert_ne!(summary.status, AnalysisStatus::Safe);
    // The exact-arity control on the same callee still passes.
    let control = generic_summary(
        "int v_use(int n) { struct v_box *b = v_make(n); if (b == 0) { return -1; } \
           v_plain(\"x\"); v_drop(b); return 0; }",
        "v_use",
    );
    assert!(
        control.diagnostics.is_empty(),
        "the exact-arity control is unaffected: {:?}",
        control.diagnostics
    );
    assert_eq!(control.status, AnalysisStatus::Safe);
}

/// A declared tail covers the *tail*, not the named vector: a call that supplies
/// fewer actuals than the declaration names, and a named position whose actual
/// is not data, are unaffected by the tail.
#[test]
fn a_declared_tail_covers_only_the_tail() {
    if !clang_available() {
        return;
    }
    // Two named parameters, a tail, and a tail actual that carries an identity:
    // the tail declaration does not rescue it.
    let summary = generic_summary(
        "int v_use(int n) { struct v_box *b = v_make(n); if (b == 0) { return -1; } \
           v_log(\"x\", b); v_drop(b); return 0; }",
        "v_use",
    );
    assert_ne!(summary.status, AnalysisStatus::Safe);
    assert!(!summary.diagnostics.is_empty());
}

// --------------------------------------------------------------------------
// Certificate rederivation
// --------------------------------------------------------------------------

fn generic_certificate(body: &str, entry: &str) -> LegacySafetyCertificate {
    match analyze_generic(body, entry) {
        UnrestrictedLegacyResult::Safe(certificate) => certificate,
        other => panic!("expected a certificate: {other:?}"),
    }
}

/// The certificate is rechecked from its own retained content, and the
/// substitutions the admission rests on are refused by the **independent**
/// recheck, not merely by the derivation.
#[test]
fn a_certificate_resting_on_the_tail_is_rederived_from_its_own_sources() {
    if !clang_available() {
        return;
    }
    let body = "int v_use(int n) { \
           struct v_box *b = v_make(n); \
           if (b == 0) { return -1; } \
           v_log(\"%s %d\", v_name(b), n); \
           v_drop(b); \
           return 0; }";
    let certificate = generic_certificate(body, "v_use");
    assert!(
        check_legacy_safety_certificate(&certificate).is_ok(),
        "the unmodified certificate rechecks"
    );
    // The tail is rederived from the certificate's own retained sources, so
    // withdrawing the clause there — leaving the loaded `functions` map
    // BYTE-IDENTICAL, since the tail is not carried on a contract — withdraws
    // the admission at the independent recheck. That is the sharp direction:
    // nothing but the declared clause moved.
    let mut forged = certificate.clone();
    let sources = forged
        .contracts
        .source_assumptions()
        .iter()
        .map(|(name, source)| {
            (
                name.clone(),
                normalize_newlines(source)
                    .replace("    ensures normal resource_variadic_data_tail();\n", ""),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_ne!(
        &sources,
        certificate.contracts.source_assumptions(),
        "the substitution applied"
    );
    let reloaded = load_resource_contract_sources(&sources).expect("still a valid environment");
    assert_eq!(
        reloaded.functions(),
        certificate.contracts.functions(),
        "the loaded contract table is byte-identical: only the declared clause moved"
    );
    forged.contracts.set_source_assumptions(sources);
    assert!(
        check_legacy_safety_certificate(&forged).is_err(),
        "the recheck admitted a certificate whose declared tail was withdrawn"
    );
}

/// Reporting aid: the measured per-exit ledger, printed for the card report.
/// `cargo test -p qcore-c-gateway --test o04_trace -- --ignored --nocapture`
#[test]
#[ignore = "reporting aid, not an assertion"]
fn print_the_measured_arm_ledger() {
    if !clang_available() {
        return;
    }
    let caller = real_caller(&real_unit());
    println!("flb_input_chunk_create: {:?}", caller.status);
    for exit in &caller.exits {
        println!(
            "  exit L{} {:?} sites={:?}",
            exit.line,
            exit.custody,
            exit.sites
                .iter()
                .map(|site| (site.acquisition_line, site.disposition))
                .collect::<Vec<_>>()
        );
    }
    println!("  diagnostics: {:?}", caller.diagnostics);
}
