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

//! Neutral-frame recursion (NFR), the W4 recursive-summary family specified in
//! `docs/design/W4_RECURSIVE_SUMMARY_PROFILES.md` (card C03) and implemented
//! here as its units U1 (derivation route) and U2 (greatest post-fixed-point
//! descent).
//!
//! # What this route does and does not assume
//!
//! The two existing recursive routes constrain the *control shape* — one
//! immutable integer or unsigned control — and admit arbitrary custody effects,
//! paying for that freedom with a well-founded rank. NFR takes the orthogonal
//! axis: it constrains the *effect content* to the bottom element and leaves
//! the control shape completely free (self, mutual, unranked, any arity).
//!
//! The only contract this route ever seeds for a member of its own strongly
//! connected component is the neutral one — every formal `Borrowed`, result
//! `None`. That restriction is a soundness requirement, not a tuning choice:
//! validation by re-analysis alone **cannot** refuse a circular *effectful*
//! candidate, because a body every concrete execution of which diverges
//! satisfies "if the call returns, the effect happened" vacuously. The design's
//! §4.3 counterexample measures exactly that, and
//! `crates/qcore-c-gateway/tests/neutral_frame_recursion.rs` reproduces it
//! against this route. Any future family that assumes an effect must therefore
//! carry a rank, as the contextual and countdown routes do.
//!
//! The neutral element is different for two reasons, both of which this module
//! relies on and neither of which any higher element has:
//!
//! 1. `N(m)` states **no precondition** — every formal is `Borrowed`, so the
//!    assumption binds no entry resource and asserts no non-nullness. It cannot
//!    delete a failure path, which is the hazard the project's standing
//!    instruction about assume-guarantee fixed points names.
//! 2. `N(m)` is a conjunction of **negative, prefix-closed** facts: at every
//!    point of the execution, returning or not, no custody event touches an
//!    identity the caller holds. Divergence cannot manufacture such a fact,
//!    because the claim about a non-returning call is the same as the claim
//!    about a returning one.
//!
//! # Lattice and convergence (§4.2)
//!
//! Per member the assumption is [`Assumption`], a two-point lattice
//! `Uncontracted ⊏ Neutral`. Component states are pointwise maps, so the
//! lattice is finite of height `|C| + 1`. The iteration starts at the top
//! element (all-`Neutral`), demotes every member whose body fails validation,
//! and repeats. Each round with any failure demotes at least one member, so the
//! descent terminates in at most `|C|` rounds and computes the **greatest
//! post-fixed point** below all-`Neutral`. No join and no widening is used or
//! needed; the intraprocedural domain and its joins are untouched.
//!
//! Publication follows the descent's fixed point rather than being fixed at
//! component granularity up front (design unit U2). A member could in principle
//! survive its component's partial demotion when every call edge of its own
//! body into a demoted member is graph-unreachable, since only a *reachable*
//! call to a demoted member raises a diagnostic. Measured, no such member
//! survives: a call into a demoted member is an `Unknown` instruction, and
//! [`validate_member`] refuses any body that retains one, because the
//! certificate firewall refuses it downstream. The fixed point of a strongly
//! connected component is therefore observably all-or-nothing, which is the
//! design's V4 default; the deviation from U2's optimism is deliberate and
//! recorded in `docs/reports/remaining-c03-nfr/README.md`.
//!
//! # Alias obligation
//!
//! Empty, by construction. No formal is bound to a resource identity, no
//! `Stored`/`Removed` mode is claimed so the sanctioned containment set is
//! empty, no `Out` mode is claimed so no slot may be written through, and the
//! result is `None` so the handback is untracked — a caller that releases it
//! reports an unresolved alias and stays `Unknown`. NFR requires no alias
//! analysis, no heap-link discovery and no region abstraction, which is exactly
//! why the design rejects heap-link chain teardown (§8) instead of extending
//! this route to it.
use super::*;

use crate::legacy_census::{NeutralComponentOutcome, NeutralMemberRefusal, NeutralRefusal};

/// What one call of [`infer_neutral_component`] decided, for both the caller
/// that installs contracts and the read-only census (card C03-REACH).
///
/// `published` is exactly what the route publishes and is the only field the
/// derivation reads; `outcome` records the same decision in the census
/// vocabulary and is never consulted by any analysis.
pub(super) struct NeutralDecision {
    pub(super) published: Option<BTreeMap<String, ResourceFunctionContract>>,
    pub(super) outcome: NeutralComponentOutcome,
}

/// One member's interprocedural assumption. `Uncontracted` is the conservative
/// element: a call to the member is an unmodelled call, hence `Unknown`.
/// `Neutral` is the optimistic one: the call site applies `N(m)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Assumption {
    Uncontracted,
    Neutral,
}

/// `N(m)`: every formal `Borrowed`, result `None`. This is not new vocabulary —
/// it is exactly the contract ordinary W4 derivation already publishes for a
/// non-recursive body with no custody effects.
pub(super) fn neutral_contract(arity: usize) -> ResourceFunctionContract {
    ResourceFunctionContract::new(
        vec![ResourceParameterMode::Borrowed; arity],
        ResourceReturnMode::None,
    )
}

