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

//! The integration semigroup across results (card **AUDIT-COMPOSITION**;
//! `docs/plans/pluggable-cards/README.md` seam G; language specification
//! §L21.2, "Composition across results").
//!
//! # The carrier
//!
//! A [`Component`] is one producer's account of one published row:
//! `(unit identity, row key, property, profile, provenance, verdict tuple,
//! reason leaves)`, plus an opaque `detail` the producer's own report view
//! renders. A [`Contribution`] is a finite set of components.
//!
//! # The operation
//!
//! `⊕` ([`Contribution::union`]) is the **provenance-keyed union**: the
//! components of both sides, kept per [`ProvenanceKey`] (the row and the
//! provenance that produced it), with byte-identical components counted once.
//! It is associative, commutative and idempotent, with the empty contribution
//! as its identity, and it erases nothing: every component of either side is a
//! component of the union.
//!
//! # The derived row
//!
//! A published row is never stored; it is derived at read time
//! ([`Contribution::derive`]) as the four-value join
//! ([`crate::verdict::Verdict::join`], `Safe < Pending < Unknown < Unsafe`)
//! over every component of that row, carrying the union of their assumptions
//! and reason leaves. Consequently the join never lowers a verdict and adding
//! a component never removes an assumption.
//!
//! **Conflict is not resolved by choice.** Two components under one
//! provenance key whose bytes differ are both kept, and the row's join gains
//! `Unknown` with a [`ReasonCategory::CompositionConflict`] leaf. A conflicting
//! row is therefore never below `Unknown`; it is exactly `Unknown` unless some
//! component is itself `Unsafe`, which the join never lowers.
//!
//! **Removal is a re-join** ([`Contribution::remove`]): the components of a
//! withdrawn contribution are left out and the rows are derived again. Nothing
//! is rewritten, so a contribution removed from a union of disjoint
//! contributions restores exactly the prior derivation.
//!
//! A row with no component has no derivation (`None`), never the join's
//! identity `Safe`: absence is refusal.
//!
//! # What this module does not do
//!
//! It decides nothing about any program and resolves nothing between
//! producers. Which verdict a component carries is decided by its producer
//! (the trusted core, a checker plugin behind the firewall). The order of a
//! report's rows is presentation, carried beside the contribution by
//! [`ComponentSidecar`], and never consulted by the join. The route semigroup
//! inside one run (ordered consultation, first answer wins) is a different
//! composition and does not live here. The Lean model is
//! `proof/QCore/AuditComposition.lean`.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::reason::ReasonCategory;
use crate::verdict::Verdict;

/// The wire version of [`Contribution`] and [`ComponentSidecar`].
pub const COMPOSITION_SCHEMA_VERSION: u32 = 1;

/// The unit a component speaks about: the library (source-set) profile and
/// the translation unit's file, as an audit report names them.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitRef {
    pub library: String,
    pub file: String,
}

/// The identity of a published row, independent of who produced it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowKey {
    pub unit: UnitRef,
    /// Which row of the unit (for example `unit#0`, the unit's first custody
    /// row).
    pub row: String,
    /// The property the row decides (for example `resource-custody.v1`).
    pub property: String,
    /// The profile the row is evaluated under.
    pub profile: String,
}

/// The gateway rule set a producer ran, with the rules it switched off.
///
/// `off` is the run's `QCORE_GATEWAY_RULES_OFF` list as the gateway applied
/// it (card GATEWAY-RULE-API), empty when no rule is switched off.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleSet {
    pub registry: String,
    #[serde(default)]
    pub off: BTreeSet<String>,
}

/// An external checker's identity, for a component a checker plugin produced.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckerIdentity {
    pub id: String,
    pub version: String,
    pub digest: String,
}

/// Who produced a component: the analyzer revision, the rule set with its
/// off-list, the digests of the contract bundles it read and, for an external
/// checker, its identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub analyzer_revision: String,
    pub rule_set: RuleSet,
    /// Bundle name to SHA-256. Empty when the producer recorded none.
    pub contract_bundles: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_checker: Option<CheckerIdentity>,
}

/// The key under which components are unioned: one row, one producer.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceKey {
    pub row: RowKey,
    pub provenance: Provenance,
}

/// The verdict contract's remaining fields (AGENTS.md invariant 2): the
/// artifact, property and profile are the component's [`RowKey`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerdictTuple {
    pub verdict: Verdict,
    pub scope: String,
    pub bounds: String,
    /// Named assumptions the verdict holds under.
    pub assumptions: BTreeSet<String>,
    pub evidence_version: String,
}

