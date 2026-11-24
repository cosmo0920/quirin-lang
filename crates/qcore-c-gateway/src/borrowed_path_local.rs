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

//! Card **K5-LOCAL-ORIGIN-ALIASES** (gateway rule 77): an automatic local
//! that only ever holds a pointer **loaded** from an untracked object (or
//! null, or a literal) is decided at a null test and at a `return` as the
//! load itself is.
//!
//! ## The gap
//!
//! P2 (W46-ACCESS-PATH-STATEMENTS) lowers `L = R->F` (and `T *L = R->F[i]`,
//! `L = L->next`, ...) on a root no statement binds to a borrowed,
//! target-free alias: no target, `unknown`, possibly null. Every read
//! through it is already admitted: rule 14n reads the binding non-null at a
//! dereference under a named premise, and rule 11d reads it as an untracked
//! root once it is known non-null. But the two decisions taken on the name
//! itself still refused:
//!
//! * a **null test** (`if (!(p = r->_private))`, `while (c != NULL)`) took
//!   rule 10b's arm for an unknown binding: it narrows the binding, keeps the
//!   identity refusal (`UnknownAlias L`) and marks both edges unmodelled, so
//!   every later access through `L` also failed rule 11d's path premise;
//! * a **`return L;`** escaped an unknown binding and reported it, although
//!   `return R->F;` itself is rule 10k (2)'s exit that escapes nothing.
//!
//! The guarded body was therefore `Unknown` where the unguarded one
//! (`n = r->next; return n->count;`) is `Safe`.
//!
//! ## The reading ([`read`])
//!
//! From Clang's AST of the function, a greatest fixed point over the
//! automatic object-pointer locals rule 62 calls automatic (one declaration,
//! automatic storage, neither `volatile` nor `_Atomic`, every reference to
//! that declaration). The never-bound closure is asked of the roots only
//! (it binds every name a written value mentions, a stepped index included):
//!
//! * **Roots.** A pointer formal (single-level, declared once, no local of its
//!   spelling) that rule 5b's full return premise answers for (the custody
//!   closure proves it never bound **and** `may_bind` does not name it), and
//!   whose every reference is a null test, a dereference base, a call actual
//!   or a `return` ([`decide`], [`Subject::Root`]): never written, rebound,
//!   address-taken, copied or used otherwise.
//! * **Values.** Every write of an admitted local (`=` or its initializer)
//!   assigns, through parentheses and `NoOp`/`BitCast` casts, a null pointer
//!   constant, a string literal, a copy of an admitted local, or a **load**:
//!   the lvalue conversion of a member (`->`, `.`) or subscript place of
//!   object-pointer type whose base chain reaches an admitted root or local
//!   through loads, `.` members, subscripts, member-array decays used as a
//!   subscript base, or `(&PLACE)->`. The value contains no call, assignment,
//!   step or statement expression. A whole read of a formal (rule 62's copy),
//!   an address (`&R->F`, an interior address, rule 5j), an array decay as a
//!   value and any other expression withhold the local ([`Value`]).
//! * **Uses.** Every reference of an admitted local is a null test, a
//!   dereference base, a call actual, a `return`, its own rebinding or a copy
//!   into another admitted local; a step, a compound write, `&`, or any other
//!   use withholds it ([`Subject::Local`]).
//!
//! ## Why it is sound
//!
//! By induction over any execution, an admitted local holds null, a literal's
//! address or a pointer loaded from an object reachable from an admitted root
//! through loads (`every_admitted_local_holds_an_untracked_value`), which is
//! exactly the value `R->F` itself has at that point. P2's premise, that a
//! never-bound root reaches no tracked place, is what already lets the load be
//! a target-free alias; this rule rests on the same premise and adds none. The
//! two decisions it changes then take no custody step:
//!
//! * the **null test** ([`guard_reads_untracked`]) is admitted only where the
//!   tested binding has no target, the identity derivation and an unwidened
//!   state: it narrows that binding exactly as the old arm did and nothing
//!   else; no obligation, site or other binding moves
//!   (`the_guard_touches_no_obligation`). Rule 10b's concern, that
//!   non-nullness does not establish a *lookup's* identity, does not arise:
//!   the value is no lookup result (a value from a call withholds the local),
//!   and a binding with no target names no identity to establish;
//! * the **return** ([`return_reads_untracked`]) under the same binding
//!   condition escapes nothing, rule 10k (2)'s answer for `return R->F;`
//!   (W45-RETURN-POINTER-RESULT-READS's "a load through an untracked base
//!   escapes nothing"; an interior address never reaches it).
//!
//! Every custody use of the local (a release, a transfer, an owned or
//! custody-taking position) still finds no target and refuses; a refusal on
//! another path still marks it. The decision table is [`decide`], mirrored in
//! Lean by `QCore.BorrowedPathLocal`.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::custody_free_copy::{
    LITERAL, Use, Visit, classify, object_pointer, single_level_pointer, type_of, visit,
};
use crate::storage_duration::{Storage, atomic, storage_of, top_level_volatile};
use crate::typed_c_ast::{children, kind};

