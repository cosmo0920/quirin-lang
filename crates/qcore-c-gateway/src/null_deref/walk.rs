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

//! The null consumer's walk of one function body (gateway rules 14k–14m,
//! 14o–14p): a forward, path-joining reading of the typed AST in which every
//! dereference is a check ([`qcore_core::null_deref::check`]).

use std::collections::{BTreeMap, BTreeSet};

use qcore_core::null_deref::{self as core, First, Nullness, Outcome, Proof, Source};
use qcore_schemas::contract::{ResourceContracts, ResourceReturnMode};
use serde_json::Value;

use super::ast::{self, callee_name, is_pointer, kids, render, ty};
use super::scope::{self, ScopeDecl, StableDecl, StableStore};
use crate::typed_c_ast::kind;

/// A loop's fixed point is sought this many times before the function is
/// refused (fail-closed).
const LOOP_ITERATIONS: usize = 16;

/// Where a value lives, when the walk can name it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Place {
    /// A tracked local or formal, by declaration id.
    Var(String),
    /// A memory path rooted at a variable: `ROOT(^.FIELD|^|.FIELD)*`.
    Path(String),
}

/// A value as the walk reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Val {
    pub(super) null: Nullness,
    /// The formal whose entry value this is (or an offset of), for the
    /// inference of the clause (rule 14p).
    pub(super) origin: Option<usize>,
    /// Where the value was read from, when it is exactly that place's value.
    pub(super) place: Option<Place>,
    /// A null pointer constant (for `p == NULL`).
    pub(super) null_constant: bool,
    /// A path fact that survived a non-character scalar store (C11 6.5p7).
    pub(super) effective_type: bool,
    /// The value's identity: every tracked variable holding the same value
    /// (a copy `q = p`, or an offset of it) carries the same one, so a test
    /// or a dereference of one decides them all. A new one at each other
    /// assignment.
    pub(super) ident: Option<usize>,
}

impl Val {
    fn of(null: Nullness) -> Self {
        Self {
            null,
            origin: None,
            place: None,
            null_constant: false,
            effective_type: false,
            ident: None,
        }
    }

    fn scalar() -> Self {
        Self::of(Nullness::maybe(Source::Other("scalar".into())))
    }

    fn address() -> Self {
        Self::of(Nullness::proved(Proof::Address))
    }

    /// `self ± n`: the same nullness and origin, no longer the place's value.
    fn offset(&self) -> Self {
        Self {
            null: core::offset(&self.null),
            origin: self.origin,
            place: None,
            null_constant: false,
            effective_type: self.effective_type,
            ident: self.ident,
        }
    }

    fn join(&self, other: &Self) -> Self {
        Self {
            null: self.null.join(&other.null),
            origin: if self.origin == other.origin {
                self.origin
            } else {
                None
            },
            place: if self.place == other.place {
                self.place.clone()
            } else {
                None
            },
            null_constant: self.null_constant && other.null_constant,
            effective_type: self.effective_type || other.effective_type,
            ident: if self.ident == other.ident {
                self.ident
            } else {
                None
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PathFact {
    null: Nullness,
    effective_type: bool,
}

/// The facts on one set of reaching paths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct State {
    live: bool,
    /// Every reaching path is the straight-line prefix from entry.
    definite: bool,
    vars: BTreeMap<String, Val>,
    paths: BTreeMap<String, PathFact>,
    /// Per formal, what the reaching paths did first with its entry value.
    firsts: Vec<BTreeSet<First>>,
}

impl State {
    fn dead(formals: usize) -> Self {
        Self {
            live: false,
            definite: false,
            vars: BTreeMap::new(),
            paths: BTreeMap::new(),
            firsts: vec![BTreeSet::new(); formals],
        }
    }

    fn join(&self, other: &Self) -> Self {
        if !self.live {
            return other.clone();
        }
        if !other.live {
            return self.clone();
        }
        let vars = self
            .vars
            .iter()
            .filter_map(|(id, value)| other.vars.get(id).map(|o| (id.clone(), value.join(o))))
            .collect();
        let paths = self
            .paths
            .iter()
            .filter_map(|(key, fact)| {
                other.paths.get(key).map(|o| {
                    (
                        key.clone(),
                        PathFact {
                            null: fact.null.join(&o.null),
                            effective_type: fact.effective_type || o.effective_type,
                        },
                    )
                })
            })
            .collect();
        let firsts = self
            .firsts
            .iter()
            .zip(&other.firsts)
            .map(|(a, b)| a.union(b).copied().collect())
            .collect();
        Self {
            live: true,
            definite: false,
            vars,
            paths,
            firsts,
        }
    }

    fn not_definite(mut self) -> Self {
        self.definite = false;
        self
    }
}

/// One recorded check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CheckRecord {
    pub(super) id: usize,
    /// Card NULL-CANDIDATE-TRIAGE: the checked node's AST key (the witness
    /// search finds the dereference by it).
    pub(super) key: String,
    pub(super) kind: &'static str,
    pub(super) line: usize,
    pub(super) subject: String,
    pub(super) outcome: Outcome,
}

/// A declared `quant null-deref` clause set of one function, as callers and
/// the body see it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Declared {
    pub(super) parameters: usize,
    /// Position -> the clause text.
    pub(super) positions: BTreeMap<usize, String>,
    /// The hold, when the declaration is not in force.
    pub(super) held: Option<String>,
}

/// An in-unit summary (rule 14p): the inferred clause positions and whether
/// every return is non-null.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Summary {
    pub(super) derefs: BTreeSet<usize>,
    pub(super) returns_non_null: bool,
    /// Card NULL-CANDIDATE-TRIAGE: the declarations the summary rests on
    /// (a caller that reads it rests on them too).
    pub(super) dependencies: BTreeSet<String>,
}

/// Card NULL-CANDIDATE-TRIAGE (rule 14t): a declared non-null member.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MemberDecl {
    /// The declaration that carries it.
    pub(super) host: String,
    /// The hold, when the declaration is not in force.
    pub(super) held: Option<String>,
}

/// Card NULL-CANDIDATE-TRIAGE (rule 14u): a declared non-null result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ResultDecl {
    pub(super) held: Option<String>,
}

/// Card NULL-CANDIDATE-TRIAGE: one store to a declared member.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct MemberStore {
    /// `TAG.FIELD`.
    pub member: String,
    pub function: String,
    pub line: usize,
    /// The stored value's check class (`proved`, `moved`, `candidate`,
    /// `violation`, ...): the store keeps the declaration only when it is
    /// discharged.
    pub outcome: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
    /// Card NULL-MEMBER-INVARIANT-SCOPE: the stable members the stored
    /// value's proof rests on (absent when none).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub relies: Vec<String>,
}

/// Card NULL-CANDIDATE-TRIAGE (rule 14v): a pointer actual of a call to a
/// function this unit neither defines nor declares, judged as a check would
/// be, for cross-unit travel.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ExternalCall {
    pub function: String,
    pub callee: String,
    pub position: usize,
    pub arguments: usize,
    pub line: usize,
    pub subject: String,
    pub outcome: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub proofs: Vec<String>,
}

/// The C struct tag a member access reads (`struct TAG *` for `->`,
/// `struct TAG` for `.`), from the base's type spelling, through the unit's
/// typedefs (`T_t *` with `typedef struct T_s T_t`).
pub(super) fn record_tag(unit: &ast::Unit, base: &str, arrow: bool) -> Option<String> {
    let words = |text: &str| {
        text.replace('*', " * ")
            .split_whitespace()
            .filter(|w| !matches!(*w, "const" | "volatile" | "restrict" | "__restrict"))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    let mut record = words(base);
    if arrow {
        if record.last().map(String::as_str) != Some("*") {
            return None;
        }
        record.pop();
        if record.contains(&"*".to_owned()) {
            return None;
        }
    }
    for _ in 0..8 {
        match record.as_slice() {
            [s, tag] if s == "struct" => return Some(tag.clone()),
            [name] => record = words(unit.typedefs.get(name)?),
            _ => return None,
        }
    }
    None
}

/// What the walk reads besides the body.
pub(super) struct Env<'a> {
    pub(super) unit: &'a ast::Unit,
    pub(super) resource: &'a ResourceContracts,
    /// Checked custody results require this property's independent source proof.
    pub(super) checked_results: &'a BTreeSet<String>,
    pub(super) declared: &'a BTreeMap<String, Declared>,
    pub(super) summaries: &'a BTreeMap<String, Summary>,
    pub(super) newlines: &'a [usize],
    /// Card NULL-CANDIDATE-TRIAGE: declared non-null members, by (tag, field).
    pub(super) members: &'a BTreeMap<(String, String), MemberDecl>,
    /// Card NULL-CANDIDATE-TRIAGE: declared non-null results, by function.
    pub(super) results: &'a BTreeMap<String, ResultDecl>,
    /// Card NULL-MEMBER-INVARIANT-SCOPE: stable members, by (tag, field).
    pub(super) stable: &'a BTreeMap<(String, String), StableDecl>,
    /// Card NULL-MEMBER-INVARIANT-SCOPE: function-scope entries, by function.
    pub(super) scopes: &'a BTreeMap<String, ScopeDecl>,
    /// Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15q): declared integer result
    /// ranges, by function (the path search reads them as bounds).
    pub(super) ranges: &'a BTreeMap<String, super::range::RangeDecl>,
    /// Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15q): this unit's validated
    /// summaries of integer result ranges, by function.
    pub(super) range_summaries: &'a BTreeMap<String, (i128, i128)>,
}

/// The entry fact of a formal under a clause.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Entry {
    Declared(String),
    Held(String),
    Inferred(String),
}

struct LoopFrame {
    breaks: State,
    continues: State,
    /// A switch's frame: a `break` ends it, a `continue` passes through.
    switch: bool,
}

/// What one walk found.
pub(super) struct Walked {
    pub(super) checks: Vec<CheckRecord>,
    pub(super) reasons: Vec<String>,
    pub(super) assumptions: BTreeSet<String>,
    /// Per formal, what every terminating path did first.
    pub(super) exits: Vec<BTreeSet<First>>,
    pub(super) returns: Option<Nullness>,
    /// Declared and inferred callees whose clause an actual met.
    pub(super) consulted: BTreeSet<String>,
    /// Calls through a function pointer (not inventoried as checks).
    pub(super) indirect_calls: usize,
    /// Card NULL-CANDIDATE-TRIAGE: stores to declared members.
    pub(super) member_stores: Vec<MemberStore>,
    /// Card NULL-CANDIDATE-TRIAGE: pointer actuals of external calls.
    pub(super) external_calls: Vec<ExternalCall>,
    /// Card NULL-MEMBER-INVARIANT-SCOPE: stores to stable members.
    pub(super) stable_stores: Vec<StableStore>,
    /// The function's returns, joined (for a declared result's check).
    pub(super) returns_null_on_a_path: bool,
}

