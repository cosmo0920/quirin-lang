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

//! Card W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE — an assignment's destination
//! is named only when the whole left-hand side is that place, or the tokens
//! before it are a declaration's specifiers and declarator stars.
//!
//! Before the card `direct_assignment_destination` named the first suffix of
//! the left-hand side that spells a place, so `recs[i].name = x_dup(s)` bound
//! the acquisition to the local `name` and `x_free(name)` "released" it: a
//! `Safe` for C that leaks one object and frees another. Every other left-hand
//! side that had such a suffix is a store into a sub-object, refused by the
//! lexical indirect-access gate (the typed scalar routes are consulted after
//! that refusal as for any other `a.b = …`).
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
void *x_alloc(unsigned long n); char *x_dup(const char *s); void x_free(void *p); \
void x_take(void *p); void x_use(int n); void x_note(void *p); \
void x_log(const char *fmt, ...); \
struct x_lock { int held; }; \
struct x_rec { int count; char *name; struct x_rec *next; struct x_lock lock; }; \
struct x_holder { struct x_rec *rec; char *name; }; \
struct x_rec *x_record(void); struct x_rec *x_maybe_record(void); \
void x_record_free(struct x_rec *r); void y_unknown(int n); void y_release(void *p);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "assignment_destination_suffix.qcore".to_owned(),
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
            eprintln!("skipping Clang-dependent destination-suffix test: {reason}");
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

// -- The hole, closed ----------------------------------------------------------

/// The refusal's write frame poisons `name` (`may_write_names` reads
/// `name =` as a write, the over-approximating direction), so the release of
/// the local records `unknown_alias`.
///
/// The card's fixture, pinned `Safe` at the base by card
/// W44-INDIRECT-CALL-ARGUMENT-ACCESS (`the_pre_existing_suffix_destination_hole_is_pinned`):
/// the acquisition is stored into `recs[i].name` and `x_free(name)` releases
/// the local, a different object.
#[test]
fn a_member_store_no_longer_binds_the_same_named_local() {
    expect(
        "`recs[i].name = x_dup(s)` is a store into a sub-object, not a write of `name`",
        "void w(const char *s, int i) { struct x_rec recs[4]; char *name = 0; \
         recs[i].name = x_dup(s); x_free(name); }",
        "Unknown",
        &[GATE, "unknown_alias:name"],
    );
}

/// The same store with a same-named formal: the formal is not the destination.
#[test]
fn a_member_store_no_longer_binds_the_same_named_formal() {
    expect(
        "`recs[i].name = x_dup(s)` does not write the formal `name`",
        "void w(const char *s, int i, char *name) { struct x_rec recs[4]; \
         recs[i].name = x_dup(s); x_free(name); }",
        "Unknown",
        &[GATE, "unknown_alias:name"],
    );
}

/// A misnamed store with no same-named object at all used to bind a name
/// nothing declares, and so reported a leak of it: the old accusation was
/// about a name the C does not have, and it is withdrawn with the refusal.
#[test]
fn a_member_store_with_no_named_object_is_refused_not_accused() {
    expect(
        "`recs[i].name = x_dup(s)` with no `name` in scope",
        "void w(const char *s, int i) { struct x_rec recs[4]; recs[i].name = x_dup(s); }",
        "Unknown",
        &[GATE],
    );
}

/// `(*pp)->name`: the old reader named `name` (the first place-spelling suffix
/// follows `)` `-` `>`).
#[test]
fn a_parenthesised_dereference_store_is_refused() {
    expect(
        "`(*pp)->name = x_dup(s)` is a store through a pointer",
        "void w(const char *s, struct x_rec **pp) { char *name = 0; \
         (*pp)->name = x_dup(s); x_free(name); }",
        "Unknown",
        &[GATE, "unknown_alias:name"],
    );
}

