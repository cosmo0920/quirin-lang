//! Interprocedural parameter/return custody summaries for the unrestricted-C
//! gateway (W4 of `docs/GENERALIZATION_PLAN.md`; closes LC01's open boundary in
//! `docs/REEVALUATION_FOLLOWUP.md`).
//!
//! # What this pass produces
//!
//! For each internal function of a translation unit the pass tries to derive a
//! [`ResourceFunctionContract`] — the *same* record a `.qcore` module declares
//! and a provider proposes — describing what the call does to each argument and
//! what custody the result carries. Derived summaries are published as
//! [`ResourceSummaryRecord`]s labelled [`ContractTrust::InferredValidated`], so
//! a consumer routes them by trust label rather than by origin.
//!
//! # The two-step discipline: infer, then validate
//!
//! Inference alone is worthless here: a summary that merely *looks* consistent
//! with a body would let a wrapper hide the very leak LC01 reproduced. Every
//! candidate is therefore re-checked by running the ordinary intraprocedural
//! analyzer a second time, with the entry state the candidate prescribes
//! ([`EntryCustody`]), and demanding that
//!
//! 1. the body is fully modeled and defect-free (`Safe`, no diagnostics) —
//!    any unsupported construct, unresolved alias, or leak rejects it;
//! 2. at least one exit is reachable;
//! 3. every reachable exit disposes of every formal exactly as the candidate
//!    says (released, consumed, or still live), with no disagreement between
//!    exits;
//! 4. no fresh resource is left attached to a caller-owned formal, because such
//!    a transfer is invisible at the call site;
//! 5. the returned value matches the declared result mode on every exit, and a
//!    formal's own site is never returned as if it were fresh.
//!
//! A candidate that fails any of these yields no summary at all, and the call
//! stays an uncontracted call — that is, `Unknown`. Precision is never bought
//! with an unvalidated claim.
//!
//! # Lattice and refinement
//!
//! Parameter modes are ordered by how much the caller must guarantee:
//! `Borrowed` (nothing) below `Owned` (a live non-null resource across the
//! call), with `Released` and `Consumed` describing effects rather than
//! requirements. The lattice the search actually walks is the *binding
//! pattern* — which formals are seeded with an entry resource at all — because
//! `Owned`, `Consumed`, and `Released` prescribe the same entry state and
//! differ only in the exit disposition they claim.
//!
//! Iteration starts from the empty binding, and this is a soundness
//! requirement rather than a tuning choice: seeding a formal with a live
//! non-null resource also asserts that it is *not null*, which refutes a null
//! test in the body and deletes the failure path a summary has to describe.
//! A formal is bound only after the analysis reports it as an unresolved
//! resource identity — that is, only once the body has been observed treating
//! it as a resource it owns. The binding set only grows, so the loop settles
//! after at most one round per formal. Once it is stable, each bound formal's
//! mode is *read off* its exit disposition (live gives `Owned`, released gives
//! `Released`, consumed gives `Consumed`), unbound formals stay `Borrowed`, and
//! the resulting contract is checked against a fresh analysis run.
//!
//! # What is deliberately not inferred
//!
//! The inferred vocabulary is `Borrowed`/`Owned`/`Consumed`/`Released`
//! parameters and `None`/`Owned` results. `Out`, `Stored`, `Removed`, and
//! `Reallocated` are not derived, and a body that needs one of them yields no
//! summary at all:
//!
//! - an `Out` formal is written through `*out = …`, a place the reduced
//!   importer does not model, so the body is not fully modeled;
//! - `Stored` would hand a fresh resource to a caller-owned aggregate, which
//!   condition (4) rejects outright;
//! - `Reallocated` makes the argument's custody depend on whether the result
//!   was null, and the analyzer refuses to escape a possibly-null place — the
//!   settled shape a validator would need never arises.
//!
//! Likewise, a formal handed straight back to the caller, a formal whose
//! disposition differs between exits, and a possibly-null result returned
//! without a null test in the callee all yield no summary. Every one of these
//! leaves the call uncontracted, hence `Unknown`.
//!
//! # Recursion
//!
//! Summaries are derived bottom-up over the call graph's strongly connected
//! components, and each component is iterated to a fixed point seeded at
//! "no summary". A member of a nontrivial component (or a self-recursive
//! function) has at least one direct callee inside its own component, so its
//! body always contains an unmodeled call and never validates; the least fixed
//! point for a recursive component is therefore empty, and its members stay
//! `Unknown`. The pass deliberately does not accept the *greatest* fixed point
//! — an assume-guarantee summary that is merely consistent with itself — since
//! such a summary is only sound for terminating executions and this gateway
//! must not certify a non-terminating program as `Safe`. This mirrors
//! `whole_program.rs`, where a recursive QCore function must *declare* its
//! contract rather than have one inferred.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::{
    ContractTrust, ResourceContracts, ResourceFunctionContract, ResourceParameterMode,
    ResourceReturnMode, ResourceSummaryRecord,
};

