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

//! Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2 (gateway rules 14a, 14b
//! and 14c).
//!
//! * 14a: a scalar statement whose value holds exactly one call to a callee
//!   whose contract has a `None` result and no custody-taking position is the
//!   call (with rule W44's reads) and then the statement's typed reads.
//! * 14b: a pointer local given `&PATH` or `PATH ± n` under a never-bound root
//!   is the target-free borrowed alias a pointer read binds.
//! * 14c: `&B->F[I]` with a scalar element is read at a `Borrowed` actual.
//!
//! Every admission has a control that keeps the gate's refusal (or another
//! refusal) and never `Safe`. The fixture contracts are the CG00 harness's
//! `q_*` assumptions, `indirect_sole.qv` and `residual_r2.qv`: trusted test
//! assumptions, not library facts.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *p);
int is_cmp(const char *a, const char *b); char *is_dup(const char *p);
int is_count(const char *p); int is_unknown(const char *p);
__INT64_TYPE__ rr_load(__INT64_TYPE__ *p); int rr_release_count(void *p);
struct S { int a; char *name; struct S *next; __INT64_TYPE__ *counts; __INT64_TYPE__ arr[4]; char *names[4]; };
";

const INDIRECT: &str = "indirect memory access is outside the resource projection";

fn contracts() -> ResourceContracts {
    let file = |name: &str, text: &str| (name.to_owned(), text.to_owned());
    load_resource_contract_sources(&BTreeMap::from([
        file("q_base.qcore", include_str!("fixtures/contracts/q_base.qv")),
        file(
            "indirect_sole.qcore",
            include_str!("fixtures/contracts/indirect_sole.qv"),
        ),
        file(
            "residual_r2.qcore",
            include_str!("fixtures/contracts/residual_r2.qv"),
        ),
    ]))
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
            eprintln!("skipping Clang-dependent control: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Run {
    status: AnalysisStatus,
    descriptions: Vec<String>,
}

fn run(source: &str) -> Run {
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
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    };
    let result = summary
        .and_then(|summary| summary.functions.get("subject"))
        .unwrap_or_else(|| panic!("`subject` projected: {outcome:?}"));
    Run {
        status: result.status,
        descriptions: result
            .diagnostics
            .iter()
            .map(|diagnostic| match &diagnostic.kind {
                LegacyDiagnosticKind::Unsupported { description } => description.clone(),
                other => format!("{other:?}"),
            })
            .collect(),
    }
}

fn assert_safe(source: &str) {
    let run = run(source);
    assert_eq!(run.status, AnalysisStatus::Safe, "{source}\n{run:?}");
}

/// The function is not `Safe` and keeps the gate's refusal.
fn assert_keeps_the_gate(source: &str) {
    let run = run(source);
    assert_ne!(run.status, AnalysisStatus::Safe, "{source}\n{run:?}");
    assert!(
        run.descriptions
            .iter()
            .any(|description| description == INDIRECT),
        "{source}\n{run:?}"
    );
}

/// The function is not `Safe`.
fn assert_not_safe(source: &str) {
    let run = run(source);
    assert_ne!(run.status, AnalysisStatus::Safe, "{source}\n{run:?}");
}

#[test]
fn rule_14a_reads_one_scalar_call_inside_a_typed_scalar_statement() {
    if !clang_available() {
        return;
    }
    // A scalar local: the call's actual is read by W44, the other operand by
    // the typed path reader.
    assert_safe("int subject(struct S *a) { int n = is_count(a->name) - a->a; return n; }");
    // A typed scalar member store of the call's value.
    assert_safe("void subject(struct S *a) { a->a = is_count(a->name); }");
    assert_safe(
        "void subject(struct S *a) { a->a = is_cmp(a->name, a->next->name) + a->next->a; }",
    );
}

