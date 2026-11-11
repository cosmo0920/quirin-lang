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

//! The two levels of a dereference contract (card **NULL-DEREF-QTT-GROUNDING**,
//! contract specification C09.3, grammar G05d; USER DECISION 2026-10-11, "Two
//! levels").
//!
//! A dereference of the pointer formal `B` is safe only when three parts hold,
//! each a projection of the same use and each owned by an existing system:
//!
//! | Part | Canonical clause | Owner |
//! | --- | --- | --- |
//! | (a) existence | `quant null-deref #B;` | the null-dereference property |
//! | (b) extent | `quant checks B, Idx #i: #i == 0;` | the bounds consumer |
//! | (c) liveness | `quant live #B;` | custody, through its own resource v1 declaration |
//!
//! Authors may write the composite shorthand `quant deref #B;`, the parts, or a
//! mix. The bounds contract v1 loader elaborates every shorthand into its
//! canonical parts before any consumer sees the declaration ([`Elaboration`]):
//! a part a shorthand gives and an explicitly written part that agrees are
//! merged; parts that disagree are refused (`quant-deref-conflict`), as is a
//! repeated shorthand. Consumers, per-clause standing and the dependency sets
//! see only canonical parts, so a shorthand and its decomposition give the
//! same clause set and the same standing.
//!
//! The pin stays over the text as written (`text_sha256`): rewriting a
//! shorthand as its decomposition changes the text and needs a re-pin. The
//! canonical digest ([`canonical_sha256`]) says when two texts mean the same.
//!
//! Custody reads resource v1 only, and resource v1 refuses every `quant`
//! form (its loader admits `requires` and `ensures normal` alone), so the
//! liveness part never reaches custody through this module. An audit route
//! admits a declaration stating it only where the function's own resource v1
//! declaration already puts the identity at an `Owned` position (the caller
//! must retain a known live resource across the call), and refuses the route
//! otherwise ([`check_live_backing`]): a stated part no consumer checks is
//! never silently dropped.
//!
//! The decisions' Lean model: `QCore.DerefDecomposition`
//! (`elaboration_preserves_meaning`, `merging_an_agreeing_overlap_is_idempotent`).

use std::collections::{BTreeMap, BTreeSet};

use qcore_frontend::frontend::SourceContractClauseKind;
use qcore_frontend::lexer::{TokenKind, lex};
use qcore_schemas::contract::{ResourceContracts, ResourceParameterMode};
use qcore_schemas::sha256_hex;

use crate::bounds_contract::{BoundsClause, BoundsClauseEntry, BoundsContracts, ReviewNote};
use crate::bounds_quant::{Formals, clause_text, read_quant_check};

/// The bound variable of the shorthand's extent part.
pub const SHORTHAND_VARIABLE: &str = "i";

/// One collocated dereference form, as written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartForm {
    /// `quant null-deref #B;`: part (a).
    NullDeref,
    /// `quant live #B;`: part (c).
    Live,
    /// `quant deref #B;`: the shorthand for (a), (b) and (c).
    Deref,
    /// Any other explicitly written part (`quant checks`).
    Explicit,
}

impl PartForm {
    /// The form a clause kind introduces, when it is one of the `#B` forms.
    #[must_use]
    pub fn of(kind: &SourceContractClauseKind) -> Option<Self> {
        match kind {
            SourceContractClauseKind::QuantNullDeref => Some(Self::NullDeref),
            SourceContractClauseKind::QuantLive => Some(Self::Live),
            SourceContractClauseKind::QuantDeref => Some(Self::Deref),
            _ => None,
        }
    }

    fn word(self) -> &'static str {
        match self {
            Self::NullDeref => "null-deref",
            Self::Live => "live",
            Self::Deref | Self::Explicit => "deref",
        }
    }

    /// The clause as written, normalised (the pinned text's line).
    #[must_use]
    pub fn written(self, parameter: &str) -> String {
        format!("quant {} #{parameter}", self.word())
    }

    #[must_use]
    pub fn shape_error(self) -> String {
        match self {
            Self::NullDeref => {
                "quant-null-deref-shape: a null-dereference check is `quant null-deref #B;`".into()
            }
            Self::Live => "quant-live-shape: a liveness part is `quant live #B;`".into(),
            Self::Deref | Self::Explicit => {
                "quant-deref-shape: a dereference shorthand is `quant deref #B;`".into()
            }
        }
    }

    #[must_use]
    pub fn parameter_error(self, name: &str) -> String {
        format!(
            "quant-{}-parameter: `#{name}` names no pointer parameter of the declaration",
            self.word()
        )
    }
}

