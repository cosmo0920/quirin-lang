//! Card W44-INDIRECT-CALL-ARGUMENT-ACCESS — typed indirect reads in the actuals
//! of a contracted call.
//!
//! A simple statement the lexical gate refused because an actual reads memory
//! is classified again with every typed read at a declared `Borrowed` position
//! or in a declared variadic data tail admitted. When that is a contracted
//! call, it lowers to the typed read of every base those actuals dereference,
//! their second obligation, then the call. A base must be unbound, a non-null
//! binding with no tracked target, or exactly one known, non-null, live
//! identity; a released base is a `use_after_release` candidate. A read at a
//! custody-taking position, or handed to an uncontracted callee, keeps the
//! refusal.
//!
//! Every test asserts the **complete** diagnostic vector of `w` with its
//! status, and every `Safe` certificate is rechecked. The fixtures use a
//! generic `x_*` surface: `Safe` here is `Safe` over synthetic trusted test
//! assumptions and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = "\
module contract;\n\
extern fn x_alloc(n: usize) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_owned(nullable); };\n\
extern fn x_dup(s: ptr<const, u8>) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(s);\n\
           ensures normal resource_return_owned(nullable); };\n\
extern fn x_free(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(p, nullable);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_take(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_consumed(p, nullable);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_use(n: i32) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_note(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(p);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_log(fmt: ptr<const, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(fmt);\n\
           ensures normal resource_return_none(); ensures normal resource_variadic_data_tail(); };\n\
extern fn x_record() -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_return_owned(nonnull); };\n\
extern fn x_maybe_record() -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_return_owned(nullable); };\n\
extern fn x_record_free(r: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(r, nonnull);\n\
           ensures normal resource_return_none(); };";

const DECLARATIONS: &str = "\
void *x_alloc(__SIZE_TYPE__ n); char *x_dup(const char *s); void x_free(void *p); \
void x_take(void *p); void x_use(int n); void x_note(void *p); \
void x_log(const char *fmt, ...); \
struct x_lock { int held; }; \
struct x_rec { int count; char *name; struct x_rec *next; struct x_lock lock; }; \
struct x_rec *x_record(void); struct x_rec *x_maybe_record(void); \
void x_record_free(struct x_rec *r); void y_unknown(int n); void y_release(void *p);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "call_argument_access.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the synthetic contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent call-argument-access test: {reason}");
            false
        }
    }
}

fn analyze(body: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS}\n{body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"))
}

fn program(result: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary,
        other => panic!("no projection at all: {other:?}"),
    }
}

/// The status and the complete, sorted, de-duplicated diagnostic vector of `w`.
fn outcome(body: &str) -> (&'static str, Vec<String>) {
    let result = analyze(body);
    let status = match &result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(certificate)
                .expect("a Safe result carries a certificate that rechecks");
            "Safe"
        }
        UnrestrictedLegacyResult::Unsafe(_) => "Unsafe",
        UnrestrictedLegacyResult::Unknown { .. } => "Unknown",
    };
    let mut keys = program(&result)
        .functions
        .get("w")
        .expect("missing function `w`")
        .diagnostics
        .iter()
        .map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::UnknownAlias { variable } => format!("unknown_alias:{variable}"),
            LegacyDiagnosticKind::Leak { .. } => "leak".to_owned(),
            LegacyDiagnosticKind::DoubleRelease { .. } => "double_release".to_owned(),
            LegacyDiagnosticKind::ReleaseAfterEscape { .. } => "release_after_escape".to_owned(),
            LegacyDiagnosticKind::CustodyConflict { .. } => "custody_conflict".to_owned(),
            LegacyDiagnosticKind::InvalidRelease { .. } => "invalid_release".to_owned(),
            LegacyDiagnosticKind::UseAfterRelease { .. } => "use_after_release".to_owned(),
            LegacyDiagnosticKind::KindMismatchedRelease { .. } => "invalid_release".to_owned(),
            LegacyDiagnosticKind::UseAfterClose { .. } => "use_after_release".to_owned(),
            LegacyDiagnosticKind::CountedUser { .. } => "counted_user".to_owned(),
            LegacyDiagnosticKind::Unsupported { description } => {
                format!("unsupported:{description}")
            }
            LegacyDiagnosticKind::SummaryConflict { .. } => "summary_conflict".to_owned(),
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    (status, keys)
}

