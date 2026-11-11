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

//! Card B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED (gateway rule 11c): a B08 field
//! link that holds on some paths only never leaves its child's obligation
//! unresolved without a diagnostic.
//!
//! Every fixture runs the real Clang-backed gateway over the synthetic `k_*`
//! surface (`o04_b08_generic.qv`). `Safe` here is `Safe` over synthetic
//! trusted test assumptions and promotes nothing. "Base" in a comment is
//! `a235ead6`, before the repair.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const K_DECLARATIONS: &str = "\
struct k_item { int weight; }; \
struct k_box { int flag; struct k_item *item; struct k_item *spare; }; \
struct k_box *k_make(int n); \
struct k_item *k_take(int n); \
void k_drop(struct k_box *b); \
void k_release(struct k_item *i); \
void k_adopt(struct k_item *i);";

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent B08-FIELD-LINK-JOIN test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Run {
    status: AnalysisStatus,
    diagnostics: Vec<(usize, LegacyDiagnosticKind)>,
    /// The functions whose W4 summaries a `Safe` certificate relied on.
    summaries: Vec<String>,
}

impl Run {
    fn leaks(&self) -> Vec<(usize, usize)> {
        self.diagnostics
            .iter()
            .filter_map(|(line, kind)| match kind {
                LegacyDiagnosticKind::Leak { acquisition_line } => Some((*line, *acquisition_line)),
                _ => None,
            })
            .collect()
    }

    fn refused(&self, description: &str) -> bool {
        self.diagnostics.iter().any(|(_, kind)| {
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

/// Analyze `function` of the unit `K_DECLARATIONS` + `body`. Each statement of
/// `body` is on its own line, so a diagnostic's line names it.
fn analyze_in(body: &str, function: &str) -> Run {
    let source = format!("{K_DECLARATIONS}\n{body}\n");
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
        function,
        &contracts,
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected by Clang: {error:?}\n{source}"));
    let summaries = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
            certificate
                .summaries
                .iter()
                .map(|record| record.function.clone())
                .collect()
        }
        _ => Vec::new(),
    };
    let result = &summary_of(&outcome).functions[function];
    Run {
        status: result.status,
        diagnostics: result
            .diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.line, diagnostic.kind.clone()))
            .collect(),
        summaries,
    }
}

fn analyze(body: &str) -> Run {
    analyze_in(body, "k_chain")
}

/// The prologue every fixture shares: an owner and a child, both checked.
/// Lines 3..=8 of the unit.
const PROLOGUE: &str = "\
struct k_item *it;
struct k_box *b;
b = k_make(n); if (!b) return RET;
it = k_take(n);
if (!it) { k_drop(b); return RET; }";

fn void_body(rest: &str) -> String {
    format!(
        "void k_chain(int n) {{\n{}\n{rest}\n}}",
        PROLOGUE.replace(" RET", "")
    )
}

fn box_body(rest: &str) -> String {
    format!(
        "struct k_box *k_chain(int n) {{\n{}\n{rest}\n}}",
        PROLOGUE.replace("RET", "0")
    )
}

/// The ledger's fixture: the child is linked on one arm and released on the
/// other, then the owner is released. Under P-B08-SHALLOW the link does not
/// discharge the child, so the arm that linked it leaks it. The base answered
/// `Safe` (the join marked the child unresolved and nothing reported it); the
/// repair reports the leak exactly as the straight-line form does.
#[test]
fn the_field_link_join_reports_the_straight_line_leak() {
    if !clang_available() {
        return;
    }
    let straight = analyze(&void_body("b->item = it;\nk_drop(b);"));
    assert_eq!(straight.status, AnalysisStatus::Unsafe, "{straight:?}");
    let branch = analyze(&void_body(
        "if (n) { b->item = it; } else { k_release(it); }\nk_drop(b);",
    ));
    assert_eq!(branch.status, AnalysisStatus::Unsafe, "{branch:?}");
    // The same leak as the straight-line form: the child's acquisition (line
    // 6), at the function's closing exit (line 0).
    assert_eq!(straight.leaks(), vec![(0, 6)], "{straight:?}");
    assert_eq!(branch.leaks(), straight.leaks(), "{branch:?}");
}

/// Control: linked on both arms, then the destructor sequence releases the
/// member through its link and then the owner. Every obligation discharges on
/// every path, so the result stays `Safe` (as it was on the base).
#[test]
fn linked_on_both_arms_then_released_stays_safe() {
    if !clang_available() {
        return;
    }
    let both = analyze(&void_body(
        "if (n) { b->item = it; } else { b->flag = 1; b->item = it; }\nk_release(b->item);\nk_drop(b);",
    ));
    assert_eq!(both.status, AnalysisStatus::Safe, "{both:?}");
    // Linked on both arms and escaped with the owner: the child escapes with
    // it on every path.
    let escaped = analyze(&box_body(
        "if (n) { b->item = it; } else { b->flag = 1; b->item = it; }\nreturn b;",
    ));
    assert_eq!(escaped.status, AnalysisStatus::Safe, "{escaped:?}");
}

