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

//! Card LATER-DECLARATOR-INITIALISER-UNREAD (gateway rule 74k): a declaration
//! of several init-declarators reads as the same declarations written one per
//! statement, or stays refused.
//!
//! Every fixture runs the real Clang-backed gateway over the synthetic `q_*`
//! surface (`q_alloc` owned non-null, `q_free` released non-null), a
//! test-local borrowed `l_use`, and uncontracted `u_*` calls, which are refused
//! statements. `Safe` here is `Safe` over synthetic trusted test assumptions
//! and promotes nothing.
//!
//! The shape is Fluent Bit librdkafka's
//! `rd_kafka_mock_handle_ConsumerGroupHeartbeat`
//! (`rdkafka_mock_handlers.c:2894-2896`): three NULL-initialised list
//! pointers in one declaration, one of them built on a path and released by
//! `RD_IF_FREE` (`if (p) destroy(p)`).

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
#define NULL ((void *)0)
#define RD_IF_FREE(PTR, FUNC) do { if ((PTR)) FUNC(PTR); } while (0)
typedef struct list_s list_t;
void *q_alloc(void); void q_free(void *p); void l_use(void *p);
int u_read(void); void u_use(void *x); void *u_get(void); void *g(void);
";

const L_USE: &str = "module contract;
extern fn l_use(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_return_none();
};
";

const INDIRECT: &str = "indirect memory access is outside the resource projection";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        (
            "q_base.qv".to_owned(),
            include_str!("fixtures/contracts/q_base.qv").to_owned(),
        ),
        ("l_use.qv".to_owned(), L_USE.to_owned()),
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
            eprintln!("skipping Clang-dependent test: {reason}");
            false
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Run {
    status: AnalysisStatus,
    diagnostics: Vec<LegacyDiagnosticKind>,
}

impl Run {
    fn leaks(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. }))
            .count()
    }

    fn refused(&self, description: &str) -> bool {
        self.diagnostics.iter().any(|kind| {
            matches!(kind, LegacyDiagnosticKind::Unsupported { description: d } if d == description)
        })
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => {
            partial.as_ref().expect("these fixtures project partially")
        }
    }
}

/// `subject`'s definition is `signature { body }`.
fn analyze(signature: &str, body: &str) -> Run {
    let source = format!("{DECLARATIONS}{signature} {{\n{body}\n}}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected by Clang: {error:?}\n{source}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let result = &summary_of(&outcome).functions["subject"];
    let mut diagnostics = result
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.kind.clone())
        .collect::<Vec<_>>();
    diagnostics.sort_by_key(|kind| format!("{kind:?}"));
    Run {
        status: result.status,
        diagnostics,
    }
}

/// The acceptance test: the declaration and the same declarations written one
/// per statement (on the same line, so no line number differs) read alike.
fn equivalent(signature: &str, declaration: &str, split: &str, rest: &str) -> Run {
    let joined = analyze(signature, &format!("{declaration}\n{rest}"));
    let one_each = analyze(signature, &format!("{split}\n{rest}"));
    assert_eq!(joined, one_each, "{declaration} versus {split}");
    joined
}

