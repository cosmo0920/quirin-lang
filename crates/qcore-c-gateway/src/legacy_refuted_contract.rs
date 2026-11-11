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

//! Card **W45-REFUTED-CONTRACT-WITHDRAWAL**: a declared contract its own body
//! refutes serves no caller (contract spec §C06.4, gateway constraint 10g).
//!
//! # The gap
//!
//! A declared contract governs every call site, and the gates that check a
//! declaration against the body it names are local. The mode gate (constraint
//! 8) withdraws a declaration whose custody *duty* the body's validated
//! summary contradicts, but only in the unit that defines the body; the
//! counted-user pass (constraint 10e) refutes a clause on the defining row and
//! keeps its *counted* half out of that unit's callers, while the custody half
//! still applies there, and in every other unit both halves apply. So a
//! declaration the audit has refuted went on discharging its callers' custody.
//!
//! # The rule
//!
//! A declared contract of a function whose body a unit analyses is
//! **refuted** by that body when
//!
//! * a counted-user clause of the function's own row — declared, or the
//!   implicit obligation of every uncovered exit (§C04.7 rules 2 and 4) — is
//!   refuted, or
//! * the validated W4 summary of the body contradicts a declared custody duty
//!   ([`super::declared_modes::mode_contradiction`]).
//!
//! A `SummaryConflict` that is a refinement inside one duty (nullability, an
//! owner index, a replaced parameter) is **not** a refutation: the inference
//! chose a narrower reading the body also satisfies, and the measured case
//! (`rd_kafka_mem_free`, `Released{nullable}` declared, `Released{nonnull}`
//! derived) is a faithful declaration (constraint 8). An undecided clause, a
//! body that does not validate, or no body at all is **unchecked**, never
//! refuted: absence of evidence is not evidence.
//!
//! A refuted declaration is withdrawn from every caller and stays in force for
//! its own body, so the defining row keeps the refutation that convicts it:
//!
//! * **in the defining unit**, from this unit's own decision ([`settle`]);
//! * **in every unit**, from the audit's refutation table
//!   (`ResourceContracts::refuted_by_body`), computed by a first pass over
//!   every unit and applied by a second.
//!
//! Every call site outside the function's own body is rewritten to a marked,
//! undeclared name, exactly as a pin mismatch or a pending review is held
//! (the LOADER-ABI technique), so W4 derivation, W4 validation and a
//! certificate recheck reproduce the hold. The statement refuses with
//! [`hold_description`], whose reason key is [`CONTRACT_REFUTED_BY_BODY`], and
//! is `Unknown`: a refutation is the tool's decision, not a pending human act,
//! so it is never `Pending`. A name the mode gate already withdrew keeps
//! constraint 8's own key in its unit.
//!
//! Monotone in the refusing direction: a held call is an unmodelled call, so
//! no verdict moves towards `Safe`. The Lean model is
//! `QCore.RefutedContractWithdrawal`.

use std::collections::BTreeSet;

use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::contract_refutation::RefutationKind;

use super::checked_spec::{self, Settled};
use super::declared_abi::is_call_position;
use super::{
    LegacyDiagnosticKind, LegacyFunctionSummary, Token, counted_users, declared_modes,
    find_function_body, matching,
};

/// The stable reason key of a held call. Its closed reason category is
/// `stale-evidence`, as constraint 8's mode withdrawal's is: the declaration
/// is evidence about a body other than the one analysed.
pub const CONTRACT_REFUTED_BY_BODY: &str = "contract-refuted-by-body";

/// The prefix a held callee's call sites are rewritten with, reserved for this
/// rewrite. A source that spelled it itself reads as a held call, which is
/// `Unknown`: the collision can only refuse.
pub(crate) const HELD_PREFIX: &str = "qcore_refuted_by_body__";

/// The original callee a rewritten name stands for.
pub(crate) fn held_callee(function: &str) -> Option<&str> {
    function.strip_prefix(HELD_PREFIX)
}

/// The refusal a held call site carries.
#[must_use]
pub fn hold_description(callee: &str) -> String {
    format!(
        "call to `{callee}` whose declared contract its own analysed body refutes \
         ({CONTRACT_REFUTED_BY_BODY})"
    )
}

/// The callee a held call site's refusal names, if `description` is one.
#[must_use]
pub fn held_call(description: &str) -> Option<&str> {
    description
        .strip_prefix("call to `")?
        .strip_suffix(&format!(
            "` whose declared contract its own analysed body refutes ({CONTRACT_REFUTED_BY_BODY})"
        ))
}

/// One refutation a function's own row carries, before the audit names its
/// unit.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BodyRefutation {
    pub kind: RefutationKind,
    /// The refuted clause, or the contradicted duty's refusal key.
    pub clause: String,
    /// Input line of the refuting exit, or 0 for the function itself.
    pub line: usize,
    pub reason: String,
}

