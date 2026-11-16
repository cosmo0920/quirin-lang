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

//! Card **THREAD-LOCAL-CUSTODY-FREE-READ** (gateway rule 54): a whole read of
//! a thread-local pointer object is a custody-free value, the reading
//! `pthread_getspecific`'s result already gets.
//!
//! ## The gap
//!
//! Rule 11h (3) says a static- or thread-duration object's value carries no
//! identity, and route G kept that only where the read was handed straight
//! out (`return t_ctx;` was already `Safe`). A copy of it into a local
//! (`evl = flb_engine_evl; return evl;`, Fluent Bit's `FLB_TLS_GET` with
//! `FLB_HAVE_C_TLS`) took the generic alias route: the unbound source was
//! reported as an unresolved identity and the local became an unknown,
//! possibly null binding, so a getter that only returns the value was
//! `Unknown`. The `pthread_getspecific` build of the same getter is `Safe`:
//! that call's contract is `resource_return_none()`, and route G drops the
//! destination's binding ([`ResourceReturnMode::None`]).
//!
//! ## The reading ([`admitted`])
//!
//! For one function, a name is a **thread-local object** when every
//! `DeclRefExpr` spelling it references a thread-local `VarDecl` (a
//! file-scope one, [`declares_thread_local`], or a block-scope one) and no
//! other declaration of the function spells it. Its **copies** are the
//! automatic locals (block-scope, no storage-class specifier, not thread-local,
//! not a parameter, every declaration of the name automatic) that receive its
//! value by a plain `=` or an initializer, closed under copying between such
//! locals. The reading is admitted for the whole function only when
//!
//! 1. every object is of object-pointer type (not an array, a record, a
//!    function pointer, `volatile` or `_Atomic`) at every reference;
//! 2. every reference to an object is a whole read (lvalue conversion,
//!    through parentheses and pointer-preserving casts) consumed by a plain
//!    `=` into a copy, a copy's initializer, a `return` operand or a null
//!    test ([`Use::CopyToLocal`] .. [`Use::NullTest`]); a write to it, `&` of
//!    it, a `.` member of it, a dereference of it, a call argument or any
//!    other use withholds the reading;
//! 3. every reference to a copy is the target of a plain `=` or a whole read
//!    consumed in the same four positions; a dereference, a call argument
//!    (a release, a transfer or a store through it), an arithmetic use or any
//!    other use withholds the reading.
//!
//! Where admitted, route G's alias transfer of `COPY = OBJECT` and of
//! `COPY = COPY'` from an unbound source drops the destination's binding
//! (`legacy_transfer`), exactly as `pthread_getspecific`'s `return_none`
//! result does: no obligation is created or discharged, the value is borrowed,
//! and every custody use of it refuses (an unbound release or escape finds no
//! binding). Where withheld, every alias keeps its unresolved-identity answer.
//! The reading states no ownership and no teardown: a caller that releases
//! the returned value still answers for it (an unbound release refuses).
//!
//! The Lean model is `QCore.ThreadLocalRead`, bound to [`DECISION_VECTORS`]
//! by a shared-vector test.
//!
//! [`ResourceReturnMode::None`]: qcore_contract::ResourceReturnMode::None

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::storage_duration::{Storage, atomic, storage_of, top_level_volatile};
use crate::typed_c_ast::{children, kind};

/// The objects and copies of one function where rule 54 admits the reading;
/// both empty where it is withheld.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ThreadLocalReads {
    /// Thread-local pointer objects whose whole read is a custody-free value.
    pub objects: BTreeSet<String>,
    /// Automatic locals that hold only such values or their own rebindings.
    pub copies: BTreeSet<String>,
}

impl ThreadLocalReads {
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty() && self.copies.is_empty()
    }

    /// Whether `COPY = SOURCE` from an unbound source is a custody-free copy.
    pub(crate) fn admits_copy(&self, destination: &str, source: &str) -> bool {
        self.copies.contains(destination)
            && (self.objects.contains(source) || self.copies.contains(source))
    }
}

/// Whether a top-level Clang declaration is a thread-local object.
pub(crate) fn declares_thread_local(node: &Value) -> bool {
    kind(node) == "VarDecl" && node["tls"].as_str().is_some()
}

/// What a reference is subject to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Subject {
    /// The thread-local object itself.
    Object,
    /// An automatic local that receives its value.
    Copy,
}

