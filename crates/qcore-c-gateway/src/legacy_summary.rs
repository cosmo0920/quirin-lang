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
//! 4. the containment attached to sites the caller holds is exactly what the
//!    contract's `Stored` modes declare, because any other transfer into a
//!    caller-held aggregate is invisible at the call site;
//! 5. the returned value matches the declared result mode on every exit, and a
//!    formal's own site is never returned as if it were fresh;
//! 6. every out slot the contract declares holds a distinct fresh live
//!    resource at every exit (or a null constant, if the mode is nullable),
//!    and no formal without an `Out` mode is written through.
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
//! Same-type immutable entry-prefix output aliases are normalized only after
//! complete use classification. The original Out slot still requires all-exit
//! validation; mutable, escaping and branch-local aliases remain unsupported.
//!
//! The inferred vocabulary includes bounded direct `Removed` wrappers: the
//! child starts contained in the distinct non-null owner formal named by a
//! modeled `Removed` call, and validation requires it detached/live on every
//! non-null exit; nullable formals carry no child custody on null paths.
//! General heap-link extraction and alias-selected removal are not inferred.
//! The ordinary live-entry search
//! still takes precedence, so store-then-remove does not gain a false entry
//! containment requirement.
//!
//! Card W43-RESULT-ALIAS-RETURNED adds a third ordinary route, consulted only
//! after the two above publish nothing: a body whose every exit returns NULL or
//! a pointer derived from one borrowed formal gets a validated
//! `ResourceReturnMode::Derived` summary ([`derived`]). The formal is tracked by
//! a name-only site that carries no custody, so the summary's `Borrowed` claim
//! for it is checked, not assumed.
//!
//! Likewise, a formal handed straight back to the caller, a formal whose
//! disposition differs between exits with no reallocation explaining it, an
//! out slot left unwritten on some path, and an out slot holding a value the
//! callee never resolved to non-null all yield no summary. Every one of these
//! leaves the call uncontracted, hence `Unknown`.
//!
//! # Recursion
//!
//! Ordinary inference runs first and never assumes a recursive summary. A
//! bounded second route partitions one immutable explicit integer formal into
//! Zero and NonZero. Zero validates without any SCC contract; NonZero may call
//! literal-zero contexts already validated at rank zero. Explicit unsigned
//! components also admit exact `control - 1` actuals by mutual strong induction
//! over concrete natural counter values (including unsigned width conversion).
//! Only those proved scalar actuals are abstracted in the resource CFG. Equal contracts
//! for both contexts are rechecked over a disjoint combined CFG before they
//! become caller contracts. Card CG27-LOOPS-RECURSION-BUDGETS (gateway rule
//! 11x) adds the signed threshold rank, consulted *last*, only for a component
//! every route above and the neutral route below published nothing for: one
//! `int`-or-wider signed control, sign guards `n <= c` / `n > c` at one literal
//! threshold, and recursive actuals `n - k` (literal `1 <= k <= 1024`, several
//! ranks at once) or `0`, validated by the same base/step discipline over the
//! rank `max(n - c, 0)` ([`crate::legacy_c::recursion::signed_rank`]).
//! Declared SCCs, unsupported control syntax, zero
//! cycles and general recursion remain outside this contextual route. This is
//! well-founded ranked validation, not a coinductive assumed contract or a general
//! termination proof. Unreachable recursive edges may already validate on the
//! ordinary route under its explicit resource entry preconditions.
//!
//! A third route, [`neutral`] (card C03's neutral-frame recursion), applies only
//! after the two above have published nothing for an undeclared recursive
//! component. It leaves the control shape completely free and fixes the *effect
//! content* at the bottom element instead: the only contract it ever seeds for a
//! member of its own component is `N(m)` — every formal `Borrowed`, result
//! `None` — and it descends a two-point per-member lattice to the greatest
//! post-fixed point below all-`Neutral`. Because that assumption states no
//! precondition and asserts only negative, prefix-closed facts, the optimistic
//! start is admissible there and nowhere above it; an effectful assumption
//! validates vacuously under divergence and therefore still requires a rank.
//!
//! # Self-dependent records
//!
//! A validated summary is published as independent corroboration of a body. A
//! record derived while a declared contract for a member of the same recursive
//! component was in force is not that: the recursive call site read the
//! declaration back. Such records are withheld from [`CustodyDerivation::records`]
//! (design unit U7). The declaration still governs every call site — no
//! authority is lost — but a certificate consumer can no longer read a
//! declaration back as an inference.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::{
    ContractTrust, DerivedExtent, ResourceContracts, ResourceFunctionContract,
    ResourceParameterMode, ResourceReturnMode, ResourceSummaryRecord,
};

use crate::legacy_c::{
    AnalysisStatus, EntryCustody, ExitObservation, LegacyCfg, LegacyDiagnosticKind,
    LegacyFunctionSummary, LegacyInstruction, OutSlotValue, ReturnedValue, SiteDisposition, Token,
    analyze_cfg_with_entry, build_function_cfg, direct_calls, function_parameters,
};
use crate::legacy_census::{NeutralComponentCensus, NeutralComponentOutcome};

/// The custody environment a translation unit's analysis runs under, together
/// with the provenance of every summary in it.
#[derive(Clone, Debug, Default)]
pub(crate) struct CustodyDerivation {
    /// Contracts in force at call sites: declared contracts take precedence
    /// over inferred ones, always.
    environment: ResourceContracts,
    contextual: BTreeMap<String, LegacyCfg>,
    /// Validated inferred summaries, including ones an explicit contract
    /// overrode (kept so the disagreement can be reported).
    inferred: BTreeMap<String, ResourceFunctionContract>,
    /// Functions whose validated inference disagrees with their declaration.
    conflicts: BTreeMap<String, (ResourceFunctionContract, ResourceFunctionContract)>,
    /// Members of a recursive component in which some member carries a declared
    /// contract. Their inferred records were derived with that declaration in
    /// force at a call site the component reaches, so they are not independent
    /// corroboration of anything and are never published (design unit U7).
    self_dependent: BTreeSet<String>,
    /// Read-only record of how each *recursive* component was disposed of
    /// (card C03-REACH). Written by this pass, read by
    /// [`crate::legacy_census`] alone: no analysis, certificate or report
    /// consults it, so it cannot move a verdict.
    census: Vec<NeutralComponentCensus>,
}

