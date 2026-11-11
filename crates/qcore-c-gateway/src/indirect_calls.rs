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

//! Card **CG22-INDIRECT-CALLS-AND-CALLBACKS** (gateway rule 11p): candidate
//! callee sets for a call through a function pointer, and the callback
//! lifetime relation.
//!
//! # Candidate sets (K01)
//!
//! A call whose callee is not a function designator is resolved only from
//! facts the analysed translation unit establishes in Clang's AST
//! ([`resolve`]):
//!
//! * a function designator, through parentheses, `*` and `&` (`(*f)(x)` and
//!   `(&f)(x)` call `f`, C11 6.5.3.2p4 and 6.3.2.1p4) is `{f}`;
//! * an automatic local of function-pointer type whose address is never
//!   taken, declared with an initializer, is the union of what its
//!   initializer and every assignment to it in the body resolve to (a sound,
//!   flow-insensitive superset of its reaching definitions);
//! * a `const` object of function-pointer type, a member of a `const` record
//!   object or an element of a `const` array (a static table, rule 11h:
//!   static initialisation is no event, and a `const` object is never
//!   written), is what its initializer resolves to (every element for an
//!   index that is not a literal);
//! * a conditional over designators is the union of its arms, when its
//!   condition has no effect.
//!
//! Every other callee is an **open set** ([`Open`]): a formal (its value is
//! the caller's), a member read through a pointer (any object of the record
//! type may be the one read), a call's result, an explicit cast, a null
//! constant, an address-taken or uninitialised local, a mutable static, a
//! candidate a local declaration shadows. No set is ever "every function of
//! this signature": the program is not closed.
//!
//! # The join
//!
//! The call's verdict is the join (`Safe < Pending < Unknown < Unsafe`) over
//! its candidates' verdicts when the set covers the pointer's values, and
//! `Unknown` when it is open ([`join_over`], `QCore.IndirectCalls`). Route G
//! realises the join by rewriting the call (`legacy_indirect_calls.rs`): a
//! singleton is the direct call of its candidate, a larger set a branch with
//! one arm per candidate, so each candidate's contract (or validated
//! summary) is applied exactly as at a direct call of it, and the flow's own
//! join decides.
//!
//! # Callbacks (K02)
//!
//! [`callback_trace`] decides the capture relation of one registered callback
//! and its context: registration needs a live context and no earlier
//! teardown, every invocation needs the registration in force and the context
//! live, the context may be released only after unregistration, and a trace
//! that ends registered has an escaped capture (`Unknown`). Route G has no
//! registry vocabulary (the contract language states no capture), so a
//! registration call stays the uncontracted call it is; an invocation through
//! a resolved pointer applies the candidate's contract, which refuses a
//! released context as a use after release.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde_json::Value;

use crate::typed_c_ast::{children, kind};

/// How deep a chain of object definitions is followed.
const DEPTH: usize = 8;

// ---------------------------------------------------------------------------
// The decision core (bound to `QCore.IndirectCalls`).
// ---------------------------------------------------------------------------

/// The four-valued verdict, ordered `Safe < Pending < Unknown < Unsafe`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Verdict {
    Safe,
    // The route consults only the open-set row; the full order is the
    // model's, exercised by the shared vectors.
    #[cfg_attr(not(test), allow(dead_code))]
    Pending,
    Unknown,
    Unsafe,
}

/// The join over a candidate set: `Unknown` for an open or empty set, else
/// the greatest candidate verdict.
pub(crate) fn join_over(covers: bool, verdicts: &[Verdict]) -> Verdict {
    if !covers {
        return Verdict::Unknown;
    }
    verdicts.iter().copied().max().unwrap_or(Verdict::Unknown)
}

/// `JOIN_VECTORS`: (covers, candidate verdicts, the join), shared with
/// `QCore.IndirectCalls.joinVectors`.
#[cfg(test)]
pub(crate) const JOIN_VECTORS: &[(bool, &[Verdict], Verdict)] = &[
    (true, &[Verdict::Safe, Verdict::Safe], Verdict::Safe),
    (true, &[Verdict::Safe, Verdict::Unknown], Verdict::Unknown),
    (true, &[Verdict::Safe, Verdict::Pending], Verdict::Pending),
    (true, &[Verdict::Unsafe, Verdict::Safe], Verdict::Unsafe),
    (
        true,
        &[Verdict::Pending, Verdict::Unknown],
        Verdict::Unknown,
    ),
    (true, &[Verdict::Safe], Verdict::Safe),
    (true, &[], Verdict::Unknown),
    (false, &[Verdict::Safe], Verdict::Unknown),
    (false, &[Verdict::Safe, Verdict::Safe], Verdict::Unknown),
    (false, &[], Verdict::Unknown),
];