/// Part (c), canonical.
#[must_use]
pub fn live_part(parameter: &str) -> BoundsClauseEntry {
    BoundsClauseEntry {
        clause: BoundsClause::Live {
            parameter: parameter.to_owned(),
        },
        text: format!("quant live #{parameter}"),
        review: None,
    }
}

/// The canonical parts of `quant deref #B;`: (a), (b) at index 0 in element
/// units, (c), read by the same readers as their written forms.
///
/// # Errors
///
/// Only if the extent part fails its own reader (it cannot for a pointer
/// formal; kept total-or-reject).
pub fn shorthand_parts(
    parameter: &str,
    formals: &dyn Formals,
) -> Result<Vec<BoundsClauseEntry>, String> {
    let v = SHORTHAND_VARIABLE;
    let tokens = lex(&format!("{parameter}, Idx #{v}: #{v} == 0"))
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|token| token.kind)
        .filter(|kind| *kind != TokenKind::Eof)
        .collect::<Vec<_>>();
    let read = read_quant_check(&tokens, formals)?;
    Ok(vec![
        BoundsClauseEntry {
            clause: BoundsClause::NullDeref {
                parameter: parameter.to_owned(),
            },
            text: format!("quant null-deref #{parameter}"),
            review: None,
        },
        BoundsClauseEntry {
            clause: BoundsClause::QuantCheck {
                parameter: read.parameter,
                variable: read.variable,
                predicate: read.predicate,
            },
            text: clause_text(&tokens),
            review: None,
        },
        live_part(parameter),
    ])
}

/// The overlap key of a canonical part (one per pointer and kind).
fn key(clause: &BoundsClause) -> Option<String> {
    match clause {
        BoundsClause::NullDeref { parameter } => Some(format!("null-deref {parameter}")),
        BoundsClause::QuantCheck { parameter, .. } => Some(format!("quant {parameter}")),
        BoundsClause::Live { parameter } => Some(format!("live {parameter}")),
        _ => None,
    }
}

/// Two parts with one key agree: the same pointer and, for an extent part, the
/// same predicate (the bound variable's spelling aside).
fn agree(left: &BoundsClause, right: &BoundsClause) -> bool {
    match (left, right) {
        (
            BoundsClause::QuantCheck {
                parameter: a,
                predicate: p,
                ..
            },
            BoundsClause::QuantCheck {
                parameter: b,
                predicate: q,
                ..
            },
        ) => a == b && p == q,
        _ => left == right,
    }
}

/// Which canonical parts a shorthand gave, while one declaration is read.
#[derive(Debug, Default)]
pub struct Elaboration {
    shorthand_keys: BTreeSet<String>,
    shorthands: BTreeSet<String>,
}

impl Elaboration {
    /// Whether the part at `key` came from a shorthand.
    #[must_use]
    pub fn from_shorthand(&self, key: &str) -> bool {
        self.shorthand_keys.contains(key)
    }

    /// Add one written form's canonical parts. A part that overlaps a part of
    /// the other level (a shorthand's against a written one) is merged when
    /// they agree, the shorthand's spelling kept, and refused when not; two
    /// written parts with one key are left to the loader's duplicate rule, as
    /// before this card.
    ///
    /// # Errors
    ///
    /// `quant-deref-conflict` for disagreeing parts; a repeated shorthand.
    pub fn add(
        &mut self,
        clauses: &mut Vec<BoundsClauseEntry>,
        parts: Vec<BoundsClauseEntry>,
        form: PartForm,
    ) -> Result<(), String> {
        let shorthand = form == PartForm::Deref;
        if shorthand {
            let Some(BoundsClause::NullDeref { parameter }) = parts.first().map(|p| &p.clause)
            else {
                return Err("quant-deref-shape: a shorthand without its parts".into());
            };
            if !self.shorthands.insert(parameter.clone()) {
                return Err(format!("duplicate bounds clause for deref {parameter}"));
            }
        }
        for part in parts {
            let Some(part_key) = key(&part.clause) else {
                clauses.push(part);
                continue;
            };
            let existing = clauses
                .iter()
                .position(|entry| key(&entry.clause).as_deref() == Some(part_key.as_str()));
            let existing_from_shorthand = self.shorthand_keys.contains(&part_key);
            match existing {
                Some(index) if shorthand != existing_from_shorthand => {
                    if !agree(&clauses[index].clause, &part.clause) {
                        return Err(format!(
                            "quant-deref-conflict: `{}` disagrees with `{}` (the shorthand `quant deref #B;` means `quant null-deref #B; quant checks B, Idx #i: #i == 0; quant live #B;`)",
                            part.text, clauses[index].text
                        ));
                    }
                    if shorthand {
                        // The shorthand's spelling is the canonical one.
                        clauses[index] = part;
                        self.shorthand_keys.insert(part_key);
                    }
                }
                _ => {
                    if shorthand {
                        self.shorthand_keys.insert(part_key);
                    }
                    clauses.push(part);
                }
            }
        }
        Ok(())
    }
}