impl CustodyDerivation {
    pub(crate) fn contextual_cfg(&self, function: &str) -> Option<&LegacyCfg> {
        self.contextual.get(function)
    }

    pub(crate) fn environment(&self) -> &ResourceContracts {
        &self.environment
    }

    /// The recursive-component census this derivation recorded (C03-REACH).
    /// Read-only evidence; nothing in the analysis consults it.
    pub(crate) fn census(self) -> Vec<NeutralComponentCensus> {
        self.census
    }

    /// The contract a defined function's own body is checked against.
    pub(crate) fn entry_contract(&self, function: &str) -> Option<&ResourceFunctionContract> {
        self.environment.contract(function)
    }

    /// The validated summary this derivation read off `function`'s own body,
    /// whether or not a declared contract overrode it.
    ///
    /// Card **LOADER-ABI-MODES** compares it against the declaration. It is
    /// deliberately the *validated* summary and nothing weaker: an entry that
    /// failed validation is absent here, so absence is the fail-closed answer
    /// "nothing to compare" rather than a claim about the body.
    pub(crate) fn inferred(&self, function: &str) -> Option<&ResourceFunctionContract> {
        self.inferred.get(function)
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
    /// contracts are not repeated here: they travel with the environment, and a
    /// record a declaration of the same recursive component produced is
    /// withheld rather than republished as an inference (U7).
    pub(crate) fn records(&self) -> Vec<ResourceSummaryRecord> {
        self.inferred
            .iter()
            .filter(|(name, _)| !self.conflicts.contains_key(*name))
            .filter(|(name, _)| !self.self_dependent.contains(*name))
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

    // Card CG23-RELATIONAL-POSTCONDITIONS: a declared return-code
    // disposition is a declaration of its component like any other.
    let declared = |name: &String| {
        contracts.contract(name).is_some()
            || contracts.relational_contract(name).is_some()
            || contracts.retaining_contract(name).is_some()
    };
    for component in strongly_connected_components(&graph) {
        // Seeded at "no summary" and iterated until nothing more installs.
        // A member is retried only while some other member of the same
        // component became available, which keeps this linear for the singleton
        // components that make up the acyclic part of the graph.
        let mut pending = component.clone();
        loop {
            let mut installed = false;
            let mut still_pending = Vec::new();
            for name in pending {
                // Card CALLEE-RETENTION-CLASS1 (rule 12t): a retaining
                // declaration is never replaced by an inference, which would
                // read its retained pointer as borrowed.
                if contracts.retaining_contract(&name).is_some() {
                    continue;
                }
                // Card CG23-RELATIONAL-POSTCONDITIONS (rule 11r): a declared
                // return-code disposition is checked against the body and
                // never replaced by an inference.
                if let Some(declared) = contracts.relational_contract(&name) {
                    let body = infer_validated_summary(tokens, &name, &derivation.environment)
                        .map(relational::BodySummary::Ordinary)
                        .or_else(|| {
                            relational::infer_exit_specific(
                                tokens,
                                &name,
                                &derivation.environment,
                                infer_validated_summary,
                            )
                            .map(relational::BodySummary::ExitSpecific)
                        })
                        // Card CALLEE-RETENTION-CLASS1 (rule 12s): a body
                        // that forwards another disposition.
                        .or_else(|| {
                            relational_forwarding::infer_forwarded(
                                tokens,
                                &name,
                                &derivation.environment,
                                infer_validated_summary,
                            )
                            .map(relational::BodySummary::Forwarded)
                        });
                    if let Some(reason) = body
                        .as_ref()
                        .and_then(|body| relational::contradiction(declared, body))
                    {
                        derivation.environment.withdraw_relational(&name, reason);
                    }
                    continue;
                }
                match infer_validated_summary(tokens, &name, &derivation.environment) {
                    Some(contract) => {
                        match contracts.contract(&name) {
                            // An explicit contract always wins. A disagreement
                            // is reported, never silently resolved either way.
                            // Card W45-FB-RETRY-LIFETIME: a derived summary
                            // states custody only, so the declaration's
                            // container and counted-user clauses are not part
                            // of the comparison; the counted-user pass checks
                            // them against the body instead.
                            Some(declared) if declared.custody_projection() != contract => {
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
                    // Card CG23-RELATIONAL-POSTCONDITIONS (rule 11r): the
                    // exit-specific route, after every ordinary one.
                    None if contracts.contract(&name).is_none() => {
                        match relational::infer_exit_specific(
                            tokens,
                            &name,
                            &derivation.environment,
                            infer_validated_summary,
                        ) {
                            // Classes that all agree are no exit-specific
                            // relation, and the ordinary route refused that
                            // contract: nothing is published.
                            Some(summary) if summary.join().is_none() => {
                                derivation.environment.insert_relational(name, summary);
                                installed = true;
                            }
                            Some(_) => still_pending.push(name),
                            // Card CALLEE-RETENTION-CLASS1 (rule 12s): the
                            // forwarding route, after the exit-specific one.
                            None => match relational_forwarding::infer_forwarded(
                                tokens,
                                &name,
                                &derivation.environment,
                                infer_validated_summary,
                            ) {
                                Some(summary) => {
                                    derivation.environment.insert_relational(name, summary);
                                    installed = true;
                                }
                                None => still_pending.push(name),
                            },
                        }
                    }
                    None => still_pending.push(name),
                }
            }
            pending = still_pending;
            if !installed || pending.is_empty() {
                break;
            }
        }
        let recursive = component.len() > 1 || graph[&component[0]].contains(&component[0]);
        // (U7) Inside a recursive component a declared contract is what the
        // recursive call site reads, so an inferred record for any member of it
        // repeats that declaration instead of corroborating it. Record the
        // dependence; the declaration keeps governing call sites either way.
        if recursive && component.iter().any(&declared) {
            derivation.self_dependent.extend(component.iter().cloned());
        }
        // Census only (C03-REACH): how this recursive component was disposed
        // of. Assigned on every arm below, read by no analysis.
        let mut outcome = None;
        if recursive && !pending.is_empty() && component.iter().all(|n| !declared(n)) {
            if let Some(contexts) = infer_contextual_component(
                tokens,
                &component,
                &derivation.environment,
                RankedRoute::ZeroNonzero,
            ) {
                let mut ranked = Vec::new();
                for (name, (contract, cfg)) in contexts {
                    // Card CG23-RELATIONAL-POSTCONDITIONS: an exit-specific
                    // summary already published stays the one reading.
                    if derivation.inferred.contains_key(&name)
                        || derivation.environment.relational_contract(&name).is_some()
                    {
                        continue;
                    }
                    derivation
                        .environment
                        .insert_contract(name.clone(), contract.clone());
                    derivation.inferred.insert(name.clone(), contract);
                    derivation.contextual.insert(name.clone(), cfg);
                    ranked.push(name);
                }
                outcome =
                    Some(NeutralComponentOutcome::PublishedByRankedRecursion { members: ranked });
            } else if pending.len() == component.len() {
                // (V2) Ordinary inference and both ranked recursive routes run
                // first. Neutral-frame recursion is consulted only for a
                // component that published nothing, and can therefore never
                // replace or weaken a contract another route published.
                let decision = neutral::infer_neutral_component(
                    tokens,
                    &component,
                    contracts,
                    &derivation.environment,
                );
                if let Some(published) = decision.published {
                    for (name, contract) in published {
                        derivation
                            .environment
                            .insert_contract(name.clone(), contract.clone());
                        derivation.inferred.insert(name, contract);
                    }
                    outcome = Some(decision.outcome);
                } else if let Some(contexts) = infer_contextual_component(
                    tokens,
                    &component,
                    &derivation.environment,
                    RankedRoute::SignedThreshold,
                ) {
                    // Card CG27-LOOPS-RECURSION-BUDGETS (gateway rule 11x): a
                    // component every earlier route left unpublished may carry
                    // a signed threshold rank. Consulted last, so it replaces no
                    // contract and moves no component another route decided;
                    // the neutral route's refusal is what an effectful body
                    // without a rank receives, and the rank is what it lacked.
                    let mut ranked = Vec::new();
                    for (name, (contract, cfg)) in contexts {
                        derivation
                            .environment
                            .insert_contract(name.clone(), contract.clone());
                        derivation.inferred.insert(name.clone(), contract);
                        derivation.contextual.insert(name.clone(), cfg);
                        ranked.push(name);
                    }
                    outcome = Some(NeutralComponentOutcome::PublishedByRankedRecursion {
                        members: ranked,
                    });
                } else {
                    outcome = Some(decision.outcome);
                }
            } else {
                outcome = Some(NeutralComponentOutcome::PartiallyPublished {
                    published: component
                        .iter()
                        .filter(|name| !pending.contains(name))
                        .cloned()
                        .collect(),
                });
            }
        }
        if recursive {
            let outcome = outcome.unwrap_or_else(|| {
                if component.iter().any(&declared) {
                    NeutralComponentOutcome::DeclaredContract
                } else {
                    NeutralComponentOutcome::PublishedByOrdinaryInference
                }
            });
            derivation.census.push(NeutralComponentCensus {
                self_recursive: component.len() == 1,
                members: component,
                outcome,
            });
        }
    }
    premises::record_derivation(&graph, &derivation.inferred, contracts);
    derivation
}

/// Which rank partition [`infer_contextual_component`] reads. Both share the
/// base/step validation below; they differ only in the descriptors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RankedRoute {
    /// `control == 0` / `control != 0`, with unsigned `control - 1` steps.
    ZeroNonzero,
    /// Card CG27-LOOPS-RECURSION-BUDGETS (gateway rule 11x): `control <= c` /
    /// `control > c` over one signed `int`-or-wider control type, with
    /// `control - k` steps ([`crate::legacy_c::recursion::signed_rank`]).
    SignedThreshold,
}

/// Zero is checked with every SCC contract absent. Nonzero admits literal-zero
/// calls to validated bases, or exact unsigned countdowns to strictly smaller
/// natural ranks. The latter uses mutual strong induction, not a finite
/// zero/nonzero dependency cycle or an unguarded recursive self-assumption.
/// Publication requires equal contracts for both exhaustive integer contexts,
/// and fresh analysis of their disjoint union. Certificates derive this again.
fn infer_contextual_component(
    tokens: &[Token],
    component: &[String],
    environment: &ResourceContracts,
    route: RankedRoute,
) -> Option<BTreeMap<String, (ResourceFunctionContract, LegacyCfg)>> {
    use crate::legacy_c::recursion;
    let descriptors = match route {
        RankedRoute::ZeroNonzero => recursion::prepare(tokens, component)?,
        RankedRoute::SignedThreshold => recursion::signed_rank::prepare(tokens, component)?,
    };
    let mut lower = ResourceContracts::default();
    lower.set_source_assumptions(environment.source_assumptions().clone());
    for (name, contract) in environment.functions() {
        if !component.contains(name) {
            lower.insert_contract(name.clone(), contract.clone());
        }
    }
    let mut zeros = BTreeMap::new();
    // Card W47-SUMMARY-VALIDATION-PROTECTED-FORMALS: the environment the zero
    // contexts are built in, kept for the re-check under the reported premises.
    let zero_environment = lower.clone();
    for name in component {
        let descriptor = &descriptors[name];
        let cfg = recursion::contextual_cfg(tokens, name, &lower, descriptor, true)?;
        let contract = infer_cfg_summary(&descriptor.parameters, cfg.clone())?;
        if contract.parameters[descriptor.control_index] != ResourceParameterMode::Borrowed {
            return None;
        }
        zeros.insert(name.clone(), (contract, cfg));
    }
    for (name, (contract, _)) in &zeros {
        lower.insert_contract(name.clone(), contract.clone());
    }
    let mut published = BTreeMap::new();
    for name in component {
        let descriptor = &descriptors[name];
        if !descriptor
            .dependencies
            .iter()
            .all(|callee| zeros.contains_key(callee))
        {
            return None;
        }
        let cfg = recursion::contextual_cfg(tokens, name, &lower, descriptor, false)?;
        let contract = infer_cfg_summary(&descriptor.parameters, cfg.clone())?;
        if contract != zeros[name].0 {
            return None;
        }
        let combined = recursion::combine(zeros[name].1.clone(), cfg);
        let entry = EntryCustody::build(&combined, &descriptor.parameters, &contract)?;
        let (summary, observations) = analyze_cfg_with_entry(combined.clone(), &entry);
        if !observations_match_contract(&contract, &entry, &summary, &observations) {
            return None;
        }
        // Card W47-SUMMARY-VALIDATION-PROTECTED-FORMALS (rule 4b): the
        // combined CFG rebuilt with the candidate in force, which is also the
        // one the reported run reads.
        let combined = premises::contextual_under_reported_premises(
            tokens,
            name,
            &zero_environment,
            &lower,
            descriptor,
            &contract,
            combined,
        )?;
        published.insert(name.clone(), (contract, combined));
    }
    Some(published)
}

/// Infer one function's summary and validate it, or return `None`.
fn infer_validated_summary(
    tokens: &[Token],
    function: &str,
    environment: &ResourceContracts,
) -> Option<ResourceFunctionContract> {
    infer_live_entry_summary(tokens, function, environment)
        .or_else(|| infer_removed_wrapper(tokens, function, environment))
        .or_else(|| infer_derived_result(tokens, function, environment))
}

/// Card **W43-RESULT-ALIAS-RETURNED**: the derived-result route, consulted
/// only after both routes above published nothing. See [`derived`].
fn infer_derived_result(
    tokens: &[Token],
    function: &str,
    environment: &ResourceContracts,
) -> Option<ResourceFunctionContract> {
    let decision = derived::infer_derived_result_summary(tokens, function, environment);
    #[cfg(feature = "indirect-census")]
    match &decision {
        Ok(contract) => crate::indirect_census::record_derived(
            function,
            "admitted",
            Some(format!("{:?}", contract.result)),
        ),
        Err(derived::DerivedRefusal::NotCandidate) => {}
        Err(refusal) => crate::indirect_census::record_derived(function, refusal.label(), None),
    }
    decision.ok()
}

/// Preserve the existing least-binding search before considering a contained
/// entry. In particular, store-then-remove remains an ordinary Owned wrapper.
fn infer_live_entry_summary(
    tokens: &[Token],
    function: &str,
    environment: &ResourceContracts,
) -> Option<ResourceFunctionContract> {
    let parameters = function_parameters(tokens, function)?;
    let cfg = build_function_cfg(tokens, function, environment).ok()?;
    let candidate = infer_cfg_summary(&parameters, cfg.clone())?;
    // Card W47-SUMMARY-VALIDATION-PROTECTED-FORMALS: the premises the
    // function's reported row reads (rule 4b).
    premises::validates_under_reported_premises(
        tokens,
        function,
        environment,
        &parameters,
        &candidate,
        premises::Route::LiveEntry,
        &cfg,
    )
    .then_some(candidate)
}

fn infer_cfg_summary(
    parameters: &[Option<String>],
    cfg: LegacyCfg,
) -> Option<ResourceFunctionContract> {
    // Which formals the body writes through. Unlike a bound formal, an out
    // slot asserts nothing about the entry state, so it can be marked from the
    // syntax alone without deleting any path.
    let out = out_parameter_candidates(&cfg, parameters);

    // Start with no formal bound. Seeding a formal with a live non-null
    // resource is not a neutral act: it also asserts the formal is *not null*,
    // which would refute null tests in the body and silently discard the very
    // failure paths a summary has to describe. A formal is bound only once the
    // analysis reports that the body already treats it as a resource it owns.
    let mut bound = vec![false; parameters.len()];
    // The binding set only grows and each retry binds at least one further
    // formal, so the loop settles after at most one round per formal. The
    // bound is written out rather than left implicit in the loop's shape.
    for _ in 0..=parameters.len() {
        // A formal is either a resource the caller already owns or a slot the
        // callee initializes. A body that treats one formal as both is outside
        // this vocabulary.
        if bound.iter().zip(&out).any(|(bound, out)| *bound && *out) {
            return None;
        }
        let probe = probe_contract(parameters, &bound, &out);
        let entry = EntryCustody::build(&cfg, parameters, &probe)?;
        let (summary, observations) = analyze_cfg_with_entry(cfg.clone(), &entry);
        if bind_unresolved_formals(&mut bound, parameters, &summary) {
            continue;
        }
        if summary.status != AnalysisStatus::Safe || !summary.diagnostics.is_empty() {
            return None;
        }
        // The binding pattern is stable, so read the modes off the settled
        // observations and check the resulting contract against a fresh run.
        let candidate = propose_contract(&entry, &observations)?;
        let entry = EntryCustody::build(&cfg, parameters, &candidate)?;
        let (summary, observations) = analyze_cfg_with_entry(cfg, &entry);
        return observations_match_contract(&candidate, &entry, &summary, &observations)
            .then_some(candidate);
    }
    None
}

/// Infer only the contained precondition explicitly demanded by a modeled
/// Removed call whose child and owner arguments are direct formal names.
/// This does not discover heap links or local aliases. Nullable preconditions
/// are proposed only by weakening a validated entry and rechecking the whole
/// body, including the newly admitted null paths.
/// The syntactic candidate is never evidence: a fresh full analysis must show
/// the original child live and detached, and the original owner still live,
/// at every reachable exit. Reassigning either formal cannot change those sites.
fn infer_removed_wrapper(
    tokens: &[Token],
    function: &str,
    environment: &ResourceContracts,
) -> Option<ResourceFunctionContract> {
    let parameters = function_parameters(tokens, function)?;
    let cfg = build_function_cfg(tokens, function, environment).ok()?;
    let mut modes = vec![ResourceParameterMode::Borrowed; parameters.len()];
    let mut found = false;
    for node in &cfg.nodes {
        let LegacyInstruction::ContractCall {
            arguments,
            contract,
            ..
        } = &node.instruction
        else {
            continue;
        };
        for (index, mode) in contract.parameters.iter().copied().enumerate() {
            let ResourceParameterMode::Removed {
                owner_parameter, ..
            } = mode
            else {
                continue;
            };
            let formal = |argument: usize| {
                let name = arguments.get(argument)?.as_ref()?;
                parameters
                    .iter()
                    .position(|parameter| parameter.as_ref() == Some(name))
            };
            let child = formal(index)?;
            let owner = formal(owner_parameter)?;
            if child == owner {
                return None;
            }
            for (index, required) in [
                (owner, ResourceParameterMode::Owned { nullable: false }),
                (
                    child,
                    ResourceParameterMode::Removed {
                        owner_parameter: owner,
                        nullable: false,
                    },
                ),
            ] {
                if modes[index] != ResourceParameterMode::Borrowed && modes[index] != required {
                    return None;
                }
                modes[index] = required;
            }
            found = true;
        }
    }
    if !found {
        return None;
    }
    let validates = |candidate: &ResourceFunctionContract| {
        let Some(entry) = EntryCustody::build(&cfg, &parameters, candidate) else {
            return false;
        };
        let (summary, observations) = analyze_cfg_with_entry(cfg.clone(), &entry);
        observations_match_contract(candidate, &entry, &summary, &observations)
    };
    let mut candidate = ResourceFunctionContract::new(modes, ResourceReturnMode::None);
    if !validates(&candidate) {
        return None;
    }
    // Admit null independently for each child only when *all* currently
    // admitted entry alternatives validate. Later weakenings retain earlier
    // ones, so the final run also checks their combinations. This is a bounded
    // linear search, not a syntactic assertion that a null guard is sufficient.
    for index in 0..candidate.parameters.len() {
        let ResourceParameterMode::Removed {
            owner_parameter, ..
        } = candidate.parameters[index]
        else {
            continue;
        };
        let mut nullable = candidate.clone();
        nullable.parameters[index] = ResourceParameterMode::Removed {
            owner_parameter,
            nullable: true,
        };
        if validates(&nullable) {
            candidate = nullable;
        }
    }
    // Card W47-SUMMARY-VALIDATION-PROTECTED-FORMALS (rule 4b).
    premises::validates_under_reported_premises(
        tokens,
        function,
        environment,
        &parameters,
        &candidate,
        premises::Route::RemovedWrapper,
        &cfg,
    )
    .then_some(candidate)
}

/// Formals the body stores through (`*out = …`). Reading this off the CFG
/// rather than off a diagnostic keeps the candidate set syntactic and
/// deterministic; whether the store is *modeled* is still decided by the
/// contract in force, and whether the resulting mode is *true* is decided by
/// validation.
fn out_parameter_candidates(cfg: &LegacyCfg, parameters: &[Option<String>]) -> Vec<bool> {
    let mut written = BTreeSet::new();
    for node in &cfg.nodes {
        let destination = match &node.instruction {
            LegacyInstruction::Alias { destination, .. }
            | LegacyInstruction::Clear { destination }
            | LegacyInstruction::ContractCall {
                destination: Some(destination),
                ..
            } => Some(destination),
            _ => None,
        };
        if let Some(name) = destination.and_then(|place| place.strip_prefix('*')) {
            written.insert(name);
        }
        if let LegacyInstruction::ContractCall {
            arguments,
            contract,
            ..
        } = &node.instruction
        {
            for (mode, argument) in contract.parameters.iter().zip(arguments) {
                if matches!(mode, ResourceParameterMode::Out { .. })
                    && let Some(name) = argument
                        .as_deref()
                        .and_then(|place| place.strip_prefix('*'))
                {
                    written.insert(name);
                }
            }
        }
    }
    parameters
        .iter()
        .map(|name| name.as_deref().is_some_and(|name| written.contains(name)))
        .collect()
}

/// The contract the observation run uses. Only the binding pattern and the out
/// slots matter here: `Owned`, `Consumed`, `Released`, and `Stored` all seed
/// the same live non-null entry resource, so the eventual mode is read from the
/// observed exit disposition rather than guessed up front.
///
/// The result is deliberately permissive. A probe that forbade a possibly-null
/// return would report an unresolved alias and hide the exit state the proposal
/// has to read; the mode the summary actually claims is derived below and
/// re-checked against a fresh, non-permissive run.
fn probe_contract(
    parameters: &[Option<String>],
    bound: &[bool],
    out: &[bool],
) -> ResourceFunctionContract {
    ResourceFunctionContract::new(
        parameters
            .iter()
            .zip(bound)
            .zip(out)
            .map(|((name, bound), out)| match (name, bound, out) {
                (Some(_), _, true) => ResourceParameterMode::Out { nullable: true },
                (Some(_), true, false) => ResourceParameterMode::Owned { nullable: false },
                _ => ResourceParameterMode::Borrowed,
            }),
        ResourceReturnMode::Owned { nullable: true },
    )
}

/// Read a candidate contract off the probe run's exit observations.
///
/// The ordinary parameter/return shape is tried first, so a body that fits it
/// keeps the summary it had before reallocation was inferable. Only when no
/// single parameter mode and result describe every exit is the reallocation
/// shape considered — which is exactly the case a `Reallocated` result exists
/// to express.
fn propose_contract(
    entry: &EntryCustody,
    observations: &[ExitObservation],
) -> Option<ResourceFunctionContract> {
    ordinary_contract(entry, observations).or_else(|| reallocated_contract(entry, observations))
}

fn ordinary_contract(
    entry: &EntryCustody,
    observations: &[ExitObservation],
) -> Option<ResourceFunctionContract> {
    let mut modes = Vec::with_capacity(entry.parameter_sites.len());
    for index in 0..entry.parameter_sites.len() {
        modes.push(parameter_mode(index, entry, observations)?);
    }
    let result = propose_result(entry, observations)?;
    Some(ResourceFunctionContract::new(modes, result))
}

/// The mode one formal's settled observations read as.
fn parameter_mode(
    index: usize,
    entry: &EntryCustody,
    observations: &[ExitObservation],
) -> Option<ResourceParameterMode> {
    if entry.out_slots[index].is_some() {
        return out_mode(index, observations);
    }
    if entry.parameter_sites[index].is_none() {
        return Some(ResourceParameterMode::Borrowed);
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
    match disposition? {
        // The formal is bound because the body needs it live, and it is
        // still live at every exit: the caller keeps custody.
        SiteDisposition::Live => Some(ResourceParameterMode::Owned { nullable: false }),
        SiteDisposition::Released => Some(ResourceParameterMode::Released { nullable: false }),
        SiteDisposition::Escaped => Some(ResourceParameterMode::Consumed { nullable: false }),
        SiteDisposition::Contained => stored_mode(index, entry, observations),
        SiteDisposition::Absent | SiteDisposition::Mixed => None,
    }
}

/// A formal the body attached to another formal's aggregate.
///
/// The owner is read off the containment the exit states record — the same
/// relation a declared `Stored` contract installs at a call site — and it has
/// to be the *same* caller-held formal at every exit. A resource attached to
/// anything else (a local aggregate, or an owner that differs between paths)
/// is a transfer no mode describes, and yields no summary.
fn stored_mode(
    index: usize,
    entry: &EntryCustody,
    observations: &[ExitObservation],
) -> Option<ResourceParameterMode> {
    let site = entry.parameter_sites[index]?;
    let mut owner_parameter = None;
    for observation in observations {
        let mut owners = observation
            .caller_containment
            .iter()
            .filter(|(_, children)| children.contains(&site))
            .map(|(owner, _)| *owner);
        let owner = owners.next()?;
        if owners.next().is_some() {
            return None;
        }
        let position = entry
            .parameter_sites
            .iter()
            .position(|parameter| *parameter == Some(owner))?;
        match owner_parameter {
            None => owner_parameter = Some(position),
            Some(previous) if previous == position => {}
            Some(_) => return None,
        }
    }
    Some(ResourceParameterMode::Stored {
        owner_parameter: owner_parameter?,
        nullable: false,
    })
}

/// The mode an out slot's settled observations read as.
///
/// `Out` promises the caller a fresh resource in the slot on return, so every
/// reachable exit has to have stored one. A slot left unwritten on some path
/// leaves the caller's variable as it was, and a stored value the callee never
/// resolved to non-null would hand back a resource that may not exist without
/// saying so: both yield no summary. `nullable` is claimed only for the one
/// determinate case — an exit that stores a null constant.
fn out_mode(index: usize, observations: &[ExitObservation]) -> Option<ResourceParameterMode> {
    let mut nullable = false;
    let mut delivered = false;
    for observation in observations {
        match observation.out_slots[index]? {
            OutSlotValue::Site {
                nullable: site_nullable,
                disposition,
                kind,
                ..
            } => {
                // Card W43-DESCRIPTOR-RESOURCE-TRACKING: no non-heap out slot
                // is inferred; the probe's slot is heap-kinded.
                if site_nullable || disposition != SiteDisposition::Live || !kind.is_heap() {
                    return None;
                }
                delivered = true;
            }
            OutSlotValue::Null => nullable = true,
            OutSlotValue::Unwritten | OutSlotValue::Ambiguous => return None,
        }
    }
    delivered.then_some(ResourceParameterMode::Out { nullable })
}

/// How one exit treats the formal a `Reallocated` result would name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReallocationExit {
    /// The replacement is unresolved: the result may be null, and the old
    /// resource is still live pending the caller's null test.
    Pending,
    /// The callee resolved the success case itself: the old resource is gone
    /// and a definitely non-null fresh one is handed back.
    Replaced,
    /// The failure case: nothing is handed back and the caller keeps the old
    /// resource.
    Kept,
}

fn reallocation_exit(
    observation: &ExitObservation,
    index: usize,
    site: usize,
    parameter_sites: &BTreeSet<usize>,
) -> Option<ReallocationExit> {
    let disposition = observation.parameters.get(index).copied().flatten();
    match observation.returned {
        ReturnedValue::Ambiguous | ReturnedValue::Derived { .. } => None,
        ReturnedValue::None | ReturnedValue::Null => (observation.pending_replacements.is_empty()
            && disposition == Some(SiteDisposition::Live))
        .then_some(ReallocationExit::Kept),
        ReturnedValue::Site {
            site: returned,
            nullable,
            disposition: returned_disposition,
            kind,
        } => {
            // Card W43-DESCRIPTOR-RESOURCE-TRACKING: a replacement relation is
            // specified for heap allocations only.
            if parameter_sites.contains(&returned)
                || returned_disposition != SiteDisposition::Escaped
                || !kind.is_heap()
            {
                return None;
            }
            match observation.pending_replacements.get(&returned) {
                Some(replaced)
                    if observation.pending_replacements.len() == 1
                        && replaced.len() == 1
                        && replaced.contains(&site) =>
                {
                    (nullable && disposition == Some(SiteDisposition::Live))
                        .then_some(ReallocationExit::Pending)
                }
                None if observation.pending_replacements.is_empty() => (!nullable
                    && disposition == Some(SiteDisposition::Released))
                .then_some(ReallocationExit::Replaced),
                _ => None,
            }
        }
    }
}

/// The formal a `Reallocated` result would name, if one describes every exit.
///
/// Both outcomes have to be present: some exit hands back a fresh resource,
/// and some exit leaves the old one with the caller. A body that releases its
/// argument on *every* path is not a reallocation — it is a `Released` formal
/// with an owned result, which `ordinary_contract` already derived and which
/// is the summary a caller needs to avoid double-freeing the old pointer.
fn reallocated_parameter(entry: &EntryCustody, observations: &[ExitObservation]) -> Option<usize> {
    let parameter_sites = parameter_sites(entry);
    (0..entry.parameter_sites.len()).find(|&index| {
        let Some(site) = entry.parameter_sites[index] else {
            return false;
        };
        let mut fresh = false;
        let mut kept = false;
        for observation in observations {
            match reallocation_exit(observation, index, site, &parameter_sites) {
                Some(ReallocationExit::Pending) => {
                    fresh = true;
                    kept = true;
                }
                Some(ReallocationExit::Replaced) => fresh = true,
                Some(ReallocationExit::Kept) => kept = true,
                None => return false,
            }
        }
        fresh && kept
    })
}

fn reallocated_contract(
    entry: &EntryCustody,
    observations: &[ExitObservation],
) -> Option<ResourceFunctionContract> {
    let parameter = reallocated_parameter(entry, observations)?;
    let mut modes = Vec::with_capacity(entry.parameter_sites.len());
    for index in 0..entry.parameter_sites.len() {
        modes.push(if index == parameter {
            // The caller must hand over a live non-null resource; what happens
            // to it afterwards is what the result mode says, not this one.
            ResourceParameterMode::Owned { nullable: false }
        } else {
            parameter_mode(index, entry, observations)?
        });
    }
    Some(ResourceFunctionContract::new(
        modes,
        ResourceReturnMode::Reallocated {
            parameter,
            nullable: true,
        },
    ))
}

fn parameter_sites(entry: &EntryCustody) -> BTreeSet<usize> {
    entry.parameter_sites.iter().flatten().copied().collect()
}

fn propose_result(
    entry: &EntryCustody,
    observations: &[ExitObservation],
) -> Option<ResourceReturnMode> {
    let parameter_sites = parameter_sites(entry);
    let mut returns_nothing = false;
    let mut returned = Vec::new();

    for observation in observations {
        match observation.returned {
            ReturnedValue::Ambiguous | ReturnedValue::Derived { .. } => return None,
            ReturnedValue::None | ReturnedValue::Null => {
                if !observation.pending_replacements.is_empty() {
                    return None;
                }
                returns_nothing = true;
            }
            ReturnedValue::Site {
                site,
                nullable,
                disposition,
                kind,
            } => {
                // A formal handed straight back is an alias the summary
                // vocabulary cannot express without double-counting custody.
                if parameter_sites.contains(&site) || disposition != SiteDisposition::Escaped {
                    return None;
                }
                // Card W43-DESCRIPTOR-RESOURCE-TRACKING: no summary is inferred
                // for a non-heap result. A body returning `-1` on one path and
                // a descriptor on another reads as a nullable result only if
                // every non-site return is the literal sentinel, which the
                // exit observation does not record, so the kinded result is
                // declined rather than guessed (`W43-DESCRIPTOR-W4-SUMMARIES`).
                if !kind.is_heap() {
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
    // Card W45-CONTAINER-ROOT-REACH: a body that took custody of an element of
    // a container its caller supplied tells its callers nothing it could say.
    if crate::legacy_c::container_publication::has_foreign_unlink(&summary.cfg) {
        return false;
    }
    // Card CG11-POINTER-VALUED-STORES (rule 10v): a body that overwrote a slot
    // of an untracked object tells no caller its formal was borrowed.
    if crate::pointer_store::has_pointer_store(&summary.cfg) {
        return false;
    }
    // (2) A function with no reachable exit describes no call behavior.
    if observations.is_empty() {
        return false;
    }
    if contract.parameters.len() != entry.parameter_sites.len() {
        return false;
    }
    let parameter_sites = parameter_sites(entry);
    // (4) The containment the contract sanctions, and nothing else.
    let Some(sanctioned) = sanctioned_containment(contract, entry) else {
        return false;
    };
    let reallocated = match contract.result {
        ResourceReturnMode::Reallocated { parameter, .. } => Some(parameter),
        _ => None,
    };
    observations.iter().all(|observation| {
        observation.caller_containment == sanctioned
            && observation.parameters.len() == contract.parameters.len()
            && observation.out_slots.len() == contract.parameters.len()
            && contract
                .parameters
                .iter()
                .copied()
                .enumerate()
                .all(|(index, mode)| {
                    reallocated == Some(index)
                        || parameter_matches(mode, observation.parameters[index])
                })
            && out_slots_match(contract, observation, &parameter_sites)
            && result_matches(contract, entry, observation, &parameter_sites)
    })
}

/// (4) The containment a contract sanctions: exactly one entry per `Stored`
/// formal, attaching that formal's site to its declared owner's site.
///
/// Everything else attached to a site the caller holds refutes the summary,
/// because such a transfer is invisible at the call site. `None` here means
/// the contract itself is unusable — a `Stored` mode naming a formal that
/// carries no site, or a resource stored inside itself.
fn sanctioned_containment(
    contract: &ResourceFunctionContract,
    entry: &EntryCustody,
) -> Option<BTreeMap<usize, BTreeSet<usize>>> {
    let mut containment: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for (index, mode) in contract.parameters.iter().copied().enumerate() {
        let ResourceParameterMode::Stored {
            owner_parameter, ..
        } = mode
        else {
            continue;
        };
        let child = entry.parameter_sites.get(index).copied().flatten()?;
        let owner = entry
            .parameter_sites
            .get(owner_parameter)
            .copied()
            .flatten()?;
        if owner == child {
            return None;
        }
        containment.entry(owner).or_default().insert(child);
    }
    Some(containment)
}

/// (6) Every out slot's disposition at one exit, and the freshness of what the
/// exit hands back.
///
/// Each resource the caller receives — through the result or through a slot —
/// has to be a *distinct* site. Handing the same one back twice, or handing
/// back a formal the caller already owns, would mint a second custody for one
/// resource, which is the failure this whole pass exists to prevent.
fn out_slots_match(
    contract: &ResourceFunctionContract,
    observation: &ExitObservation,
    parameter_sites: &BTreeSet<usize>,
) -> bool {
    let mut claimed = BTreeSet::new();
    if let ReturnedValue::Site { site, .. } = observation.returned {
        claimed.insert(site);
    }
    contract
        .parameters
        .iter()
        .copied()
        .enumerate()
        .all(|(index, mode)| {
            let ResourceParameterMode::Out { nullable } = mode else {
                // A formal the contract does not declare `Out` writes through
                // no slot, so the analysis registered none for it.
                return observation.out_slots[index].is_none();
            };
            match observation.out_slots[index] {
                Some(OutSlotValue::Site {
                    site,
                    nullable: site_nullable,
                    disposition,
                    kind,
                }) => {
                    // Card W43-DESCRIPTOR-RESOURCE-TRACKING: the slot delivers
                    // exactly the declared kind.
                    kind == contract.parameter_kind(index)
                        && disposition == SiteDisposition::Live
                        && !site_nullable
                        && !parameter_sites.contains(&site)
                        && claimed.insert(site)
                }
                // A null store is only consistent with a slot the caller has
                // been told to null-check.
                Some(OutSlotValue::Null) => nullable,
                _ => false,
            }
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
        | ResourceParameterMode::Removed { nullable, .. } => (SiteDisposition::Live, nullable),
        // A stored formal ends up inside the owner's aggregate, not in the
        // caller's direct custody; `sanctioned_containment` checks which
        // aggregate it landed in.
        ResourceParameterMode::Stored { nullable, .. } => (SiteDisposition::Contained, nullable),
        ResourceParameterMode::Consumed { nullable } => (SiteDisposition::Escaped, nullable),
        ResourceParameterMode::Released { nullable } => (SiteDisposition::Released, nullable),
    };
    // A nullable formal carries no custody on the path where it was null.
    observed == expected || (nullable && observed == SiteDisposition::Absent)
}

/// (5) One exit's returned value against the declared result mode.
fn result_matches(
    contract: &ResourceFunctionContract,
    entry: &EntryCustody,
    observation: &ExitObservation,
    parameter_sites: &BTreeSet<usize>,
) -> bool {
    match contract.result {
        ResourceReturnMode::None => {
            matches!(
                observation.returned,
                ReturnedValue::None | ReturnedValue::Null
            ) && observation.pending_replacements.is_empty()
        }
        ResourceReturnMode::Owned { nullable } => match observation.returned {
            // Returning nothing is only consistent with a nullable result.
            ReturnedValue::None | ReturnedValue::Null => {
                nullable && observation.pending_replacements.is_empty()
            }
            ReturnedValue::Ambiguous | ReturnedValue::Derived { .. } => false,
            ReturnedValue::Site {
                site,
                nullable: site_nullable,
                disposition,
                kind,
            } => {
                // Card W43-DESCRIPTOR-RESOURCE-TRACKING: the exit returns
                // exactly the declared kind.
                kind == contract.result_kind
                    && !parameter_sites.contains(&site)
                    && disposition == SiteDisposition::Escaped
                    && (nullable || !site_nullable)
                    && observation.pending_replacements.is_empty()
            }
        },
        // A reallocation makes the argument's custody depend on whether the
        // result was null, so the exit has to exhibit one of the three shapes
        // that dependence can take: an unresolved replacement, the success
        // case the callee resolved itself, or the failure case that leaves the
        // old resource with the caller. A non-nullable `Reallocated` describes
        // no such dependence — it is a released formal with an owned result,
        // and is validated as one — so it is refused here.
        ResourceReturnMode::Reallocated {
            parameter,
            nullable,
        } => match entry.parameter_sites.get(parameter).copied().flatten() {
            Some(site) => {
                nullable
                    && reallocation_exit(observation, parameter, site, parameter_sites).is_some()
            }
            None => false,
        },
        // Card W43-RESULT-ALIAS-RETURNED (W43-LIBC-RESULT-ALIAS answered
        // `false` here). Every exit hands back either a null pointer constant,
        // which only a nullable result permits, or a derived binding over the
        // named formal's name-only site whose relation the contract covers: an
        // exact step is covered by either extent and an interior one only by
        // `Interior`; a step that may be null, or drops `const`, only by a
        // contract that says so. Every other returned value — none, a fresh or
        // formal site, an ambiguous or mixed binding, or a pointer derived from
        // anything else — refutes it. An exit may not leave a reallocation
        // pending either.
        ResourceReturnMode::Derived {
            parameter,
            extent,
            nullable,
            const_dropped,
        } => {
            observation.pending_replacements.is_empty()
                && !observation.derived_base_touched
                && match observation.returned {
                    ReturnedValue::Null => nullable,
                    ReturnedValue::Derived {
                        site,
                        extent: observed_extent,
                        nullable: observed_nullable,
                        const_dropped: observed_dropped,
                    } => {
                        entry.derived_bases.get(&site) == Some(&parameter)
                            && (observed_extent == DerivedExtent::Exact
                                || extent == DerivedExtent::Interior)
                            && (!observed_nullable || nullable)
                            && (!observed_dropped || const_dropped)
                    }
                    ReturnedValue::None | ReturnedValue::Site { .. } | ReturnedValue::Ambiguous => {
                        false
                    }
                }
        }
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

#[path = "legacy_neutral.rs"]
mod neutral;

#[path = "legacy_derived_summary.rs"]
mod derived;

#[path = "legacy_summary_premises.rs"]
mod premises;

// Card CG23-RELATIONAL-POSTCONDITIONS (rule 11r): exit-specific summaries and
// the body check of a declared return-code disposition.
#[path = "legacy_relational_summary.rs"]
mod relational;

// Card CALLEE-RETENTION-CLASS1, stage (a) (rule 12s): relational forwarding
// and the per-class row.
#[path = "legacy_relational_forwarding.rs"]
pub(crate) mod relational_forwarding;

#[cfg(test)]
#[path = "../tests/unit/legacy_summary.rs"]
mod tests;