pub(super) struct Walker<'a> {
    env: &'a Env<'a>,
    function: String,
    formals: Vec<(String, String, bool)>,
    tracked: BTreeSet<String>,
    locals: BTreeSet<String>,
    recording: bool,
    sites: BTreeMap<String, usize>,
    checks: BTreeMap<usize, CheckRecord>,
    reasons: Vec<String>,
    assumptions: BTreeSet<String>,
    loops: Vec<LoopFrame>,
    /// Each enclosing switch's entry state and the loop depth it was entered at.
    switches: Vec<(State, usize)>,
    gotos: BTreeMap<String, State>,
    backward: BTreeSet<String>,
    exits: Vec<BTreeSet<First>>,
    returns: Option<Nullness>,
    consulted: BTreeSet<String>,
    indirect_calls: BTreeSet<usize>,
    member_stores: BTreeMap<usize, MemberStore>,
    external_calls: BTreeMap<(usize, usize), ExternalCall>,
    /// Card NULL-MEMBER-INVARIANT-SCOPE: each tracked pointer variable's
    /// pointee struct tag, and the stores to stable members.
    pointee: BTreeMap<String, String>,
    stable_stores: BTreeMap<(usize, String), StableStore>,
}

/// The AST key of a node (its id, else its offset).
pub(super) fn node_key(node: &Value) -> String {
    node["id"]
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("@{}", node["o"]))
}

fn strip(node: &Value) -> &Value {
    let mut node = node;
    while matches!(
        kind(node),
        "ParenExpr" | "ConstantExpr" | "ExprWithCleanups"
    ) || (kind(node) == "UnaryOperator" && node["opcode"] == "__extension__")
    {
        match kids(node).next() {
            Some(inner) => node = inner,
            None => break,
        }
    }
    node
}

fn present(node: &Value) -> bool {
    node.as_object().is_some_and(|o| o.contains_key("kind"))
}

impl<'a> Walker<'a> {
    /// Analyse `function`'s body with `entries` in force at its formals.
    pub(super) fn walk(
        env: &'a Env<'a>,
        name: &str,
        function: &Value,
        entries: &BTreeMap<usize, Entry>,
    ) -> Walked {
        let formals = kids(function)
            .filter(|c| kind(c) == "ParmVarDecl")
            .map(|c| {
                (
                    c["id"].as_str().unwrap_or("").to_owned(),
                    c["name"].as_str().unwrap_or("").to_owned(),
                    is_pointer(ty(c)),
                )
            })
            .collect::<Vec<_>>();
        let body = kids(function).find(|c| kind(c) == "CompoundStmt");
        let mut taken = BTreeSet::new();
        let mut locals = BTreeSet::new();
        let mut tracked = BTreeSet::new();
        if let Some(body) = body {
            ast::address_taken(body, &mut taken);
        }
        fn declared_locals(node: &Value, out: &mut Vec<Value>) {
            if kind(node) == "VarDecl" {
                out.push(node.clone());
            }
            for child in kids(node) {
                declared_locals(child, out);
            }
        }
        let mut declared = Vec::new();
        if let Some(body) = body {
            declared_locals(body, &mut declared);
        }
        for (id, _, pointer) in &formals {
            locals.insert(id.clone());
            if *pointer && !taken.contains(id) {
                tracked.insert(id.clone());
            }
        }
        for decl in &declared {
            let Some(id) = decl["id"].as_str() else {
                continue;
            };
            let storage = decl["storageClass"].as_str();
            if matches!(storage, Some("static" | "extern")) {
                continue;
            }
            locals.insert(id.to_owned());
            let spelled = ty(decl);
            if is_pointer(spelled) && !spelled.contains("volatile") && !taken.contains(id) {
                tracked.insert(id.to_owned());
            }
        }
        // Card NULL-MEMBER-INVARIANT-SCOPE: the struct a tracked pointer
        // variable points to, when it is one with a stable member.
        let mut pointee = BTreeMap::new();
        if !env.stable.is_empty() {
            let typed = kids(function)
                .filter(|c| kind(c) == "ParmVarDecl")
                .chain(declared.iter());
            for decl in typed {
                let Some(id) = decl["id"].as_str() else {
                    continue;
                };
                if tracked.contains(id)
                    && let Some(tag) = record_tag(env.unit, ty(decl), true)
                    && env.stable.keys().any(|(t, _)| *t == tag)
                {
                    pointee.insert(id.to_owned(), tag);
                }
            }
        }
        let mut backward = BTreeSet::new();
        if let Some(body) = body {
            let mut order = 0;
            let mut labels = BTreeMap::new();
            let mut gotos = Vec::new();
            ast::labels_and_gotos(body, &mut order, &mut labels, &mut gotos);
            for (label, at) in gotos {
                if labels.get(&label).is_some_and(|defined| *defined < at) {
                    backward.insert(label);
                }
            }
        }
        let count = formals.len();
        let mut walker = Walker {
            env,
            function: name.to_owned(),
            formals,
            tracked,
            locals,
            recording: true,
            sites: BTreeMap::new(),
            checks: BTreeMap::new(),
            reasons: Vec::new(),
            assumptions: BTreeSet::new(),
            loops: Vec::new(),
            switches: Vec::new(),
            gotos: BTreeMap::new(),
            backward,
            exits: vec![BTreeSet::new(); count],
            returns: None,
            consulted: BTreeSet::new(),
            indirect_calls: BTreeSet::new(),
            member_stores: BTreeMap::new(),
            external_calls: BTreeMap::new(),
            pointee,
            stable_stores: BTreeMap::new(),
        };
        let mut state = State {
            live: true,
            definite: true,
            vars: BTreeMap::new(),
            paths: BTreeMap::new(),
            firsts: vec![BTreeSet::from([First::Pending]); count],
        };
        let parameter_sites = kids(function)
            .filter(|c| kind(c) == "ParmVarDecl")
            .map(|c| walker.site(c))
            .collect::<Vec<_>>();
        for (index, (id, pname, pointer)) in walker.formals.clone().iter().enumerate() {
            if !*pointer {
                continue;
            }
            let null = match entries.get(&index) {
                Some(Entry::Declared(text)) => {
                    walker
                        .assumptions
                        .insert(format!("declared: `{name}` {text}"));
                    Nullness::proved(Proof::Precondition(format!("declared: {text}")))
                }
                Some(Entry::Held(hold)) => {
                    Nullness::proved(Proof::Precondition(format!("held: {hold}")))
                }
                Some(Entry::Inferred(text)) => {
                    walker
                        .assumptions
                        .insert(format!("inferred: `{name}` {text}"));
                    Nullness::proved(Proof::Precondition(format!("inferred: {text}")))
                }
                None => Nullness::maybe(Source::Formal(pname.clone())),
            };
            if walker.tracked.contains(id) {
                state.vars.insert(
                    id.clone(),
                    Val {
                        null,
                        origin: Some(index),
                        place: Some(Place::Var(id.clone())),
                        null_constant: false,
                        effective_type: false,
                        ident: parameter_sites.get(index).copied(),
                    },
                );
            } else {
                state.paths.insert(
                    format!("g:{id}"),
                    PathFact {
                        null,
                        effective_type: false,
                    },
                );
            }
        }
        // Card NULL-MEMBER-INVARIANT-SCOPE (rule 14z): a declared scope
        // entry's member is non-null at entry (held when the declaration is).
        if let Some(scope) = env.scopes.get(name)
            && scope.parameters == walker.formals.len()
        {
            for entry in &scope.entries {
                let Some((id, _, true)) = walker.formals.get(entry.position).cloned() else {
                    continue;
                };
                let proof = match &scope.held {
                    Some(hold) => Proof::Precondition(format!("held: {hold}")),
                    None => {
                        walker
                            .assumptions
                            .insert(format!("declared: `{name}` {}", entry.text));
                        Proof::Precondition(format!("declared: {}", entry.text))
                    }
                };
                state.paths.insert(
                    format!("{}^.{}", walker.var_root(&id), entry.field),
                    PathFact {
                        null: Nullness::proved(proof),
                        effective_type: false,
                    },
                );
            }
        }
        match body {
            Some(body) => {
                let end = walker.stmt(body, state);
                if end.live {
                    walker.exit(&end);
                }
            }
            None => walker.reason("null-import-failure: the definition has no body".into()),
        }
        let returns_null_on_a_path = match &walker.returns {
            Some(Nullness::Null) => true,
            Some(Nullness::Maybe(sources)) => sources.contains(&Source::NullLiteral),
            _ => false,
        };
        Walked {
            checks: walker.checks.into_values().collect(),
            reasons: walker.reasons,
            assumptions: walker.assumptions,
            exits: walker.exits,
            returns: walker.returns,
            consulted: walker.consulted,
            indirect_calls: walker.indirect_calls.len(),
            member_stores: walker.member_stores.into_values().collect(),
            external_calls: walker.external_calls.into_values().collect(),
            stable_stores: walker.stable_stores.into_values().collect(),
            returns_null_on_a_path,
        }
    }

    fn reason(&mut self, reason: String) {
        if !self.reasons.contains(&reason) {
            self.reasons.push(reason);
        }
    }

    fn line(&self, node: &Value) -> usize {
        node["o"]
            .as_u64()
            .map(|offset| {
                let offset = offset as usize;
                self.env.newlines.partition_point(|n| *n < offset) + 1
            })
            .unwrap_or(0)
    }

    fn site(&mut self, node: &Value) -> usize {
        let key = node["id"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("@{}", node["o"]));
        let next = self.sites.len();
        *self.sites.entry(key).or_insert(next)
    }

    fn exit(&mut self, state: &State) {
        for (index, firsts) in state.firsts.iter().enumerate() {
            self.exits[index].extend(firsts.iter().copied());
        }
    }

    // -----------------------------------------------------------------------
    // Places and facts
    // -----------------------------------------------------------------------

    fn set_place(state: &mut State, place: &Place, null: Nullness) {
        match place {
            Place::Var(id) => {
                if let Some(value) = state.vars.get_mut(id) {
                    value.null = null;
                }
            }
            Place::Path(key) => {
                state.paths.insert(
                    key.clone(),
                    PathFact {
                        null,
                        effective_type: false,
                    },
                );
            }
        }
    }

    /// Give `value`'s place, and every tracked variable holding the same
    /// value, the null-ness `null`.
    fn set_value(state: &mut State, value: &Val, null: Nullness) {
        if let Some(ident) = value.ident {
            for held in state.vars.values_mut() {
                if held.ident == Some(ident) {
                    held.null = null.clone();
                }
            }
        }
        if let Some(place) = &value.place {
            Self::set_place(state, place, null);
        }
    }

    /// Every path rooted at the variable `id` is forgotten (it was written).
    fn forget_rooted(state: &mut State, id: &str) {
        let root = format!("t:{id}");
        state.paths.retain(|key, _| {
            !(key == &root
                || key.starts_with(&format!("{root}^"))
                || key.starts_with(&format!("{root}.")))
        });
    }