/// One event of a registered callback's life ([`callback_trace`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CallbackEvent {
    /// The callback is registered with its context.
    Register,
    /// The registry invokes the callback with the context. Route G sees no
    /// invocation by a registry and no unregistration: no contract states
    /// either, so these two are the model's (shared vectors).
    #[cfg_attr(not(test), allow(dead_code))]
    Invoke,
    /// The callback is unregistered (teardown of the registration).
    #[cfg_attr(not(test), allow(dead_code))]
    Unregister,
    /// The context is released.
    Release,
}

/// Decide one callback's trace: `Unsafe` at the first violation, `Unknown`
/// when it ends registered (the capture escapes), else `Safe`.
pub(crate) fn callback_trace(events: &[CallbackEvent]) -> Verdict {
    let (mut registered, mut live, mut torn_down) = (false, true, false);
    for event in events {
        match event {
            CallbackEvent::Register => {
                if registered || !live || torn_down {
                    return Verdict::Unsafe;
                }
                registered = true;
            }
            CallbackEvent::Invoke => {
                if !registered || !live {
                    return Verdict::Unsafe;
                }
            }
            CallbackEvent::Unregister => {
                if !registered {
                    return Verdict::Unsafe;
                }
                registered = false;
                torn_down = true;
            }
            CallbackEvent::Release => {
                if registered || !live {
                    return Verdict::Unsafe;
                }
                live = false;
            }
        }
    }
    if registered {
        Verdict::Unknown
    } else {
        Verdict::Safe
    }
}

/// `CALLBACK_VECTORS`: a trace and its decision, shared with
/// `QCore.IndirectCalls.callbackVectors`.
#[cfg(test)]
pub(crate) const CALLBACK_VECTORS: &[(&[CallbackEvent], Verdict)] = {
    use CallbackEvent::{Invoke, Register, Release, Unregister};
    &[
        (
            &[Register, Invoke, Invoke, Unregister, Release],
            Verdict::Safe,
        ),
        (&[Register, Release, Invoke], Verdict::Unsafe),
        (&[Register, Invoke, Release], Verdict::Unsafe),
        (&[Release, Register], Verdict::Unsafe),
        (&[Register, Unregister, Register], Verdict::Unsafe),
        (&[Register, Unregister, Invoke], Verdict::Unsafe),
        (&[Register, Invoke], Verdict::Unknown),
        (&[Invoke], Verdict::Unsafe),
        (&[Register, Register], Verdict::Unsafe),
        (&[], Verdict::Safe),
    ]
};

// ---------------------------------------------------------------------------
// Unit facts: constant tables and record member order.
// ---------------------------------------------------------------------------

/// A file-scope object whose initializer names a function.
#[derive(Clone, Debug)]
struct TableObject {
    init: Arc<Value>,
}

/// The translation unit's file-scope facts this module reads.
#[derive(Clone, Debug, Default)]
pub(crate) struct TableFacts {
    /// `VarDecl` id → a file-scope object with an initializer naming a
    /// function (others are unread: an open set).
    objects: BTreeMap<String, TableObject>,
    /// `FieldDecl` id → its position among its `struct`'s fields.
    fields: BTreeMap<String, usize>,
}

impl TableFacts {
    /// Collect from one top-level declaration.
    pub(crate) fn collect(node: &Value, facts: &mut Self) {
        match kind(node) {
            "VarDecl" => {
                if let (Some(id), Some(init)) = (node["id"].as_str(), initializer(node))
                    && names_a_function(init)
                {
                    facts.objects.insert(
                        id.to_owned(),
                        TableObject {
                            init: Arc::new(init.clone()),
                        },
                    );
                }
            }
            "RecordDecl" => Self::collect_record(node, facts),
            _ => {}
        }
    }

    fn collect_record(node: &Value, facts: &mut Self) {
        if node["tagUsed"].as_str() == Some("struct") {
            let mut index = 0;
            for child in children(node) {
                if kind(child) == "FieldDecl" {
                    if let Some(id) = child["id"].as_str() {
                        facts.fields.insert(id.to_owned(), index);
                    }
                    index += 1;
                }
            }
        }
        for child in children(node) {
            if kind(child) == "RecordDecl" {
                Self::collect_record(child, facts);
            }
        }
    }
}

/// A `VarDecl`'s initializer (its last child, when Clang marks one).
fn initializer(decl: &Value) -> Option<&Value> {
    if decl["init"].is_null() {
        return None;
    }
    children(decl).last()
}

fn names_a_function(node: &Value) -> bool {
    (kind(node) == "DeclRefExpr" && node["referencedDecl"]["kind"].as_str() == Some("FunctionDecl"))
        || children(node).iter().any(names_a_function)
}

// ---------------------------------------------------------------------------
// Resolution.
// ---------------------------------------------------------------------------

/// The callee's shape, for the census.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Shape {
    /// A function designator through `*`, `&` or parentheses.
    Designator,
    /// An automatic local pointer.
    Local,
    /// A formal pointer.
    Formal,
    /// A static or file-scope pointer object.
    Static,
    /// A member of a record object (`o.f`).
    Member,
    /// A member read through a pointer (`p->f`, vtable-like).
    MemberThroughPointer,
    /// An element of an array of pointers (`t[i]`).
    Element,
    /// A conditional over callees.
    Conditional,
    /// A call's result (`g()(x)`).
    CallResult,
    /// Anything else (a cast, a pointer read through `*`, ...).
    Other,
}

