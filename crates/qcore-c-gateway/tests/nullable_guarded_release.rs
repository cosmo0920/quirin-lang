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

//! Card NULLABLE-GUARDED-RELEASE (gateway rule 74c): a refused release is
//! no leak, and a guarded release of a binding the sparing frame keeps is
//! decided.
//!
//! Every fixture runs the real Clang-backed gateway over the synthetic `q_*`
//! surface (`q_alloc` owned non-null, `q_free` released non-null) and
//! uncontracted `u_*` calls, which are refused statements. `Safe` here is
//! `Safe` over synthetic trusted test assumptions and promotes nothing.
//!
//! The Metadata shape is Fluent Bit librdkafka's
//! `rd_kafka_mock_handle_Metadata` (`rdkafka_mock_handlers.c:1362-1494`):
//! a NULL-initialised list built only when a count is positive, read through
//! `&p->elems[i]` beside refused statements, released under `if (p)` on both
//! exits.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *p);
struct L { int cnt; int *elems; };
int u_read(void); void u_use(int *x); void u_take(void **slot); void *u_get(void);
";

const NULLABLE: &str = "nullable resource `p` used by a non-null contract";

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
            eprintln!("skipping Clang-dependent test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
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

    fn unresolved(&self, name: &str) -> bool {
        self.diagnostics.iter().any(|kind| {
            matches!(kind, LegacyDiagnosticKind::UnknownAlias { variable } if variable == name)
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

/// `body` is `int subject(int flag, int n)`'s body.
fn analyze(body: &str) -> Run {
    let source = format!("{DECLARATIONS}int subject(int flag, int n) {{\n{body}\n}}\n");
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
    Run {
        status: result.status,
        diagnostics: result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect(),
    }
}

/// The Metadata shape: refused statements beside `&p->elems[i]` no longer
/// poison `p`, so both guards decide, both `nonnull` releases are accepted
/// and the NULL path owes nothing. The row stays `Unknown` on the refused
/// statements' own leaves, with no leak and no custody leaf on `p`.
#[test]
fn the_metadata_shape_is_leak_free() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "struct L *p = 0;\nint i;\n\
         if (u_read()) goto err;\n\
         if (n > 0) p = q_alloc();\n\
         for (i = 0; i < n; i++) { if (u_read()) goto err; }\n\
         if (flag) { i = 0; } else if (p) { u_use(&p->elems[0]); }\n\
         u_read();\n\
         if (p) q_free(p);\n\
         return 0;\n\
         err:\n\
         if (p) q_free(p);\n\
         return -1;",
    );
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert_eq!(run.leaks(), 0, "{run:?}");
    assert!(!run.refused(NULLABLE), "{run:?}");
    assert!(!run.unresolved("p"), "{run:?}");
}

/// The same shape with no refused statement is `Safe`: the guard refinement
/// was already there; the frame only lets the binding reach it.
#[test]
fn the_metadata_shape_without_refusals_is_safe() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "struct L *p = 0;\n\
         if (n > 0) p = q_alloc();\n\
         if (flag) goto err;\n\
         if (p) q_free(p);\n\
         return 0;\n\
         err:\n\
         if (p) q_free(p);\n\
         return -1;",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
}

/// An unguarded `nonnull` release of a nullable resource is still refused:
/// the row is `Unknown` on the nullable leaf and no leak is claimed.
#[test]
fn an_unguarded_nullable_release_is_refused_without_a_leak() {
    if !clang_available() {
        return;
    }
    let run = analyze("void *p = 0;\nif (n > 0) p = q_alloc();\nq_free(p);\nreturn 0;");
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(run.refused(NULLABLE), "{run:?}");
    assert_eq!(run.leaks(), 0, "{run:?}");
}

/// A reassignment between the guard and the release: the released value is
/// not the guarded site, so the release is refused (the site may be owed),
/// and the refusal claims nothing.
#[test]
fn a_reassignment_between_guard_and_release_is_refused() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "void *p = 0;\nif (n > 0) p = q_alloc();\n\
         if (p) { p = u_get(); q_free(p); }\nreturn 0;",
    );
    assert_ne!(run.status, AnalysisStatus::Safe, "{run:?}");
    assert!(run.unresolved("p") || run.refused(NULLABLE), "{run:?}");
}

/// An escaped slot (`&p` handed to a refused call) is not spared: the guard
/// and the release are refused; nothing is admitted and no leak is claimed.
#[test]
fn an_escaped_slot_is_refused() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "void *p = 0;\nif (n > 0) p = q_alloc();\nu_take(&p);\n\
         if (p) q_free(p);\nreturn 0;",
    );
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(run.unresolved("p"), "{run:?}");
    assert_eq!(run.leaks(), 0, "{run:?}");
}

/// A slot aliased through a pointer to it (`void **pp = &p;`) is not spared
/// either.
#[test]
fn an_aliased_slot_is_refused() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "void *p = 0;\nvoid **pp = &p;\nif (n > 0) p = q_alloc();\nu_read();\n\
         if (p) q_free(*pp);\nreturn 0;",
    );
    assert_ne!(run.status, AnalysisStatus::Safe, "{run:?}");
}

/// A refused guard (the slot escaped) and the refused release behind it:
/// the release marks the site, and no exit claims it. The schedule-dependent
/// half of rule 74c's part A (an iterate reaching the exit before the marker)
/// is pinned on the settled states directly in
/// `tests/unit/legacy_settled_leak.rs`; this fixture pins the outcome.
#[test]
fn a_refused_guard_and_release_claim_nothing() {
    if !clang_available() {
        return;
    }
    let run =
        analyze("void *p = q_alloc();\nu_take(&p);\nif (!p) { } else { q_free(p); }\nreturn 0;");
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(run.unresolved("p"), "{run:?}");
    assert_eq!(run.leaks(), 0, "{run:?}");
}

/// A genuine leak on the non-NULL path stays a provisional `Unsafe`, a
/// refused statement beside it or not.
#[test]
fn a_genuine_leak_on_the_non_null_path_stays_unsafe() {
    if !clang_available() {
        return;
    }
    for body in [
        "void *p = 0;\nif (n > 0) p = q_alloc();\nif (flag) return 1;\nif (p) q_free(p);\nreturn 0;",
        "void *p = 0;\nif (n > 0) p = q_alloc();\nu_read();\nif (flag) return 1;\n\
         if (p) q_free(p);\nreturn 0;",
    ] {
        let run = analyze(body);
        assert_eq!(run.status, AnalysisStatus::Unsafe, "{body}: {run:?}");
        assert_eq!(run.leaks(), 1, "{body}: {run:?}");
    }
}
