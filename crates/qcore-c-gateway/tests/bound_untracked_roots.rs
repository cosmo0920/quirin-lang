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

//! Card CG10-BOUND-UNTRACKED-ROOTS (gateway rule 11d): a typed place on a root
//! the body may bind but, on every path reaching the access, has not bound and
//! has not handed to a custody position.
//!
//! The fixture contracts are the CG00 harness's `q_*` assumptions plus
//! `bound_untracked.qv` (a borrowed result, a derived result, a borrowed
//! string reader). Trusted test assumptions, not library facts.

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *p); void q_transfer(void *p);
void q_observe(void *p); void q_mutate(void *p); void q_consume(void *p);
void q_store(void *owner, void *item); void q_make(void **d);
void *bu_get(void *p); void *bu_interior(void *p); int bu_name(const char *p);
void bu_opaque(void *p);
struct S { int a; struct I { void *p; int x; } in; int arr[4]; char *name; struct S *next; };
";

fn contracts() -> ResourceContracts {
    let file = |name: &str, text: &str| (name.to_owned(), text.to_owned());
    load_resource_contract_sources(&BTreeMap::from([
        file("q_base.qcore", include_str!("fixtures/contracts/q_base.qv")),
        file(
            "q_rich_modes.qcore",
            include_str!("fixtures/contracts/q_rich_modes.qv"),
        ),
        file(
            "q_container_store.qcore",
            include_str!("fixtures/contracts/q_container_store.qv"),
        ),
        file(
            "bound_untracked.qcore",
            include_str!("fixtures/contracts/bound_untracked.qv"),
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
    /// The lines of every unresolved-identity diagnostic naming `name`.
    fn alias_lines(&self, name: &str) -> BTreeSet<usize> {
        self.diagnostics
            .iter()
            .filter(|(_, kind)| {
                matches!(kind, LegacyDiagnosticKind::UnknownAlias { variable } if variable == name)
            })
            .map(|(line, _)| *line)
            .collect()
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

/// The C statement each event of `bound_untracked::EVENT_VECTORS` is.
fn statement(event: &str) -> &'static str {
    match event {
        "borrowed" => "q_observe(r);",
        "released" => "q_free(r);",
        "consumed" => "q_consume(r);",
        "stored" => "q_store(o, r);",
        "owned" => "q_mutate(r);",
        "uncontracted" => "bu_opaque(r);",
        "rebind-none" => "r = bu_get(o);",
        "rebind-derived" => "r = bu_interior(o);",
        "rebind-owned" => "r = q_alloc();",
        "null-test" => "if (!r) return;",
        "member-store" => "q_make(&r->in.p);",
        other => panic!("unknown event {other}"),
    }
}

/// Whether the access `r->in.x = 1;` after `events` is refused at its own
/// line: the run with the access carries an unresolved identity `r` on a line
/// the run without it does not.
fn access_refused(events: &[&str]) -> (bool, Run) {
    let body = events
        .iter()
        .map(|event| statement(event))
        .collect::<Vec<_>>()
        .join("\n");
    // The root starts as a local bound to a result that carries no custody:
    // no binding (a formal at an `Owned` position is bound at entry instead).
    let without = run(&format!(
        "void subject(struct S *o) {{\nstruct S *r = bu_get(o);\n{body}\n}}"
    ));
    let with = run(&format!(
        "void subject(struct S *o) {{\nstruct S *r = bu_get(o);\n{body}\nr->in.x = 1;\n}}"
    ));
    let before = without.alias_lines("r");
    let refused = with.alias_lines("r").difference(&before).next().is_some();
    (refused, with)
}

#[test]
fn a_borrowed_actual_then_an_access_is_admitted() {
    if !clang_available() {
        return;
    }
    // Control: `r` is handed only to a `Borrowed` position, so the closure
    // does not bind it (never bound, as before).
    let run = run("void subject(struct S *r) { q_observe(r); r->in.x = 1; }");
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    // CG10 flip (rule 11d): a root the closure binds (assigned from a call,
    // later handed to a custody position) is read as untracked on the path
    // before that position; the release itself is refused (an untracked
    // object). BEFORE (rule 10t alone): `unknown_alias o` at the access too.
    let run = self::run(
        "void subject(struct S *x) {\n struct S *o = bu_get(x);\n q_observe(o);\n o->in.x = 1;\n q_free(o);\n}",
    );
    assert_eq!(run.alias_lines("o").len(), 1, "{run:?}");
}

#[test]
fn every_event_vector_decides_the_access_as_the_lean_model_does() {
    if !clang_available() {
        return;
    }
    // The gateway outcome of the access, per shared event vector: admitted
    // where the premise holds or the tracked reading admits a live root,
    // refused where the path passed a refusal, a rooted place is bound, or
    // the root may be null.
    for &(events, premise) in qcore_c_gateway_bound_untracked_vectors::EVENT_VECTORS {
        let (refused, with) = access_refused(events);
        let expected = match premise {
            "holds" => false,
            "path-refused" | "rooted-place" => true,
            // `bound`: the tracked reading decides: a live owned result is
            // read, a possibly-null derived result is not.
            "bound" => !events.contains(&"rebind-owned"),
            other => panic!("unknown premise {other}"),
        };
        assert_eq!(refused, expected, "{events:?} ({premise}): {with:?}");
    }
}

/// The event vectors of `crates/qcore-c-gateway/src/bound_untracked.rs`
/// (`EVENT_VECTORS`), repeated here because the module is crate-private;
/// the unit test `event_vectors_match_the_integration_copy` pins the copy.
mod qcore_c_gateway_bound_untracked_vectors {
    pub const EVENT_VECTORS: &[(&[&str], &str)] = &[
        (&[], "holds"),
        (&["borrowed"], "holds"),
        (&["borrowed", "borrowed"], "holds"),
        (&["released"], "path-refused"),
        (&["consumed"], "path-refused"),
        (&["stored"], "path-refused"),
        (&["owned"], "path-refused"),
        (&["uncontracted"], "path-refused"),
        (&["released", "borrowed"], "path-refused"),
        (&["uncontracted", "rebind-none"], "path-refused"),
        (&["rebind-none"], "holds"),
        (&["rebind-derived"], "bound"),
        (&["rebind-derived", "null-test"], "holds"),
        (&["rebind-owned"], "bound"),
        (&["member-store"], "rooted-place"),
    ];
}

#[test]
fn a_released_actual_then_an_access_is_refused() {
    if !clang_available() {
        return;
    }
    let (refused, run) = access_refused(&["released"]);
    assert!(refused, "{run:?}");
    assert!(run.status != AnalysisStatus::Safe);
}

#[test]
fn an_uncontracted_call_then_an_access_is_refused() {
    if !clang_available() {
        return;
    }
    let (refused, run) = access_refused(&["uncontracted"]);
    assert!(refused, "{run:?}");
    assert!(run.status != AnalysisStatus::Safe);
}

#[test]
fn a_borrowed_result_root_is_admitted() {
    if !clang_available() {
        return;
    }
    // `r` is assigned from a call (class c): the closure binds it. The
    // result carries no custody, so on this path it is an untracked
    // borrowed object. BEFORE: `unknown_alias r`, Unknown.
    let run = run(
        "void subject(struct S *o) { struct S *r; q_observe(o); r = bu_get(o); r->in.x = 1; r->arr[2] = r->in.x; }",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    // A derived result is possibly null until a test refines it.
    let run = self::run("void subject(struct S *o) { struct S *r = bu_interior(o); r->in.x = 1; }");
    assert!(run.alias_lines("r").len() == 1, "{run:?}");
    let run = self::run(
        "void subject(struct S *o) { struct S *r = bu_interior(o); if (r) { r->in.x = 1; } }",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
}

#[test]
fn the_premise_is_per_path() {
    if !clang_available() {
        return;
    }
    // A release on one branch ends the premise after the join, not before.
    let run = run(
        "void subject(struct S *r, int c) {\n r->in.x = 1;\n if (c) q_free(r);\n r->in.x = 2;\n}",
    );
    let lines = run.alias_lines("r");
    assert_eq!(
        lines.len(),
        2,
        "the release and the access after it: {run:?}"
    );
    // Rebinding to a borrowed result after the refused call does not restore
    // the premise: the path was refused.
    let (refused, _) = access_refused(&["uncontracted", "rebind-none"]);
    assert!(refused);
}

#[test]
fn an_address_taken_root_stays_refused() {
    if !clang_available() {
        return;
    }
    // Class (d): `&r` anywhere makes the root ineligible.
    let run = run(
        "void subject(struct S *r) {\n struct S **slot = &r;\n r->in.x = 1;\n q_observe(slot);\n}",
    );
    assert!(run.status != AnalysisStatus::Safe, "{run:?}");
}

#[test]
fn a_tracked_root_keeps_the_tracked_reading() {
    if !clang_available() {
        return;
    }
    // A live acquisition is read as before; a released one is accused or
    // refused, never read as untracked.
    let run = run("void subject(void) { struct S *r = q_alloc(); r->in.x = 1; q_free(r); }");
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    let run = self::run("void subject(void) { struct S *r = q_alloc(); q_free(r); r->in.x = 1; }");
    assert!(run.status != AnalysisStatus::Safe, "{run:?}");
}

#[test]
fn a_borrowed_pointer_actual_on_a_bound_root_is_read_by_the_flow() {
    if !clang_available() {
        return;
    }
    // Rule 10k(3) with rule 11d: `bu_name(r->name)` hands a pointer read
    // from `r` to a `Borrowed` position. BEFORE: refused at classification
    // (a pointer-valued actual needs a never-bound root).
    let run = run(
        "int subject(struct S *x) {\n struct S *r = bu_get(x);\n int n = bu_name(r->name);\n q_free(r);\n return n;\n}",
    );
    assert_eq!(run.alias_lines("r").len(), 1, "only the release: {run:?}");
    // After a release on the path the actual is refused at the flow.
    let run = self::run(
        "int subject(struct S *x) {\n struct S *r = bu_get(x);\n q_free(r);\n int n = bu_name(r->name);\n return n;\n}",
    );
    assert!(run.alias_lines("r").len() >= 2, "{run:?}");
    // An address actual keeps rule 10n's never-bound premise.
    let run = self::run("void subject(struct S *r) {\n q_observe(&r->in);\n q_free(r);\n}");
    assert!(run.status != AnalysisStatus::Safe, "{run:?}");
}
