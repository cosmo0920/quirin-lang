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

//! Card **AG2-UNRESOLVED-ALIAS** (gateway rule 62): copy sources that carry
//! no resource identity of their own.
//!
//! ## The gap
//!
//! Route G lowers `D = S;` (and `T *D = S;`) to one `Alias` node. When `S`
//! has no binding, the generic transfer reports `S` as an *unresolved
//! resource identity* and gives `D` an unknown, possibly-null binding. Rule
//! W43-INDEX-TRAVERSAL already leaves `D` unbound, with no report, when both
//! names are declared scalars or declared non-formal pointer objects; rule 54
//! does the same for a copy of a thread-local pointer object. Three copy
//! sources still took the generic arm in the user's `current` run although
//! none of them can be a resource identity:
//!
//! 1. **A string or character literal** (`s = "Info";`). The tokenizer
//!    collapses every literal to `__qcore_literal__`; the value is the address
//!    of a static array (C11 6.4.5p6) or an `int` (6.4.4.4p10), never an
//!    object a statement of this body acquired.
//! 2. **An enumeration constant** (`rd_kafka_resp_err_t err =
//!    RD_KAFKA_RESP_ERR_NO_ERROR;`). An enumerator is an integer constant
//!    (6.4.4.3p2, 6.7.2.2p3); it designates no object at all. The traversal
//!    exemption missed it because an enumerator is not a declared variable,
//!    and the declaration reader does not classify a typedef of an anonymous
//!    `enum` as a scalar.
//! 3. **A pointer formal no statement binds, copied into a local that is only
//!    read through** (`struct ctx *c = (struct ctx *) context;` in a
//!    callback, `const T *a = _a;` in a comparator). W43-INDEX-TRAVERSAL kept
//!    the unresolved alias for a formal's copy because W4 summary derivation
//!    binds exactly the formals an analysis reports
//!    (`W43-INDEX-TRAVERSAL-FORMAL-COPY`). This module admits the copy as
//!    unbound only where no custody use of the copy can need that binding.
//!
//! ## The reading ([`read`])
//!
//! For one function, from Clang's AST of its body:
//!
//! * **Destinations.** Every reading writes only an *automatic local*: a name
//!   with exactly one declaration in the function, a block-scope `VarDecl`
//!   with automatic storage of a type that is neither `volatile` nor
//!   `_Atomic` at its top level, no parameter of that spelling, and every
//!   reference to the spelling designating that declaration (the projection
//!   names objects by spelling, so a shadowed name is refused).
//! * **Literals** ([`Reading::Literal`]): admitted unless some reference of
//!   the function names an object spelled `__qcore_literal__` (the placeholder
//!   would then be ambiguous).
//! * **Enumeration constants** ([`Reading::Constant`]): a spelling every
//!   reference of which is a `DeclRefExpr` to an `EnumConstantDecl`, and that
//!   no declaration of the function spells.
//! * **Formal copies** ([`Reading::FormalCopy`]): a pointer formal `F`
//!   (single-level object-pointer type, not `T **`, declared once, no other declaration of its
//!   spelling) that is never written (`=`, a compound assignment, `++`/`--`)
//!   nor address-taken, and that the never-bound premise (rule 5b's closure
//!   joined with rule 10l's) answers for; its **copies** are the automatic
//!   locals of object-pointer type that receive its whole value by a plain
//!   `=` or an initializer (through parentheses and pointer-preserving
//!   casts), closed under copying between such locals. Every reference to a
//!   copy must be the target of a plain `=` or a whole read consumed as a
//!   copy into another copy, a null test, the base of a member, subscript or
//!   `*` dereference, or a call actual; and the never-bound closure must
//!   answer for every copy. A `return` of a copy, `&` of it, a compound write
//!   or step, a store of its value anywhere but into another copy, or any
//!   other use withholds the reading for that formal and all its copies.
//!
//! ## Why each reading is sound
//!
//! The transfer (`legacy_transfer`) applies a reading only to an `Alias` node
//! whose source has **no binding** on the incoming state, and its whole
//! effect is to leave the destination unbound, which is exactly what
//! W43-INDEX-TRAVERSAL and rule 54 already do for their copies. An unbound
//! name carries no identity: no obligation is created or discharged, and
//! every custody use of it (a release, an escape, an owned or custody-taking
//! position) finds no binding and refuses. Nothing is admitted that the
//! source itself would not get:
//!
//! * a literal or an enumerator *cannot* be bound by any statement, so its
//!   value is custody-free by C's semantics, not by a premise;
//! * an unbound formal's value is the object the caller handed in, which no
//!   statement of this body bound; its copy denotes the same object and gets
//!   the same answer (unbound) at every later use, decided by the same rules
//!   on the copy's own name (the never-bound premise is required of every
//!   copy, so a read through it is admitted only where a read through the
//!   formal would be).
//!
//! The withheld uses are those where the old answer is still needed: a
//! custody use of a copy is refused either way, but W4 summary derivation
//! must still see the formal named, so that it can retry with the formal
//! bound (`legacy_summary::bind_unresolved_formals`); a `return` of a copy is
//! a formal handed straight back, which only rule 36 decides. Where the
//! reading is withheld every alias keeps its unresolved-identity answer.
//!
//! The decision table is [`decide`], mirrored in Lean by
//! `QCore.CustodyFreeCopy`.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::storage_duration::{Storage, atomic, storage_of, top_level_volatile};
use crate::typed_c_ast::{children, kind};