/// How one reference is used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Use {
    /// A whole read assigned by a plain `=` to an automatic local.
    CopyToLocal,
    /// A whole read initializing an automatic local.
    InitLocal,
    /// A whole read returned.
    Return,
    /// A whole read compared, negated or tested as a condition.
    NullTest,
    /// The target of a plain `=`.
    Rebind,
    /// A compound assignment or an increment or decrement.
    Write,
    /// `&` of it, or an array decaying.
    Address,
    /// A `.` or `->` member selection on it, or a dereference.
    Dereference,
    /// A call actual.
    CallArgument,
    /// The object is not of object-pointer type, or is `volatile`/`_Atomic`.
    NotObjectPointer,
    /// Anything else.
    Other,
}

/// The decision for one reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    Admit,
    Withhold,
}

/// The decision table (`QCore.ThreadLocalRead.decide`).
pub(crate) fn decide(subject: Subject, used: Use) -> Decision {
    match (subject, used) {
        (_, Use::CopyToLocal | Use::InitLocal | Use::Return | Use::NullTest)
        | (Subject::Copy, Use::Rebind) => Decision::Admit,
        _ => Decision::Withhold,
    }
}

/// Every `(subject, use, decision)`, spelled as Lean's `decisionVectors`
/// (read by the unit tests, and as text by the shared-vector test).
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(&str, &str, &str)] = &[
    ("object", "copyToLocal", "admit"),
    ("object", "initLocal", "admit"),
    ("object", "return", "admit"),
    ("object", "nullTest", "admit"),
    ("object", "rebind", "withhold"),
    ("object", "write", "withhold"),
    ("object", "address", "withhold"),
    ("object", "dereference", "withhold"),
    ("object", "callArgument", "withhold"),
    ("object", "notObjectPointer", "withhold"),
    ("object", "other", "withhold"),
    ("copy", "copyToLocal", "admit"),
    ("copy", "initLocal", "admit"),
    ("copy", "return", "admit"),
    ("copy", "nullTest", "admit"),
    ("copy", "rebind", "admit"),
    ("copy", "write", "withhold"),
    ("copy", "address", "withhold"),
    ("copy", "dereference", "withhold"),
    ("copy", "callArgument", "withhold"),
    ("copy", "notObjectPointer", "withhold"),
    ("copy", "other", "withhold"),
];

/// The spelling of a subject and a use in [`DECISION_VECTORS`].
#[cfg(test)]
pub(crate) fn spelled(subject: Subject, used: Use) -> (&'static str, &'static str) {
    let subject = match subject {
        Subject::Object => "object",
        Subject::Copy => "copy",
    };
    let used = match used {
        Use::CopyToLocal => "copyToLocal",
        Use::InitLocal => "initLocal",
        Use::Return => "return",
        Use::NullTest => "nullTest",
        Use::Rebind => "rebind",
        Use::Write => "write",
        Use::Address => "address",
        Use::Dereference => "dereference",
        Use::CallArgument => "callArgument",
        Use::NotObjectPointer => "notObjectPointer",
        Use::Other => "other",
    };
    (subject, used)
}

/// Every subject and use, for the table test.
#[cfg(test)]
pub(crate) const ALL: [(Subject, Use); 22] = {
    const USES: [Use; 11] = [
        Use::CopyToLocal,
        Use::InitLocal,
        Use::Return,
        Use::NullTest,
        Use::Rebind,
        Use::Write,
        Use::Address,
        Use::Dereference,
        Use::CallArgument,
        Use::NotObjectPointer,
        Use::Other,
    ];
    let mut out = [(Subject::Object, Use::Other); 22];
    let mut i = 0;
    while i < 11 {
        out[i] = (Subject::Object, USES[i]);
        out[i + 11] = (Subject::Copy, USES[i]);
        i += 1;
    }
    out
};

/// Whether a Clang type spelling is an object pointer: a `*` declarator at
/// the top level, no function or array declarator, neither `volatile` nor
/// `_Atomic` at its top level.
fn object_pointer(text: &str) -> bool {
    let text = text.trim();
    text.ends_with('*')
        && !text.contains('(')
        && !text.contains('[')
        && !top_level_volatile(text)
        && !atomic(text)
}

/// One node with the chain of its ancestors inside the function.
struct Visit<'a> {
    node: &'a Value,
    ancestors: Vec<&'a Value>,
}