/// ConsumerGroupHeartbeat: three NULL-initialised typedef'd list pointers, the
/// middle one built on one path and released by `RD_IF_FREE`. The later
/// declarators were stores through `*existing`, so the name had no binding on
/// the skipping path and the guard was refused. One per statement it is Safe;
/// so is the declaration now.
#[test]
fn the_heartbeat_declaration_reads_as_one_per_statement() {
    if !clang_available() {
        return;
    }
    let run = equivalent(
        "int subject(int n)",
        "list_t *sent = NULL, *existing = NULL, *next = NULL;",
        "list_t *sent = NULL; list_t *existing = NULL; list_t *next = NULL;",
        "if (n) existing = q_alloc();\nl_use(sent);\nl_use(next);\n\
         RD_IF_FREE(existing, q_free);\nreturn 0;",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
}

/// The NULLABLE-LAZY-CONSTRUCTOR-IN-LOOP pin `void *a = 0, *r = 0;`, now the
/// one-declarator-per-statement verdict.
#[test]
fn two_null_initialised_pointers_read_as_two_declarations() {
    if !clang_available() {
        return;
    }
    let run = equivalent(
        "int subject(int n)",
        "void *a = 0, *r = 0;",
        "void *a = 0; void *r = 0;",
        "if (n) r = q_alloc();\nif (r) q_free(r);\nreturn 0;",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    assert!(!run.refused(INDIRECT), "{run:?}");
}

/// A genuine leak behind a later declarator is found, as it is one per
/// statement: the reading is exact in both directions.
#[test]
fn a_leak_behind_a_later_declarator_is_found() {
    if !clang_available() {
        return;
    }
    let run = equivalent(
        "int subject(int n)",
        "void *a = 0, *r = q_alloc();",
        "void *a = 0; void *r = q_alloc();",
        "if (n) return 1;\nq_free(r);\nreturn 0;",
    );
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
    assert_eq!(run.leaks(), 1, "{run:?}");
}

/// Mixed declarators: a scalar beside a pointer, an uninitialised pointer
/// before an initialised one, and an alias of an earlier declarator.
#[test]
fn mixed_declarators_read_as_one_per_statement() {
    if !clang_available() {
        return;
    }
    let run = equivalent(
        "int subject(int *q)",
        "int n = 0, *p = q;",
        "int n = 0; int *p = q;",
        "if (n) l_use(p);\nreturn 0;",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    let run = equivalent(
        "int subject(void)",
        "void *a, *b = q_alloc();",
        "void *a; void *b = q_alloc();",
        "q_free(b);\nreturn 0;",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    let run = equivalent(
        "int subject(void)",
        "list_t *a, *b = q_alloc();",
        "list_t *a; list_t *b = q_alloc();",
        "q_free(b);\nreturn 0;",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    let run = equivalent(
        "int subject(void)",
        "void *a = q_alloc(), *b = a;",
        "void *a = q_alloc(); void *b = a;",
        "q_free(b);\nreturn 0;",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
}

/// A later declarator the reader does not model keeps the whole declaration
/// refused, with no leak: an array, a function pointer, a parenthesised
/// declarator.
#[test]
fn unmodelled_declarators_stay_refused() {
    if !clang_available() {
        return;
    }
    for declaration in [
        "char *a = 0, b[4];",
        "void *a = 0, *(*fp)(void) = g;",
        "void *a = 0, *(r) = 0;",
    ] {
        let run = analyze(
            "int subject(int n)",
            &format!(
                "{declaration}\nvoid *r2 = 0;\nif (n) r2 = q_alloc();\nif (r2) q_free(r2);\nl_use(a);\nreturn 0;"
            ),
        );
        assert_eq!(
            run.status,
            AnalysisStatus::Unknown,
            "{declaration}: {run:?}"
        );
        assert_eq!(run.leaks(), 0, "{declaration}: {run:?}");
    }
}

/// An initialiser the reader cannot read stays refused, one per statement as
/// in the declaration: an uncontracted call.
#[test]
fn an_unreadable_initialiser_stays_refused() {
    if !clang_available() {
        return;
    }
    let run = equivalent(
        "int subject(void)",
        "void *a = 0, *r = u_get();",
        "void *a = 0; void *r = u_get();",
        "l_use(a);\nreturn 0;",
    );
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert_eq!(run.leaks(), 0, "{run:?}");
}

/// Uninitialised typedef'd declarators (rule 38h's admission) stay admitted.
#[test]
fn uninitialised_declarators_stay_admitted() {
    if !clang_available() {
        return;
    }
    let run = equivalent(
        "int subject(void)",
        "list_t *a, *b;",
        "list_t *a; list_t *b;",
        "a = q_alloc();\nb = a;\nq_free(b);\nreturn 0;",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
}

/// Declarators the decision does not admit keep the whole declaration on its
/// old reading (the access refusal): a cast that is not the null constant
/// (rule 12h's repair reads it only from the statement's own Clang node), a
/// scalar initialiser that is not a numeric literal, and a `static` object
/// (rule 11h's no-event declaration; the store into the static object is
/// refused as a publication, as before).
#[test]
fn declarations_the_decision_refuses_keep_the_old_reading() {
    if !clang_available() {
        return;
    }
    for (declaration, access) in [
        ("void *a = 0, *r = (void *) s;", true),
        ("int n = k, *r = 0;", true),
        ("static void *a = 0, *r = 0;", false),
    ] {
        let run = analyze(
            "int subject(void *s, int k)",
            &format!("{declaration}\nr = q_alloc();\nq_free(r);\nreturn 0;"),
        );
        assert_eq!(
            run.status,
            AnalysisStatus::Unknown,
            "{declaration}: {run:?}"
        );
        assert_eq!(run.refused(INDIRECT), access, "{declaration}: {run:?}");
        assert_eq!(run.leaks(), 0, "{declaration}: {run:?}");
    }
}