impl Shape {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Designator => "designator",
            Self::Local => "local",
            Self::Formal => "formal",
            Self::Static => "static",
            Self::Member => "member",
            Self::MemberThroughPointer => "member-through-pointer",
            Self::Element => "element",
            Self::Conditional => "conditional",
            Self::CallResult => "call-result",
            Self::Other => "other",
        }
    }
}

/// Why a candidate set is open.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Open {
    /// A formal's value is the caller's.
    Formal,
    /// A member read through a pointer: any object of the type may be read.
    MemberThroughPointer,
    /// A call's result.
    CallResult,
    /// An explicit cast on the way.
    Cast,
    /// A null pointer constant among the definitions (a call through it is
    /// undefined).
    Null,
    /// An automatic local whose address is taken.
    AddressTaken,
    /// An automatic local declared without an initializer.
    NoInitializer,
    /// A static or file-scope object that is not a constant table.
    MutableStatic,
    /// A table element or member this reader does not place.
    UnreadTable,
    /// An index, condition or base with an effect.
    Effect,
    /// A candidate a declaration in the function shadows.
    Shadowed(String),
    /// A chain of definitions deeper than [`DEPTH`].
    Depth,
    /// Any other form.
    Unread(String),
}

impl Open {
    pub(crate) fn describe(&self) -> String {
        match self {
            Self::Formal => "formal".into(),
            Self::MemberThroughPointer => "member read through a pointer".into(),
            Self::CallResult => "call result".into(),
            Self::Cast => "explicit cast".into(),
            Self::Null => "null pointer constant".into(),
            Self::AddressTaken => "address-taken local".into(),
            Self::NoInitializer => "local without an initializer".into(),
            Self::MutableStatic => "static object that is not a constant table".into(),
            Self::UnreadTable => "unread table member or element".into(),
            Self::Effect => "effectful callee expression".into(),
            Self::Shadowed(name) => format!("candidate `{name}` shadowed in the function"),
            Self::Depth => "definition chain too deep".into(),
            Self::Unread(form) => format!("unread form {form}"),
        }
    }
}

/// A callee's shape and its candidate set.
#[derive(Clone, Debug)]
pub(crate) struct Resolution {
    pub(crate) shape: Shape,
    pub(crate) candidates: Result<BTreeSet<String>, Open>,
}

/// Whether a call's callee is a direct function designator (no resolution
/// needed): through parentheses and the decay only, as `elaboration` reads it.
pub(crate) fn is_direct(callee: &Value) -> bool {
    crate::elaboration::direct_callee(callee).is_some() || is_builtin(callee)
}

/// A builtin's call (`__builtin_expect(…)`, Clang's `BuiltinFnToFnPtr`
/// decay): a direct call of a builtin, not an indirect one (card CG21's
/// builtin boundary owns it).
fn is_builtin(callee: &Value) -> bool {
    let node = strip_parens(callee);
    kind(node) == "ImplicitCastExpr"
        && node["castKind"].as_str() == Some("BuiltinFnToFnPtr")
        && children(node)
            .first()
            .map(strip_parens)
            .is_some_and(|inner| {
                kind(inner) == "DeclRefExpr"
                    && inner["referencedDecl"]["kind"].as_str() == Some("FunctionDecl")
            })
}

/// Resolve a call's callee expression to its candidate set.
pub(crate) fn resolve(callee: &Value, function: &Value, tables: &TableFacts) -> Resolution {
    let reader = Reader { function, tables };
    let shape = reader.shape(callee);
    let mut visited = BTreeSet::new();
    let candidates = reader.value(callee, 0, &mut visited).and_then(|set| {
        if set.is_empty() {
            return Err(Open::Unread("empty".into()));
        }
        let shadowed = declared_names(function);
        match set.iter().find(|name| shadowed.contains(*name)) {
            Some(name) => Err(Open::Shadowed(name.clone())),
            None => Ok(set),
        }
    });
    Resolution { shape, candidates }
}

struct Reader<'a> {
    function: &'a Value,
    tables: &'a TableFacts,
}

fn strip_parens(mut node: &Value) -> &Value {
    while kind(node) == "ParenExpr" {
        match children(node).first() {
            Some(inner) => node = inner,
            None => break,
        }
    }
    node
}

fn type_of(node: &Value) -> &str {
    node["type"]["desugaredQualType"]
        .as_str()
        .or_else(|| node["type"]["qualType"].as_str())
        .unwrap_or("")
}

/// Whether a type spelling is a function type (not a pointer to one).
fn is_function_type(spelling: &str) -> bool {
    spelling
        .find('(')
        .is_some_and(|open| !spelling[open + 1..].trim_start().starts_with('*'))
}