/// The refutations of `summary`'s declaration by its own body, read from what
/// the row already records: its own refuted counted-user clauses and a
/// `SummaryConflict` that contradicts a custody duty. Empty for a function
/// with no declaration in `contracts`, and for every refinement.
#[must_use]
pub fn refutations(
    summary: &LegacyFunctionSummary,
    contracts: &ResourceContracts,
) -> Vec<BodyRefutation> {
    if contracts.contract(&summary.function).is_none() {
        return Vec::new();
    }
    let mut found = summary
        .counted_users
        .iter()
        .filter(|record| record.verdict == counted_users::ClauseVerdict::Refuted)
        .filter(|record| !record.clause.starts_with("callee `"))
        .map(|record| BodyRefutation {
            kind: RefutationKind::CountedUserClause,
            clause: record.clause.clone(),
            line: record.line,
            reason: record.reason.clone(),
        })
        .collect::<Vec<_>>();
    for diagnostic in &summary.diagnostics {
        if let LegacyDiagnosticKind::SummaryConflict { inferred, declared } = &diagnostic.kind
            && let Some(refusal) = declared_modes::mode_contradiction(declared, inferred)
        {
            found.push(BodyRefutation {
                kind: RefutationKind::CustodyDuty,
                clause: refusal.as_str().to_owned(),
                line: diagnostic.line,
                reason: refusal.reason().to_owned(),
            });
        }
    }
    found.sort();
    found.dedup();
    found
}

/// The token range of `function`'s own body, if the unit defines one.
fn own_body_range(tokens: &[Token], function: &str) -> Option<(usize, usize)> {
    let (open, _) = find_function_body(tokens, function)?;
    let close = matching(tokens, open, "{", "}")?;
    Some((open, close))
}

/// Rewrite every call site of a held callee, except inside that callee's own
/// body: a function's own recursive call is the evidence, not a caller
/// trusting the declaration (the exemption constraint 8's mode gate keeps).
/// `None` means the stream is unchanged.
pub(crate) fn hold_call_sites(tokens: &[Token], held: &BTreeSet<String>) -> Option<Vec<Token>> {
    if held.is_empty() {
        return None;
    }
    let exempt = held
        .iter()
        .filter_map(|name| own_body_range(tokens, name).map(|range| (name.as_str(), range)))
        .collect::<Vec<_>>();
    let mut rewritten: Option<Vec<Token>> = None;
    for index in 0..tokens.len() {
        if tokens.get(index + 1).is_none_or(|token| token.text != "(") {
            continue;
        }
        let name = &tokens[index].text;
        if !held.contains(name) || !is_call_position(tokens, index) {
            continue;
        }
        if exempt
            .iter()
            .any(|(own, (start, end))| own == name && index > *start && index < *end)
        {
            continue;
        }
        let target = rewritten.get_or_insert_with(|| tokens.to_vec());
        target[index].text = format!("{HELD_PREFIX}{name}");
    }
    rewritten
}

/// The declared functions of `scope` whose own bodies refute a counted-user
/// clause over the settled stream and environment: the unit's own half of
/// the table.
fn local_refutations(
    tokens: &[Token],
    scope: &[String],
    environment: &ResourceContracts,
) -> BTreeSet<String> {
    scope
        .iter()
        .filter(|name| environment.contract(name).is_some())
        .filter(|name| !counted_users::refuted_own_clauses(tokens, name, environment).is_empty())
        .cloned()
        .collect()
}

/// Settle the unit's checked specifications (`checked_spec::settle`) and then
/// hold every call site of a refuted declaration: the audit's table `table`
/// (read from the environment before any gate rebuilt it) and this unit's own
/// counted-user refutations. A name the mode gate withdrew keeps that gate's
/// key. When anything is held the unit is settled again over the held stream,
/// so every gate, W4 and every classifier read the hold.
pub(crate) fn settle(
    tokens: &[Token],
    scope: &[String],
    contracts: &ResourceContracts,
    table: &BTreeSet<String>,
) -> Settled {
    let first = checked_spec::settle(tokens, scope, contracts);
    let settled_tokens = first.gate.tokens.as_deref().unwrap_or(tokens);
    let mut held = table.clone();
    held.extend(local_refutations(
        settled_tokens,
        scope,
        first.gate.derivation.environment(),
    ));
    for name in first.gate.agreement.withdrawn_names() {
        held.remove(name);
    }
    let Some(rewritten) = hold_call_sites(tokens, &held) else {
        return first;
    };
    let mut second = checked_spec::settle(&rewritten, scope, contracts);
    if second.gate.tokens.is_none() {
        second.gate.tokens = Some(rewritten);
    }
    second
}
