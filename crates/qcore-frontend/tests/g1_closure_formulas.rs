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

//! Card **P0-G1-CLOSURE-RECORD**: REEVALUATION_SPEC §9's gate G1 criterion
//! ("invalid candidates and insufficient evidence cannot obtain the affected
//! certificates; positive controls survive", with the deliverable's explicit
//! profile restrictions) as formulas of the C04.18.9 typed rendering over the
//! G1 block of registered terms (`RENDERED_TERMS`, block `G1`; sorts
//! `g1_finding`, `g1_case`), each checked by the premise-formula validator
//! `guarded_proposition`. Prose is refused.
//!
//! [`CASES`] is the record's case table
//! (`docs/reports/g1-closure-record/README.md`), one row per executed control
//! case, transcribed from the probe output and the named tests at the
//! record's revision. The evaluation below decides each formula over that
//! table, so the record's verdict follows from its rows; the rows themselves
//! are observations, not something this test re-runs.

use qcore_frontend::premise_formula::{
    RENDERED_TERMS, RENDERED_VALUE_SORTS, RenderedSort, guarded_proposition, term,
};

/// The G1 criterion, `(id, formula)`: the per-case conjuncts, the
/// per-finding closure, the gate, then the named refusals.
pub const FORMULAS: &[(&str, &str)] = &[
    (
        "g1_negative_controls_contained",
        "forall (#c: g1_case) { ((#g1_invalid(#c) || #g1_insufficient(#c)) => !#g1_certified(#c)) }",
    ),
    (
        "g1_positive_controls_survive",
        "forall (#c: g1_case) { (#g1_positive(#c) => #g1_certified(#c)) }",
    ),
    (
        "g1_positive_is_valid_and_sufficient",
        "forall (#c: g1_case) { (#g1_positive(#c) => (!#g1_invalid(#c) && !#g1_insufficient(#c))) }",
    ),
    (
        "g1_certificate_inside_profile",
        "forall (#c: g1_case) { (#g1_certified(#c) => #g1_in_profile(#c)) }",
    ),
    (
        "g1_outside_profile_is_unknown",
        "forall (#c: g1_case) { (!#g1_in_profile(#c) => #g1_unknown(#c)) }",
    ),
    (
        "g1_unknown_is_uncertified",
        "forall (#c: g1_case) { (#g1_unknown(#c) => !#g1_certified(#c)) }",
    ),
    (
        "g1_finding_closed_needs_controls",
        "forall (#f: g1_finding) { (#g1_closed(#f) => ((exists (#n: g1_case) { (#g1_of(#n,#f) && (#g1_invalid(#n) || #g1_insufficient(#n))) } && exists (#p: g1_case) { (#g1_of(#p,#f) && #g1_positive(#p)) }) && exists (#o: g1_case) { (#g1_of(#o,#f) && !#g1_in_profile(#o)) })) }",
    ),
    (
        "g1_finding_closed_needs_every_case",
        "forall (#f: g1_finding) { (#g1_closed(#f) => forall (#c: g1_case) { (#g1_of(#c,#f) => (((#g1_executed(#c) && ((#g1_invalid(#c) || #g1_insufficient(#c)) => !#g1_certified(#c))) && (#g1_positive(#c) => #g1_certified(#c))) && ((#g1_certified(#c) => #g1_in_profile(#c)) && (!#g1_in_profile(#c) => #g1_unknown(#c))))) }) }",
    ),
    (
        "g1_finding_closed_when",
        "forall (#f: g1_finding) { ((((exists (#n: g1_case) { (#g1_of(#n,#f) && (#g1_invalid(#n) || #g1_insufficient(#n))) } && exists (#p: g1_case) { (#g1_of(#p,#f) && #g1_positive(#p)) }) && exists (#o: g1_case) { (#g1_of(#o,#f) && !#g1_in_profile(#o)) }) && forall (#c: g1_case) { (#g1_of(#c,#f) => (((#g1_executed(#c) && ((#g1_invalid(#c) || #g1_insufficient(#c)) => !#g1_certified(#c))) && (#g1_positive(#c) => #g1_certified(#c))) && ((#g1_certified(#c) => #g1_in_profile(#c)) && (!#g1_in_profile(#c) => #g1_unknown(#c))))) }) => #g1_closed(#f)) }",
    ),
    (
        "g1_gate_met_needs_every_finding",
        "(#g1_gate_met() => forall (#f: g1_finding) { #g1_closed(#f) })",
    ),
    (
        "g1_gate_met_when",
        "(forall (#f: g1_finding) { #g1_closed(#f) } => #g1_gate_met())",
    ),
    (
        "g1_refuse_no_negative_control",
        "forall (#f: g1_finding) { (!exists (#n: g1_case) { (#g1_of(#n,#f) && (#g1_invalid(#n) || #g1_insufficient(#n))) } => !#g1_closed(#f)) }",
    ),
    (
        "g1_refuse_no_positive_control",
        "forall (#f: g1_finding) { (!exists (#p: g1_case) { (#g1_of(#p,#f) && #g1_positive(#p)) } => !#g1_closed(#f)) }",
    ),
    (
        "g1_refuse_no_profile_control",
        "forall (#f: g1_finding) { (!exists (#o: g1_case) { (#g1_of(#o,#f) && !#g1_in_profile(#o)) } => !#g1_closed(#f)) }",
    ),
    (
        "g1_refuse_unexecuted",
        "forall (#c: g1_case) { (!#g1_executed(#c) => forall (#f: g1_finding) { (#g1_of(#c,#f) => !#g1_closed(#f)) }) }",
    ),
    (
        "g1_refuse_open_finding",
        "(exists (#f: g1_finding) { !#g1_closed(#f) } => !#g1_gate_met())",
    ),
];