use crate::legacy_c::{
    AnalysisStatus, EntryCustody, ExitObservation, LegacyDiagnosticKind, LegacyFunctionSummary,
    ReturnedValue, SiteDisposition, Token, analyze_cfg_with_entry, build_function_cfg,
    direct_calls, function_parameters,
};

/// The custody environment a translation unit's analysis runs under, together
/// with the provenance of every summary in it.
#[derive(Clone, Debug, Default)]
pub(crate) struct CustodyDerivation {
    /// Contracts in force at call sites: declared contracts take precedence
    /// over inferred ones, always.
    environment: ResourceContracts,
    /// Validated inferred summaries, including ones an explicit contract
    /// overrode (kept so the disagreement can be reported).
    inferred: BTreeMap<String, ResourceFunctionContract>,
    /// Functions whose validated inference disagrees with their declaration.
    conflicts: BTreeMap<String, (ResourceFunctionContract, ResourceFunctionContract)>,
}

impl CustodyDerivation {
    pub(crate) fn environment(&self) -> &ResourceContracts {
        &self.environment
    }

    /// The contract a defined function's own body is checked against.
    pub(crate) fn entry_contract(&self, function: &str) -> Option<&ResourceFunctionContract> {
        self.environment.contract(function)
    }

    /// `(inferred, declared)` when the two disagree for `function`.
    pub(crate) fn conflict(
        &self,
        function: &str,
    ) -> Option<(&ResourceFunctionContract, &ResourceFunctionContract)> {
        self.conflicts
            .get(function)
            .map(|(inferred, declared)| (inferred, declared))
    }

    /// The summaries this derivation contributed, as schema records. Declared
    /// contracts are not repeated here: they travel with the environment.
    pub(crate) fn records(&self) -> Vec<ResourceSummaryRecord> {
        self.inferred
            .iter()
            .filter(|(name, _)| !self.conflicts.contains_key(*name))
            .map(|(name, contract)| {
                ResourceSummaryRecord::new(
                    name.clone(),
                    ContractTrust::InferredValidated,
                    contract.clone(),
                )
            })
            .collect()
    }
}

/// Derive and validate custody summaries for `names`, bottom-up over the call
/// graph. `contracts` supplies the declared environment, which is never
/// overridden.
pub(crate) fn derive_custody_summaries(
    tokens: &[Token],
    names: &[String],
    contracts: &ResourceContracts,
) -> CustodyDerivation {
    let mut derivation = CustodyDerivation {
        environment: contracts.clone(),
        ..CustodyDerivation::default()
    };
    let candidates = names.iter().cloned().collect::<BTreeSet<_>>();
    let graph = candidates
        .iter()
        .map(|name| (name.clone(), direct_calls(tokens, name, &candidates)))
        .collect::<BTreeMap<_, _>>();

    for component in strongly_connected_components(&graph) {
        // Seeded at "no summary" and iterated until nothing more installs.
        // A member is retried only while some other member of the same
        // component became available, which keeps this linear for the singleton
        // components that make up the acyclic part of the graph.
        let mut pending = component;
        loop {
            let mut installed = false;
            let mut still_pending = Vec::new();
            for name in pending {
                match infer_validated_summary(tokens, &name, &derivation.environment) {
                    Some(contract) => {
                        match contracts.contract(&name) {
                            // An explicit contract always wins. A disagreement
                            // is reported, never silently resolved either way.
                            Some(declared) if *declared != contract => {
                                derivation
                                    .conflicts
                                    .insert(name.clone(), (contract.clone(), declared.clone()));
                            }
                            Some(_) => {}
                            None => {
                                derivation
                                    .environment
                                    .insert_contract(name.clone(), contract.clone());
                                installed = true;
                            }
                        }
                        derivation.inferred.insert(name, contract);
                    }
                    None => still_pending.push(name),
                }
            }
            pending = still_pending;
            if !installed || pending.is_empty() {
                break;
            }
        }
    }
    derivation
}