/// The placeholder the tokenizer gives every string and character literal.
pub(crate) const LITERAL: &str = "__qcore_literal__";

/// The custody-free copy sources of one function, and the automatic locals
/// they may be copied into. Empty wherever nothing is admitted.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CustodyFreeCopies {
    /// Whether a literal source (`__qcore_literal__`) is unambiguous here.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub literals: bool,
    /// Enumeration constants the function references.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub constants: BTreeSet<String>,
    /// Pointer formals whose copies are only read through.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub formals: BTreeSet<String>,
    /// The copies of those formals.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub copies: BTreeSet<String>,
    /// Every automatic local of the function (the admissible destinations of
    /// a literal or constant copy).
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub locals: BTreeSet<String>,
}

/// Which reading admits one copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reading {
    Literal,
    Constant,
    FormalCopy,
}

impl CustodyFreeCopies {
    pub fn is_empty(&self) -> bool {
        !self.literals
            && self.constants.is_empty()
            && self.formals.is_empty()
            && self.copies.is_empty()
    }

    /// The reading under which `destination = source` (with `source`
    /// unbound on the incoming state) leaves the destination unbound, if any.
    pub(crate) fn admits(&self, destination: &str, source: &str) -> Option<Reading> {
        if self.copies.contains(destination)
            && (self.formals.contains(source) || self.copies.contains(source))
        {
            return Some(Reading::FormalCopy);
        }
        if !self.locals.contains(destination) {
            return None;
        }
        if self.literals && source == LITERAL {
            return Some(Reading::Literal);
        }
        if self.constants.contains(source) {
            return Some(Reading::Constant);
        }
        None
    }
}

/// How one reference to a formal or a copy is used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Use {
    /// A whole read assigned by a plain `=` to an automatic local.
    CopyToLocal,
    /// A whole read initializing an automatic local.
    InitLocal,
    /// A whole read compared, negated or tested as a condition.
    NullTest,
    /// The base of `->`, `[]` or a unary `*`.
    Dereference,
    /// A whole read passed as a call actual.
    CallArgument,
    /// The target of a plain `=`.
    Rebind,
    /// A whole read returned.
    Return,
    /// A compound assignment or an increment or decrement.
    Write,
    /// `&` of it, or an array decaying.
    Address,
    /// Anything else (a store of its value, arithmetic, a `.` member...).
    Other,
}

/// What a reference is subject to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Subject {
    /// The pointer formal.
    Formal,
    /// An automatic local holding its value.
    Copy,
}

/// The decision for one reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    Admit,
    Withhold,
}

/// The decision table (`QCore.CustodyFreeCopy.decide`). A formal's own reads
/// (a dereference, a call actual, a null test, a `return`) are decided by the
/// existing rules on the formal and do not concern its copies; only a write
/// to the formal or its address changes what a copy holds.
pub(crate) fn decide(subject: Subject, used: Use) -> Decision {
    match (subject, used) {
        (Subject::Formal, Use::Rebind | Use::Write | Use::Address | Use::Other) => {
            Decision::Withhold
        }
        (Subject::Formal, _) => Decision::Admit,
        (
            Subject::Copy,
            Use::CopyToLocal
            | Use::InitLocal
            | Use::NullTest
            | Use::Dereference
            | Use::CallArgument
            | Use::Rebind,
        ) => Decision::Admit,
        (Subject::Copy, Use::Return | Use::Write | Use::Address | Use::Other) => Decision::Withhold,
    }
}

/// Every `(subject, use, decision)`, spelled as Lean's `decisionVectors`.
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(&str, &str, &str)] = &[
    ("formal", "copyToLocal", "admit"),
    ("formal", "initLocal", "admit"),
    ("formal", "nullTest", "admit"),
    ("formal", "dereference", "admit"),
    ("formal", "callArgument", "admit"),
    ("formal", "rebind", "withhold"),
    ("formal", "return", "admit"),
    ("formal", "write", "withhold"),
    ("formal", "address", "withhold"),
    ("formal", "other", "withhold"),
    ("copy", "copyToLocal", "admit"),
    ("copy", "initLocal", "admit"),
    ("copy", "nullTest", "admit"),
    ("copy", "dereference", "admit"),
    ("copy", "callArgument", "admit"),
    ("copy", "rebind", "admit"),
    ("copy", "return", "withhold"),
    ("copy", "write", "withhold"),
    ("copy", "address", "withhold"),
    ("copy", "other", "withhold"),
];

