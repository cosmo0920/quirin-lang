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

//! Card NULLABLE-LAZY-CONSTRUCTOR-IN-LOOP (gateway rule 74d): the lazy
//! constructor guard `if (!p) p = ctor();` is decided in a loop and outside
//! one, and a resolved, possibly-null resource returned under a non-null
//! result mode is refused as exactly that.
//!
//! Every fixture runs the real Clang-backed gateway over the synthetic `q_*`
//! surface (`q_alloc` owned non-null, `q_free` released non-null), a
//! test-local borrowed `l_use`, and uncontracted `u_*` calls, which are refused
//! statements. `Safe` here is `Safe` over synthetic trusted test assumptions
//! and promotes nothing.
//!
//! The loop shape is Fluent Bit librdkafka's
//! `rd_kafka_cgrp_owned_but_not_exist_partitions` (`rdkafka_cgrp.c:7595`):
//! a NULL-initialised list built lazily inside a loop beside refused calls and
//! returned, NULL when no partition qualified. The return shape without a
//! lazy guard is `rd_kafka_cgrp_calculate_subscribe_revoking_partitions`
//! (`rdkafka_cgrp.c:5921`): a list built non-null, destroyed and set to NULL
//! when empty, and returned.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *p); void q_transfer(void *p); void l_use(void *p);
int u_read(void); void u_use(void *x); void u_take(void **slot); void *u_get(void);
";

/// A borrowed, non-owning use: the lazily built name passed on each
/// iteration, as `rd_kafka_topic_partition_list_add_copy(result, curr)` is.
const L_USE: &str = "module contract;
extern fn l_use(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_return_none();
};
";

const NULLABLE: &str = "nullable resource `r` used by a non-null contract";
const NULL_RETURN: &str = "possibly-null resource `r` returned under a non-null result mode";

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
    Run {
        status: result.status,
        diagnostics: result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect(),
    }
}

/// Lazy construction inside a loop, a borrowed use on every iteration, and a
/// guarded release after the loop: the guard's non-null edge keeps the
/// earlier site, its null edge constructs, the use sees a non-null name, and
/// the release is decided by rules 14l and 10b. No new machinery: this was
/// already `Safe` at the base.
#[test]
fn lazy_construction_in_a_loop_is_decided() {
    if !clang_available() {
        return;
    }
    for test in ["!r", "r == 0"] {
        let run = analyze(
            "int subject(int n)",
            &format!(
                "void *r = 0; int i;\nfor (i = 0; i < n; i++) {{\nif ({test}) r = q_alloc();\n\
                 l_use(r);\n}}\nif (r) q_free(r);\nreturn 0;"
            ),
        );
        assert_eq!(run.status, AnalysisStatus::Safe, "{test}: {run:?}");
    }
}

/// The same loop with a release and reset on every iteration: each back edge
/// carries NULL, and each iteration's construction is discharged.
#[test]
fn lazy_construction_released_per_iteration_is_decided() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "int subject(int n)",
        "void *r = 0; int i;\nfor (i = 0; i < n; i++) {\nif (!r) r = q_alloc();\n\
         q_free(r); r = 0;\n}\nreturn 0;",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
}

/// Lazy construction outside a loop: after the guard the name is non-null and
/// denotes the one site, so the unguarded `nonnull` release is accepted, beside
/// a refused statement or alone.
#[test]
fn lazy_construction_outside_a_loop_is_decided() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "int subject(int n)",
        "void *r = 0;\nif (n) u_read();\nif (!r) r = q_alloc();\nq_free(r);\nreturn 0;",
    );
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(!run.refused(NULLABLE), "{run:?}");
    assert!(!run.unresolved("r"), "{run:?}");
    assert_eq!(run.leaks(), 0, "{run:?}");
    let run = analyze(
        "int subject(int n)",
        "void *r = 0;\nif (!r) r = q_alloc();\nq_free(r);\nreturn 0;",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
}

/// Outside a loop with an earlier construction on one path, the guard leaves
/// the name non-null (no nullable leaf), but it denotes one of two sites, so
/// the release is refused on identity, not on nullability, and claims no
/// leak.
#[test]
fn a_two_site_lazy_guard_refuses_on_identity_only() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "int subject(int n)",
        "void *r = 0;\nif (n) r = q_alloc();\nif (!r) r = q_alloc();\nq_free(r);\nreturn 0;",
    );
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(!run.refused(NULLABLE), "{run:?}");
    assert!(run.unresolved("r"), "{run:?}");
    assert_eq!(run.leaks(), 0, "{run:?}");
}

