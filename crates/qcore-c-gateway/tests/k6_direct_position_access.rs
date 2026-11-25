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

//! Card **K6-DIRECT-POSITION-ACCESS** (gateway rule 81), end to end through
//! the real gateway: a member of an embedded record spelled through the
//! record's address, `(&p->q)->f` (librdkafka's `TAILQ_FIRST(&p->q)`,
//! `RD_KAFKAP_BYTES_LEN(&m->bytes)`, `RD_KAFKAP_STR_LEN(&p->a[i].Host)`), is
//! read as its plain spelling `p->q.f` by the lexical `may_bind` reader (a
//! pointer return) and by the elaborator's place reader (a rule 10z edge).
//!
//! Every row asserts the status and the complete set of diagnostic keys, and
//! every near-miss is checked beside its plain spelling: the rule claims the
//! plain spelling's reading, never more. `Safe` is `Safe` over the synthetic
//! `k_*` / `d_*` test contracts, with a certificate that rechecks. The bases
//! are typedef-spelled (`mp_t *`, `ms_t *`), the case the lexical recogniser
//! does not type, so the typed readers decide.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
void *k_alloc(__SIZE_TYPE__ size); \
void k_free(void *p); \
void ext_take(void *p); \
struct lr { struct lr *tqh_first; struct lr **tqh_last; }; \
typedef struct mp_s { long id; struct lr q; } mp_t; \
typedef struct ks_s { int len; const char *str; } ks_t; \
typedef struct ms_s { ks_t bytes; struct ms_s *next; } ms_t; \
typedef struct ne_s { ks_t Host; } ne_t; \
typedef struct nes_s { int cnt; ne_t *eps; } nes_t;";

const RETURN: &str =
    "unsupported:unmodeled call, aggregate or indirect access in return expression";
