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

//! Connecting measurements and contracts (card **QUIRIN-AUDIT-CONNECT**; user
//! decisions of 2026-11-26): the shared, pure part.
//!
//! * [`FORMULAS`]: every condition and refusal of the three parts, each a
//!   formula of the C04.18.9 typed rendering over the **CX block** of
//!   registered terms (`cx_`-prefixed; `qcore_frontend::premise_formula`'s
//!   `RENDERED_TERMS`, block `CX`), checked by the premise-formula validator
//!   `guarded_proposition` (`crates/qcore-frontend/tests/connect_formulas.rs`).
//!   Records name a condition by its id; prose only glosses.
//! * Part B, [`compose_bundles`]: the declarations of two or more contract
//!   bundles composed for one measurement or verdict. A declaration of a
//!   libc/POSIX name by any bundle but `quirin-provider-libc` is refused
//!   (`cx_refuse_libc_redeclared`); several declarations of one function are
//!   decided by the bundles' pin stamps exactly as the loader decides them
//!   ([`crate::contract_supersession::decide`], contract spec C06.6), and an
//!   undecided set is refused (`cx_refuse_conflicting_declarations`). Each
//!   function's verdict is the four-value join of every contributor's
//!   [`ReasonedVerdict`] with every reason kept
//!   ([`ReasonedVerdict::fold`], the semigroup of `QCore.VerdictSemigroup`).
//! * Part C, [`resolve`]: declared cross-crate contract dependencies
//!   (`requires resource_contract_dependency(...)`, contract spec C15.1)
//!   resolved against a loaded environment. An unsatisfied dependency (crate
//!   missing, declaration unloaded, crate not a declared dependency, version
//!   out of range, digest changed, a libc target named under another crate)
//!   or a cycle **holds** the depending declaration, and a held declaration
//!   holds every declaration that depends on it. A hold is `Pending` only
//!   when every cause is human-resolvable (a changed digest, a target held
//!   `Pending`); otherwise `Unknown`. Never `Safe`. The result is the least
//!   fixed point of a monotone operator over the **set** of dependencies, so
//!   it does not depend on their order.
//!
//! The Lean model is `QCore.MeasureConnect`.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::contract_supersession::{Candidate, Decision, decide};
use crate::library_version::LibraryVersion;
use crate::verdict::{PendingReason, ReasonedVerdict, Verdict, VerdictReason};

/// The crate that owns every libc/POSIX declaration.
pub const LIBC_CRATE: &str = "quirin-provider-libc";

