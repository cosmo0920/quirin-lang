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

//! Declared-contract / derived-summary **custody mode** agreement
//! (card **LOADER-ABI-MODES**).
//!
//! # The fail-open this closes
//!
//! Card LOADER-ABI compares the *types* a declared resource contract writes
//! against the C declaration in the analysed unit, and recorded its own limit
//! as the follow-on this module answers:
//!
//! > this card compares **types**. It does not compare a declared *mode*
//! > against anything the source says.
//!
//! A declaration may write `resource_borrowed(p)` for a body that frees `p`,
//! `resource_released(p)` for a body that never releases it, or
//! `resource_return_none()` for a body that hands back a fresh acquisition.
//! Every one of those is type-faithful, so LOADER-ABI admits it, and every one
//! of them buys a **caller** a `Safe` verdict the body contradicts. All three
//! are reproduced in `crates/qcore-c-gateway/tests/loader_abi_modes.rs`, and in
//! each the caller was measured `Safe` with no diagnostic at all.
//!
//! # What was already there, and why it was not enough
//!
//! W4 already derives a *validated* custody summary for a function defined in
//! the analysed unit, and `derive_custody_summaries` already notices when that
//! summary disagrees with the function's declared contract: it records the
//! disagreement, and [`crate::legacy_c::analyze_defined_function`] reports it
//! as a `SummaryConflict`, makes the **defining** function `Unknown` and wipes
//! its per-exit claims.
//!
//! That is a report about the definition. It is not a decision about the
//! *interface*: the declared contract stays in the environment and keeps
//! governing every call site, so a caller in the same unit goes on being
//! checked against a mode this pass has already measured the body to
//! contradict. The three reproductions above are exactly that — a definition
//! reported `Unknown` beside a caller reported `Safe`.
//!
//! # What this module decides
//!
//! For a contracted name **defined in this unit** whose body yields a validated
//! W4 summary, the declared custody *duty* is compared against the derived one
//! position by position. A **contradiction** — not a mere disagreement —
//! withdraws the declaration's application for this unit: every call site is
//! rewritten to a marked uncontracted callee, which refuses with a named key.
//!
//! Three properties are deliberate and load-bearing.
//!
//! * **Only contradictions refuse.** A declared mode this pass cannot check —
//!   no definition in the unit, no validated summary, an arity the derivation
//!   read differently — keeps the declared contract as the trusted premise it
//!   already was. Requiring validation *before* applying a contract would
//!   withdraw the whole declared-trust architecture, and this card does not.
//!   Absence of evidence is not evidence.
//! * **Refusal only, never repair.** The withdrawal rewrites call sites to an
//!   uncontracted callee, exactly as card LOADER-ABI does. It never installs
//!   the derived summary in the declaration's place: an inferred summary can be
//!   *weaker* than the declaration it replaces — dropping a non-null
//!   precondition, say — and swapping one in would be a route towards `Safe`.
//!   An uncontracted call is `Unknown` and poisons its function, so the
//!   withdrawal is monotone in the refusing direction.
//! * **The environment is left alone.** Unlike LOADER-ABI, this module does not
//!   remove the name from the contract environment. It is precisely the
//!   environment's declared contract that makes `derive_custody_summaries`
//!   record the conflict, so removing it would *delete* the `SummaryConflict`
//!   diagnostic and the `Unknown` it forces on the defining function. Keeping
//!   it retains that evidence and adds the call-site refusal on top: strictly
//!   more refusing than either half alone.
//!
//! # Why a fixed point
//!
//! Withdrawing a callee changes the token stream, which changes the summaries
//! derived for the bodies that call it, which can expose or retract a further
//! contradiction. The withdrawn set is therefore grown to a fixed point: each
//! round re-derives from the original stream with the whole accumulated set
//! applied, and the set only grows, so the loop settles after at most one round
//! per contracted name. In the overwhelmingly common case — no contradiction at
//! all — the loop runs its single derivation, which is the one the caller
//! needed anyway, and nothing is re-derived.
//!
//! # The boundary this version keeps, recorded rather than hidden
//!
//! The derived summary is derived *under the environment's other declared
//! premises*. A contradiction is therefore a contradiction relative to those
//! premises, not an absolute statement about the C. That is the same trust
//! boundary every W4 summary already sits behind, and it is recorded as
//! `LOADER-ABI-MODES-RELATIVE`. A declaration whose callee is defined in
//! another translation unit is never compared at all
//! (`LOADER-ABI-MODES-CROSS-UNIT`), which the census in the card report
//! measures rather than assumes.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::{
    ResourceContracts, ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
};
use qcore_schemas::reason::ReasonCategory;

