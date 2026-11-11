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

//! Card NULL-SAFETY-PROPERTY (gateway rule 14n, USER DECISION 2026-10-10,
//! decision 1): a scalar access through a target-free, possibly-null
//! borrowed alias (a pointer-valued path read, rule 14b's interior alias) is
//! read non-null under a named premise the summary lists; every other
//! binding keeps its refusal.
//!
//! The fixture contracts are the CG00 harness's `q_*` assumptions: trusted
//! test assumptions, not library facts.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *p);
struct S { int a; char *name; struct S *next; long counts[4]; };
struct iovec { void *iov_base; unsigned long iov_len; };
struct source { struct iovec *iov; int iovlen; unsigned curvec; unsigned curoff; };
";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "q_base.qv".to_owned(),
        include_str!("fixtures/contracts/q_base.qv").to_owned(),
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
            eprintln!("skipping Clang-dependent control: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Run {
    status: AnalysisStatus,
    descriptions: Vec<String>,
    premises: Vec<(usize, String)>,
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
        premises: result
            .null_premises
            .iter()
            .map(|premise| (premise.line, premise.variable.clone()))
            .collect(),
    }
}

/// `Safe`, resting on exactly the named premises on these variables.
fn assert_safe_on(source: &str, variables: &[&str]) {
    let outcome = run(source);
    assert_eq!(
        outcome.status,
        AnalysisStatus::Safe,
        "{source}\n{outcome:?}"
    );
    assert!(outcome.descriptions.is_empty(), "{source}\n{outcome:?}");
    let mut named = outcome
        .premises
        .iter()
        .map(|(_, v)| v.as_str())
        .collect::<Vec<_>>();
    named.dedup();
    assert_eq!(named, variables, "{source}\n{outcome:?}");
}

#[test]
fn an_interior_alias_dereference_rests_on_a_named_premise() {
    if !clang_available() {
        return;
    }
    // Rule 14b's alias (CG10-INTERIOR-ALIAS-NULLABILITY's shape); before
    // this card, `UnknownAlias { variable: "n" }`.
    assert_safe_on(
        "int subject(struct S *a) { struct S *n = &a->next[a->a]; return n->a; }",
        &["n"],
    );
    assert_safe_on(
        "int subject(struct S *a) { struct S *n = &a->next[a->a]; int v = n->a; return v; }",
        &["n"],
    );
    // snappy.c's `peek`, reduced: one premise per line and variable (both
    // dereferences of `iv` are on one line).
    let source = "unsigned long subject(struct source *s, unsigned long *len) {
           if (s->curvec < (unsigned) s->iovlen) {
             struct iovec *iv = &s->iov[s->curvec];
             if (s->curoff < iv->iov_len) { *len = iv->iov_len - s->curoff; return 1; }
           }
           *len = 0; return 0; }";
    assert_safe_on(source, &["iv"]);
    let outcome = run(source);
    assert_eq!(outcome.premises.len(), 1, "{outcome:?}");
}

#[test]
fn no_premise_where_nothing_is_read_through_the_alias() {
    if !clang_available() {
        return;
    }
    let outcome =
        run("int subject(struct S *a) { struct S *n = &a->next[a->a]; n = 0; return a->a; }");
    assert_eq!(outcome.status, AnalysisStatus::Safe, "{outcome:?}");
    assert!(outcome.premises.is_empty(), "{outcome:?}");
    // A plain pointer read of a never-bound root leaves its destination
    // unbound: custody already reads its dereference as an untracked object
    // (W43-INDIRECT-ACCESS-GENERAL), with no premise (pre-existing; the null
    // property checks it).
    let outcome = run("int subject(struct S *a) { struct S *n = a->next; return n->a; }");
    assert_eq!(outcome.status, AnalysisStatus::Safe, "{outcome:?}");
    assert!(outcome.premises.is_empty(), "{outcome:?}");
}

#[test]
fn every_other_obligation_is_decided_as_before() {
    if !clang_available() {
        return;
    }
    // A null literal is no unknown alias: no premise (custody's reading of
    // it is unchanged).
    let outcome = run("int subject(void) { struct S *p = 0; int v = p->a; return v; }");
    assert!(outcome.premises.is_empty(), "{outcome:?}");
    // A tracked allocation stays the custody rule's question.
    let outcome =
        run("int subject(void) { struct S *p = q_alloc(); int v = p->a; q_free(p); return v; }");
    assert!(outcome.premises.is_empty(), "{outcome:?}");
    // Deliberate pin update, LEXICAL-ROUTES-PREEMPT-TYPED-FALLBACKS:
    // W42's original store still refuses its unsettled base. An independent
    // scalar-only Clang plan now discharges the access under the named null
    // premise; the pointer itself stays nullable and custody positions below
    // still refuse it. Syntax admission alone would not justify this result.
    assert_safe_on(
        "int subject(struct S *a) { struct S *n = &a->next[a->a]; n->a = 1; return 0; }",
        &["n"],
    );
    // Deliberate pin update for the same card's W44 chain alternative:
    // Clang independently establishes this unique, non-address-taken root's
    // bound-untracked eligibility, and every per-hop obligation discharges.
    // The root remains nullable under the same explicitly named premise.
    assert_safe_on(
        "int subject(struct S *a) { struct S *n = &a->next[a->a]; int v = n->a + n->next->a; return v; }",
        &["n"],
    );
    // The alias stays possibly null for custody: releasing it discharges
    // nothing and is refused as before.
    let outcome = run("void subject(struct S *a) { struct S *n = &a->next[1]; q_free(n); }");
    assert_ne!(outcome.status, AnalysisStatus::Safe, "{outcome:?}");
    assert!(outcome.premises.is_empty(), "{outcome:?}");
}
