//! A supplied object a callback takes depending on its return code (card
//! **SUPPLIED-OWNED-RETURN-CODE-HANDOFF**; contract spec §C04.19.5, grammar
//! G05d, gateway rules 12m and 13l).
//!
//! USER DECISIONS 2026-10-15, verbatim in the spec:
//!
//! ```text
//! quant live #rko: #owner |-> #callback when #result != @RD_KAFKA_OP_RES_PASS;
//! alias #PASS = @RD_KAFKA_OP_RES_PASS;
//! quant live #rko: #owner |-> #callback when #result != #PASS;
//! ```
//!
//! * `#` marks binders (`#rko`, `#result`) and value keywords (`#absent`,
//!   `#owner`, `#callback`); the clause-kind word after `quant` stays bare.
//! * `@NAME` is a C enumerator Clang resolves: an enumerator of the guarded
//!   value's own enum type, its value pinned at both revisions.
//! * An alias is module-local, declared once, its right-hand side an `@`
//!   name; both spellings load to one canonical [`HandOffGuard`].
//!
//! Meaning: the object the formal `#rko` designates is owned by the callback
//! after the call exactly when the guard holds of the call's result, and
//! stays with the caller otherwise ([`owner_after`]). The clause elaborates
//! into the formal's liveness part (`quant live #rko;`, backed by
//! `resource_owned(rko, nonnull)`) and the guarded transfer.

use serde::{Deserialize, Serialize};

/// The value keywords and the result binder a `#` form reserves: a
/// declaration carrying a `#` form rejects a parameter so named, and an
/// alias may not take one of these names.
///
/// Card **OP-CALLBACK-MEMBER-PATH-RELEASE** adds the member-path states
/// `released`, `moved`, `stale` and `kept` (`crate::member_path`); card
/// **MK-EVENT-DESCRIPTOR-QUANT** the cell words `fresh`, `same` and `pointee`
/// (`crate::descriptor_cell`).
pub const RESERVED_KEYWORDS: [&str; 11] = [
    "absent", "owner", "callback", "result", "released", "moved", "stale", "kept", "fresh", "same",
    "pointee",
];

/// How a guard compares the call's result with its constant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardOp {
    /// `#result == @K`.
    Eq,
    /// `#result != @K`.
    Ne,
}

impl GuardOp {
    /// The written operator.
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Eq => "==",
            Self::Ne => "!=",
        }
    }
}

/// `when #result OP @K`: the guard of a hand-off, its constant canonical
/// (the C enumerator's name, an alias already resolved).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HandOffGuard {
    pub op: GuardOp,
    /// The C enumerator, without its `@`.
    pub constant: String,
}

impl HandOffGuard {
    /// Whether the guard holds of `result` once its constant resolved to
    /// `value`.
    #[must_use]
    pub fn holds(&self, result: i64, value: i64) -> bool {
        match self.op {
            GuardOp::Eq => result == value,
            GuardOp::Ne => result != value,
        }
    }

    /// The canonical text, `#result OP @K`.
    #[must_use]
    pub fn spelling(&self) -> String {
        format!("#result {} @{}", self.op.spelling(), self.constant)
    }
}

/// `quant live #P: #owner |-> #callback when GUARD;`: the parameter at
/// `parameter` is the callback's after the call exactly when `guard` holds.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HandOff {
    pub parameter: usize,
    pub guard: HandOffGuard,
}

/// Who owns the handed object after the call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Owner {
    /// The caller keeps it (it keeps or disposes of it).
    Caller,
    /// The callback took it: the caller never touches it again.
    Callback,
}

/// Ownership after the call is a function of the result and the guard
/// (`QCore.ReturnCodeHandOff.owner_after`).
#[must_use]
pub fn owner_after(guard: &HandOffGuard, result: i64, value: i64) -> Owner {
    if guard.holds(result, value) {
        Owner::Callback
    } else {
        Owner::Caller
    }
}

/// `OWNER_VECTORS`: (op is `!=`, result, constant value, the callback owns),
/// shared with `QCore.ReturnCodeHandOff.ownerVectors`.
pub const OWNER_VECTORS: &[(bool, i64, i64, bool)] = &[
    (true, 0, 0, false),
    (true, 1, 0, true),
    (true, 2, 0, true),
    (true, 3, 0, true),
    (false, 2, 2, true),
    (false, 1, 2, false),
    (false, 0, 2, false),
    (false, 3, 2, false),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_vectors_hold() {
        for &(ne, result, value, callback) in OWNER_VECTORS {
            let guard = HandOffGuard {
                op: if ne { GuardOp::Ne } else { GuardOp::Eq },
                constant: "K".into(),
            };
            assert_eq!(
                owner_after(&guard, result, value) == Owner::Callback,
                callback,
                "{ne} {result} {value}"
            );
        }
    }
}