/// Every condition and refusal of card QUIRIN-AUDIT-CONNECT, `(id, formula)`:
/// Part A (links, completeness, rows, attribution), Part B (composition),
/// Part C (declared dependencies). Each is the canonical printing of a
/// C04.18.9 typed-rendering formula over the CX block.
pub const FORMULAS: &[(&str, &str)] = &[
    // ---- Part A: links ----
    (
        "cx_joined_is_every_identity",
        "forall (#s: cx_step) { forall (#t: cx_step) { (#cx_joined(#s,#t) => ((#cx_next(#s,#t) && forall (#f: cx_field) { #cx_after_id(#s,#f) == #cx_before_id(#t,#f) }) && forall (#r: cx_route) { ((#cx_compares(#s,#r) && #cx_compares(#t,#r)) => #cx_after_inputs(#s,#r) == #cx_before_inputs(#t,#r)) })) } }",
    ),
    (
        "cx_every_identity_is_joined",
        "forall (#s: cx_step) { forall (#t: cx_step) { ((((#cx_next(#s,#t) && forall (#f: cx_field) { (#cx_after_id(#s,#f) == #cx_before_id(#t,#f) && (#cx_required(#f) => !#cx_after_id(#s,#f) == #cx_unrecorded)) }) && exists (#r: cx_route) { (#cx_compares(#s,#r) && #cx_compares(#t,#r)) }) && forall (#r: cx_route) { ((#cx_compares(#s,#r) && #cx_compares(#t,#r)) => (#cx_after_inputs(#s,#r) == #cx_before_inputs(#t,#r) && !#cx_after_inputs(#s,#r) == #cx_unrecorded)) }) => #cx_joined(#s,#t)) } }",
    ),
    (
        "cx_refuse_unrecorded",
        "forall (#s: cx_step) { forall (#t: cx_step) { forall (#f: cx_field) { ((#cx_required(#f) && (#cx_after_id(#s,#f) == #cx_unrecorded || #cx_before_id(#t,#f) == #cx_unrecorded)) => !#cx_joined(#s,#t)) } } }",
    ),
    (
        "cx_refuse_tool",
        "forall (#s: cx_step) { forall (#t: cx_step) { (!#cx_after_id(#s,#cx_f_tool) == #cx_before_id(#t,#cx_f_tool) => !#cx_joined(#s,#t)) } }",
    ),
    (
        "cx_refuse_revision",
        "forall (#s: cx_step) { forall (#t: cx_step) { (!#cx_after_id(#s,#cx_f_revision) == #cx_before_id(#t,#cx_f_revision) => !#cx_joined(#s,#t)) } }",
    ),
    (
        "cx_refuse_analyzer",
        "forall (#s: cx_step) { forall (#t: cx_step) { (!#cx_after_id(#s,#cx_f_analyzer) == #cx_before_id(#t,#cx_f_analyzer) => !#cx_joined(#s,#t)) } }",
    ),
    (
        "cx_refuse_bundles",
        "forall (#s: cx_step) { forall (#t: cx_step) { (!#cx_after_id(#s,#cx_f_bundles) == #cx_before_id(#t,#cx_f_bundles) => !#cx_joined(#s,#t)) } }",
    ),
    (
        "cx_refuse_build_commit",
        "forall (#s: cx_step) { forall (#t: cx_step) { (!#cx_after_id(#s,#cx_f_build_commit) == #cx_before_id(#t,#cx_f_build_commit) => !#cx_joined(#s,#t)) } }",
    ),
    (
        "cx_refuse_link_resolution",
        "forall (#s: cx_step) { forall (#t: cx_step) { (!#cx_after_id(#s,#cx_f_link_resolution) == #cx_before_id(#t,#cx_f_link_resolution) => !#cx_joined(#s,#t)) } }",
    ),
    (
        "cx_refuse_rules_off",
        "forall (#s: cx_step) { forall (#t: cx_step) { (!#cx_after_id(#s,#cx_f_rules_off) == #cx_before_id(#t,#cx_f_rules_off) => !#cx_joined(#s,#t)) } }",
    ),
    (
        "cx_refuse_dependency_graph",
        "forall (#s: cx_step) { forall (#t: cx_step) { (!#cx_after_id(#s,#cx_f_dependency_graph) == #cx_before_id(#t,#cx_f_dependency_graph) => !#cx_joined(#s,#t)) } }",
    ),
    (
        "cx_refuse_inputs",
        "forall (#s: cx_step) { forall (#t: cx_step) { forall (#r: cx_route) { (((#cx_compares(#s,#r) && #cx_compares(#t,#r)) && !#cx_after_inputs(#s,#r) == #cx_before_inputs(#t,#r)) => !#cx_joined(#s,#t)) } } }",
    ),
    (
        "cx_refuse_no_shared_route",
        "forall (#s: cx_step) { forall (#t: cx_step) { (!exists (#r: cx_route) { (#cx_compares(#s,#r) && #cx_compares(#t,#r)) } => !#cx_joined(#s,#t)) } }",
    ),
    (
        "cx_joined_shares_the_side",
        "forall (#s: cx_step) { forall (#t: cx_step) { (#cx_joined(#s,#t) => #cx_after(#s) == #cx_before(#t)) } }",
    ),
    // ---- Part A: completeness ----
    (
        "cx_complete_is_every_step",
        "forall (#r: cx_route) { (#cx_chain_complete(#r) => forall (#s: cx_step) { (#cx_member(#s) => ((((#cx_compares(#s,#r) && #cx_status_complete(#s,#r)) && !#cx_unmatched(#s,#r)) && (!#cx_unmatched_units(#s,#r) && !#cx_conflicted(#s,#r))) && !#cx_disagrees(#s,#r))) }) }",
    ),
    (
        "cx_incomplete_not_measured",
        "forall (#r: cx_route) { forall (#s: cx_step) { ((#cx_member(#s) && !#cx_compares(#s,#r)) => !#cx_chain_complete(#r)) } }",
    ),
    (
        "cx_incomplete_status",
        "forall (#r: cx_route) { forall (#s: cx_step) { ((#cx_member(#s) && !#cx_status_complete(#s,#r)) => !#cx_chain_complete(#r)) } }",
    ),
    (
        "cx_incomplete_unmatched",
        "forall (#r: cx_route) { forall (#s: cx_step) { ((#cx_member(#s) && #cx_unmatched(#s,#r)) => !#cx_chain_complete(#r)) } }",
    ),
    (
        "cx_incomplete_unmatched_units",
        "forall (#r: cx_route) { forall (#s: cx_step) { ((#cx_member(#s) && #cx_unmatched_units(#s,#r)) => !#cx_chain_complete(#r)) } }",
    ),
    (
        "cx_incomplete_conflicts",
        "forall (#r: cx_route) { forall (#s: cx_step) { ((#cx_member(#s) && #cx_conflicted(#s,#r)) => !#cx_chain_complete(#r)) } }",
    ),
    (
        "cx_incomplete_row_disagrees",
        "forall (#r: cx_route) { forall (#s: cx_step) { ((#cx_member(#s) && #cx_disagrees(#s,#r)) => !#cx_chain_complete(#r)) } }",
    ),
    // ---- Part A: rows ----
    (
        "cx_unlisted_is_unchanged",
        "forall (#s: cx_step) { forall (#w: cx_row) { ((#cx_compares(#s,#cx_row_route(#w)) && !#cx_listed(#s,#w)) => #cx_verdict(#cx_before(#s),#w) == #cx_verdict(#cx_after(#s),#w)) } }",
    ),
    (
        "cx_net_is_first",
        "forall (#w: cx_row) { #cx_net_before(#w) == #cx_verdict(#cx_first,#w) }",
    ),
    (
        "cx_net_is_last",
        "forall (#w: cx_row) { #cx_net_after(#w) == #cx_verdict(#cx_last,#w) }",
    ),
    (
        "cx_moved_is_a_change",
        "forall (#s: cx_step) { forall (#w: cx_row) { (#cx_moved(#s,#w) => !#cx_verdict(#cx_before(#s),#w) == #cx_verdict(#cx_after(#s),#w)) } }",
    ),
    (
        "cx_moved_is_reported",
        "forall (#s: cx_step) { forall (#w: cx_row) { (#cx_moved(#s,#w) => #cx_reported(#w)) } }",
    ),
    (
        "cx_cancelled_is_reported",
        "forall (#w: cx_row) { (#cx_cancelled(#w) => ((#cx_reported(#w) && #cx_moves(#w) >= 2) && #cx_net_before(#w) == #cx_net_after(#w))) }",
    ),
    (
        "cx_cascade_is_reported",
        "forall (#w: cx_row) { (#cx_cascade(#w) => ((#cx_reported(#w) && #cx_moves(#w) >= 2) && !#cx_net_before(#w) == #cx_net_after(#w))) }",
    ),
    (
        "cx_evidence_change_is_reported",
        "forall (#s: cx_step) { forall (#w: cx_row) { (#cx_evidence_changed(#s,#w) => #cx_reported(#w)) } }",
    ),
    (
        "cx_reports_only",
        "forall (#p: cx_point) { forall (#w: cx_row) { #cx_published(#p,#w) == #cx_verdict(#p,#w) } }",
    ),
    // ---- Part A: attribution ----
    (
        "cx_new_safe_is_a_move_to_safe",
        "forall (#s: cx_step) { forall (#w: cx_row) { (#cx_new_safe(#s,#w) => (#cx_moved(#s,#w) && #cx_verdict(#cx_after(#s),#w) == #cx_safe)) } }",
    ),
    (
        "cx_unattributed_move_is_listed",
        "forall (#s: cx_step) { forall (#w: cx_row) { ((#cx_moved(#s,#w) && !#cx_declared_move(#s,#w)) => #cx_unattributed(#s,#w)) } }",
    ),
    (
        "cx_unattributed_new_safe_is_reviewed",
        "forall (#s: cx_step) { forall (#w: cx_row) { ((#cx_new_safe(#s,#w) && !#cx_declared_move(#s,#w)) => #cx_review(#s,#w)) } }",
    ),
    (
        "cx_review_is_never_accepted",
        "forall (#s: cx_step) { forall (#w: cx_row) { (#cx_review(#s,#w) => !#cx_accepted(#s,#w)) } }",
    ),
    (
        "cx_dependency_change_listed",
        "forall (#s: cx_step) { forall (#w: cx_row) { ((#cx_moved(#s,#w) && #cx_dependency_changed(#s,#w)) => #cx_dependency_listed(#s,#w)) } }",
    ),
    // ---- Part B: composition ----
    (
        "cx_refuse_libc_redeclared",
        "forall (#b: cx_bundle) { forall (#g: cx_fn) { (((#cx_declares(#b,#g) && #cx_libc_name(#g)) && !#cx_libc_owner(#b)) => #cx_refused(#b,#g)) } }",
    ),
    (
        "cx_refuse_conflicting_declarations",
        "forall (#b: cx_bundle) { forall (#c: cx_bundle) { forall (#g: cx_fn) { ((((#cx_declares(#b,#g) && #cx_declares(#c,#g)) && !#cx_text(#b,#g) == #cx_text(#c,#g)) && !#cx_stamps_decide(#g)) => (#cx_refused(#b,#g) && #cx_refused(#c,#g))) } } }",
    ),
    (
        "cx_refused_is_unknown",
        "forall (#b: cx_bundle) { forall (#g: cx_fn) { (#cx_refused(#b,#g) => #cx_rank(#cx_unknown) <= #cx_rank(#cx_contribution(#b,#g))) } }",
    ),
    (
        "cx_composed_bounds_every_member",
        "forall (#b: cx_bundle) { forall (#g: cx_fn) { (#cx_declares(#b,#g) => #cx_rank(#cx_contribution(#b,#g)) <= #cx_rank(#cx_composed(#g))) } }",
    ),
    (
        "cx_composed_bounds_the_host",
        "forall (#g: cx_fn) { #cx_rank(#cx_host(#g)) <= #cx_rank(#cx_composed(#g)) }",
    ),
    (
        "cx_composed_is_a_member",
        "forall (#g: cx_fn) { (#cx_composed(#g) == #cx_host(#g) || exists (#b: cx_bundle) { (#cx_declares(#b,#g) && #cx_composed(#g) == #cx_contribution(#b,#g)) }) }",
    ),
    (
        "cx_every_reason_is_kept",
        "forall (#b: cx_bundle) { forall (#g: cx_fn) { (#cx_declares(#b,#g) => #cx_reasons_kept(#b,#g)) } }",
    ),
    (
        "cx_uncertified_plugin_is_pending",
        "forall (#b: cx_bundle) { forall (#g: cx_fn) { (((#cx_plugin(#b) && #cx_proposes_decision(#b,#g)) && !#cx_checked(#b,#g)) => #cx_checker_verdict(#b,#g) == #cx_pending) } }",
    ),
    (
        "cx_checker_bounds_contribution",
        "forall (#b: cx_bundle) { forall (#g: cx_fn) { #cx_rank(#cx_checker_verdict(#b,#g)) <= #cx_rank(#cx_contribution(#b,#g)) } }",
    ),
    (
        "cx_refuse_misclassified_record",
        "forall (#b: cx_bundle) { forall (#g: cx_fn) { (!#cx_recorded_verdict(#b,#g) == #cx_checker_verdict(#b,#g) => #cx_rank(#cx_unknown) <= #cx_rank(#cx_contribution(#b,#g))) } }",
    ),
    // ---- Part C: declared dependencies ----
    (
        "cx_satisfied_is_every_check",
        "forall (#d: cx_dep) { (#cx_satisfied(#d) => ((((#cx_loaded(#d) && #cx_in_range(#d)) && #cx_digest_matches(#d)) && (#cx_declared_crate(#d) && #cx_owner_named(#d))) && !#cx_held(#cx_target_bundle(#d),#cx_target(#d)))) }",
    ),
    (
        "cx_every_check_is_satisfied",
        "forall (#d: cx_dep) { (((((#cx_loaded(#d) && #cx_in_range(#d)) && #cx_digest_matches(#d)) && (#cx_declared_crate(#d) && #cx_owner_named(#d))) && !#cx_held(#cx_target_bundle(#d),#cx_target(#d))) => #cx_satisfied(#d)) }",
    ),
    (
        "cx_unsatisfied_holds_the_dependent",
        "forall (#d: cx_dep) { (!#cx_satisfied(#d) => #cx_held(#cx_dependent_bundle(#d),#cx_dependent(#d))) }",
    ),
    (
        "cx_held_dependency_holds_dependents",
        "forall (#d: cx_dep) { (#cx_held(#cx_target_bundle(#d),#cx_target(#d)) => #cx_held(#cx_dependent_bundle(#d),#cx_dependent(#d))) }",
    ),
    (
        "cx_cycle_holds",
        "forall (#b: cx_bundle) { forall (#g: cx_fn) { (#cx_on_cycle(#b,#g) => #cx_held(#b,#g)) } }",
    ),
    (
        "cx_refuse_libc_target_elsewhere",
        "forall (#d: cx_dep) { ((#cx_libc_name(#cx_target(#d)) && !#cx_libc_owner(#cx_target_bundle(#d))) => !#cx_owner_named(#d)) }",
    ),
    (
        "cx_missing_is_unknown",
        "forall (#d: cx_dep) { (!#cx_loaded(#d) => #cx_rank(#cx_unknown) <= #cx_rank(#cx_hold_verdict(#cx_dependent_bundle(#d),#cx_dependent(#d)))) }",
    ),
    (
        "cx_out_of_range_is_unknown",
        "forall (#d: cx_dep) { (!#cx_in_range(#d) => #cx_rank(#cx_unknown) <= #cx_rank(#cx_hold_verdict(#cx_dependent_bundle(#d),#cx_dependent(#d)))) }",
    ),
    (
        "cx_undeclared_crate_is_unknown",
        "forall (#d: cx_dep) { ((!#cx_declared_crate(#d) || !#cx_owner_named(#d)) => #cx_rank(#cx_unknown) <= #cx_rank(#cx_hold_verdict(#cx_dependent_bundle(#d),#cx_dependent(#d)))) }",
    ),
    (
        "cx_digest_change_is_at_least_pending",
        "forall (#d: cx_dep) { (!#cx_digest_matches(#d) => #cx_rank(#cx_pending) <= #cx_rank(#cx_hold_verdict(#cx_dependent_bundle(#d),#cx_dependent(#d)))) }",
    ),
    (
        "cx_cycle_is_unknown",
        "forall (#b: cx_bundle) { forall (#g: cx_fn) { (#cx_on_cycle(#b,#g) => #cx_rank(#cx_unknown) <= #cx_rank(#cx_hold_verdict(#b,#g))) } }",
    ),
    (
        "cx_hold_is_pending_or_unknown",
        "forall (#b: cx_bundle) { forall (#g: cx_fn) { (#cx_held(#b,#g) => (#cx_hold_verdict(#b,#g) == #cx_pending || #cx_hold_verdict(#b,#g) == #cx_unknown)) } }",
    ),
    (
        "cx_pending_hold_is_human_resolvable",
        "forall (#b: cx_bundle) { forall (#g: cx_fn) { ((#cx_held(#b,#g) && #cx_hold_verdict(#b,#g) == #cx_pending) => #cx_human_resolvable(#b,#g)) } }",
    ),
    (
        "cx_hold_bounds_contribution",
        "forall (#b: cx_bundle) { forall (#g: cx_fn) { (#cx_held(#b,#g) => #cx_rank(#cx_hold_verdict(#b,#g)) <= #cx_rank(#cx_contribution(#b,#g))) } }",
    ),
    (
        "cx_held_is_never_safe",
        "forall (#b: cx_bundle) { forall (#g: cx_fn) { (#cx_held(#b,#g) => !#cx_contribution(#b,#g) == #cx_safe) } }",
    ),
];