/// Whether a place's type is a `const`-qualified function pointer.
fn const_pointer_place(node: &Value) -> bool {
    let spelling = type_of(node);
    spelling.contains("(*const)")
        || node["type"]["qualType"]
            .as_str()
            .is_some_and(|q| q.starts_with("const "))
}

/// Whether an expression has an effect: a call, an assignment, an increment
/// or decrement, a statement expression, or a `volatile` read.
pub(crate) fn has_effect(node: &Value) -> bool {
    match kind(node) {
        "CallExpr" | "StmtExpr" | "CompoundAssignOperator" => return true,
        "UnaryOperator" if matches!(node["opcode"].as_str(), Some("++" | "--")) => return true,
        "BinaryOperator" if node["opcode"].as_str() == Some("=") => return true,
        "DeclRefExpr" if type_of(node).contains("volatile") => return true,
        _ => {}
    }
    children(node).iter().any(has_effect)
}

/// Every name an object or formal of the function declares.
fn declared_names(function: &Value) -> BTreeSet<String> {
    fn walk(node: &Value, out: &mut BTreeSet<String>) {
        if matches!(kind(node), "VarDecl" | "ParmVarDecl")
            && let Some(name) = node["name"].as_str()
        {
            out.insert(name.to_owned());
        }
        for child in children(node) {
            walk(child, out);
        }
    }
    let mut out = BTreeSet::new();
    walk(function, &mut out);
    out
}

/// Whether a member or element place is read through a pointer anywhere on
/// its base chain (`p->a.f`, `(*p).f`, `p[i].f`): any object of the type may
/// be the one read, so its value is open.
fn through_pointer(place: &Value) -> bool {
    let place = strip_parens(place);
    match kind(place) {
        "MemberExpr" => {
            place["isArrow"].as_bool() == Some(true)
                || children(place).first().is_some_and(through_pointer)
        }
        "ArraySubscriptExpr" => match children(place).first().map(strip_parens) {
            Some(base)
                if kind(base) == "ImplicitCastExpr"
                    && base["castKind"].as_str() == Some("ArrayToPointerDecay") =>
            {
                children(base).first().is_some_and(through_pointer)
            }
            _ => true,
        },
        "UnaryOperator" => place["opcode"].as_str() == Some("*"),
        "ImplicitCastExpr" => children(place).first().is_some_and(through_pointer),
        _ => false,
    }
}

fn find_decl<'a>(node: &'a Value, id: &str) -> Option<&'a Value> {
    if matches!(kind(node), "VarDecl" | "ParmVarDecl") && node["id"].as_str() == Some(id) {
        return Some(node);
    }
    children(node).iter().find_map(|child| find_decl(child, id))
}

fn refers_to(node: &Value, id: &str) -> bool {
    let node = strip_parens(node);
    kind(node) == "DeclRefExpr" && node["referencedDecl"]["id"].as_str() == Some(id)
}

/// The assignments `id = E` in the function (their right operands), and
/// whether `&id` appears.
fn definitions<'a>(node: &'a Value, id: &str, values: &mut Vec<&'a Value>, taken: &mut bool) {
    match kind(node) {
        "BinaryOperator" if node["opcode"].as_str() == Some("=") => {
            if let [left, right] = children(node)
                && refers_to(left, id)
            {
                values.push(right);
            }
        }
        "UnaryOperator"
            if node["opcode"].as_str() == Some("&")
                && children(node)
                    .first()
                    .is_some_and(|operand| refers_to(operand, id)) =>
        {
            *taken = true;
        }
        _ => {}
    }
    for child in children(node) {
        definitions(child, id, values, taken);
    }
}