/// An escape cascade over a child linked on one arm only: the arm that did not
/// link it leaks it, the arm that did hands it to the caller. The base answered
/// `Safe` (the cascade escaped the joined child); the repair refuses.
#[test]
fn an_escape_cascade_over_a_one_sided_live_child_is_not_safe() {
    if !clang_available() {
        return;
    }
    let one_sided = analyze(&box_body("if (n) { b->item = it; }\nreturn b;"));
    assert_eq!(one_sided.status, AnalysisStatus::Unknown, "{one_sided:?}");
    assert!(
        one_sided.refused(ESCAPE_REFUSED),
        "the cascade reports its refusal: {one_sided:?}"
    );
    // Released on the other arm: correct on every path, but the joined status
    // does not say which path the live bit came from, so the flow refuses
    // rather than escape it (the conservative reading, not an accusation).
    let released = analyze(&box_body(
        "if (n) { b->item = it; } else { k_release(it); }\nreturn b;",
    ));
    assert_eq!(released.status, AnalysisStatus::Unknown, "{released:?}");
    assert!(released.refused(ESCAPE_REFUSED), "{released:?}");
    assert!(released.leaks().is_empty(), "{released:?}");
}

/// The cascade's other arm: a child linked on every path whose status is mixed
/// (live on one path, released through its own name on the other, the member
/// left dangling) was left unresolved silently. It may be live, so the repair
/// refuses.
#[test]
fn an_escape_cascade_over_a_mixed_linked_child_reports() {
    if !clang_available() {
        return;
    }
    let mixed = analyze(&box_body(
        "b->item = it;\nif (n) { k_release(it); }\nreturn b;",
    ));
    assert_eq!(mixed.status, AnalysisStatus::Unknown, "{mixed:?}");
    assert!(mixed.refused(ESCAPE_REFUSED), "{mixed:?}");
}

/// A W4 summary over a body whose exit escaped a one-sided child is not
/// validated, so its caller is not `Safe` on it. On the base the callee was
/// `Safe`, its summary validated, and the caller `Safe`.
#[test]
fn a_summary_over_a_one_sided_escape_is_not_validated() {
    if !clang_available() {
        return;
    }
    let unit = format!(
        "{}\nvoid k_chain(int n) {{\nstruct k_box *c = k_build(n);\nif (c) k_drop(c);\n}}",
        box_body("if (n) { b->item = it; }\nreturn b;").replace("k_chain", "k_build")
    );
    let callee = analyze_in(&unit, "k_build");
    assert_ne!(callee.status, AnalysisStatus::Safe, "{callee:?}");
    let caller = analyze_in(&unit, "k_chain");
    assert_ne!(caller.status, AnalysisStatus::Safe, "{caller:?}");
    assert!(
        !caller.summaries.iter().any(|name| name == "k_build"),
        "{caller:?}"
    );
}

/// The refusal an escape cascade reports for a linked child that may be live
/// on a path it does not escape.
const ESCAPE_REFUSED: &str =
    "field-link escape: a linked child may still be live on a path the escape does not carry it";

/// `(shape, returns the owner, statements after the prologue, verdict)`: the
/// pinned shapes, shared with `QCore.FieldLinkJoin.shapeVectors` (the model's
/// verdict of each shape's two paths and the owner's action).
const SHAPE_VECTORS: &[(&str, bool, &str, &str)] = &[
    (
        "straight-line-release",
        false,
        "b->item = it;\nk_drop(b);",
        "unsafe",
    ),
    (
        "branch-release",
        false,
        "if (n) { b->item = it; } else { k_release(it); }\nk_drop(b);",
        "unsafe",
    ),
    (
        "both-arms-member-released",
        false,
        "if (n) { b->item = it; } else { b->flag = 1; b->item = it; }\nk_release(b->item);\nk_drop(b);",
        "safe",
    ),
    (
        "both-arms-escape",
        true,
        "if (n) { b->item = it; } else { b->flag = 1; b->item = it; }\nreturn b;",
        "safe",
    ),
    (
        "one-sided-escape",
        true,
        "if (n) { b->item = it; }\nreturn b;",
        "unknown",
    ),
    (
        "released-arm-escape",
        true,
        "if (n) { b->item = it; } else { k_release(it); }\nreturn b;",
        "unknown",
    ),
    (
        "mixed-linked-escape",
        true,
        "b->item = it;\nif (n) { k_release(it); }\nreturn b;",
        "unknown",
    ),
];

fn verdict(status: AnalysisStatus) -> &'static str {
    match status {
        AnalysisStatus::Safe => "safe",
        AnalysisStatus::Pending => "pending",
        AnalysisStatus::Unknown => "unknown",
        AnalysisStatus::Unsafe => "unsafe",
    }
}

/// Each pinned shape decides as the Lean model's verdict of its paths.
#[test]
fn the_pinned_shapes_decide_as_the_model() {
    if !clang_available() {
        return;
    }
    for (shape, returns, rest, expected) in SHAPE_VECTORS {
        let body = if *returns {
            box_body(rest)
        } else {
            void_body(rest)
        };
        let run = analyze(&body);
        assert_eq!(verdict(run.status), *expected, "{shape}: {run:?}");
    }
}

const LEAN: &str = include_str!("../../../proof/QCore/FieldLinkJoin.lean");

#[test]
fn b08_lean_shape_vectors_match_the_rust_table() {
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
            let row = row.trim_end_matches(',').trim_start_matches('(');
            let row = row.trim_end_matches(')');
            let (shape, verdict) = row.split_once(',').expect("two cells");
            (
                shape.trim().trim_matches('"').to_owned(),
                verdict
                    .trim()
                    .trim_start_matches('.')
                    .replace(['«', '»'], ""),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), SHAPE_VECTORS.len());
    for ((shape, verdict), (rust_shape, _, _, rust_verdict)) in rows.iter().zip(SHAPE_VECTORS) {
        assert_eq!(
            (shape.as_str(), verdict.as_str()),
            (*rust_shape, *rust_verdict)
        );
    }
}
