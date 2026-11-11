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

//! Card W44-LEAK-PIN-AUDIT — the three base leak semantics
//! W44-INDIRECT-CALL-ARGUMENT-ACCESS exposed on the corpus
//! (`W44-CALL-ARGUMENT-EXPOSED-LEAKS`), each on a body with **no**
//! call-argument access, as that card pinned them at the base.
//!
//! 1. an unsanctioned out-slot store `*out = p`;
//! 2. an object linked into an owner that is then returned;
//! 3. a release through an uncontracted helper.
//!
//! Every test asserts the **complete** diagnostic vector of `w` with its
//! status. None of these may become `Safe` through this card: a leak claimed
//! through an unresolved effect is withdrawn, never discharged. The fixtures
//! use a generic `x_*` surface: `Safe` here is `Safe` over synthetic trusted
//! test assumptions and promotes nothing.

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
extern fn x_free(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(p, nullable);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_record() -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_return_owned(nonnull); };\n\
extern fn x_record_free(r: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(r, nonnull);\n\
           ensures normal resource_return_none(); };";

const DECLARATIONS: &str = "\
void *x_alloc(__SIZE_TYPE__ n); void x_free(void *p); \
struct x_rec { int count; char *name; struct x_rec *next; }; \
struct x_rec *x_record(void); void x_record_free(struct x_rec *r); \
void y_release(void *p);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "exposed_leak_mechanisms.qcore".to_owned(),
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
            eprintln!("skipping Clang-dependent exposed-leak test: {reason}");
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
    let mut keys = program(&result)
        .functions
        .get("w")
        .expect("missing function `w`")
        .diagnostics
        .iter()
        .map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::UnknownAlias { variable } => format!("unknown_alias:{variable}"),
            LegacyDiagnosticKind::Leak { .. } => "leak".to_owned(),
            LegacyDiagnosticKind::Unsupported { description } => {
                format!("unsupported:{description}")
            }
            other => format!("{other:?}")
                .split([' ', '{'])
                .next()
                .unwrap_or("")
                .to_owned(),
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

/// Mechanism 1: the refused store `*out = p` hands `p` on as its value, so
/// the refused hand-on rule marks `p` unresolved there. At the base (rule
/// off) the same body is `Unsafe` with a `leak`.
#[test]
fn mechanism_1_an_unsanctioned_out_slot_store_is_not_a_leak_claim() {
    // Card OUT-PARAMETER-BODY-SUMMARY (rule 11r(6)): an `int` body returning
    // `-1` with the slot unwritten and `0` with it written validates its own
    // return-code relation, so the mechanism is pinned on a `void` body.
    expect(
        "`*out = p` with no out-parameter contract",
        "void w(char **out, unsigned long n) {\n char *p = x_alloc(n);\n if (!p) return;\n \
         *out = p;\n}",
        "Unknown",
        &["unsupported:store through `*out` without an out-parameter contract"],
    );
}

/// Mechanism 2, as W44-INDIRECT-CALL-ARGUMENT-ACCESS pinned it (one statement
/// per line, so the per-line hand-on table reads each alone): the link store
/// `b->name = ub` into the record is itself refused and hands `ub` on as its
/// value. At the base (rule off) it is `Unsafe` with a `leak`.
#[test]
fn mechanism_2_an_object_linked_into_a_returned_owner_is_not_a_leak_claim() {
    expect(
        "`b->name = ub; return b;` with a subscripted fill in between",
        "struct x_rec *w(char *bk, unsigned long n) {\n unsigned long i;\n char *ub;\n \
         struct x_rec *b;\n ub = x_alloc(n);\n if (!ub) return 0;\n b = x_record();\n \
         b->name = ub;\n for (i = 0; i < n; i++) {\n ub[i] = bk[i];\n }\n return b;\n}",
        "Unknown",
        &[
            "unknown_alias:ub",
            "unsupported:indirect memory access is outside the resource projection",
        ],
    );
}

/// Mechanism 2 in the corpus's own shape: the refused statement is again the
/// link store `b->upper = upper` (the typed fill is not what withdraws it: a
/// subscript base is not a hand-on). At the base (rule off) it is `Unsafe`
/// with a `leak`.
#[test]
fn mechanism_2_the_corpus_bucket_shape() {
    expect(
        "`cmt_histogram_buckets_create_size`'s shape: two nullable acquisitions, the link, \
         a guarded typed fill, the owner returned",
        "struct x_bk { unsigned long count; double *upper; };\n\
         struct x_bk *w(double *bkts, unsigned long count) {\n unsigned long i;\n \
         double *upper;\n struct x_bk *b;\n if (count < 1) {\n return 0;\n }\n \
         upper = x_alloc(8 * (count + 1));\n if (!upper) {\n return 0;\n }\n \
         b = x_alloc(16);\n if (!b) {\n x_free(upper);\n return 0;\n }\n \
         b->count = count;\n b->upper = upper;\n if (bkts != 0) {\n \
         for (i = 0; i < count; i++) {\n upper[i] = bkts[i];\n }\n }\n return b;\n}",
        "Unknown",
        &[
            "unknown_alias:upper",
            "unsupported:indirect memory access is outside the resource projection",
        ],
    );
}

/// Mechanism 3: `y_release(p)` is refused and hands `p` to the callee, which
/// may release or retain it; nothing is discharged. At the base (rule off) it
/// is `Unsafe` with a `leak`.
#[test]
fn mechanism_3_a_release_through_an_uncontracted_helper_is_not_a_leak_claim() {
    expect(
        "`y_release(p)` is refused and discharges nothing",
        "void w(unsigned long n) {\n char *p = x_alloc(n);\n if (!p) return;\n y_release(p);\n}",
        "Unknown",
        &["unsupported:call to uncontracted function `y_release`"],
    );
}