/// One reason a component's verdict is what it is: its closed §L21.1 category
/// when it has one (`None` asserts no category) and its text.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReasonLeaf {
    pub category: Option<ReasonCategory>,
    pub text: String,
}

/// One producer's account of one row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Component {
    pub unit: UnitRef,
    pub row: String,
    pub property: String,
    pub profile: String,
    pub provenance: Provenance,
    pub verdict: VerdictTuple,
    pub reasons: Vec<ReasonLeaf>,
    /// The producer's own presentation of the row. Opaque to the join: it
    /// takes part in a component's bytes (so two producers that differ only
    /// here conflict) and is never read by [`Contribution::derive`].
    #[serde(default)]
    pub detail: Value,
}

impl Component {
    /// The published row this component speaks about.
    #[must_use]
    pub fn row_key(&self) -> RowKey {
        RowKey {
            unit: self.unit.clone(),
            row: self.row.clone(),
            property: self.property.clone(),
            profile: self.profile.clone(),
        }
    }

    /// The key it is unioned under.
    #[must_use]
    pub fn provenance_key(&self) -> ProvenanceKey {
        ProvenanceKey {
            row: self.row_key(),
            provenance: self.provenance.clone(),
        }
    }

    /// The component's bytes: its canonical JSON (every container is ordered,
    /// so the serialization is deterministic). Two components are identical
    /// exactly when their bytes are.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("a component is plain serializable data")
    }
}

/// A finite set of components, unioned per provenance key.
///
/// Every construction goes through [`Self::insert`], so a deserialized
/// contribution is the union of its listed components: duplicates collapse
/// and conflicts are kept, exactly as `⊕` would have built it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Contribution {
    components: BTreeMap<ProvenanceKey, BTreeMap<Vec<u8>, Component>>,
}

impl Contribution {
    /// The identity of `⊕`: no component.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The contribution of one component.
    #[must_use]
    pub fn singleton(component: Component) -> Self {
        let mut contribution = Self::new();
        contribution.insert(component);
        contribution
    }

    /// `self ⊕ {component}`. A byte-identical component is already present
    /// and changes nothing.
    pub fn insert(&mut self, component: Component) {
        let bytes = component.bytes();
        self.components
            .entry(component.provenance_key())
            .or_default()
            .entry(bytes)
            .or_insert(component);
    }

    /// `self ⊕ other`: provenance-keyed union.
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        let mut union = self.clone();
        for component in other.components() {
            union.insert(component.clone());
        }
        union
    }

    /// The re-join without `other`: every component of `self` that is not a
    /// byte-identical component of `other`.
    #[must_use]
    pub fn remove(&self, other: &Self) -> Self {
        let mut rest = Self::new();
        for component in self.components() {
            if !other.contains(component) {
                rest.insert(component.clone());
            }
        }
        rest
    }

    /// Whether a byte-identical component is present.
    #[must_use]
    pub fn contains(&self, component: &Component) -> bool {
        self.components
            .get(&component.provenance_key())
            .is_some_and(|by_bytes| by_bytes.contains_key(&component.bytes()))
    }

    /// Every component, in canonical order (provenance key, then bytes).
    pub fn components(&self) -> impl Iterator<Item = &Component> {
        self.components.values().flat_map(BTreeMap::values)
    }

    /// The number of distinct components.
    #[must_use]
    pub fn len(&self) -> usize {
        self.components.values().map(BTreeMap::len).sum()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// Every row some component speaks about.
    #[must_use]
    pub fn rows(&self) -> BTreeSet<RowKey> {
        self.components.keys().map(|key| key.row.clone()).collect()
    }

    /// The provenance keys holding more than one distinct component.
    pub fn conflicts(&self) -> impl Iterator<Item = &ProvenanceKey> {
        self.components
            .iter()
            .filter(|(_, by_bytes)| by_bytes.len() > 1)
            .map(|(key, _)| key)
    }

    /// The published row: the join over its components. `None` when no
    /// component speaks about `row`.
    #[must_use]
    pub fn derive(&self, row: &RowKey) -> Option<RowDerivation<'_>> {
        let mut verdict = Verdict::Safe;
        let mut assumptions = BTreeSet::new();
        let mut reasons = BTreeSet::new();
        let mut conflicts = Vec::new();
        let mut components = Vec::new();
        for (key, by_bytes) in self
            .components
            .range(row_start(row)..)
            .take_while(|(key, _)| &key.row == row)
        {
            if by_bytes.len() > 1 {
                verdict = verdict.join(Verdict::Unknown);
                reasons.insert(conflict_leaf(key));
                conflicts.push(key);
            }
            for component in by_bytes.values() {
                verdict = verdict.join(component.verdict.verdict);
                assumptions.extend(component.verdict.assumptions.iter().cloned());
                reasons.extend(component.reasons.iter().cloned());
                components.push(component);
            }
        }
        if components.is_empty() {
            return None;
        }
        Some(RowDerivation {
            verdict,
            assumptions,
            reasons,
            conflicts,
            components,
        })
    }
}