/// What a reference is subject to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Subject {
    /// A pointer formal whose loads the locals may hold.
    Root,
    /// An automatic local holding only loads, null or literals.
    Local,
}

/// The decision for one reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    Admit,
    Withhold,
}

/// The decision table (`QCore.BorrowedPathLocal.decide`).
pub(crate) fn decide(subject: Subject, used: Use) -> Decision {
    match (subject, used) {
        (Subject::Root, Use::NullTest | Use::Dereference | Use::CallArgument | Use::Return) => {
            Decision::Admit
        }
        (Subject::Root, _) => Decision::Withhold,
        (
            Subject::Local,
            Use::CopyToLocal
            | Use::InitLocal
            | Use::NullTest
            | Use::Dereference
            | Use::CallArgument
            | Use::Rebind
            | Use::Return,
        ) => Decision::Admit,
        (Subject::Local, Use::Write | Use::Address | Use::Other) => Decision::Withhold,
    }
}

/// The kind of one value written to a candidate local.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Written {
    /// A null pointer constant.
    Null,
    /// A string literal (a static array's address, C11 6.4.5p6).
    Literal,
    /// A load of an object-pointer member or element on an admitted base.
    Load,
    /// A whole read of another admitted local.
    Copy,
    /// Anything else: a call, an address, a formal's whole value, arithmetic.
    Other,
}

/// Whether a written value keeps the local admitted (`QCore.BorrowedPathLocal.valueAdmits`).
pub(crate) fn value_admits(value: Written) -> bool {
    !matches!(value, Written::Other)
}

/// Every `(subject, use, decision)`, spelled as Lean's `decisionVectors`.
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(&str, &str, &str)] = &[
    ("root", "copyToLocal", "withhold"),
    ("root", "initLocal", "withhold"),
    ("root", "nullTest", "admit"),
    ("root", "dereference", "admit"),
    ("root", "callArgument", "admit"),
    ("root", "rebind", "withhold"),
    ("root", "return", "admit"),
    ("root", "write", "withhold"),
    ("root", "address", "withhold"),
    ("root", "other", "withhold"),
    ("local", "copyToLocal", "admit"),
    ("local", "initLocal", "admit"),
    ("local", "nullTest", "admit"),
    ("local", "dereference", "admit"),
    ("local", "callArgument", "admit"),
    ("local", "rebind", "admit"),
    ("local", "return", "admit"),
    ("local", "write", "withhold"),
    ("local", "address", "withhold"),
    ("local", "other", "withhold"),
];

/// Every value kind and whether it admits, spelled as Lean's `valueVectors`.
#[cfg(test)]
pub(crate) const VALUE_VECTORS: &[(&str, bool)] = &[
    ("null", true),
    ("literal", true),
    ("load", true),
    ("copy", true),
    ("other", false),
];

/// The transfer's binding condition (`QCore.BorrowedPathLocal.untracked`):
/// the tested or returned name's binding has no target, the identity
/// derivation, and the state is not widened.
#[must_use]
pub(crate) fn untracked(targets_empty: bool, identity: bool, widened: bool) -> bool {
    targets_empty && identity && !widened
}

/// The binding vectors (targets empty, identity, widened, untracked).
#[cfg(test)]
pub(crate) const BINDING_VECTORS: &[(bool, bool, bool, bool)] = &[
    (true, true, false, true),
    (false, true, false, false),
    (true, false, false, false),
    (true, true, true, false),
];

/// The admitted locals of one function. Empty wherever nothing is admitted.
pub(crate) type BorrowedPathLocals = BTreeSet<String>;

fn strip_parens(mut node: &Value) -> &Value {
    while kind(node) == "ParenExpr" {
        match children(node).first() {
            Some(child) => node = child,
            None => break,
        }
    }
    node
}

/// Through parentheses and pointer-preserving casts.
fn strip_value(mut node: &Value) -> &Value {
    loop {
        let transparent = kind(node) == "ParenExpr"
            || (matches!(kind(node), "ImplicitCastExpr" | "CStyleCastExpr")
                && matches!(node["castKind"].as_str(), Some("NoOp" | "BitCast")));
        if !transparent {
            return node;
        }
        match children(node).first() {
            Some(child) => node = child,
            None => return node,
        }
    }
}

/// No call, assignment, step or statement expression anywhere below.
fn pure(node: &Value) -> bool {
    let impure = match kind(node) {
        "CallExpr" | "StmtExpr" | "CompoundAssignOperator" => true,
        "BinaryOperator" => node["opcode"].as_str() == Some("="),
        "UnaryOperator" => matches!(node["opcode"].as_str(), Some("++" | "--")),
        _ => false,
    };
    !impure && children(node).iter().all(pure)
}