/// The formula `(id, text)` of an id.
#[must_use]
pub fn formula(id: &str) -> Option<(&'static str, &'static str)> {
    FORMULAS.iter().copied().find(|(known, _)| *known == id)
}

// ---- Part B: composing contract bundles ----

/// One bundle's declaration of one function, as its lock gives it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleDeclaration {
    /// The bundle (a contract crate's package name).
    pub bundle: String,
    pub function: String,
    /// The module key that declares it (`CRATE:MODULE`).
    pub module: String,
    /// The SHA-256 of the declaration's normalised text (C06.2).
    pub contract_sha256: String,
    /// Its lock entry's pin stamp.
    pub stamp: crate::contract_supersession::StampStanding,
}

/// Why a bundle's declaration takes no part in the composition.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Refusal {
    /// The formula id that refuses it.
    pub formula: &'static str,
    pub detail: String,
}

/// One function's composed declarations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ComposedDeclaration {
    pub function: String,
    /// The declaration in force, when one is.
    pub in_force: Option<BundleDeclaration>,
    /// Every other declaration, kept with why it is not in force.
    pub superseded: Vec<BundleDeclaration>,
    pub refused: Vec<(BundleDeclaration, Refusal)>,
}

impl ComposedDeclaration {
    /// The verdict the declaration standing contributes: `Safe` (the
    /// identity) when one declaration is in force and nothing was refused;
    /// `Unknown` with every refusal's reason otherwise.
    #[must_use]
    pub fn standing(&self) -> ReasonedVerdict {
        let refusals = self.refused.iter().map(|(declaration, refusal)| {
            ReasonedVerdict::unknown(
                crate::reason::ReasonCategory::ContractConflict,
                format!(
                    "{} ({}): {}",
                    declaration.bundle, refusal.formula, refusal.detail
                ),
            )
            .expect("contract-conflict is not human-resolvable")
        });
        let mut standing = ReasonedVerdict::fold(refusals);
        if self.in_force.is_none() && self.refused.is_empty() {
            standing = standing.join(
                ReasonedVerdict::unknown(
                    crate::reason::ReasonCategory::ContractConflict,
                    "no declaration in force",
                )
                .expect("not human-resolvable"),
            );
        }
        standing
    }
}

