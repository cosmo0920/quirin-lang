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

//! Card CALLEE-RETENTION-CLASS1 (gateway rules 12s and 12t): the brief's
//! positive and negative controls per shape, on route G.
//!
//! Stage (a) reads the `rp_*` dispositions (`fixtures/contracts/relational_post.qv`);
//! stage (b) the `r_*` retentions (`fixtures/contracts/retained_by.qv`). Both
//! are synthetic trusted test assumptions: a `Safe` here is `Safe` over them
//! and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const RELATIONAL: &str = "\
void *q_alloc(void); void q_free(void *p); void q_transfer(void *p);
int rp_take(void *p);
";

const RETAINED: &str = "\
struct st { const char *base; int n; };
void *r_alloc(__SIZE_TYPE__ n); void r_free(void *p);
int r_keep(struct st *state, const char *src); void r_reset(struct st *state);
void r_global(const char *value);
";

fn contracts(files: &[(&str, &str)]) -> ResourceContracts {
    load_resource_contract_sources(
        &files
            .iter()
            .map(|(name, text)| ((*name).to_owned(), (*text).to_owned()))
            .collect::<BTreeMap<_, _>>(),
    )
    .expect("the fixture contracts load")
}

fn relational() -> ResourceContracts {
    contracts(&[
        ("q_base.qcore", include_str!("fixtures/contracts/q_base.qv")),
        (
            "relational_post.qcore",
            include_str!("fixtures/contracts/relational_post.qv"),
        ),
    ])
}