/// The findings G1 names.
pub const FINDINGS: &[&str] = &["WM01", "WM02", "WM03", "VP01", "VP02"];

/// One executed control case of the record.
#[derive(Clone, Copy, Debug)]
pub struct Case {
    pub id: &'static str,
    pub findings: &'static [&'static str],
    pub invalid: bool,
    pub insufficient: bool,
    pub positive: bool,
    pub certified: bool,
    pub in_profile: bool,
    pub unknown: bool,
    pub executed: bool,
}

const WM: &[&str] = &["WM01", "WM02", "WM03"];
const VP: &[&str] = &["VP01", "VP02"];

const fn negative(id: &'static str, findings: &'static [&'static str]) -> Case {
    Case {
        id,
        findings,
        invalid: true,
        insufficient: false,
        positive: false,
        certified: false,
        in_profile: true,
        unknown: true,
        executed: true,
    }
}

const fn insufficient(id: &'static str, findings: &'static [&'static str]) -> Case {
    Case {
        invalid: false,
        insufficient: true,
        ..negative(id, findings)
    }
}

const fn positive(id: &'static str, findings: &'static [&'static str]) -> Case {
    Case {
        id,
        findings,
        invalid: false,
        insufficient: false,
        positive: true,
        certified: true,
        in_profile: true,
        unknown: false,
        executed: true,
    }
}

const fn outside(id: &'static str, findings: &'static [&'static str]) -> Case {
    Case {
        id,
        findings,
        invalid: false,
        insufficient: false,
        positive: false,
        certified: false,
        in_profile: false,
        unknown: true,
        executed: true,
    }
}

/// The record's case table, row for row with the README's.
pub const CASES: &[Case] = &[
    // WM01: per-operation order legality.
    negative("g1_case_probe_wm01_acq_rel_plain_operations", &["WM01"]),
    negative("g1_case_test_wm01_illegal_orders", &["WM01"]),
    positive("g1_case_probe_control_relaxed_plain_operations", &["WM01"]),
    positive("g1_case_test_wm01_legal_orders", &["WM01"]),
    // WM02: coherence against happens-before.
    negative("g1_case_probe_wm02_reversed_write_coherence", &["WM02"]),
    negative("g1_case_test_wm02_reversed_coherence", &["WM02"]),
    positive("g1_case_test_wm02_forward_coherence", &["WM02"]),
    // WM03: SC order and SC reads.
    negative("g1_case_probe_wm03_sc_store_buffering_zero_zero", &["WM03"]),
    negative("g1_case_test_wm03_serial_oracle_forbidden", &["WM03"]),
    negative("g1_case_test_cg28_lean_vector_sb_seq_cst", &["WM03"]),
    positive("g1_case_test_wm03_serial_oracle_permitted", &["WM03"]),
    positive("g1_case_test_relaxed_store_buffering_permitted", &["WM03"]),
    positive("g1_case_test_cg28_lean_vector_sb_relaxed", &["WM03"]),
    // The WM profile restriction: forms outside the represented N1570
    // atomic fragment are refused by the atomic source route.
    outside("g1_case_test_atomic_route_refusals", WM),
    // VP01: assertion-only evidence.
    negative(
        "g1_case_probe_vp01_recognition_is_not_resource_proof",
        &["VP01"],
    ),
    negative("g1_case_test_vp01_mutations", &["VP01"]),
    negative("g1_case_test_vp01_process_nonempty", &["VP01"]),
    // VP02: empty evidence.
    insufficient("g1_case_probe_vp02_empty_evidence_firewall", &["VP02"]),
    insufficient("g1_case_test_vp02_empty_in_process", &["VP02"]),
    insufficient("g1_case_test_vp02_process_empty", &["VP02"]),
    insufficient("g1_case_vp02_v2_empty_inventory", &["VP02"]),
    // Both VP findings at the v2 firewall: insufficient replays, positive
    // controls, and the profile restriction.
    insufficient("g1_case_test_v2_malicious_proposals", VP),
    positive("g1_case_test_v2_registry_certificate", VP),
    positive("g1_case_test_v2_source_effect_honest_coverage", VP),
    positive("g1_case_vp02_v2_positive_control", VP),
    outside("g1_case_vp_v2_unmatched_target", VP),
    outside("g1_case_test_v2_task_envelope_rebound", VP),
];

fn of(case: &Case, finding: &str) -> bool {
    case.findings.contains(&finding)
}

fn contained(c: &Case) -> bool {
    !(c.invalid || c.insufficient) || !c.certified
}

fn survives(c: &Case) -> bool {
    !c.positive || c.certified
}

fn inside(c: &Case) -> bool {
    !c.certified || c.in_profile
}

fn outside_unknown(c: &Case) -> bool {
    c.in_profile || c.unknown
}

/// `g1_finding_closed_when`'s antecedent: the finding's closure.
fn closed(finding: &str) -> bool {
    let cases = CASES.iter().filter(|c| of(c, finding)).collect::<Vec<_>>();
    cases.iter().any(|c| c.invalid || c.insufficient)
        && cases.iter().any(|c| c.positive)
        && cases.iter().any(|c| !c.in_profile)
        && cases
            .iter()
            .all(|c| c.executed && contained(c) && survives(c) && inside(c) && outside_unknown(c))
}

#[test]
fn every_formula_is_a_validated_typed_rendering() {
    for (id, formula) in FORMULAS {
        guarded_proposition(id, formula).unwrap_or_else(|error| panic!("{id}: {error}"));
    }
    let mut ids = FORMULAS.iter().map(|(id, _)| *id).collect::<Vec<_>>();
    assert!(ids.iter().all(|id| id.starts_with("g1_")));
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), FORMULAS.len());
}

#[test]
fn prose_is_refused_by_the_validator() {
    for prose in [
        "positive controls survive",
        "\"#g1_certified(#c)\"",
        "forall (#c: g1_case) { #g1_unregistered(#c) }",
        // A registered term in the wrong sort.
        "forall (#f: g1_finding) { #g1_certified(#f) }",
        // A free variable.
        "#g1_certified(#c)",
        // Not the canonical printing.
        "forall (#c: g1_case) { #g1_positive(#c) => #g1_certified(#c) }",
    ] {
        assert!(guarded_proposition("g1_probe", prose).is_err(), "{prose}");
    }
}

#[test]
fn every_g1_term_is_used_and_typed() {
    let g1 = RENDERED_TERMS
        .iter()
        .filter(|t| t.block == "G1")
        .collect::<Vec<_>>();
    assert_eq!(g1.len(), 10);
    let registered = |s: &str| RENDERED_VALUE_SORTS.iter().any(|(v, _)| *v == s);
    for t in &g1 {
        assert!(t.name.starts_with("g1_"), "#{}", t.name);
        assert!(term(t.name).is_none(), "#{} is a premise term", t.name);
        assert_eq!(
            RENDERED_TERMS.iter().filter(|u| u.name == t.name).count(),
            1,
            "#{}",
            t.name
        );
        let sorts: Vec<&str> = match t.sort {
            RenderedSort::Value(v) => vec![v],
            RenderedSort::Predicate(a) => a.to_vec(),
            RenderedSort::Function(a, r) => a.iter().copied().chain([r]).collect(),
        };
        assert!(sorts.iter().all(|s| registered(s)), "#{}", t.name);
        let spelled = format!("#{}", t.name);
        assert!(
            FORMULAS.iter().any(|(_, f)| f
                .split(|c: char| !(c == '#' || c == '_' || c.is_ascii_alphanumeric()))
                .any(|w| w == spelled)),
            "#{} is named by no formula",
            t.name
        );
    }
    let sorts = RENDERED_VALUE_SORTS
        .iter()
        .filter(|(s, _)| s.starts_with("g1_"))
        .collect::<Vec<_>>();
    assert_eq!(sorts.len(), 2);
    // The G1 block is the last block, contiguous, after the CX block.
    let blocks: Vec<_> = RENDERED_TERMS.iter().map(|t| t.block).collect();
    let first = blocks.iter().position(|b| *b == "G1").unwrap();
    assert!(blocks[first..].iter().all(|b| *b == "G1"));
    assert_eq!(blocks[first - 1], "CX");
}

/// Each formula decided over the record's case table: every per-case
/// conjunct holds of every row, every finding is closed, and so the gate is
/// met in the record's scope.
#[test]
fn the_case_table_meets_every_formula() {
    let mut ids = CASES.iter().map(|c| c.id).collect::<Vec<_>>();
    assert!(ids.iter().all(|id| id.starts_with("g1_case_")));
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), CASES.len(), "case ids are unique");
    for c in CASES {
        assert!(c.findings.iter().all(|f| FINDINGS.contains(f)), "{}", c.id);
        // The classes are exclusive: a case is a negative control, a positive
        // control, or a profile control.
        let classes = [c.invalid || c.insufficient, c.positive, !c.in_profile];
        assert_eq!(classes.iter().filter(|x| **x).count(), 1, "{}", c.id);
        assert!(contained(c), "g1_negative_controls_contained: {}", c.id);
        assert!(survives(c), "g1_positive_controls_survive: {}", c.id);
        assert!(
            !c.positive || (!c.invalid && !c.insufficient),
            "g1_positive_is_valid_and_sufficient: {}",
            c.id
        );
        assert!(inside(c), "g1_certificate_inside_profile: {}", c.id);
        assert!(
            outside_unknown(c),
            "g1_outside_profile_is_unknown: {}",
            c.id
        );
        assert!(
            !c.unknown || !c.certified,
            "g1_unknown_is_uncertified: {}",
            c.id
        );
        assert!(c.executed, "g1_refuse_unexecuted: {}", c.id);
    }
    for finding in FINDINGS {
        assert!(closed(finding), "g1_finding_closed_when: {finding}");
    }
    let gate_met = FINDINGS.iter().all(|f| closed(f));
    assert!(gate_met, "g1_gate_met_when");
}

