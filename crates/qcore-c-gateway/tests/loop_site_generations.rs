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

//! Card CG04-LOOP-SITE-GENERATIONS: a re-acquired site on a CFG cycle keeps
//! its earlier generations apart (`loop_generations.rs`,
//! `legacy_loop_generations.rs`), on route G.
//!
//! Every fixture runs the real Clang-backed gateway over the synthetic `q_*`,
//! `k_*` surfaces. `Safe` here is `Safe` over synthetic trusted test
//! assumptions and promotes nothing. "Before" in a comment is the base
//! `ca634dc1`, measured with the generation shift disabled.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
typedef __SIZE_TYPE__ size_t;
void *q_alloc(void); void q_free(void *p); void q_observe(void *p);
void *q_realloc(void *p, size_t n); void q_make(void **d);
void *q_maybe_alloc(void); void q_nullable_free(void *p);
int k_open(const char *path); int k_close(int fd);
";

const LOOP: &str = "unmodeled call, aggregate or indirect access in loop condition";
const RETAINED: &str = "loop exit edge retained: a modeled value cannot remove it";

fn contracts() -> ResourceContracts {
    let file = |name: &str, text: &str| (name.to_owned(), text.to_owned());
    load_resource_contract_sources(&BTreeMap::from([
        file("q_base.qv", include_str!("fixtures/contracts/q_base.qv")),
        file(
            "q_rich_modes.qv",
            include_str!("fixtures/contracts/q_rich_modes.qv"),
        ),
        file(
            "q_realloc.qv",
            include_str!("fixtures/contracts/q_realloc.qv"),
        ),
        file(
            "q_nullable.qv",
            include_str!("fixtures/contracts/q_nullable.qv"),
        ),
        file(
            "descriptor_generic.qv",
            include_str!("fixtures/contracts/descriptor_generic.qv"),
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
            eprintln!("skipping Clang-dependent CG04-LOOP-SITE-GENERATIONS test: {reason}");
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

    fn accused(&self) -> bool {
        self.diagnostics.iter().any(|kind| {
            !matches!(
                kind,
                LegacyDiagnosticKind::Unsupported { .. }
                    | LegacyDiagnosticKind::UnknownAlias { .. }
                    | LegacyDiagnosticKind::SummaryConflict { .. }
            )
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

/// `body` is the function `subject(int flag, int n)`'s body.
fn analyze(body: &str) -> Run {
    let source = format!("{DECLARATIONS}void subject(int flag, int n) {{\n{body}\n}}\n");
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

fn verdict(status: AnalysisStatus) -> &'static str {
    match status {
        AnalysisStatus::Safe => "safe",
        AnalysisStatus::Unsafe => "unsafe",
        _ => "unknown",
    }
}

/// `(shape, fixture, verdict)`: the pinned shapes, shared with
/// `QCore.LoopSiteGenerations.shapeVectors` (the model's verdict of the join
/// of each shape's path exits).
const SHAPE_VECTORS: &[(&str, &str, &str)] = &[
    (
        "for-break",
        "void *p;\nfor (;;) { p = q_maybe_alloc(); if (p == 0) break; q_observe(p); }",
        "unsafe",
    ),
    (
        "do-while",
        "void *p;\ndo { p = q_maybe_alloc(); if (p) q_observe(p); } while (p != 0);",
        "unsafe",
    ),
    (
        "release-each-iteration",
        "void *p;\nwhile (flag) { p = q_maybe_alloc(); if (p == 0) return; q_free(p); }",
        "safe",
    ),
    (
        "release-after-the-loop",
        "void *p = q_alloc();\nwhile (flag) { p = q_alloc(); }\nq_free(p);",
        "unsafe",
    ),
];

/// The two shapes of the defect were `Safe` before (every earlier generation
/// leaks); the other two are the controls. Each decides as the Lean model's
/// joined exit does.
#[test]
fn the_pinned_shapes_decide_as_the_model() {
    if !clang_available() {
        return;
    }
    for (shape, body, expected) in SHAPE_VECTORS {
        let run = analyze(body);
        assert_eq!(verdict(run.status), *expected, "{shape}: {run:?}");
        if *expected == "unsafe" {
            assert!(
                run.leaks() >= 1,
                "{shape}: the earlier generation leaks: {run:?}"
            );
        }
    }
}

const LEAN: &str = include_str!("../../../proof/QCore/LoopSiteGenerations.lean");

#[test]
fn cg04_lean_shape_vectors_match_the_rust_table() {
    let table = LEAN
        .split("def shapeVectors : List (String × Verdict) := [")
        .nth(1)
        .expect("table header");
    let table = &table[..table.find("\n]").expect("end of table")];
    let rows = table
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|row| {
            let row = row
                .trim_end_matches(',')
                .trim_start_matches('(')
                .trim_end_matches(')');
            let (shape, verdict) = row.split_once(',').expect("two cells");
            (
                shape.trim().trim_matches('"').to_owned(),
                verdict
                    .trim()
                    .trim_start_matches('.')
                    .trim_start_matches('«')
                    .trim_end_matches('»')
                    .to_owned(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), SHAPE_VECTORS.len());
    for ((shape, verdict), (rust_shape, _, rust_verdict)) in rows.iter().zip(SHAPE_VECTORS) {
        assert_eq!(
            (shape.as_str(), verdict.as_str()),
            (*rust_shape, *rust_verdict)
        );
    }
}

/// Control: acquire, release and re-acquire in one iteration, and a
/// `while (flag)` loop releasing each generation: every generation is
/// discharged.
#[test]
fn a_generation_released_in_its_iteration_stays_safe() {
    if !clang_available() {
        return;
    }
    for body in [
        "void *p;\nwhile (flag) { p = q_alloc(); q_observe(p); q_free(p); p = q_alloc(); q_free(p); }",
        "void *p;\nwhile (flag) { p = q_alloc(); q_free(p); }",
        "void *p;\nfor (int i = 0; i < n; i++) { p = q_maybe_alloc(); if (!p) continue; q_free(p); }",
    ] {
        let run = analyze(body);
        assert_eq!(run.status, AnalysisStatus::Safe, "{body}: {run:?}");
    }
}

/// Control: the previous generation is one object, so a release through a
/// binding that still names it is precise. Before the card the one site
/// merged it with the fresh one, and this correct program was accused of a
/// double release (a false `Unsafe`).
#[test]
fn the_previous_generation_is_released_precisely() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "void *prev = 0; void *cur;\n\
         while (flag) { cur = q_alloc(); if (prev) q_free(prev); prev = cur; }\n\
         if (prev) q_free(prev);",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
}

/// Control: acquisition on each iteration, release only after the loop.
/// Every generation but the last leaks, and the leak is reported.
#[test]
fn a_release_after_the_loop_reports_the_earlier_generations() {
    if !clang_available() {
        return;
    }
    let run = analyze("void *p;\nwhile (flag) { p = q_maybe_alloc(); }\nif (p) q_free(p);");
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
    assert!(run.leaks() >= 1, "{run:?}");
}

/// Control: reallocation in a loop.
///
/// * A buffer acquired and reallocated in each iteration: the reallocation's
///   failure edge keeps the old identity, and every generation is released.
/// * The growing buffer (`q = realloc(p, n); if (!q) …; p = q;`): at the loop
///   header `p` names two identities (the first allocation and the
///   reallocated one), so the flow refuses the reallocation, as it did
///   before. A refused re-acquisition leaves the generation the call may have
///   produced unresolved, so no leak is claimed for it: `Unknown`, not the
///   false `Unsafe` a joined state would otherwise report.
#[test]
fn a_reallocation_in_a_loop() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "void *p; void *q;\n\
         while (flag) { p = q_alloc(); q = q_realloc(p, 8); if (q == 0) { q_free(p); return; } q_free(q); }",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    let run = analyze(
        "void *p = q_alloc(); void *q;\n\
         while (flag) { q = q_realloc(p, 8); if (q == 0) { q_free(p); return; } p = q; }\n\
         q_free(p);",
    );
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert_eq!(run.leaks(), 0, "{run:?}");
    assert!(!run.accused(), "{run:?}");
}

/// Control: an `Out`-parameter acquisition in a loop. Released each
/// iteration it is `Safe`; overwritten each iteration, every earlier
/// generation leaks (before: `Unknown`, the earlier generations merged into
/// the one the release refused).
#[test]
fn an_out_parameter_acquisition_in_a_loop() {
    if !clang_available() {
        return;
    }
    let run = analyze("void *p;\nwhile (flag) { q_make(&p); q_free(p); }");
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    let run = analyze("void *p;\nq_make(&p);\nwhile (flag) { q_make(&p); }\nq_free(p);");
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
    assert!(run.leaks() >= 1, "{run:?}");
}

/// Control: a descriptor acquisition in a loop, split on its sentinel.
#[test]
fn a_descriptor_acquisition_in_a_loop() {
    if !clang_available() {
        return;
    }
    let run =
        analyze("int fd;\nwhile (flag) { fd = k_open(\"x\"); if (fd < 0) break; k_close(fd); }");
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    let run = analyze("int fd;\nwhile (flag) { fd = k_open(\"x\"); if (fd < 0) break; }");
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
    assert_eq!(run.leaks(), 1, "{run:?}");
}

/// Rule 10x's condition form once its placement guard is lifted (the second,
/// separately measured step of the card): a custody split on a CFG cycle
/// decides as the statement form does, each earlier generation apart.
#[test]
fn the_condition_form_is_split_on_a_cycle() {
    if !clang_available() {
        return;
    }
    for (body, expected, leaks) in [
        (
            "void *p;\nwhile ((p = q_maybe_alloc()) != 0) { q_free(p); }",
            AnalysisStatus::Safe,
            0,
        ),
        (
            "void *p;\nwhile ((p = q_maybe_alloc()) != 0) { q_observe(p); }",
            AnalysisStatus::Unsafe,
            1,
        ),
        (
            "for (int i = 0; i < n; i++) {\nvoid *p;\nif ((p = q_maybe_alloc()) == 0) { break; }\nq_observe(p);\n}",
            AnalysisStatus::Unsafe,
            1,
        ),
        (
            "void *p;\nretry:\nif ((p = q_maybe_alloc()) == 0) { goto retry; }\nq_free(p);",
            AnalysisStatus::Safe,
            0,
        ),
        (
            "int fd;\nwhile ((fd = k_open(\"x\")) >= 0) { k_close(fd); }",
            AnalysisStatus::Safe,
            0,
        ),
    ] {
        let run = analyze(body);
        assert!(!run.refused(LOOP), "{body}: the split is admitted: {run:?}");
        assert_eq!(run.status, expected, "{body}: {run:?}");
        assert_eq!(run.leaks(), leaks, "{body}: {run:?}");
    }
    // A non-null allocation in a loop condition: the exit edge the value
    // route calls infeasible is retained with the header's state (rule
    // SCALAR-ROUTE-EXIT-EDGE), exactly as for its statement pair
    // `p = q_alloc(); while (p != 0) { … }`; neither is `Safe`.
    let run = analyze("void *p;\nwhile ((p = q_alloc()) != 0) { q_free(p); }");
    assert!(run.refused(RETAINED), "{run:?}");
    assert_ne!(run.status, AnalysisStatus::Safe, "{run:?}");
    let pair = analyze("void *p = q_alloc();\nwhile (p != 0) { q_free(p); p = q_alloc(); }");
    assert_ne!(pair.status, AnalysisStatus::Safe, "{pair:?}");
}

const K_DECLARATIONS: &str = "\
struct k_item { int weight; }; \
struct k_box { int flag; struct k_item *item; struct k_item *spare; }; \
struct k_box *k_make(int n); \
struct k_item *k_take(int n); \
void k_drop(struct k_box *b); \
void k_release(struct k_item *i);";

fn analyze_k(body: &str) -> Run {
    let source = format!("{K_DECLARATIONS} {body}");
    let contracts = load_resource_contract_sources(&BTreeMap::from([(
        "o04_b08_generic.qv".to_owned(),
        include_str!("fixtures/contracts/o04_b08_generic.qv").to_owned(),
    )]))
    .expect("the k contracts load");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "k_chain",
        &contracts,
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected by Clang: {error:?}\n{source}"));
    let result = &summary_of(&outcome).functions["k_chain"];
    Run {
        status: result.status,
        diagnostics: result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect(),
    }
}

/// The sibling this card found and registered
/// (`B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED`), flipped by that card (gateway
/// rule 11c). The B08 join marked a child linked on one side only as
/// unresolved, which suppressed its leak, and no diagnostic reported it, so
/// the branch form was `Safe`. Under P-B08-SHALLOW the member's child leaks
/// when the owner is released; the repaired join adds no unresolved effect,
/// so the branch form reports the straight-line form's leak.
///
/// The loop form was `Safe` on CG04's base too (every stored generation but
/// the last is overwritten in the member and leaks, and the last one leaks
/// with the owner's release). With generations and the old join, the live
/// previous generation met the summary the join left unresolved and the
/// merge refused (rule 11a): `Unknown`. With the repaired join nothing is
/// left unresolved, the earlier generations merge into a live summary no
/// place holds, and the leak is reported: `Unsafe`, the source's own leak.
#[test]
fn the_field_link_join_sibling_stays_fixed() {
    if !clang_available() {
        return;
    }
    let straight = analyze_k(
        "void k_chain(int n) { struct k_item *it; struct k_box *b; \
         b = k_make(n); if (!b) return; \
         it = k_take(n); if (!it) { k_drop(b); return; } b->item = it; \
         k_drop(b); }",
    );
    assert_eq!(straight.status, AnalysisStatus::Unsafe, "{straight:?}");
    let branch = analyze_k(
        "void k_chain(int n) { struct k_item *it; struct k_box *b; \
         b = k_make(n); if (!b) return; \
         it = k_take(n); if (!it) { k_drop(b); return; } \
         if (n) { b->item = it; } else { k_release(it); } \
         k_drop(b); }",
    );
    assert_eq!(branch.status, AnalysisStatus::Unsafe, "{branch:?}");
    assert_eq!(
        branch.leaks(),
        straight.leaks(),
        "{branch:?} vs {straight:?}"
    );
    let looped = analyze_k(
        "void k_chain(int n) { struct k_item *it; struct k_box *b; int i; \
         b = k_make(n); if (!b) return; \
         for (i = 0; i < n; i++) { it = k_take(n); if (!it) break; b->item = it; } \
         k_drop(b); }",
    );
    assert_eq!(looped.status, AnalysisStatus::Unsafe, "{looped:?}");
    assert!(looped.leaks() >= 1, "{looped:?}");
    assert!(
        !looped
            .refused("loop re-acquisition: an earlier generation that may be live is still held"),
        "{looped:?}"
    );
}
