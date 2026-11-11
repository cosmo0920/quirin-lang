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

//! The read-only recursive-component census (card **C03-REACH**).
//!
//! These tests pin two things at once. First, that the census *reports* what
//! the route actually did — every disposal arm, and, for a refused member, the
//! obligation the route stopped at with the instructions that blocked it.
//! Second, and more important, that the census is inert: for each fixture the
//! ordinary analysis is run as well, and its verdicts, diagnostics and
//! published summary records must be exactly what card C03-NFR's own matrix
//! already pins for the same shape. A census that could move a verdict would
//! be a defect, not a measurement.
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, ContractTrust, ResourceContracts, ResourceParameterMode,
    ResourceReturnMode, UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_c_gateway::legacy_census::{
    NeutralComponentCensus, NeutralComponentOutcome, NeutralRefusal, neutral_reach_census,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "void *q_alloc(void); void q_free(void *); void q_transfer(void *);";

fn contracts(declared: Option<&str>) -> ResourceContracts {
    let mut sources = vec![(
        "q_base.qcore".to_owned(),
        include_str!("fixtures/contracts/q_base.qv").to_owned(),
    )];
    if let Some(declared) = declared {
        sources.push(("reach_declared.qcore".to_owned(), declared.to_owned()));
    }
    load_resource_contract_sources(&sources.into_iter().collect()).unwrap()
}

fn unit(source: &str) -> C11TranslationUnit<'_> {
    C11TranslationUnit {
        source,
        ..Default::default()
    }
}

fn census(program: &str, declared: Option<&str>) -> Vec<NeutralComponentCensus> {
    let source = format!("{DECLARATIONS}\n{program}");
    neutral_reach_census(&unit(&source), &contracts(declared), None)
        .unwrap_or_else(|error| panic!("valid fixture rejected by Clang: {error:?}\n{source}"))
        .components
}

/// The ordinary analysis of the same fixture, so every census assertion is
/// paired with the verdict surface it must not have moved.
fn analyze(program: &str, declared: Option<&str>) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS}\n{program}");
    analyze_unrestricted_c11(&unit(&source), "subject", &contracts(declared))
        .unwrap_or_else(|error| panic!("valid fixture rejected by Clang: {error:?}\n{source}"))
}

fn one(components: &[NeutralComponentCensus]) -> &NeutralComponentCensus {
    match components {
        [component] => component,
        other => panic!("expected exactly one recursive component, got {other:?}"),
    }
}

fn obligations(component: &NeutralComponentCensus) -> Vec<(&str, &'static str)> {
    match &component.outcome {
        NeutralComponentOutcome::Refused { members }
        | NeutralComponentOutcome::Admitted {
            refused: members, ..
        } => members
            .iter()
            .map(|entry| (entry.function.as_str(), entry.refusal.obligation()))
            .collect(),
        other => panic!("expected a refusal list, got {other:?}"),
    }
}

/// A neutral self-recursive body: the route publishes `N(m)`, the census says
/// so by name, and the ordinary analysis reaches `Safe` with the neutral
/// summary record — the same result C03-NFR pins.
#[test]
fn an_admitted_component_is_reported_as_admitted_and_still_analyses_safe() {
    const PROGRAM: &str = r"
static void walk(int n) { if (n > 0) { walk(n - 1); } }
void subject(void) { void *r = q_alloc(); walk(3); q_free(r); }
";
    let components = census(PROGRAM, None);
    let component = one(&components);
    assert_eq!(component.members, vec!["walk".to_owned()]);
    assert!(component.self_recursive);
    match &component.outcome {
        NeutralComponentOutcome::Admitted { members, refused } => {
            assert_eq!(members, &vec!["walk".to_owned()]);
            assert!(refused.is_empty(), "no member should have been demoted");
        }
        other => panic!("expected an admission, got {other:?}"),
    }

    let UnrestrictedLegacyResult::Safe(certificate) = analyze(PROGRAM, None) else {
        panic!("the admitted component should leave its caller Safe");
    };
    let record = certificate
        .summaries
        .iter()
        .find(|record| record.function == "walk")
        .expect("a published neutral summary for `walk`");
    assert_eq!(record.trust, ContractTrust::InferredValidated);
    assert_eq!(
        record.contract.parameters,
        vec![ResourceParameterMode::Borrowed]
    );
    assert_eq!(record.contract.result, ResourceReturnMode::None);
}