impl Reader<'_> {
    fn shape(&self, callee: &Value) -> Shape {
        // Through parentheses, the decay, `NoOp` and a function-valued `*`
        // or `&` to the callee's core.
        let mut node = strip_parens(callee);
        loop {
            let through = match kind(node) {
                "ImplicitCastExpr" => matches!(
                    node["castKind"].as_str(),
                    Some("FunctionToPointerDecay" | "NoOp")
                ),
                "UnaryOperator" => match node["opcode"].as_str() {
                    Some("*") => is_function_type(type_of(node)),
                    Some("&") => children(node)
                        .first()
                        .is_some_and(|inner| is_function_type(type_of(inner))),
                    _ => false,
                },
                _ => false,
            };
            match children(node).first() {
                Some(inner) if through => node = strip_parens(inner),
                _ => break,
            }
        }
        match kind(node) {
            "DeclRefExpr" if node["referencedDecl"]["kind"].as_str() == Some("FunctionDecl") => {
                Shape::Designator
            }
            "ImplicitCastExpr" if node["castKind"].as_str() == Some("LValueToRValue") => {
                let Some(place) = children(node).first().map(strip_parens) else {
                    return Shape::Other;
                };
                match kind(place) {
                    "DeclRefExpr" => match place["referencedDecl"]["kind"].as_str() {
                        Some("ParmVarDecl") => Shape::Formal,
                        Some("VarDecl") => {
                            let id = place["referencedDecl"]["id"].as_str().unwrap_or("");
                            match find_decl(self.function, id) {
                                Some(decl) if decl["storageClass"].is_null() => Shape::Local,
                                _ => Shape::Static,
                            }
                        }
                        _ => Shape::Other,
                    },
                    "MemberExpr" | "ArraySubscriptExpr" if through_pointer(place) => {
                        Shape::MemberThroughPointer
                    }
                    "MemberExpr" => Shape::Member,
                    "ArraySubscriptExpr" => Shape::Element,
                    _ => Shape::Other,
                }
            }
            "ConditionalOperator" => Shape::Conditional,
            "CallExpr" => Shape::CallResult,
            _ => Shape::Other,
        }
    }

    /// The candidates a function-pointer (or function) valued expression
    /// may evaluate to.
    fn value(
        &self,
        expression: &Value,
        depth: usize,
        visited: &mut BTreeSet<String>,
    ) -> Result<BTreeSet<String>, Open> {
        if depth > DEPTH {
            return Err(Open::Depth);
        }
        let node = strip_parens(expression);
        match kind(node) {
            "DeclRefExpr" => match node["referencedDecl"]["kind"].as_str() {
                Some("FunctionDecl") => Ok(node["referencedDecl"]["name"]
                    .as_str()
                    .map(|name| BTreeSet::from([name.to_owned()]))
                    .unwrap_or_default()),
                _ => Err(Open::Unread("object designator".into())),
            },
            "ImplicitCastExpr" => {
                let inner = children(node).first().ok_or(Open::Unread("cast".into()))?;
                match node["castKind"].as_str() {
                    Some("FunctionToPointerDecay" | "NoOp") => {
                        self.value(inner, depth + 1, visited)
                    }
                    Some("LValueToRValue") => self.object(inner, depth + 1, visited),
                    Some("NullToPointer") => Err(Open::Null),
                    Some(other) => Err(Open::Unread(format!("implicit {other}"))),
                    None => Err(Open::Unread("cast".into())),
                }
            }
            "UnaryOperator" => {
                let inner = children(node)
                    .first()
                    .ok_or(Open::Unread("operator".into()))?;
                match node["opcode"].as_str() {
                    // `*f` of a function pointer or designator is a designator.
                    Some("*") if is_function_type(type_of(node)) => {
                        self.value(inner, depth + 1, visited)
                    }
                    // `&f` of a function designator.
                    Some("&") if is_function_type(type_of(inner)) => {
                        self.value(inner, depth + 1, visited)
                    }
                    Some("*") => Err(Open::Unread("pointer read through `*`".into())),
                    _ => Err(Open::Unread("operator".into())),
                }
            }
            "ConditionalOperator" => {
                let [condition, left, right] = children(node) else {
                    return Err(Open::Unread("conditional".into()));
                };
                if has_effect(condition) {
                    return Err(Open::Effect);
                }
                let mut set = self.value(left, depth + 1, visited)?;
                set.extend(self.value(right, depth + 1, visited)?);
                Ok(set)
            }
            "CallExpr" => Err(Open::CallResult),
            "CStyleCastExpr" | "ExplicitCastExpr" => Err(Open::Cast),
            "IntegerLiteral" | "GNUNullExpr" | "ImplicitValueInitExpr" => Err(Open::Null),
            other => Err(Open::Unread(other.to_owned())),
        }
    }

    /// The candidates an object of function-pointer type may hold, read
    /// through its place.
    fn object(
        &self,
        place: &Value,
        depth: usize,
        visited: &mut BTreeSet<String>,
    ) -> Result<BTreeSet<String>, Open> {
        let place = strip_parens(place);
        match kind(place) {
            "DeclRefExpr" => {
                let id = place["referencedDecl"]["id"].as_str().unwrap_or("");
                match place["referencedDecl"]["kind"].as_str() {
                    Some("ParmVarDecl") => return Err(Open::Formal),
                    Some("VarDecl") => {}
                    _ => return Err(Open::Unread("object".into())),
                }
                if !visited.insert(id.to_owned()) {
                    // Already being resolved: its definitions are in the union.
                    return Ok(BTreeSet::new());
                }
                if let Some(decl) = find_decl(self.function, id) {
                    if decl["storageClass"].is_null() {
                        return self.automatic(decl, id, depth, visited);
                    }
                    // A static local: a constant table only.
                    if !const_pointer_place(place) {
                        return Err(Open::MutableStatic);
                    }
                    let init = initializer(decl).ok_or(Open::NoInitializer)?;
                    return self.value(init, depth + 1, visited);
                }
                if !const_pointer_place(place) {
                    return Err(Open::MutableStatic);
                }
                let object = self.tables.objects.get(id).ok_or(Open::MutableStatic)?;
                self.value(&object.init, depth + 1, visited)
            }
            "MemberExpr" | "ArraySubscriptExpr" if through_pointer(place) => {
                Err(Open::MemberThroughPointer)
            }
            "MemberExpr" | "ArraySubscriptExpr" => {
                if !const_pointer_place(place) {
                    return Err(Open::MutableStatic);
                }
                let inits = self.place_inits(place, depth)?;
                let mut set = BTreeSet::new();
                for init in inits {
                    set.extend(self.value(&init, depth + 1, visited)?);
                }
                Ok(set)
            }
            "UnaryOperator" => Err(Open::Unread("pointer read through `*`".into())),
            other => Err(Open::Unread(other.to_owned())),
        }
    }

    /// An automatic local: its initializer and every assignment, when its
    /// address is never taken.
    fn automatic(
        &self,
        decl: &Value,
        id: &str,
        depth: usize,
        visited: &mut BTreeSet<String>,
    ) -> Result<BTreeSet<String>, Open> {
        let init = initializer(decl).ok_or(Open::NoInitializer)?;
        let mut values = Vec::new();
        let mut taken = false;
        definitions(self.function, id, &mut values, &mut taken);
        if taken {
            return Err(Open::AddressTaken);
        }
        let mut set = self.value(init, depth + 1, visited)?;
        for value in values {
            set.extend(self.value(value, depth + 1, visited)?);
        }
        Ok(set)
    }

    /// The initializer expressions a member or element place of a constant
    /// object reads: one for a member or a literal index, every element for
    /// any other index.
    fn place_inits(&self, place: &Value, depth: usize) -> Result<Vec<Value>, Open> {
        if depth > DEPTH {
            return Err(Open::Depth);
        }
        let place = strip_parens(place);
        match kind(place) {
            "DeclRefExpr" => {
                let id = place["referencedDecl"]["id"].as_str().unwrap_or("");
                if place["referencedDecl"]["kind"].as_str() != Some("VarDecl") {
                    return Err(Open::UnreadTable);
                }
                if let Some(decl) = find_decl(self.function, id) {
                    // The caller checked that the place read is `const`, so
                    // the object region is never written: its initializer
                    // is its value, whatever its storage.
                    let init = initializer(decl).ok_or(Open::NoInitializer)?;
                    return Ok(vec![init.clone()]);
                }
                let object = self.tables.objects.get(id).ok_or(Open::UnreadTable)?;
                Ok(vec![(*object.init).clone()])
            }
            "MemberExpr" => {
                if place["isArrow"].as_bool() == Some(true) {
                    return Err(Open::MemberThroughPointer);
                }
                let base = children(place).first().ok_or(Open::UnreadTable)?;
                let field = place["referencedMemberDecl"].as_str().unwrap_or("");
                let index = *self.tables.fields.get(field).ok_or(Open::UnreadTable)?;
                let mut out = Vec::new();
                for init in self.place_inits(base, depth + 1)? {
                    let init = strip_parens(&init);
                    if kind(init) != "InitListExpr" || !type_of(init).starts_with("struct") {
                        return Err(Open::UnreadTable);
                    }
                    out.push(children(init).get(index).cloned().ok_or(Open::Null)?);
                }
                Ok(out)
            }
            "ArraySubscriptExpr" => {
                let [base, index] = children(place) else {
                    return Err(Open::UnreadTable);
                };
                if has_effect(index) || has_effect(base) {
                    return Err(Open::Effect);
                }
                let base = strip_parens(base);
                if kind(base) != "ImplicitCastExpr"
                    || base["castKind"].as_str() != Some("ArrayToPointerDecay")
                {
                    return Err(Open::UnreadTable);
                }
                let array = children(base).first().ok_or(Open::UnreadTable)?;
                let literal = {
                    let index = strip_parens(index);
                    (kind(index) == "IntegerLiteral")
                        .then(|| {
                            index["value"]
                                .as_str()
                                .and_then(|v| v.parse::<usize>().ok())
                        })
                        .flatten()
                };
                let mut out = Vec::new();
                for init in self.place_inits(array, depth + 1)? {
                    let init = strip_parens(&init);
                    if kind(init) != "InitListExpr" {
                        return Err(Open::UnreadTable);
                    }
                    // An array filler (fewer initializers than elements)
                    // holds null pointers.
                    if !init["array_filler"].is_null() && literal.is_none() {
                        return Err(Open::Null);
                    }
                    match literal {
                        Some(at) => out.push(children(init).get(at).cloned().ok_or(Open::Null)?),
                        None => out.extend(children(init).iter().cloned()),
                    }
                }
                Ok(out)
            }
            _ => Err(Open::UnreadTable),
        }
    }
}