/// Compose the declarations of several bundles (Part B). `libc_names` is the
/// libc crate's name list (`providers/quirin-provider-libc/libc-names.txt`
/// and every name the libc crate itself declares). A function of the **set**
/// of declarations.
#[must_use]
pub fn compose_bundles(
    declarations: &[BundleDeclaration],
    libc_names: &BTreeSet<String>,
) -> BTreeMap<String, ComposedDeclaration> {
    let mut by_function: BTreeMap<&str, Vec<&BundleDeclaration>> = BTreeMap::new();
    for declaration in declarations {
        by_function
            .entry(declaration.function.as_str())
            .or_default()
            .push(declaration);
    }
    let mut out = BTreeMap::new();
    for (function, mut candidates) in by_function {
        candidates.sort();
        candidates.dedup();
        let mut refused = Vec::new();
        let mut admitted = Vec::new();
        for declaration in candidates {
            if (libc_names.contains(function) || declaration.bundle == LIBC_CRATE)
                && declaration.bundle != LIBC_CRATE
            {
                refused.push((
                    declaration.clone(),
                    Refusal {
                        formula: "cx_refuse_libc_redeclared",
                        detail: format!(
                            "`{function}` is a libc/POSIX name, owned by {LIBC_CRATE}: a \
                             dependency on it names that crate and never re-declares it"
                        ),
                    },
                ));
            } else {
                admitted.push(declaration);
            }
        }
        let mut composed = ComposedDeclaration {
            function: function.to_owned(),
            in_force: None,
            superseded: Vec::new(),
            refused,
        };
        match admitted.as_slice() {
            [] => {}
            [only] => composed.in_force = Some((*only).clone()),
            several => {
                let decision = decide(
                    &several
                        .iter()
                        .map(|declaration| Candidate {
                            module: declaration.module.clone(),
                            contract_sha256: declaration.contract_sha256.clone(),
                            stamp: declaration.stamp.clone(),
                        })
                        .collect::<Vec<_>>(),
                );
                match decision {
                    Decision::InForce(winner) => {
                        for declaration in several {
                            if declaration.module == winner.module
                                && declaration.contract_sha256 == winner.contract_sha256
                                && composed.in_force.is_none()
                            {
                                composed.in_force = Some((*declaration).clone());
                            } else {
                                composed.superseded.push((*declaration).clone());
                            }
                        }
                    }
                    Decision::Conflict(reason) => {
                        for declaration in several {
                            composed.refused.push((
                                (*declaration).clone(),
                                Refusal {
                                    formula: "cx_refuse_conflicting_declarations",
                                    detail: format!(
                                        "{} bundles declare `{function}` and their pin stamps do \
                                         not decide ({}, contract spec C06.6)",
                                        several.len(),
                                        reason.as_str()
                                    ),
                                },
                            ));
                        }
                    }
                }
            }
        }
        out.insert(function.to_owned(), composed);
    }
    out
}

/// A checker plugin's record for one function, as the audit's firewall
/// classified it (`qcore_audit::plugin_verdicts::Answer`): its class and the
/// verdict the record contributes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckerRecord {
    pub bundle: String,
    pub label: String,
    /// The firewall's class (`certified-safe`, `validated-unsafe`,
    /// `uncertified-safe`, `unvalidated-unsafe`, `undecided`, `failure`).
    pub claim: String,
    pub verdict: Verdict,
    pub reasons: Vec<VerdictReason>,
}

/// The verdict a firewall class stands for (rule 10h, contract spec C11.1):
/// a certified `Safe` or validated `Unsafe` only for those classes; an
/// uncertified `Safe` or unvalidated `Unsafe` is `Pending`; anything else is
/// `Unknown`. `None` for an unknown class.
#[must_use]
pub fn class_verdict(claim: &str) -> Option<Verdict> {
    Some(match claim {
        "certified-safe" => Verdict::Safe,
        "validated-unsafe" => Verdict::Unsafe,
        "uncertified-safe" | "unvalidated-unsafe" => Verdict::Pending,
        "undecided" | "failure" => Verdict::Unknown,
        _ => return None,
    })
}

/// One checker record's contribution: its class's verdict when the record
/// states that verdict; `Unknown` (`cx_refuse_misclassified_record`)
/// otherwise. The record's own reasons are kept beside it by
/// [`compose_function`], never folded into the value: a reason is reporting,
/// never evidence (rule 10h).
#[must_use]
pub fn checker_contribution(record: &CheckerRecord) -> ReasonedVerdict {
    let detail = format!("{} ({}): {}", record.label, record.bundle, record.claim);
    match class_verdict(&record.claim) {
        Some(expected) if expected == record.verdict => match expected {
            Verdict::Safe => ReasonedVerdict::safe(),
            Verdict::Pending => {
                ReasonedVerdict::pending(PendingReason::PluginClaimUncertified, detail)
            }
            Verdict::Unknown => {
                ReasonedVerdict::unknown(crate::reason::ReasonCategory::ProviderFailure, detail)
                    .expect("provider failure is not human-resolvable")
            }
            Verdict::Unsafe => ReasonedVerdict::violation(
                record
                    .reasons
                    .first()
                    .map_or_else(|| "validated-unsafe".to_owned(), |reason| reason.code.clone()),
                detail,
            ),
        },
        _ => ReasonedVerdict::unknown(
            crate::reason::ReasonCategory::ProviderFailure,
            format!(
                "{} ({}): the record says {} for class `{}` (cx_refuse_misclassified_record)",
                record.label, record.bundle, record.verdict, record.claim
            ),
        )
        .expect("provider failure is not human-resolvable"),
    }
}