    /// A store to memory (or a call, or an asm): every path fact is
    /// forgotten, except across a non-character scalar store, which cannot
    /// modify a pointer object (C11 6.5p7): those facts are kept, marked.
    ///
    /// Card NULL-MEMBER-INVARIANT-SCOPE (rule 14y): a non-null fact on
    /// `V->FIELD`, `V` a tracked variable pointing to a `struct TAG` whose
    /// `FIELD` is a stable member in force, is kept (every store that could
    /// make it NULL is checked), with the member's proof added.
    fn memory_written(&self, state: &mut State, wide_scalar: bool) {
        if wide_scalar {
            for fact in state.paths.values_mut() {
                fact.effective_type = true;
            }
            return;
        }
        let kept = std::mem::take(&mut state.paths)
            .into_iter()
            .filter_map(|(key, fact)| {
                let member = self.stable_member_of_key(&key)?;
                let null = qcore_core::null_member_scope::persist(&fact.null, &member, true)?;
                Some((
                    key,
                    PathFact {
                        null,
                        effective_type: fact.effective_type,
                    },
                ))
            })
            .collect();
        state.paths = kept;
    }

    /// Card NULL-MEMBER-INVARIANT-SCOPE: the stable member in force
    /// (`TAG.FIELD`) a persistable path key names, if any.
    fn stable_member_of_key(&self, key: &str) -> Option<String> {
        let (id, field) = scope::persistable_key(key)?;
        let tag = self.pointee.get(id)?;
        self.env
            .stable
            .get(&(tag.clone(), field.to_owned()))
            .filter(|decl| decl.held.is_none())
            .map(|_| format!("{tag}.{field}"))
    }

    fn var_root(&self, id: &str) -> String {
        if self.tracked.contains(id) {
            format!("t:{id}")
        } else {
            format!("g:{id}")
        }
    }

