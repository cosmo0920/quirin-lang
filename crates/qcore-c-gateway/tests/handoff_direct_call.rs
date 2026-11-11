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

//! Card HANDOFF-DIRECT-CALL-READING (gateway rule 12m, amended in place): a
//! direct call of a hand-off declaration on route G, over the synthetic
//! callees of `fixtures/contracts/handoff_direct.qv` (trusted test
//! assumptions: a `Safe` here is `Safe` over them and promotes nothing).
//!
//! The caller's state for the handed object forks at the call (taken where
//! the guard holds, kept where it fails) and joins only at the caller's own
//! test of the result, once the calling body's reading admits the call. A
//! caller that agrees with the guard is decided; one that keeps the object
//! for a class the guard gives the callback, ignores the result, or tests it
//! anywhere the reading does not follow, refuses.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = "\
#include <stddef.h>
typedef enum { D_PASS, D_HANDLED, D_KEEP, D_YIELD } d_res_t;
struct dop { int x; };
struct dop *d_new(size_t n);
void d_free(struct dop *op);
d_res_t d_poll(void *q, struct dop *op);
d_res_t d_keep(void *q, struct dop *op);
";

const DIRECT: &str = "callback-handoff-direct";
const UNBRANCHED: &str = "callback-handoff-unbranched";

fn environment() -> ResourceContracts {
    let source = include_str!("fixtures/contracts/handoff_direct.qv");
    let source = if cfg!(windows) {
        source.replace("-> u32", "-> i32")
    } else {
        source.to_owned()
    };
    load_resource_contract_sources(&BTreeMap::from([("handoff_direct.qv".to_owned(), source)]))
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
            eprintln!("skipping Clang-dependent HANDOFF-DIRECT-CALL-READING test: {reason}");
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

/// The row of `entry`, the last definition of `PRELUDE + source`.
fn row(source: &str, entry: &str) -> Row {
    let text = format!("{PRELUDE}\n{source}\n");
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
    let row = &summary.functions[entry];
    Row {
        status: row.status,
        unsupported: row
            .diagnostics
            .iter()
            .filter_map(|d| match &d.kind {
                LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
                _ => None,
            })
            .collect(),
    }
}

/// A caller that agrees with the guard is decided: the object is gone where
/// the callback took it and the caller's where it did not, at the call's
/// own test, at the next statement's test of the stored result, with either
/// operator, and for the `==` form.
#[test]
fn a_caller_that_agrees_with_the_guard_is_admitted() {
    if !clang_available() {
        return;
    }
    for (entry, source) in [
        (
            "in_place",
            "int in_place(void *q) { struct dop *op = d_new(1); \
             if (d_poll(q, op) == D_PASS) d_free(op); return 0; }",
        ),
        (
            "stored_next",
            "int stored_next(void *q) { struct dop *op = d_new(1); d_res_t res; \
             res = d_poll(q, op); if (res == D_PASS) d_free(op); return 0; }",
        ),
        (
            "not_pass_returns",
            "int not_pass_returns(void *q) { struct dop *op = d_new(1); \
             if (d_poll(q, op) != D_PASS) return 1; d_free(op); return 0; }",
        ),
        (
            "keep_form",
            "int keep_form(void *q) { struct dop *op = d_new(1); \
             if (d_keep(q, op) == D_KEEP) return 1; d_free(op); return 0; }",
        ),
    ] {
        let admitted = row(source, entry);
        assert!(admitted.safe(), "{entry}: {admitted:?}");
    }
}

/// The split is not a blanket hand-off: the edge where the caller keeps the
/// object still owes it, so dropping it there is the caller's own leak.
#[test]
fn the_kept_edge_still_owes_the_object() {
    if !clang_available() {
        return;
    }
    let leaked = row(
        "int leaked(void *q) { struct dop *op = d_new(1); \
         if (d_poll(q, op) == D_PASS) return 1; return 0; }",
        "leaked",
    );
    assert!(!leaked.safe(), "{leaked:?}");
    assert!(
        !leaked
            .unsupported
            .iter()
            .any(|d| d.contains(DIRECT) || d.contains(UNBRANCHED)),
        "{leaked:?}"
    );
}

/// A caller that keeps the object for a class the guard gives the callback
/// refuses (`callback-handoff-unbranched`), never a finding.
#[test]
fn a_caller_that_keeps_the_object_for_the_wrong_class_refuses() {
    if !clang_available() {
        return;
    }
    for (entry, source) in [
        (
            "wrong_class",
            "int wrong_class(void *q) { struct dop *op = d_new(1); d_res_t res; \
             res = d_poll(q, op); if (res == D_HANDLED) return 1; d_free(op); return 0; }",
        ),
        (
            "both_arms",
            "int both_arms(void *q) { struct dop *op = d_new(1); \
             if (d_poll(q, op) == D_PASS) d_free(op); else d_free(op); return 0; }",
        ),
        (
            "keep_destroyed",
            "int keep_destroyed(void *q) { struct dop *op = d_new(1); \
             if (d_keep(q, op) == D_HANDLED) return 1; d_free(op); return 0; }",
        ),
    ] {
        let refused = row(source, entry);
        assert!(refused.refused_with(UNBRANCHED), "{entry}: {refused:?}");
        assert_ne!(
            refused.status,
            AnalysisStatus::Unsafe,
            "{entry}: {refused:?}"
        );
    }
}

/// A caller that ignores the result refuses: the call is not tested
/// (`callback-handoff-direct`), or, where it names the object afterwards,
/// the reading disagrees.
#[test]
fn a_caller_that_ignores_the_result_refuses() {
    if !clang_available() {
        return;
    }
    let ignored = row(
        "int ignored(void *q) { struct dop *op = d_new(1); d_poll(q, op); return 0; }",
        "ignored",
    );
    assert!(ignored.refused_with(DIRECT), "{ignored:?}");
    let freed = row(
        "int freed(void *q) { struct dop *op = d_new(1); d_poll(q, op); d_free(op); return 0; }",
        "freed",
    );
    assert!(freed.refused_with(UNBRANCHED), "{freed:?}");
}

/// A stored result tested later is decided only where the reading follows
/// it to the test: by the next statement (above). With a statement between,
/// with an edge that reaches results the guard splits, or through a
/// narrowing store, the call refuses.
#[test]
fn a_stored_result_branched_later_is_decided_only_where_read() {
    if !clang_available() {
        return;
    }
    for (entry, source) in [
        (
            "statement_between",
            "int statement_between(void *q) { struct dop *op = d_new(1); d_res_t res; int n; \
             res = d_poll(q, op); n = 1; if (res == D_PASS) d_free(op); return n; }",
        ),
        (
            "split_edge",
            "int split_edge(void *q) { struct dop *op = d_new(1); d_res_t res; \
             res = d_poll(q, op); if (res == D_HANDLED) return 1; \
             if (res == D_PASS) d_free(op); return 0; }",
        ),
        (
            "narrowed",
            "int narrowed(void *q) { struct dop *op = d_new(1); unsigned char res; \
             res = d_poll(q, op); if (res == D_PASS) d_free(op); return 0; }",
        ),
        (
            "initialised",
            "int initialised(void *q) { struct dop *op = d_new(1); \
             d_res_t res = d_poll(q, op); if (res == D_PASS) d_free(op); return 0; }",
        ),
    ] {
        let refused = row(source, entry);
        assert!(refused.refused_with(DIRECT), "{entry}: {refused:?}");
    }
}
