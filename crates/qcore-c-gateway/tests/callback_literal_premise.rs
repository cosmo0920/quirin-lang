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

//! Card OPENSSL-PEM-CALLBACK-ARM (gateway rule 57): a literal-argument
//! premise beside a synchronous invocation, on route G, over the synthetic
//! `p_*` callees below (trusted test assumptions: a `Safe` here is `Safe`
//! over them and promotes nothing). `p_read` is the PEM readers' shape: the
//! declaration applies only where `x` is a null pointer constant, and there
//! its password callback is decided as rule 13u decides every invocation.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = "module contract;

extern fn p_alloc(n: usize) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(n);
    ensures normal resource_return_owned(nullable);
};

extern fn p_free(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(p, nullable);
    ensures normal resource_return_none();
};

extern fn p_read(out: ptr<mut, u8>, x: ptr<mut, ptr<mut, u8>>, cb: fnptr, u: ptr<mut, u8>) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    requires resource_literal_argument(x, 0);
    ensures normal resource_borrowed(out);
    ensures normal resource_borrowed(x);
    ensures normal resource_borrowed(cb);
    ensures normal resource_borrowed(u);
    ensures normal resource_callback_invoked_or_default(cb, _, _, _, u);
    ensures normal resource_return_owned(nullable);
};

extern fn p_read_plain(out: ptr<mut, u8>, x: ptr<mut, ptr<mut, u8>>, cb: fnptr, u: ptr<mut, u8>) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    requires resource_literal_argument(x, 0);
    ensures normal resource_borrowed(out);
    ensures normal resource_borrowed(x);
    ensures normal resource_borrowed(cb);
    ensures normal resource_borrowed(u);
    ensures normal resource_callback_invoked(cb, _, _, _, u);
    ensures normal resource_return_owned(nullable);
};
";

const PRELUDE: &str = "\
#include <stddef.h>
typedef int pass_cb(char *buf, int size, int rwflag, void *u);
void *p_alloc(__SIZE_TYPE__ n); void p_free(void *p);
void *p_read(void *out, void **x, pass_cb *cb, void *u);
void *p_read_plain(void *out, void **x, pass_cb *cb, void *u);
void unknown_effect(void *p);
pass_cb *pick(void);
";

const UNIT: &str = "\
static int quiet(char *buf, int size, int rwflag, void *u) { return size; }
static int reads(char *buf, int size, int rwflag, void *u) { return *(int *)u + size; }
static int frees(char *buf, int size, int rwflag, void *u) { p_free(u); return 0; }
static int opaque(char *buf, int size, int rwflag, void *u) { unknown_effect(u); return 0; }
void in_premise(void *bio, void *u) { void *r = p_read(bio, NULL, quiet, u); p_free(r); }
void in_premise_zero(void *bio, void *u) { void *r = p_read(bio, 0, quiet, u); p_free(r); }
void in_premise_leak(void *bio, void *u) { void *r = p_read(bio, NULL, quiet, u); }
void u_is_not_kept(void *bio) { void *u = p_alloc(8); void *r = p_read(bio, NULL, quiet, u); p_free(r); p_free(u); }
void u_is_not_released(void *bio) { void *u = p_alloc(8); void *r = p_read(bio, NULL, quiet, u); p_free(r); }
void reuse_arm(void *bio, void *u) { void *slot = NULL; void *r = p_read(bio, &slot, quiet, u); p_free(r); }
void reuse_formal(void *bio, void **x, void *u) { void *r = p_read(bio, x, quiet, u); p_free(r); }
void effectful(void *bio, void *u) { void *r = p_read(bio, NULL, frees, u); p_free(r); }
void uncontracted(void *bio, void *u) { void *r = p_read(bio, NULL, opaque, u); p_free(r); }
void null_callback(void *bio, void *u) { void *r = p_read(bio, NULL, NULL, u); p_free(r); }
void null_callback_zero(void *bio, void *u) { void *r = p_read(bio, 0, 0, u); p_free(r); }
void null_callback_cast(void *bio, void *u) { void *r = p_read(bio, NULL, (pass_cb *)0, u); p_free(r); }
void null_callback_leak(void *bio, void *u) { void *r = p_read(bio, NULL, NULL, u); }
void null_callback_reuse(void *bio, void *u) { void *slot = NULL; void *r = p_read(bio, &slot, NULL, u); p_free(r); }
void null_callback_plain(void *bio, void *u) { void *r = p_read_plain(bio, NULL, NULL, u); p_free(r); }
void plain_identity(void *bio, void *u) { void *r = p_read_plain(bio, NULL, quiet, u); p_free(r); }
void result_callback(void *bio, void *u) { void *r = p_read(bio, NULL, pick(), u); p_free(r); }
void local_callback(void *bio, void *u, int which) { pass_cb *cb = which ? quiet : NULL; void *r = p_read(bio, NULL, cb, u); p_free(r); }
void formal_callback(void *bio, pass_cb *cb, void *u) { void *r = p_read(bio, NULL, cb, u); p_free(r); }
void requires_u(void *bio, void *u) { void *r = p_read(bio, NULL, reads, u); p_free(r); }
void requires_u_direct(void *bio, void *u) { int n = reads(NULL, 0, 0, u); (void)n; }
";

fn environment() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "premise_callbacks.qv".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the fixture contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent OPENSSL-PEM-CALLBACK-ARM test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Row {
    status: AnalysisStatus,
    unsupported: Vec<String>,
}