/// A member that carries a custody effect refuses at **V5**, and the census
/// names the retained instruction rather than a bare failure. The caller stays
/// `Unknown`, exactly as C03-NFR's refusal matrix requires.
#[test]
fn an_effectful_member_refuses_at_v5_and_names_its_blocking_instruction() {
    // Card CG27-LOOPS-RECURSION-BUDGETS (gateway rule 11x): `if (n > 0)` would
    // now carry a signed threshold rank, published after this refusal; the
    // unranked `if (n)` keeps the census arm under test.
    const PROGRAM: &str = r"
static void walk(void *p, int n) { if (n) { walk(p, n - 1); return; } q_free(p); }
void subject(void) { void *r = q_alloc(); walk(r, 3); }
";
    let components = census(PROGRAM, None);
    let component = one(&components);
    assert_eq!(obligations(component), vec![("walk", "V5-body-not-clean")]);

    let NeutralComponentOutcome::Refused { members } = &component.outcome else {
        panic!("expected a refusal");
    };
    let NeutralRefusal::BodyNotClean {
        status,
        diagnostics,
    } = &members[0].refusal
    else {
        panic!("expected a not-clean body, got {:?}", members[0].refusal);
    };
    assert_ne!(*status, AnalysisStatus::Safe);
    assert!(
        !diagnostics.is_empty(),
        "a not-clean body must carry the diagnostics that made it so"
    );

    // The withdrawn component leaves the caller with an unexplained
    // acquisition, which is the leak C03-NFR's matrix records — the important
    // half is that it is never `Safe`.
    assert!(
        !matches!(analyze(PROGRAM, None), UnrestrictedLegacyResult::Safe(_)),
        "the refused component must never leave its caller Safe"
    );
}

/// An unmodelled construct is refused by the containment gate *before* the
/// abstract interpretation runs, so the census reports the retained
/// instructions. This is the corpus's dominant arm and the reason the report
/// quotes retained instructions rather than published diagnostics.
#[test]
fn an_unmodelled_construct_refuses_at_the_containment_gate_with_its_descriptions() {
    const PROGRAM: &str = r"
struct pair { int *left; int *right; };
static int walk(struct pair *p, int n) { if (n > 0) { return walk(p, n - 1) + *p->left; } return 0; }
void subject(struct pair *p) { walk(p, 3); }
";
    let components = census(PROGRAM, None);
    let component = one(&components);
    assert_eq!(
        obligations(component),
        vec![("walk", "V5-retained-unknown-instruction")]
    );

    let NeutralComponentOutcome::Refused { members } = &component.outcome else {
        panic!("expected a refusal");
    };
    let NeutralRefusal::RetainedUnknownInstruction { descriptions } = &members[0].refusal else {
        panic!(
            "expected a containment refusal, got {:?}",
            members[0].refusal
        );
    };
    assert!(
        !descriptions.is_empty(),
        "the containment refusal must carry the instructions that blocked it"
    );
    assert!(
        descriptions.iter().all(|(line, _)| *line > 0,),
        "every retained instruction carries a preprocessed line: {descriptions:?}"
    );
}