use super::Token;
use crate::legacy_summary::{CustodyDerivation, derive_custody_summaries};

/// The prefix a mode-withdrawn callee's call sites are rewritten with.
///
/// Deliberately distinct from card LOADER-ABI's `qcore_abi_withdrawn__`: the
/// two gates answer different questions, and a report that conflated them could
/// not say whether a refusal came from the C declaration or from the body.
pub(crate) const WITHDRAWN_PREFIX: &str = "qcore_mode_withdrawn__";

/// The separator between the refusal token and the original callee name.
const WITHDRAWN_SEPARATOR: &str = "__";

/// The refusal and callee a rewritten name stands for, for diagnostics.
pub(crate) fn withdrawn_callee(function: &str) -> Option<(ModeRefusal, &str)> {
    let rest = function.strip_prefix(WITHDRAWN_PREFIX)?;
    let (token, callee) = rest.split_once(WITHDRAWN_SEPARATOR)?;
    Some((ModeRefusal::from_token(token)?, callee))
}

/// Why one declared contract's application was withdrawn for one unit.
///
/// The vocabulary is closed and carries a stable key, a full prose reason and a
/// [`ReasonCategory`], the shape [`crate::legacy_c::declared_abi::AbiRefusal`]
/// established. Both are `StaleEvidence`: a declaration the body contradicts is
/// evidence bound to a different body than the one under check. Neither is an
/// environment statement, so neither can be read as saying the analysis could
/// not run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModeRefusal {
    /// A declared parameter's custody duty is contradicted by the validated
    /// summary derived from the function's own body in this unit.
    ParameterCustodyContradiction,
    /// The declared result's custody duty is contradicted likewise.
    ResultCustodyContradiction,
}

impl ModeRefusal {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ParameterCustodyContradiction => "mode-parameter-contradiction",
            Self::ResultCustodyContradiction => "mode-result-contradiction",
        }
    }

    /// The identifier spelling carried inside a rewritten call token.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::ParameterCustodyContradiction => "parameter_custody",
            Self::ResultCustodyContradiction => "result_custody",
        }
    }

    /// The refusal a rewritten call token names.
    #[must_use]
    pub fn from_token(token: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|kind| kind.token() == token)
    }

    /// Every refusal, in declaration order. The vocabulary is closed.
    pub const ALL: [Self; 2] = [
        Self::ParameterCustodyContradiction,
        Self::ResultCustodyContradiction,
    ];

    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::ParameterCustodyContradiction => {
                "a declared contract parameter's custody duty is contradicted by the validated \
                 summary derived from the function's own body in this translation unit"
            }
            Self::ResultCustodyContradiction => {
                "the declared contract's result custody duty is contradicted by the validated \
                 summary derived from the function's own body in this translation unit"
            }
        }
    }

    #[must_use]
    pub const fn category(self) -> ReasonCategory {
        ReasonCategory::StaleEvidence
    }
}

/// What the mode comparison decided about one contracted name in one unit.
///
/// The four non-deciding outcomes are the whole point of the census: they size
/// how much of the declared corpus this family can *reach* at all, which is a
/// property of where the bodies live, not of whether the declarations are
/// faithful.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModeOutcome {
    /// The function is not among the definitions this pass considered, so there
    /// is no body to derive a summary from and nothing to compare
    /// (`LOADER-ABI-MODES-CROSS-UNIT`).
    ///
    /// Almost always that means the unit does not define it — a contract corpus
    /// of library interfaces has its bodies in other translation units. It also
    /// covers a definition an explicit `selected_functions` filter excluded,
    /// which is the same decision for the same reason: nothing was derived, so
    /// nothing is compared. The census entry point applies no such filter, so a
    /// census row means the first thing.
    Undefined,
    /// The unit defines the function but W4 derived no validated summary for
    /// it: the body is outside the projection, or no single mode describes
    /// every exit. Nothing is compared and the declaration applies.
    NoSummary,
    /// A validated summary was derived and reads exactly as the declaration.
    Agreed,
    /// A validated summary was derived and differs from the declaration
    /// without contradicting any custody duty — a nullability or owner-index
    /// refinement. Today's `SummaryConflict` still reports it; nothing is
    /// withdrawn.
    Disagreed,
    /// A validated summary was derived and contradicts a declared custody duty.
    Contradicted(ModeRefusal),
}

