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

//! Card **CG15-STORAGE-DURATION-AND-VOLATILE** (gateway rule 11h): storage
//! duration and `volatile`, read from Clang's typed AST, with the decision
//! table every route consults. The route-G adapter is
//! `legacy_storage_duration.rs`; route T names the storage-duration forms it
//! refuses (`typed_source.rs`).
//!
//! # Storage duration (gap entry M24)
//!
//! An object's [`Storage`] is Clang's: a `VarDecl` with no storage class (or
//! `register`) inside a function is **automatic** and lives for one frame; a
//! `static` or `extern` one, and every file-scope object, has **static**
//! storage and lives for the whole program; a `_Thread_local`/`__thread` one
//! lives for its thread (C11 6.2.4). What follows from it:
//!
//! * **Static initialisation is no event.** A block-scope static-duration
//!   declaration is not executed when control reaches it: its initializer is
//!   a constant expression (C11 6.7.9p4) evaluated before the program starts,
//!   so the declaration carries no custody and no per-call value
//!   ([`Form::StaticDeclaration`]). Before this card route G lowered
//!   `static void *cache = 0;` as `cache = 0` on **every** call, so a later
//!   `if (cache) …` was decided by a value the object does not hold on any
//!   call after the first (a soundness repair).
//! * **A static object's value carries no identity.** Its value on entry is
//!   whatever an earlier call or another function stored, so a read yields a
//!   borrowed alias with no identity ([`Form::StaticRead`]); a custody-taking
//!   use of it refuses ([`Form::StaticReadReleased`]). Its address is a
//!   pointer with no identity whose object outlives every frame
//!   ([`Form::StaticAddress`]).
//! * **A tracked identity stored into static storage escapes the frame to no
//!   named party** ([`Form::StaticIdentityStore`]). The frame no longer
//!   accounts for it, and no contract says who does, so the function is
//!   refused: neither a per-call leak nor `Safe`. A function-local `static`
//!   pointer that holds an allocation across calls is this form (a retained
//!   resource). A registry contract with a release protocol would decide it;
//!   none exists (`CG15-STATIC-REGISTRY-CONTRACT`). This is card
//!   CTWIN-PUBLISH's refusal, unchanged, named here.
//!
//! # Volatile (gap entry M23)
//!
//! **The premise, stated.** The gateway's abstract state assumes that no
//! object it holds a value-derived fact about is modified asynchronously: a
//! binding, a null or validity fact, a scalar's known value. C11 5.1.2.3p2
//! makes that false for a `volatile` object only (and for an object shared
//! with another thread, which is `_Atomic`'s and card CG19's). This module
//! gives the importer the qualifier, so the premise is checked, not assumed:
//!
//! * a `volatile` object is never bound to a tracked identity
//!   ([`Form::VolatileBinding`] refuses), never given a known value
//!   ([`Form::VolatileValueFact`] forgets it) and never refined by a test
//!   ([`Form::VolatileRefinement`] keeps both edges unrefined);
//! * a branch whose condition reads a `volatile` object and whose arms differ
//!   in any custody event is a **custody decision on a value that may change
//!   asynchronously**, and refuses ([`Form::VolatileDecision`]);
//! * every other `volatile` access is custody-neutral and stays admitted
//!   ([`Form::VolatileNeutral`]). It is a separate observable effect with a
//!   count and an order ([`volatile_trace`]): each access once, ordered only
//!   where C sequences it, and two unsequenced accesses of one object never
//!   ordered (the trace names the pair; C11 6.5p2).
//!
//! `volatile` is not `_Atomic`: an atomic access stays card CG19's
//! ([`Form::AtomicAccess`]).
//!
//! The Lean model is `QCore.StorageDuration`, bound to [`FORM_TABLE`] and the
//! gateway's verdicts on its shapes by shared-vector tests.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::typed_c_ast::{children, kind};

