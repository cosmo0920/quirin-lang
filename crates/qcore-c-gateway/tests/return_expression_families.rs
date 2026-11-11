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

//! Card **W45-RETURN-EXPRESSION-FAMILIES**: the scalar-result read (S) and the
//! cast of a whole call (K) in `return` position, end to end through the real
//! gateway.
//!
//! Every row asserts the status and the **complete** set of diagnostic keys,
//! so a rule that silently dropped an accusation fails. Three kinds of row:
//!
//! * admissions — the family's shapes the return position used to refuse,
//!   now decided (`Safe` over the synthetic contracts), and the owned result of
//!   a pointer-cast call escaping exactly its fresh site;
//! * counterexamples — the returned value's custody stated exactly: a read
//!   through a released object is accused, an owned object read and not
//!   returned leaks, an interior pointer of a released object and a derived
//!   pointer under a cast keep the refusal, and a caller that frees what a
//!   derived wrapper returns is never certified;
//! * boundaries — every shape outside the families keeps the return-position
//!   refusal verbatim.
//!
//! The fixtures use the generic `k_*` surface of
//! `fixtures/contracts/descriptor_generic.qcore` and the `d_*` surface of
//! `fixtures/contracts/derived_result_generic.qcore`; `Safe` here is `Safe`
//! over those synthetic trusted test assumptions.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
int k_open(const char *path); \
int k_close(int fd); \
void *k_alloc(__SIZE_TYPE__ size); \
void k_free(void *p); \
void *d_alloc(__SIZE_TYPE__ n); \
void d_free(void *p); \
__SIZE_TYPE__ d_len(const char *s); \
char *d_chr(const char *s, int c); \
int z_unknown(int n); \
struct k_box { void *item; long n; }; \
typedef struct j_item { int type; struct j_item *next; } j_item; \
typedef enum { K_READ, K_WRITE } k_kind_t; \
typedef struct k_acl { k_kind_t op; int code; int fatal; } k_acl_t; \
typedef union k_state { long table[8]; void *p; } k_state_t; \
struct k_u16 { unsigned short v; }; \
struct k_named { char name[16]; long n; };";

const RETURN: &str =
    "unsupported:unmodeled call, aggregate or indirect access in return expression";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        (
            "descriptor_generic.qcore".to_owned(),
            include_str!("fixtures/contracts/descriptor_generic.qv").to_owned(),
        ),
        (
            "derived_result_generic.qcore".to_owned(),
            include_str!("fixtures/contracts/derived_result_generic.qv").to_owned(),
        ),
    ]))
    .expect("the fixtures load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent return-family test: {reason}");
            false
        }
    }
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

fn key(kind: &LegacyDiagnosticKind) -> String {
    match kind {
        LegacyDiagnosticKind::UnknownAlias { variable } => format!("unknown_alias:{variable}"),
        LegacyDiagnosticKind::Leak { .. } => "leak".to_owned(),
        LegacyDiagnosticKind::Unsupported { description } => format!("unsupported:{description}"),
        other => format!("{other:?}")
            .split([' ', '{'])
            .next()
            .unwrap_or("")
            .to_owned(),
    }
}

/// The status of `w` and the sorted, de-duplicated keys of its diagnostics.
fn outcome(body: &str) -> (&'static str, Vec<String>) {
    let source = format!("{DECLARATIONS} {body}");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts(),
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
    let function = program(&result).functions.get("w").expect("w analysed");
    if status == "Safe" {
        assert_eq!(function.status, AnalysisStatus::Safe);
    }
    let mut keys = function
        .diagnostics
        .iter()
        .map(|diagnostic| key(&diagnostic.kind))
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    (status, keys)
}

