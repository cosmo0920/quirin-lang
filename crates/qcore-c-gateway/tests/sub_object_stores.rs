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

//! Card W45-SUB-OBJECT-STORES — a store into a sub-object through a resolved,
//! typed base (`crates/qcore-c-gateway/src/legacy_sub_object_store.rs`).
//!
//! Gateway constraint 5i refuses every store whose left-hand side is a
//! sub-object at the lexical gate, and W43's typed route re-admits only the
//! scalar `p->f = e` / `s.f = e` with no call. This card admits the other
//! sound shapes through a base the declaration reader resolved: a scalar or
//! untracked value is custody-neutral, and every store of a tracked identity
//! (owned, a formal's, a descriptor) keeps the refusal — which still hands its
//! identities on.
//!
//! Every test asserts the **complete** diagnostic vector of `w` with its
//! status, and every `Safe` certificate is rechecked. The fixtures use a
//! generic `x_*`/`k_*` surface: `Safe` here is `Safe` over synthetic trusted
//! test assumptions and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = "\
module contract;\n\
extern fn x_dup(s: ptr<const, u8>) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(s);\n\
           ensures normal resource_return_owned(nullable); };\n\
extern fn x_free(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(p, nullable);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_len(s: ptr<const, u8>) -> usize\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(s);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_name(s: ptr<const, u8>) -> ptr<const, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(s);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_record() -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_return_owned(nonnull); };\n\
extern fn x_record_free(r: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(r, nonnull);\n\
           ensures normal resource_return_none(); };\n\
extern fn k_open(path: ptr<const, u8>) -> i32\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(path);\n\
           ensures normal resource_return_owned(sentinel);\n\
           ensures normal resource_return_kind(descriptor); };\n\
extern fn k_close(fd: i32) -> i32\n\
contract { requires resource_contract_v1; ensures normal resource_released(fd, valid);\n\
           ensures normal resource_kind(fd, descriptor); ensures normal resource_return_none(); };";

const DECLARATIONS: &str = "\
char *x_dup(const char *s); const char *x_name(const char *s); void x_free(void *p); __SIZE_TYPE__ x_len(const char *s); \
struct x_rec { int count; char *name; struct x_rec *next; }; \
struct x_holder { struct x_rec *rec; char *name; int fd; }; \
struct x_rec *x_record(void); void x_record_free(struct x_rec *r); \
int k_open(const char *path); int k_close(int fd); void y_unknown(int n);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "sub_object_stores.qcore".to_owned(),
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
            eprintln!("skipping Clang-dependent sub-object store test: {reason}");
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

// -- Scalar and untracked values: custody-neutral ----------------------------