/// An object's storage duration (C11 6.2.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Storage {
    /// One frame: a block-scope object with no storage class or `register`.
    Automatic,
    /// The whole program: `static`, `extern`, or file scope.
    Static,
    /// Its thread: `_Thread_local` / `__thread`.
    ThreadLocal,
}

impl Storage {
    /// Whether the object outlives every frame of the function that names it.
    pub(crate) fn outlives_frame(self) -> bool {
        !matches!(self, Storage::Automatic)
    }
}

/// The storage duration of a `VarDecl` Clang node, `None` for any other node.
/// `block` says the declaration is inside a function body.
pub(crate) fn storage_of(declaration: &Value, block: bool) -> Option<Storage> {
    if kind(declaration) != "VarDecl" {
        return None;
    }
    if declaration["tls"].as_str().is_some() {
        return Some(Storage::ThreadLocal);
    }
    match declaration["storageClass"].as_str() {
        Some("static" | "extern") => Some(Storage::Static),
        None | Some("register") if block => Some(Storage::Automatic),
        None | Some("register") => Some(Storage::Static),
        // `auto`, `__private_extern__` and anything else Clang may print.
        Some(_) => None,
    }
}

/// Whether a Clang type spelling is `volatile` at its top level: after the
/// last declarator `*` for a pointer, anywhere in the specifiers otherwise
/// (an array of `volatile` elements counts: every element access is one).
pub(crate) fn top_level_volatile(text: &str) -> bool {
    let tail = text.rsplit_once('*').map_or(text, |(_, tail)| tail);
    tail.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .any(|word| word == "volatile")
}

/// Whether a Clang type spelling is `_Atomic` anywhere (CG19's).
pub(crate) fn atomic(text: &str) -> bool {
    text.contains("_Atomic")
}

/// Every form this card decides.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Form {
    /// A block-scope `static`/`extern`/thread-local declaration, whatever its
    /// constant initializer.
    StaticDeclaration,
    /// A read of a static- or thread-duration object's value.
    StaticRead,
    /// The address of (or an array decaying from) a static- or
    /// thread-duration object.
    StaticAddress,
    /// A scalar store into static storage.
    StaticScalarStore,
    /// A tracked identity stored into static storage, no registry contract.
    StaticIdentityStore,
    /// A custody-taking use of a value read from static storage.
    StaticReadReleased,
    /// A `volatile` access whose value no custody event uses.
    VolatileNeutral,
    /// A pointer or tracked identity stored into, or bound to, a `volatile`
    /// object.
    VolatileBinding,
    /// A known value (a null or constant store) given to a `volatile` object.
    VolatileValueFact,
    /// A null or validity test of a `volatile` object.
    VolatileRefinement,
    /// A branch on a `volatile` read whose arms differ in a custody event.
    VolatileDecision,
    /// An `_Atomic` access (CG19's).
    AtomicAccess,
}

/// What the route does with one form (`QCore.StorageDuration.Disposition`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Disposition {
    /// No event at all: nothing runs when control reaches it.
    NoEvent,
    /// A value or pointer with no identity; custody-neutral.
    NoIdentity,
    /// Admitted, custody-neutral.
    Neutral,
    /// The object's value fact is forgotten (no must-fact is kept).
    Forgotten,
    /// Both edges kept, neither refined.
    Unrefined,
    /// Refused by this card's rule.
    Refused,
    /// Refused as another card's.
    Owned(&'static str),
}