fn expect(label: &str, body: &str, status: &str, vector: &[&str]) {
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

// -- S: the scalar-result read ------------------------------------------------

/// The corpus's accessor shapes, each refused by the return position for its
/// operand's typing (an unreadable `* const` base, an enumeration a typedef
/// defines, a member through a cast, a record `sizeof`), now decided: the
/// value is a scalar and carries nothing, and every name read is untracked.
#[test]
fn a_scalar_result_read_over_untracked_names_is_decided() {
    if !clang_available() {
        return;
    }
    for body in [
        "int w(const j_item * const item) { return (item->type & 0xFF) == (1 << 5); }",
        "k_kind_t w(const k_acl_t *acl) { return acl->op; }",
        "int w(const k_acl_t *e) { return e && e->fatal ? 1 : 0; }",
        "int w(const k_acl_t *e) { return e ? e->code : 0; }",
        "unsigned short w(const void *ptr) { return ((const struct k_u16 *) ptr)->v; }",
        "int w(void) { return (int) sizeof(k_state_t); }",
        "int w(const j_item * const item) { return item->next->next->type > 0; }",
    ] {
        expect("scalar read", body, "Safe", &[]);
    }
}

/// A read through a name the body binds is the root of one dereference C
/// always evaluates, and keeps the typed route's obligation: on a live
/// identity the read is admitted — and the object, which the scalar result
/// does not carry away, is still owed (a true leak) — and on a released one it
/// is a `use_after_release` candidate.
#[test]
fn a_bound_base_keeps_its_dereference_obligation() {
    if !clang_available() {
        return;
    }
    expect(
        "live owned base",
        "long w(void) { struct k_box *b = k_alloc(1); if (!b) return 0; return b->n + 1; }",
        "Unsafe",
        &["leak"],
    );
    expect(
        "released base",
        "long w(void) { struct k_box *b = k_alloc(1); if (!b) return 0; k_free(b); return b->n; }",
        "Unsafe",
        &["UseAfterRelease", "unknown_alias:b"],
    );
}

/// Counterexample: C reads `b->n` only when `f` is set, so an accusation on
/// the other path would be one C does not have. This family's route keeps
/// the refusal for a bound base in a skipped position.
///
/// Re-pinned deliberately by W47-ACCESS-PATH-POSITION-RESIDUALS (rule 10n):
/// the connective return asked after this route desugars `||` and `?:` as C
/// evaluates them, so an operand is read only on the edges where C reads it.
/// `*str == '-'` is read only where `*str == '+'` is false, each read a hop on
/// the unbound formal: `Safe` (the corpus's `is_incremental`). `b->n` is read
/// only on the edge where `f` is set, which is after `k_free(b)`: the
/// use-after-release C has on that path, and no accusation on the other.
#[test]
fn a_bound_base_in_a_skipped_position_is_read_only_where_c_reads_it() {
    if !clang_available() {
        return;
    }
    expect(
        "conditional dereference of a protected formal",
        "int w(const char *str) { return (*str == '+' || *str == '-'); }",
        "Safe",
        &[],
    );
    expect(
        "conditional bound base",
        "long w(int f) { struct k_box *b = k_alloc(1); if (!b) return 0; k_free(b); \
         return f ? b->n : 0; }",
        "Unsafe",
        &["UseAfterRelease", "unknown_alias:b"],
    );
}

/// Counterexample: a pointer-valued result is not a scalar. `n->name` is an
/// interior address of `*n` (an array member's decay); returning it after
/// `n` is released, or at all, keeps the refusal — it is never published as a
/// result that carries nothing.
#[test]
fn a_pointer_result_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    expect(
        "interior pointer of a released object",
        "char *w(void) { struct k_named *n = k_alloc(32); if (!n) return 0; k_free(n); \
         return n->name; }",
        "Unknown",
        &[RETURN],
    );
    expect(
        "interior pointer of a borrowed object",
        "const char *w(const struct k_named * const n) { return n->name; }",
        "Unknown",
        &[RETURN],
    );
}

/// The shapes outside the family keep the return-position refusal: a call in
/// the expression, a tracked descriptor read as data, an address.
#[test]
fn every_other_scalar_shape_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    expect(
        "a call inside the expression",
        "int w(const j_item * const item) { return z_unknown(item->type) + 1; }",
        "Unknown",
        &[RETURN],
    );
    expect(
        "an address",
        "int w(const j_item * const item) { return &item->type != 0; }",
        "Unknown",
        &[RETURN],
    );
}

/// A tracked descriptor is custody, never data: an expression that mentions
/// one keeps the refusal (whose hand-on withdraws the descriptor's leak, as
/// W44-LEAK-PIN-AUDIT's rule does for every refused statement naming it).
#[test]
fn a_descriptor_read_as_data_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    // Without the descriptor the same expression is the family's.
    expect(
        "no descriptor",
        "int w(const struct k_box *b, int fd) { return fd >= 0 ? (int) b->n : 0; }",
        "Safe",
        &[],
    );
    expect(
        "a tracked descriptor",
        "int w(const struct k_box *b, const char *path) { int fd = k_open(path); \
         return fd >= 0 ? (int) b->n : 0; }",
        "Unknown",
        &[RETURN],
    );
}

