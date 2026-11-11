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

//! Card **W43-RESULT-ALIAS-RETURNED**: inferred, validated derived-result
//! summaries.
//!
//! # The shape
//!
//! Card W43-LIBC-RESULT-ALIAS gave the vocabulary a derived result
//! (`resource_return_alias(p, n)`, `resource_return_interior(p, n)`, loading to
//! [`ResourceReturnMode::Derived`]) and refused every escape of a derived
//! pointer, so a wrapper such as
//!
//! ```c
//! static void *XXH_memcpy(void *dest, const void *src, size_t size) { return memcpy(dest, src, size); }
//! char *mk_string_casestr(char *heystack, char *needle) {
//!     if (!heystack || !needle) { return NULL; }
//!     return strcasestr(heystack, needle);
//! }
//! ```
//!
//! stayed `Unknown`, and so did every call to it. This route infers the summary
//! a declaration would have stated — "the result is derived from formal `k`"
//! — and publishes it only after the ordinary validation gate
//! ([`super::observations_match_contract`]) has re-checked it against a fresh
//! analysis of the body. Callers then apply it through the unchanged
//! W43-LIBC-RESULT-ALIAS call rules, so `use_after_release` and
//! `invalid_release` reach a wrapper's result exactly as they reach `strchr`'s.
//!
//! # How a formal is tracked without being owned
//!
//! A borrowed formal is not bound in the ordinary entry state, so a pointer
//! derived from it has no target and nothing says which formal it came from.
//! [`EntryCustody::seed_derived_base`] binds the formal to a **name-only**
//! site: a target, but not a resource. It has no status, so
//!
//! * every custody-bearing use of the formal — release, consumption, escape,
//!   an `Owned` position, an aggregate owner, a store, a bound read — fails
//!   the live-identity check the transfer already applies and refuses, which is
//!   what makes the summary's `Borrowed` claim for it true;
//! * nothing is exempted from leak reporting, because nothing is owned;
//! * the binding may be null, so a null test keeps both edges and no path is
//!   deleted by the seeding (the soundness requirement the ordinary search
//!   states for binding a formal at all).
//!
//! A pointer derived from the formal shares that site, and returning it is the
//! one escape the transfer admits under such an entry: it escapes nothing.
//! `observe_exit` reports it as `ReturnedValue::Derived`.
//!
//! # The rule
//!
//! A summary `Derived { parameter: k, extent, nullable, const_dropped }` is
//! proposed only when, on the CFG that desugars a derived `return f(…)`
//! ([`build_derived_return_cfg`]):
//!
//! 1. the body is fully modeled and defect-free under the entry that seeds `k`
//!    alone (condition (1) of the ordinary gate), after the ordinary
//!    least-binding search has bound every other formal the body owns;
//! 2. the search never needs to bind `k` itself, and no exit sees a custody
//!    effect on `k`'s name-only site (it would have acquired a status: a
//!    nullable release, an escape, a cascade through an aggregate that held
//!    it). A body that uses `k` at a custody-bearing position — it releases,
//!    consumes, stores, owns or returns it — is refused, since the result
//!    would then be derived from a formal that is not borrowed;
//! 3. every reachable exit returns either a null pointer constant or a derived
//!    binding over `k`'s site, and at least one returns the latter. Any other
//!    exit — a fresh resource, an untracked or opaque pointer, no pointer, an
//!    ambiguous or mixed binding, a pointer derived from something else — is a
//!    mixed exit and refuses;
//! 4. `extent` is `Exact` only when every derived exit is exact; `nullable` when
//!    some exit returns NULL or some derivation step may; `const_dropped` when
//!    some step dropped it;
//! 5. the fresh validation run of the proposed contract passes the ordinary
//!    gate, including `result_matches`' derived arm.
//!
//! The ordinary live-entry search and the removed-wrapper route run first, so
//! no non-neutral result either publishes is replaced. Rule 36 may refine a
//! neutral None result with a validated alias and identical parameter modes.
//!
//! # What is deliberately not inferred
//!
//! Rule 36 additionally admits a source-confirmed original borrowed formal
//! through `ExactFormalReturn`; all other identity escapes still refuse.
//!
//! * a rebound, untyped, shadowed or custody-bearing formal;
//! * derivation from anything but a formal: a local resource, a global, a
//!   string literal, an untracked expression;
//! * derivation through a local copy the body rebinds, or offsets by more than
//!   the one token `derived_base` reads: the copy's binding is lost or refused,
//!   and so is the derivation;
//! * derivation from different formals on different exits.
//!
//! # Lean
//!
//! `QCore.DerivedResultInference` models the proposal, the gate's derived arm,
//! the name-only seeding and the caller's application, and proves the gate
//! sound against the contract semantics of `QCore.DerivedResult`. As with every
//! gateway module, the Rust–Lean correspondence is maintained by review
//! (trust boundary 28).