/// The decision table (`QCore.StorageDuration.formTable`), row for row.
pub(crate) const FORM_TABLE: [(Form, &str, Disposition); 12] = [
    (
        Form::StaticDeclaration,
        "static declaration",
        Disposition::NoEvent,
    ),
    (Form::StaticRead, "static read", Disposition::NoIdentity),
    (
        Form::StaticAddress,
        "static address",
        Disposition::NoIdentity,
    ),
    (
        Form::StaticScalarStore,
        "static scalar store",
        Disposition::Neutral,
    ),
    (
        Form::StaticIdentityStore,
        "static identity store",
        Disposition::Refused,
    ),
    (
        Form::StaticReadReleased,
        "static read released",
        Disposition::Refused,
    ),
    (
        Form::VolatileNeutral,
        "volatile neutral",
        Disposition::Neutral,
    ),
    (
        Form::VolatileBinding,
        "volatile binding",
        Disposition::Refused,
    ),
    (
        Form::VolatileValueFact,
        "volatile value fact",
        Disposition::Forgotten,
    ),
    (
        Form::VolatileRefinement,
        "volatile refinement",
        Disposition::Unrefined,
    ),
    (
        Form::VolatileDecision,
        "volatile decision",
        Disposition::Refused,
    ),
    (
        Form::AtomicAccess,
        "atomic access",
        Disposition::Owned("CG19"),
    ),
];

/// The disposition of a form by name; an unlisted name is refused
/// (`QCore.StorageDuration.dispositionOf`; the routes ask [`decide`]).
#[cfg(test)]
pub(crate) fn disposition(name: &str) -> Disposition {
    FORM_TABLE
        .iter()
        .find(|(_, spelled, _)| *spelled == name)
        .map_or(Disposition::Refused, |(_, _, disposition)| *disposition)
}

/// The disposition of a listed form.
pub(crate) fn decide(form: Form) -> Disposition {
    FORM_TABLE
        .iter()
        .find(|(listed, _, _)| *listed == form)
        .map_or(Disposition::Refused, |(_, _, disposition)| *disposition)
}

/// The refusal a `volatile` binding reports (rule 11h).
pub(crate) fn volatile_binding_refusal(name: &str) -> String {
    format!(
        "custody through the volatile object `{name}` is outside the resource projection: its value may change asynchronously (rule 11h)"
    )
}

/// Route T's refusal of a block-scope static- or thread-duration declaration:
/// no typed-source consumer decides a lifetime longer than the frame.
pub(crate) const T_STATIC_REFUSAL: &str =
    "typed-source static- or thread-duration local is decided by no typed-source consumer (CG15)";

/// The refusal a custody decision on a `volatile` read reports (rule 11h).
pub(crate) const VOLATILE_DECISION_REFUSAL: &str = "custody decision on a volatile read: the branch's arms differ in custody and the value may change asynchronously (rule 11h)";

/// Card CG15's facts about one function, read from its Clang node.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FunctionFacts {
    /// Every name an object of the function's scope is declared or referenced
    /// under whose own type is `volatile` at its top level. A name one object
    /// shadows under another type is included when any of them is volatile.
    pub(crate) volatile_objects: BTreeSet<String>,
    /// Every `(declaration, id)` of a block-scope static- or thread-duration
    /// object.
    pub(crate) static_locals: BTreeSet<String>,
    /// The lines of every condition that reads a `volatile` object: the
    /// statement's own line and its condition's line.
    pub(crate) volatile_conditions: Vec<VolatileCondition>,
}

/// A branch whose deciding operand reads a `volatile` object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VolatileCondition {
    /// The node kind (`IfStmt`, `WhileStmt`, …, `ConditionalOperator`).
    pub(crate) kind: String,
    /// The lines a route's branch node for it may carry.
    pub(crate) lines: BTreeSet<usize>,
}

/// Whether a node is a read of a `volatile` object: C's lvalue conversion of
/// a glvalue whose type is `volatile` at its top level.
fn volatile_read(node: &Value) -> bool {
    kind(node) == "ImplicitCastExpr"
        && node["castKind"].as_str() == Some("LValueToRValue")
        && children(node)
            .first()
            .and_then(crate::typed_access_path::qual)
            .is_some_and(top_level_volatile)
}

/// Whether a subtree reads a `volatile` object anywhere.
pub(crate) fn reads_volatile(node: &Value) -> bool {
    volatile_read(node) || children(node).iter().any(reads_volatile)
}