/// The least provenance key of `row`, where its range in the key order starts.
fn row_start(row: &RowKey) -> ProvenanceKey {
    ProvenanceKey {
        row: row.clone(),
        provenance: Provenance {
            analyzer_revision: String::new(),
            rule_set: RuleSet {
                registry: String::new(),
                off: BTreeSet::new(),
            },
            contract_bundles: BTreeMap::new(),
            external_checker: None,
        },
    }
}

/// The reason a conflicted provenance key adds to its row. It names the key
/// only, never how many components conflict, so adding a component never
/// replaces a reason the row already carried.
fn conflict_leaf(key: &ProvenanceKey) -> ReasonLeaf {
    ReasonLeaf {
        category: Some(ReasonCategory::CompositionConflict),
        text: format!(
            "composition-conflict: components of row `{}` under one provenance (analyzer \
             `{}`) differ; all are kept and none is chosen",
            key.row.row, key.provenance.analyzer_revision
        ),
    }
}

/// A published row, derived.
#[derive(Clone, Debug, PartialEq)]
pub struct RowDerivation<'a> {
    /// The join over every component, and `Unknown` when any key conflicts.
    pub verdict: Verdict,
    /// The union of every component's assumptions.
    pub assumptions: BTreeSet<String>,
    /// The union of every component's reason leaves, and one
    /// `composition-conflict` leaf per conflicted key.
    pub reasons: BTreeSet<ReasonLeaf>,
    /// The conflicted keys, in canonical order.
    pub conflicts: Vec<&'a ProvenanceKey>,
    /// Every component of the row, in canonical order.
    pub components: Vec<&'a Component>,
}

impl RowDerivation<'_> {
    /// Whether some provenance key of the row holds conflicting components.
    #[must_use]
    pub fn is_conflicted(&self) -> bool {
        !self.conflicts.is_empty()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContributionWire<C> {
    schema_version: u32,
    components: C,
}

impl Serialize for Contribution {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ContributionWire {
            schema_version: COMPOSITION_SCHEMA_VERSION,
            components: self.components().collect::<Vec<_>>(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Contribution {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = ContributionWire::<Vec<Component>>::deserialize(deserializer)?;
        if wire.schema_version != COMPOSITION_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(format!(
                "composition schema version {} is not {COMPOSITION_SCHEMA_VERSION}",
                wire.schema_version
            )));
        }
        let mut contribution = Self::new();
        for component in wire.components {
            contribution.insert(component);
        }
        Ok(contribution)
    }
}

/// The component store written beside a report: the contribution, and the
/// order in which the report presents its rows.
///
/// The order is presentation only. [`Self::validate`] requires it to name
/// every row of the contribution exactly once and nothing else, so no
/// component is unpresented and no presented row lacks a component.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentSidecar {
    pub schema_version: u32,
    /// How the producer renders a row from its derivation (for example
    /// `qcore-audit.report-units.v1`).
    pub view: String,
    pub order: Vec<RowKey>,
    pub contribution: Contribution,
}

impl ComponentSidecar {
    /// Refuse a sidecar whose order and contribution disagree.
    ///
    /// # Errors
    ///
    /// Names the first version mismatch, repeated row, row without component
    /// or component outside the order.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != COMPOSITION_SCHEMA_VERSION {
            return Err(format!(
                "component sidecar schema version {} is not {COMPOSITION_SCHEMA_VERSION}",
                self.schema_version
            ));
        }
        let rows = self.contribution.rows();
        let mut seen = BTreeSet::new();
        for row in &self.order {
            if !seen.insert(row) {
                return Err(format!("row {row:?} is presented twice"));
            }
            if !rows.contains(row) {
                return Err(format!("row {row:?} is presented without a component"));
            }
        }
        if let Some(row) = rows.iter().find(|row| !seen.contains(row)) {
            return Err(format!("row {row:?} has components but is not presented"));
        }
        Ok(())
    }
}