/// The definitions of the automatic local a callee reads.
pub(crate) struct LocalDefinitions<'a> {
    /// The local's declaration id.
    pub(crate) id: String,
    /// The callee's own read of it.
    pub(crate) read: &'a Value,
    /// Its initializer and the right operand of every assignment to it.
    pub(crate) values: Vec<&'a Value>,
    /// How many references to it the function holds, and how many of them
    /// are an assignment's left operand.
    pub(crate) references: usize,
    pub(crate) assigned: usize,
}

/// The definitions of the automatic local `callee` reads, if it reads one.
/// Route G replaces each value by the null constant once every other
/// reference to the local is a rewritten call's callee: the local is then
/// never read again, so its value is unobservable, and the designator stops
/// reading as an unknown object named like the function.
pub(crate) fn local_definitions<'a>(
    callee: &'a Value,
    function: &'a Value,
) -> Option<LocalDefinitions<'a>> {
    fn count(node: &Value, id: &str, references: &mut usize) {
        if kind(node) == "DeclRefExpr" && node["referencedDecl"]["id"].as_str() == Some(id) {
            *references += 1;
        }
        for child in children(node) {
            count(child, id, references);
        }
    }
    let mut node = strip_parens(callee);
    while kind(node) == "ImplicitCastExpr" && node["castKind"].as_str() == Some("NoOp")
        || kind(node) == "UnaryOperator" && node["opcode"].as_str() == Some("*")
        || kind(node) == "ImplicitCastExpr"
            && node["castKind"].as_str() == Some("FunctionToPointerDecay")
    {
        node = strip_parens(children(node).first()?);
    }
    if kind(node) != "ImplicitCastExpr" || node["castKind"].as_str() != Some("LValueToRValue") {
        return None;
    }
    let place = strip_parens(children(node).first()?);
    if kind(place) != "DeclRefExpr" || place["referencedDecl"]["kind"].as_str() != Some("VarDecl") {
        return None;
    }
    let id = place["referencedDecl"]["id"].as_str()?;
    let decl = find_decl(function, id)?;
    if !decl["storageClass"].is_null() {
        return None;
    }
    let mut values = Vec::new();
    let mut taken = false;
    definitions(function, id, &mut values, &mut taken);
    if taken {
        return None;
    }
    let assigned = values.len();
    let mut references = 0;
    count(function, id, &mut references);
    Some(LocalDefinitions {
        id: id.to_owned(),
        read: place,
        values: initializer(decl).into_iter().chain(values).collect(),
        references,
        assigned,
    })
}