/// The operand a branching node decides on, by Clang's child layout.
fn deciding_operand(node: &Value) -> Option<&Value> {
    let inner = children(node);
    match kind(node) {
        // `if`, `while`, `switch`: the condition is the first expression
        // child after an optional declaration (Clang lists a condition
        // variable's `DeclStmt` first).
        "IfStmt" | "WhileStmt" | "SwitchStmt" => inner
            .iter()
            .find(|child| !matches!(kind(child), "DeclStmt" | "NullStmt")),
        // `do S while (C)`: the condition is the last child.
        "DoStmt" => inner.last(),
        // `for (I; C; N) S`: Clang's JSON lists the present children only, so
        // the condition is found as the one that is an expression and neither
        // the init statement nor the body or the increment; a `for` whose
        // clauses read volatile anywhere is treated as deciding on it.
        "ForStmt" => None,
        "ConditionalOperator" | "BinaryConditionalOperator" => inner.first(),
        "BinaryOperator" if matches!(node["opcode"].as_str(), Some("&&" | "||")) => inner.first(),
        _ => None,
    }
}

impl FunctionFacts {
    /// Read the facts of one `FunctionDecl` node; `line` gives a node's line.
    pub(crate) fn read(function: &Value, line: &dyn Fn(&Value) -> Option<usize>) -> Self {
        let mut facts = Self::default();
        facts.walk(function, false, line);
        facts
    }

    fn walk(&mut self, node: &Value, block: bool, line: &dyn Fn(&Value) -> Option<usize>) {
        match kind(node) {
            "VarDecl" | "ParmVarDecl" => {
                let text = crate::typed_access_path::qual(node).unwrap_or("");
                if let Some(name) = node["name"].as_str() {
                    if top_level_volatile(text) {
                        self.volatile_objects.insert(name.to_owned());
                    }
                    if block && storage_of(node, true).is_some_and(Storage::outlives_frame) {
                        self.static_locals.insert(name.to_owned());
                    }
                }
            }
            "DeclRefExpr" => {
                let reference = &node["referencedDecl"];
                if matches!(reference["kind"].as_str(), Some("VarDecl" | "ParmVarDecl"))
                    && reference["type"]["qualType"]
                        .as_str()
                        .is_some_and(top_level_volatile)
                    && let Some(name) = reference["name"].as_str()
                {
                    self.volatile_objects.insert(name.to_owned());
                }
            }
            _ => {}
        }
        let deciding = match kind(node) {
            "ForStmt" => Some(reads_volatile(node).then_some(node)).flatten(),
            _ => deciding_operand(node).filter(|operand| reads_volatile(operand)),
        };
        if let Some(operand) = deciding {
            let mut lines = BTreeSet::new();
            lines.extend(line(node));
            lines.extend(line(operand));
            self.volatile_conditions.push(VolatileCondition {
                kind: kind(node).to_owned(),
                lines,
            });
        }
        let block = block || kind(node) == "CompoundStmt";
        for child in children(node) {
            self.walk(child, block, line);
        }
    }

    /// Whether `name` may denote a `volatile` object.
    pub(crate) fn is_volatile(&self, name: &str) -> bool {
        self.volatile_objects.contains(root_name(name))
    }
}

/// The declaration ids of every formal and automatic local `function`
/// declares: a referenced object outside this set has static or thread
/// storage duration (a block-scope `static`/`extern`/thread-local, or a
/// file-scope object).
pub(crate) fn automatic_ids(function: &Value) -> BTreeSet<String> {
    fn walk(node: &Value, block: bool, out: &mut BTreeSet<String>) {
        let automatic = match kind(node) {
            "ParmVarDecl" => true,
            "VarDecl" => storage_of(node, block) == Some(Storage::Automatic),
            _ => false,
        };
        if automatic && let Some(id) = node["id"].as_str() {
            out.insert(id.to_owned());
        }
        let block = block || kind(node) == "CompoundStmt";
        for child in children(node) {
            walk(child, block, out);
        }
    }
    let mut out = BTreeSet::new();
    walk(function, false, &mut out);
    out
}