/// One contributor kept in a composed function: who, its verdict with the
/// composition's reasons, and its own reasons as it gave them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Kept {
    pub source: String,
    pub verdict: ReasonedVerdict,
    pub own_reasons: Vec<VerdictReason>,
}

/// A composed function: the join of every contributor, and each one kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ComposedFunction {
    pub verdict: ReasonedVerdict,
    pub contributors: Vec<Kept>,
}

/// The composed verdict of one function (Part B): the join of the host's
/// verdict, the declarations' standing, every checker record and every
/// dependency hold, each contributor kept with its own reasons
/// (`cx_every_reason_is_kept`).
#[must_use]
pub fn compose_function(
    host: &ReasonedVerdict,
    declarations: &ComposedDeclaration,
    records: &[CheckerRecord],
    holds: &[ReasonedVerdict],
) -> ComposedFunction {
    let mut contributors = vec![
        Kept {
            source: "host".into(),
            verdict: host.clone(),
            own_reasons: Vec::new(),
        },
        Kept {
            source: "declarations".into(),
            verdict: declarations.standing(),
            own_reasons: Vec::new(),
        },
    ];
    contributors.extend(records.iter().map(|record| Kept {
        source: format!("{}:{}", record.bundle, record.label),
        verdict: checker_contribution(record),
        own_reasons: record.reasons.clone(),
    }));
    contributors.extend(holds.iter().map(|hold| Kept {
        source: "dependency-hold".into(),
        verdict: hold.clone(),
        own_reasons: Vec::new(),
    }));
    ComposedFunction {
        verdict: ReasonedVerdict::fold(contributors.iter().map(|kept| kept.verdict.clone())),
        contributors,
    }
}

// ---- Part C: declared contract dependencies ----

/// One declared dependency of a declaration on another crate's declaration:
/// `requires resource_contract_dependency("CRATE", NS::NAME[, since "V"]
/// [, removed "W"][, sha256 "HEX"]);` (contract spec C15.1).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclaredDependency {
    /// The crate that owns the depending declaration.
    pub dependent_crate: String,
    /// The depending declaration's function.
    pub dependent: String,
    /// The depending declaration's module (crate-relative).
    pub module: String,
    /// The crate whose declaration it depends on.
    pub target_crate: String,
    /// The namespace the reference was spelled with (`NS` of `NS::NAME`).
    pub target_namespace: String,
    /// The function whose declaration it depends on.
    pub target_function: String,
    /// The inclusive lower bound on the target crate's version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<LibraryVersion>,
    /// The exclusive upper bound on the target crate's version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub removed: Option<LibraryVersion>,
    /// The SHA-256 of the target declaration's normalised text (C06.2), as
    /// its lock records it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

impl DeclaredDependency {
    /// A stable identifier: `DEPENDENT_CRATE:DEPENDENT->TARGET_CRATE:TARGET`.
    #[must_use]
    pub fn id(&self) -> String {
        format!(
            "{}:{}->{}:{}",
            self.dependent_crate, self.dependent, self.target_crate, self.target_function
        )
    }

    /// The written clause (the canonical spelling).
    #[must_use]
    pub fn clause(&self) -> String {
        let mut parts = vec![
            format!("\"{}\"", self.target_crate),
            format!("{}::{}", self.target_namespace, self.target_function),
        ];
        if let Some(since) = self.since {
            parts.push(format!("since \"{since}\""));
        }
        if let Some(removed) = self.removed {
            parts.push(format!("removed \"{removed}\""));
        }
        if let Some(sha256) = &self.sha256 {
            parts.push(format!("sha256 \"{sha256}\""));
        }
        format!(
            "requires resource_contract_dependency({});",
            parts.join(", ")
        )
    }

    /// The static checks a loader applies to the clause alone: an identity
    /// is stated, the bounds are ordered, a digest is 64 lowercase hex
    /// digits, no declaration depends on itself, and a libc/POSIX target
    /// names [`LIBC_CRATE`] (`cx_refuse_libc_target_elsewhere`).
    ///
    /// # Errors
    ///
    /// The first check that fails.
    pub fn validate(&self, libc_names: Option<&BTreeSet<String>>) -> Result<(), String> {
        if self.since.is_none() && self.sha256.is_none() {
            return Err(format!(
                "dependency {} states no identity: give `since \"V\"` (a version range) or \
                 `sha256 \"HEX\"` (the target declaration's digest)",
                self.id()
            ));
        }
        if self.removed.is_some() && self.since.is_none() {
            return Err(format!(
                "dependency {}: `removed` needs `since`",
                self.id()
            ));
        }
        if let (Some(since), Some(removed)) = (self.since, self.removed)
            && removed <= since
        {
            return Err(format!(
                "dependency {}: `removed \"{removed}\"` is not above `since \"{since}\"`",
                self.id()
            ));
        }
        if let Some(sha256) = &self.sha256
            && (sha256.len() != 64
                || !sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
        {
            return Err(format!(
                "dependency {}: `sha256 \"{sha256}\"` is not 64 lowercase hex digits",
                self.id()
            ));
        }
        if self.dependent_crate == self.target_crate && self.dependent == self.target_function {
            return Err(format!("dependency {}: a declaration depends on itself", self.id()));
        }
        let libc_target = self.target_namespace == "libc"
            || libc_names.is_some_and(|names| names.contains(&self.target_function));
        if libc_target && self.target_crate != LIBC_CRATE {
            return Err(format!(
                "dependency {} (cx_refuse_libc_target_elsewhere): `{}` is a libc/POSIX \
                 declaration, owned by {LIBC_CRATE}; name that crate",
                self.id(),
                self.target_function
            ));
        }
        Ok(())
    }
}

/// What the loaded environment says of one crate.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrateFacts {
    pub version: Option<LibraryVersion>,
    /// Its declared (Cargo) dependencies that are load-path crates.
    pub dependencies: BTreeSet<String>,
    /// Its loaded declarations: function to the SHA-256 of the normalised
    /// text and, when the environment already holds it (a pin, a review, a
    /// library version, a supersession conflict), that hold's class.
    pub declarations: BTreeMap<String, DeclarationFacts>,
}

/// One loaded declaration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclarationFacts {
    pub sha256: String,
    /// `Some` when the environment already withholds it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub held: Option<HoldClass>,
}

/// A hold's class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HoldClass {
    /// A person resolves it (review, re-pin).
    Pending,
    /// The tool cannot decide it as configured.
    Unknown,
}

impl HoldClass {
    /// The verdict a hold of this class contributes.
    #[must_use]
    pub const fn verdict(self) -> Verdict {
        match self {
            Self::Pending => Verdict::Pending,
            Self::Unknown => Verdict::Unknown,
        }
    }
}