use std::collections::BTreeSet;

use qcore_schemas::contract::{
    DerivedExtent, ResourceContracts, ResourceFunctionContract, ResourceParameterMode,
    ResourceReturnMode,
};

use crate::legacy_c::{
    AnalysisStatus, EntryCustody, ExitObservation, LegacyCfg, LegacyInstruction, ReturnedValue,
    Token, analyze_cfg_with_entry, body_calls_derived_result, build_derived_return_cfg,
    first_free_site, function_parameters,
};

use super::{
    bind_unresolved_formals, observations_match_contract, out_parameter_candidates, parameter_mode,
    probe_contract,
};

/// Why the derived-result route declined a candidate. Every arm leaves the
/// function without a summary, so its calls stay uncontracted: `Unknown`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DerivedRefusal {
    /// The body calls no callee with a derived result, or has no `return`.
    /// Not a candidate; never counted.
    NotCandidate,
    /// The body calls a derived-result callee and returns, but no exit returns
    /// the destination of such a call. Not a candidate either; counted only to
    /// size the population the route looked at.
    NoDerivedExit,
    /// No exit returns a pointer derived from a formal.
    NoFormalBase,
    /// Different exits return pointers derived from different formals.
    SeveralFormals,
    /// The body uses the base formal at a custody-bearing position.
    BaseCustody,
    /// Some exit returns something other than NULL or a pointer derived from
    /// the base formal.
    MixedExits,
    /// The body is not fully modeled or not defect-free under the entry.
    BodyNotModeled,
    /// The proposed contract failed its fresh validation run.
    ValidationFailed,
}

impl DerivedRefusal {
    /// The census spelling of the refused shape.
    #[cfg_attr(not(feature = "indirect-census"), allow(dead_code))]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::NotCandidate => "not_candidate",
            Self::NoDerivedExit => "no_derived_exit",
            Self::NoFormalBase => "no_formal_base",
            Self::SeveralFormals => "several_formals",
            Self::BaseCustody => "base_custody",
            Self::MixedExits => "mixed_exits",
            Self::BodyNotModeled => "body_not_modeled",
            Self::ValidationFailed => "validation_failed",
        }
    }
}

/// Infer and validate a derived-result summary for `function`, or say why not.
pub(crate) fn infer_derived_result_summary(
    tokens: &[Token],
    function: &str,
    environment: &ResourceContracts,
) -> Result<ResourceFunctionContract, DerivedRefusal> {
    if !body_calls_derived_result(tokens, function, environment) {
        return Err(DerivedRefusal::NotCandidate);
    }
    let parameters = function_parameters(tokens, function).ok_or(DerivedRefusal::NotCandidate)?;
    let cfg = build_derived_return_cfg(tokens, function, environment)
        .map_err(|_| DerivedRefusal::NotCandidate)?;
    if !returns_derived_destination(&cfg) {
        return Err(DerivedRefusal::NoDerivedExit);
    }
    let out = out_parameter_candidates(&cfg, &parameters);
    let base = select_base(&cfg, &parameters, &out)?;
    let candidate = infer_for_base(&cfg, &parameters, &out, base)?;
    // Card W47-SUMMARY-VALIDATION-PROTECTED-FORMALS (rule 4b).
    if super::premises::validates_under_reported_premises(
        tokens,
        function,
        environment,
        &parameters,
        &candidate,
        super::premises::Route::DerivedResult,
        &cfg,
    ) {
        Ok(candidate)
    } else {
        Err(DerivedRefusal::ValidationFailed)
    }
}