fn strip(mut node: &Value) -> &Value {
    while kind(node) == "ParenExpr"
        || (kind(node) == "ImplicitCastExpr" && node["castKind"].as_str() == Some("NoOp"))
    {
        match children(node).first() {
            Some(child) => node = child,
            None => break,
        }
    }
    node
}

/// Whether an index expression only reads: literals, enumerators, reads of
/// non-`volatile` scalar objects, arithmetic on them, and `sizeof` of a type
/// that is not variably modified (unevaluated). A call, a store, an
/// increment, a `volatile` read or anything else declines.
fn pure_index(node: &Value) -> bool {
    let inner = children(node);
    match kind(node) {
        "IntegerLiteral" | "CharacterLiteral" => true,
        "ParenExpr" | "ConstantExpr" => inner.iter().all(pure_index),
        "ImplicitCastExpr" | "CStyleCastExpr" => {
            matches!(
                node["castKind"].as_str(),
                Some("LValueToRValue" | "IntegralCast" | "NoOp")
            ) && !volatile_read(node)
                && inner.iter().all(pure_index)
        }
        "DeclRefExpr" => {
            matches!(
                node["referencedDecl"]["kind"].as_str(),
                Some("VarDecl" | "ParmVarDecl" | "EnumConstantDecl")
            ) && crate::typed_access_path::is_scalar(node)
        }
        "BinaryOperator" => {
            matches!(
                node["opcode"].as_str(),
                Some("+" | "-" | "*" | "/" | "%" | "&" | "|" | "^" | "<<" | ">>")
            ) && inner.iter().all(pure_index)
        }
        "UnaryOperator" => {
            matches!(node["opcode"].as_str(), Some("-" | "+" | "~")) && inner.iter().all(pure_index)
        }
        "UnaryExprOrTypeTraitExpr" => {
            node["name"].as_str() == Some("sizeof")
                && !crate::typed_access_path::qual(node)
                    .is_some_and(crate::cast_representation::variably_modified)
                && node["argType"]["qualType"]
                    .as_str()
                    .is_none_or(|text| !crate::cast_representation::variably_modified(text))
                && inner.iter().all(|operand| {
                    crate::typed_access_path::qual(operand)
                        .is_none_or(|text| !crate::cast_representation::variably_modified(text))
                })
        }
        _ => false,
    }
}

/// The root object of a place reached from an array or record object by
/// subscripts of an array (not a pointer) and `.` members only, with every
/// index [`pure_index`]; `None` for anything else.
fn object_place_root(node: &Value) -> Option<&Value> {
    let node = strip(node);
    let inner = children(node);
    match kind(node) {
        "DeclRefExpr" => Some(node),
        "MemberExpr" if node["isArrow"].as_bool() == Some(false) => {
            object_place_root(inner.first()?)
        }
        "ArraySubscriptExpr" => {
            let [base, index] = inner else { return None };
            let base = strip(base);
            // The base must be an array decaying to its first element, never
            // a pointer value read from memory.
            if !(kind(base) == "ImplicitCastExpr"
                && base["castKind"].as_str() == Some("ArrayToPointerDecay"))
                || !pure_index(index)
            {
                return None;
            }
            object_place_root(children(base).first()?)
        }
        _ => None,
    }
}

/// Whether a value is the address of (part of) a static- or thread-duration
/// object ([`Form::StaticAddress`]): an array of such an object decaying to
/// its first element, or `&` of a place in one, reached by array subscripts
/// and `.` members with side-effect-free indices. `automatic` is
/// [`automatic_ids`] of the function. Such a pointer carries no identity and
/// its object outlives every frame.
pub(crate) fn static_address(value: &Value, automatic: &BTreeSet<String>) -> bool {
    let value = strip(value);
    let place = match kind(value) {
        "ImplicitCastExpr" if value["castKind"].as_str() == Some("ArrayToPointerDecay") => {
            children(value).first()
        }
        "UnaryOperator" if value["opcode"].as_str() == Some("&") => children(value).first(),
        _ => None,
    };
    let Some(root) = place.and_then(object_place_root) else {
        return false;
    };
    let reference = &root["referencedDecl"];
    reference["kind"].as_str() == Some("VarDecl")
        && reference["id"]
            .as_str()
            .is_some_and(|id| !automatic.contains(id))
        && !reference["type"]["qualType"]
            .as_str()
            .is_some_and(|text| top_level_volatile(text) || atomic(text))
}