#[test]
fn a_scalar_store_through_a_parenthesised_base_is_admitted() {
    expect(
        "`(r)->count = 1` is `r->count = 1`",
        "void w(struct x_rec *r) { (r)->count = 1; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_scalar_store_through_the_address_of_a_record_is_admitted() {
    expect(
        "`(&l)->count = n` stores into this function's own record",
        "void w(int n) { struct x_rec l; (&l)->count = n; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_scalar_store_into_an_element_member_is_admitted() {
    expect(
        "`recs[i].count = 0` through a `struct x_rec *` formal",
        "void w(struct x_rec *recs, int i) { recs[i].count = 0; }",
        "Safe",
        &[],
    );
}

#[test]
fn a_null_store_into_a_pointer_member_is_admitted() {
    expect(
        "`r->name = NULL` stores no identity",
        "void w(struct x_rec *r) { r->name = ((void *) 0); r->next = 0; }",
        "Safe",
        &[],
    );
}

#[test]
fn an_untracked_local_pointer_stored_into_a_member_is_custody_neutral() {
    expect(
        "`h->name = t` with `t` bound to no identity",
        "void w(struct x_holder *h) { char *t = 0; h->name = t; }",
        "Safe",
        &[],
    );
}

/// A formal's pointer is the caller's object. Storing it into a record hands
/// that object on in a way no summary mode states: a derived summary would
/// call the formal borrowed, and a caller would then claim a leak of an
/// object its callee linked into a list (`cfl_list_add`, `mk_list_init`).
/// So the store keeps its refusal.
#[test]
fn a_formal_pointer_stored_into_a_member_keeps_its_refusal() {
    // CG11 flip (rule 10v, CG11-POINTER-VALUED-STORES): `r->next = r` stores
    // the destination's own object into it, which moves no custody; the body
    // is admitted and publishes no summary, so no caller reads `r` as
    // borrowed (the `mk_list_init` hazard this pin guarded). BEFORE: Unknown
    // with the gate's refusal.
    expect(
        "`r->next = r` stores the caller's own object into itself",
        "void w(struct x_rec *r) { r->next = r; }",
        "Safe",
        &[],
    );
    expect(
        "`h->name = (char *) s` stores the caller's string",
        "void w(struct x_holder *h, const char *s) { h->name = (char *) s; }",
        "Unknown",
        &[GATE],
    );
}

#[test]
fn a_none_result_call_stored_into_a_scalar_member_keeps_its_refusal() {
    // **Deliberate pin update, card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round
    // 2** (rule 14a): this was `Unknown` with the gate's refusal. A scalar
    // statement whose value is one call to a `None`-result, all-`Borrowed`
    // contract is the call, then the typed scalar store, so the store is read.
    expect(
        "`r->count = x_len(s)`",
        "void w(struct x_rec *r, const char *s) { r->count = x_len(s); }",
        "Safe",
        &[],
    );
    // Two calls in the value keep the refusal (C leaves their order open).
    expect(
        "`r->count = x_len(s) + x_len(s)`",
        "void w(struct x_rec *r, const char *s) { r->count = x_len(s) + x_len(s); }",
        "Unknown",
        &[GATE],
    );
}

// -- Owned identities: the refusal stands and hands them on --------------------

/// The brief's counterexample 3: an owned pointer stored into a record the
/// caller reaches escapes this function's leak accounting — the refusal hands
/// `n` on, so no leak is claimed — but it does not make the function `Safe`:
/// the caller now holds an obligation no summary mode states, so a caller's
/// `Safe` would rest on a transfer it cannot see. This is the decision O04-B08
/// (`a_store_into_a_borrowed_in_aggregate_is_never_admitted`) and
/// W44-LEAK-PIN-AUDIT (`a_refused_store_of_the_identity_withdraws_the_leak`)
/// pin, and this card keeps it.
#[test]
fn an_owned_pointer_stored_into_a_caller_visible_record_escapes_the_leak_claim_only() {
    expect(
        "`h->name = n` hands `n` to the caller's record",
        "void w(struct x_holder *h, const char *s) { char *n = x_dup(s); h->name = n; }",
        "Unknown",
        &[GATE],
    );
}

/// An owned call result is never read as a stored value: the call keeps the
/// gate's refusal whole, so no effect of it applies and nothing is claimed.
#[test]
fn an_owned_call_result_stored_into_a_caller_visible_record_keeps_its_refusal() {
    expect(
        "`h->name = x_dup(s)`",
        "void w(struct x_holder *h, const char *s) { h->name = x_dup(s); }",
        "Unknown",
        // Card MEMBER-EFFECT-SUMMARY (rule 38i) re-pins this deliberately: the owned result stored into a formal's
        // member names its premise; still `Unknown`.
        &[
            "unsupported:store of a custody-carrying value into `h->name`, a member of the \
           formal `h` (member-store-through-formal)",
        ],
    );
}

/// A call whose result carries no custody is applied, and its untracked
/// result stored: custody-neutral.
#[test]
fn a_none_result_call_stored_into_a_pointer_member_is_admitted() {
    expect(
        "`h->name = x_name(s)` with a `None` result",
        "void w(struct x_holder *h, const char *s) { h->name = (char *) x_name(s); }",
        "Safe",
        &[],
    );
}

/// A release after the store is not `Safe` and not an accusation: the store
/// keeps its refusal, which hands `n` on.
#[test]
fn releasing_an_identity_after_storing_it_into_the_callers_record_is_not_safe() {
    expect(
        "`h->name = n; x_free(n);`",
        "void w(struct x_holder *h, const char *s) { char *n = x_dup(s); h->name = n; \
         x_free(n); }",
        "Unknown",
        &[GATE],
    );
}

/// An owned identity stored into this function's own record object keeps the
/// refusal, which hands `n` on, so no leak is claimed for it.
#[test]
fn an_owned_pointer_stored_into_a_local_record_keeps_its_refusal_and_hands_on() {
    expect(
        "`l.name = n` into a local record",
        "void w(const char *s) { struct x_holder l; char *n = x_dup(s); l.name = n; }",
        "Unknown",
        &[GATE],
    );
}

#[test]
fn an_owned_call_result_stored_into_a_local_record_keeps_its_refusal() {
    expect(
        "`l.name = x_dup(s)` into a local record",
        "void w(const char *s) { struct x_holder l; l.name = x_dup(s); }",
        "Unknown",
        &[GATE],
    );
}

// -- The brief's soundness counterexamples ------------------------------------

/// Counterexample 1: `recs[i].name = x_dup(s); x_free(name);` stays non-Safe.
/// `recs` is a local array: the store's owned call keeps the gate's refusal
/// and binds no `name`.
#[test]
fn the_misnamed_member_store_stays_refused() {
    expect(
        "`recs[i].name = x_dup(s)` through a local array",
        "void w(const char *s, int i) { struct x_rec recs[4]; char *name = 0; \
         recs[i].name = x_dup(s); x_free(name); }",
        "Unknown",
        &[GATE, "unknown_alias:name"],
    );
}

/// The same store through a caller's array binds no local either.
#[test]
fn the_member_store_through_a_callers_array_binds_no_local() {
    expect(
        "`recs[i].name = x_dup(s); x_free(name);` through a formal",
        "void w(struct x_rec *recs, const char *s, int i) { char *name; \
         recs[i].name = x_dup(s); x_free(name); }",
        "Unknown",
        &[GATE, "unknown_alias:name"],
    );
}

/// Counterexample 2: a descriptor stored into a member stays custody. The
/// statement reads a tracked descriptor, so this rule declines it; W43's
/// typed scalar route reads it as data and the descriptor guard refuses that,
/// exactly as before the card; the refused hand-on rule withdraws the leak
/// claim, so `fd` is neither leaked nor discharged.
#[test]
fn a_descriptor_stored_into_a_member_stays_custody() {
    expect(
        "`h->fd = fd` with `fd` a tracked descriptor",
        "void w(struct x_holder *h, const char *p) { int fd = k_open(p); \
         if (fd < 0) { return; } h->fd = fd; }",
        "Unknown",
        &[
            "unsupported:tracked descriptor `fd` read by a custody-neutral scalar route is \
           outside the resource projection",
        ],
    );
}

/// The same descriptor through a parenthesised base, which only this rule
/// reads: the rule declines it and the gate's refusal stands.
#[test]
fn a_descriptor_stored_through_a_parenthesised_base_keeps_the_gate() {
    expect(
        "`(h)->fd = fd` with `fd` a tracked descriptor",
        "void w(struct x_holder *h, const char *p) { int fd = k_open(p); \
         if (fd < 0) { return; } (h)->fd = fd; }",
        "Unknown",
        &[
            "unsupported:tracked descriptor `fd` read by a custody-neutral scalar route is outside the resource projection",
        ],
    );
}

/// An ordinary integer into the same member is data.
#[test]
fn an_integer_stored_into_the_same_member_is_data() {
    expect(
        "`h->fd = n`",
        "void w(struct x_holder *h, int n) { h->fd = n; }",
        "Safe",
        &[],
    );
}

// -- Flow premises ------------------------------------------------------------

/// A store through a base released on the path keeps the refusal.
#[test]
fn a_store_through_a_released_base_keeps_its_refusal() {
    expect(
        "`x_record_free(r); (r)->count = 1;`",
        "void w(void) { struct x_rec *r = x_record(); x_record_free(r); (r)->count = 1; }",
        "Unknown",
        &[GATE],
    );
}

/// A pointer store into a member of an aggregate this body tracks may
/// overwrite a field link; only O04-B08's aggregate store may rewrite one.
#[test]
fn a_pointer_store_through_a_tracked_base_keeps_its_refusal() {
    expect(
        "`(r)->name = (char *) s` with `r` tracked",
        "void w(const char *s) { struct x_rec *r = x_record(); (r)->name = (char *) s; \
         x_record_free(r); }",
        "Unknown",
        &[GATE],
    );
}

/// An untyped or unresolved base keeps its refusal on this card's route.
///
/// CG12 flip (rule 11j): a `void *` formal viewed as `struct x_rec *` is read
/// by the typed path route (an untracked root, a scalar store), after this
/// route declined. BEFORE: Unknown with the gate's refusal. A view of a
/// tracked `void *` keeps the refusal.
#[test]
fn a_cast_base_keeps_its_refusal() {
    expect(
        "`((struct x_rec *) p)->count = 1`",
        "void w(void *p) { ((struct x_rec *) p)->count = 1; }",
        "Safe",
        &[],
    );
    expect(
        "a view of a tracked `void *`",
        "void w(void) { void *p = x_record(); ((struct x_rec *) p)->count = 1; x_record_free(p); }",
        "Unknown",
        &["unknown_alias:p"],
    );
}

#[test]
fn a_member_chain_keeps_its_refusal() {
    // CG11 flip (rule 10v): a null store into a chain's member through a
    // never-bound root is read on the typed statement route, the chain's
    // owner untracked. BEFORE: Unknown with the gate's refusal. This rule's
    // own reader still refuses the chain (`read_destination`).
    expect(
        "`h->rec->name = 0` is a chain",
        "void w(struct x_holder *h) { h->rec->name = 0; }",
        "Safe",
        &[],
    );
}
