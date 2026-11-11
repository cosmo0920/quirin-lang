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

//! Card CG10-INDIRECT-ACCESS-SOLE-BLOCKERS (gateway rules 12i and 12j).
//!
//! Rule 12i: a `return` whose one call's actuals the lexical gate refused is
//! read with rule W44's call-argument access, exactly as the same call as a
//! statement. Rule 12j: a `.`-only scalar place on a record object of the
//! function's automatic storage is read with no root premise.
//!
//! The fixture contracts are the CG00 harness's `q_*` assumptions, the return
//! position's `r_*` assumptions and `indirect_sole.qv`. Trusted test
//! assumptions, not library facts. Every admission has a refusal control.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *p); void q_observe(void *p);
int r_release_code(void *p);
int is_cmp(const char *a, const char *b); char *is_dup(const char *p);
int is_count(const char *p); int is_unknown(const char *p);
struct S { int a; char *name; struct S *next; struct I { int x; long y; } in; };
union V { long i; double d; };
struct W { union V v; char *p; int n; };
";

const INDIRECT: &str = "indirect memory access is outside the resource projection";

fn contracts() -> ResourceContracts {
    let file = |name: &str, text: &str| (name.to_owned(), text.to_owned());
    load_resource_contract_sources(&BTreeMap::from([
        file("q_base.qcore", include_str!("fixtures/contracts/q_base.qv")),
        file(
            "q_rich_modes.qcore",
            include_str!("fixtures/contracts/q_rich_modes.qv"),
        ),
        file(
            "r_return_call.qcore",
            include_str!("fixtures/contracts/r_return_call.qv"),
        ),
        file(
            "indirect_sole.qcore",
            include_str!("fixtures/contracts/indirect_sole.qv"),
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
    diagnostics: Vec<(usize, LegacyDiagnosticKind)>,
}

impl Run {
    fn refuses_access(&self) -> bool {
        self.diagnostics.iter().any(|(_, kind)| {
            matches!(kind, LegacyDiagnosticKind::Unsupported { description, .. }
                if description == INDIRECT)
        })
    }
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
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
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
        diagnostics: result
            .diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.line, diagnostic.kind.clone()))
            .collect(),
    }
}

fn assert_safe(source: &str) {
    let run = run(source);
    assert_eq!(run.status, AnalysisStatus::Safe, "{source}\n{run:?}");
}

fn assert_refused(source: &str) {
    let run = run(source);
    assert_ne!(run.status, AnalysisStatus::Safe, "{source}\n{run:?}");
}

// Rule 12i.

#[test]
fn a_whole_call_return_reads_its_member_actuals_as_the_statement_would() {
    if !clang_available() {
        return;
    }
    assert_safe("int subject(struct S *a, struct S *b) { return is_cmp(a->name, b->name); }");
    assert_safe("int subject(struct S *a) { return is_count(a->name); }");
}

#[test]
fn an_owned_result_of_a_whole_call_return_escapes_to_the_caller() {
    if !clang_available() {
        return;
    }
    assert_safe("char *subject(struct S *a) { return is_dup(a->name); }");
}

#[test]
fn a_scalar_return_reads_its_one_call_as_b01_with_its_actuals() {
    if !clang_available() {
        return;
    }
    assert_safe("int subject(struct S *a) { return 1 + is_count(a->name); }");
}

#[test]
fn a_custody_position_keeps_the_refusal_at_the_return() {
    if !clang_available() {
        return;
    }
    let run = run("int subject(struct S *a) { return r_release_code(a->name); }");
    assert_ne!(run.status, AnalysisStatus::Safe, "{run:?}");
    assert!(run.refuses_access(), "{run:?}");
}

#[test]
fn an_uncontracted_callee_keeps_the_refusal_at_the_return() {
    if !clang_available() {
        return;
    }
    assert_refused("int subject(struct S *a) { return is_unknown(a->name); }");
}

#[test]
fn a_released_root_read_at_the_return_is_refused() {
    if !clang_available() {
        return;
    }
    // The root's typed read is W44's obligation: a released identity refuses.
    assert_refused(
        "int subject(void) { struct S *s = q_alloc(); if (!s) return 0; \
         q_free(s); return is_count(s->name); }",
    );
}

// Rule 12j. Each record object's address is handed to a `Borrowed` position,
// so the custody closure does not prove it never bound (class (d)); before the
// rule every one of these member stores was the gate's refusal.

#[test]
fn a_scalar_store_into_an_addressed_record_object_is_read() {
    if !clang_available() {
        return;
    }
    assert_safe("void subject(int n) { struct S o; q_observe(&o); o.a = n + 1; }");
    assert_safe("void subject(int n) { struct S o; o.in.x = n * 2; q_observe(&o); }");
}

#[test]
fn a_scalar_store_through_a_custody_free_union_member_is_read() {
    if !clang_available() {
        return;
    }
    assert_safe(
        "void subject(int n) { struct W w; q_observe(&w); w.v.i = n + 3; \
         w.n = (int) w.v.i + 1; }",
    );
}

#[test]
fn a_by_value_record_parameter_is_automatic_storage() {
    if !clang_available() {
        return;
    }
    assert_safe("int subject(struct S o, int n) { q_observe(&o); o.a = n + 1; return o.a + 1; }");
}

#[test]
fn a_static_record_object_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    let run = run("void subject(int n) { static struct S o; q_observe(&o); o.a = n + 1; }");
    assert_ne!(run.status, AnalysisStatus::Safe, "{run:?}");
    assert!(run.refuses_access(), "{run:?}");
}

#[test]
fn a_file_scope_record_object_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    let run = run("struct S g; void subject(int n) { q_observe(&g); g.a = n + 1; }");
    assert_ne!(run.status, AnalysisStatus::Safe, "{run:?}");
    assert!(run.refuses_access(), "{run:?}");
}

#[test]
fn a_hop_through_a_member_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    let run = run("void subject(int n) { struct S o; q_observe(&o); o.next->a = n + 1; }");
    assert_ne!(run.status, AnalysisStatus::Safe, "{run:?}");
    assert!(run.refuses_access(), "{run:?}");
}

#[test]
fn a_pointer_value_read_from_a_record_object_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    // `o.name` is a pointer: rule 12j reads scalars only.
    assert_refused(
        "int subject(void) { struct S o; q_observe(&o); return is_count(o.name) + o.a; }",
    );
}