/// Does some exit escape a place a derived-result call assigned? Structural:
/// it reads the CFG, not an analysis.
fn returns_derived_destination(cfg: &LegacyCfg) -> bool {
    let destinations = cfg
        .nodes
        .iter()
        .filter_map(|node| match &node.instruction {
            LegacyInstruction::ContractCall {
                destination: Some(destination),
                contract,
                ..
            } if matches!(contract.result, ResourceReturnMode::Derived { .. }) => {
                Some(destination.as_str())
            }
            LegacyInstruction::IndirectScalarAccess {
                destination: Some(destination),
                ..
            } if node.lookup_read.is_some() => Some(destination.as_str()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    cfg.nodes.iter().any(|node| {
        node.is_exit
            && (matches!(node.instruction, LegacyInstruction::ExactFormalReturn { .. }) || matches!(
                &node.instruction,
                LegacyInstruction::EscapeIfOwned { source } if destinations.contains(source.as_str())
            ))
    })
}

/// Which formal the returned pointers are derived from.
///
/// One selection run seeds **every** named formal that is not an out slot with
/// its own name-only site and reads which of them the derived exits target.
/// This only chooses the candidate: the run may carry refusals, and nothing it
/// observes is published. [`infer_for_base`] re-runs everything with the one
/// chosen formal seeded.
fn select_base(
    cfg: &LegacyCfg,
    parameters: &[Option<String>],
    out: &[bool],
) -> Result<usize, DerivedRefusal> {
    let borrowed = ResourceFunctionContract::new(
        vec![ResourceParameterMode::Borrowed; parameters.len()],
        ResourceReturnMode::None,
    );
    let mut entry =
        EntryCustody::build(cfg, parameters, &borrowed).ok_or(DerivedRefusal::NotCandidate)?;
    let first = first_free_site(cfg);
    for (index, name) in parameters.iter().enumerate() {
        let Some(name) = name else { continue };
        if out[index] {
            continue;
        }
        let site = first
            .checked_add(index)
            .ok_or(DerivedRefusal::NotCandidate)?;
        entry.seed_derived_base(index, name, site);
    }
    let (_, observations) = analyze_cfg_with_entry(cfg.clone(), &entry);
    let formals = observations
        .iter()
        .filter_map(|observation| match observation.returned {
            ReturnedValue::Derived { site, .. } => entry.derived_bases.get(&site).copied(),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut formals = formals.into_iter();
    match (formals.next(), formals.next()) {
        (None, _) => Err(DerivedRefusal::NoFormalBase),
        (Some(_), Some(_)) => Err(DerivedRefusal::SeveralFormals),
        (Some(base), None) => Ok(base),
    }
}

/// The ordinary least-binding search, with `base` seeded name-only and its
/// result read as a derived one, then the ordinary validation gate.
fn infer_for_base(
    cfg: &LegacyCfg,
    parameters: &[Option<String>],
    out: &[bool],
    base: usize,
) -> Result<ResourceFunctionContract, DerivedRefusal> {
    let mut bound = vec![false; parameters.len()];
    for _ in 0..=parameters.len() {
        // (2) The body treats the base as a resource it owns.
        if bound[base] {
            return Err(DerivedRefusal::BaseCustody);
        }
        if bound.iter().zip(out).any(|(bound, out)| *bound && *out) {
            return Err(DerivedRefusal::BodyNotModeled);
        }
        let mut probe = probe_contract(parameters, &bound, out);
        // A permissive derived result: every relation the proposal can read is
        // admitted by the probe, and the proposal is re-checked below.
        probe.result = ResourceReturnMode::Derived {
            parameter: base,
            extent: DerivedExtent::Interior,
            nullable: true,
            const_dropped: true,
        };
        let entry =
            EntryCustody::build(cfg, parameters, &probe).ok_or(DerivedRefusal::BodyNotModeled)?;
        let (summary, observations) = analyze_cfg_with_entry(cfg.clone(), &entry);
        if bind_unresolved_formals(&mut bound, parameters, &summary) {
            continue;
        }
        // (2) A custody effect reached the base's name-only site without a
        // diagnostic (a nullable release, a cascade through an aggregate that
        // held it): the base is not borrowed.
        if observations
            .iter()
            .any(|observation| observation.derived_base_touched)
        {
            return Err(DerivedRefusal::BaseCustody);
        }
        if summary.status != AnalysisStatus::Safe || !summary.diagnostics.is_empty() {
            return Err(if has_mixed_exit(&entry, &observations, base) {
                DerivedRefusal::MixedExits
            } else {
                DerivedRefusal::BodyNotModeled
            });
        }
        let candidate = propose(&entry, &observations, base)?;
        let entry = EntryCustody::build(cfg, parameters, &candidate)
            .ok_or(DerivedRefusal::ValidationFailed)?;
        let (summary, observations) = analyze_cfg_with_entry(cfg.clone(), &entry);
        return if observations_match_contract(&candidate, &entry, &summary, &observations) {
            Ok(candidate)
        } else {
            Err(DerivedRefusal::ValidationFailed)
        };
    }
    Err(DerivedRefusal::BodyNotModeled)
}

/// Whether some exit returns something other than NULL or a pointer derived
/// from the base formal's site.
fn has_mixed_exit(entry: &EntryCustody, observations: &[ExitObservation], base: usize) -> bool {
    observations
        .iter()
        .any(|observation| match observation.returned {
            ReturnedValue::Null => false,
            ReturnedValue::Derived { site, .. } => entry.derived_bases.get(&site) != Some(&base),
            ReturnedValue::None | ReturnedValue::Site { .. } | ReturnedValue::Ambiguous => true,
        })
}

/// (3)–(4) Read the contract off a clean run's exits.
fn propose(
    entry: &EntryCustody,
    observations: &[ExitObservation],
    base: usize,
) -> Result<ResourceFunctionContract, DerivedRefusal> {
    let mut extent = DerivedExtent::Exact;
    let mut nullable = false;
    let mut const_dropped = false;
    let mut derived = false;
    for observation in observations {
        match observation.returned {
            ReturnedValue::Null => nullable = true,
            ReturnedValue::Derived {
                site,
                extent: observed_extent,
                nullable: observed_nullable,
                const_dropped: observed_dropped,
            } if entry.derived_bases.get(&site) == Some(&base) => {
                derived = true;
                // Card SLOT-MOVE-AND-INTERIOR-BORROW (rule 12y): one borrowed
                // exit makes the summary borrowed, never a plain interior
                // result that would lose the borrow's ending rules.
                if observed_extent == DerivedExtent::Borrowed {
                    extent = DerivedExtent::Borrowed;
                } else if observed_extent == DerivedExtent::Interior
                    && extent != DerivedExtent::Borrowed
                {
                    extent = DerivedExtent::Interior;
                }
                nullable |= observed_nullable;
                const_dropped |= observed_dropped;
            }
            _ => return Err(DerivedRefusal::MixedExits),
        }
    }
    if !derived {
        return Err(DerivedRefusal::NoFormalBase);
    }
    let mut modes = Vec::with_capacity(entry.parameter_sites.len());
    for index in 0..entry.parameter_sites.len() {
        modes.push(if index == base {
            ResourceParameterMode::Borrowed
        } else {
            parameter_mode(index, entry, observations).ok_or(DerivedRefusal::BodyNotModeled)?
        });
    }
    Ok(ResourceFunctionContract::new(
        modes,
        ResourceReturnMode::Derived {
            parameter: base,
            extent,
            nullable,
            const_dropped,
        },
    ))
}

#[cfg(test)]
#[path = "../tests/unit/legacy_derived_summary.rs"]
mod tests;