struct Names<'a> {
    roots: &'a BTreeSet<String>,
    locals: &'a BTreeSet<String>,
}

impl Names<'_> {
    fn declaration(node: &Value) -> Option<(&str, &str)> {
        if kind(node) != "DeclRefExpr" {
            return None;
        }
        let declaration = &node["referencedDecl"];
        Some((declaration["name"].as_str()?, declaration["kind"].as_str()?))
    }

    /// A pointer value a member arrow or a subscript may dereference.
    fn base_value(&self, node: &Value) -> bool {
        let node = strip_value(node);
        match (kind(node), node["castKind"].as_str()) {
            ("ImplicitCastExpr", Some("LValueToRValue")) => {
                let Some(place) = children(node).first().map(strip_parens) else {
                    return false;
                };
                match Self::declaration(place) {
                    Some((name, "ParmVarDecl")) => self.roots.contains(name),
                    Some((name, "VarDecl")) => self.locals.contains(name),
                    Some(_) => false,
                    None => self.load(node, place),
                }
            }
            ("ImplicitCastExpr", Some("ArrayToPointerDecay")) => children(node)
                .first()
                .is_some_and(|place| self.object_place(strip_parens(place))),
            ("UnaryOperator", _) if node["opcode"].as_str() == Some("&") => children(node)
                .first()
                .is_some_and(|place| self.object_place(strip_parens(place))),
            _ => false,
        }
    }

    /// A member or element place whose base reaches an admitted name.
    fn object_place(&self, node: &Value) -> bool {
        match kind(node) {
            "MemberExpr" => {
                let Some(base) = children(node).first() else {
                    return false;
                };
                if node["isArrow"].as_bool() == Some(true) {
                    self.base_value(base)
                } else {
                    self.object_place(strip_parens(base))
                }
            }
            "ArraySubscriptExpr" => match children(node) {
                [base, index] => self.base_value(base) && pure(index),
                _ => false,
            },
            _ => false,
        }
    }

    /// A load: the lvalue conversion `conversion` of a member or element
    /// `place`, whose value (unqualified, C11 6.3.2.1p2) is an object pointer.
    fn load(&self, conversion: &Value, place: &Value) -> bool {
        type_of(conversion).is_some_and(object_pointer) && self.object_place(place)
    }

    /// The kind of one written value.
    fn value(&self, node: &Value) -> Written {
        if !pure(node) {
            return Written::Other;
        }
        let stripped = strip_value(node);
        match (kind(stripped), stripped["castKind"].as_str()) {
            (_, Some("NullToPointer")) => Written::Null,
            ("ImplicitCastExpr", Some("ArrayToPointerDecay"))
                if children(stripped)
                    .first()
                    .is_some_and(|child| kind(strip_parens(child)) == "StringLiteral") =>
            {
                Written::Literal
            }
            ("ImplicitCastExpr", Some("LValueToRValue")) => {
                let Some(place) = children(stripped).first().map(strip_parens) else {
                    return Written::Other;
                };
                match Self::declaration(place) {
                    Some((name, "VarDecl")) if self.locals.contains(name) => Written::Copy,
                    Some(_) => Written::Other,
                    None if self.load(stripped, place) => Written::Load,
                    None => Written::Other,
                }
            }
            _ => Written::Other,
        }
    }
}

struct Reference<'a> {
    at: usize,
    id: &'a str,
}