fn visit<'a>(node: &'a Value, ancestors: &mut Vec<&'a Value>, out: &mut Vec<Visit<'a>>) {
    out.push(Visit {
        node,
        ancestors: ancestors.clone(),
    });
    ancestors.push(node);
    for child in children(node) {
        visit(child, ancestors, out);
    }
    ancestors.pop();
}

/// Whether `child` is the first child of `parent` (by identity).
fn first_child(parent: &Value, child: &Value) -> bool {
    children(parent)
        .first()
        .is_some_and(|first| std::ptr::eq(first, child))
}

fn last_child(parent: &Value, child: &Value) -> bool {
    children(parent)
        .last()
        .is_some_and(|last| std::ptr::eq(last, child))
}

/// The consumer of a value: the first ancestor that is not a parenthesis or a
/// pointer-preserving cast, with the value as that ancestor's child.
fn consumer<'a>(mut value: &'a Value, ancestors: &[&'a Value]) -> Option<(&'a Value, &'a Value)> {
    for parent in ancestors.iter().rev() {
        let transparent = kind(parent) == "ParenExpr"
            || (matches!(kind(parent), "ImplicitCastExpr" | "CStyleCastExpr")
                && matches!(parent["castKind"].as_str(), Some("NoOp" | "BitCast")));
        if !transparent {
            return Some((parent, value));
        }
        value = parent;
    }
    None
}

/// The use one reference makes, with the automatic local it copies into.
fn classify(
    visit: &Visit<'_>,
    copies_into: &dyn Fn(&Value) -> Option<String>,
) -> (Use, Option<String>) {
    let reference = visit.node;
    let Some(parent) = visit.ancestors.last().copied() else {
        return (Use::Other, None);
    };
    match (kind(parent), parent["opcode"].as_str()) {
        ("UnaryOperator", Some("&")) => return (Use::Address, None),
        ("UnaryOperator", Some("++" | "--")) => return (Use::Write, None),
        ("MemberExpr", _) => return (Use::Dereference, None),
        ("ImplicitCastExpr", _) if parent["castKind"].as_str() == Some("ArrayToPointerDecay") => {
            return (Use::Address, None);
        }
        ("BinaryOperator", Some("=")) if first_child(parent, reference) => {
            return (Use::Rebind, None);
        }
        ("CompoundAssignOperator", _) if first_child(parent, reference) => {
            return (Use::Write, None);
        }
        _ => {}
    }
    if !(kind(parent) == "ImplicitCastExpr"
        && parent["castKind"].as_str() == Some("LValueToRValue"))
    {
        return (Use::Other, None);
    }
    let ancestors = &visit.ancestors[..visit.ancestors.len() - 1];
    let Some((user, value)) = consumer(parent, ancestors) else {
        return (Use::Other, None);
    };
    match (kind(user), user["opcode"].as_str()) {
        ("BinaryOperator", Some("=")) if last_child(user, value) => {
            match children(user).first().and_then(copies_into) {
                Some(local) => (Use::CopyToLocal, Some(local)),
                None => (Use::Other, None),
            }
        }
        ("VarDecl", _) => match copies_into(user) {
            Some(local) => (Use::InitLocal, Some(local)),
            None => (Use::Other, None),
        },
        ("ReturnStmt", _) => (Use::Return, None),
        ("BinaryOperator", Some("==" | "!=" | "&&" | "||")) | ("UnaryOperator", Some("!")) => {
            (Use::NullTest, None)
        }
        ("IfStmt" | "WhileStmt" | "ConditionalOperator", _) if first_child(user, value) => {
            (Use::NullTest, None)
        }
        ("DoStmt", _) if last_child(user, value) => (Use::NullTest, None),
        ("MemberExpr" | "ArraySubscriptExpr", _) | ("UnaryOperator", Some("*")) => {
            (Use::Dereference, None)
        }
        ("CallExpr", _) => (Use::CallArgument, None),
        _ => (Use::Other, None),
    }
}

/// Every reference to a variable in `function`, by name: `(referenced id,
/// type spelling)` per reference.
type References<'a> = BTreeMap<String, Vec<(usize, Option<&'a str>, &'a str)>>;

