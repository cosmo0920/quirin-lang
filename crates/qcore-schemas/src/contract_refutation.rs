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

//! Declared contracts their own bodies refute (card
//! **W45-REFUTED-CONTRACT-WITHDRAWAL**, contract spec §C06.4).
//!
//! A declared contract of a function whose body an audit analyses is a
//! specification checked against that body. When the body **refutes** it — a
//! counted-user clause of the function's own row is refuted (§C04.7), or the
//! validated summary W4 derives from the body contradicts a declared custody
//! *duty* (§C04.1, the mode arm of gateway constraint 8) — the declaration is
//! withdrawn from every caller. The refutation is found only where the
//! defining unit is analysed, while callers may sit in any unit, so an audit
//! runs in two phases: the first computes a **refutation table** over every
//! unit, and the second analyses every unit with the table recorded in the
//! environment ([`crate::ResourceContracts::refuted_by_body`]).
//!
//! These are the data types only. The gateway
//! (`qcore_c_gateway::legacy_c::refuted_contract`) finds refutations and holds
//! call sites; the audit (`qcore_audit::refuted_contracts`) folds the table
//! and publishes it with its digest.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// What in a body refutes a declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefutationKind {
    /// A counted-user clause of the function's own row, declared or implicit,
    /// is refuted on a path of its body (§C04.7).
    CountedUserClause,
    /// The validated summary of the body contradicts a declared custody duty
    /// (§C04.1; constraint 8's `mode-parameter-contradiction` /
    /// `mode-result-contradiction`). A refinement inside one duty is not a
    /// refutation.
    CustodyDuty,
}

impl RefutationKind {
    /// The report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CountedUserClause => "counted_user_clause",
            Self::CustodyDuty => "custody_duty",
        }
    }
}

/// One refutation of one declared contract, as one unit's analysis found it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ContractRefutation {
    /// The analysed unit's file, as the report names it.
    pub unit: String,
    pub kind: RefutationKind,
    /// The refuted clause as a declaration spells it, or the refusal key of a
    /// contradicted duty.
    pub clause: String,
    /// The original `file:line` of the refuting exit, where it is established.
    pub origin: Option<String>,
    /// What refutes it.
    pub reason: String,
}

/// The refutation table of one run: every declared function some analysed
/// body refutes, with every refutation found. Ordered and duplicate-free, so
/// its canonical serialization — and therefore its digest — is a function of
/// the set of refutations alone, never of the order units were analysed in.
pub type RefutationTable = BTreeMap<String, Vec<ContractRefutation>>;

/// Insert one refutation, keeping each entry sorted and duplicate-free.
pub fn insert_refutation(
    table: &mut RefutationTable,
    function: &str,
    refutation: ContractRefutation,
) {
    let entry = table.entry(function.to_owned()).or_default();
    if let Err(position) = entry.binary_search(&refutation) {
        entry.insert(position, refutation);
    }
}

/// The union of two tables: the fold every composition uses. Associative,
/// commutative and idempotent, so a table folded over groups of units equals
/// the table folded over all of them.
#[must_use]
pub fn union(mut left: RefutationTable, right: &RefutationTable) -> RefutationTable {
    for (function, refutations) in right {
        for refutation in refutations {
            insert_refutation(&mut left, function, refutation.clone());
        }
    }
    left
}

/// The SHA-256 of the table's canonical serialization
/// ([`crate::CANONICAL_SERIALIZER`]): the identity a result computed under the
/// table carries, and the key a checkpoint that composes units must join.
/// The empty table has a digest too, so "no refutation" is itself a checked
/// input, never an absent one.
#[must_use]
pub fn table_digest(table: &RefutationTable) -> String {
    let bytes = serde_json::to_vec(table).expect("a refutation table always serializes");
    crate::sha256_hex(&bytes)
}
