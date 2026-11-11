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

//! Card CG23-RELATIONAL-POSTCONDITIONS (gateway rule 11r): return-code ×
//! resource postconditions at call sites, exit-specific summaries, and the
//! body check of a declared disposition, on route G.
//!
//! The `rp_*` contracts (`fixtures/contracts/relational_post.qv`) and the
//! `q_*` family (`q_base.qv`) are synthetic trusted test assumptions. A
//! `Safe` here is `Safe` over them and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *p); void q_transfer(void *p);
int rp_take(void *p);
int rp_open_list(const char *name, void **out);
void rp_free_list(void *list);
int rp_release_on_error(void *p);
int rp_contradicted(void *p);
";

const UNTESTED: &str = "relational-untested";
const WITHDRAWN: &str = "relational-withdrawn";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        (
            "q_base.qcore".to_owned(),
            include_str!("fixtures/contracts/q_base.qv").to_owned(),
        ),
        (
            "relational_post.qcore".to_owned(),
            include_str!("fixtures/contracts/relational_post.qv").to_owned(),
        ),
    ]))
    .expect("the CG23 fixture contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent CG23 test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Outcome {
    status: AnalysisStatus,
    diagnostics: Vec<String>,
    unsupported: Vec<String>,
}

impl Outcome {
    fn safe(&self) -> bool {
        self.status == AnalysisStatus::Safe
    }
    fn refused_with(&self, key: &str) -> bool {
        self.unsupported.iter().any(|d| d.contains(key))
    }
    fn has(&self, fragment: &str) -> bool {
        self.diagnostics.iter().any(|d| d.contains(fragment))
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> Option<&LegacyProgramSummary> {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    }
}

fn run(source: &str) -> Outcome {
    let text = format!("{DECLARATIONS}\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let result = summary_of(&outcome)
        .and_then(|summary| summary.functions.get("subject"))
        .unwrap_or_else(|| panic!("no row for `subject`: {outcome:?}"));
    Outcome {
        status: result.status,
        diagnostics: result
            .diagnostics
            .iter()
            .map(|d| format!("{:?}", d.kind))
            .collect(),
        unsupported: result
            .diagnostics
            .iter()
            .filter_map(|d| match &d.kind {
                LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
                _ => None,
            })
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Declared dispositions at the call site
// ---------------------------------------------------------------------------

#[test]
fn a_tested_transfer_instantiates_each_edge() {
    if !clang_available() {
        return;
    }
    // The brief's shape: transferred on 0, released by the caller otherwise.
    for source in [
        "void subject(void) { void *p = q_alloc(); if (rp_take(p) == 0) { } else q_free(p); }",
        "void subject(void) { void *p = q_alloc(); if (rp_take(p) != 0) q_free(p); }",
        "void subject(void) { void *p = q_alloc(); if (rp_take(p)) q_free(p); }",
        "void subject(void) { void *p = q_alloc(); if (!rp_take(p)) return; q_free(p); }",
        "void subject(void) { void *p = q_alloc(); if (0 != rp_take(p)) q_free(p); }",
    ] {
        let outcome = run(source);
        assert!(outcome.safe(), "{source}: {outcome:?}");
    }
}

#[test]
fn the_same_call_without_a_test_is_not_safe() {
    if !clang_available() {
        return;
    }
    for source in [
        "void subject(void) { void *p = q_alloc(); rp_take(p); }",
        "void subject(void) { void *p = q_alloc(); int rc = rp_take(p); if (rc != 0) q_free(p); }",
        "void subject(void) { void *p = q_alloc(); int rc; rc = rp_take(p); (void)rc; }",
    ] {
        let outcome = run(source);
        assert!(!outcome.safe(), "{source}: {outcome:?}");
        assert!(outcome.refused_with(UNTESTED), "{source}: {outcome:?}");
    }
}

#[test]
fn a_test_that_does_not_separate_the_classes_is_refused() {
    if !clang_available() {
        return;
    }
    // `>= 0` reaches both the `eq 0` and the `ne 0` class.
    let outcome =
        run("void subject(void) { void *p = q_alloc(); if (rp_take(p) >= 0) q_free(p); }");
    assert!(!outcome.safe(), "{outcome:?}");
    // A connective is no accepted test.
    let outcome = run(
        "void subject(int c) { void *p = q_alloc(); if (c && rp_take(p) == 0) return; q_free(p); }",
    );
    assert!(!outcome.safe(), "{outcome:?}");
}

#[test]
fn each_edge_carries_its_own_class() {
    if !clang_available() {
        return;
    }
    // Freed on the edge where it was transferred: a violation candidate.
    let outcome =
        run("void subject(void) { void *p = q_alloc(); if (rp_take(p) == 0) q_free(p); }");
    assert!(!outcome.safe(), "{outcome:?}");
    assert_eq!(outcome.status, AnalysisStatus::Unsafe, "{outcome:?}");
    // Kept on the failure edge and never released: a leak candidate.
    let outcome = run("void subject(void) { void *p = q_alloc(); if (rp_take(p) != 0) return; }");
    assert_eq!(outcome.status, AnalysisStatus::Unsafe, "{outcome:?}");
    assert!(outcome.has("Leak"), "{outcome:?}");
}

#[test]
fn a_stored_return_tested_next_is_the_tested_call() {
    if !clang_available() {
        return;
    }
    let outcome = run(
        "void subject(void) { void *p = q_alloc(); int rc; rc = rp_take(p); if (rc != 0) q_free(p); }",
    );
    assert!(outcome.safe(), "{outcome:?}");
    let outcome = run(
        "void subject(void) { void *p = q_alloc(); int rc; rc = rp_release_on_error(p); if (rc < 0) return; q_free(p); }",
    );
    assert!(outcome.safe(), "{outcome:?}");
}

/// Card RELATIONAL-REWRITE-LINE-ORDER: the stored-return rewrite of a test
/// written on a later line keeps the stream's lines in order, so the unit's
/// other statements keep their typed occurrences: two identical member
/// stores on one line are told apart (rule 10t) in the same body. Before the
/// card the rewritten `if` stood on its own line ahead of the moved call's,
/// every occurrence of the unit was declined and the stores refused.
#[test]
fn a_rewritten_unit_keeps_its_typed_occurrences() {
    if !clang_available() {
        return;
    }
    let outcome = run("struct S { struct { int x; } in; };\n\
         void subject(struct S *s)\n\
         {\n\
           void *p = q_alloc();\n\
           int rc;\n\
           rc = rp_take(p);\n\
           if (rc != 0)\n\
             q_free(p);\n\
           s->in.x = 1; s->in.x = 1;\n\
         }");
    assert!(outcome.safe(), "{outcome:?}");
    // The rewrite's own reading is unchanged on a later line too.
    let outcome = run("void subject(void)\n\
         {\n\
           void *p = q_alloc();\n\
           int rc;\n\
           rc = rp_take(p);\n\
           if (rc == 0)\n\
             return;\n\
           q_free(p);\n\
         }");
    assert!(outcome.safe(), "{outcome:?}");
}

#[test]
fn a_status_witnessed_out_slot_exists_only_on_success() {
    if !clang_available() {
        return;
    }
    let outcome =
        run("void subject(void) { void *l; if (rp_open_list(\"x\", &l) == 0) rp_free_list(l); }");
    assert!(outcome.safe(), "{outcome:?}");
    let outcome = run(
        "void subject(void) { void *l; int rc; rc = rp_open_list(\"x\", &l); if (rc != 0) return; rp_free_list(l); }",
    );
    assert!(outcome.safe(), "{outcome:?}");
    // The list is owed on success.
    let outcome = run("void subject(void) { void *l; if (rp_open_list(\"x\", &l) == 0) return; }");
    assert_eq!(outcome.status, AnalysisStatus::Unsafe, "{outcome:?}");
    // Without the test, nothing is applied.
    let outcome = run("void subject(void) { void *l; rp_open_list(\"x\", &l); }");
    assert!(outcome.refused_with(UNTESTED), "{outcome:?}");
}

// ---------------------------------------------------------------------------
// The body: exit-specific summaries and the check of a declaration
// ---------------------------------------------------------------------------

#[test]
fn an_exit_specific_summary_is_instantiated_by_the_test() {
    if !clang_available() {
        return;
    }
    let helper = "static int rel(void *p, int c) { if (c) { q_free(p); return 0; } return -1; }\n";
    let outcome = run(&format!(
        "{helper}void subject(int c) {{ void *p = q_alloc(); if (rel(p, c) != 0) q_free(p); }}"
    ));
    assert!(outcome.safe(), "{outcome:?}");
    let outcome = run(&format!(
        "{helper}void subject(int c) {{ void *p = q_alloc(); if (rel(p, c) < 0) q_free(p); }}"
    ));
    assert!(outcome.safe(), "{outcome:?}");
    // Untested, the classes disagree: refused.
    let outcome = run(&format!(
        "{helper}void subject(int c) {{ void *p = q_alloc(); rel(p, c); }}"
    ));
    assert!(outcome.refused_with(UNTESTED), "{outcome:?}");
    // A test the returned values cannot satisfy reaches no class: refused.
    let outcome = run(&format!(
        "{helper}void subject(int c) {{ void *p = q_alloc(); if (rel(p, c) > 0) q_free(p); }}"
    ));
    assert!(!outcome.safe(), "{outcome:?}");
}

#[test]
fn a_helper_with_one_disposition_on_every_exit_stays_ordinary() {
    if !clang_available() {
        return;
    }
    let outcome = run(
        "static int rel(void *p, int c) { q_free(p); if (c) return 0; return -1; }\n\
         void subject(int c) { void *p = q_alloc(); rel(p, c); }",
    );
    assert!(outcome.safe(), "{outcome:?}");
}

#[test]
fn a_helper_returning_anything_but_literals_has_no_exit_specific_summary() {
    if !clang_available() {
        return;
    }
    let outcome = run(
        "static int rel(void *p, int c) { if (c) { q_free(p); return c; } return -1; }\n\
         void subject(int c) { void *p = q_alloc(); if (rel(p, c) != -1) return; q_free(p); }",
    );
    assert!(!outcome.safe(), "{outcome:?}");
}

#[test]
fn a_disposition_its_body_contradicts_is_withdrawn() {
    if !clang_available() {
        return;
    }
    let outcome = run("int rp_contradicted(void *p) { q_free(p); return 0; }\n\
         void subject(void) { void *p = q_alloc(); if (rp_contradicted(p) != 0) q_free(p); }");
    assert!(!outcome.safe(), "{outcome:?}");
    assert!(outcome.refused_with(WITHDRAWN), "{outcome:?}");
    // The same declaration without a body in the unit stands as declared.
    let outcome =
        run("void subject(void) { void *p = q_alloc(); if (rp_contradicted(p) != 0) q_free(p); }");
    assert!(outcome.safe(), "{outcome:?}");
}

#[test]
fn a_disposition_its_body_establishes_stands() {
    if !clang_available() {
        return;
    }
    let outcome = run("extern int ready;\n\
         int rp_take(void *p) { if (!ready) return -1; q_transfer(p); return 0; }\n\
         void subject(void) { void *p = q_alloc(); if (rp_take(p) != 0) q_free(p); }");
    assert!(outcome.safe(), "{outcome:?}");
}

// ---------------------------------------------------------------------------
// The loader
// ---------------------------------------------------------------------------

fn load(declaration: &str) -> Result<ResourceContracts, String> {
    load_resource_contract_sources(&BTreeMap::from([(
        "probe.qcore".to_owned(),
        format!("module contract;\n\n{declaration}\n"),
    )]))
}

fn take(clauses: &str, result: &str) -> String {
    format!(
        "extern fn take(p: ptr<mut, u8>, q: ptr<mut, ptr<mut, u8>>) -> {result}\ncontract {{\n    requires resource_contract_v1;\n{clauses}    ensures normal resource_return_none();\n}};"
    )
}

#[test]
fn the_loader_installs_a_disposition_outside_the_function_map() {
    let contracts = load(&take(
        "    ensures normal resource_borrowed(q);\n    ensures normal resource_on_return(eq, 0, resource_consumed(p, nonnull));\n    ensures normal resource_on_return(ne, 0, resource_owned(p, nonnull));\n",
        "i32",
    ))
    .expect("loads");
    assert!(contracts.contract("take").is_none());
    let relational = contracts.relational_contract("take").expect("relational");
    assert_eq!(relational.classes.len(), 2);
    qcore_contract::contract_loader::check_source_assumptions(&contracts)
        .expect("the retained text reloads to the same dispositions");
}

#[test]
fn the_loader_refuses_malformed_dispositions() {
    let refused = [
        // Overlapping classes.
        "    ensures normal resource_borrowed(q);\n    ensures normal resource_on_return(le, 0, resource_consumed(p, nonnull));\n    ensures normal resource_on_return(ge, 0, resource_owned(p, nonnull));\n",
        // Not total.
        "    ensures normal resource_borrowed(q);\n    ensures normal resource_on_return(eq, 0, resource_consumed(p, nonnull));\n    ensures normal resource_on_return(gt, 0, resource_owned(p, nonnull));\n",
        // Every class agrees: a plain declaration, written plainly.
        "    ensures normal resource_borrowed(q);\n    ensures normal resource_on_return(eq, 0, resource_owned(p, nonnull));\n    ensures normal resource_on_return(ne, 0, resource_owned(p, nonnull));\n",
        // A parameter both unconditional and conditional.
        "    ensures normal resource_borrowed(q);\n    ensures normal resource_owned(p, nonnull);\n    ensures normal resource_on_return(eq, 0, resource_consumed(p, nonnull));\n    ensures normal resource_on_return(ne, 0, resource_owned(p, nonnull));\n",
        // A class that leaves a parameter without a mode.
        "    ensures normal resource_on_return(eq, 0, resource_consumed(p, nonnull));\n    ensures normal resource_on_return(ne, 0, resource_owned(p, nonnull));\n    ensures normal resource_on_return(eq, 0, resource_out(q, nonnull));\n",
        // Entry requirements disagree (custody versus none).
        "    ensures normal resource_borrowed(q);\n    ensures normal resource_on_return(eq, 0, resource_consumed(p, nonnull));\n    ensures normal resource_on_return(ne, 0, resource_borrowed(p));\n",
        // Nullability of the entry disagrees.
        "    ensures normal resource_borrowed(q);\n    ensures normal resource_on_return(eq, 0, resource_consumed(p, nonnull));\n    ensures normal resource_on_return(ne, 0, resource_owned(p, nullable));\n",
        // A stored mode inside a class.
        "    ensures normal resource_borrowed(q);\n    ensures normal resource_on_return(eq, 0, resource_stored(p, q, nonnull));\n    ensures normal resource_on_return(ne, 0, resource_owned(p, nonnull));\n",
    ];
    for clauses in refused {
        assert!(load(&take(clauses, "i32")).is_err(), "{clauses}");
    }
    // An unsigned or pointer result.
    let clauses = "    ensures normal resource_borrowed(q);\n    ensures normal resource_on_return(eq, 0, resource_consumed(p, nonnull));\n    ensures normal resource_on_return(ne, 0, resource_owned(p, nonnull));\n";
    for result in ["u32", "ptr<mut, u8>", "unit"] {
        assert!(load(&take(clauses, result)).is_err(), "{result}");
    }
    // Beside a review marker.
    let marked = take(
        "    requires resource_review_pending(\"unreviewed\");\n    ensures normal resource_borrowed(q);\n    ensures normal resource_on_return(eq, 0, resource_consumed(p, nonnull));\n    ensures normal resource_on_return(ne, 0, resource_owned(p, nonnull));\n",
        "i32",
    );
    assert!(load(&marked).is_err());
    // Beside an in-place kind (rule 11v), whose initialiser status is already
    // its own witness: one status is never read twice.
    let in_place = "extern fn init(preg: ptr<mut, u8>, p: ptr<mut, u8>) -> i32\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_out(preg, nullable);\n    ensures normal resource_kind(preg, regex);\n    ensures normal resource_on_return(eq, 0, resource_consumed(p, nonnull));\n    ensures normal resource_on_return(ne, 0, resource_owned(p, nonnull));\n    ensures normal resource_return_none();\n};";
    let error = load(in_place).expect_err("refused beside an in-place kind");
    assert!(error.contains("in-place kind"), "{error}");
}