/// Why a dependency is not satisfied.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "failure")]
pub enum DependencyFailure {
    /// The target crate is not on the load path.
    CrateMissing,
    /// The target crate is not a declared dependency of the dependent's
    /// crate.
    CrateUndeclared,
    /// A libc/POSIX target named under a crate other than
    /// [`LIBC_CRATE`].
    LibcElsewhere,
    /// The target declaration is not loaded.
    Unloaded,
    /// The target crate's version is outside the range.
    OutOfRange { version: Option<String> },
    /// The target declaration's digest is not the declared one.
    DigestChanged { loaded: String },
    /// The target declaration is held.
    TargetHeld { class: HoldClass },
    /// The dependency lies on a cycle.
    Cycle,
}

impl DependencyFailure {
    /// The class of a hold for this failure alone: `Pending` for a changed
    /// digest (a review and re-pin resolve it) and a target held `Pending`;
    /// `Unknown` otherwise.
    #[must_use]
    pub const fn class(&self) -> HoldClass {
        match self {
            Self::DigestChanged { .. }
            | Self::TargetHeld {
                class: HoldClass::Pending,
            } => HoldClass::Pending,
            _ => HoldClass::Unknown,
        }
    }

    /// The formula that names it.
    #[must_use]
    pub const fn formula(&self) -> &'static str {
        match self {
            Self::CrateMissing | Self::Unloaded => "cx_missing_is_unknown",
            Self::CrateUndeclared | Self::LibcElsewhere => "cx_undeclared_crate_is_unknown",
            Self::OutOfRange { .. } => "cx_out_of_range_is_unknown",
            Self::DigestChanged { .. } => "cx_digest_change_is_at_least_pending",
            Self::TargetHeld { .. } => "cx_held_dependency_holds_dependents",
            Self::Cycle => "cx_cycle_is_unknown",
        }
    }

    /// The stable spelling.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CrateMissing => "dependency-crate-missing",
            Self::CrateUndeclared => "dependency-crate-undeclared",
            Self::LibcElsewhere => "dependency-libc-elsewhere",
            Self::Unloaded => "dependency-unloaded",
            Self::OutOfRange { .. } => "dependency-out-of-range",
            Self::DigestChanged { .. } => "dependency-digest-changed",
            Self::TargetHeld { .. } => "dependency-held",
            Self::Cycle => "dependency-cycle",
        }
    }
}

/// How one depending declaration stands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Standing {
    pub crate_name: String,
    pub function: String,
    /// `None` when every dependency is satisfied.
    pub held: Option<HoldClass>,
    /// Every failing dependency, by id, with why.
    pub causes: Vec<(String, DependencyFailure)>,
}

/// The resolution of every declared dependency (Part C).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolution {
    /// Keyed by `(crate, function)` of every depending declaration.
    pub standings: BTreeMap<(String, String), Standing>,
    /// The declarations on a dependency cycle.
    pub cycles: BTreeSet<(String, String)>,
}

impl Resolution {
    /// The held declarations.
    pub fn held(&self) -> impl Iterator<Item = &Standing> {
        self.standings
            .values()
            .filter(|standing| standing.held.is_some())
    }
}

/// The failures of one dependency that do not depend on other holds.
fn local_failures(
    dependency: &DeclaredDependency,
    environment: &BTreeMap<String, CrateFacts>,
    libc_names: Option<&BTreeSet<String>>,
) -> Vec<DependencyFailure> {
    let mut failures = Vec::new();
    let libc_target = dependency.target_namespace == "libc"
        || libc_names.is_some_and(|names| names.contains(&dependency.target_function));
    if libc_target && dependency.target_crate != LIBC_CRATE {
        failures.push(DependencyFailure::LibcElsewhere);
    }
    let declared = environment
        .get(&dependency.dependent_crate)
        .is_some_and(|facts| facts.dependencies.contains(&dependency.target_crate));
    if !declared && dependency.dependent_crate != dependency.target_crate {
        failures.push(DependencyFailure::CrateUndeclared);
    }
    let Some(target) = environment.get(&dependency.target_crate) else {
        failures.push(DependencyFailure::CrateMissing);
        return failures;
    };
    if let Some(since) = dependency.since {
        let admitted = target.version.is_some_and(|version| {
            since <= version && dependency.removed.is_none_or(|removed| version < removed)
        });
        if !admitted {
            failures.push(DependencyFailure::OutOfRange {
                version: target.version.map(|version| version.to_string()),
            });
        }
    }
    match target.declarations.get(&dependency.target_function) {
        None => failures.push(DependencyFailure::Unloaded),
        Some(facts) => {
            if let Some(expected) = &dependency.sha256
                && *expected != facts.sha256
            {
                failures.push(DependencyFailure::DigestChanged {
                    loaded: facts.sha256.clone(),
                });
            }
            if let Some(class) = facts.held {
                failures.push(DependencyFailure::TargetHeld { class });
            }
        }
    }
    failures
}