/// Infer one function's summary and validate it, or return `None`.
fn infer_validated_summary(
    tokens: &[Token],
    function: &str,
    environment: &ResourceContracts,
) -> Option<ResourceFunctionContract> {
    let parameters = function_parameters(tokens, function)?;
    let cfg = build_function_cfg(tokens, function, environment).ok()?;

    // Start with no formal bound. Seeding a formal with a live non-null
    // resource is not a neutral act: it also asserts the formal is *not null*,
    // which would refute null tests in the body and silently discard the very
    // failure paths a summary has to describe. A formal is bound only once the
    // analysis reports that the body already treats it as a resource it owns.
    let mut bound = vec![false; parameters.len()];
    for _ in 0..=parameters.len() {
        let probe = binding_contract(&parameters, &bound);
        let entry = EntryCustody::build(&cfg, &parameters, &probe)?;
        let (summary, observations) = analyze_cfg_with_entry(cfg.clone(), &entry);
        if bind_unresolved_formals(&mut bound, &parameters, &summary) {
            continue;
        }
        if summary.status != AnalysisStatus::Safe || !summary.diagnostics.is_empty() {
            return None;
        }
        // The binding pattern is stable, so read the modes off the settled
        // observations and check the resulting contract against a fresh run.
        let candidate = propose_contract(&probe, &entry, &observations)?;
        let entry = EntryCustody::build(&cfg, &parameters, &candidate)?;
        let (summary, observations) = analyze_cfg_with_entry(cfg, &entry);
        return observations_match_contract(&candidate, &entry, &summary, &observations)
            .then_some(candidate);
    }
    None
}

/// The contract that binds exactly the formals marked in `bound`. Only the
/// binding pattern matters here: `Owned`, `Consumed`, and `Released` all seed
/// the same live non-null entry resource, so the eventual mode is read from the
/// observed exit disposition rather than guessed up front.
fn binding_contract(parameters: &[Option<String>], bound: &[bool]) -> ResourceFunctionContract {
    ResourceFunctionContract::new(
        parameters
            .iter()
            .zip(bound)
            .map(|(name, bound)| match (name, bound) {
                (Some(_), true) => ResourceParameterMode::Owned { nullable: false },
                _ => ResourceParameterMode::Borrowed,
            }),
        ResourceReturnMode::None,
    )
}

/// Read a candidate contract off the probe run's exit observations.
fn propose_contract(
    probe: &ResourceFunctionContract,
    entry: &EntryCustody,
    observations: &[ExitObservation],
) -> Option<ResourceFunctionContract> {
    let mut modes = Vec::with_capacity(probe.parameters.len());
    for index in 0..probe.parameters.len() {
        if entry.parameter_sites[index].is_none() {
            modes.push(ResourceParameterMode::Borrowed);
            continue;
        }
        let mut disposition = None;
        for observation in observations {
            let current = observation.parameters[index]?;
            match disposition {
                None => disposition = Some(current),
                Some(previous) if previous == current => {}
                // Released on one path and live on another: no single mode.
                Some(_) => return None,
            }
        }
        modes.push(match disposition? {
            // The formal is bound because the body needs it live, and it is
            // still live at every exit: the caller keeps custody.
            SiteDisposition::Live => ResourceParameterMode::Owned { nullable: false },
            SiteDisposition::Released => ResourceParameterMode::Released { nullable: false },
            SiteDisposition::Escaped => ResourceParameterMode::Consumed { nullable: false },
            SiteDisposition::Contained | SiteDisposition::Absent | SiteDisposition::Mixed => {
                return None;
            }
        });
    }

    let result = propose_result(entry, observations)?;
    Some(ResourceFunctionContract::new(modes, result))
}