impl Row {
    fn safe(&self) -> bool {
        self.status == AnalysisStatus::Safe
    }
    fn refused_with(&self, key: &str) -> bool {
        self.status != AnalysisStatus::Safe && self.unsupported.iter().any(|d| d.contains(key))
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> Option<&LegacyProgramSummary> {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    }
}

fn rows() -> BTreeMap<String, Row> {
    rows_of(UNIT, "requires_u_direct")
}

fn rows_of(unit: &str, entry: &str) -> BTreeMap<String, Row> {
    let text = format!("{PRELUDE}\n{unit}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        entry,
        &environment(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = summary_of(&outcome).unwrap_or_else(|| panic!("no summary: {outcome:?}"));
    summary
        .functions
        .iter()
        .map(|(name, row)| {
            (
                name.clone(),
                Row {
                    status: row.status,
                    unsupported: row
                        .diagnostics
                        .iter()
                        .filter_map(|d| match &d.kind {
                            LegacyDiagnosticKind::Unsupported { description } => {
                                Some(description.clone())
                            }
                            _ => None,
                        })
                        .collect(),
                },
            )
        })
        .collect()
}

const OUTSIDE: &str = "outside its declared literal-argument premise";

#[test]
fn within_the_premise_an_identity_callback_applies_the_declaration() {
    if !clang_available() {
        return;
    }
    let rows = rows();
    for name in ["in_premise", "in_premise_zero", "u_is_not_kept"] {
        assert!(rows[name].safe(), "{name}: {:?}", rows[name]);
    }
    // The declaration really applies: its fresh result is owed.
    assert_eq!(
        rows["in_premise_leak"].status,
        AnalysisStatus::Unsafe,
        "{:?}",
        rows["in_premise_leak"]
    );
    // `u` is read during the call, not kept and not released: the caller
    // still owes the object it handed.
    assert_eq!(
        rows["u_is_not_released"].status,
        AnalysisStatus::Unsafe,
        "{:?}",
        rows["u_is_not_released"]
    );
}

#[test]
fn a_non_null_x_stays_outside_the_premise() {
    if !clang_available() {
        return;
    }
    let rows = rows();
    for name in ["reuse_arm", "reuse_formal"] {
        assert!(rows[name].refused_with(OUTSIDE), "{name}: {:?}", rows[name]);
        // Withdrawn before the callback route reads the call.
        assert!(
            !rows[name]
                .unsupported
                .iter()
                .any(|d| d.contains("callback-")),
            "{name}: {:?}",
            rows[name]
        );
    }
}

#[test]
fn no_callback_is_assumed_benign_within_the_premise() {
    if !clang_available() {
        return;
    }
    let rows = rows();
    assert!(
        rows["uncontracted"].refused_with("callback-uncontracted"),
        "{:?}",
        rows["uncontracted"]
    );
    assert!(
        rows["effectful"].refused_with("callback-effectful"),
        "{:?}",
        rows["effectful"]
    );
    // A formal and a call's result are open candidate sets: the null
    // default admits only a null pointer constant.
    for name in ["formal_callback", "result_callback"] {
        assert!(
            rows[name].refused_with("callback-undecided"),
            "{name}: {:?}",
            rows[name]
        );
    }
}

#[test]
fn a_callback_reading_u_puts_its_requirement_on_the_call() {
    if !clang_available() {
        return;
    }
    let rows = rows();
    // As rule 13u: the call is decided as a direct call of the callback with
    // the same `u` is.
    assert_eq!(
        rows["requires_u"].status, rows["requires_u_direct"].status,
        "{:?} {:?}",
        rows["requires_u"], rows["requires_u_direct"]
    );
    assert_eq!(
        rows["requires_u"].unsupported.is_empty(),
        rows["requires_u_direct"].unsupported.is_empty()
    );
}

#[test]
fn a_null_callback_under_the_null_default_invokes_nothing() {
    if !clang_available() {
        return;
    }
    let rows = rows();
    for name in ["null_callback", "null_callback_zero", "null_callback_cast"] {
        assert!(rows[name].safe(), "{name}: {:?}", rows[name]);
    }
    // The declaration still applies: the result is owed.
    assert_eq!(
        rows["null_callback_leak"].status,
        AnalysisStatus::Unsafe,
        "{:?}",
        rows["null_callback_leak"]
    );
    // The null default changes nothing outside the `x` premise.
    assert!(
        rows["null_callback_reuse"].refused_with(OUTSIDE),
        "{:?}",
        rows["null_callback_reuse"]
    );
}

#[test]
fn without_the_null_default_a_null_callback_still_refuses() {
    if !clang_available() {
        return;
    }
    let rows = rows();
    assert!(
        rows["null_callback_plain"].refused_with("callback-undecided"),
        "{:?}",
        rows["null_callback_plain"]
    );
    // A non-null callback reads the same under either clause.
    assert!(
        rows["plain_identity"].safe(),
        "{:?}",
        rows["plain_identity"]
    );
    // A local that may hold a function or null is not a null pointer
    // constant: decided as rule 13u decides it, never read as no invocation.
    assert!(
        rows["local_callback"].refused_with("callback-undecided"),
        "{:?}",
        rows["local_callback"]
    );
}

/// No structural check establishes a null default: a unit defining the
/// declared function withdraws the declaration there (rule 13y), whatever
/// its body does with the callback.
const DEFINING: &str = "\
static int quiet(char *buf, int size, int rwflag, void *u) { return size; }
void *p_read(void *out, void **x, pass_cb *cb, void *u) { if (cb) cb(0, 0, 0, u); return p_alloc(1); }
void caller(void *bio, void *u) { void *r = p_read(bio, NULL, quiet, u); p_free(r); }
";

#[test]
fn a_defining_body_never_validates_a_null_default() {
    if !clang_available() {
        return;
    }
    let rows = rows_of(DEFINING, "caller");
    assert!(
        rows["caller"].refused_with("callback-withdrawn"),
        "{:?}",
        rows["caller"]
    );
}
