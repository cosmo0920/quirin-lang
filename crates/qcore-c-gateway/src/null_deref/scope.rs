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

//! Card **NULL-MEMBER-INVARIANT-SCOPE** (contract specification C09.3,
//! grammar G05f, gateway rules 14x–14z): the declarations of stable members
//! and function-scope entries as the walk reads them, the stores it records
//! against them, and the key shapes a stable member's fact may persist on.
//! The walk's hooks are in `walk.rs` (`memory_written`, `store`,
//! `record_initializer`, `call`, the entry facts); the decisions are
//! [`qcore_core::null_member_scope`].

use std::collections::BTreeMap;

use qcore_contract::bounds_contract::{BoundsClause, BoundsContracts};

/// A declared stable member (`nonnull_member(TAG, FIELD) where quant #scope [[stable]]`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct StableDecl {
    /// The declaration that carries it.
    pub(super) host: String,
    /// The hold, when the declaration is not in force.
    pub(super) held: Option<String>,
}

/// One function-scope entry (`nonnull_member(TAG, FIELD, P) where quant #scope [[scope]]`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ScopeEntry {
    pub(super) position: usize,
    pub(super) tag: String,
    pub(super) field: String,
    /// The clause as written.
    pub(super) text: String,
}

/// A function's declared scope entries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ScopeDecl {
    pub(super) parameters: usize,
    pub(super) entries: Vec<ScopeEntry>,
    pub(super) held: Option<String>,
}

/// The stable members (by tag and field) and the scope declarations (by
/// function) of the bounds environment.
pub(super) fn declared_stable_and_scopes(
    contracts: &BoundsContracts,
) -> (
    BTreeMap<(String, String), StableDecl>,
    BTreeMap<String, ScopeDecl>,
) {
    let mut stable = BTreeMap::new();
    let mut scopes = BTreeMap::new();
    for (name, contract) in contracts.null_declarations() {
        let held = contract.held().then(|| contract.holds().join("; "));
        let mut entries = Vec::new();
        for entry in &contract.clauses {
            match &entry.clause {
                BoundsClause::NonnullMemberStable { tag, field } => {
                    let decl = StableDecl {
                        host: name.clone(),
                        held: held.clone(),
                    };
                    // Two hosts of one member: held when either is.
                    stable
                        .entry((tag.clone(), field.clone()))
                        .and_modify(|existing: &mut StableDecl| {
                            if existing.held.is_none() {
                                existing.held.clone_from(&decl.held);
                            }
                        })
                        .or_insert(decl);
                }
                BoundsClause::NonnullMemberScope {
                    tag,
                    field,
                    parameter,
                } => {
                    if let Some((position, _)) = contract.parameter(parameter) {
                        entries.push(ScopeEntry {
                            position,
                            tag: tag.clone(),
                            field: field.clone(),
                            // The evidence label names the fact kind; card
                            // NONNULL-MEMBER-UNIFIED-QUALIFIER kept it byte
                            // for byte (rows and the fold are unchanged), so
                            // it is no surface spelling: the clause is
                            // `nonnull_member(TAG, FIELD, P) where quant
                            // #scope [[scope]]`.
                            text: format!("nonnull_member_scope({tag}, {field}, {parameter})"),
                        });
                    }
                }
                _ => {}
            }
        }
        if !entries.is_empty() {
            scopes.insert(
                name.clone(),
                ScopeDecl {
                    parameters: contract.parameters.len(),
                    entries,
                    held,
                },
            );
        }
    }
    (stable, scopes)
}

/// Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER (USER DECISION 2026-10-15, "Only
/// nonnull_member* need it"): one named premise a null row rests on, through
/// the member facts that state it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct NullPremise {
    pub name: String,
    pub statement: String,
    /// The row dependencies that carry it: `stable:TAG.FIELD` (a stable
    /// member's fact), `member:TAG.FIELD` (a plain member's) or a function
    /// name (its scope entries).
    pub through: Vec<String>,
}

/// The dependency prefix a row lists a premise under.
pub const PREMISE_DEPENDENCY: &str = "premise:";

/// Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: the named premises of the member
/// facts, keyed by the row dependency each fact is listed under: a stable
/// member's `stable:TAG.FIELD` (its host declaration's premises) and a scope
/// declaration's function name (the declaration's premises); card
/// NONNULL-MEMBER-UNIFIED-QUALIFIER (USER DECISION 2026-10-16, "Yes, all
/// kinds"): and a plain member's `member:TAG.FIELD` (rule 14t; its host
/// declaration's premises). A held declaration keeps its premises: a row
/// naming it lists them too.
#[must_use]
pub fn member_fact_premises(
    contracts: &BoundsContracts,
) -> BTreeMap<String, Vec<(String, String)>> {
    let mut out: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for (name, contract) in contracts.null_declarations() {
        let premises = contract
            .premises()
            .into_iter()
            .map(|(n, s)| (n.to_owned(), s.to_owned()))
            .collect::<Vec<_>>();
        if premises.is_empty() {
            continue;
        }
        for entry in &contract.clauses {
            let key = match &entry.clause {
                BoundsClause::NonnullMember { tag, field } => format!("member:{tag}.{field}"),
                BoundsClause::NonnullMemberStable { tag, field } => format!("stable:{tag}.{field}"),
                BoundsClause::NonnullMemberScope { .. } => name.clone(),
                _ => continue,
            };
            let listed = out.entry(key).or_default();
            for premise in &premises {
                if !listed.contains(premise) {
                    listed.push(premise.clone());
                }
            }
        }
    }
    out
}

/// Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: the premises a row whose
/// dependency set is `dependencies` rests on, by name, each with the
/// dependencies that carry it.
#[must_use]
pub fn premises_of<'a>(
    dependencies: impl IntoIterator<Item = &'a str>,
    premises: &BTreeMap<String, Vec<(String, String)>>,
) -> Vec<NullPremise> {
    let mut by_name: BTreeMap<String, NullPremise> = BTreeMap::new();
    for dependency in dependencies {
        for (name, statement) in premises.get(dependency).into_iter().flatten() {
            let premise = by_name.entry(name.clone()).or_insert_with(|| NullPremise {
                name: name.clone(),
                statement: statement.clone(),
                through: Vec::new(),
            });
            if !premise.through.iter().any(|d| d == dependency) {
                premise.through.push(dependency.to_owned());
            }
        }
    }
    let mut out = by_name.into_values().collect::<Vec<_>>();
    for premise in &mut out {
        premise.through.sort();
    }
    out
}

/// One store to a stable member (rule 14x's evidence).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct StableStore {
    /// `TAG.FIELD`.
    pub member: String,
    pub function: String,
    pub line: usize,
    /// `assignment` (an existing object: examined), `initializer` (an
    /// initializer list: a fresh object, exempt) or `whole-object` (a struct
    /// assignment or a typed fill or copy of an existing object: examined,
    /// never proved).
    pub kind: &'static str,
    /// The stored value's check class (`proved`, `moved`, `candidate`, ...).
    pub outcome: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
    /// The stable members the stored value's proof rests on (the fold
    /// withdraws this store's proof with any of them).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub relies: Vec<String>,
}

/// The tracked variable and field of a path key a stable member's fact may
/// persist on: exactly `t:ID^.FIELD` (one dereference of a tracked local or
/// formal, whose value only an assignment the walk sees can change).
pub(super) fn persistable_key(key: &str) -> Option<(&str, &str)> {
    let rest = key.strip_prefix("t:")?;
    let (id, field) = rest.split_once("^.")?;
    if id.is_empty() || field.is_empty() || field.contains(['^', '.']) || id.contains(['^', '.']) {
        return None;
    }
    Some((id, field))
}

/// A C library call that fills or copies the object its first argument
/// points to (a whole-object store when that object is a `struct TAG`).
pub(super) fn is_object_fill(name: &str) -> bool {
    matches!(
        name,
        "memset"
            | "memcpy"
            | "memmove"
            | "bzero"
            | "explicit_bzero"
            | "__builtin_memset"
            | "__builtin_memcpy"
            | "__builtin_memmove"
            | "__builtin___memset_chk"
            | "__builtin___memcpy_chk"
            | "__builtin___memmove_chk"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_dereference_of_a_tracked_variable_persists() {
        assert_eq!(persistable_key("t:0x1^.rkb_rk"), Some(("0x1", "rkb_rk")));
        assert_eq!(persistable_key("g:0x1^.rkb_rk"), None);
        assert_eq!(persistable_key("t:0x1^.a^.b"), None);
        assert_eq!(persistable_key("t:0x1^.a.b"), None);
        assert_eq!(persistable_key("t:0x1.a"), None);
        assert_eq!(persistable_key("t:0x1^"), None);
    }
}