/// The accessor's summary: a caller that reads through it and then releases
/// is decided; a caller that releases first and then reads through it is not
/// certified — the accessor borrows its formal.
#[test]
fn an_accessor_validates_a_borrowing_summary() {
    if !clang_available() {
        return;
    }
    expect(
        "read then release",
        "static long acc(const struct k_box * const b) { return b->n > 0; } \
         long w(void) { struct k_box *b = k_alloc(1); long r; if (!b) return 0; \
         r = acc(b); k_free(b); return r; }",
        "Safe",
        &[],
    );
    let (status, keys) = outcome(
        "static long acc(const struct k_box * const b) { return b->n > 0; } \
         long w(void) { struct k_box *b = k_alloc(1); long r; if (!b) return 0; \
         k_free(b); r = acc(b); return r; }",
    );
    assert_ne!(status, "Safe", "release then read: {keys:?}");
}

// -- K: a cast of a whole call ------------------------------------------------

/// An owned result under a pointer cast is the uncast return: the conversion
/// keeps the object, so the exit escapes exactly the fresh site.
#[test]
fn a_pointer_cast_of_an_owned_call_escapes_its_fresh_site() {
    if !clang_available() {
        return;
    }
    let plain = outcome("void *w(void) { return k_alloc(1); }");
    assert_eq!(plain, ("Safe", vec![]));
    for body in [
        "void *w(void) { return (char *) k_alloc(1); }",
        "struct k_box *w(void) { return (struct k_box *) (void *) k_alloc(1); }",
        "void *w(void) { return (k_alloc(1)); }",
    ] {
        assert_eq!(outcome(body), plain, "{body}");
    }
    // A caller that frees what the wrapper returns is decided; one that drops
    // it is accused.
    expect(
        "caller frees",
        "static struct k_box *make(void) { return (struct k_box *) k_alloc(1); } \
         void w(void) { struct k_box *b = make(); k_free(b); }",
        "Safe",
        &[],
    );
    expect(
        "caller drops",
        "static struct k_box *make(void) { return (struct k_box *) k_alloc(1); } \
         void w(void) { struct k_box *b = make(); (void) b; }",
        "Unsafe",
        &["leak"],
    );
}

/// An owned result under an integer cast crosses carriers and is refused;
/// a result that carries nothing may be converted to anything.
#[test]
fn only_a_pointer_cast_of_an_owned_call_is_admitted() {
    if !clang_available() {
        return;
    }
    expect(
        "integer cast of an owned call",
        "long w(void) { return (long) k_alloc(1); }",
        "Unknown",
        &[RETURN],
    );
    expect(
        "integer cast of a result that carries nothing",
        "long w(const char *s) { return (long) d_len(s); }",
        "Safe",
        &[],
    );
}

/// A refused call under a cast is lowered to the classifier's own refusal,
/// as W43 lowers the uncast one: re-attribution, never admission.
#[test]
fn a_refused_call_under_a_cast_names_its_own_blocker() {
    if !clang_available() {
        return;
    }
    let cast = outcome("long w(int n) { return (long) z_unknown(n); }");
    let uncast = outcome("long w(int n) { return z_unknown(n); }");
    assert_eq!(cast, uncast);
    assert_ne!(cast.0, "Safe");
    assert!(!cast.1.iter().any(|key| key == RETURN), "{cast:?}");
}

/// Counterexample: a derived result under a cast keeps the refusal, so a
/// wrapper returning it validates no summary and a caller that frees the
/// derived pointer is never certified. The uncast wrapper validates a derived
/// summary, and the same caller is accused.
#[test]
fn a_derived_result_under_a_cast_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    expect(
        "derived under a cast",
        "char *w(char *s) { return (char *) d_chr(s, 'x'); }",
        "Unknown",
        &[RETURN],
    );
    let cast = outcome(
        "static char *find(char *s) { return (char *) d_chr(s, 'x'); } \
         void w(void) { char *s = d_alloc(8); char *r; if (!s) return; r = find(s); \
         d_free(r); d_free(s); }",
    );
    assert_ne!(cast.0, "Safe", "{cast:?}");
    let uncast = outcome(
        "static char *find(char *s) { return d_chr(s, 'x'); } \
         void w(void) { char *s = d_alloc(8); char *r; if (!s) return; r = find(s); \
         d_free(r); d_free(s); }",
    );
    assert_ne!(uncast.0, "Safe", "{uncast:?}");
}