fn propose_result(
    entry: &EntryCustody,
    observations: &[ExitObservation],
) -> Option<ResourceReturnMode> {
    let parameter_sites = entry
        .parameter_sites
        .iter()
        .flatten()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut returns_nothing = false;
    let mut returned = Vec::new();

    for observation in observations {
        match observation.returned {
            ReturnedValue::Ambiguous => return None,
            ReturnedValue::None => {
                if !observation.pending_replacements.is_empty() {
                    return None;
                }
                returns_nothing = true;
            }
            ReturnedValue::Site {
                site,
                nullable,
                disposition,
            } => {
                // A formal handed straight back is an alias the summary
                // vocabulary cannot express without double-counting custody.
                if parameter_sites.contains(&site) || disposition != SiteDisposition::Escaped {
                    return None;
                }
                // An unresolved reallocation would make the result's custody
                // path-dependent, which no inferred mode here expresses.
                if !observation.pending_replacements.is_empty() {
                    return None;
                }
                returned.push(nullable);
            }
        }
    }

    if returned.is_empty() {
        return Some(ResourceReturnMode::None);
    }
    Some(ResourceReturnMode::Owned {
        // A path that returns no resource makes the result possibly null.
        nullable: returns_nothing || returned.into_iter().any(|nullable| nullable),
    })
}

/// Bind every not-yet-bound formal the analysis reported as an unresolved
/// resource identity. Returns whether anything changed; the binding set only
/// grows, so the surrounding loop terminates after at most one round per
/// formal.
fn bind_unresolved_formals(
    bound: &mut [bool],
    parameters: &[Option<String>],
    summary: &LegacyFunctionSummary,
) -> bool {
    let unresolved = summary
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::UnknownAlias { variable } => Some(variable.as_str()),
            _ => None,
        })
        // Aggregate diagnostics name an `owner/item` pair.
        .flat_map(|variable| variable.split('/'))
        .map(base_identifier)
        .collect::<BTreeSet<_>>();
    let mut changed = false;
    for (index, name) in parameters.iter().enumerate() {
        let Some(name) = name else { continue };
        if !bound[index] && unresolved.contains(name.as_str()) {
            bound[index] = true;
            changed = true;
        }
    }
    changed
}

/// The variable a resource place is rooted at: `s->field[0]` roots at `s`.
fn base_identifier(place: &str) -> &str {
    let end = place.find(['.', '-', '[']).unwrap_or(place.len());
    &place[..end]
}

/// Check a contract against the body that is supposed to implement it.
///
/// This is the single gate between "inferred" and "trusted at call sites"; the
/// numbered conditions are the ones in this module's header comment.
pub(crate) fn observations_match_contract(
    contract: &ResourceFunctionContract,
    entry: &EntryCustody,
    summary: &LegacyFunctionSummary,
    observations: &[ExitObservation],
) -> bool {
    // (1) A summary is only as good as the analysis that produced it.
    if summary.status != AnalysisStatus::Safe || !summary.diagnostics.is_empty() {
        return false;
    }
    // (2) A function with no reachable exit describes no call behavior.
    if observations.is_empty() {
        return false;
    }
    if contract.parameters.len() != entry.parameter_sites.len() {
        return false;
    }
    let parameter_sites = entry
        .parameter_sites
        .iter()
        .flatten()
        .copied()
        .collect::<BTreeSet<_>>();
    let reallocated = match contract.result {
        ResourceReturnMode::Reallocated { parameter, .. } => Some(parameter),
        _ => None,
    };
    observations.iter().all(|observation| {
        // (4) Custody may not leak into a caller-owned aggregate unannounced.
        !observation.attached_to_parameter
            && observation.parameters.len() == contract.parameters.len()
            && contract
                .parameters
                .iter()
                .copied()
                .enumerate()
                .all(|(index, mode)| {
                    reallocated == Some(index)
                        || parameter_matches(mode, observation.parameters[index])
                })
            && result_matches(contract, observation, &parameter_sites)
    })
}

/// (3) One formal's exit disposition against its declared mode.
fn parameter_matches(mode: ResourceParameterMode, observed: Option<SiteDisposition>) -> bool {
    let Some(observed) = observed else {
        // Nothing was bound, so the body could not have used the formal as a
        // resource without raising an unresolved alias, which condition (1)
        // already rejected.
        return matches!(
            mode,
            ResourceParameterMode::Borrowed | ResourceParameterMode::Out { .. }
        );
    };
    let (expected, nullable) = match mode {
        ResourceParameterMode::Borrowed | ResourceParameterMode::Out { .. } => return false,
        ResourceParameterMode::Owned { nullable }
        | ResourceParameterMode::Stored { nullable, .. }
        | ResourceParameterMode::Removed { nullable, .. } => (SiteDisposition::Live, nullable),
        ResourceParameterMode::Consumed { nullable } => (SiteDisposition::Escaped, nullable),
        ResourceParameterMode::Released { nullable } => (SiteDisposition::Released, nullable),
    };
    // A nullable formal carries no custody on the path where it was null.
    observed == expected || (nullable && observed == SiteDisposition::Absent)
}