/// A reassignment after the guard, inside the loop or outside one: the value
/// released is not the constructed site, so the release is refused and no
/// leak is claimed.
#[test]
fn a_reassignment_after_the_guard_is_refused() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "int subject(int n)",
        "void *r = 0; int i;\nfor (i = 0; i < n; i++) {\nif (!r) r = q_alloc();\nr = u_get();\n}\n\
         if (r) q_free(r);\nreturn 0;",
    );
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(run.unresolved("r"), "{run:?}");
    assert_eq!(run.leaks(), 0, "{run:?}");
    let run = analyze(
        "int subject(int n)",
        "void *r = 0;\nif (!r) r = q_alloc();\nr = u_get();\nq_free(r);\nreturn 0;",
    );
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(run.unresolved("r"), "{run:?}");
    assert_eq!(run.leaks(), 0, "{run:?}");
}

/// An address-taken name: a refused call may store anything through `&r`, so
/// the guard fact does not survive and the release is refused.
#[test]
fn an_escaped_slot_after_the_guard_is_refused() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "int subject(int n)",
        "void *r = 0; int i;\nfor (i = 0; i < n; i++) {\nif (!r) r = q_alloc();\nu_take(&r);\n}\n\
         if (r) q_free(r);\nreturn 0;",
    );
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(run.unresolved("r"), "{run:?}");
    assert_eq!(run.leaks(), 0, "{run:?}");
}

/// A release of the possibly-null value outside any guard after the loop:
/// `r` is NULL when the loop never ran (the loop head joins the NULL entry
/// edge), so the `nonnull` release is refused, and claims no leak.
#[test]
fn a_release_outside_any_guard_is_refused() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "int subject(int n)",
        "void *r = 0; int i;\nfor (i = 0; i < n; i++) {\nif (!r) r = q_alloc();\n}\n\
         q_free(r);\nreturn 0;",
    );
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(run.refused(NULLABLE), "{run:?}");
    assert_eq!(run.leaks(), 0, "{run:?}");
}

/// A genuine leak in a lazily constructed loop: the early return leaves the
/// constructed site unreleased. It stays `Unsafe`.
#[test]
fn a_genuine_leak_in_a_lazy_loop_stays_unsafe() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "int subject(int n)",
        "void *r = 0; int i;\nfor (i = 0; i < n; i++) {\nif (!r) r = q_alloc();\n\
         if (i == 3) return -1;\n}\nif (r) q_free(r);\nreturn 0;",
    );
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
    assert_eq!(run.leaks(), 1, "{run:?}");
}

/// The returned lazy loop with no refusal: the in-unit summary validates
/// `Owned { nullable: true }`, so the possibly-null return is accepted.
#[test]
fn a_returned_lazy_loop_validates_a_nullable_summary() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "void *subject(int n)",
        "void *r = 0; int i;\nfor (i = 0; i < n; i++) {\nif (!r) r = q_alloc();\nl_use(r);\n}\nreturn r;",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
}

/// The `owned_but_not_exist` shape: a refused call blocks the summary, the
/// contract-free run's result mode is non-null, and the return of a NULL-or-
/// site binding is refused. The leaf names that, not a contract no call
/// carries; no leak is claimed. The base reported `UnknownAlias r` and the
/// nullable leaf here. `UnknownAlias r` remains: on an iterate before the back
/// edge joined, only the NULL entry edge had reached the return (a binding
/// with no target, which keeps the old refusal), and rule 6's per-iterate
/// reporting keeps that leaf.
#[test]
fn a_possibly_null_return_under_a_non_null_mode_is_named() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "void *subject(int n)",
        "void *r = 0; int i;\nfor (i = 0; i < n; i++) {\nif (!r) r = q_alloc();\nu_use(r);\n}\nreturn r;",
    );
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(run.refused(NULL_RETURN), "{run:?}");
    assert!(!run.refused(NULLABLE), "{run:?}");
    assert!(run.unresolved("r"), "{run:?}");
    assert_eq!(run.leaks(), 0, "{run:?}");
}

/// The `revoking` shape: built non-null, destroyed and set to NULL when
/// empty, returned beside a refused call. Same refusal, same name.
#[test]
fn a_destroyed_and_nulled_return_is_named() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "void *subject(int n)",
        "void *r = q_alloc(); int i;\nfor (i = 0; i < n; i++) {\nu_use(r);\n}\n\
         if (n == 0) { q_free(r); r = 0; }\nreturn r;",
    );
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(run.refused(NULL_RETURN), "{run:?}");
    assert!(!run.refused(NULLABLE), "{run:?}");
    assert!(!run.unresolved("r"), "{run:?}");
    assert_eq!(run.leaks(), 0, "{run:?}");
}

/// The rename touches only a resolved binding: a poisoned (unknown) binding
/// returned keeps its identity refusal.
#[test]
fn an_unknown_binding_returned_keeps_its_identity_refusal() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "void *subject(int n)",
        "void *r = q_alloc();\nu_take(&r);\nreturn r;",
    );
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(run.unresolved("r"), "{run:?}");
    assert!(!run.refused(NULL_RETURN), "{run:?}");
    assert_eq!(run.leaks(), 0, "{run:?}");
}