/// The spelling of a subject and a use in [`DECISION_VECTORS`].
#[cfg(test)]
pub(crate) fn spelled(subject: Subject, used: Use) -> (&'static str, &'static str) {
    let subject = match subject {
        Subject::Formal => "formal",
        Subject::Copy => "copy",
    };
    let used = match used {
        Use::CopyToLocal => "copyToLocal",
        Use::InitLocal => "initLocal",
        Use::NullTest => "nullTest",
        Use::Dereference => "dereference",
        Use::CallArgument => "callArgument",
        Use::Rebind => "rebind",
        Use::Return => "return",
        Use::Write => "write",
        Use::Address => "address",
        Use::Other => "other",
    };
    (subject, used)
}

/// Every subject and use, for the table test.
#[cfg(test)]
pub(crate) const ALL_USES: [Use; 10] = [
    Use::CopyToLocal,
    Use::InitLocal,
    Use::NullTest,
    Use::Dereference,
    Use::CallArgument,
    Use::Rebind,
    Use::Return,
    Use::Write,
    Use::Address,
    Use::Other,
];

/// The type spelling Clang gives a node, desugared where it prints one.
pub(crate) fn type_of(node: &Value) -> Option<&str> {
    node["type"]["desugaredQualType"]
        .as_str()
        .or_else(|| node["type"]["qualType"].as_str())
}

/// Whether a Clang type spelling is an object pointer: a `*` declarator at
/// the top level, no function or array declarator, neither `volatile` nor
/// `_Atomic` at its top level.
pub(crate) fn object_pointer(text: &str) -> bool {
    let text = text.trim();
    text.ends_with('*')
        && !text.contains('(')
        && !text.contains('[')
        && !top_level_volatile(text)
        && !atomic(text)
}

/// An object pointer whose pointee is not itself a pointer (`T **` is an
/// out slot's type, whose stores rule 11r decides, not a borrowed object).
pub(crate) fn single_level_pointer(text: &str) -> bool {
    if !object_pointer(text) {
        return false;
    }
    let Some(mut pointee) = text.trim().strip_suffix('*') else {
        return false;
    };
    loop {
        let trimmed = pointee.trim_end();
        let stripped = ["const", "volatile", "restrict", "__restrict"]
            .iter()
            .find_map(|qualifier| trimmed.strip_suffix(qualifier));
        match stripped {
            Some(rest) => pointee = rest,
            None => {
                pointee = trimmed;
                break;
            }
        }
    }
    !pointee.ends_with('*')
}

pub(crate) struct Visit<'a> {
    pub(crate) node: &'a Value,
    pub(crate) ancestors: Vec<&'a Value>,
}

pub(crate) fn visit<'a>(node: &'a Value, ancestors: &mut Vec<&'a Value>, out: &mut Vec<Visit<'a>>) {
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
pub(crate) fn classify(
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
        // `F->m` reads the pointer and dereferences it; `F.m` is impossible
        // on a pointer, but a `.` on anything else is not a pointer read.
        ("MemberExpr", _) if parent["isArrow"].as_bool() == Some(true) => {
            return (Use::Dereference, None);
        }
        ("MemberExpr", _) => return (Use::Other, None),
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
        ("MemberExpr", _) if user["isArrow"].as_bool() == Some(true) => (Use::Dereference, None),
        ("ArraySubscriptExpr", _) if first_child(user, value) => (Use::Dereference, None),
        ("UnaryOperator", Some("*")) => (Use::Dereference, None),
        ("CallExpr", _) if !first_child(user, value) => (Use::CallArgument, None),
        _ => (Use::Other, None),
    }
}

/// One reference: its visit position, the referenced declaration's kind and
/// id, and the reference's type spelling.
struct Reference<'a> {
    at: usize,
    decl_kind: &'a str,
    id: &'a str,
    ty: Option<&'a str>,
}

