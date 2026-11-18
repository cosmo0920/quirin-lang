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

//! Card **AG3-CONDITION-RETURN-CALLS** (gateway rule 63), end to end through
//! the real gateway: (1) a call-bearing condition leaf whose residual reads
//! memory, decided by the typed read beside identity calls, and (2) a
//! condition refused at an uncontracted call naming that callee.
//!
//! Every row asserts the status and the complete set of diagnostic keys.
//! `Safe` is `Safe` over the synthetic `k_*` / `d_*` test contracts.

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
__SIZE_TYPE__ d_len(const char *s); \
char *d_chr(const char *s, int c); \
int ext(int); \
int ext2(int); \
struct box { long n; char *name; struct box *next; int (*cb)(int); };";

const BRANCH: &str = "unsupported:unmodeled call, aggregate or indirect access in branch condition";
const LOOP: &str = "unsupported:unmodeled call, aggregate or indirect access in loop condition";
const SWITCH: &str =
    "unsupported:unmodeled call, aggregate or indirect access in switch expression";
const EXT: &str = "unsupported:call to uncontracted function `ext` in branch condition";
const EXT_LOOP: &str = "unsupported:call to uncontracted function `ext` in loop condition";
const EXT_FOR: &str = "unsupported:call to uncontracted function `ext` in for-loop condition";
const EXT_DO: &str = "unsupported:call to uncontracted function `ext` in do-while condition";

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
            eprintln!("skipping Clang-dependent condition-leaf test: {reason}");
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

// -- (1) the residual read beside identity calls ------------------------------

/// A call-bearing leaf whose residual reads a member, in each loop and branch
/// position, was refused; with an identity call it is the call then the read.
#[test]
fn a_residual_read_beside_an_identity_call_is_decided() {
    if !clang_available() {
        return;
    }
    for body in [
        "int w(const char *s, struct box *b) { if (d_len(s) > b->n) return 1; return 0; }",
        "int w(const char *s, struct box *b) { int i = 0; while (d_len(s) > b->n) { i++; b->n++; } return i; }",
        "int w(const char *s, struct box *b) { int i; for (i = 0; d_len(s) > b->n; i++) b->n++; return i; }",
        "int w(const char *s, struct box *b) { do { b->n++; } while (d_len(s) > b->n); return 0; }",
        "int w(const char *s, struct box *b) { if (d_chr(s, 'x') != 0 && d_len(s) < b->n) return 1; return 0; }",
        "int w(const char *s, struct box *b) { if (!(d_len(s) == b->next->n)) return 1; return 0; }",
    ] {
        expect("residual read", body, "Safe", &[]);
    }
}

/// The read keeps its base's obligation: a live owned base is read (and still
/// owed when not released), a released one is a `use_after_release`
/// candidate, and a loop that releases the base inside its body is never
/// `Safe`.
#[test]
fn the_residual_read_keeps_its_base_obligation() {
    if !clang_available() {
        return;
    }
    expect(
        "live owned base, released after",
        "int w(const char *s) { struct box *b = k_alloc(sizeof *b); int r = 0; if (!b) return 0; b->n = 1; if (d_len(s) > b->n) r = 1; k_free(b); return r; }",
        "Safe",
        &[],
    );
    expect(
        "read after release",
        "int w(const char *s) { struct box *b = k_alloc(sizeof *b); if (!b) return 0; b->n = 1; k_free(b); if (d_len(s) > b->n) return 1; return 0; }",
        "Unsafe",
        &["UseAfterRelease", "unknown_alias:b"],
    );
    let (status, keys) = outcome(
        "int w(const char *s) { struct box *b = k_alloc(sizeof *b); if (!b) return 0; b->n = 1; while (d_len(s) > b->n) { k_free(b); } return 0; }",
    );
    assert_ne!(status, "Safe", "a base released in the loop body: {keys:?}");
    assert!(keys.iter().any(|key| key == "UseAfterRelease"), "{keys:?}");
    expect(
        "a leak beside the decided condition is still reported",
        "int w(const char *s, struct box *b) { void *p = k_alloc(4); if (d_len(s) > b->n) return 1; k_free(p); return 0; }",
        "Unsafe",
        &["leak"],
    );
}