/// The other rows' shapes stay refused with no leak; each names its obstacle
/// in another leaf, and is recorded as a follow-up, not refined here.
///
/// * A later declarator's NULL initialiser (`T *a = 0, *b = 0;`, the
///   `rd_kafka_mock_handle_ConsumerGroupHeartbeat` declaration) was read as a
///   store through `*b` (`indirect memory access`), so `b` had no binding on
///   the path that skips its construction, the join made it unknown, and the
///   guarded release was refused. Deliberate pin update, card
///   LATER-DECLARATOR-INITIALISER-UNREAD (rule 74k): each init-declarator is
///   now read as its own initialisation, so the declaration has the verdict
///   of the same body with one declarator per statement, `Safe` (was
///   `Unknown` with the access, nullable and identity leaves).
/// * A join with an uncontracted pointer result
///   (`rd_kafka_mock_cgrp_consumer_member_needs_revocation`,
///   `rd_kafka_cgrp_handle_SyncGroup_memberstate`): the uncontracted arm
///   leaves the name unknown.
/// * A release correlated with a flag recording which value the name holds
///   (`balance`'s copy-on-write `leftoverUnassignedPartitions`).
#[test]
fn the_other_shapes_stay_refused_without_a_leak() {
    if !clang_available() {
        return;
    }
    let declarator = analyze(
        "int subject(int n)",
        "void *a = 0, *r = 0;\nif (n) r = q_alloc();\nif (r) q_free(r);\nreturn 0;",
    );
    assert_eq!(declarator.status, AnalysisStatus::Safe, "{declarator:?}");
    let split = analyze(
        "int subject(int n)",
        "void *a = 0; void *r = 0;\nif (n) r = q_alloc();\nif (r) q_free(r);\nreturn 0;",
    );
    assert_eq!(split.status, AnalysisStatus::Safe, "{split:?}");
    let uncontracted = analyze(
        "int subject(int n)",
        "void *r;\nif (n) r = u_get(); else r = q_alloc();\nq_free(r);\nreturn 0;",
    );
    assert_eq!(
        uncontracted.status,
        AnalysisStatus::Unknown,
        "{uncontracted:?}"
    );
    assert!(uncontracted.unresolved("r"), "{uncontracted:?}");
    assert_eq!(uncontracted.leaks(), 0, "{uncontracted:?}");
    let flag = analyze(
        "int subject(void *s, int n)",
        "void *r = s; int made = 0;\nif (n) { made = 1; r = q_alloc(); }\nl_use(r);\n\
         if (made) q_free(r);\nreturn 0;",
    );
    assert_eq!(flag.status, AnalysisStatus::Unknown, "{flag:?}");
    assert_eq!(flag.leaks(), 0, "{flag:?}");
}

/// The `rd_kafka_mock_handle_ShareFetch` shape (rule 74d (C)): the lexer
/// spells `flag && p` as `flag & & p`, and rule 10a's frame read the second
/// `&` as taking `p`'s address, so every refused statement poisoned `p`, the
/// guard was refused and the guarded `nonnull` release left the nullable leaf.
/// An even run of `&` takes no address: `p` survives, the guard decides, and
/// the row is `Unknown` on the refused statements alone, with no leak.
#[test]
fn a_logical_and_takes_no_address() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "int subject(int flag, int n)",
        "void *p = 0;\nif (n > 0) p = q_alloc();\nu_read();\nif (flag && p) l_use(p);\n\
         u_read();\nif (p) q_free(p);\nreturn 0;",
    );
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(
        !run.refused("nullable resource `p` used by a non-null contract"),
        "{run:?}"
    );
    assert!(!run.unresolved("p"), "{run:?}");
    assert_eq!(run.leaks(), 0, "{run:?}");
    let run = analyze(
        "int subject(int flag, int n)",
        "void *p = 0;\nif (n > 0) p = q_alloc();\nif (flag && p) l_use(p);\nif (p) q_free(p);\nreturn 0;",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
}

/// An odd run still ends in the address operator: `flag && &p` takes `p`'s
/// address, so a refused statement may write `p` through it, and the guarded
/// release is refused (no leak).
#[test]
fn an_odd_run_still_takes_the_address() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "int subject(int flag, int n)",
        "void *p = 0; int t;\nif (n > 0) p = q_alloc();\nt = flag && &p;\nu_read();\n\
         if (p) q_free(p);\nreturn t;",
    );
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(run.unresolved("p"), "{run:?}");
    assert_eq!(run.leaks(), 0, "{run:?}");
}