/// The storage-class keywords that open a block-scope static- or
/// thread-duration declaration.
pub(crate) const STATIC_STORAGE_KEYWORDS: &[&str] =
    &["static", "extern", "_Thread_local", "__thread"];

/// A copy of a function body's token texts in which the declarator part of
/// every block-scope static-duration declaration (a statement whose first
/// token, after at most one `const`/`volatile`, is `static`, `extern`,
/// `_Thread_local` or `__thread`) is masked: its initialization is not a
/// write the body performs (C11 6.7.9p4: a constant, evaluated before the
/// program starts). Only the declarators are masked; every token of an
/// initializer stays, so an `&` in one is still read.
pub(crate) fn mask_static_declarators<'a>(body: &[&'a str]) -> Vec<&'a str> {
    let mut masked = body.to_vec();
    let mut start = 0;
    while start < body.len() {
        let opens = |at: usize| {
            body.get(at)
                .is_some_and(|t| STATIC_STORAGE_KEYWORDS.contains(t))
        };
        let is_start = start == 0 || matches!(body[start - 1], ";" | "{" | "}");
        let declaration = is_start
            && (opens(start) || (matches!(body[start], "const" | "volatile") && opens(start + 1)));
        if !declaration {
            start += 1;
            continue;
        }
        // Mask declarator tokens up to each depth-0 `=`, keep the initializer
        // up to the next depth-0 `,` or `;`.
        let mut depth = 0usize;
        let mut in_initializer = false;
        let mut at = start;
        while at < body.len() {
            match body[at] {
                "(" | "[" | "{" => depth += 1,
                ")" | "]" | "}" => depth = depth.saturating_sub(1),
                "=" if depth == 0 => in_initializer = true,
                "," if depth == 0 => in_initializer = false,
                ";" if depth == 0 => break,
                text if !in_initializer
                    && text
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_') =>
                {
                    masked[at] = "0";
                }
                _ => {}
            }
            at += 1;
        }
        start = at + 1;
    }
    masked
}

/// The root object a route-G place names: `p` of `p`, `p->f`, `p.f`, `*p`,
/// `p[0]`.
pub(crate) fn root_name(place: &str) -> &str {
    let place = place.trim_start_matches(['*', '&', '(']);
    let end = place
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(place.len());
    &place[..end]
}

/// One `volatile` access of a trace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VolatileAccess {
    /// The spelling of the object's root (`v` of `*v`, `r` of `s.r`).
    pub(crate) object: String,
    pub(crate) write: bool,
}

/// The observable `volatile` effects of one full expression
/// (`QCore.StorageDuration.Trace`): every access once, in one permitted
/// serialisation, with the pairs C sequences and the pairs it leaves
/// unsequenced. No left-to-right order is chosen: an unsequenced pair is
/// listed, never ordered.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Trace {
    pub(crate) accesses: Vec<VolatileAccess>,
    /// `(a, b)`: access `a` is sequenced before access `b`.
    pub(crate) sequenced: Vec<(usize, usize)>,
    /// `(a, b)`, `a < b`: C leaves the two unsequenced.
    pub(crate) unsequenced: Vec<(usize, usize)>,
}

impl Trace {
    /// Whether two unsequenced accesses may touch one object (C11 6.5p2:
    /// both are side effects on it).
    pub(crate) fn unsequenced_conflict(&self) -> Option<&str> {
        self.unsequenced.iter().find_map(|&(a, b)| {
            (self.accesses[a].object == self.accesses[b].object)
                .then_some(self.accesses[a].object.as_str())
        })
    }
}