    /// The value read from `place`.
    fn read_place(&self, place: &Place, text: &str, state: &State) -> Val {
        match place {
            Place::Var(id) => state.vars.get(id).cloned().unwrap_or_else(|| Val {
                place: Some(place.clone()),
                ..Val::of(Nullness::maybe(Source::Uninitialized(text.to_owned())))
            }),
            Place::Path(key) => {
                let (null, effective_type) = match state.paths.get(key) {
                    Some(fact) => (fact.null.clone(), fact.effective_type),
                    None => {
                        let source = if let Some(id) = key.strip_prefix("g:")
                            && !key.contains(['^', '.'])
                        {
                            if self.locals.contains(id) {
                                Source::MemoryField(text.to_owned())
                            } else {
                                Source::Global(text.to_owned())
                            }
                        } else {
                            Source::MemoryField(text.to_owned())
                        };
                        (Nullness::maybe(source), false)
                    }
                };
                Val {
                    null,
                    origin: None,
                    place: Some(place.clone()),
                    null_constant: false,
                    effective_type,
                    ident: None,
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Checks
    // -----------------------------------------------------------------------

    /// The check at one dereference of `value` (`node` is the dereferencing
    /// expression, `subject` the pointer expression). Returns its id.
    fn check(
        &mut self,
        node: &Value,
        check_kind: &'static str,
        subject: &Value,
        value: &Val,
        state: &mut State,
    ) -> usize {
        let id = self.site(node);
        let outcome = if !state.live {
            Outcome::Excluded
        } else {
            let decided = core::check(&value.null, state.definite);
            match (&decided, &value.null) {
                (Outcome::Proved(_) | Outcome::Moved(_), Nullness::NonNull(proofs))
                    if proofs.iter().all(
                        |p| matches!(p, Proof::Precondition(text) if text.starts_with("held: ")),
                    ) =>
                {
                    let Some(Proof::Precondition(text)) = proofs.first() else {
                        unreachable!("a non-empty proof set")
                    };
                    Outcome::Held(text.trim_start_matches("held: ").to_owned())
                }
                _ => decided,
            }
        };
        if self.recording {
            if outcome.discharged() {
                self.note_assumptions(&value.null, value.effective_type);
            }
            let record = CheckRecord {
                id,
                key: node_key(node),
                kind: check_kind,
                line: self.line(node),
                subject: render(subject),
                outcome: outcome.clone(),
            };
            match self.checks.get_mut(&id) {
                Some(existing) if existing.outcome != outcome => {
                    existing.outcome = worse(&existing.outcome, &outcome);
                }
                Some(_) => {}
                None => {
                    self.checks.insert(id, record);
                }
            }
        }
        if state.live {
            if let Some(formal) = value.origin {
                state.firsts[formal] = state.firsts[formal]
                    .iter()
                    .map(|f| f.dereference())
                    .collect();
            }
            if value.place.is_some() || value.ident.is_some() {
                let after = core::after_dereference(&value.null, id);
                Self::set_value(state, value, after);
            }
        }
        id
    }

    fn note_assumptions(&mut self, null: &Nullness, effective_type: bool) {
        if let Nullness::NonNull(proofs) = null {
            for proof in proofs {
                match proof {
                    Proof::Asserted(line) => {
                        self.assumptions.insert(format!(
                            "asserted: the assertion at line {line} holds (NDEBUG absent, as in the analysed text)"
                        ));
                    }
                    Proof::NonNullResult(name) if name.starts_with("declared:") => {
                        self.assumptions
                            .insert(format!("{name}: a non-null result (resource contract)"));
                    }
                    Proof::NonNullResult(name) if name.starts_with("member:") => {
                        self.assumptions.insert(format!(
                            "declared-{name}: a non-null member (nonnull_member; every analysed store checked, construction premised)"
                        ));
                    }
                    Proof::NonNullResult(name) if name.starts_with("declared-result:") => {
                        self.assumptions.insert(format!(
                            "{name}: a non-null result (nonnull_result; checked against the analysed bodies)"
                        ));
                    }
                    // Card NULL-MEMBER-INVARIANT-SCOPE (rule 14y): a fact
                    // kept across a call or a store by a stable member. The
                    // label names the fact kind, kept byte for byte by card
                    // NONNULL-MEMBER-UNIFIED-QUALIFIER (the clause is
                    // `nonnull_member(TAG, FIELD) where quant #scope
                    // [[stable]]`).
                    Proof::NonNullResult(name) if name.starts_with("stable:") => {
                        self.assumptions.insert(format!(
                            "{name}: a non-null member kept (nonnull_member_stable; every store to an existing object checked, whole-object writes premised)"
                        ));
                        self.consulted.insert(name.clone());
                    }
                    _ => {}
                }
            }
            if effective_type {
                self.assumptions.insert(
                    "effective-type: a non-character scalar store does not modify a pointer object (C11 6.5p7)".into(),
                );
            }
        }
    }

    /// A NULL test of `value` on both arms.
    fn refine(&mut self, value: &Val, state: State, proof: Proof) -> (State, State) {
        let mut on_true = state.clone().not_definite();
        let mut on_false = state.not_definite();
        if let Some(formal) = value.origin {
            for arm in [&mut on_true, &mut on_false] {
                if arm.live {
                    arm.firsts[formal] = arm.firsts[formal].iter().map(|f| f.test()).collect();
                }
            }
        }
        // An arm the value contradicts is infeasible whether or not the walk
        // can name where the value lives.
        match core::refine(&value.null, true, proof.clone()) {
            Some(null) => Self::set_value(&mut on_true, value, null),
            None => on_true.live = false,
        }
        match core::refine(&value.null, false, proof) {
            Some(null) => Self::set_value(&mut on_false, value, null),
            None => on_false.live = false,
        }
        (on_true, on_false)
    }

    // -----------------------------------------------------------------------
    // Expressions
    // -----------------------------------------------------------------------

    /// The place an lvalue designates, after the checks its evaluation makes.
    fn lvalue(&mut self, node: &Value, state: &mut State) -> Option<Place> {
        let node = strip(node);
        match kind(node) {
            "DeclRefExpr" => {
                let decl = &node["referencedDecl"];
                let id = decl["id"].as_str()?;
                match decl["kind"].as_str() {
                    Some("VarDecl" | "ParmVarDecl") => {
                        if self.tracked.contains(id) {
                            Some(Place::Var(id.to_owned()))
                        } else {
                            Some(Place::Path(format!("g:{id}")))
                        }
                    }
                    _ => None,
                }
            }
            "MemberExpr" => {
                let base = kids(node).next()?;
                let field = node["name"].as_str().unwrap_or("?");
                if node["isArrow"] == true {
                    let value = self.eval(base, state);
                    self.check(node, "member-access", base, &value, state);
                    value
                        .place
                        .as_ref()
                        .map(|place| Place::Path(format!("{}^.{field}", self.place_key(place))))
                } else {
                    let place = self.lvalue(base, state)?;
                    Some(Place::Path(format!("{}.{field}", self.place_key(&place))))
                }
            }
            "UnaryOperator" if node["opcode"] == "*" => {
                let operand = kids(node).next()?;
                let value = self.eval(operand, state);
                self.check(node, "dereference", operand, &value, state);
                value
                    .place
                    .as_ref()
                    .map(|place| Place::Path(format!("{}^", self.place_key(place))))
            }
            "ArraySubscriptExpr" => {
                self.element(node, state);
                None
            }
            // C11 6.5.1.1: only the selected association is evaluated.
            "GenericSelectionExpr" => match selected_association(node) {
                Some(selected) => self.lvalue(selected, state),
                None => {
                    self.reason("null-unsupported-form: GenericSelectionExpr".into());
                    None
                }
            },
            _ => {
                self.eval(node, state);
                None
            }
        }
    }

    fn place_key(&self, place: &Place) -> String {
        match place {
            Place::Var(id) => self.var_root(id),
            Place::Path(key) => key.clone(),
        }
    }

    /// `E1[E2]`: the pointer operand is dereferenced, unless it is an array
    /// object's decay, whose storage is then reached as an lvalue.
    fn element(&mut self, node: &Value, state: &mut State) {
        let parts = kids(node).collect::<Vec<_>>();
        let (Some(first), Some(second)) = (parts.first(), parts.get(1)) else {
            self.reason("null-unsupported-form: ArraySubscriptExpr".into());
            return;
        };
        let (base, index) = if is_pointer(ty(strip(first))) || !is_pointer(ty(strip(second))) {
            (*first, *second)
        } else {
            (*second, *first)
        };
        let stripped = strip(base);
        if kind(stripped) == "ImplicitCastExpr" && stripped["castKind"] == "ArrayToPointerDecay" {
            if let Some(array) = kids(stripped).next() {
                self.lvalue(array, state);
            }
            self.eval(index, state);
            return;
        }
        let value = self.eval(base, state);
        self.eval(index, state);
        self.check(node, "element-access", base, &value, state);
    }

    /// `&operand`.
    fn address_of(&mut self, operand: &Value, state: &mut State) -> Val {
        let operand = strip(operand);
        match kind(operand) {
            "MemberExpr" => {
                let Some(base) = kids(operand).next() else {
                    return Val::address();
                };
                if operand["isArrow"] == true {
                    // Forming `&B->G` is not a dereference of `B` (decision 1).
                    self.eval(base, state).offset()
                } else {
                    self.address_of(base, state).offset()
                }
            }
            "ArraySubscriptExpr" => {
                let parts = kids(operand).collect::<Vec<_>>();
                let (Some(first), Some(second)) = (parts.first(), parts.get(1)) else {
                    return Val::address();
                };
                let (base, index) =
                    if is_pointer(ty(strip(first))) || !is_pointer(ty(strip(second))) {
                        (*first, *second)
                    } else {
                        (*second, *first)
                    };
                let value = self.eval(base, state).offset();
                self.eval(index, state);
                value
            }
            "UnaryOperator" if operand["opcode"] == "*" => match kids(operand).next() {
                Some(inner) => self.eval(inner, state),
                None => Val::address(),
            },
            "DeclRefExpr" | "CompoundLiteralExpr" | "StringLiteral" | "PredefinedExpr" => {
                if kind(operand) == "CompoundLiteralExpr" {
                    for child in kids(operand) {
                        self.eval(child, state);
                    }
                }
                Val::address()
            }
            _ => {
                self.eval(operand, state);
                Val::address()
            }
        }
    }

    /// The value read from an lvalue.
    fn read(&mut self, node: &Value, state: &mut State) -> Val {
        self.read_as(node, false, state)
    }

    /// The value read from an lvalue, a pointer when its type or the
    /// reading conversion's type is one.
    fn read_as(&mut self, node: &Value, pointer: bool, state: &mut State) -> Val {
        let pointer = pointer || is_pointer(ty(node));
        let place = self.lvalue(node, state);
        if !pointer {
            return Val::scalar();
        }
        // Card NULL-CANDIDATE-TRIAGE (rule 14t): a declared non-null member
        // read with no fact of its own on the path.
        let member = self.declared_member(node);
        let unfacted = match &place {
            Some(Place::Path(key)) => !state.paths.contains_key(key),
            Some(Place::Var(_)) => false,
            None => true,
        };
        if let Some(member) = member
            && unfacted
        {
            self.consulted.insert(format!("member:{member}"));
            return Val {
                place,
                ..Val::of(Nullness::proved(Proof::NonNullResult(format!(
                    "member:{member}"
                ))))
            };
        }
        match place {
            Some(place) => self.read_place(&place, &render(node), state),
            None => Val::of(Nullness::maybe(Source::MemoryField(render(node)))),
        }
    }

    /// Card NULL-CANDIDATE-TRIAGE: the declared member (`TAG.FIELD`, in
    /// force) a member access names, if any.
    fn declared_member(&self, node: &Value) -> Option<String> {
        let node = strip(node);
        if kind(node) != "MemberExpr" {
            return None;
        }
        let field = node["name"].as_str()?;
        let base = kids(node).next()?;
        let tag = record_tag(self.env.unit, ty(base), node["isArrow"] == true)?;
        let key = (tag, field.to_owned());
        self.env
            .members
            .get(&key)
            .filter(|m| m.held.is_none())
            .map(|_| format!("{}.{}", key.0, key.1))
    }

    /// Card NULL-CANDIDATE-TRIAGE: record a store of `value` to the declared
    /// member `member` at `node`.
    fn member_store(&mut self, node: &Value, member: String, value: &Nullness, state: &State) {
        if !self.recording || !state.live {
            return;
        }
        let id = self.site(node);
        let outcome = core::check(value, false);
        let sources = match &outcome {
            Outcome::Candidate(sources) => sources.iter().map(Source::render).collect(),
            _ => Vec::new(),
        };
        let relies = self.stable_relies(value);
        let record = MemberStore {
            member,
            function: self.function.clone(),
            line: self.line(node),
            outcome: outcome.class(),
            sources,
            relies,
        };
        // A store reached twice keeps its worse record: a discharged one
        // never replaces a recorded failure.
        match self.member_stores.get_mut(&id) {
            Some(_) if outcome.discharged() => {}
            Some(existing) => *existing = record,
            None => {
                self.member_stores.insert(id, record);
            }
        }
    }

    /// Card NULL-MEMBER-INVARIANT-SCOPE: the withdrawable declarations a
    /// value's proof rests on (a stable member is also a dependency of the
    /// row; the others are recorded where they were read).
    fn stable_relies(&mut self, null: &Nullness) -> Vec<String> {
        let Nullness::NonNull(proofs) = null else {
            return Vec::new();
        };
        // A store's proof resting on a declaration the fold may withdraw
        // (a stable member, a declared member, a declared result).
        let relies = proofs
            .iter()
            .filter_map(|proof| match proof {
                Proof::NonNullResult(name)
                    if name.starts_with("stable:") || name.starts_with("member:") =>
                {
                    Some(name.clone())
                }
                Proof::NonNullResult(name) => name
                    .strip_prefix("declared-result:")
                    .map(|f| format!("result:{f}")),
                _ => None,
            })
            .collect::<Vec<_>>();
        if self.recording {
            self.consulted.extend(
                relies
                    .iter()
                    .filter(|name| name.starts_with("stable:"))
                    .cloned(),
            );
        }
        relies
    }

    /// Card NULL-MEMBER-INVARIANT-SCOPE: the stable member (`TAG.FIELD`,
    /// in force) a member access names, if any.
    fn stable_member(&self, node: &Value) -> Option<String> {
        let node = strip(node);
        if kind(node) != "MemberExpr" || self.env.stable.is_empty() {
            return None;
        }
        let field = node["name"].as_str()?;
        let base = kids(node).next()?;
        let tag = record_tag(self.env.unit, ty(base), node["isArrow"] == true)?;
        let key = (tag, field.to_owned());
        self.env
            .stable
            .get(&key)
            .filter(|m| m.held.is_none())
            .map(|_| format!("{}.{}", key.0, key.1))
    }

    /// Card NULL-MEMBER-INVARIANT-SCOPE: the stable members (in force) of a
    /// `struct TAG` type spelling (not a pointer).
    fn stable_fields_of(&self, spelled: &str) -> Vec<String> {
        if self.env.stable.is_empty() {
            return Vec::new();
        }
        let Some(tag) = record_tag(self.env.unit, spelled, false) else {
            return Vec::new();
        };
        self.env
            .stable
            .iter()
            .filter(|((t, _), m)| *t == tag && m.held.is_none())
            .map(|((t, f), _)| format!("{t}.{f}"))
            .collect()
    }

    /// Card NULL-MEMBER-INVARIANT-SCOPE (rule 14x): record a store of
    /// `value` to the stable member `member` at `node`, of kind `kind`.
    fn stable_store(
        &mut self,
        node: &Value,
        member: String,
        store_kind: &'static str,
        value: &Nullness,
        state: &State,
    ) {
        if !self.recording || !state.live {
            return;
        }
        let id = self.site(node);
        let outcome = if store_kind == "whole-object" {
            Outcome::Candidate(BTreeSet::from([Source::Other(
                "a whole-object write".into(),
            )]))
        } else {
            core::check(value, false)
        };
        let sources = match &outcome {
            Outcome::Candidate(sources) => sources.iter().map(Source::render).collect(),
            _ => Vec::new(),
        };
        let relies = self.stable_relies(value);
        let record = StableStore {
            member: member.clone(),
            function: self.function.clone(),
            line: self.line(node),
            kind: store_kind,
            outcome: outcome.class(),
            sources,
            relies,
        };
        match self.stable_stores.get_mut(&(id, member.clone())) {
            Some(_) if outcome.discharged() => {}
            Some(existing) => *existing = record,
            None => {
                self.stable_stores.insert((id, member), record);
            }
        }
    }

    /// Store `value` to the place `lhs` designates.
    fn store(&mut self, lhs: &Value, place: Option<Place>, value: &Val, state: &mut State) {
        if let Some(member) = self.declared_member(lhs) {
            let at = strip(lhs);
            self.member_store(at, member, &value.null, state);
        }
        // Card NULL-MEMBER-INVARIANT-SCOPE (rule 14x): a store to a stable
        // member of an existing object; a struct assignment of a record
        // with one is a whole-object store.
        if let Some(member) = self.stable_member(lhs) {
            let at = strip(lhs);
            self.stable_store(at, member, "assignment", &value.null, state);
        } else if !is_pointer(ty(lhs)) {
            for member in self.stable_fields_of(ty(lhs)) {
                let at = strip(lhs);
                self.stable_store(at, member, "whole-object", &value.null, state);
            }
        }
        match place {
            Some(Place::Var(id)) => {
                Self::forget_rooted(state, &id);
                if is_pointer(ty(lhs)) {
                    let mut stored = value.clone();
                    stored.place = Some(Place::Var(id.clone()));
                    stored.null_constant = false;
                    if stored.ident.is_none() {
                        stored.ident = Some(self.site(lhs));
                    }
                    state.vars.insert(id, stored);
                }
            }
            Some(Place::Path(key)) => {
                self.memory_written(state, ast::is_wide_scalar(ty(lhs)));
                if is_pointer(ty(lhs)) {
                    state.paths.insert(
                        key,
                        PathFact {
                            null: value.null.clone(),
                            effective_type: false,
                        },
                    );
                }
            }
            None => self.memory_written(state, ast::is_wide_scalar(ty(lhs))),
        }
    }

    pub(super) fn eval(&mut self, node: &Value, state: &mut State) -> Val {
        match kind(node) {
            "ParenExpr" | "ConstantExpr" | "ExprWithCleanups" | "OpaqueValueExpr" => {
                match kids(node).next() {
                    Some(inner) => self.eval(inner, state),
                    None => Val::scalar(),
                }
            }
            "ImplicitCastExpr" | "CStyleCastExpr" | "BuiltinBitCastExpr" => self.cast(node, state),
            "DeclRefExpr" => match node["referencedDecl"]["kind"].as_str() {
                Some("FunctionDecl") => Val::address(),
                Some("VarDecl" | "ParmVarDecl") => self.read(node, state),
                _ => Val::scalar(),
            },
            "IntegerLiteral" => {
                let mut value = Val::scalar();
                value.null_constant = node["value"].as_str() == Some("0");
                value
            }
            "CharacterLiteral" | "FloatingLiteral" | "ImaginaryLiteral" | "FixedPointLiteral" => {
                Val::scalar()
            }
            "StringLiteral" | "PredefinedExpr" | "AddrLabelExpr" => Val::address(),
            "UnaryOperator" => self.unary(node, state),
            "BinaryOperator" => self.binary(node, state),
            "CompoundAssignOperator" => {
                let mut parts = kids(node);
                let (Some(lhs), Some(rhs)) = (parts.next(), parts.next()) else {
                    return Val::scalar();
                };
                let current = self.read_for_update(lhs, state);
                self.eval(rhs, state);
                let place = current.1;
                let updated = if is_pointer(ty(lhs)) {
                    current.0.offset()
                } else {
                    Val::scalar()
                };
                self.store(lhs, place, &updated, state);
                updated
            }
            "ConditionalOperator" => {
                let parts = kids(node).collect::<Vec<_>>();
                let [condition, yes, no] = parts.as_slice() else {
                    self.reason("null-unsupported-form: ConditionalOperator".into());
                    return Val::scalar();
                };
                let (mut on_true, mut on_false) = self.condition(
                    condition,
                    state.clone(),
                    Proof::Tested(self.line(condition)),
                );
                let a = self.eval(yes, &mut on_true);
                let b = self.eval(no, &mut on_false);
                *state = on_true.join(&on_false);
                match (on_true.live, on_false.live) {
                    (true, false) => a,
                    (false, true) => b,
                    _ => a.join(&b),
                }
            }
            "BinaryConditionalOperator" => {
                let parts = kids(node).collect::<Vec<_>>();
                let (Some(common), Some(no)) = (parts.first(), parts.last()) else {
                    return Val::scalar();
                };
                let value = self.eval(common, state);
                let (on_true, mut on_false) =
                    self.refine(&value, state.clone(), Proof::Tested(self.line(common)));
                let kept = match core::refine(&value.null, true, Proof::Tested(self.line(common))) {
                    Some(null) => Val {
                        null,
                        ..value.clone()
                    },
                    None => value.clone(),
                };
                let b = self.eval(no, &mut on_false);
                *state = on_true.join(&on_false);
                match (on_true.live, on_false.live) {
                    (true, false) => kept,
                    (false, true) => b,
                    _ => kept.join(&b),
                }
            }
            "CallExpr" => self.call(node, state),
            "MemberExpr" | "ArraySubscriptExpr" => {
                // An lvalue in a value position (a discarded `p->f;`): its
                // evaluation is what checks.
                self.read(node, state)
            }
            "UnaryExprOrTypeTraitExpr" | "OffsetOfExpr" | "TypeTraitExpr" => Val::scalar(),
            // C11 6.5.1.1: only the selected association is evaluated.
            "GenericSelectionExpr" => match selected_association(node) {
                Some(selected) => self.eval(selected, state),
                None => {
                    self.reason("null-unsupported-form: GenericSelectionExpr".into());
                    Val::of(Nullness::maybe(Source::Other("generic-selection".into())))
                }
            },
            "MaterializeTemporaryExpr" => match kids(node).next() {
                Some(inner) => self.eval(inner, state),
                None => Val::scalar(),
            },
            // A type is never evaluated (a `typeof` operand, a trait's types).
            kind_name if kind_name.ends_with("Type") => Val::scalar(),
            "InitListExpr" if self.declared_record(node).is_some() => {
                self.record_initializer(node, state);
                Val::scalar()
            }
            "InitListExpr"
            | "ParenListExpr"
            | "DesignatedInitExpr"
            | "ShuffleVectorExpr"
            | "ConvertVectorExpr"
            | "ExtVectorElementExpr" => {
                for child in kids(node) {
                    self.eval(child, state);
                }
                Val::scalar()
            }
            "ImplicitValueInitExpr" | "NoInitExpr" | "ArrayInitIndexExpr" => Val::scalar(),
            "CompoundLiteralExpr" => {
                for child in kids(node) {
                    self.eval(child, state);
                }
                Val::scalar()
            }
            "StmtExpr" => {
                let Some(body) = kids(node).next() else {
                    return Val::scalar();
                };
                let parts = kids(body).collect::<Vec<_>>();
                let Some((last, rest)) = parts.split_last() else {
                    return Val::scalar();
                };
                let mut current = state.clone();
                for part in rest {
                    current = self.stmt(part, current);
                }
                let value = if is_expression(last) {
                    self.eval(last, &mut current)
                } else {
                    current = self.stmt(last, current);
                    Val::scalar()
                };
                *state = current;
                value
            }
            "VAArgExpr" => {
                for child in kids(node) {
                    self.eval(child, state);
                }
                Val::of(Nullness::maybe(Source::Other("va_arg".into())))
            }
            "AtomicExpr" => {
                let parts = kids(node).collect::<Vec<_>>();
                if let Some(pointer) = parts.first() {
                    let value = self.eval(pointer, state);
                    self.check(node, "atomic-access", pointer, &value, state);
                }
                for part in parts.iter().skip(1) {
                    self.eval(part, state);
                }
                self.memory_written(state, false);
                if is_pointer(ty(node)) {
                    Val::of(Nullness::maybe(Source::MemoryField(render(node))))
                } else {
                    Val::scalar()
                }
            }
            other => {
                self.reason(format!("null-unsupported-form: {other}"));
                for child in kids(node) {
                    self.eval(child, state);
                }
                Val::of(Nullness::maybe(Source::Other(other.to_owned())))
            }
        }
    }

    /// Card NULL-CANDIDATE-TRIAGE: the declared members of the struct an
    /// initializer list builds, if it builds one with any.
    fn declared_record(&self, node: &Value) -> Option<String> {
        let tag = record_tag(self.env.unit, ty(node), false)?;
        (self
            .env
            .members
            .iter()
            .any(|((t, _), m)| *t == tag && m.held.is_none())
            || self
                .env
                .stable
                .iter()
                .any(|((t, _), m)| *t == tag && m.held.is_none()))
        .then_some(tag)
    }

    /// Card NULL-CANDIDATE-TRIAGE: an initializer list of a struct with a
    /// declared member stores each initializer (an omitted one is zero, a
    /// null store); a list the walk cannot attribute to the members is a
    /// store not proved.
    fn record_initializer(&mut self, node: &Value, state: &mut State) {
        let Some(tag) = record_tag(self.env.unit, ty(node), false) else {
            return;
        };
        let values = kids(node)
            .map(|child| (child, self.eval(child, state)))
            .collect::<Vec<_>>();
        let fields = self.env.unit.records.get(&tag).cloned();
        let declared = self
            .env
            .members
            .iter()
            .filter(|((t, _), m)| *t == tag && m.held.is_none())
            .map(|((_, f), _)| f.clone())
            .collect::<Vec<_>>();
        for field in declared {
            let member = format!("{tag}.{field}");
            let stored = fields
                .as_ref()
                .filter(|fields| fields.len() == values.len())
                .and_then(|fields| fields.iter().position(|f| *f == field))
                .and_then(|index| values.get(index));
            let null = match stored {
                Some((child, _)) if matches!(kind(child), "ImplicitValueInitExpr") => {
                    Nullness::Null
                }
                Some((_, value)) => value.null.clone(),
                None => Nullness::maybe(Source::Other("an initializer list".into())),
            };
            self.member_store(node, member, &null, state);
        }
        // Card NULL-MEMBER-INVARIANT-SCOPE (rule 14x): an initializer list
        // writes a fresh object: each stable member's store is recorded and
        // exempt (it cannot make an existing object's member NULL).
        let stable = self
            .env
            .stable
            .iter()
            .filter(|((t, _), m)| *t == tag && m.held.is_none())
            .map(|((_, f), _)| f.clone())
            .collect::<Vec<_>>();
        for field in stable {
            let member = format!("{tag}.{field}");
            let stored = fields
                .as_ref()
                .filter(|fields| fields.len() == values.len())
                .and_then(|fields| fields.iter().position(|f| *f == field))
                .and_then(|index| values.get(index));
            let null = match stored {
                Some((child, _)) if matches!(kind(child), "ImplicitValueInitExpr") => {
                    Nullness::Null
                }
                Some((_, value)) => value.null.clone(),
                None => Nullness::maybe(Source::Other("an initializer list".into())),
            };
            self.stable_store(node, member, "initializer", &null, state);
        }
    }

    /// Read an lvalue that is about to be written (`++`, `+=`): its place and
    /// current value.
    fn read_for_update(&mut self, lhs: &Value, state: &mut State) -> (Val, Option<Place>) {
        let place = self.lvalue(lhs, state);
        // Card NULL-CANDIDATE-TRIAGE: a declared member's current value.
        if let Some(member) = self.declared_member(lhs)
            && let Some(Place::Path(key)) = &place
            && !state.paths.contains_key(key)
        {
            self.consulted.insert(format!("member:{member}"));
            let value = Val {
                place: place.clone(),
                ..Val::of(Nullness::proved(Proof::NonNullResult(format!(
                    "member:{member}"
                ))))
            };
            return (value, place);
        }
        let value = match (&place, is_pointer(ty(lhs))) {
            (Some(place), true) => self.read_place(place, &render(lhs), state),
            _ => Val::scalar(),
        };
        (value, place)
    }

    fn cast(&mut self, node: &Value, state: &mut State) -> Val {
        let Some(operand) = kids(node).next() else {
            return Val::scalar();
        };
        match node["castKind"].as_str() {
            Some("LValueToRValue") => self.read_as(operand, is_pointer(ty(node)), state),
            Some("ArrayToPointerDecay") => {
                // Forming the address of an array's first element reads no
                // pointer; an array member of `B->` is `&B->G` (decision 1).
                let array = strip(operand);
                if kind(array) == "MemberExpr"
                    && array["isArrow"] == true
                    && let Some(base) = kids(array).next()
                {
                    return self.eval(base, state).offset();
                }
                if kind(array) == "MemberExpr" {
                    return self.address_of(array, state);
                }
                if kind(array) == "UnaryOperator" && array["opcode"] == "*" {
                    return self.address_of(array, state);
                }
                if kind(array) == "ArraySubscriptExpr" {
                    return self.address_of(array, state);
                }
                if matches!(kind(array), "CompoundLiteralExpr" | "StmtExpr" | "CallExpr") {
                    self.eval(array, state);
                }
                Val::address()
            }
            Some("FunctionToPointerDecay" | "BuiltinFnToFnPtr") => {
                self.eval(operand, state);
                Val::address()
            }
            Some("NullToPointer") => {
                self.eval(operand, state);
                Val {
                    null_constant: true,
                    ..Val::of(Nullness::Null)
                }
            }
            Some("IntegralToPointer") => {
                let value = self.eval(operand, state);
                if value.null_constant {
                    Val {
                        null_constant: true,
                        ..Val::of(Nullness::Null)
                    }
                } else {
                    Val::of(Nullness::maybe(Source::Other("integer-to-pointer".into())))
                }
            }
            Some("BitCast" | "NoOp" | "AddressSpaceConversion") => {
                let value = self.eval(operand, state);
                if is_pointer(ty(node)) {
                    value
                } else {
                    Val::scalar()
                }
            }
            _ => {
                let value = self.eval(operand, state);
                if is_pointer(ty(node)) {
                    value
                } else {
                    Val {
                        null_constant: value.null_constant,
                        ..Val::scalar()
                    }
                }
            }
        }
    }

    fn unary(&mut self, node: &Value, state: &mut State) -> Val {
        let Some(operand) = kids(node).next() else {
            return Val::scalar();
        };
        match node["opcode"].as_str().unwrap_or("") {
            "*" => self.read(node, state),
            "&" => self.address_of(operand, state),
            "++" | "--" => {
                let (current, place) = self.read_for_update(operand, state);
                let updated = if is_pointer(ty(operand)) {
                    current.offset()
                } else {
                    Val::scalar()
                };
                self.store(operand, place, &updated, state);
                if node["isPostfix"] == true {
                    Val {
                        place: None,
                        ..current
                    }
                } else {
                    updated
                }
            }
            "__extension__" => self.eval(operand, state),
            _ => {
                self.eval(operand, state);
                Val::scalar()
            }
        }
    }

    fn binary(&mut self, node: &Value, state: &mut State) -> Val {
        let mut parts = kids(node);
        let (Some(lhs), Some(rhs)) = (parts.next(), parts.next()) else {
            return Val::scalar();
        };
        match node["opcode"].as_str().unwrap_or("") {
            "=" => {
                let place = self.lvalue(lhs, state);
                let value = self.eval(rhs, state);
                self.store(lhs, place.clone(), &value, state);
                // The assignment's value is its left operand's after the store
                // (C11 6.5.16p3): a test of `(p = f()) != NULL` tests `p`.
                let readable = match &place {
                    Some(Place::Var(id)) => state.vars.contains_key(id),
                    Some(Place::Path(key)) => state.paths.contains_key(key),
                    None => false,
                };
                Val {
                    place: if readable && is_pointer(ty(lhs)) {
                        place
                    } else {
                        None
                    },
                    null_constant: false,
                    ..value
                }
            }
            "," => {
                self.eval(lhs, state);
                self.eval(rhs, state)
            }
            "&&" | "||" => {
                let (on_true, on_false) =
                    self.condition(node, state.clone(), Proof::Tested(self.line(node)));
                *state = on_true.join(&on_false);
                Val::scalar()
            }
            "+" | "-" => {
                let left = self.eval(lhs, state);
                let right = self.eval(rhs, state);
                if is_pointer(ty(node)) {
                    if is_pointer(ty(lhs)) {
                        left.offset()
                    } else {
                        right.offset()
                    }
                } else {
                    Val::scalar()
                }
            }
            _ => {
                self.eval(lhs, state);
                self.eval(rhs, state);
                Val::scalar()
            }
        }
    }

    /// Card NULL-MEMBER-INVARIANT-SCOPE (rule 14z): the caller's check of
    /// one scope entry: the actual's `FIELD`, read from the caller's facts
    /// (a tracked actual's path, a memory path's own), must be non-null.
    fn scope_obligation(
        &mut self,
        callee: &str,
        held: &Option<String>,
        entry: &super::scope::ScopeEntry,
        arguments: &[&Value],
        values: &[Val],
        state: &mut State,
    ) {
        let (Some(argument), Some(value)) =
            (arguments.get(entry.position), values.get(entry.position))
        else {
            return;
        };
        let text = format!("{}->{}", render(argument), entry.field);
        let place = match &value.place {
            Some(Place::Var(id)) => Some(Place::Path(format!(
                "{}^.{}",
                self.var_root(id),
                entry.field
            ))),
            Some(Place::Path(key)) => Some(Place::Path(format!("{key}^.{}", entry.field))),
            None => None,
        };
        let member = match &place {
            Some(place) => self.read_place(place, &text, state),
            None => Val::of(Nullness::maybe(Source::MemoryField(text.clone()))),
        };
        let at = serde_json::json!({
            "id": format!("{}#scope:{}", node_key(argument), entry.field),
            "o": argument["o"],
        });
        let mut probe = state.clone();
        let check_id = self.check(&at, "call-precondition", argument, &member, &mut probe);
        if let Some(hold) = held {
            if self.recording
                && let Some(record) = self.checks.get_mut(&check_id)
                && !record.outcome.discharged()
                && record.outcome != Outcome::Excluded
            {
                record.outcome = Outcome::Held(hold.clone());
            }
        } else {
            *state = probe;
        }
        if self.recording
            && let Some(record) = self.checks.get_mut(&check_id)
        {
            record.subject = format!("{text} at `{callee}` {}", entry.text);
        }
    }

    fn call(&mut self, node: &Value, state: &mut State) -> Val {
        let parts = kids(node).collect::<Vec<_>>();
        let Some((callee, arguments)) = parts.split_first() else {
            return Val::scalar();
        };
        let name = callee_name(node);
        match name.as_deref() {
            Some("__builtin_expect" | "__builtin_expect_with_probability") => {
                let value = arguments
                    .first()
                    .map(|a| self.eval(a, state))
                    .unwrap_or_else(Val::scalar);
                for rest in arguments.iter().skip(1) {
                    self.eval(rest, state);
                }
                return value;
            }
            Some(
                "__builtin_constant_p"
                | "__builtin_object_size"
                | "__builtin_dynamic_object_size"
                | "__builtin_classify_type",
            ) => return Val::scalar(),
            _ => {}
        }
        if name.is_none() {
            // An indirect call: its callee expression is evaluated (a
            // `s->cb` dereferences `s`); the function pointer itself is not
            // checked (a registered residual, counted on the row).
            self.eval(callee, state);
            let site = self.site(node);
            self.indirect_calls.insert(site);
        }
        let values = arguments
            .iter()
            .map(|argument| self.eval(argument, state))
            .collect::<Vec<_>>();
        // The callee's clause: a declaration in the bounds environment (in
        // force or held), else this unit's inference.
        let mut obligations = Vec::new();
        if let Some(callee_name) = &name {
            if let Some(declared) = self.env.declared.get(callee_name) {
                self.consulted.insert(callee_name.clone());
                if declared.parameters == values.len() {
                    for (position, text) in &declared.positions {
                        obligations.push((*position, text.clone(), declared.held.clone()));
                    }
                } else {
                    self.reason(format!(
                        "null-declaration-withdrawn: `{callee_name}`'s declaration does not match the call's arguments"
                    ));
                }
            } else if let Some(summary) = self.env.summaries.get(callee_name)
                && callee_name != &self.function
            {
                if !summary.derefs.is_empty() {
                    self.consulted.extend(summary.dependencies.iter().cloned());
                }
                for position in &summary.derefs {
                    if *position < values.len() {
                        let formal = self
                            .env
                            .unit
                            .definitions
                            .get(callee_name)
                            .and_then(|definition| {
                                kids(definition)
                                    .filter(|c| kind(c) == "ParmVarDecl")
                                    .nth(*position)
                            })
                            .and_then(|p| p["name"].as_str())
                            .unwrap_or("?")
                            .to_owned();
                        obligations.push((
                            *position,
                            format!("inferred `{callee_name}` quant null-deref #{formal}"),
                            None,
                        ));
                    }
                }
                if !summary.derefs.is_empty() {
                    self.consulted.insert(format!("inferred:{callee_name}"));
                }
            }
        }
        for (position, text, held) in obligations {
            let (Some(argument), Some(value)) = (arguments.get(position), values.get(position))
            else {
                continue;
            };
            let mut probe = state.clone();
            let check_id = self.check(argument, "call-precondition", argument, value, &mut probe);
            if let Some(hold) = &held {
                // A held clause is not in force: it accuses nothing and moves
                // nothing (Pending at most).
                if self.recording
                    && let Some(record) = self.checks.get_mut(&check_id)
                    && !record.outcome.discharged()
                    && record.outcome != Outcome::Excluded
                {
                    record.outcome = Outcome::Held(hold.clone());
                }
            } else {
                *state = probe;
            }
            if self.recording
                && let Some(record) = self.checks.get_mut(&check_id)
            {
                record.subject = format!("{} at {text}", record.subject);
            }
        }
        // Card NULL-MEMBER-INVARIANT-SCOPE (rule 14z): a declared scope
        // entry is a caller obligation on the actual's member.
        if let Some(callee_name) = &name
            && let Some(scope) = self.env.scopes.get(callee_name)
        {
            self.consulted.insert(callee_name.clone());
            if scope.parameters == values.len() {
                for entry in scope.entries.clone() {
                    self.scope_obligation(
                        callee_name,
                        &scope.held.clone(),
                        &entry,
                        arguments,
                        &values,
                        state,
                    );
                }
            } else {
                self.reason(format!(
                    "null-declaration-withdrawn: `{callee_name}`'s scope declaration does not match the call's arguments"
                ));
            }
        }
        // Card NULL-MEMBER-INVARIANT-SCOPE (rule 14x): a fill or copy of a
        // `struct TAG` object with a stable member is a whole-object store.
        if let Some(callee_name) = &name
            && scope::is_object_fill(callee_name)
            && let Some(destination) = arguments.first()
        {
            let mut target = strip(destination);
            while matches!(kind(target), "ImplicitCastExpr" | "CStyleCastExpr")
                && let Some(inner) = kids(target).next()
            {
                target = strip(inner);
            }
            let spelled = ty(target).trim().to_owned();
            if let Some(pointee) = spelled.strip_suffix('*') {
                for member in self.stable_fields_of(pointee.trim()) {
                    self.stable_store(
                        node,
                        member,
                        "whole-object",
                        &Nullness::maybe(Source::Other("a whole-object write".into())),
                        state,
                    );
                }
            }
        }
        // Card NULL-CANDIDATE-TRIAGE (rule 14v): the pointer actuals of a
        // call to a function this unit neither defines nor declares, judged
        // as a check would be, for cross-unit travel.
        if self.recording
            && state.live
            && let Some(callee_name) = &name
            && !self.env.unit.definitions.contains_key(callee_name)
            && !self.env.declared.contains_key(callee_name)
        {
            for (position, (argument, value)) in arguments.iter().zip(&values).enumerate() {
                if !is_pointer(ty(argument)) {
                    continue;
                }
                let outcome = core::check(&value.null, false);
                let (sources, proofs) = match &outcome {
                    Outcome::Candidate(sources) => {
                        (sources.iter().map(Source::render).collect(), Vec::new())
                    }
                    Outcome::Proved(proofs) | Outcome::Moved(proofs) => {
                        (Vec::new(), proofs.iter().map(super::render_proof).collect())
                    }
                    _ => (Vec::new(), Vec::new()),
                };
                let site = self.site(node);
                let record = ExternalCall {
                    function: self.function.clone(),
                    callee: callee_name.clone(),
                    position,
                    arguments: arguments.len(),
                    line: self.line(argument),
                    subject: render(argument),
                    outcome: outcome.class(),
                    sources,
                    proofs,
                };
                self.external_calls
                    .entry((site, position))
                    .and_modify(|existing| {
                        if existing.outcome != record.outcome && record.outcome != "proved" {
                            *existing = record.clone();
                        }
                    })
                    .or_insert(record);
            }
        }
        // A call may write any memory a path names.
        self.memory_written(state, false);
        state.definite = false;
        let returns_pointer = is_pointer(ty(node));
        if let Some(callee_name) = &name
            && (self.env.unit.no_return.contains(callee_name)
                || ast::builtin_no_return(callee_name))
        {
            state.live = false;
        }
        if !returns_pointer {
            return Val::scalar();
        }
        let Some(callee_name) = name else {
            return Val::of(Nullness::maybe(Source::UncontractedResult(
                "indirect call".into(),
            )));
        };
        // `__builtin_alloca` returns the address of the space it allocated:
        // the builtin has no failure result (exhausting the stack is not a
        // null return).
        if matches!(
            callee_name.as_str(),
            "__builtin_alloca" | "__builtin_alloca_with_align"
        ) {
            return Val::of(Nullness::proved(Proof::NonNullResult(format!(
                "builtin:{callee_name}"
            ))));
        }
        // `errno` expands to a modifiable lvalue (C11 7.5p2): the function
        // the C library spells it through returns the address of an object.
        if ast::is_errno_location(&callee_name) {
            return Val::of(Nullness::proved(Proof::NonNullResult(format!(
                "c11-7.5p2-errno:{callee_name}"
            ))));
        }
        // Card NULL-CANDIDATE-TRIAGE (rule 14u): a declared non-null result.
        if self
            .env
            .results
            .get(&callee_name)
            .is_some_and(|r| r.held.is_none())
        {
            self.consulted.insert(format!("result:{callee_name}"));
            return Val::of(Nullness::proved(Proof::NonNullResult(format!(
                "declared-result:{callee_name}"
            ))));
        }
        if let Some(summary) = self.env.summaries.get(&callee_name)
            && summary.returns_non_null
            && callee_name != self.function
        {
            self.consulted.extend(summary.dependencies.iter().cloned());
            return Val::of(Nullness::proved(Proof::NonNullResult(format!(
                "summary:{callee_name}"
            ))));
        }
        let contract = crate::contract_dependencies::unrecorded(|| {
            self.env
                .resource
                .contract(&callee_name)
                .filter(|_| !self.env.checked_results.contains(&callee_name))
                .cloned()
        });
        let null = match contract.map(|c| c.result) {
            Some(ResourceReturnMode::Owned { nullable: true })
            | Some(ResourceReturnMode::Reallocated { nullable: true, .. }) => {
                Nullness::maybe(Source::Allocation(callee_name.clone()))
            }
            Some(ResourceReturnMode::Owned { nullable: false })
            | Some(ResourceReturnMode::Reallocated {
                nullable: false, ..
            }) => Nullness::proved(Proof::NonNullResult(format!("declared:{callee_name}"))),
            Some(ResourceReturnMode::Derived { nullable: true, .. }) => {
                Nullness::maybe(Source::Lookup(callee_name.clone()))
            }
            Some(ResourceReturnMode::Derived {
                nullable: false,
                parameter,
                ..
            }) => match values.get(parameter) {
                Some(base) => core::offset(&base.null),
                None => Nullness::maybe(Source::NullableResult(callee_name.clone())),
            },
            Some(ResourceReturnMode::None | ResourceReturnMode::Removed { .. }) | None => {
                Nullness::maybe(Source::UncontractedResult(callee_name.clone()))
            }
        };
        Val::of(null)
    }

    /// A condition on both arms.
    fn condition(&mut self, node: &Value, state: State, proof: Proof) -> (State, State) {
        let node = strip(node);
        // A pointer in a boolean context (`!p`'s operand, a `p` Clang left
        // without a `PointerToBoolean` cast) is a NULL test.
        if is_pointer(ty(node)) {
            let mut state = state;
            let value = self.eval(node, &mut state);
            return self.refine(&value, state, proof);
        }
        match kind(node) {
            "UnaryOperator" if node["opcode"] == "!" => {
                let Some(operand) = kids(node).next() else {
                    return (state.clone(), state);
                };
                let (on_true, on_false) = self.condition(operand, state, proof);
                (on_false, on_true)
            }
            "BinaryOperator" => {
                let mut parts = kids(node);
                let (Some(lhs), Some(rhs)) = (parts.next(), parts.next()) else {
                    return (state.clone(), state);
                };
                match node["opcode"].as_str().unwrap_or("") {
                    "&&" => {
                        let (t1, f1) = self.condition(lhs, state, proof.clone());
                        let (t2, f2) = self.condition(rhs, t1, proof);
                        (t2, f1.join(&f2))
                    }
                    "||" => {
                        let (t1, f1) = self.condition(lhs, state, proof.clone());
                        let (t2, f2) = self.condition(rhs, f1, proof);
                        (t1.join(&t2), f2)
                    }
                    "," => {
                        let mut state = state;
                        self.eval(lhs, &mut state);
                        self.condition(rhs, state, proof)
                    }
                    op @ ("==" | "!=") => {
                        let mut state = state;
                        let left = self.eval(lhs, &mut state);
                        let right = self.eval(rhs, &mut state);
                        let tested = if right.null_constant && is_pointer(ty(strip(lhs))) {
                            Some(left)
                        } else if left.null_constant && is_pointer(ty(strip(rhs))) {
                            Some(right)
                        } else {
                            None
                        };
                        let Some(tested) = tested else {
                            return (state.clone().not_definite(), state.not_definite());
                        };
                        let (non_null, null) = self.refine(&tested, state, proof);
                        if op == "!=" {
                            (non_null, null)
                        } else {
                            (null, non_null)
                        }
                    }
                    _ => {
                        let mut state = state;
                        self.eval(node, &mut state);
                        (state.clone().not_definite(), state.not_definite())
                    }
                }
            }
            "ImplicitCastExpr" if node["castKind"] == "PointerToBoolean" => {
                let mut state = state;
                let Some(operand) = kids(node).next() else {
                    return (state.clone(), state);
                };
                let value = self.eval(operand, &mut state);
                self.refine(&value, state, proof)
            }
            "ImplicitCastExpr" | "CStyleCastExpr"
                if matches!(
                    node["castKind"].as_str(),
                    Some("IntegralCast" | "IntegralToBoolean" | "NoOp")
                ) =>
            {
                let Some(operand) = kids(node).next() else {
                    return (state.clone(), state);
                };
                self.condition(operand, state, proof)
            }
            "CallExpr"
                if matches!(
                    callee_name(node).as_deref(),
                    Some("__builtin_expect" | "__builtin_expect_with_probability")
                ) =>
            {
                let parts = kids(node).collect::<Vec<_>>();
                let Some(first) = parts.get(1) else {
                    return (state.clone(), state);
                };
                let (on_true, on_false) = self.condition(first, state, proof);
                let mut on_true = on_true;
                let mut on_false = on_false;
                for rest in parts.iter().skip(2) {
                    self.eval(rest, &mut on_true);
                    self.eval(rest, &mut on_false);
                }
                (on_true, on_false)
            }
            "IntegerLiteral" => {
                let zero = node["value"].as_str() == Some("0");
                let dead = State::dead(state.firsts.len());
                if zero { (dead, state) } else { (state, dead) }
            }
            // `*"text"`, the first character of a non-empty string literal,
            // is a nonzero constant (`rd_assert(!*"...")`, `RD_BUG`).
            _ if string_literal_first_char(node).is_some_and(|nonzero| nonzero) => {
                let dead = State::dead(state.firsts.len());
                (state, dead)
            }
            _ => {
                let mut state = state;
                self.eval(node, &mut state);
                (state.clone().not_definite(), state.not_definite())
            }
        }
    }

    // -----------------------------------------------------------------------
    // Statements
    // -----------------------------------------------------------------------

    pub(super) fn stmt(&mut self, node: &Value, state: State) -> State {
        match kind(node) {
            "CompoundStmt" => {
                let mut state = state;
                for child in kids(node) {
                    state = self.stmt(child, state);
                }
                state
            }
            "DeclStmt" => {
                let mut state = state;
                for decl in kids(node) {
                    if kind(decl) != "VarDecl" {
                        continue;
                    }
                    let id = decl["id"].as_str().unwrap_or("").to_owned();
                    let init = kids(decl).find(|c| !kind(c).ends_with("Attr") && present(c));
                    let static_storage =
                        matches!(decl["storageClass"].as_str(), Some("static" | "extern"));
                    match init {
                        Some(init) if !static_storage => {
                            let value = self.eval(init, &mut state);
                            if self.tracked.contains(&id) {
                                let ident = value.ident.or_else(|| Some(self.site(decl)));
                                state.vars.insert(
                                    id.clone(),
                                    Val {
                                        place: Some(Place::Var(id.clone())),
                                        null_constant: false,
                                        ident,
                                        ..value
                                    },
                                );
                            } else if is_pointer(ty(decl)) {
                                state.paths.insert(
                                    format!("g:{id}"),
                                    PathFact {
                                        null: value.null,
                                        effective_type: false,
                                    },
                                );
                            }
                        }
                        Some(_) => {}
                        None if self.tracked.contains(&id) => {
                            let name = decl["name"].as_str().unwrap_or("?").to_owned();
                            state.vars.insert(
                                id.clone(),
                                Val {
                                    place: Some(Place::Var(id.clone())),
                                    ..Val::of(Nullness::maybe(Source::Uninitialized(name)))
                                },
                            );
                        }
                        None => {}
                    }
                }
                state
            }
            "NullStmt" => state,
            "IfStmt" => self.if_stmt(node, state),
            "WhileStmt" => {
                let parts = kids(node).filter(|c| present(c)).collect::<Vec<_>>();
                let (Some(cond), Some(body)) = (parts.first(), parts.last()) else {
                    self.reason("null-unsupported-form: WhileStmt".into());
                    return state;
                };
                self.loop_stmt(Some(cond), body, None, false, state)
            }
            "DoStmt" => {
                let parts = kids(node).collect::<Vec<_>>();
                let (Some(body), Some(cond)) = (parts.first(), parts.get(1)) else {
                    self.reason("null-unsupported-form: DoStmt".into());
                    return state;
                };
                self.loop_stmt(Some(cond), body, None, true, state)
            }
            "ForStmt" => {
                let parts = kids(node).collect::<Vec<_>>();
                if parts.len() != 5 {
                    self.reason("null-unsupported-form: ForStmt".into());
                    return state;
                }
                let mut state = state;
                if present(parts[0]) {
                    state = self.stmt(parts[0], state);
                }
                let cond = present(parts[2]).then_some(parts[2]);
                let inc = present(parts[3]).then_some(parts[3]);
                self.loop_stmt(cond, parts[4], inc, false, state)
            }
            "SwitchStmt" => self.switch_stmt(node, state),
            "CaseStmt" | "DefaultStmt" => {
                let entry = self.switches.last().cloned();
                let mut state = match entry {
                    Some((entry, depth)) if depth == self.loops.len() => state.join(&entry),
                    _ => {
                        // A label inside a loop nested in its switch, or no
                        // switch: the jump into it is not read.
                        self.reason(
                            "null-unsupported-form: a case label the walk cannot enter".into(),
                        );
                        self.widen_all(state, "case")
                    }
                };
                state.definite = false;
                let sub = kids(node).rfind(|c| present(c));
                let is_case = kind(node) == "CaseStmt";
                match sub {
                    Some(sub) if !is_case || kids(node).filter(|c| present(c)).count() > 1 => {
                        self.stmt(sub, state)
                    }
                    _ => state,
                }
            }
            "BreakStmt" => {
                if let Some(frame) = self.loops.last_mut() {
                    frame.breaks = frame.breaks.join(&state);
                } else {
                    self.reason("null-unsupported-form: a break outside a loop or switch".into());
                }
                State::dead(state.firsts.len())
            }
            "ContinueStmt" => {
                if let Some(frame) = self.loops.iter_mut().rev().find(|f| !f.switch) {
                    frame.continues = frame.continues.join(&state);
                } else {
                    self.reason("null-unsupported-form: a continue outside a loop".into());
                }
                State::dead(state.firsts.len())
            }
            "ReturnStmt" => {
                let mut state = state;
                if let Some(value) = kids(node).next() {
                    let returned = self.eval(value, &mut state);
                    if state.live && is_pointer(ty(value)) {
                        self.stable_relies(&returned.null);
                        self.returns = Some(match &self.returns {
                            Some(previous) => previous.join(&returned.null),
                            None => returned.null,
                        });
                    }
                }
                if state.live {
                    self.exit(&state);
                }
                State::dead(state.firsts.len())
            }
            "LabelStmt" => {
                let label = node["declId"].as_str().unwrap_or("").to_owned();
                let mut state = state;
                if self.backward.contains(&label) {
                    let name = node["name"].as_str().unwrap_or("?").to_owned();
                    state = self.widen_all(state, &name);
                } else if let Some(arriving) = self.gotos.get(&label) {
                    state = state.join(arriving);
                }
                state.definite = false;
                match kids(node).next() {
                    Some(sub) => self.stmt(sub, state),
                    None => state,
                }
            }
            "GotoStmt" => {
                let label = node["targetLabelDeclId"].as_str().unwrap_or("").to_owned();
                if !self.backward.contains(&label) {
                    let joined = match self.gotos.get(&label) {
                        Some(previous) => previous.join(&state),
                        None => state.clone(),
                    };
                    self.gotos.insert(label, joined);
                }
                State::dead(state.firsts.len())
            }
            "IndirectGotoStmt" => {
                self.reason("null-unsupported-form: IndirectGotoStmt".into());
                State::dead(state.firsts.len())
            }
            "AttributedStmt" => {
                let mut state = state;
                for child in kids(node).filter(|c| !kind(c).ends_with("Attr")) {
                    state = self.stmt(child, state);
                }
                state
            }
            "GCCAsmStmt" | "MSAsmStmt" => {
                let mut state = state;
                for child in kids(node) {
                    self.eval(child, &mut state);
                }
                let label = "asm".to_owned();
                self.widen_all(state, &label)
            }
            _ => {
                let mut state = state;
                self.eval(node, &mut state);
                state
            }
        }
    }

    /// Every fact forgotten (a label a later `goto` reaches, an asm).
    fn widen_all(&self, mut state: State, why: &str) -> State {
        for value in state.vars.values_mut() {
            *value = Val {
                place: value.place.clone(),
                ..Val::of(Nullness::maybe(Source::Other(format!("label:{why}"))))
            };
        }
        state.paths.clear();
        for firsts in &mut state.firsts {
            firsts.insert(First::Other);
        }
        state.definite = false;
        state
    }

    fn if_stmt(&mut self, node: &Value, state: State) -> State {
        let parts = kids(node).filter(|c| present(c)).collect::<Vec<_>>();
        let (Some(cond), Some(then)) = (parts.first(), parts.get(1)) else {
            self.reason("null-unsupported-form: IfStmt".into());
            return state;
        };
        let otherwise = parts.get(2);
        // `assert(B)` expands to `if (B) ; else __assert_fail(...);`: its
        // surviving arm is an assertion (a listed premise), not a test.
        let asserted = kind(then) == "NullStmt"
            && otherwise.is_some_and(|e| {
                kind(e) == "CallExpr" && callee_name(e).is_some_and(|n| ast::is_assert_failure(&n))
            });
        let line = self.line(cond);
        let proof = if asserted {
            Proof::Asserted(line)
        } else {
            Proof::Tested(line)
        };
        let (on_true, on_false) = self.condition(cond, state, proof);
        let after_then = self.stmt(then, on_true);
        let after_else = match otherwise {
            Some(otherwise) => self.stmt(otherwise, on_false),
            None => on_false,
        };
        after_then.join(&after_else)
    }

    fn switch_stmt(&mut self, node: &Value, state: State) -> State {
        let parts = kids(node).filter(|c| present(c)).collect::<Vec<_>>();
        let (Some(cond), Some(body)) = (parts.first(), parts.last()) else {
            self.reason("null-unsupported-form: SwitchStmt".into());
            return state;
        };
        let mut state = state;
        self.eval(cond, &mut state);
        state.definite = false;
        let has_default = contains_default(body);
        let formals = state.firsts.len();
        self.loops.push(LoopFrame {
            breaks: State::dead(formals),
            continues: State::dead(formals),
            switch: true,
        });
        self.switches.push((state.clone(), self.loops.len()));
        // The body is entered only through its labels.
        let end = self.stmt(body, State::dead(formals));
        self.switches.pop();
        let frame = self.loops.pop().expect("the switch frame");
        let mut out = end.join(&frame.breaks);
        if !has_default {
            out = out.join(&state);
        }
        out
    }

    fn loop_stmt(
        &mut self,
        cond: Option<&Value>,
        body: &Value,
        inc: Option<&Value>,
        body_first: bool,
        entry: State,
    ) -> State {
        let entry = entry.not_definite();
        let formals = entry.firsts.len();
        let recording = self.recording;
        self.recording = false;
        let mut head = entry.clone();
        let mut converged = false;
        for _ in 0..LOOP_ITERATIONS {
            let (_, back) = self.loop_once(cond, body, inc, body_first, head.clone(), formals);
            let next = entry.join(&back);
            if next == head {
                converged = true;
                break;
            }
            head = next;
        }
        self.recording = recording;
        if !converged {
            self.reason("null-loop-unconverged: a loop's facts did not reach a fixed point".into());
        }
        let (exit, _) = self.loop_once(cond, body, inc, body_first, head, formals);
        exit
    }

    /// One pass of a loop from `head`: the exit state and the back edge.
    fn loop_once(
        &mut self,
        cond: Option<&Value>,
        body: &Value,
        inc: Option<&Value>,
        body_first: bool,
        head: State,
        formals: usize,
    ) -> (State, State) {
        self.loops.push(LoopFrame {
            breaks: State::dead(formals),
            continues: State::dead(formals),
            switch: false,
        });
        let (exit_test, back) = if body_first {
            let after_body = self.stmt(body, head);
            let frame_continues = self
                .loops
                .last()
                .map(|f| f.continues.clone())
                .expect("frame");
            let before_test = after_body.join(&frame_continues);
            match cond {
                Some(cond) => {
                    let line = self.line(cond);
                    let (on_true, on_false) =
                        self.condition(cond, before_test, Proof::Tested(line));
                    (on_false, on_true)
                }
                None => (State::dead(formals), before_test),
            }
        } else {
            let (on_true, on_false) = match cond {
                Some(cond) => {
                    let line = self.line(cond);
                    self.condition(cond, head, Proof::Tested(line))
                }
                None => (head, State::dead(formals)),
            };
            let after_body = self.stmt(body, on_true);
            let frame_continues = self
                .loops
                .last()
                .map(|f| f.continues.clone())
                .expect("frame");
            let mut back = after_body.join(&frame_continues);
            if let Some(inc) = inc {
                self.eval(inc, &mut back);
            }
            (on_false, back)
        };
        let frame = self.loops.pop().expect("the loop frame");
        (exit_test.join(&frame.breaks), back)
    }
}

/// Whether `node` reads the first character of a string literal (`*"x"`,
/// through value-preserving casts), and whether that character is nonzero.
fn string_literal_first_char(node: &Value) -> Option<bool> {
    let mut node = strip(node);
    while kind(node) == "ImplicitCastExpr"
        && matches!(
            node["castKind"].as_str(),
            Some("LValueToRValue" | "IntegralCast" | "NoOp")
        )
    {
        node = strip(kids(node).next()?);
    }
    if kind(node) != "UnaryOperator" || node["opcode"] != "*" {
        return None;
    }
    let mut operand = strip(kids(node).next()?);
    while kind(operand) == "ImplicitCastExpr"
        && matches!(
            operand["castKind"].as_str(),
            Some("ArrayToPointerDecay" | "NoOp")
        )
    {
        operand = strip(kids(operand).next()?);
    }
    if kind(operand) != "StringLiteral" {
        return None;
    }
    let text = operand["value"].as_str()?;
    // The JSON spelling is the source literal, quotes included.
    let first = text.strip_prefix('"')?.chars().next()?;
    Some(first != '"' && !text.starts_with("\"\\0"))
}

/// The expression of a generic selection's selected association.
fn selected_association(node: &Value) -> Option<&Value> {
    kids(node)
        .find(|c| c["selected"] == true)
        .and_then(|association| kids(association).next_back())
}

fn contains_default(node: &Value) -> bool {
    match kind(node) {
        "DefaultStmt" => true,
        "SwitchStmt" => false,
        _ => kids(node).any(contains_default),
    }
}

const STATEMENT_KINDS: [&str; 19] = [
    "CompoundStmt",
    "DeclStmt",
    "NullStmt",
    "IfStmt",
    "WhileStmt",
    "DoStmt",
    "ForStmt",
    "SwitchStmt",
    "CaseStmt",
    "DefaultStmt",
    "BreakStmt",
    "ContinueStmt",
    "ReturnStmt",
    "LabelStmt",
    "GotoStmt",
    "IndirectGotoStmt",
    "AttributedStmt",
    "GCCAsmStmt",
    "MSAsmStmt",
];

pub(super) fn is_expression_kind(node: &Value) -> bool {
    is_expression(node)
}

fn is_expression(node: &Value) -> bool {
    !STATEMENT_KINDS.contains(&kind(node))
}

/// The worse of two outcomes recorded for one check (a check reached twice).
fn worse(a: &Outcome, b: &Outcome) -> Outcome {
    let rank = |o: &Outcome| match o {
        Outcome::Excluded => 0,
        Outcome::Proved(_) => 1,
        Outcome::Moved(_) => 2,
        Outcome::Held(_) => 3,
        Outcome::Candidate(_) => 4,
        Outcome::Violation { reachable: false } => 5,
        Outcome::Violation { reachable: true } => 6,
    };
    match (a, b) {
        (Outcome::Candidate(x), Outcome::Candidate(y)) => {
            Outcome::Candidate(x.union(y).cloned().collect())
        }
        _ if rank(a) >= rank(b) => a.clone(),
        _ => b.clone(),
    }
}