/// (5) One exit's returned value against the declared result mode.
fn result_matches(
    contract: &ResourceFunctionContract,
    observation: &ExitObservation,
    parameter_sites: &BTreeSet<usize>,
) -> bool {
    match contract.result {
        ResourceReturnMode::None => {
            observation.returned == ReturnedValue::None
                && observation.pending_replacements.is_empty()
        }
        ResourceReturnMode::Owned { nullable } => match observation.returned {
            // Returning nothing is only consistent with a nullable result.
            ReturnedValue::None => nullable && observation.pending_replacements.is_empty(),
            ReturnedValue::Ambiguous => false,
            ReturnedValue::Site {
                site,
                nullable: site_nullable,
                disposition,
            } => {
                !parameter_sites.contains(&site)
                    && disposition == SiteDisposition::Escaped
                    && (nullable || !site_nullable)
                    && observation.pending_replacements.is_empty()
            }
        },
        // A conditional replacement makes the argument's custody depend on
        // whether the result was null, which this validator cannot confirm from
        // a may-alias exit state: the analyzer refuses to escape a possibly-null
        // place in the first place, so the shape never reaches a settled exit.
        // A function declared `Reallocated` therefore falls back to the
        // contract-free analysis rather than being certified against a claim
        // that was never checked.
        ResourceReturnMode::Reallocated { .. } => false,
    }
}

/// Tarjan's algorithm, iteratively, returning components in reverse
/// topological order: every callee's component precedes its callers'.
fn strongly_connected_components(graph: &BTreeMap<String, BTreeSet<String>>) -> Vec<Vec<String>> {
    #[derive(Clone, Copy)]
    struct Node {
        index: usize,
        low: usize,
        on_stack: bool,
    }

    let nodes = graph.keys().cloned().collect::<Vec<_>>();
    let position = nodes
        .iter()
        .enumerate()
        .map(|(index, name)| (name.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let successors = nodes
        .iter()
        .map(|name| {
            graph[name]
                .iter()
                .filter_map(|callee| position.get(callee).copied())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let mut state = vec![None::<Node>; nodes.len()];
    let mut stack = Vec::new();
    let mut components = Vec::new();
    let mut next_index = 0usize;

    for root in 0..nodes.len() {
        if state[root].is_some() {
            continue;
        }
        // (node, next successor to visit)
        let mut work = vec![(root, 0usize)];
        state[root] = Some(Node {
            index: next_index,
            low: next_index,
            on_stack: true,
        });
        next_index += 1;
        stack.push(root);
        while let Some((node, cursor)) = work.pop() {
            if cursor < successors[node].len() {
                work.push((node, cursor + 1));
                let successor = successors[node][cursor];
                match state[successor] {
                    None => {
                        state[successor] = Some(Node {
                            index: next_index,
                            low: next_index,
                            on_stack: true,
                        });
                        next_index += 1;
                        stack.push(successor);
                        work.push((successor, 0));
                    }
                    Some(existing) if existing.on_stack => {
                        let current = state[node].expect("visited node");
                        state[node] = Some(Node {
                            low: current.low.min(existing.index),
                            ..current
                        });
                    }
                    Some(_) => {}
                }
                continue;
            }
            let current = state[node].expect("visited node");
            if current.low == current.index {
                let mut component = Vec::new();
                while let Some(member) = stack.pop() {
                    let member_state = state[member].expect("stacked node");
                    state[member] = Some(Node {
                        on_stack: false,
                        ..member_state
                    });
                    component.push(nodes[member].clone());
                    if member == node {
                        break;
                    }
                }
                component.sort();
                components.push(component);
            }
            if let Some((parent, _)) = work.last().copied() {
                let parent_state = state[parent].expect("visited node");
                state[parent] = Some(Node {
                    low: parent_state.low.min(current.low),
                    ..parent_state
                });
            }
        }
    }
    components
}

#[cfg(test)]
#[path = "../tests/unit/legacy_summary.rs"]
mod tests;