/// A nested member `hs[i].rec` names `rec`; a local `rec` bound by it made
/// O04-B08's aggregate store and the release of `rec` certify. The aggregate
/// premise reader counts `hs[i].rec = …` as `rec`'s one acquired binding (its
/// syntactic premise 3); the flow check the store rests on is what refuses
/// once the misnamed binding is gone.
#[test]
fn a_misnamed_binding_no_longer_feeds_the_aggregate_store() {
    expect(
        "`hs[i].rec = x_record()` does not bind the local `rec`",
        "void w(int i) { struct x_holder hs[2]; struct x_rec *rec; \
         hs[i].rec = x_record(); rec->count = 1; x_record_free(rec); }",
        "Unknown",
        &[GATE, "unknown_alias:rec"],
    );
}

// -- Unchanged: the whole left-hand side, or a declaration ---------------------

#[test]
fn a_pointer_declaration_is_still_named() {
    expect(
        "`char * name = x_dup(s)` declares and binds `name`",
        "void w(const char *s) { char *name = x_dup(s); x_free(name); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_qualified_declaration_is_still_named() {
    expect(
        "`struct x_rec * const r = x_record()` declares and binds `r`",
        "void w(void) { struct x_rec * const r = x_record(); x_record_free(r); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_second_declarator_is_still_named() {
    expect(
        "`char * a , * name = x_dup(s)`: the comma run is a declarator prefix",
        "void w(const char *s) { char *a, *name = x_dup(s); x_free(name); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_plain_assignment_is_still_named() {
    expect(
        "`name = x_dup(s)` writes the local",
        "void w(const char *s) { char *name; name = x_dup(s); x_free(name); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_whole_member_place_keeps_the_aggregate_store() {
    expect(
        "`r->name = x_dup(s)` is the whole left-hand side, O04-B08's store",
        "void w(const char *s) { struct x_rec *r = x_record(); r->name = x_dup(s); \
         x_free(r->name); x_record_free(r); }",
        "Safe",
        &[],
    );
}

// -- Composition with descriptor custody ---------------------------------------

const DESCRIPTOR_CONTRACTS: &str = include_str!("fixtures/contracts/descriptor_generic.qv");

const DESCRIPTOR_DECLARATIONS: &str = "\
int k_open(const char *path); int k_close(int fd); \
struct k_conn { int fd; int flags; };";

fn descriptor_outcome(body: &str) -> (&'static str, Vec<String>) {
    let contracts = load_resource_contract_sources(&BTreeMap::from([(
        "descriptor_generic.qcore".to_owned(),
        DESCRIPTOR_CONTRACTS.to_owned(),
    )]))
    .expect("the descriptor fixture loads");
    let source = format!("{DESCRIPTOR_DECLARATIONS}\n{body}");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts,
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"));
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
            LegacyDiagnosticKind::Unsupported { description } => {
                format!("unsupported:{description}")
            }
            LegacyDiagnosticKind::UnknownAlias { variable } => format!("unknown_alias:{variable}"),
            other => format!("{other:?}"),
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    (status, keys)
}

/// `conns[fd].flags = 0` reads the tracked descriptor `fd` as an index. At the
/// base the destination was named `flags`, so the store lowered to a clear of a
/// name the C does not have, the descriptor guard read `flags` as a plain-name
/// write and missed the index read, and the body certified `Safe` (measured).
/// The store is now refused by the lexical gate; were a typed route to admit
/// it, the guard now sees the read (the unit test
/// `an_index_read_under_a_member_store_is_a_read`).
#[test]
fn a_descriptor_read_as_an_index_of_a_member_store_is_seen() {
    if !clang_available() {
        return;
    }
    let body = "void w(struct k_conn *conns) { int fd = k_open(\"p\"); if (fd < 0) return; \
                conns[fd].flags = 0; k_close(fd); }";
    let (status, keys) = descriptor_outcome(body);
    assert_eq!(
        (status, keys.clone()),
        (
            "Unknown",
            vec!["unsupported:tracked descriptor `fd` read by a custody-neutral scalar route is outside the resource projection".to_owned()],
        ),
        "{keys:?}\n{body}"
    );
}