/// Derive validated neutral summaries for one recursive component, or `None`.
///
/// `declared` is the *declared* environment, consulted only to enforce V1;
/// `environment` is the environment in force at this point of the bottom-up
/// walk, whose contracts for functions outside the component are reused
/// unchanged.
pub(super) fn infer_neutral_component(
    tokens: &[Token],
    component: &[String],
    declared: &ResourceContracts,
    environment: &ResourceContracts,
) -> NeutralDecision {
    // (V1) A declared contract keeps the component on the existing route.
    // Declared precedence is never touched, and this route never proposes a
    // contract for a function whose interface a `.qcore` module already fixes.
    if component
        .iter()
        .any(|name| declared.contract(name).is_some())
    {
        return NeutralDecision {
            published: None,
            outcome: NeutralComponentOutcome::DeclaredContract,
        };
    }
    let mut members = BTreeMap::new();
    for name in component {
        let Some(parameters) = function_parameters(tokens, name) else {
            return NeutralDecision {
                published: None,
                outcome: NeutralComponentOutcome::Refused {
                    members: vec![NeutralMemberRefusal {
                        function: name.clone(),
                        refusal: NeutralRefusal::ParametersUnreadable,
                    }],
                },
            };
        };
        let contract = neutral_contract(parameters.len());
        members.insert(name.clone(), (parameters, contract));
    }
    // (V3) The seeded assumption is exactly `N(m)` for every member; no other
    // candidate is ever installed for a member of its own component.
    let mut assumption = component
        .iter()
        .map(|name| (name, Assumption::Neutral))
        .collect::<BTreeMap<_, _>>();
    // (V4) The descent of §4.2. The bound is written out rather than left
    // implicit: each round that demotes anything demotes at least one member,
    // so `|C| + 1` rounds suffice to reach the greatest post-fixed point or to
    // exhaust the component.
    // Census only: the refusal each member reported the first time it was
    // examined, i.e. under the all-`Neutral` seed. Later rounds re-examine a
    // survivor under a weaker map, so the first reading is the one that
    // describes the member's own body rather than a cascade from a sibling.
    let mut first_refusals: Vec<NeutralMemberRefusal> = Vec::new();
    for _ in 0..=component.len() {
        let environment = component_environment(component, environment, &members, &assumption);
        let mut demoted = Vec::new();
        for name in component {
            if assumption[name] != Assumption::Neutral {
                continue;
            }
            let (parameters, contract) = &members[name];
            if let Err(refusal) = explain_member(tokens, name, &environment, parameters, contract) {
                if !first_refusals.iter().any(|entry| entry.function == *name) {
                    first_refusals.push(NeutralMemberRefusal {
                        function: name.clone(),
                        refusal,
                    });
                }
                demoted.push(name);
            }
        }
        if demoted.is_empty() {
            // A stable round is the greatest post-fixed point below
            // all-`Neutral`: every surviving member's body validated under the
            // very assumption map that survives with it.
            let published = component
                .iter()
                .filter(|name| assumption[*name] == Assumption::Neutral)
                .map(|name| (name.clone(), members[name].1.clone()))
                .collect::<BTreeMap<_, _>>();
            if published.is_empty() {
                return NeutralDecision {
                    published: None,
                    outcome: NeutralComponentOutcome::Refused {
                        members: first_refusals,
                    },
                };
            }
            let admitted = published.keys().cloned().collect::<Vec<_>>();
            return NeutralDecision {
                published: Some(published),
                outcome: NeutralComponentOutcome::Admitted {
                    members: admitted,
                    refused: first_refusals,
                },
            };
        }
        for name in demoted {
            assumption.insert(name, Assumption::Uncontracted);
        }
    }
    NeutralDecision {
        published: None,
        outcome: NeutralComponentOutcome::Refused {
            members: first_refusals,
        },
    }
}

/// The contract environment one descent round runs under: everything already in
/// force outside the component, plus `N(m)` for exactly the members the current
/// assumption map still holds at `Neutral`. A demoted member contributes no
/// contract, so every reachable call to it is an unmodelled call.
fn component_environment(
    component: &[String],
    environment: &ResourceContracts,
    members: &BTreeMap<String, (Vec<Option<String>>, ResourceFunctionContract)>,
    assumption: &BTreeMap<&String, Assumption>,
) -> ResourceContracts {
    let mut round = ResourceContracts::default();
    round.set_source_assumptions(environment.source_assumptions().clone());
    round.carry_cross_unit_imports(environment);
    for (name, contract) in environment.functions() {
        if !component.contains(name) {
            round.insert_contract(name.clone(), contract.clone());
        }
    }
    for name in component {
        if assumption[name] == Assumption::Neutral {
            round.insert_contract(name.clone(), members[name].1.clone());
        }
    }
    round
}