#[test]
fn rule_14a_keeps_the_refusal_for_every_other_call() {
    if !clang_available() {
        return;
    }
    // Two calls: C leaves their order unspecified, and neither is read here.
    assert_keeps_the_gate(
        "void subject(struct S *a) { a->a = is_count(a->name) + is_count(a->next->name); }",
    );
    // A custody-taking position: the call changes what the other operand reads.
    assert_not_safe("void subject(struct S *a) { a->a = rr_release_count(a->name) + a->a; }");
    // An owned result: its custody is not a scalar.
    assert_not_safe("void subject(struct S *a) { a->a = (is_dup(a->name) != 0) + a->a; }");
    // An uncontracted callee.
    assert_not_safe("void subject(struct S *a) { a->a = is_unknown(a->name) + a->a; }");
}

#[test]
fn rule_14b_reads_an_interior_pointer_of_a_never_bound_root() {
    if !clang_available() {
        return;
    }
    assert_safe("void subject(struct S *a) { struct S *n = &a->next[a->a]; n = 0; }");
    assert_safe("void subject(struct S *a) { char *p = a->name + a->a; p = 0; }");
    assert_safe("void subject(struct S *a) { __INT64_TYPE__ *c; c = &a->counts[2]; c = 0; }");
    // The alias is target-free and possibly null, exactly as a pointer read
    // binds it: a dereference through it is decided by its own rule, never
    // by the gate. Card NULL-SAFETY-PROPERTY (rule 14n, decision 1): that
    // rule now reads the dereference non-null under a named premise (before
    // it, an unresolved alias); `tests/null_premise_custody.rs` holds the
    // premise's controls.
    let run = run("int subject(struct S *a) { struct S *n = &a->next[a->a]; return n->a; }");
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    assert!(run.descriptions.is_empty(), "{run:?}");
}

#[test]
fn rule_14b_keeps_the_refusal_where_custody_could_move() {
    if !clang_available() {
        return;
    }
    // A bound root: the object may be tracked.
    assert_not_safe(
        "void subject(void) { struct S *a = q_alloc(); struct S *n = &a->next[0]; (void)n; q_free(a); }",
    );
    // The address of a pointer member is a slot, not an interior read.
    assert_keeps_the_gate("void subject(struct S *a) { char **slot = &a->name; *slot = 0; }");
    // The alias carries no custody: releasing it is never a release of a
    // tracked object.
    assert_not_safe("void subject(struct S *a) { char *p = a->name + 1; q_free(p); }");
}

#[test]
fn rule_14c_reads_a_scalar_element_address_at_a_borrowed_actual() {
    if !clang_available() {
        return;
    }
    assert_safe(
        "__INT64_TYPE__ subject(struct S *a, int i) { __INT64_TYPE__ v = rr_load(&a->counts[i]); return v; }",
    );
    assert_safe(
        "__INT64_TYPE__ subject(struct S *a, int i) { __INT64_TYPE__ v = rr_load(&a->arr[i]); return v; }",
    );
}

#[test]
fn rule_14c_keeps_the_refusal_for_an_element_that_is_not_a_scalar() {
    if !clang_available() {
        return;
    }
    // A pointer element: a write through the address reaches a pointer.
    assert_not_safe(
        "int subject(struct S *a, int i) { return is_count((const char *)&a->names[i]); }",
    );
    // A record element.
    assert_not_safe(
        "int subject(struct S *a, int i) { return is_count((const char *)&a->next[i]); }",
    );
}

#[test]
fn rule_14d_reads_an_interior_offset_at_a_borrowed_actual() {
    if !clang_available() {
        return;
    }
    assert_safe("int subject(struct S *a) { return is_count(a->name + a->a); }");
    assert_safe("void subject(struct S *a, char *out) { is_count(out + a->a); }");
}

#[test]
fn rule_14d_keeps_the_refusal_for_a_bound_root_or_a_custody_position() {
    if !clang_available() {
        return;
    }
    // A bound root: the object may be tracked.
    assert_not_safe(
        "void subject(void) { struct S *a = q_alloc(); is_count(a->name + 1); q_free(a); }",
    );
    // A releasing position: an interior pointer is never what was acquired.
    assert_not_safe("void subject(struct S *a) { q_free(a->name + 1); }");
    // A cast operand is not read.
    assert_keeps_the_gate("int subject(struct S *a) { return is_count((char *)a->next + a->a); }");
}
