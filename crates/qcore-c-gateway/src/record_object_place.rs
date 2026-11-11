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

//! Card **CG10-INDIRECT-ACCESS-SOLE-BLOCKERS** (gateway rule 12j): a scalar
//! place selected with `.` only from a record object of the function's own
//! automatic storage, read with no root premise.
//!
//! ## The gap
//!
//! The typed reader ([`crate::typed_access_path::Reader::path`]) admits a place
//! whose first step is `.` only on a root the custody closure proves never
//! bound (rules 10k, 10l, 10o), and the tracked-root route of rule 10t refuses
//! every record-object root (`record-root`). A record object the closure does
//! not prove never bound (its address handed to `memset`, its member name also
//! spelled in an uncontracted call's actual, a member that receives a pointer)
//! was therefore refused at every scalar member read or store, though such an
//! access reads no pointer at all: `heartbeat_total.int_value = current;`,
//! `state.v1 = seed + PRIME32_1;`, `p.len = 1;`.
//!
//! ## The rule ([`decide`])
//!
//! Asked only where the never-bound premise failed, for a place
//!
//! 1. whose **every step is a member selection with `.`** (a union or
//!    bit-field member step is rule 10w's and keeps its decision and frame):
//!    no pointer is read from memory and dereferenced, so the access designates
//!    storage of the root object itself;
//! 2. whose **root is a record object of automatic storage** declared by this
//!    function: a parameter (C passes a record by value) or a block-scope
//!    variable with no storage-class specifier (C11 6.2.4p5: its lifetime is
//!    the block, and a name that designates it is in scope only inside it), read
//!    by its Clang declaration id;
//! 3. whose **access is a scalar read or a scalar store**: the value read or
//!    written is a builtin scalar or an enumeration, never a pointer value, an
//!    address, a pointer-valued store or a whole aggregate.
//!
//! Where it holds the place lowers to no chain and no base: no pointer is read,
//! so there is no dereference obligation, and the store writes only the scalar
//! member's own storage, which no binding of the projection names (bindings
//! name pointer places). It changes no custody, status, marker, field link or
//! binding; a store through a union that can hold a pointer keeps rule 10w's
//! W43 refused-write frame. Everything else keeps the existing refusal:
//! a hop (`->`, a subscript or `*`), a `static`, `extern` or file-scope root,
//! a declaration the function does not own, a pointer-valued, address or
//! aggregate access.
//!
//! ## The interlock
//!
//! The never-bound answer is unchanged: the rule is consulted only after it
//! refused. Nothing older is asked later (hard invariant 3).
//!
//! The Lean model is `QCore.RecordObjectPlace`; its vectors are
//! [`DECISION_VECTORS`].

use serde_json::Value;

use crate::typed_c_ast::{children, kind};

/// The root object's storage, read from its Clang declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RootStorage {
    /// A block-scope `VarDecl` of this function with no storage-class
    /// specifier.
    Automatic,
    /// A `ParmVarDecl` of this function.
    Parameter,
    /// A block-scope `static`, `extern`, `register` or thread-local
    /// declaration of this function.
    Specified,
    /// No declaration of this function has the id: a file-scope object.
    Outside,
}

impl RootStorage {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Automatic => "automatic",
            Self::Parameter => "parameter",
            Self::Specified => "specified",
            Self::Outside => "outside",
        }
    }
}

/// What the access does with the place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Access {
    ScalarRead,
    ScalarStore,
    /// A pointer value read through the place, an address `&PLACE`, a
    /// pointer-valued store or a whole-aggregate read or store.
    Other,
}

impl Access {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::ScalarRead => "scalar-read",
            Self::ScalarStore => "scalar-store",
            Self::Other => "other",
        }
    }
}

/// The rule's answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    Admit,
    /// A step reads a pointer (`->`, a subscript or `*`).
    Hop,
    /// The root is not of this function's automatic storage.
    Storage,
    /// Not a scalar read or store.
    Access,
}

impl Decision {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Admit => "admit",
            Self::Hop => "hop",
            Self::Storage => "storage",
            Self::Access => "access",
        }
    }
}

/// `QCore.RecordObjectPlace.decide`: `dot_only` is whether every step is a
/// `.` member selection.
pub(crate) fn decide(dot_only: bool, storage: RootStorage, access: Access) -> Decision {
    if !dot_only {
        Decision::Hop
    } else if !matches!(storage, RootStorage::Automatic | RootStorage::Parameter) {
        Decision::Storage
    } else if access == Access::Other {
        Decision::Access
    } else {
        Decision::Admit
    }
}

/// The decision vectors shared with `proof/QCore/RecordObjectPlace.lean`
/// (`decisionVectors`): dot-only, storage, access, decision.
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(bool, &str, &str, &str)] = &[
    (true, "automatic", "scalar-store", "admit"),
    (true, "automatic", "scalar-read", "admit"),
    (true, "parameter", "scalar-read", "admit"),
    (true, "parameter", "scalar-store", "admit"),
    (false, "automatic", "scalar-store", "hop"),
    (false, "outside", "other", "hop"),
    (true, "specified", "scalar-store", "storage"),
    (true, "outside", "scalar-read", "storage"),
    (true, "automatic", "other", "access"),
    (true, "parameter", "other", "access"),
];

/// The storage of the declaration `decl` (a Clang declaration id) as
/// `function` (a `FunctionDecl` node) declares it.
pub(crate) fn root_storage(function: &Value, decl: &str) -> RootStorage {
    let mut found = None;
    visit(function, &mut |node| {
        if found.is_none()
            && matches!(kind(node), "ParmVarDecl" | "VarDecl")
            && node["id"].as_str() == Some(decl)
        {
            found = Some(
                match (kind(node), node["storageClass"].as_str(), node.get("tls")) {
                    ("ParmVarDecl", None, None) => RootStorage::Parameter,
                    ("VarDecl", None, None) => RootStorage::Automatic,
                    _ => RootStorage::Specified,
                },
            );
        }
    });
    found.unwrap_or(RootStorage::Outside)
}

fn visit(node: &Value, f: &mut impl FnMut(&Value)) {
    f(node);
    for child in children(node) {
        visit(child, f);
    }
}

/// Parity only: the decision as a `typed-place` place row, into the
/// consultation's own record when its consumer records one (card
/// GAPO-PARITY-DECISIONS-NONDETERMINISTIC, rule 12q).
pub(crate) fn record(
    decisions: Option<&crate::consultation_decisions::Decisions>,
    root: &str,
    storage: RootStorage,
    access: Access,
    decision: Decision,
) {
    let Some(decisions) = decisions else {
        return;
    };
    decisions.place(|| {
        serde_json::json!({
            "rule": "12j",
            "record_object": {
                "name": root,
                "storage": storage.code(),
                "access": access.code(),
            },
            "admitted": decision == Decision::Admit,
            "refusal": (decision != Decision::Admit).then(|| decision.code()),
        })
    });
}

#[cfg(test)]
#[path = "../tests/unit/record_object_place.rs"]
mod tests;