/// A call's position in its statement, for the rewrite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Form {
    /// `CALLEE(args);`
    Statement,
    /// `NAME = CALLEE(args);`
    Assignment,
    /// `return CALLEE(args);`
    Return,
    /// `T NAME = CALLEE(args);`
    Declaration,
    /// Inside a larger expression.
    Nested,
}

impl Form {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Statement => "statement",
            Self::Assignment => "assignment",
            Self::Return => "return",
            Self::Declaration => "declaration",
            Self::Nested => "nested",
        }
    }
}

/// One indirect call of a function body, with its form and the node that
/// is its whole statement (for [`Form::Statement`], [`Form::Assignment`] and
/// [`Form::Return`]).
pub(crate) struct Site<'a> {
    pub(crate) call: &'a Value,
    pub(crate) form: Form,
    pub(crate) statement: Option<&'a Value>,
}

/// Every call in `function` whose callee is not a direct designator.
pub(crate) fn indirect_sites(function: &Value) -> Vec<Site<'_>> {
    fn statement_parent(kind: &str) -> bool {
        matches!(
            kind,
            "CompoundStmt"
                | "IfStmt"
                | "WhileStmt"
                | "ForStmt"
                | "DoStmt"
                | "LabelStmt"
                | "CaseStmt"
                | "DefaultStmt"
        )
    }
    fn walk<'a>(node: &'a Value, parents: &mut Vec<&'a Value>, out: &mut Vec<Site<'a>>) {
        if kind(node) == "CallExpr"
            && let Some(callee) = children(node).first()
            && !is_direct(callee)
        {
            let parent = parents.last().copied();
            let grand = parents.len().checked_sub(2).map(|at| parents[at]);
            let (form, statement) = match parent.map(kind) {
                Some(parent_kind) if statement_parent(parent_kind) => {
                    // A child of an `if`/loop header is its condition, not
                    // its body, unless it is the body (last child).
                    let body = parent.is_some_and(|parent| {
                        let siblings = children(parent);
                        let position = siblings.iter().position(|child| std::ptr::eq(child, node));
                        match parent_kind {
                            "CompoundStmt" => true,
                            "IfStmt" => position.is_some_and(|at| at >= 1),
                            "DoStmt" => position == Some(0),
                            _ => position.is_some_and(|at| at + 1 == siblings.len()),
                        }
                    });
                    if body {
                        (Form::Statement, Some(node))
                    } else {
                        (Form::Nested, None)
                    }
                }
                Some("BinaryOperator")
                    if parent.is_some_and(|p| {
                        p["opcode"].as_str() == Some("=")
                            && children(p)
                                .get(1)
                                .is_some_and(|right| std::ptr::eq(right, node))
                            && children(p)
                                .first()
                                .is_some_and(|left| kind(strip_parens(left)) == "DeclRefExpr")
                    }) && grand.is_some_and(|g| kind(g) == "CompoundStmt") =>
                {
                    (Form::Assignment, parent)
                }
                Some("ReturnStmt") => (Form::Return, parent),
                Some("VarDecl") => (Form::Declaration, None),
                _ => (Form::Nested, None),
            };
            out.push(Site {
                call: node,
                form,
                statement,
            });
        }
        parents.push(node);
        for child in children(node) {
            walk(child, parents, out);
        }
        parents.pop();
    }
    let mut out = Vec::new();
    walk(function, &mut Vec::new(), &mut out);
    out
}