/// Resolve every declared dependency against the environment (Part C). The
/// result is the least fixed point of: a declaration is held when one of
/// its dependencies fails locally, lies on a cycle, or targets a held
/// declaration; its class is the join (`Pending < Unknown`) of its causes.
/// A function of the **set** of `dependencies`.
#[must_use]
pub fn resolve(
    dependencies: &[DeclaredDependency],
    environment: &BTreeMap<String, CrateFacts>,
    libc_names: Option<&BTreeSet<String>>,
) -> Resolution {
    let mut dependencies: Vec<&DeclaredDependency> = dependencies.iter().collect();
    dependencies.sort();
    dependencies.dedup();
    let node = |crate_name: &str, function: &str| (crate_name.to_owned(), function.to_owned());
    // Edges: dependent -> target.
    let mut edges: BTreeMap<(String, String), BTreeSet<(String, String)>> = BTreeMap::new();
    for dependency in &dependencies {
        edges
            .entry(node(&dependency.dependent_crate, &dependency.dependent))
            .or_default()
            .insert(node(&dependency.target_crate, &dependency.target_function));
    }
    // A node lies on a cycle when it reaches itself.
    let mut cycles = BTreeSet::new();
    for start in edges.keys() {
        let mut seen = BTreeSet::new();
        let mut stack: Vec<&(String, String)> = edges[start].iter().collect();
        while let Some(next) = stack.pop() {
            if next == start {
                cycles.insert(start.clone());
                break;
            }
            if seen.insert(next.clone())
                && let Some(more) = edges.get(next)
            {
                stack.extend(more.iter());
            }
        }
    }
    let mut standings: BTreeMap<(String, String), Standing> = BTreeMap::new();
    for dependency in &dependencies {
        let key = node(&dependency.dependent_crate, &dependency.dependent);
        let standing = standings.entry(key.clone()).or_insert_with(|| Standing {
            crate_name: key.0.clone(),
            function: key.1.clone(),
            held: None,
            causes: Vec::new(),
        });
        for failure in local_failures(dependency, environment, libc_names) {
            standing.causes.push((dependency.id(), failure));
        }
        if cycles.contains(&key) {
            standing.causes.push((dependency.id(), DependencyFailure::Cycle));
        }
    }
    // Propagate holds to the least fixed point.
    loop {
        let mut changed = false;
        for standing in standings.values_mut() {
            let class = standing
                .causes
                .iter()
                .map(|(_, failure)| failure.class())
                .max();
            if class != standing.held {
                standing.held = class;
                changed = true;
            }
        }
        let held: BTreeMap<(String, String), HoldClass> = standings
            .iter()
            .filter_map(|(key, standing)| standing.held.map(|class| (key.clone(), class)))
            .collect();
        for dependency in &dependencies {
            let target = node(&dependency.target_crate, &dependency.target_function);
            let Some(class) = held.get(&target).copied() else {
                continue;
            };
            let key = node(&dependency.dependent_crate, &dependency.dependent);
            let standing = standings.get_mut(&key).expect("every dependent has a standing");
            let failure = DependencyFailure::TargetHeld { class };
            let entry = (dependency.id(), failure);
            if !standing.causes.contains(&entry) {
                // A stronger class of the same target replaces nothing: both
                // are kept, and the class is their join.
                standing.causes.push(entry);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for standing in standings.values_mut() {
        standing.causes.sort();
        standing.causes.dedup();
    }
    Resolution { standings, cycles }
}

/// The verdict a standing's hold contributes, with every cause as a reason:
/// `Pending` (`contract-pin-mismatch`, a review and re-pin resolve it) when
/// every cause is human-resolvable, `Unknown` otherwise; `Safe`, the
/// identity, when nothing is held.
#[must_use]
pub fn hold_verdict(standing: &Standing) -> ReasonedVerdict {
    let detail = |(id, failure): &(String, DependencyFailure)| {
        format!("{id}: {} ({})", failure.as_str(), failure.formula())
    };
    match standing.held {
        None => ReasonedVerdict::safe(),
        Some(HoldClass::Pending) => ReasonedVerdict::fold(standing.causes.iter().map(|cause| {
            ReasonedVerdict::pending(PendingReason::ContractPinMismatch, detail(cause))
        })),
        Some(HoldClass::Unknown) => ReasonedVerdict::fold(standing.causes.iter().map(|cause| {
            match cause.1.class() {
                HoldClass::Pending => {
                    ReasonedVerdict::pending(PendingReason::ContractPinMismatch, detail(cause))
                }
                HoldClass::Unknown => ReasonedVerdict::unknown(
                    crate::reason::ReasonCategory::MissingPremise,
                    detail(cause),
                )
                .expect("missing-premise is not human-resolvable"),
            }
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> LibraryVersion {
        LibraryVersion::parse(text).unwrap()
    }

    fn dependency(dependent: (&str, &str), target: (&str, &str, &str)) -> DeclaredDependency {
        DeclaredDependency {
            dependent_crate: dependent.0.into(),
            dependent: dependent.1.into(),
            module: "m.qv".into(),
            target_crate: target.0.into(),
            target_namespace: target.1.into(),
            target_function: target.2.into(),
            since: Some(v("0.1.0")),
            removed: Some(v("0.2.0")),
            sha256: Some("a".repeat(64)),
        }
    }

    fn environment() -> BTreeMap<String, CrateFacts> {
        let declaration = |sha: &str| DeclarationFacts {
            sha256: sha.into(),
            held: None,
        };
        BTreeMap::from([
            (
                LIBC_CRATE.to_owned(),
                CrateFacts {
                    version: Some(v("0.1.0")),
                    dependencies: BTreeSet::new(),
                    declarations: BTreeMap::from([
                        ("malloc".to_owned(), declaration(&"a".repeat(64))),
                        ("free".to_owned(), declaration(&"a".repeat(64))),
                    ]),
                },
            ),
            (
                "quirin-provider-onigmo".to_owned(),
                CrateFacts {
                    version: Some(v("0.1.0")),
                    dependencies: BTreeSet::from([LIBC_CRATE.to_owned()]),
                    declarations: BTreeMap::from([
                        ("onig_new".to_owned(), declaration(&"b".repeat(64))),
                        ("onig_free".to_owned(), declaration(&"c".repeat(64))),
                    ]),
                },
            ),
        ])
    }

    #[test]
    fn a_satisfied_dependency_holds_nothing() {
        let deps = [dependency(
            ("quirin-provider-onigmo", "onig_new"),
            (LIBC_CRATE, "libc", "malloc"),
        )];
        let resolution = resolve(&deps, &environment(), None);
        assert_eq!(resolution.held().count(), 0);
    }

    #[test]
    fn every_failure_holds_with_its_class() {
        let mut env = environment();
        let base = dependency(
            ("quirin-provider-onigmo", "onig_new"),
            (LIBC_CRATE, "libc", "malloc"),
        );
        // Digest changed: Pending.
        let mut changed = base.clone();
        changed.sha256 = Some("d".repeat(64));
        let r = resolve(&[changed], &env, None);
        assert_eq!(r.held().next().unwrap().held, Some(HoldClass::Pending));
        // Out of range: Unknown.
        let mut range = base.clone();
        range.since = Some(v("0.2.0"));
        range.removed = None;
        let r = resolve(&[range], &env, None);
        assert_eq!(r.held().next().unwrap().held, Some(HoldClass::Unknown));
        // Missing target declaration: Unknown.
        let mut missing = base.clone();
        missing.target_function = "calloc".into();
        let r = resolve(&[missing], &env, None);
        assert!(matches!(
            r.held().next().unwrap().causes[0].1,
            DependencyFailure::Unloaded
        ));
        // Missing crate.
        env.remove(LIBC_CRATE);
        let r = resolve(std::slice::from_ref(&base), &env, None);
        assert!(
            r.held()
                .next()
                .unwrap()
                .causes
                .iter()
                .any(|(_, f)| *f == DependencyFailure::CrateMissing)
        );
    }

    #[test]
    fn a_held_target_holds_its_dependents_transitively() {
        let mut env = environment();
        env.insert(
            "quirin-provider-acme".into(),
            CrateFacts {
                version: Some(v("0.1.0")),
                dependencies: BTreeSet::from(["quirin-provider-onigmo".to_owned()]),
                declarations: BTreeMap::new(),
            },
        );
        let mut first = dependency(
            ("quirin-provider-onigmo", "onig_new"),
            (LIBC_CRATE, "libc", "malloc"),
        );
        first.sha256 = Some("e".repeat(64));
        let mut second = dependency(
            ("quirin-provider-acme", "acme_open"),
            ("quirin-provider-onigmo", "onigmo", "onig_new"),
        );
        second.sha256 = Some("b".repeat(64));
        let forward = resolve(&[first.clone(), second.clone()], &env, None);
        let backward = resolve(&[second, first], &env, None);
        assert_eq!(forward, backward, "order independent");
        let acme = &forward.standings[&("quirin-provider-acme".into(), "acme_open".into())];
        assert_eq!(acme.held, Some(HoldClass::Pending));
        assert!(
            acme.causes
                .iter()
                .any(|(_, f)| matches!(f, DependencyFailure::TargetHeld { .. }))
        );
    }

    #[test]
    fn a_cycle_is_held_unknown() {
        let env = environment();
        let mut a = dependency(
            ("quirin-provider-onigmo", "onig_new"),
            ("quirin-provider-onigmo", "onigmo", "onig_free"),
        );
        a.sha256 = Some("c".repeat(64));
        let mut b = dependency(
            ("quirin-provider-onigmo", "onig_free"),
            ("quirin-provider-onigmo", "onigmo", "onig_new"),
        );
        b.sha256 = Some("b".repeat(64));
        let r = resolve(&[a, b], &env, None);
        assert_eq!(r.cycles.len(), 2);
        assert!(r.held().all(|s| s.held == Some(HoldClass::Unknown)));
    }

    #[test]
    fn a_libc_target_names_the_libc_crate() {
        let wrong = dependency(
            ("quirin-provider-onigmo", "onig_new"),
            ("quirin-provider-onigmo", "libc", "malloc"),
        );
        assert!(wrong.validate(None).unwrap_err().contains("cx_refuse_libc"));
        let names = BTreeSet::from(["malloc".to_owned()]);
        let mut spelled = wrong.clone();
        spelled.target_namespace = "onigmo".into();
        assert!(spelled.validate(Some(&names)).is_err());
        let r = resolve(&[wrong], &environment(), None);
        assert!(
            r.held()
                .next()
                .unwrap()
                .causes
                .iter()
                .any(|(_, f)| *f == DependencyFailure::LibcElsewhere)
        );
    }

    #[test]
    fn the_static_checks_refuse_malformed_clauses() {
        let mut d = dependency(
            ("quirin-provider-onigmo", "onig_new"),
            (LIBC_CRATE, "libc", "malloc"),
        );
        assert!(d.validate(None).is_ok());
        assert_eq!(
            d.clause(),
            format!(
                "requires resource_contract_dependency(\"{LIBC_CRATE}\", libc::malloc, since \
                 \"0.1.0\", removed \"0.2.0\", sha256 \"{}\");",
                "a".repeat(64)
            )
        );
        d.since = None;
        assert!(d.validate(None).is_err(), "removed without since");
        d.removed = None;
        d.sha256 = None;
        assert!(d.validate(None).is_err(), "no identity");
        d.sha256 = Some("A".repeat(64));
        assert!(d.validate(None).is_err(), "uppercase digest");
    }

    fn declaration(bundle: &str, function: &str, sha: &str, stamp: &str) -> BundleDeclaration {
        BundleDeclaration {
            bundle: bundle.into(),
            function: function.into(),
            module: format!("{bundle}:m.qv"),
            contract_sha256: sha.repeat(64),
            stamp: crate::contract_supersession::StampStanding::of_entry(Some(stamp)),
        }
    }

    #[test]
    fn bundles_compose_agree_disagree_and_refuse_libc() {
        let names = BTreeSet::from(["malloc".to_owned()]);
        let agreeing = [
            declaration("quirin-provider-a", "f", "1", "2026-11-01T00:00:00Z"),
            declaration("quirin-provider-b", "f", "1", "2026-11-01T00:00:00Z"),
        ];
        let composed = compose_bundles(&agreeing, &names);
        assert!(composed["f"].in_force.is_some());
        assert!(composed["f"].refused.is_empty());
        assert_eq!(composed["f"].standing().verdict(), Verdict::Safe);
        let disagreeing = [
            declaration("quirin-provider-a", "f", "1", "2026-11-01T00:00:00Z"),
            declaration("quirin-provider-b", "f", "2", "2026-11-01T00:00:00Z"),
        ];
        let composed = compose_bundles(&disagreeing, &names);
        assert!(composed["f"].in_force.is_none());
        assert_eq!(composed["f"].refused.len(), 2);
        assert_eq!(composed["f"].standing().verdict(), Verdict::Unknown);
        let libc = [
            declaration(LIBC_CRATE, "malloc", "1", "2026-11-01T00:00:00Z"),
            declaration("quirin-provider-a", "malloc", "1", "2026-11-02T00:00:00Z"),
        ];
        let composed = compose_bundles(&libc, &names);
        assert_eq!(
            composed["malloc"].in_force.as_ref().unwrap().bundle,
            LIBC_CRATE
        );
        assert_eq!(
            composed["malloc"].refused[0].1.formula,
            "cx_refuse_libc_redeclared"
        );
    }

    #[test]
    fn a_plugin_record_keeps_its_class_and_reasons() {
        let record = CheckerRecord {
            bundle: "quirin-provider-onigmo".into(),
            label: "onigmo-check".into(),
            claim: "uncertified-safe".into(),
            verdict: Verdict::Pending,
            reasons: vec![VerdictReason {
                code: "plugin-reason".into(),
                detail: "the region is freed".into(),
            }],
        };
        assert_eq!(checker_contribution(&record).verdict(), Verdict::Pending);
        let mut lying = record.clone();
        lying.verdict = Verdict::Safe;
        assert_eq!(checker_contribution(&lying).verdict(), Verdict::Unknown);
        let declarations = compose_bundles(
            &[declaration(
                "quirin-provider-onigmo",
                "f",
                "1",
                "2026-11-01T00:00:00Z",
            )],
            &BTreeSet::new(),
        );
        let composed = compose_function(
            &ReasonedVerdict::safe(),
            &declarations["f"],
            std::slice::from_ref(&record),
            &[],
        );
        assert_eq!(composed.verdict.verdict(), Verdict::Pending);
        assert!(composed.contributors.iter().any(|kept| {
            kept.own_reasons
                .iter()
                .any(|reason| reason.detail == "the region is freed")
        }));
        // The join never lowers the host.
        let host =
            ReasonedVerdict::unknown(crate::reason::ReasonCategory::UnsupportedSyntax, "host")
                .unwrap();
        let composed = compose_function(&host, &declarations["f"], &[record], &[]);
        assert_eq!(composed.verdict.verdict(), Verdict::Unknown);
    }

    #[test]
    fn every_formula_id_is_unique_and_prefixed() {
        let mut ids: Vec<&str> = FORMULAS.iter().map(|(id, _)| *id).collect();
        assert!(ids.iter().all(|id| id.starts_with("cx_")));
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), FORMULAS.len());
        for formula_id in [
            "cx_refuse_libc_redeclared",
            "cx_refuse_conflicting_declarations",
            "cx_refuse_misclassified_record",
            "cx_missing_is_unknown",
            "cx_undeclared_crate_is_unknown",
            "cx_out_of_range_is_unknown",
            "cx_digest_change_is_at_least_pending",
            "cx_held_dependency_holds_dependents",
            "cx_cycle_is_unknown",
            "cx_refuse_libc_target_elsewhere",
        ] {
            assert!(formula(formula_id).is_some(), "{formula_id}");
        }
    }
}