/// The root spelling of an lvalue.
fn lvalue_root(node: &Value) -> String {
    let mut node = node;
    loop {
        match kind(node) {
            "DeclRefExpr" => {
                return node["referencedDecl"]["name"]
                    .as_str()
                    .unwrap_or("?")
                    .to_owned();
            }
            _ => match children(node).first() {
                Some(child) => node = child,
                None => return "?".to_owned(),
            },
        }
    }
}

/// The `volatile` accesses of one subtree, with the order C gives them. The
/// order model is card CG02's: operands of an operator or call are
/// unsequenced, `,` `&&` `||` `?:` and a full expression sequence what
/// precedes them, an assignment's store follows its operands' value
/// computations.
pub(crate) fn volatile_trace(node: &Value) -> Trace {
    let mut trace = Trace::default();
    collect(node, &mut trace);
    trace
}

/// Collect `node`'s accesses into `trace`, returning their indices.
fn collect(node: &Value, trace: &mut Trace) -> Vec<usize> {
    let inner = children(node);
    let own = |trace: &mut Trace, write: bool, target: &Value| {
        trace.accesses.push(VolatileAccess {
            object: lvalue_root(target),
            write,
        });
        trace.accesses.len() - 1
    };
    let sequence = |trace: &mut Trace, first: &[usize], then: &[usize]| {
        for &a in first {
            for &b in then {
                trace.sequenced.push((a, b));
            }
        }
    };
    let unsequenced = |trace: &mut Trace, groups: &[Vec<usize>]| {
        for (i, one) in groups.iter().enumerate() {
            for other in &groups[i + 1..] {
                for &a in one {
                    for &b in other {
                        trace.unsequenced.push((a.min(b), a.max(b)));
                    }
                }
            }
        }
    };
    if volatile_read(node) {
        let mut below = collect(&inner[0], trace);
        let read = own(trace, false, &inner[0]);
        sequence(trace, &below, &[read]);
        below.push(read);
        return below;
    }
    match (kind(node), node["opcode"].as_str()) {
        ("BinaryOperator", Some("," | "&&" | "||")) | ("ConditionalOperator", _) => {
            let mut all: Vec<usize> = Vec::new();
            for child in inner {
                let part = collect(child, trace);
                sequence(trace, &all, &part);
                all.extend(part);
            }
            all
        }
        ("BinaryOperator", Some("=")) | ("CompoundAssignOperator", _) => {
            let place = collect(&inner[0], trace);
            let value = collect(&inner[1], trace);
            unsequenced(trace, &[place.clone(), value.clone()]);
            let target = &inner[0];
            let mut all = place;
            all.extend(value);
            if crate::typed_access_path::qual(target).is_some_and(top_level_volatile) {
                let write = own(trace, true, target);
                sequence(trace, &all, &[write]);
                all.push(write);
            }
            all
        }
        ("UnaryOperator", Some("++" | "--")) => {
            let target = &inner[0];
            let mut all = collect(target, trace);
            if crate::typed_access_path::qual(target).is_some_and(top_level_volatile) {
                let read = own(trace, false, target);
                let write = own(trace, true, target);
                sequence(trace, &all, &[read, write]);
                sequence(trace, &[read], &[write]);
                all.extend([read, write]);
            }
            all
        }
        ("CompoundStmt", _) => {
            let mut all: Vec<usize> = Vec::new();
            for child in inner {
                let part = collect(child, trace);
                sequence(trace, &all, &part);
                all.extend(part);
            }
            all
        }
        _ => {
            let groups = inner
                .iter()
                .map(|child| collect(child, trace))
                .collect::<Vec<_>>();
            unsequenced(trace, &groups);
            groups.into_iter().flatten().collect()
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/storage_duration.rs"]
mod tests;