/// Rule 77's reading for one function. `pointer_root` is rule 5b's return
/// premise of a typed position (the custody closure proves the name never
/// bound and `may_bind` does not name it), asked of a formal; a formal it does
/// not answer for is no root. A local is not asked the closure: the closure
/// binds every name a written value mentions (an index `i` stepped by the
/// loop binds `c = a->kids[i]`), while what makes a local's value untracked is
/// read here directly (every write a load, null, literal or admitted copy),
/// and every custody use of it is decided on its binding, which has no target
/// (a custody-taking actual refuses at the call, as rule 11d records).
pub(crate) fn read(function: &Value, pointer_root: &dyn Fn(&str) -> bool) -> BorrowedPathLocals {
    let mut visits: Vec<Visit<'_>> = Vec::new();
    visit(function, &mut Vec::new(), &mut visits);
    let mut parameters: BTreeMap<&str, Vec<&Value>> = BTreeMap::new();
    let mut variables: BTreeMap<&str, Vec<&Value>> = BTreeMap::new();
    for visit in &visits {
        let node = visit.node;
        let Some(name) = node["name"].as_str() else {
            continue;
        };
        match kind(node) {
            "ParmVarDecl" => parameters.entry(name).or_default().push(node),
            "VarDecl" => variables.entry(name).or_default().push(node),
            _ => {}
        }
    }
    let mut references: BTreeMap<&str, Vec<Reference<'_>>> = BTreeMap::new();
    for (at, visit) in visits.iter().enumerate() {
        let node = visit.node;
        if kind(node) != "DeclRefExpr" {
            continue;
        }
        let declaration = &node["referencedDecl"];
        let (Some(name), Some(id)) = (declaration["name"].as_str(), declaration["id"].as_str())
        else {
            continue;
        };
        references
            .entry(name)
            .or_default()
            .push(Reference { at, id });
    }
    // A literal placeholder spelled by the function makes route G's literal
    // ambiguous: nothing is read then.
    if references.contains_key(LITERAL)
        || parameters.contains_key(LITERAL)
        || variables.contains_key(LITERAL)
    {
        return BorrowedPathLocals::new();
    }
    let single = |name: &str, declaration: &Value| {
        declaration["id"].as_str().is_some_and(|id| {
            references
                .get(name)
                .into_iter()
                .flatten()
                .all(|reference| reference.id == id)
        })
    };
    // Roots: formals whose every reference is a read the table admits.
    let roots: BTreeSet<String> = parameters
        .iter()
        .filter(|(name, declarations)| {
            let [declaration] = declarations.as_slice() else {
                return false;
            };
            !variables.contains_key(*name)
                && type_of(declaration).is_some_and(single_level_pointer)
                && single(name, declaration)
                && pointer_root(name)
                && references
                    .get(*name)
                    .into_iter()
                    .flatten()
                    .all(|reference| {
                        let (used, _) = classify(&visits[reference.at], &|_| None);
                        decide(Subject::Root, used) == Decision::Admit
                    })
        })
        .map(|(name, _)| (*name).to_owned())
        .collect();
    // Candidates: automatic object-pointer locals, one declaration each.
    let mut locals: BTreeSet<String> = variables
        .iter()
        .filter(|(name, declarations)| {
            let [declaration] = declarations.as_slice() else {
                return false;
            };
            !parameters.contains_key(*name)
                && storage_of(declaration, true) == Some(Storage::Automatic)
                && type_of(declaration)
                    .is_some_and(|ty| object_pointer(ty) && !top_level_volatile(ty) && !atomic(ty))
                && single(name, declaration)
        })
        .map(|(name, _)| (*name).to_owned())
        .collect();
    // The greatest fixed point: drop a local any of whose writes or uses the
    // current set does not admit, until nothing changes.
    loop {
        let names = Names {
            roots: &roots,
            locals: &locals,
        };
        let copies_into = |node: &Value| -> Option<String> {
            let name = match kind(node) {
                "DeclRefExpr" if node["referencedDecl"]["kind"].as_str() == Some("VarDecl") => {
                    node["referencedDecl"]["name"].as_str()?
                }
                "VarDecl" => node["name"].as_str()?,
                _ => return None,
            };
            locals.contains(name).then(|| name.to_owned())
        };
        let withheld: Vec<String> = locals
            .iter()
            .filter(|name| {
                let declaration = variables[name.as_str()][0];
                let initializer_admits = declaration.get("init").is_none()
                    || children(declaration)
                        .last()
                        .is_some_and(|value| value_admits(names.value(value)));
                if !initializer_admits {
                    return true;
                }
                references
                    .get(name.as_str())
                    .into_iter()
                    .flatten()
                    .any(|reference| {
                        let at = &visits[reference.at];
                        let (used, _) = classify(at, &copies_into);
                        if decide(Subject::Local, used) == Decision::Withhold {
                            return true;
                        }
                        if used != Use::Rebind {
                            return false;
                        }
                        // The rebinding's assigned value.
                        let Some(assignment) = at.ancestors.last() else {
                            return true;
                        };
                        !children(assignment)
                            .get(1)
                            .is_some_and(|value| value_admits(names.value(value)))
                    })
            })
            .cloned()
            .collect();
        if withheld.is_empty() {
            return locals;
        }
        for name in withheld {
            locals.remove(&name);
        }
    }
}

/// The null-test decision (`QCore.BorrowedPathLocal.guard`): an admitted
/// local whose incoming binding is untracked is narrowed with no identity
/// refusal.
#[must_use]
pub(crate) fn guard_reads_untracked(admitted: bool, untracked_binding: bool) -> bool {
    admitted && untracked_binding
}

/// The return decision (`QCore.BorrowedPathLocal.returned`): an admitted
/// local whose binding is untracked escapes nothing.
#[must_use]
pub(crate) fn return_reads_untracked(admitted: bool, untracked_binding: bool) -> bool {
    admitted && untracked_binding
}

#[cfg(test)]
#[path = "../tests/unit/borrowed_path_local.rs"]
mod tests;