/// Near misses keep the refusal: a call that releases, a release sequenced
/// inside the leaf, an assignment, a switch.
#[test]
fn a_non_identity_call_an_assignment_or_a_switch_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    expect(
        "a releasing call in the leaf",
        "int w(int fd, struct box *b) { if (k_close(fd) > b->n) return 1; return 0; }",
        "Unknown",
        &[BRANCH],
    );
    expect(
        "a release sequenced inside the leaf",
        "int w(const char *s) { struct box *b = k_alloc(sizeof *b); if (!b) return 0; b->n = 1; if ((k_free(b), d_len(s)) > b->n) return 1; return 0; }",
        "Unknown",
        &[BRANCH],
    );
    expect(
        "an assignment in the leaf",
        "int w(const char *s, struct box *b) { __SIZE_TYPE__ n; if ((n = d_len(s)) > b->n) return 1; return 0; }",
        "Unknown",
        &[BRANCH],
    );
    expect(
        "a switch",
        "int w(const char *s, struct box *b) { switch (d_len(s) + b->n) { case 1: return 1; default: return 0; } }",
        "Unknown",
        &[SWITCH],
    );
}

// -- (2) the uncontracted call's named refusal -------------------------------

/// A condition every route refused at an uncontracted call names the callee,
/// in each loop and branch position; the status stays `Unknown`.
#[test]
fn a_condition_refused_at_an_uncontracted_call_names_it() {
    if !clang_available() {
        return;
    }
    for (body, text) in [
        ("int w(int a) { if (ext(a) == 2) return 1; return 0; }", EXT),
        (
            "int w(int a) { int i = 0; while (ext(a) > i) i++; return i; }",
            EXT_LOOP,
        ),
        (
            "int w(int a) { int i, t = 0; for (i = 0; i < ext(a); i++) t++; return t; }",
            EXT_FOR,
        ),
        (
            "int w(int a) { int i = 0; do { i++; } while (ext(a) > i); return i; }",
            EXT_DO,
        ),
        (
            "int w(int a) { if (ext(a) == 1 || ext2(a) == 2) return 1; return 0; }",
            EXT,
        ),
        (
            "int w(const char *s, int a) { if (d_len(s) > 2 && ext(a)) return 1; return 0; }",
            EXT,
        ),
    ] {
        expect("named refusal", body, "Unknown", &[text]);
    }
}

/// The named refusal is the refusal node it replaces: the refused-operand
/// hand-on rule reads it as a controlling expression's refusal (its text ends
/// with the position), so a leak beside it is accused exactly as beside the
/// position's own refusal; a descriptor read or a switch keeps the
/// position's text.
#[test]
fn the_named_refusal_keeps_the_node_and_its_exceptions() {
    if !clang_available() {
        return;
    }
    expect(
        "a leak past the named refusal",
        "int w(int a) { void *p = k_alloc(4); if (ext(a) == 2) return 1; k_free(p); return 0; }",
        "Unsafe",
        &["leak", EXT],
    );
    expect(
        "the same leak past the position's refusal",
        "int w(struct box *b) { void *p = k_alloc(4); if (b->cb != 0) return 1; k_free(p); return 0; }",
        "Unsafe",
        &["leak", BRANCH],
    );
    // A consumer that reads a plan as a decision (the value-connective route,
    // rule 10z) never receives the named refusal: its older answer stands.
    expect(
        "a conditional value keeps the value route's answer",
        "int w(int a) { int r = ext(a) == 2 ? 1 : 0; return r; }",
        "Unknown",
        &["unsupported:call to uncontracted function `ext`"],
    );
    expect(
        "a connective return keeps the return refusal",
        "int w(int a, int b) { return ext(a) && b; }",
        "Unknown",
        &["unsupported:unmodeled call, aggregate or indirect access in return expression"],
    );
    expect(
        "a descriptor read keeps the position's text",
        "int w(const char *p) { int fd = k_open(p); if (fd < 0) return 0; if (ext(fd) == 2) { k_close(fd); return 1; } k_close(fd); return 0; }",
        "Unknown",
        &[BRANCH],
    );
    expect(
        "a switch keeps the position's text",
        "int w(int a) { switch (ext(a)) { case 1: return 1; default: return 0; } }",
        "Unknown",
        &[SWITCH],
    );
    expect(
        "a loop keeps the loop text where the refusal is not a call",
        "int w(struct box *b) { int i = 0; while (b->cb != 0 && i < 3) i++; return i; }",
        "Unknown",
        &[LOOP],
    );
}