/// **V1.** A declared contract for a member keeps the component on the
/// declared route, and the census says `DeclaredContract` rather than
/// inventing a refusal.
#[test]
fn a_declared_component_is_reported_as_declared_not_refused() {
    const PROGRAM: &str = r"
static void walk(void *p, int n) { if (n > 0) { walk(p, n - 1); return; } q_free(p); }
void subject(void) { void *r = q_alloc(); walk(r, 3); }
";
    const DECLARED: &str = "module contract;\n\
        extern fn walk(p: ptr<mut, u8>, n: i32) -> unit\ncontract {\n\
        \x20   requires resource_contract_v1;\n\
        \x20   ensures normal resource_released(p, nonnull);\n\
        \x20   ensures normal resource_borrowed(n);\n\
        \x20   ensures normal resource_return_none();\n};\n";
    let components = census(PROGRAM, Some(DECLARED));
    let component = one(&components);
    assert!(matches!(
        component.outcome,
        NeutralComponentOutcome::DeclaredContract
    ));
}

/// **V2.** A component a ranked recursive route publishes is reported as
/// that route's, not as a neutral admission and not as a refusal. Reporting it
/// otherwise would credit the neutral route with reach it does not have, which
/// is precisely the error this census exists to avoid.
#[test]
fn a_ranked_component_is_credited_to_the_ranked_route() {
    const PROGRAM: &str = r"
static void release(void *p, int flag) { if (flag) { release(p, 0); return; } q_free(p); }
void subject(void) { void *p = q_alloc(); release(p, 1); }
";
    let components = census(PROGRAM, None);
    let component = one(&components);
    match &component.outcome {
        NeutralComponentOutcome::PublishedByRankedRecursion { members } => {
            assert_eq!(members, &vec!["release".to_owned()]);
        }
        other => panic!("expected the ranked route to be credited, got {other:?}"),
    }

    let UnrestrictedLegacyResult::Safe(certificate) = analyze(PROGRAM, None) else {
        panic!("the ranked route's own fixture should still be Safe");
    };
    let record = certificate
        .summaries
        .iter()
        .find(|record| record.function == "release")
        .expect("the ranked route's published summary");
    assert_eq!(
        record.contract.parameters[0],
        ResourceParameterMode::Released { nullable: false },
        "the ranked route's `Released` contract is not a neutral one"
    );
}

/// A non-recursive translation unit produces no census row at all: the census
/// is a record of recursive components, not of every function.
#[test]
fn a_non_recursive_unit_has_an_empty_census() {
    const PROGRAM: &str = r"
static void helper(void *p) { q_free(p); }
void subject(void) { void *r = q_alloc(); helper(r); }
";
    assert!(census(PROGRAM, None).is_empty());
    assert!(matches!(
        analyze(PROGRAM, None),
        UnrestrictedLegacyResult::Safe(_)
    ));
}

/// A mutually recursive pair is one component, and publication is all-or-
/// nothing over it: when one member carries an effect the census reports both
/// members' refusals, not a partial admission.
#[test]
fn a_mixed_mutual_pair_refuses_as_one_component() {
    const PROGRAM: &str = r"
static void even_step(void *p, int n);
static void odd_step(void *p, int n) { if (n > 0) { even_step(p, n - 1); return; } q_free(p); }
static void even_step(void *p, int n) { if (n > 0) { odd_step(p, n - 1); } }
void subject(void) { void *r = q_alloc(); even_step(r, 4); }
";
    let components = census(PROGRAM, None);
    let component = one(&components);
    assert!(!component.self_recursive);
    assert_eq!(
        component.members,
        vec!["even_step".to_owned(), "odd_step".to_owned()]
    );
    assert!(matches!(
        component.outcome,
        NeutralComponentOutcome::Refused { .. }
    ));
    let refused = obligations(component);
    assert!(
        refused.iter().any(|(name, _)| *name == "odd_step"),
        "the effectful member must appear: {refused:?}"
    );
    assert!(
        !matches!(analyze(PROGRAM, None), UnrestrictedLegacyResult::Safe(_)),
        "a withdrawn component never leaves its caller Safe"
    );
}