/// One census row: what the comparison decided about one contracted name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModeAgreementRow {
    /// The contracted function name.
    pub function: String,
    /// The decision.
    pub outcome: ModeOutcome,
}

/// Decide mode agreement for every contracted name over one preprocessed
/// translation unit.
///
/// This is the same decision the analyzer applies, exposed for measurement: the
/// census, the tests and the card report read it rather than a second
/// implementation.
///
/// A unit that defines **no** contracted name short-circuits, and the
/// short-circuit is *exact* rather than an approximation: with no contracted
/// name among the definitions, [`ModeAgreement::read`] classifies every one of
/// them `Undefined` whatever the derivation says, so deriving summaries for the
/// unit's other definitions cannot change a single row. The analyzer itself
/// still derives — it needs those summaries for its *undeclared* callees — but
/// the census does not, and without this a corpus-wide census pays a full W4
/// derivation per unit to produce rows that were settled by the definition list
/// alone.
#[must_use]
pub fn declared_mode_census(
    preprocessed: &str,
    contracts: &ResourceContracts,
) -> Vec<ModeAgreementRow> {
    let (tokens, conditionals) = super::tokenize_with_conditionals(preprocessed);
    // Card W45-PARENTHESISED-CALLEE: the stream the analysis reads.
    let (tokens, conditionals) = super::parenthesised_callee::normalise_owned(tokens, conditionals);
    // Card W45-SR5-SOUNDNESS: the escape the analysis applies.
    let tokens = super::reserved_spelling::escape_owned(tokens);
    let names = super::function_names(&tokens, &conditionals).definitions;
    let defined = names.iter().collect::<BTreeSet<_>>();
    if !contracts
        .functions()
        .keys()
        .any(|name| defined.contains(name))
    {
        let mut rows = contracts
            .functions()
            .keys()
            .map(|function| ModeAgreementRow {
                function: function.clone(),
                outcome: ModeOutcome::Undefined,
            })
            .collect::<Vec<_>>();
        rows.sort();
        return rows;
    }
    let gate = ModeGate::apply(&tokens, &names, contracts);
    gate.agreement.rows()
}

/// The custody duty one declared parameter mode places on the **caller**.
///
/// This is the abstraction the comparison is taken in, and it is chosen so that
/// a difference is a difference *the caller can be wrong about*. Nullability,
/// the owner index of a `Stored`/`Removed` mode and the replaced parameter of a
/// `Reallocated` result are refinements within one duty: they change what the
/// caller must additionally prove, never which side holds the resource after
/// the call. They are therefore not contradictions here, and the existing
/// `SummaryConflict` remains their only report (`LOADER-ABI-MODES-REFINEMENT`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParameterDuty {
    /// The caller still holds the resource after the call: `Borrowed`, `Owned`.
    Retained,
    /// The caller's direct duty ends at the call, because the callee destroyed
    /// the resource, took it, or attached it to another of the caller's:
    /// `Released`, `Consumed`, `Stored`.
    Transferred,
    /// The callee writes fresh custody into the pointed-to slot: `Out`.
    Delivered,
    /// The callee detaches the resource from the aggregate that owned it and
    /// hands direct custody back: `Removed`.
    Restored,
}

impl ParameterDuty {
    const fn of(mode: ResourceParameterMode) -> Self {
        match mode {
            ResourceParameterMode::Borrowed | ResourceParameterMode::Owned { .. } => Self::Retained,
            ResourceParameterMode::Released { .. }
            | ResourceParameterMode::Consumed { .. }
            | ResourceParameterMode::Stored { .. } => Self::Transferred,
            ResourceParameterMode::Out { .. } => Self::Delivered,
            ResourceParameterMode::Removed { .. } => Self::Restored,
        }
    }
}

/// The custody duty one declared result mode places on the caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResultDuty {
    /// The call produces no custody the caller must account for.
    Absent,
    /// The call hands the caller custody it must account for: `Owned`,
    /// `Reallocated`.
    Produced,
}

impl ResultDuty {
    const fn of(mode: ResourceReturnMode) -> Self {
        match mode {
            // Card W43-LIBC-RESULT-ALIAS: a derived result owns nothing, so it
            // places no custody duty on the caller.
            ResourceReturnMode::None | ResourceReturnMode::Derived { .. } => Self::Absent,
            ResourceReturnMode::Owned { .. }
            | ResourceReturnMode::Reallocated { .. }
            | ResourceReturnMode::Removed { .. } => Self::Produced,
        }
    }
}