fn review_text(note: &ReviewNote) -> String {
    format!(
        "review {:?} owner {:?} {}",
        note.reason,
        note.owner,
        note.category()
    )
}

/// The canonical digest: the signature, then the sorted canonical clause set
/// (each part's canonical text with its review note), then the contract-level
/// review note.
#[must_use]
pub fn canonical_sha256(
    signature: &str,
    clauses: &[BoundsClauseEntry],
    contract_review: Option<&ReviewNote>,
) -> String {
    let parts = canonical_parts(clauses);
    let mut text = String::from(signature);
    text.push_str("\nrequires bounds_contract_v1");
    for part in &parts {
        text.push('\n');
        text.push_str(part);
    }
    if let Some(note) = contract_review {
        text.push_str("\ncontract ");
        text.push_str(&review_text(note));
    }
    sha256_hex(text.as_bytes())
}

/// The canonical clause set: every part's canonical text (with its review
/// note), sorted.
#[must_use]
pub fn canonical_parts(clauses: &[BoundsClauseEntry]) -> BTreeSet<String> {
    clauses
        .iter()
        .map(|entry| match &entry.review {
            Some(note) => format!("{} [{}]", entry.text, review_text(note)),
            None => entry.text.clone(),
        })
        .collect()
}

/// Whether any declaration states a part of this card's new forms (the
/// liveness part; a shorthand always gives one).
#[must_use]
pub fn states_live(contracts: &BoundsContracts) -> bool {
    contracts.functions.values().any(|contract| {
        contract
            .clauses
            .iter()
            .any(|entry| matches!(entry.clause, BoundsClause::Live { .. }))
    })
}

/// Each declaration's canonical digest, by name.
#[must_use]
pub fn canonical_digests(contracts: &BoundsContracts) -> BTreeMap<String, String> {
    contracts
        .functions
        .iter()
        .map(|(name, contract)| (name.clone(), contract.canonical_sha256.clone()))
        .collect()
}

/// Admit every liveness part only where custody's own resource v1 declaration
/// of the function states it: the parameter's resource mode is `Owned` (the
/// caller must retain a known live resource across the call) in a declaration
/// in force with the same arity. Returns one line per backed part.
///
/// # Errors
///
/// `quant-live-unbacked` (no resource v1 declaration in force, or a different
/// arity) and `quant-live-contradicts-custody` (another mode): the route is
/// refused, total-or-reject, so a stated part no consumer checks never passes
/// silently.
pub fn check_live_backing(
    bounds: &BoundsContracts,
    resource: &ResourceContracts,
) -> Result<Vec<String>, String> {
    let mut backed = Vec::new();
    for (name, contract) in &bounds.functions {
        for entry in &contract.clauses {
            let BoundsClause::Live { parameter } = &entry.clause else {
                continue;
            };
            let Some((index, _)) = contract.parameter(parameter) else {
                return Err(format!(
                    "quant-live-parameter: `{name}` names no parameter `{parameter}`"
                ));
            };
            let Some(custody) = resource.functions().get(name) else {
                return Err(format!(
                    "quant-live-unbacked: `{name}` states `quant live #{parameter}`, but no resource v1 declaration of `{name}` is in force; custody reads liveness from resource v1 only"
                ));
            };
            if custody.parameters.len() != contract.parameters.len() {
                return Err(format!(
                    "quant-live-unbacked: `{name}`'s resource v1 declaration has {} parameters, its bounds declaration {}",
                    custody.parameters.len(),
                    contract.parameters.len()
                ));
            }
            match custody.parameters[index] {
                ResourceParameterMode::Owned { .. } => backed.push(format!(
                    "{name}: quant live #{parameter} backed by resource_owned at position {index}"
                )),
                ref other => {
                    return Err(format!(
                        "quant-live-contradicts-custody: `{name}` states `quant live #{parameter}`, but its resource v1 declaration gives position {index} the mode {other:?}, not resource_owned"
                    ));
                }
            }
        }
    }
    Ok(backed)
}