const GATE: &str = "unsupported:indirect memory access is outside the resource projection";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        (
            "descriptor_generic.qv".to_owned(),
            include_str!("fixtures/contracts/descriptor_generic.qv").to_owned(),
        ),
        (
            "derived_result_generic.qv".to_owned(),
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
            eprintln!("skipping Clang-dependent K6 test: {reason}");
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

fn outcome(body: &str) -> (&'static str, Vec<String>) {
    let source = format!("{DECLARATIONS}\n{body}");
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

/// The row and its plain spelling (`(&X)->f` written `X.f`) decide alike.
fn expect_pair(label: &str, arrow: &str, plain: &str, status: &str, vector: &[&str]) {
    expect(
        &format!("{label} (arrow over an address)"),
        arrow,
        status,
        vector,
    );
    expect(&format!("{label} (plain spelling)"), plain, status, vector);
}

// -- admitted -----------------------------------------------------------------

/// `return TAILQ_FIRST(&p->q);` (rule 81 (1)): the pointer-return premise of
/// rule 5b no longer names `p` for the dereferenced group, as for
/// `return p->q.tqh_first;`. BEFORE: the return refusal.
#[test]
fn a_returned_member_through_an_address_is_the_plain_return() {
    if !clang_available() {
        return;
    }
    expect_pair(
        "return",
        "struct lr *w(mp_t *m) {\n return ((&m->q)->tqh_first);\n}",
        "struct lr *w(mp_t *m) {\n return ((m->q.tqh_first));\n}",
        "Safe",
        &[],
    );
    expect(
        "return through a scalar member",
        "long w(mp_t *m) {\n if (!((&m->q)->tqh_first))\n  return 0;\n return m->id;\n}",
        "Safe",
        &[],
    );
}

/// `RD_KAFKAP_BYTES_LEN(&m->bytes)` and `RD_KAFKAP_STR_LEN(&p->a[i].Host)`
/// (rule 81 (2)): the rule 10z edges read their `(&X)->len` places as
/// `X.len`. BEFORE: the gate (`place: path through UnaryOperator`).
#[test]
fn a_conditional_over_an_address_member_is_lowered_as_the_plain_one() {
    if !clang_available() {
        return;
    }
    expect_pair(
        "BYTES_LEN assignment",
        "int w(ms_t *m, int c) {\n int size;\n size = (int)((((&m->bytes)->len) == -1 ? 0 : ((&m->bytes)->len)) / c);\n return size;\n}",
        "int w(ms_t *m, int c) {\n int size;\n size = (int)((((m->bytes.len)) == -1 ? 0 : ((m->bytes.len))) / c);\n return size;\n}",
        "Safe",
        &[],
    );
    expect_pair(
        "STR_LEN declaration over an element",
        "long w(nes_t *p, int i) {\n long s = (((&p->eps[i].Host)->len) == -1 ? 0 : ((&p->eps[i].Host)->len)) + 1;\n return s;\n}",
        "long w(nes_t *p, int i) {\n long s = (((p->eps[i].Host.len)) == -1 ? 0 : ((p->eps[i].Host.len))) + 1;\n return s;\n}",
        "Safe",
        &[],
    );
    expect_pair(
        "an owned local read before its release",
        "int w(void) {\n ms_t *m = k_alloc(sizeof *m);\n int s;\n if (!m)\n  return 0;\n s = (((&m->bytes)->len) == -1 ? 0 : ((&m->bytes)->len));\n k_free(m);\n return s;\n}",
        "int w(void) {\n ms_t *m = k_alloc(sizeof *m);\n int s;\n if (!m)\n  return 0;\n s = (((m->bytes.len)) == -1 ? 0 : ((m->bytes.len)));\n k_free(m);\n return s;\n}",
        "Safe",
        &[],
    );
}

// -- refused and accused ------------------------------------------------------

/// A released, rebound, consumed or addressed root keeps its refusal, and a
/// read after a release is the same accusation as its plain spelling's.
#[test]
fn near_misses_decide_as_their_plain_spelling() {
    if !clang_available() {
        return;
    }
    expect_pair(
        "released before the return",
        "struct lr *w(mp_t *m) {\n k_free(m);\n return ((&m->q)->tqh_first);\n}",
        "struct lr *w(mp_t *m) {\n k_free(m);\n return ((m->q.tqh_first));\n}",
        "Unknown",
        &["unknown_alias:m", RETURN],
    );
    expect_pair(
        "handed to an uncontracted callee",
        "struct lr *w(mp_t *m) {\n ext_take(m);\n return ((&m->q)->tqh_first);\n}",
        "struct lr *w(mp_t *m) {\n ext_take(m);\n return ((m->q.tqh_first));\n}",
        "Unknown",
        &[
            "unsupported:call to uncontracted function `ext_take`",
            RETURN,
        ],
    );
    expect_pair(
        "an outer address is returned",
        "struct lr **w(mp_t *m) {\n return &((&m->q)->tqh_first);\n}",
        "struct lr **w(mp_t *m) {\n return &((m->q.tqh_first));\n}",
        "Unknown",
        &[RETURN],
    );
    expect_pair(
        "another statement takes the member's address",
        "struct lr *w(mp_t *m) {\n struct lr **s = &m->q.tqh_first;\n *s = 0;\n return ((&m->q)->tqh_first);\n}",
        "struct lr *w(mp_t *m) {\n struct lr **s = &m->q.tqh_first;\n *s = 0;\n return ((m->q.tqh_first));\n}",
        "Unknown",
        &[
            GATE,
            "unsupported:store through `*s` without an out-parameter contract",
            RETURN,
        ],
    );
    expect(
        "the outer address of a dereferenced group is stored",
        "struct lr *w(mp_t *m) {\n struct lr **s = &((&m->q)->tqh_first);\n *s = 0;\n return ((&m->q)->tqh_first);\n}",
        "Unknown",
        &[
            GATE,
            "unsupported:store through `*s` without an out-parameter contract",
            RETURN,
        ],
    );
    expect_pair(
        "released before the conditional read",
        "int w(ms_t *m) {\n int s;\n k_free(m);\n s = (((&m->bytes)->len) == -1 ? 0 : 1);\n return s;\n}",
        "int w(ms_t *m) {\n int s;\n k_free(m);\n s = (((m->bytes.len)) == -1 ? 0 : 1);\n return s;\n}",
        "Unknown",
        &["unknown_alias:m"],
    );
    expect_pair(
        "a loop rebinding the root",
        "long w(ms_t *m, int c) {\n long t = 0;\n while (c-- > 0 && m) {\n  t += (((&m->bytes)->len) == -1 ? 0 : ((&m->bytes)->len));\n  m = m->next;\n }\n return t;\n}",
        "long w(ms_t *m, int c) {\n long t = 0;\n while (c-- > 0 && m) {\n  t += (((m->bytes.len)) == -1 ? 0 : ((m->bytes.len)));\n  m = m->next;\n }\n return t;\n}",
        "Unknown",
        &["unknown_alias:m"],
    );
    expect_pair(
        "an owned local read after its release",
        "int w(void) {\n ms_t *m = k_alloc(sizeof *m);\n int s;\n if (!m)\n  return 0;\n k_free(m);\n s = (((&m->bytes)->len) == -1 ? 0 : ((&m->bytes)->len));\n return s;\n}",
        "int w(void) {\n ms_t *m = k_alloc(sizeof *m);\n int s;\n if (!m)\n  return 0;\n k_free(m);\n s = (((m->bytes.len)) == -1 ? 0 : ((m->bytes.len)));\n return s;\n}",
        "Unsafe",
        &["UseAfterRelease", "unknown_alias:m"],
    );
}

/// The one spelling pair rule 81 does not make agree: a rebound root. The
/// plain return is accused of the leak by an older route; the arrow spelling
/// keeps the return refusal (fail-closed, never `Safe`).
#[test]
fn a_rebound_root_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    expect(
        "rebound, arrow over an address",
        "struct lr *w(mp_t *m) {\n m = k_alloc(8);\n return ((&m->q)->tqh_first);\n}",
        "Unknown",
        &[RETURN],
    );
    expect(
        "rebound, plain spelling",
        "struct lr *w(mp_t *m) {\n m = k_alloc(8);\n return ((m->q.tqh_first));\n}",
        "Unsafe",
        &["leak", RETURN],
    );
}