/// Rule 62's reading for one function. `formal_root` is the pointer-root
/// premise of a typed position (rule 5b's closure joined with rule 10l's, and
/// the declaration reader's `may_bind`), asked of a formal; `never_bound` is
/// the closure alone, asked of a copy (a copy is written by the copy itself,
/// so `may_bind` always names it). A name either does not answer for is
/// never admitted.
pub(crate) fn read(
    function: &Value,
    formal_root: &dyn Fn(&str) -> bool,
    never_bound: &dyn Fn(&str) -> bool,
) -> CustodyFreeCopies {
    let mut visits = Vec::new();
    visit(function, &mut Vec::new(), &mut visits);
    // Declarations of the function, by name.
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
    // Every declaration reference, by spelling.
    let mut references: BTreeMap<&str, Vec<Reference<'_>>> = BTreeMap::new();
    for (at, visit) in visits.iter().enumerate() {
        let node = visit.node;
        if kind(node) != "DeclRefExpr" {
            continue;
        }
        let declaration = &node["referencedDecl"];
        let (Some(name), Some(decl_kind), Some(id)) = (
            declaration["name"].as_str(),
            declaration["kind"].as_str(),
            declaration["id"].as_str(),
        ) else {
            continue;
        };
        references.entry(name).or_default().push(Reference {
            at,
            decl_kind,
            id,
            ty: type_of(node),
        });
    }
    // An automatic local: exactly one declaration of the spelling in the
    // function, a block-scope `VarDecl` with automatic storage that is
    // neither `volatile` nor `_Atomic`, no parameter of that spelling, and
    // every reference to the spelling designating that declaration (the
    // projection names objects by spelling, so a shadowed name is refused).
    let automatic = |name: &str| {
        let Some([declaration]) = variables.get(name).map(Vec::as_slice) else {
            return false;
        };
        !parameters.contains_key(name)
            && storage_of(declaration, true) == Some(Storage::Automatic)
            && type_of(declaration).is_some_and(|ty| !top_level_volatile(ty) && !atomic(ty))
            && declaration["id"].as_str().is_some_and(|id| {
                references
                    .get(name)
                    .into_iter()
                    .flatten()
                    .all(|reference| reference.id == id)
            })
    };
    let locals: BTreeSet<String> = variables
        .keys()
        .filter(|name| automatic(name))
        .map(|name| (*name).to_owned())
        .collect();
    let literals = !references.contains_key(LITERAL)
        && !parameters.contains_key(LITERAL)
        && !variables.contains_key(LITERAL);
    let constants: BTreeSet<String> = references
        .iter()
        .filter(|(name, uses)| {
            uses.iter()
                .all(|reference| reference.decl_kind == "EnumConstantDecl")
                && !parameters.contains_key(*name)
                && !variables.contains_key(*name)
        })
        .map(|(name, _)| (*name).to_owned())
        .collect();
    let pointer_local = |name: &str| {
        automatic(name)
            && variables[name]
                .iter()
                .all(|declaration| type_of(declaration).is_some_and(object_pointer))
    };
    let copies_into = |node: &Value| -> Option<String> {
        let name = match kind(node) {
            "DeclRefExpr" if node["referencedDecl"]["kind"].as_str() == Some("VarDecl") => {
                node["referencedDecl"]["name"].as_str()?
            }
            "VarDecl" => node["name"].as_str()?,
            _ => return None,
        };
        pointer_local(name).then(|| name.to_owned())
    };
    let mut formals = BTreeSet::new();
    let mut copies = BTreeSet::new();
    for (name, declarations) in &parameters {
        let [declaration] = declarations.as_slice() else {
            continue;
        };
        let Some(id) = declaration["id"].as_str() else {
            continue;
        };
        if variables.contains_key(name)
            || !type_of(declaration).is_some_and(single_level_pointer)
            || !formal_root(name)
        {
            continue;
        }
        let Some(uses) = references.get(name) else {
            continue;
        };
        if uses.iter().any(|reference| reference.id != id) {
            continue;
        }
        let mut decisions = Vec::new();
        let mut frontier = Vec::new();
        for reference in uses {
            let (used, local) = classify(&visits[reference.at], &copies_into);
            decisions.push((Subject::Formal, used));
            frontier.extend(local);
        }
        if frontier.is_empty() {
            continue;
        }
        let mut own: BTreeSet<String> = BTreeSet::new();
        while let Some(copy) = frontier.pop() {
            if !own.insert(copy.clone()) {
                continue;
            }
            if !never_bound(&copy) {
                decisions.push((Subject::Copy, Use::Other));
            }
            for reference in references.get(copy.as_str()).into_iter().flatten() {
                if !reference.ty.is_some_and(object_pointer) {
                    decisions.push((Subject::Copy, Use::Other));
                    continue;
                }
                let (used, local) = classify(&visits[reference.at], &copies_into);
                decisions.push((Subject::Copy, used));
                frontier.extend(local);
            }
        }
        // A local shared by the copies of two formals is not admitted for
        // either: the reading names one formal per copy.
        if own.iter().any(|copy| copies.contains(copy))
            || decisions
                .iter()
                .any(|(subject, used)| decide(*subject, *used) == Decision::Withhold)
        {
            continue;
        }
        formals.insert((*name).to_owned());
        copies.extend(own);
    }
    CustodyFreeCopies {
        literals,
        constants,
        formals,
        copies,
        locals,
    }
}

#[cfg(test)]
#[path = "../tests/unit/custody_free_copy.rs"]
mod tests;