/// Whether a validated summary contradicts a declared contract, and how.
///
/// The result is `None` for equality, for any difference inside one duty, and
/// for a summary whose parameter vector this pass cannot line up with the
/// declaration's — a length difference is a statement about the *signature*,
/// which is card LOADER-ABI's gate and is already decided there for a function
/// this unit defines, a definition being a declaration.
///
/// The parameter refusal is reported in preference to the result one when both
/// hold, so the key is a function of the pair rather than of iteration order.
///
/// Public because it is the whole relation: the tests decide it row by row and
/// the Lean namespace `QCore.DeclaredModeAgreement` decides the same finite
/// duty domain independently, with both sides' verdicts tied by a digest.
#[must_use]
pub fn mode_contradiction(
    declared: &ResourceFunctionContract,
    derived: &ResourceFunctionContract,
) -> Option<ModeRefusal> {
    if declared.parameters.len() == derived.parameters.len()
        && declared
            .parameters
            .iter()
            .zip(&derived.parameters)
            .any(|(declared, derived)| ParameterDuty::of(*declared) != ParameterDuty::of(*derived))
    {
        return Some(ModeRefusal::ParameterCustodyContradiction);
    }
    if ResultDuty::of(declared.result) != ResultDuty::of(derived.result) {
        return Some(ModeRefusal::ResultCustodyContradiction);
    }
    None
}

/// The mode agreement decision for one translation unit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ModeAgreement {
    /// Contracted names withdrawn for this unit, with the reason.
    withdrawn: BTreeMap<String, ModeRefusal>,
    /// Contracted names whose derived summary reads as the declaration does.
    agreed: BTreeSet<String>,
    /// Contracted names whose derived summary differs without contradicting.
    disagreed: BTreeSet<String>,
    /// Contracted names defined here for which no validated summary exists.
    no_summary: BTreeSet<String>,
    /// Contracted names this unit does not define.
    undefined: BTreeSet<String>,
}

impl ModeAgreement {
    /// Card W45-REFUTED-CONTRACT-WITHDRAWAL: the names this gate withdrew,
    /// which keep this gate's own refusal key in their unit.
    pub(crate) fn withdrawn_names(&self) -> impl Iterator<Item = &String> {
        self.withdrawn.keys()
    }

    /// Every decision, as census rows.
    fn rows(&self) -> Vec<ModeAgreementRow> {
        let mut rows = Vec::new();
        let mut push = |function: &String, outcome: ModeOutcome| {
            rows.push(ModeAgreementRow {
                function: function.clone(),
                outcome,
            });
        };
        for (function, refusal) in &self.withdrawn {
            push(function, ModeOutcome::Contradicted(*refusal));
        }
        for function in &self.agreed {
            push(function, ModeOutcome::Agreed);
        }
        for function in &self.disagreed {
            push(function, ModeOutcome::Disagreed);
        }
        for function in &self.no_summary {
            push(function, ModeOutcome::NoSummary);
        }
        for function in &self.undefined {
            push(function, ModeOutcome::Undefined);
        }
        rows.sort();
        rows
    }

    /// Classify every contracted name against one settled derivation.
    fn read(
        derivation: &CustodyDerivation,
        names: &[String],
        contracts: &ResourceContracts,
        carried: &BTreeMap<String, ModeRefusal>,
    ) -> Self {
        let defined = names.iter().collect::<BTreeSet<_>>();
        let mut agreement = Self {
            withdrawn: carried.clone(),
            ..Self::default()
        };
        for (name, declared) in contracts.functions() {
            if agreement.withdrawn.contains_key(name) {
                continue;
            }
            if !defined.contains(name) {
                agreement.undefined.insert(name.clone());
                continue;
            }
            let Some(derived) = derivation.inferred(name) else {
                agreement.no_summary.insert(name.clone());
                continue;
            };
            match mode_contradiction(declared, derived) {
                Some(refusal) => {
                    agreement.withdrawn.insert(name.clone(), refusal);
                }
                None if declared == derived => {
                    agreement.agreed.insert(name.clone());
                }
                None => {
                    agreement.disagreed.insert(name.clone());
                }
            }
        }
        agreement
    }

