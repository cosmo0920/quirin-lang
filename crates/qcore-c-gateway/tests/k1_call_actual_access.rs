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

//! Card **K1-CALL-ACTUAL-ACCESS** (gateway rule 75), end to end through the
//! real gateway: a member address over a parenthesised name, `&(p)->f` (the
//! expansion of librdkafka's `rd_kafka_toppar_lock(rktp)`,
//! `mtx_lock(&(rktp)->rktp_lock)`), handed to a `Borrowed` position in a
//! statement, a condition and a return, is read as `&p->f` is.
//!
//! Every row asserts the status and the complete set of diagnostic keys.
//! `Safe` is `Safe` over the synthetic `k_*` / `d_*` test contracts. The base
//! is typedef-spelled (`t_t *`), the case the lexical recogniser does not
//! type, so the typed actual (rule 10k(3), rule 10n(1)) decides and needs the
//! never-bound premise rule 75 corrects.

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
__SIZE_TYPE__ d_len(const void *s); \
void *d_copy(void *dst, const void *src, __SIZE_TYPE__ n); \
void d_consume(void *p); \
typedef union { char b[40]; long a; } mtx_t; \
typedef struct t { mtx_t lock; long n; char *name; struct t *next; long a[4]; } t_t;";

const GATE: &str = "unsupported:indirect memory access is outside the resource projection";
const BRANCH: &str = "unsupported:unmodeled call, aggregate or indirect access in branch condition";

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
            eprintln!("skipping Clang-dependent K1 test: {reason}");
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

// -- admitted -----------------------------------------------------------------

/// The macro-expanded member address is read in every position rule 10k
/// reads a call actual: a statement, a condition, a return, a scalar `?:`
/// return, and over a union-typed member (`mtx_t`).
#[test]
fn a_parenthesised_member_address_is_read_as_the_plain_one() {
    if !clang_available() {
        return;
    }
    for (label, body) in [
        (
            "statement, union member",
            "long w(t_t *t) {\n d_len(&(t)->lock);\n return 0;\n}",
        ),
        (
            "statement, scalar member",
            "long w(t_t *t) {\n d_len(&(t)->n);\n return 0;\n}",
        ),
        (
            "lock, read, unlock",
            "long w(t_t *t) {\n d_len(&(t)->lock);\n long n = t->n;\n d_copy(&(t)->lock, t, 1);\n return n;\n}",
        ),
        (
            "condition",
            "long w(t_t *t) {\n if (d_len(&(t)->n))\n  return 1;\n return 0;\n}",
        ),
        (
            "whole-call return",
            "long w(t_t *t) {\n return d_len(&(t)->n);\n}",
        ),
        (
            "scalar ?: return (rd_kafka_terminating)",
            "long w(t_t *t) {\n return (d_len(&(t)->n) & 0x1) ? 3 : 4;\n}",
        ),
        (
            "doubly parenthesised",
            "long w(t_t *t) {\n d_len(&((t))->n);\n return 0;\n}",
        ),
        (
            "a later member hop",
            "long w(t_t *t) {\n d_len(&(t)->next->n);\n return 0;\n}",
        ),
    ] {
        expect(label, body, "Safe", &[]);
    }
}

/// The plain spelling was already read; rule 75 makes the parenthesised one
/// agree with it, never exceed it.
#[test]
fn the_plain_spelling_is_unchanged() {
    if !clang_available() {
        return;
    }
    expect(
        "plain",
        "long w(t_t *t) {\n d_len(&t->n);\n return 0;\n}",
        "Safe",
        &[],
    );
}

// -- refused: the root is bound by another event -------------------------------

/// A root released, whole-addressed, reassigned from a call, incremented or
/// bound from an allocation keeps the refusal (and the released one its
/// obligation): rule 75 drops only the member operand's own seed.
#[test]
fn a_root_bound_by_another_event_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    expect(
        "released after the address",
        "long w(t_t *t) {\n d_len(&(t)->n);\n k_free(t);\n return 0;\n}",
        "Unknown",
        &["unknown_alias:t", GATE],
    );
    expect(
        "released before the address",
        "long w(t_t *t) {\n k_free(t);\n d_len(&(t)->n);\n return 0;\n}",
        "Unknown",
        &["unknown_alias:t", GATE],
    );
    expect(
        "whole address elsewhere",
        "void g(t_t **p);\nlong w(t_t *t) {\n g(&(t));\n d_len(&(t)->n);\n return 0;\n}",
        "Unknown",
        &["unsupported:call to uncontracted function `g`", GATE],
    );
    expect(
        "incremented",
        "long w(t_t *t) {\n (t)++;\n d_len(&(t)->n);\n return 0;\n}",
        "Unknown",
        &[
            GATE,
            "unsupported:unmodeled increment or decrement expression",
        ],
    );
    expect(
        "owned local",
        "long w(void) {\n t_t *t = k_alloc(sizeof *t);\n if (!t)\n  return 0;\n d_len(&(t)->n);\n k_free(t);\n return 0;\n}",
        "Unknown",
        &[GATE],
    );
    expect(
        "owned local released first",
        "long w(void) {\n t_t *t = k_alloc(sizeof *t);\n if (!t)\n  return 0;\n k_free(t);\n d_len(&(t)->n);\n return 0;\n}",
        "Unknown",
        &[GATE],
    );
}

// -- refused: not this shape ---------------------------------------------------

/// A pointer or array member's address (a write through it may reach a
/// linked member), a custody-taking position and a branch condition whose
/// root is released keep their refusals.
#[test]
fn other_shapes_keep_their_refusals() {
    if !clang_available() {
        return;
    }
    expect(
        "pointer member address",
        "long w(t_t *t) {\n d_len(&(t)->name);\n return 0;\n}",
        "Unknown",
        &[GATE],
    );
    expect(
        "array member address",
        "long w(t_t *t) {\n d_len(&(t)->a);\n return 0;\n}",
        "Unknown",
        &[GATE],
    );
    expect(
        "custody-taking position",
        "long w(t_t *t) {\n d_consume(&(t)->n);\n return 0;\n}",
        "Unknown",
        &[GATE],
    );
    expect(
        "released root in a condition",
        "long w(t_t *t) {\n k_free(t);\n if (d_len(&(t)->n))\n  return 1;\n return 0;\n}",
        "Unknown",
        &["unknown_alias:t", BRANCH],
    );
}

/// A loop that reassigns the root from an admitted field read keeps the
/// existing never-bound reading (unchanged by rule 75), and one that
/// releases the root in its body is never `Safe`.
#[test]
fn loops_are_unchanged() {
    if !clang_available() {
        return;
    }
    expect(
        "walk",
        "long w(t_t *t, int c) {\n while (c--) {\n  d_len(&(t)->n);\n  t = t->next;\n }\n return 0;\n}",
        "Safe",
        &[],
    );
    let (status, keys) = outcome(
        "long w(t_t *t, int c) {\n while (c--) {\n  d_len(&(t)->n);\n  k_free(t);\n }\n return 0;\n}",
    );
    assert_ne!(status, "Safe", "a root released in the loop body: {keys:?}");
}