/// One call that passes a function designator as an actual: a callback
/// registration, in the census's sense.
pub(crate) struct Registration<'a> {
    pub(crate) call: &'a Value,
    /// The registrar, when it is a direct designator.
    pub(crate) callee: Option<String>,
    /// The functions passed.
    pub(crate) designators: Vec<String>,
    /// Every other pointer actual (a context), by declaration id when it is
    /// a named object.
    pub(crate) contexts: Vec<Option<String>>,
}

/// Every callback registration in `function`, in source order.
pub(crate) fn registrations(function: &Value) -> Vec<Registration<'_>> {
    fn walk<'a>(node: &'a Value, out: &mut Vec<Registration<'a>>) {
        if kind(node) == "CallExpr"
            && let [callee, actuals @ ..] = children(node)
        {
            let mut designators = Vec::new();
            let mut contexts = Vec::new();
            for actual in actuals {
                let mut inner = strip_parens(actual);
                while kind(inner) == "ImplicitCastExpr"
                    && matches!(
                        inner["castKind"].as_str(),
                        Some("FunctionToPointerDecay" | "NoOp" | "BitCast")
                    )
                {
                    inner = children(inner).first().map_or(inner, strip_parens);
                }
                if kind(inner) == "UnaryOperator" && inner["opcode"].as_str() == Some("&") {
                    inner = children(inner).first().map_or(inner, strip_parens);
                }
                if kind(inner) == "DeclRefExpr"
                    && inner["referencedDecl"]["kind"].as_str() == Some("FunctionDecl")
                    && let Some(name) = inner["referencedDecl"]["name"].as_str()
                {
                    designators.push(name.to_owned());
                } else if type_of(actual).contains('*') && !is_function_type(type_of(actual)) {
                    contexts.push(named_object(actual).map(str::to_owned));
                }
            }
            if !designators.is_empty() {
                out.push(Registration {
                    call: node,
                    callee: crate::elaboration::direct_callee(callee).map(str::to_owned),
                    designators,
                    contexts,
                });
            }
        }
        for child in children(node) {
            walk(child, out);
        }
    }
    let mut out = Vec::new();
    walk(function, &mut out);
    out
}

/// The declaration id a pointer actual reads, through parentheses and
/// value-preserving casts.
fn named_object(actual: &Value) -> Option<&str> {
    let mut node = strip_parens(actual);
    while kind(node) == "ImplicitCastExpr"
        && matches!(
            node["castKind"].as_str(),
            Some("LValueToRValue" | "NoOp" | "BitCast")
        )
    {
        node = strip_parens(children(node).first()?);
    }
    (kind(node) == "DeclRefExpr"
        && matches!(
            node["referencedDecl"]["kind"].as_str(),
            Some("VarDecl" | "ParmVarDecl")
        ))
    .then(|| node["referencedDecl"]["id"].as_str())
    .flatten()
}

fn begin_offset(node: &Value) -> Option<u64> {
    node["range"]["begin"]["offset"]
        .as_u64()
        .or_else(|| node["range"]["begin"]["expansionLoc"]["offset"].as_u64())
}

/// The callback trace a function shows for one registration's context: the
/// registration, then each later direct call that releases the context
/// (`released(callee, position)`). Nothing in the function is an
/// unregistration: no contract states one.
pub(crate) fn context_trace(
    function: &Value,
    registration: &Registration<'_>,
    context: &str,
    released: &dyn Fn(&str, usize) -> bool,
) -> Vec<CallbackEvent> {
    fn calls<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
        if kind(node) == "CallExpr" {
            out.push(node);
        }
        for child in children(node) {
            calls(child, out);
        }
    }
    let start = begin_offset(registration.call).unwrap_or(u64::MAX);
    let mut all = Vec::new();
    calls(function, &mut all);
    let mut later = all
        .into_iter()
        .filter(|call| begin_offset(call).is_some_and(|at| at > start))
        .collect::<Vec<_>>();
    later.sort_by_key(|call| begin_offset(call));
    let mut trace = vec![CallbackEvent::Register];
    for call in later {
        let [callee, actuals @ ..] = children(call) else {
            continue;
        };
        let Some(name) = crate::elaboration::direct_callee(callee) else {
            continue;
        };
        if actuals
            .iter()
            .enumerate()
            .any(|(at, actual)| named_object(actual) == Some(context) && released(name, at))
        {
            trace.push(CallbackEvent::Release);
        }
    }
    trace
}

#[cfg(test)]
#[path = "../tests/unit/indirect_calls.rs"]
mod tests;