/// Rule 54's reading for one function. `thread_locals` holds the unit's
/// file-scope thread-local declaration ids ([`declares_thread_local`]).
/// Empty unless the function reads a thread-local object and every condition
/// of the module documentation holds.
pub(crate) fn admitted(function: &Value, thread_locals: &BTreeSet<String>) -> ThreadLocalReads {
    let mut visits = Vec::new();
    visit(function, &mut Vec::new(), &mut visits);
    // Declarations of the function, by name: whether each is automatic,
    // thread-local, or anything else; and the ids of the thread-local ones.
    let mut local_tls: BTreeSet<&str> = BTreeSet::new();
    let mut declared: BTreeMap<&str, Vec<Option<Storage>>> = BTreeMap::new();
    for visit in &visits {
        let node = visit.node;
        let Some(name) = node["name"].as_str() else {
            continue;
        };
        match kind(node) {
            "ParmVarDecl" => declared.entry(name).or_default().push(None),
            "VarDecl" => {
                let storage = storage_of(node, true);
                if storage == Some(Storage::ThreadLocal)
                    && let Some(id) = node["id"].as_str()
                {
                    local_tls.insert(id);
                }
                declared.entry(name).or_default().push(storage);
            }
            _ => {}
        }
    }
    let is_tls_id = |id: &str| thread_locals.contains(id) || local_tls.contains(id);
    let automatic = |name: &str| {
        declared.get(name).is_some_and(|storages| {
            !storages.is_empty()
                && storages
                    .iter()
                    .all(|storage| *storage == Some(Storage::Automatic))
        })
    };
    let copies_into = |node: &Value| -> Option<String> {
        let name = match kind(node) {
            "DeclRefExpr" if node["referencedDecl"]["kind"].as_str() == Some("VarDecl") => {
                node["referencedDecl"]["name"].as_str()?
            }
            "VarDecl" => node["name"].as_str()?,
            _ => return None,
        };
        automatic(name).then(|| name.to_owned())
    };
    // Every variable reference, by name.
    let mut references: References<'_> = BTreeMap::new();
    for (at, visit) in visits.iter().enumerate() {
        let node = visit.node;
        if kind(node) != "DeclRefExpr"
            || !matches!(
                node["referencedDecl"]["kind"].as_str(),
                Some("VarDecl" | "ParmVarDecl")
            )
        {
            continue;
        }
        let (Some(name), Some(id)) = (
            node["referencedDecl"]["name"].as_str(),
            node["referencedDecl"]["id"].as_str(),
        ) else {
            continue;
        };
        references.entry(name.to_owned()).or_default().push((
            at,
            node["type"]["qualType"].as_str(),
            id,
        ));
    }
    // The thread-local objects: every reference to the name is to a
    // thread-local declaration, and no other declaration spells it.
    let objects: BTreeSet<String> = references
        .iter()
        .filter(|(name, uses)| {
            uses.iter().all(|(_, _, id)| is_tls_id(id))
                && declared.get(name.as_str()).is_none_or(|storages| {
                    storages
                        .iter()
                        .all(|storage| *storage == Some(Storage::ThreadLocal))
                })
        })
        .map(|(name, _)| name.clone())
        .collect();
    if objects.is_empty() {
        return ThreadLocalReads::default();
    }
    let mut decisions: Vec<(Subject, Use)> = Vec::new();
    let mut copies: BTreeSet<String> = BTreeSet::new();
    let mut frontier: Vec<String> = Vec::new();
    for name in &objects {
        for (at, qual, _) in &references[name] {
            if !qual.is_some_and(object_pointer) {
                decisions.push((Subject::Object, Use::NotObjectPointer));
                continue;
            }
            let (used, local) = classify(&visits[*at], &copies_into);
            decisions.push((Subject::Object, used));
            frontier.extend(local);
        }
    }
    while let Some(name) = frontier.pop() {
        if objects.contains(&name) || !copies.insert(name.clone()) {
            continue;
        }
        for (at, _, _) in references.get(&name).into_iter().flatten() {
            let (used, local) = classify(&visits[*at], &copies_into);
            decisions.push((Subject::Copy, used));
            frontier.extend(local);
        }
    }
    if decisions
        .iter()
        .all(|(subject, used)| decide(*subject, *used) == Decision::Admit)
    {
        ThreadLocalReads { objects, copies }
    } else {
        ThreadLocalReads::default()
    }
}

#[cfg(test)]
#[path = "../tests/unit/thread_local_read.rs"]
mod tests;