    /// Rewrite every call site of a withdrawn callee so that the rest of the
    /// analyzer sees an uncontracted, *named* callee.
    ///
    /// `None` means the stream is unchanged. The call-position rule is card
    /// LOADER-ABI's, reused rather than restated.
    ///
    /// **A withdrawn function's own recursive call is exempt**, and the
    /// exemption is load-bearing rather than an optimisation. Rewriting it
    /// changes the body the derivation reads, so the very summary that convicted
    /// the declaration stops being derivable, and with it the `SummaryConflict`
    /// that keeps the defining function `Unknown` with its per-exit claims
    /// wiped. A fail-closed pin from card C03-NFR
    /// (`assumed_neutral_refuses_every_body_that_carries_an_effect`) measured
    /// exactly that and was not weakened.
    ///
    /// The exemption costs nothing that the withdrawal exists to buy: the
    /// withdrawal stops *callers* trusting an interface the body contradicts,
    /// and a function's own recursive call is not a caller trusting it — it is
    /// the evidence. The defining function is already `Unknown` on the conflict,
    /// so nothing about it is trusted either way, and every genuine caller,
    /// including a mutually recursive one, is still rewritten.
    pub(crate) fn withdraw_call_sites(&self, tokens: &[Token]) -> Option<Vec<Token>> {
        if self.withdrawn.is_empty() {
            return None;
        }
        let exempt = self
            .withdrawn
            .keys()
            .filter_map(|name| own_body_range(tokens, name).map(|range| (name.clone(), range)))
            .collect::<BTreeMap<_, _>>();
        let mut rewritten: Option<Vec<Token>> = None;
        for index in 0..tokens.len() {
            if tokens.get(index + 1).is_none_or(|token| token.text != "(") {
                continue;
            }
            let name = &tokens[index].text;
            let Some(refusal) = self.withdrawn.get(name).copied() else {
                continue;
            };
            if !super::declared_abi::is_call_position(tokens, index) {
                continue;
            }
            if exempt
                .get(name)
                .is_some_and(|(start, end)| index > *start && index < *end)
            {
                continue;
            }
            let target = rewritten.get_or_insert_with(|| tokens.to_vec());
            target[index].text = format!(
                "{WITHDRAWN_PREFIX}{}{WITHDRAWN_SEPARATOR}{}",
                refusal.token(),
                tokens[index].text
            );
        }
        rewritten
    }
}

/// The token range of `function`'s own body in this unit, if it defines one.
fn own_body_range(tokens: &[Token], function: &str) -> Option<(usize, usize)> {
    let (open, _) = super::find_function_body(tokens, function)?;
    let close = super::matching(tokens, open, "{", "}")?;
    Some((open, close))
}

/// The settled result of the gate: the decision, the stream every consumer
/// below must read, and the derivation that decided it.
///
/// The derivation is handed back rather than recomputed: the whole-unit pass
/// needs one anyway, so the common no-contradiction path costs exactly what it
/// cost before this card.
pub(crate) struct ModeGate {
    pub(crate) agreement: ModeAgreement,
    /// `None` when no call site was rewritten.
    pub(crate) tokens: Option<Vec<Token>>,
    /// The derivation over the stream this gate settled on.
    pub(crate) derivation: CustodyDerivation,
}

impl ModeGate {
    /// Grow the withdrawn set to a fixed point over `tokens`.
    ///
    /// Each round derives summaries over the original stream with the whole
    /// accumulated withdrawal applied, so a round never *un*-withdraws. The set
    /// only grows and is bounded by the number of contracted names, so the loop
    /// settles; the bound is written out rather than left implicit.
    pub(crate) fn apply(tokens: &[Token], names: &[String], contracts: &ResourceContracts) -> Self {
        // One round: apply the accumulated withdrawal to the *original* stream,
        // derive over the result, and classify every contracted name against it.
        let round = |carried: &BTreeMap<String, ModeRefusal>| {
            let staged = ModeAgreement {
                withdrawn: carried.clone(),
                ..ModeAgreement::default()
            };
            let rewritten = staged.withdraw_call_sites(tokens);
            let derivation =
                derive_custody_summaries(rewritten.as_deref().unwrap_or(tokens), names, contracts);
            let agreement = ModeAgreement::read(&derivation, names, contracts, carried);
            Self {
                agreement,
                tokens: rewritten,
                derivation,
            }
        };
        let mut carried: BTreeMap<String, ModeRefusal> = BTreeMap::new();
        // Each iteration that continues adds at least one name to a set bounded
        // by the contracted names, so the loop settles; the bound is written out
        // rather than left implicit in the loop's shape. Falling out of it
        // settles on the last round, which keeps every name found so far
        // withdrawn — the refusing direction.
        let rounds = contracts.functions().len();
        let mut settled = round(&carried);
        for _ in 0..rounds {
            if settled.agreement.withdrawn.len() == carried.len() {
                return settled;
            }
            carried = settled.agreement.withdrawn.clone();
            settled = round(&carried);
        }
        settled
    }
}