/// The evaluation refuses: dropping a finding's positive controls, its
/// profile controls, or marking one of its cases unexecuted or certified
/// while invalid leaves it open.
#[test]
fn the_refusals_open_a_finding() {
    let decide = |cases: &[Case], finding: &str| {
        let cases = cases.iter().filter(|c| of(c, finding)).collect::<Vec<_>>();
        cases.iter().any(|c| c.invalid || c.insufficient)
            && cases.iter().any(|c| c.positive)
            && cases.iter().any(|c| !c.in_profile)
            && cases.iter().all(|c| {
                c.executed && contained(c) && survives(c) && inside(c) && outside_unknown(c)
            })
    };
    for finding in FINDINGS {
        assert!(decide(CASES, finding));
        let no_positive = CASES
            .iter()
            .copied()
            .filter(|c| !c.positive)
            .collect::<Vec<_>>();
        assert!(
            !decide(&no_positive, finding),
            "g1_refuse_no_positive_control"
        );
        let no_profile = CASES
            .iter()
            .copied()
            .filter(|c| c.in_profile)
            .collect::<Vec<_>>();
        assert!(
            !decide(&no_profile, finding),
            "g1_refuse_no_profile_control"
        );
        let no_negative = CASES
            .iter()
            .copied()
            .filter(|c| !(c.invalid || c.insufficient))
            .collect::<Vec<_>>();
        assert!(
            !decide(&no_negative, finding),
            "g1_refuse_no_negative_control"
        );
        let mut unexecuted = CASES.to_vec();
        let index = unexecuted.iter().position(|c| of(c, finding)).unwrap();
        unexecuted[index].executed = false;
        assert!(!decide(&unexecuted, finding), "g1_refuse_unexecuted");
        let mut leaked = CASES.to_vec();
        let index = leaked
            .iter()
            .position(|c| of(c, finding) && (c.invalid || c.insufficient))
            .unwrap();
        leaked[index].certified = true;
        assert!(!decide(&leaked, finding), "g1_negative_controls_contained");
    }
}

/// The record states every formula verbatim and lists every case.
#[test]
fn the_record_states_every_formula_and_case() {
    let readme = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/reports/g1-closure-record/README.md"),
    )
    .expect("the G1 closure record");
    for (id, formula) in FORMULAS {
        assert!(readme.contains(id), "{id}");
        assert!(
            readme.contains(formula),
            "{id}: formula not stated verbatim"
        );
    }
    for case in CASES {
        assert!(readme.contains(case.id), "{}", case.id);
    }
}