fn expect(label: &str, body: &str, status: &str, diagnostics: &[&str]) {
    if !clang_available() {
        return;
    }
    let (actual_status, actual) = outcome(body);
    let mut wanted = diagnostics
        .iter()
        .map(|text| (*text).to_owned())
        .collect::<Vec<_>>();
    wanted.sort();
    assert_eq!(
        (actual_status, &actual),
        (status, &wanted),
        "{label}\n{body}"
    );
}

const GATE: &str = "unsupported:indirect memory access is outside the resource projection";

// -- Admitted ----------------------------------------------------------------

#[test]
fn the_brief_duplicate_of_an_element_certifies() {
    expect(
        "`strdup(src[i])` over a caller's `char **`, result guarded and released",
        "void w(char **src, int i) { char *d = x_dup(src[i]); if (!d) return; x_free(d); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_scalar_member_of_a_caller_record_certifies() {
    expect(
        "`x_use(r->count)` at a `Borrowed` position, `r` an unbound formal",
        "void w(struct x_rec *r) { x_use(r->count); x_use(r->count + 1); }",
        "Safe",
        &[],
    );
}

#[test]
fn reads_through_a_live_identity_certify() {
    expect(
        "a scalar member at a `Borrowed` position and a pointer member in a tail, \
         through a live record this body acquired",
        "void w(void) { struct x_rec *r = x_record(); x_use(r->count); \
         x_log(\"%s\", r->name); x_record_free(r); }",
        "Safe",
        &[],
    );
}

#[test]
fn an_existing_bound_read_keeps_its_own_answer() {
    expect(
        "`x_note(r->name)` over a record this body acquired is O04-READ-BOUND's actual, which \
         requires a field link and refuses without one; this card adds nothing to it",
        "void w(void) { struct x_rec *r = x_record(); x_note(r->name); x_record_free(r); }",
        "Unknown",
        &["unknown_alias:r"],
    );
}

#[test]
fn a_pointer_read_through_a_linked_owner_refuses() {
    expect(
        "`r->next` holds a linked child and is itself bound: the pointer read may name a \
         tracked object, so both the owner and the place refuse",
        "void w(void) { struct x_rec *r = x_record(); struct x_rec *c = x_record(); \
         r->next = c; x_log(\"%p\", r->next); x_record_free(c); x_record_free(r); }",
        "Unknown",
        &["unknown_alias:r", "unknown_alias:r->next"],
    );
}

#[test]
fn a_tail_actual_certifies() {
    expect(
        "a typed read in a declared variadic data tail",
        "void w(struct x_rec *r, int *n) { x_log(\"%d %d %s\", r->count, *n, r->name); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_sub_object_address_certifies() {
    expect(
        "`&r->lock`, its macro-expanded `&(r)->lock` and `&buf[i]` at `Borrowed` positions",
        "void w(struct x_rec *r, int i) { char buf[8]; x_note(&r->lock); x_note(&(r)->lock); \
         x_note(&buf[i]); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_type_size_times_a_member_certifies() {
    expect(
        "`x_alloc(sizeof(int) * r->count)` guarded and released",
        "void w(struct x_rec *r) { char *p = x_alloc(sizeof(int) * r->count); if (!p) return; \
         x_free(p); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_guarded_nullable_base_certifies() {
    expect(
        "a nullable record read after its null test",
        "void w(void) { struct x_rec *r = x_maybe_record(); if (!r) return; x_use(r->count); \
         x_record_free(r); }",
        "Safe",
        &[],
    );
}

// -- Accused -----------------------------------------------------------------

#[test]
fn a_read_after_release_is_accused() {
    expect(
        "`x_use(r->count)` after `x_record_free(r)`",
        "void w(void) { struct x_rec *r = x_record(); x_record_free(r); x_use(r->count); }",
        "Unsafe",
        &["unknown_alias:r", "use_after_release"],
    );
}

#[test]
fn a_member_address_after_release_is_accused() {
    expect(
        "`x_log(\"%p\", &r->lock)` after `x_record_free(r)` (a tail address, which \
         O04-B09B10's `Borrowed`-only route does not cover)",
        "void w(void) { struct x_rec *r = x_record(); x_record_free(r); \
         x_log(\"%p\", &r->lock); }",
        "Unsafe",
        &["unknown_alias:r", "use_after_release"],
    );
}

// -- Refused -----------------------------------------------------------------

#[test]
fn an_unguarded_nullable_base_refuses() {
    expect(
        "a possibly-null record read without its null test (the release's own refusal kept)",
        "void w(void) { struct x_rec *r = x_maybe_record(); x_use(r->count); x_record_free(r); }",
        "Unknown",
        &[
            "unknown_alias:r",
            "unsupported:nullable resource `r` used by a non-null contract",
        ],
    );
}

#[test]
fn a_releasing_position_keeps_the_refusal() {
    expect(
        "`x_free(src[i])`: the callee would release an object the projection cannot name",
        "void w(char **src, int i) { x_free(src[i]); }",
        "Unknown",
        &[GATE],
    );
}

#[test]
fn a_consuming_position_keeps_the_refusal() {
    expect(
        "`x_take(r->name)`",
        "void w(struct x_rec *r) { x_take(r->name); }",
        "Unknown",
        &[GATE],
    );
}

#[test]
fn an_uncontracted_callee_keeps_the_refusal() {
    // Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (rule 12k) re-pins this
    // deliberately: the actual is one this rule reads at a `Borrowed`
    // position, so the callee is what decides and the refusal names it. The
    // statement stays refused and the status Unknown.
    expect(
        "`y_unknown(r->count)`",
        "void w(struct x_rec *r) { y_unknown(r->count); }",
        "Unknown",
        &["unsupported:call to uncontracted function `y_unknown`"],
    );
}

#[test]
fn a_chain_keeps_the_refusal() {
    // Card W47-ACCESS-PATH-POSITIONS (rule 10k) re-pins this deliberately:
    // the lexical recogniser still refuses the chain, and the typed actual
    // now admits it with its per-link obligation, which the bound record's
    // unstored `next` link cannot discharge. The gate refusal is exchanged
    // for the precise `UnknownAlias` at `r->next`; the status stays Unknown.
    expect(
        "`x_use(r->next->count)` through a bound record keeps its link obligation",
        "void w(void) { struct x_rec *r = x_record(); x_use(r->next->count); x_record_free(r); }",
        "Unknown",
        &["unknown_alias:r->next"],
    );
}

#[test]
fn a_pointer_member_address_keeps_the_refusal() {
    expect(
        "`&r->name` is a handle to a pointer slot",
        "void w(struct x_rec *r) { x_note(&r->name); }",
        "Unknown",
        &[GATE],
    );
}

#[test]
fn an_expression_size_is_not_folded() {
    expect(
        "`sizeof(*r) * r->count` keeps the refusal (W44-COLLECTION-ALLOCATION's shape), and \
         the refused binding keeps its release unresolved",
        "void w(struct x_rec *r) { char *p = x_alloc(sizeof(*r) * r->count); if (!p) return; \
         x_free(p); }",
        "Unknown",
        &[GATE, "unknown_alias:p"],
    );
}

#[test]
fn a_leak_the_call_does_not_touch_is_kept() {
    expect(
        "an unreleased acquisition beside an admitted read",
        "void w(struct x_rec *r) { char *d = x_dup(r->name); x_use(r->count); }",
        "Unsafe",
        &["leak"],
    );
}

#[test]
fn a_collection_filled_from_a_caller_array_certifies() {
    expect(
        "the brief's collection example: `v[i] = x_dup(src[i])` with partial cleanup",
        "int w(char **src, unsigned long n) { char **v; unsigned long i; unsigned long j; \
         v = x_alloc(n * 8); if (!v) return 0; \
         for (i = 0; i < n; i++) { v[i] = x_dup(src[i]); \
           if (!v[i]) { for (j = 0; j < i; j++) x_free(v[j]); x_free(v); return 0; } } \
         for (i = 0; i < n; i++) x_free(v[i]); x_free(v); return 1; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_misnamed_destination_keeps_the_refusal() {
    expect(
        "`recs[i].name = x_dup(names[i])`: the classifier names the destination `name`, a \
         suffix of the left-hand side, so the admitted actual must not admit the statement",
        "void w(char **names, int i) { struct x_rec recs[4]; char *name = 0; \
         recs[i].name = x_dup(names[i]); x_free(name); }",
        "Unknown",
        &[GATE, "unknown_alias:name"],
    );
}

/// `W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE`, found by this card and repaired
/// on main by that card (gateway constraint 5i): `recs[i].name` is a store
/// into a sub-object, refused at the lexical gate, and no longer binds the
/// local `name`. Re-pinned from `Safe` at the merge of main `70ce6b44`, to the
/// answer main's `a_member_store_no_longer_binds_the_same_named_local` pins.
#[test]
fn the_pre_existing_suffix_destination_hole_is_pinned() {
    expect(
        "`recs[i].name = x_dup(s)` no longer binds the local `name`",
        "void w(const char *s, int i) { struct x_rec recs[4]; char *name = 0; \
         recs[i].name = x_dup(s); x_free(name); }",
        "Unknown",
        &[GATE, "unknown_alias:name"],
    );
}

// -- The pre-existing leak semantics the AFTER run exposed ---------------------
//
// `W44-CALL-ARGUMENT-EXPOSED-LEAKS`: the corpus leaves the call argument rule
// exposed on its base came from three leak semantics that base already had,
// each pinned here on a body with **no** call-argument access. W44-LEAK-PIN-AUDIT's
// refused hand-on rule (on main, `legacy_refused_operands.rs`) withdraws all
// three: a live site handed on by a refused statement carries an unresolved
// effect and no leak is claimed. Re-pinned at the merge of main `70ce6b44`
// from `Unsafe` + `leak` to `Unknown` with the refusal kept; main's own
// fixtures are `tests/exposed_leak_mechanisms.rs`.

#[test]
fn an_unsanctioned_out_slot_store_is_withdrawn_by_the_hand_on_rule() {
    // Card OUT-PARAMETER-BODY-SUMMARY (rule 11r(6)): the `int` body returning
    // `-1` with the slot unwritten and `0` with it written now validates its
    // own return-code relation, so the unsanctioned store is pinned on a
    // `void` body, which no relation describes.
    expect(
        "`*out = p` with no out-parameter contract keeps `p` owed",
        "void w(char **out, unsigned long n) { char *p = x_alloc(n); if (!p) return; \
         *out = p; }",
        "Unknown",
        &["unsupported:store through `*out` without an out-parameter contract"],
    );
    expect(
        "the same store under the body's validated return-code relation",
        "int w(char **out, unsigned long n) { char *p = x_alloc(n); if (!p) return -1; \
         *out = p; return 0; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_release_through_an_uncontracted_helper_is_withdrawn_by_the_hand_on_rule() {
    expect(
        "`y_release(p)` is refused and discharges nothing",
        "void w(unsigned long n) { char *p = x_alloc(n); if (!p) return; y_release(p); }",
        "Unknown",
        &["unsupported:call to uncontracted function `y_release`"],
    );
}

#[test]
fn an_object_linked_into_a_returned_owner_is_withdrawn_by_the_hand_on_rule() {
    expect(
        "`b->name = ub; return b;` with a subscripted fill in between",
        "struct x_rec *w(char *bk, unsigned long n) { unsigned long i; char *ub; \
         struct x_rec *b; ub = x_alloc(n); if (!ub) return 0; b = x_record(); \
         b->name = ub; for (i = 0; i < n; i++) { ub[i] = bk[i]; } return b; }",
        "Unknown",
        &[
            "unknown_alias:ub",
            "unsupported:indirect memory access is outside the resource projection",
        ],
    );
}
