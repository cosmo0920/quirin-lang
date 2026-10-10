//! Card **NULL-MEMBER-INVARIANT-SCOPE** (contract specification C09.3,
//! grammar G05f, gateway rules 14x–14z): the declarations of stable members
//! and function-scope entries as the walk reads them, the stores it records
//! against them, and the key shapes a stable member's fact may persist on.
//! The walk's hooks are in `walk.rs` (`memory_written`, `store`,
//! `record_initializer`, `call`, the entry facts); the decisions are
//! [`qcore_core::null_member_scope`].

use std::collections::BTreeMap;

use qcore_contract::bounds_contract::{BoundsClause, BoundsContracts};

/// A declared stable member (`nonnull_member_stable(TAG, FIELD)`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct StableDecl {
    /// The declaration that carries it.
    pub(super) host: String,
    /// The hold, when the declaration is not in force.
    pub(super) held: Option<String>,
}

/// One function-scope entry (`nonnull_member_scope(TAG, FIELD, P)`).
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