#[cfg(test)]
#[test]
fn neutral_recursive_round_keeps_cross_unit_import() {
    use qcore_schemas::contract::CrossUnitImportIdentity;
    let identity = CrossUnitImportIdentity {
        raw_source_sha256: "a".repeat(64),
        preprocessed_source_sha256: "b".repeat(64),
        scope_evidence_sha256: "c".repeat(64),
        analysis_request_sha256: "d".repeat(64),
        environment_sha256: "e".repeat(64),
    };
    let mut environment = ResourceContracts::default();
    environment.set_inferred_imports(BTreeMap::from([(
        "external".to_owned(),
        ResourceFunctionContract::default(),
    )]));
    environment
        .set_cross_unit_import_identities(BTreeMap::from([("external".to_owned(), identity)]));
    environment.set_link_resolution_sha256(Some("f".repeat(64)));
    let component = ["recursive".to_owned()];
    let members = BTreeMap::from([(component[0].clone(), (Vec::new(), neutral_contract(0)))]);
    let assumption = BTreeMap::from([(&component[0], Assumption::Neutral)]);
    let round = component_environment(&component, &environment, &members, &assumption);
    assert_eq!(round.inferred_imports(), environment.inferred_imports());
    assert_eq!(
        round.cross_unit_import_identities(),
        environment.cross_unit_import_identities()
    );
    assert_eq!(
        round.link_resolution_sha256(),
        environment.link_resolution_sha256()
    );
}

/// Validate one member's body against `N(m)` under the round's assumption,
/// naming the first obligation that refuses it.
///
/// This is where V5–V8 are discharged. Nothing here is bespoke: the entry state
/// is the ordinary one the contract prescribes, and the gate is the unchanged
/// [`observations_match_contract`].
///
/// The `Err` payload is read only by the read-only census (card C03-REACH);
/// the route itself branches on `is_err` alone, so naming the obligation
/// cannot change what is published. The one decomposition — testing
/// [`observations_match_contract`]'s own first condition (a `Safe` analysis
/// with no diagnostics) before calling it — is behaviour-preserving because
/// that condition is exactly what the gate tests first.
fn explain_member(
    tokens: &[Token],
    function: &str,
    environment: &ResourceContracts,
    parameters: &[Option<String>],
    contract: &ResourceFunctionContract,
) -> Result<(), NeutralRefusal> {
    let cfg = build_function_cfg(tokens, function, environment)
        .map_err(|error| NeutralRefusal::CfgUnavailable(error.to_string()))?;
    // The neutral contract binds nothing, so this is the *empty* entry custody
    // (V5): no parameter site, no out slot, a non-nullable result.
    let Some(entry) = EntryCustody::build(&cfg, parameters, contract) else {
        return Err(NeutralRefusal::EntryUnavailable);
    };
    if entry.parameter_sites.iter().any(Option::is_some)
        || entry.out_slots.iter().any(Option::is_some)
        || entry.result_nullable
    {
        return Err(NeutralRefusal::NonEmptyEntry);
    }
    // (V5, containment) A node the analysis never reached raises no diagnostic,
    // so an unmodelled construct can survive an otherwise clean run by sitting
    // on a graph-unreachable path — a call into a member this round demoted is
    // exactly that shape. The certificate firewall refuses any retained
    // `Unknown` instruction, so publishing such a member would offer a summary
    // its own containment check later rejects. Refuse it here instead, as the
    // contextual route already refuses a context CFG that retains one.
    let retained = cfg
        .nodes
        .iter()
        .filter_map(|node| match &node.instruction {
            LegacyInstruction::Unknown { description, .. } => {
                Some((node.line, description.clone()))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if !retained.is_empty() {
        return Err(NeutralRefusal::RetainedUnknownInstruction {
            descriptions: retained,
        });
    }
    let (summary, observations) = analyze_cfg_with_entry(cfg, &entry);
    // (V5) `Safe` with zero diagnostics — an unresolved alias, an unsupported
    // construct, a call to a demoted or uncontracted callee, a leak or a
    // summary conflict each refuse the member.
    // (V8) At least one exit is reachable *in the abstract semantics*; this is
    // explicitly not a claim that an exit is concretely reachable.
    // (V6) Every reachable exit leaves every formal untracked, the sanctioned
    // containment set is empty, no out slot is claimed or written, and no exit
    // returns a tracked resource.
    if summary.status != AnalysisStatus::Safe || !summary.diagnostics.is_empty() {
        return Err(NeutralRefusal::BodyNotClean {
            status: summary.status,
            diagnostics: summary.diagnostics.clone(),
        });
    }
    if !observations_match_contract(contract, &entry, &summary, &observations) {
        return Err(if observations.is_empty() {
            NeutralRefusal::NoReachableExit
        } else {
            NeutralRefusal::ExitsDisagree
        });
    }
    // (V7) The neutral contract must be what validation *reads off* the body,
    // not merely something consistent with it. Reading the modes back off the
    // settled observations must reproduce `N(m)` exactly; there is no fallback
    // to a stronger mode, and a body whose observations read as anything else
    // refuses the candidate outright.
    if propose_contract(&entry, &observations).as_ref() != Some(contract) {
        return Err(NeutralRefusal::ContractNotReadBack);
    }
    Ok(())
}
