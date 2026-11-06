//! Card W43-INDIRECT-ACCESS-GENERAL — typed scalar accesses through pointers.
//!
//! A statement whose every memory access is a typed scalar access (`*B`,
//! `B[I]`, `B->F`, `S.F`) and whose destination and value are scalar is
//! custody-neutral. What it keeps is the dereference obligation of each base:
//! unbound on every reaching path, a non-null binding with no tracked target,
//! or exactly one known, non-null, live identity. A base released on a
//! reaching path is a `use_after_release` candidate.
//!
//! Every test asserts the **complete** diagnostic vector of `w` with its
//! status, so a rule that dropped a pre-existing leak or double release while
//! adding its own answer fails. The fixtures use a generic `x_*` surface:
//! `Safe` here is `Safe` over synthetic trusted test assumptions and promotes
//! nothing.

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
extern fn x_fresh(n: usize) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_owned(nonnull); };\n\
extern fn x_free(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(p, nullable);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_len(s: ptr<const, u8>) -> usize\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(s);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_chr(s: ptr<const, u8>, c: i32) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(s);\n\
           ensures normal resource_borrowed(c);\n\
           ensures normal resource_return_interior(s, nullable); };\n\
extern fn x_record() -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_return_owned(nonnull); };\n\
extern fn x_record_free(r: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(r, nonnull);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_note(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(p);\n\
           ensures normal resource_return_none(); };";

const DECLARATIONS: &str = "\
char *x_alloc(__SIZE_TYPE__ n); char *x_fresh(__SIZE_TYPE__ n); void x_free(void *p); \
__SIZE_TYPE__ x_len(const char *s); char *x_chr(const char *s, int c); \
struct x_rec { int count; unsigned long total; struct x_rec *next; }; \
struct x_rec *x_record(void); void x_record_free(struct x_rec *r); \
void x_note(void *p); \
enum x_kind { X_ONE = 1, X_TWO = 2 };";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "indirect_scalar_access.qcore".to_owned(),
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
            eprintln!("skipping Clang-dependent indirect-scalar-access test: {reason}");
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

fn expect(label: &str, body: &str, status: &str, vector: &[&str]) {
    if !clang_available() {
        return;
    }
    let (measured, keys) = outcome(body);
    assert_eq!(
        (
            measured,
            keys.iter().map(String::as_str).collect::<Vec<_>>()
        ),
        (status, vector.to_vec()),
        "{label}\n{body}"
    );
}

// -- Admitted: unbound bases -------------------------------------------------

#[test]
fn a_scalar_store_through_an_unbound_formal_certifies() {
    expect(
        "the out-parameter idiom `*n = v` with no out-parameter contract",
        "void w(int *n, int v) { if (n != 0) { *n = v; } }",
        "Safe",
        &[],
    );
}

#[test]
fn every_admitted_form_through_unbound_bases_certifies() {
    expect(
        "stores, compound stores, increments and reads through formals and a local array",
        "unsigned w(unsigned char *bytes, unsigned long len, struct x_rec *r, int *cnt) { \
         unsigned h = 5381; unsigned long i; char buf[16]; \
         for (i = 0; i < len; i++) h = ((h << 5) + h) + bytes[i]; \
         int counts[4]; buf[0] = 'a'; buf[1] = 0; bytes[0] ^= 0x3f; *cnt += 2; \
         counts[0] = 0; counts[0]++; --counts[0]; \
         r->count++; r->total += len; r->count = (int) r->total + X_ONE; \
         h = h + buf[0] + counts[0]; return h; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_member_of_a_record_object_is_stored_without_a_base() {
    expect(
        "`s.f = e` on a local struct object",
        "int w(unsigned long n) { struct x_rec s; s.count = (int) n; s.total = n * 2; \
         return 0; }",
        "Safe",
        &[],
    );
}

// -- Admitted: a tracked identity --------------------------------------------

#[test]
fn filling_a_guarded_allocation_certifies() {
    expect(
        "mk_string_dup's shape: allocate, null-test, fill, return",
        "char *w(const char *s) { unsigned long len; char *p; len = x_len(s); \
         p = x_alloc(len + 1); if (!p) { return 0; } p[len] = '\\0'; *p = 'x'; return p; }",
        "Safe",
        &[],
    );
}

#[test]
fn an_unguarded_nullable_allocation_is_refused() {
    expect(
        "a possibly-null base must be guarded",
        "void w(void) { char *p; p = x_alloc(4); p[0] = 0; x_free(p); }",
        "Unknown",
        &["unknown_alias:p"],
    );
}

#[test]
fn a_store_after_release_is_a_violation_candidate() {
    expect(
        "the base was released on the path reaching the store",
        "void w(void) { char *p; p = x_fresh(4); x_free(p); p[0] = 0; }",
        "Unsafe",
        &["unknown_alias:p", "use_after_release"],
    );
}

#[test]
fn a_read_after_a_release_on_one_branch_is_a_violation_candidate() {
    expect(
        "released on some reaching path: the DoubleRelease reading of the bit",
        "int w(int c) { char *p; int v; p = x_fresh(4); if (c) { x_free(p); } \
         v = *p; return v; }",
        "Unsafe",
        // The path that skips the release also leaks `p`: both are reported.
        &["leak", "unknown_alias:p", "use_after_release"],
    );
}

#[test]
fn a_member_store_through_a_released_record_is_a_violation_candidate() {
    expect(
        "`r->count += 1` after the record's release",
        "void w(void) { struct x_rec *r; r = x_record(); x_record_free(r); r->count += 1; }",
        "Unsafe",
        &["unknown_alias:r", "use_after_release"],
    );
}

// -- Derived pointers ---------------------------------------------------------

#[test]
fn a_guarded_derived_pointer_over_a_callers_string_certifies() {
    expect(
        "the strchr scanner: `*p = '\\0'` after a null test",
        "void w(char *line) { char *p; p = x_chr(line, 58); if (p == 0) { return; } \
         *p = '\\0'; }",
        "Safe",
        &[],
    );
}

#[test]
fn an_unguarded_derived_pointer_is_refused() {
    expect(
        "strchr may return NULL",
        "void w(char *line) { char *p; p = x_chr(line, 58); *p = 0; }",
        "Unknown",
        &["unknown_alias:p"],
    );
}

#[test]
fn a_derived_pointer_into_a_live_allocation_certifies() {
    expect(
        "strchr into this body's own buffer, then freed after the store",
        "void w(void) { char *b; char *p; b = x_fresh(8); p = x_chr(b, 58); \
         if (p) { *p = 0; } x_free(b); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_derived_pointer_after_its_base_is_released_is_a_violation_candidate() {
    expect(
        "`*p = 0` after freeing the base strchr scanned",
        "void w(void) { char *b; char *p; b = x_fresh(8); p = x_chr(b, 58); \
         if (p) { x_free(b); *p = 0; } else { x_free(b); } }",
        "Unsafe",
        &["unknown_alias:p", "use_after_release"],
    );
}

// -- Refused shapes stay refused ----------------------------------------------

#[test]
fn a_post_increment_through_the_pointer_stays_refused() {
    // `*p` anywhere in the body protects the formal `p`, so the pre-existing
    // formal-mutation refusal fires first; that refusal is not consulted.
    expect(
        "`*p++ = c` increments the pointer",
        "void w(char *p, char c) { *p++ = c; }",
        "Unknown",
        &["unsupported:unmodeled expression may mutate a formal resource identity"],
    );
    // Deliberate pin update, card W43-INDEX-TRAVERSAL: through a local
    // pointer, `*p++ = c` is now the typed store through the old `p` followed
    // by a modeled pointer step, and `p = buf` copies an automatic array no
    // path bound, so the copy is unbound too. The formal case above keeps the
    // formal-mutation refusal (W43-INDIRECT-PROTECTED-FORMAL-INCREMENT).
    expect(
        "`*p++ = c` through a local pointer",
        "void w(char c) { char buf[4]; char *p; p = buf; *p++ = c; }",
        "Safe",
        &[],
    );
}

/// Deliberate pin update, card CG02-ELABORATION-CORE (rule 10s): `(*n)++`
/// writes the object `n` points to, not the formal `n` (the lexical gate's
/// formal-mutation refusal read it as the latter). The elaborator reads it as
/// one typed read and one typed write of `*n`, the accesses `*n = *n + 1` has,
/// with the same base obligation and refused-write frame.
#[test]
fn an_increment_through_a_formal_pointer_is_its_typed_read_and_write() {
    expect(
        "`(*n)++` writes the pointee, not the formal",
        "void w(int *n) { (*n)++; }",
        "Safe",
        &[],
    );
    expect(
        "`*n = *n + 1`, its desugaring",
        "void w(int *n) { *n = *n + 1; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_pointer_valued_store_stays_refused() {
    expect(
        "`*pp = q` stores a pointer",
        "void w(char **pp, char *q) { *pp = q; }",
        "Unknown",
        &["unsupported:store through `*pp` without an out-parameter contract"],
    );
}

#[test]
fn a_pointer_member_store_stays_refused() {
    expect(
        "`r->next = s` stores a pointer member",
        "void w(struct x_rec *r, struct x_rec *s) { r->next = s; r->next->count = 1; }",
        "Unknown",
        // Card MEMBER-EFFECT-SUMMARY (rule 38i) re-pins this deliberately: the pointer store into a formal's member
        // names its premise; still `Unknown`.
        &[
            "unsupported:store of a custody-carrying value into `r->next`, a member of the \
           formal `r` (member-store-through-formal)",
        ],
    );
}

/// Flipped by CG05-ELABORATOR-CALL-FOOTPRINTS (rule 11n): the statement holds a
/// cast, so CG02's elaborator is asked after the refusal; the call's contract
/// footprint (`x_len` borrows `s`) commutes with the unordered read of `p`,
/// and the call is applied through the one application point. The store is
/// sequenced after the call, so even a call borrowing the destination's own
/// base is ordered with it (the refusal controls are in
/// `cg05_elaborator_call_footprints.rs`).
#[test]
fn a_call_in_the_statement_is_applied_through_its_footprint() {
    expect(
        "the value is a call's result",
        "void w(char *p, const char *s) { p[0] = (char) x_len(s); }",
        "Safe",
        &[],
    );
    expect(
        "the store follows the call that borrows its base",
        "void w(char *p) { p[0] = (char) x_len(p); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_subscript_index_can_read_a_second_live_scalar_path() {
    expect(
        "`p[q[0]] = 1`: both untracked bases have typed read obligations",
        "void w(char *p, int *q) { p[q[0]] = 1; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_released_pointer_read_in_a_subscript_index_is_a_violation_candidate() {
    expect(
        "the index read must discharge its own liveness obligation",
        "void w(char *p) { char *q; q = x_fresh(1); x_free(q); p[q[0]] = 1; }",
        "Unsafe",
        &["unknown_alias:q", "use_after_release"],
    );
}

// -- The frame ----------------------------------------------------------------

#[test]
fn a_byte_store_through_a_pointer_poisons_bindings_outside_its_frame() {
    // `&r->count` takes an address inside `r`'s name, so `r` is outside the
    // frame: a byte store through the unbound `c` could rewrite `r`'s slot,
    // and the release that follows cannot be credited.
    expect(
        "a character store keeps the refusal's frame on bindings",
        "void w(char *c) { struct x_rec *r; r = x_record(); x_note(&r->count); *c = 0; \
         x_record_free(r); }",
        "Unknown",
        &["unknown_alias:r"],
    );
}

#[test]
fn the_same_body_without_the_store_certifies() {
    expect(
        "the control for the frame test: only the store differs",
        "void w(char *c) { struct x_rec *r; r = x_record(); x_note(&r->count); \
         x_record_free(r); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_member_store_keeps_every_binding() {
    expect(
        "a scalar member store writes one member and nothing else",
        "void w(struct x_rec *s) { struct x_rec *r; r = x_record(); x_note(&r->count); \
         s->count = (int) s->total; x_record_free(r); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_protected_binding_survives_a_byte_store() {
    expect(
        "a never-address-taken local is outside every pointer's reach",
        "void w(char *c) { char *q; q = x_fresh(4); *c = 0; x_free(q); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_leak_the_store_does_not_touch_stays_visible() {
    expect(
        "an admitted store keeps unrelated residual evidence",
        "void w(char *c) { char *q; q = x_fresh(4); *c = 0; }",
        "Unsafe",
        &["leak"],
    );
}