fn retained() -> ResourceContracts {
    contracts(&[(
        "retained_by.qcore",
        include_str!("fixtures/contracts/retained_by.qv"),
    )])
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent CALLEE-RETENTION-CLASS1 test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Row {
    status: AnalysisStatus,
    unsupported: Vec<String>,
    kinds: Vec<String>,
}

impl Row {
    fn safe(&self) -> bool {
        self.status == AnalysisStatus::Safe
    }
    fn refused_with(&self, key: &str) -> bool {
        self.unsupported.iter().any(|d| d.contains(key))
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> Option<&LegacyProgramSummary> {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    }
}

/// Every row of the unit `prelude + source`, keyed by function.
fn rows(prelude: &str, source: &str, environment: &ResourceContracts) -> BTreeMap<String, Row> {
    let text = format!("{prelude}\n{source}\n");
    let entry = source
        .split('(')
        .next()
        .and_then(|head| head.split_whitespace().last())
        .expect("a definition")
        .trim_start_matches('*')
        .to_owned();
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        &entry,
        environment,
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
                    kinds: row
                        .diagnostics
                        .iter()
                        .map(|d| format!("{:?}", d.kind))
                        .collect(),
                },
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Stage (a): custody-taking inserts, rule 12s
// ---------------------------------------------------------------------------

const WRAPPERS: &str = "\
int wrap_take(void *p) { return rp_take(p); }
int wrap_wrap(void *p) { return wrap_take(p); }
int wrap_guard(void *p, int c) { if (c) return 1; return (rp_take(p)); }
long wrap_long(void *p) { return rp_take(p); }
int wrap_tested(void *p) { if (rp_take(p) != 0) { q_free(p); return -1; } return 0; }
";

#[test]
fn a_forwarding_wrapper_takes_its_callee_classes_and_its_row_is_decided_per_class() {
    if !clang_available() {
        return;
    }
    let rows = rows(RELATIONAL, WRAPPERS, &relational());
    for name in ["wrap_take", "wrap_wrap", "wrap_guard", "wrap_tested"] {
        assert!(rows[name].safe(), "{name}: {:?}", rows[name]);
    }
    // C converts `int` to `long` here: no forwarding, the older refusal.
    assert!(
        rows["wrap_long"].refused_with("relational-untested"),
        "{:?}",
        rows["wrap_long"]
    );
}

#[test]
fn a_transfer_on_success_with_the_caller_releasing_on_failure_is_safe() {
    if !clang_available() {
        return;
    }
    let source = format!(
        "{WRAPPERS}void subject(void) {{ void *p = q_alloc(); if (wrap_take(p) != 0) q_free(p); }}\n\
         void subject_wrapped(void) {{ void *p = q_alloc(); if (wrap_wrap(p) == 0) return; q_free(p); }}"
    );
    let rows = rows(RELATIONAL, &source, &relational());
    assert!(rows["subject"].safe(), "{:?}", rows["subject"]);
    assert!(
        rows["subject_wrapped"].safe(),
        "{:?}",
        rows["subject_wrapped"]
    );
}

#[test]
fn releasing_on_success_or_leaving_the_value_untested_is_never_safe() {
    if !clang_available() {
        return;
    }
    let source = format!(
        "{WRAPPERS}void wrong_edge(void) {{ void *p = q_alloc(); if (wrap_take(p) == 0) q_free(p); }}\n\
         void untested(void) {{ void *p = q_alloc(); wrap_take(p); }}"
    );
    let rows = rows(RELATIONAL, &source, &relational());
    // Released after the transfer on the success edge, kept on the other.
    assert_eq!(
        rows["wrong_edge"].status,
        AnalysisStatus::Unsafe,
        "{:?}",
        rows["wrong_edge"]
    );
    assert!(
        rows["wrong_edge"]
            .kinds
            .iter()
            .any(|k| k.starts_with("ReleaseAfterEscape")),
        "{:?}",
        rows["wrong_edge"]
    );
    assert!(
        rows["untested"].refused_with("relational-untested"),
        "{:?}",
        rows["untested"]
    );
}

// ---------------------------------------------------------------------------
// Stage (b): retained pointers, rule 12t
// ---------------------------------------------------------------------------

const CONTROLS: &str = "\
int frame_formal(const char *src) { struct st s; return r_keep(&s, src); }
void reset_then_release(void) { char *p = r_alloc(8); struct st s; if (!p) return; r_keep(&s, p); r_reset(&s); r_free(p); }
void release_before_reset(void) { char *p = r_alloc(8); struct st s; if (!p) return; r_keep(&s, p); r_free(p); r_reset(&s); }
void holder_released_first(void) { char *p; struct st *h; p = r_alloc(8); if (!p) return; h = r_alloc(16); if (!h) { r_free(p); return; } r_keep(h, p); r_free(h); r_free(p); }
void holder_outlives_p(void) { char *p; struct st *h; p = r_alloc(8); if (!p) return; h = r_alloc(16); if (!h) { r_free(p); return; } r_keep(h, p); r_free(p); r_free(h); }
void global_literal(void) { r_global(\"constant\"); }
void global_heap(void) { char *p = r_alloc(8); if (!p) return; r_global(p); r_free(p); }
int unknown_holder(struct st *state, const char *src) { return r_keep(state, src); }
void frame_copied(const char *src, struct st *out) { struct st s; r_keep(&s, src); *out = s; }
";

fn controls() -> BTreeMap<String, Row> {
    rows(RETAINED, CONTROLS, &retained())
}

#[test]
fn a_pointer_outliving_its_holder_is_safe() {
    if !clang_available() {
        return;
    }
    let rows = controls();
    // A caller's object outlives an automatic holder of this frame.
    assert!(rows["frame_formal"].safe(), "{:?}", rows["frame_formal"]);
    // The holder is reset before the pointer is released.
    assert!(
        rows["reset_then_release"].safe(),
        "{:?}",
        rows["reset_then_release"]
    );
    // A tracked heap holder is released before the pointer.
    assert!(
        rows["holder_released_first"].safe(),
        "{:?}",
        rows["holder_released_first"]
    );
}

#[test]
fn a_release_before_the_holder_reset_is_never_safe() {
    if !clang_available() {
        return;
    }
    let rows = controls();
    let row = &rows["release_before_reset"];
    assert_eq!(row.status, AnalysisStatus::Unknown, "{row:?}");
    assert!(row.refused_with("retention-released-while-held"), "{row:?}");
}

#[test]
fn a_holder_outliving_the_pointer_is_never_safe() {
    if !clang_available() {
        return;
    }
    let rows = controls();
    let row = &rows["holder_outlives_p"];
    assert!(!row.safe(), "{row:?}");
    assert!(row.refused_with("retention-released-while-held"), "{row:?}");
    // A refusal, never an accusation of its own.
    assert!(
        !row.kinds.iter().any(|k| k.starts_with("UseAfterRelease")),
        "{row:?}"
    );
}

#[test]
fn a_global_holder_keeps_only_static_storage() {
    if !clang_available() {
        return;
    }
    let rows = controls();
    assert!(
        rows["global_literal"].safe(),
        "{:?}",
        rows["global_literal"]
    );
    let row = &rows["global_heap"];
    assert_eq!(row.status, AnalysisStatus::Unknown, "{row:?}");
    assert!(row.refused_with("retention-undecided"), "{row:?}");
}

#[test]
fn an_unknown_holder_lifetime_is_unknown() {
    if !clang_available() {
        return;
    }
    let rows = controls();
    for name in ["unknown_holder", "frame_copied"] {
        let row = &rows[name];
        assert_eq!(row.status, AnalysisStatus::Unknown, "{name}: {row:?}");
        assert!(row.refused_with("retention-undecided"), "{name}: {row:?}");
    }
}

#[test]
fn a_retaining_declaration_is_never_in_the_function_map() {
    let environment = retained();
    assert!(environment.contract("r_keep").is_none());
    assert!(environment.retaining_contract("r_keep").is_some());
    assert!(environment.retaining_contract("r_reset").is_some());
    assert!(environment.contract("r_alloc").is_some());
}
