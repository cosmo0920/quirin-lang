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

//! Card **OP-CONSTRUCTOR-DECLARATIONS** (gateway rules 12o, 13w and 14e;
//! contract spec §C04.19.4): the facts that let a constructor's stated result
//! state stand at a later call, and the body check of the `absent` state.
//!
//! A constructor states its result's discriminant
//! (`resource_discriminant_set`) and the state of a function-pointer member
//! (`resource_callback_member_registered`, or `absent` by rule 14e). The
//! calling body records that state per path on the tracked result (rules 12o,
//! 13w). It can be read at a later call only while nothing changed the
//! member, so every store into it **anywhere in the run** is classified
//! ([`StateStore`]), over Clang's AST of every analysed unit (premise
//! `P-OPCTOR-STORES-SEEN`: every unit that can store into the member is
//! analysed, and no store is type-punned past the record):
//!
//! * a store into an object the storing function obtained from a direct call
//!   only (a local assigned nowhere else) — a constructor's own result, never
//!   another activation's object once the callee is checked to return an
//!   owned result ([`StateStore::Fresh`]); a store into an automatic record
//!   ([`StateStore::Automatic`]);
//! * a null pointer constant into a function-pointer member (it removes a
//!   candidate: reading the stated set stays a superset);
//! * a flag-or `P->F |= E`, `E` folding to constants ([`StateStore::FlagOr`]):
//!   the reachable values gain those bits;
//! * a rewrite `P->F = C` inside a case group of a `switch` reading the same
//!   `P->F` (`& E` allowed), `P` never assigned in the function
//!   ([`StateStore::Rewrite`]): the reachable values gain `C` when one of them
//!   satisfies a label (the `PARTITION_LEAVE` → `PARTITION_JOIN` rewrite);
//! * anything else, and a whole-object write of the record (an assignment, a
//!   `memcpy`- or `memset`-family call), is unclassified and withdraws every
//!   per-path state on the member, in the unit and, through the fold, in the
//!   run.
//!
//! A function that stores into the member itself never reads a per-path
//! state on it (its own stores are not replayed on the path: the state is
//! open there).
//!
//! The body check of rule 14e ([`result_state_validates`]) reads the
//! constructor's own Clang body: one local is returned on every `return`, is
//! assigned once from a zeroing allocator (`calloc`, `rd_calloc`), is used
//! only as the base of member stores, the return and inside `sizeof`, and no
//! store reaches an `absent` member; a stated discriminant is stored exactly
//! once, from its parameter (with its flags).

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::constructed_state::{ConstructedState, MemberValue, Rewrite};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::arm_release_check::fold_with;
use crate::callback_member_closure::{MemberFacts, direct_callee, null_constant};
use crate::typed_c_ast::{children, kind};

/// The zeroing allocators rule 14e reads a fresh object from (`rd_calloc`
/// is `calloc` and an assertion, `rd.h`).
pub const ZEROING_ALLOCATORS: [&str; 2] = ["calloc", "rd_calloc"];

/// The calls that write a whole object (copies and fills).
const WHOLE_WRITERS: [&str; 11] = [
    "memcpy",
    "memmove",
    "bcopy",
    "memset",
    "bzero",
    "explicit_bzero",
    "__builtin_memcpy",
    "__builtin_memmove",
    "__builtin_memset",
    "__builtin___memcpy_chk",
    "__builtin___memset_chk",
];

/// One classified store into a member, with the function that makes it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "store")]
pub enum StateStore {
    /// Into an object a local holds that only direct calls of `callees`
    /// assign.
    Fresh {
        function: String,
        callees: BTreeSet<String>,
    },
    /// Into an automatic record object of the function.
    Automatic { function: String },
    /// A null pointer constant into a function-pointer member.
    Null { function: String },
    /// `P->F |= E`, `E` folding to `bits`.
    FlagOr { function: String, bits: u64 },
    /// A case-guarded rewrite.
    Rewrite { function: String, rewrite: Rewrite },
    /// Anything else.
    Other { function: String, why: String },
}

impl StateStore {
    #[must_use]
    pub fn function(&self) -> &str {
        match self {
            Self::Fresh { function, .. }
            | Self::Automatic { function }
            | Self::Null { function }
            | Self::FlagOr { function, .. }
            | Self::Rewrite { function, .. }
            | Self::Other { function, .. } => function,
        }
    }
}

/// One unit's classified stores into the members a result state names.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateFacts {
    /// `TAG.PATH` -> its classified stores.
    pub stores: BTreeMap<String, BTreeSet<StateStore>>,
    /// Records a whole-object write may change, with why.
    pub whole_writes: BTreeMap<String, String>,
}

/// The members (`TAG.PATH`) a set of result states names.
#[must_use]
pub fn stated_keys<'a>(states: impl IntoIterator<Item = &'a ConstructedState>) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for state in states {
        if let Some(set) = &state.discriminant {
            keys.insert(set.member.spelling());
        }
        for member in &state.members {
            keys.insert(member.member.spelling());
        }
    }
    keys
}

fn strip_all(mut node: &Value) -> &Value {
    loop {
        match kind(node) {
            "ParenExpr" | "ImplicitCastExpr" | "CStyleCastExpr" => match children(node).first() {
                Some(inner) => node = inner,
                None => return node,
            },
            _ => return node,
        }
    }
}

fn last_segment(member: &str) -> &str {
    member.rsplit('.').next().unwrap_or(member)
}

fn record_of_key(member: &str) -> &str {
    member.split_once('.').map_or(member, |(tag, _)| tag)
}

/// The variable a node reads, when it is a plain reference to one.
fn variable(node: &Value) -> Option<&str> {
    let node = strip_all(node);
    (kind(node) == "DeclRefExpr"
        && matches!(
            node["referencedDecl"]["kind"].as_str(),
            Some("VarDecl" | "ParmVarDecl")
        ))
    .then(|| node["referencedDecl"]["id"].as_str())
    .flatten()
}

/// Every value a constant expression may have: a conditional's arms, folded.
fn constant_values(node: &Value, enumerators: &BTreeMap<String, i64>) -> Option<BTreeSet<i64>> {
    let node = strip_all(node);
    if kind(node) == "ConditionalOperator" {
        let arms = children(node);
        if arms.len() != 3 {
            return None;
        }
        let mut out = constant_values(&arms[1], enumerators)?;
        out.extend(constant_values(&arms[2], enumerators)?);
        return Some(out);
    }
    fold_with(node, enumerators).map(|value| BTreeSet::from([value]))
}

/// The base of a member chain and whether the outermost step through it is an
/// arrow: `V->a.b` is `(V, true)`, `s.a` is `(s, false)`.
fn member_base(member: &Value) -> Option<(&Value, bool)> {
    let mut node = member;
    loop {
        let base = children(node).first()?;
        if node["isArrow"].as_bool() == Some(true) {
            return Some((strip_all(base), true));
        }
        let base = strip_all(base);
        if kind(base) == "MemberExpr" {
            node = base;
            continue;
        }
        return Some((base, false));
    }
}

/// What a function's locals are: assigned only from direct calls (with the
/// callees), and which variables are assigned at all after declaration.
#[derive(Default)]
struct Locals {
    /// Local id -> callees of every assignment (`None`: something else).
    sources: BTreeMap<String, Option<BTreeSet<String>>>,
    /// Automatic (non-static) record locals.
    automatic: BTreeSet<String>,
    /// Variables assigned other than by their initializer (a null pointer
    /// constant excepted: a store through it after that reaches no object), or
    /// whose address is taken.
    rebound: BTreeSet<String>,
}

impl Locals {
    fn read(function: &Value) -> Self {
        let mut locals = Self::default();
        locals.walk(function);
        locals
    }

    fn assign(&mut self, id: &str, value: Option<&Value>) {
        let callee = value
            .map(strip_all)
            .filter(|value| kind(value) == "CallExpr")
            .and_then(direct_callee)
            .map(str::to_owned);
        let entry = self
            .sources
            .entry(id.to_owned())
            .or_insert_with(|| Some(BTreeSet::new()));
        match (entry.as_mut(), callee) {
            (Some(callees), Some(callee)) => {
                callees.insert(callee);
            }
            _ => *entry = None,
        }
    }

    fn walk(&mut self, node: &Value) {
        match kind(node) {
            "VarDecl" => {
                if let Some(id) = node["id"].as_str()
                    && node["storageClass"].is_null()
                {
                    let ty = node["type"]["qualType"].as_str().unwrap_or("");
                    if !ty.contains('*') && (ty.starts_with("struct ") || !ty.contains('(')) {
                        self.automatic.insert(id.to_owned());
                    }
                    let init = children(node).first();
                    if init.is_some() {
                        self.assign(id, init);
                    } else {
                        self.sources
                            .entry(id.to_owned())
                            .or_insert_with(|| Some(BTreeSet::new()));
                    }
                } else if let Some(id) = node["id"].as_str() {
                    // A static local: never fresh.
                    self.sources.insert(id.to_owned(), None);
                    self.rebound.insert(id.to_owned());
                }
            }
            "BinaryOperator" if node["opcode"].as_str() == Some("=") => {
                if let [left, right] = children(node)
                    && let Some(id) = variable(left)
                {
                    self.assign(id, Some(right));
                    if !null_constant(right) {
                        self.rebound.insert(id.to_owned());
                    }
                }
            }
            "CompoundAssignOperator" => {
                if let Some(id) = children(node).first().and_then(variable) {
                    self.sources.insert(id.to_owned(), None);
                    self.rebound.insert(id.to_owned());
                }
            }
            "UnaryOperator" if matches!(node["opcode"].as_str(), Some("&" | "++" | "--")) => {
                if let Some(id) = children(node).first().and_then(variable) {
                    self.sources.insert(id.to_owned(), None);
                    self.rebound.insert(id.to_owned());
                }
            }
            _ => {}
        }
        for child in children(node) {
            self.walk(child);
        }
    }

    fn fresh(&self, id: &str) -> Option<&BTreeSet<String>> {
        self.sources
            .get(id)
            .and_then(Option::as_ref)
            .filter(|callees| !callees.is_empty())
    }
}

/// A switch on `V->F` in force around a statement.
struct Guard {
    key: String,
    variable: String,
    mask: Option<u64>,
    /// `None`: a label the check cannot fold, `default:` or a jump target.
    labels: Option<BTreeSet<i64>>,
}

struct Collector<'a> {
    facts: &'a MemberFacts,
    enumerators: &'a BTreeMap<String, i64>,
    keys: &'a BTreeSet<String>,
    /// `?.FIELD` spellings of the keys (a store whose record is not read).
    unread: BTreeMap<String, String>,
    records: BTreeSet<String>,
    out: StateFacts,
}

impl Collector<'_> {
    fn key_of(&self, member: &Value) -> Option<String> {
        let key = self.facts.member_key(member);
        if self.keys.contains(&key) {
            return Some(key);
        }
        self.unread.get(&key).cloned()
    }

    /// A write of a whole sub-object `place` (a record-typed member path, or
    /// `&` of one): every stated key under it is stored opaquely. `true` when
    /// `place` is such a member path (handled), `false` for anything else.
    fn sub_object(&mut self, name: &str, place: &Value, why: &str) -> bool {
        let place = strip_all(place);
        let place = if kind(place) == "UnaryOperator" && place["opcode"].as_str() == Some("&") {
            match children(place).first() {
                Some(inner) => strip_all(inner),
                None => return false,
            }
        } else {
            place
        };
        if kind(place) != "MemberExpr" {
            return false;
        }
        let key = self.facts.member_key(place);
        let under = self
            .keys
            .iter()
            .filter(|stated| stated.starts_with(&format!("{key}.")) || **stated == key)
            .cloned()
            .collect::<Vec<_>>();
        for stated in under {
            self.record(
                stated,
                StateStore::Other {
                    function: name.to_owned(),
                    why: why.to_owned(),
                },
            );
        }
        true
    }

    fn record(&mut self, key: String, store: StateStore) {
        self.out.stores.entry(key).or_default().insert(store);
    }

    fn function(&mut self, name: &str, body: &Value) {
        let locals = Locals::read(body);
        let mut guards = Vec::new();
        self.walk(name, body, &locals, &mut guards);
    }

    fn base_class(&self, name: &str, member: &Value, locals: &Locals) -> Option<StateStore> {
        let (base, arrow) = member_base(member)?;
        let id = variable(base)?;
        if arrow {
            locals.fresh(id).map(|callees| StateStore::Fresh {
                function: name.to_owned(),
                callees: callees.clone(),
            })
        } else {
            locals
                .automatic
                .contains(id)
                .then(|| StateStore::Automatic {
                    function: name.to_owned(),
                })
        }
    }

    fn rewrite(
        &self,
        key: &str,
        member: &Value,
        value: &Value,
        locals: &Locals,
        guards: &[Guard],
    ) -> Option<Rewrite> {
        let (base, true) = member_base(member)? else {
            return None;
        };
        let id = variable(base)?;
        if locals.rebound.contains(id) {
            return None;
        }
        let guard = guards
            .iter()
            .rev()
            .find(|guard| guard.key == key && guard.variable == id)?;
        let from = guard.labels.clone()?;
        let to = fold_with(strip_all(value), self.enumerators)?;
        Some(Rewrite {
            mask: guard.mask,
            from,
            to,
        })
    }

    fn store(
        &mut self,
        name: &str,
        place: &Value,
        opcode: &str,
        value: Option<&Value>,
        locals: &Locals,
        guards: &[Guard],
    ) {
        let place = strip_all(place);
        if kind(place) != "MemberExpr" {
            return;
        }
        let Some(key) = self.key_of(place) else {
            return;
        };
        if key.starts_with("?.") || self.facts.member_key(place).starts_with("?.") {
            self.record(
                key,
                StateStore::Other {
                    function: name.to_owned(),
                    why: "a store whose record is not read".to_owned(),
                },
            );
            return;
        }
        if let Some(class) = self.base_class(name, place, locals) {
            self.record(key, class);
            return;
        }
        let class = match (opcode, value) {
            ("=", Some(value)) if null_constant(value) && self.facts.function_pointer(place) => {
                StateStore::Null {
                    function: name.to_owned(),
                }
            }
            ("=", Some(value)) => match self.rewrite(&key, place, value, locals, guards) {
                Some(rewrite) => StateStore::Rewrite {
                    function: name.to_owned(),
                    rewrite,
                },
                None => StateStore::Other {
                    function: name.to_owned(),
                    why: "a store into an existing object".to_owned(),
                },
            },
            ("|=", Some(value)) => match constant_values(value, self.enumerators) {
                Some(values) => StateStore::FlagOr {
                    function: name.to_owned(),
                    bits: values
                        .iter()
                        .fold(0u64, |bits, value| bits | (*value as u64)),
                },
                None => StateStore::Other {
                    function: name.to_owned(),
                    why: "a flag-or of a value that does not fold".to_owned(),
                },
            },
            (opcode, _) => StateStore::Other {
                function: name.to_owned(),
                why: format!("a `{opcode}` of an existing object's member"),
            },
        };
        self.record(key, class);
    }

    fn switch_guard(&self, condition: &Value) -> Option<Guard> {
        let condition = strip_all(condition);
        let (member, mask) =
            if kind(condition) == "BinaryOperator" && condition["opcode"].as_str() == Some("&") {
                let [left, right] = children(condition) else {
                    return None;
                };
                let kept = fold_with(strip_all(right), self.enumerators)?;
                (strip_all(left), Some(!(kept as u64)))
            } else {
                (condition, None)
            };
        if kind(member) != "MemberExpr" || member["isArrow"].as_bool() != Some(true) {
            return None;
        }
        let key = self.key_of(member)?;
        let (base, true) = member_base(member)? else {
            return None;
        };
        Some(Guard {
            key,
            variable: variable(base)?.to_owned(),
            mask,
            labels: Some(BTreeSet::new()),
        })
    }

    fn walk(&mut self, name: &str, node: &Value, locals: &Locals, guards: &mut Vec<Guard>) {
        match kind(node) {
            "BinaryOperator" if node["opcode"].as_str() == Some("=") => {
                if let [left, right] = children(node) {
                    self.store(name, left, "=", Some(right), locals, guards);
                    if let Some(record) = self.facts.record_of_node(strip_all(left), false)
                        && !self.sub_object(name, left, "a sub-object assignment covers it")
                        && self.records.contains(&record)
                    {
                        self.out
                            .whole_writes
                            .entry(record)
                            .or_insert_with(|| format!("a whole-object assignment in `{name}`"));
                    }
                }
            }
            "CompoundAssignOperator" => {
                if let [left, right] = children(node) {
                    let opcode = node["opcode"].as_str().unwrap_or("?").to_owned();
                    self.store(name, left, &opcode, Some(right), locals, guards);
                }
            }
            "UnaryOperator" => {
                if let (Some(opcode), Some(operand)) =
                    (node["opcode"].as_str(), children(node).first())
                    && matches!(opcode, "++" | "--" | "&")
                {
                    let opcode = if opcode == "&" { "&" } else { opcode };
                    let operand = strip_all(operand);
                    if kind(operand) == "MemberExpr"
                        && let Some(key) = self.key_of(operand)
                    {
                        // An address taken of the member escapes the walk,
                        // even from a fresh object.
                        if opcode == "&" {
                            self.record(
                                key,
                                StateStore::Other {
                                    function: name.to_owned(),
                                    why: "its address is taken".to_owned(),
                                },
                            );
                        } else {
                            self.store(name, operand, opcode, None, locals, guards);
                        }
                    }
                }
            }
            "CallExpr" => {
                if let Some(callee) = direct_callee(node)
                    && WHOLE_WRITERS.contains(&callee)
                    && let Some(destination) = children(node).get(1)
                    && !self.sub_object(
                        name,
                        destination,
                        &format!("a `{callee}` into a sub-object covers it"),
                    )
                    && let Some(record) = self.facts.record_of_node(strip_all(destination), true)
                    && self.records.contains(&record)
                {
                    self.out
                        .whole_writes
                        .entry(record)
                        .or_insert_with(|| format!("a `{callee}` into it in `{name}`"));
                }
            }
            "SwitchStmt" => {
                let parts = children(node);
                if let ([condition, body], Some(guard)) = (
                    parts,
                    parts
                        .first()
                        .and_then(|condition| self.switch_guard(condition)),
                ) {
                    self.walk(name, condition, locals, guards);
                    guards.push(guard);
                    self.switch_body(name, body, locals, guards);
                    guards.pop();
                    return;
                }
            }
            _ => {}
        }
        for child in children(node) {
            self.walk(name, child, locals, guards);
        }
    }

    /// A guarded switch's body: labels accumulate over fall-through and reset
    /// at a top-level `break` or `return`.
    fn switch_body(&mut self, name: &str, body: &Value, locals: &Locals, guards: &mut Vec<Guard>) {
        let statements: Vec<&Value> = if kind(body) == "CompoundStmt" {
            children(body).iter().collect()
        } else {
            vec![body]
        };
        for statement in statements {
            let mut inner = statement;
            loop {
                match kind(inner) {
                    "CaseStmt" => {
                        let parts = children(inner);
                        let value = parts
                            .first()
                            .and_then(|label| fold_with(label, self.enumerators));
                        if let Some(top) = guards.last_mut() {
                            match (value, top.labels.as_mut(), parts.len()) {
                                (Some(value), Some(labels), 2) => {
                                    labels.insert(value);
                                }
                                _ => top.labels = None,
                            }
                        }
                        match parts.last() {
                            Some(next) if parts.len() >= 2 => inner = next,
                            _ => break,
                        }
                    }
                    "DefaultStmt" | "LabelStmt" => {
                        if let Some(top) = guards.last_mut() {
                            top.labels = None;
                        }
                        match children(inner).last() {
                            Some(next) => inner = next,
                            None => break,
                        }
                    }
                    _ => break,
                }
            }
            self.walk(name, inner, locals, guards);
            if matches!(kind(inner), "BreakStmt" | "ReturnStmt")
                && let Some(top) = guards.last_mut()
            {
                top.labels = Some(BTreeSet::new());
            }
        }
    }
}

/// One unit's classified stores into `keys`, over every function body it
/// defines (`functions`), reading records and typedefs as `facts` does.
#[must_use]
pub fn collect<'a>(
    facts: &MemberFacts,
    functions: impl IntoIterator<Item = (&'a str, &'a Value)>,
    keys: &BTreeSet<String>,
) -> StateFacts {
    let mut collector = Collector {
        facts,
        enumerators: facts.enumerators(),
        keys,
        unread: keys
            .iter()
            .map(|key| (format!("?.{}", last_segment(key)), key.clone()))
            .collect(),
        records: keys
            .iter()
            .map(|key| record_of_key(key).to_owned())
            .collect(),
        out: StateFacts::default(),
    };
    for (name, function) in functions {
        if let Some(body) = children(function)
            .iter()
            .find(|child| kind(child) == "CompoundStmt")
        {
            collector.function(name, body);
        }
    }
    collector.out
}

/// One unit's classified stores into `keys` read from its whole Clang AST,
/// for a unit the gateway does not otherwise analyse (a data-only unit, card
/// CALLBACK-MEMBER-DATA-ONLY-UNITS): every function body Clang read, a
/// header's inline function included. `Err` when Clang or the JSON fails;
/// the caller then records no facts, which opens every stated member.
///
/// # Errors
///
/// Clang's or the JSON reader's failure.
pub fn read_unit(
    source: &str,
    unit: &crate::legacy_c::C11TranslationUnit<'_>,
    keys: &BTreeSet<String>,
) -> Result<StateFacts, String> {
    let root = crate::typed_c_ast::translation_unit_ast_with(source, unit, |bytes| {
        crate::deep_unit_ast::from_slice::<Value>(bytes)
    })?;
    if kind(&root) != "TranslationUnitDecl" {
        return Err("expected Clang TranslationUnitDecl".to_owned());
    }
    let mut facts = MemberFacts::default();
    for node in children(&root) {
        facts.collect(node);
    }
    let functions = children(&root)
        .iter()
        .filter(|node| kind(node) == "FunctionDecl")
        .filter_map(|node| node["name"].as_str().map(|name| (name, node)));
    Ok(collect(&facts, functions, keys))
}

/// What the stores of a member let a stated state carry: the rewrites and
/// flag bits a decided discriminant's values close under.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Stable {
    pub rewrites: Vec<Rewrite>,
    pub flags: BTreeSet<u64>,
}

/// Whether the stores of `key` over `units` keep a stated state:
/// `Ok` with what a decided value closes under, `Err` with every reason it
/// does not. A unit with no facts (`None`) opens it. `owned` says whether a
/// callee hands its caller an owned result (a fresh object).
///
/// # Errors
///
/// Every unclassified store, whole-object write and missing unit.
pub fn stability(
    units: &[Option<&StateFacts>],
    key: &str,
    discriminant: bool,
    owned: &dyn Fn(&str) -> bool,
) -> Result<Stable, BTreeSet<String>> {
    let mut stable = Stable::default();
    let mut open = BTreeSet::new();
    let record = record_of_key(key);
    for unit in units {
        let Some(unit) = unit else {
            open.insert("an analysed unit recorded no facts".to_owned());
            continue;
        };
        if let Some(why) = unit.whole_writes.get(record) {
            open.insert(format!("{why} writes a `struct {record}`"));
        }
        for store in unit.stores.get(key).into_iter().flatten() {
            match store {
                StateStore::Fresh { function, callees } => {
                    if let Some(callee) = callees.iter().find(|callee| !owned(callee)) {
                        open.insert(format!(
                            "`{function}` stores into an object `{callee}` returned, which no \
                             declaration hands over owned"
                        ));
                    }
                }
                StateStore::Automatic { .. } => {}
                StateStore::Null { .. } if !discriminant => {}
                StateStore::FlagOr { bits, .. } if discriminant => {
                    stable.flags.insert(*bits);
                }
                StateStore::Rewrite { rewrite, .. } if discriminant => {
                    if !stable.rewrites.contains(rewrite) {
                        stable.rewrites.push(rewrite.clone());
                    }
                }
                StateStore::Null { function }
                | StateStore::FlagOr { function, .. }
                | StateStore::Rewrite { function, .. } => {
                    open.insert(format!("`{function}` stores a value of another kind"));
                }
                StateStore::Other { function, why } => {
                    open.insert(format!("`{function}`: {why}"));
                }
            }
        }
    }
    if open.is_empty() {
        Ok(stable)
    } else {
        Err(open)
    }
}

/// Whether `function` itself stores into `key` (its per-path state is then
/// open there).
#[must_use]
pub fn stores_into(facts: &StateFacts, key: &str, function: &str) -> bool {
    facts
        .stores
        .get(key)
        .is_some_and(|stores| stores.iter().any(|store| store.function() == function))
}

/// The values a decided discriminant may hold: the least set containing
/// `value`, closed under the flag bits and the classified rewrites.
#[must_use]
pub fn reachable(value: i64, stable: &Stable) -> BTreeSet<i64> {
    let mut out = BTreeSet::from([value]);
    loop {
        let mut more = BTreeSet::new();
        for current in &out {
            for bits in &stable.flags {
                more.insert(current | (*bits as i64));
            }
            for rewrite in &stable.rewrites {
                if rewrite.applies(*current) {
                    more.insert(rewrite.to);
                }
            }
        }
        if more.is_subset(&out) {
            return out;
        }
        out.extend(more);
    }
}

/// Rule 14e: whether a constructor's Clang body validates its stated result
/// state (only a state stating `absent` is checked).
///
/// # Errors
///
/// Why the body does not validate it.
pub fn result_state_validates(
    function: &Value,
    state: &ConstructedState,
    enumerators: &BTreeMap<String, i64>,
) -> Result<(), String> {
    let parameters = children(function)
        .iter()
        .filter(|child| kind(child) == "ParmVarDecl")
        .map(|parameter| parameter["id"].as_str().unwrap_or(""))
        .collect::<Vec<_>>();
    let body = children(function)
        .iter()
        .find(|child| kind(child) == "CompoundStmt")
        .ok_or("the body is not read")?;
    // Every return names one local.
    let mut returns = Vec::new();
    find(body, "ReturnStmt", &mut returns);
    let returned = returns
        .iter()
        .map(|stmt| children(stmt).first().and_then(variable))
        .collect::<Option<BTreeSet<_>>>()
        .ok_or("a return does not name a local")?;
    let [result] = returned.into_iter().collect::<Vec<_>>()[..] else {
        return Err("the returns do not name one local".to_owned());
    };
    if parameters.contains(&result) {
        return Err("the result is a parameter".to_owned());
    }
    // Assigned once, from a zeroing allocator.
    let locals = Locals::read(body);
    let mut assignments = 0usize;
    count_assignments(body, result, &mut assignments);
    match locals.sources.get(result) {
        Some(Some(callees))
            if assignments == 1
                && callees.len() == 1
                && callees
                    .iter()
                    .all(|callee| ZEROING_ALLOCATORS.contains(&callee.as_str())) => {}
        _ => {
            return Err(format!(
                "the result is not assigned once from a zeroing allocator ({})",
                ZEROING_ALLOCATORS.join(", ")
            ));
        }
    }
    // Every other use of the local is a member store's base, the return or
    // inside `sizeof`.
    let mut uses = Uses::default();
    uses.walk(body, result, false);
    if uses.other > 0 {
        return Err("the result is handed on or read other than as a store's base".to_owned());
    }
    for member in &state.members {
        if member.value == MemberValue::Absent
            && uses.stored.iter().any(|(path, _)| {
                path == &member.member.path
                    || path.starts_with(&format!("{}.", member.member.path))
                    || member.member.path.starts_with(&format!("{path}."))
            })
        {
            return Err(format!(
                "the body stores into `{}`, stated `absent`",
                member.member.spelling()
            ));
        }
    }
    if let Some(set) = &state.discriminant {
        let parameter = parameters
            .get(set.parameter)
            .ok_or("the discriminant's parameter is not read")?;
        let stores = uses
            .stored
            .iter()
            .filter(|(path, _)| *path == set.member.path)
            .collect::<Vec<_>>();
        let [(_, Some(value))] = stores.as_slice() else {
            return Err(format!(
                "the body does not store `{}` exactly once",
                set.member.path
            ));
        };
        let value = strip_all(value);
        let matches = if set.flags == 0 {
            variable(value) == Some(parameter)
        } else {
            kind(value) == "BinaryOperator"
                && value["opcode"].as_str() == Some("|")
                && matches!(children(value), [left, right]
                    if variable(left) == Some(parameter)
                        && fold_with(strip_all(right), enumerators) == i64::try_from(set.flags).ok())
        };
        if !matches {
            return Err(format!(
                "`{}` is not stored from the stated parameter and flags",
                set.member.path
            ));
        }
    }
    Ok(())
}

fn find<'a>(node: &'a Value, wanted: &str, out: &mut Vec<&'a Value>) {
    if kind(node) == wanted {
        out.push(node);
    }
    for child in children(node) {
        find(child, wanted, out);
    }
}

fn count_assignments(node: &Value, id: &str, count: &mut usize) {
    match kind(node) {
        "VarDecl" if node["id"].as_str() == Some(id) && !children(node).is_empty() => *count += 1,
        "BinaryOperator"
            if node["opcode"].as_str() == Some("=")
                && children(node).first().and_then(variable) == Some(id) =>
        {
            *count += 1;
        }
        _ => {}
    }
    for child in children(node) {
        count_assignments(child, id, count);
    }
}

/// The uses of the result local: member stores (path, value) and anything
/// else.
#[derive(Default)]
struct Uses<'a> {
    stored: Vec<(String, Option<&'a Value>)>,
    other: usize,
}

fn member_path(member: &Value, id: &str) -> Option<String> {
    let mut segments = Vec::new();
    let mut node = member;
    loop {
        segments.push(node["name"].as_str()?.to_owned());
        let base = strip_all(children(node).first()?);
        if node["isArrow"].as_bool() == Some(true) {
            if variable(base) != Some(id) {
                return None;
            }
            segments.reverse();
            return Some(segments.join("."));
        }
        if kind(base) != "MemberExpr" {
            return None;
        }
        node = base;
    }
}

impl<'a> Uses<'a> {
    fn walk(&mut self, node: &'a Value, id: &str, in_store: bool) {
        match kind(node) {
            "UnaryExprOrTypeTraitExpr" => return,
            "ReturnStmt" if children(node).first().and_then(variable) == Some(id) => return,
            "VarDecl" if node["id"].as_str() == Some(id) => {
                for child in children(node) {
                    self.walk(child, id, false);
                }
                return;
            }
            "BinaryOperator" | "CompoundAssignOperator" => {
                if let [left, right] = children(node) {
                    let place = strip_all(left);
                    if node["opcode"].as_str() == Some("=") && variable(place) == Some(id) {
                        self.walk(right, id, false);
                        return;
                    }
                    if kind(place) == "MemberExpr"
                        && let Some(path) = member_path(place, id)
                    {
                        let value = (node["opcode"].as_str() == Some("=")).then_some(right);
                        if value.is_none() {
                            // A compound update of the result's member.
                            self.stored.push((path, None));
                            self.other += 1;
                        } else {
                            self.stored.push((path, value));
                        }
                        self.walk_member_bases(place, id);
                        self.walk(right, id, false);
                        return;
                    }
                }
            }
            "DeclRefExpr" if variable(node) == Some(id) && !in_store => {
                self.other += 1;
                return;
            }
            _ => {}
        }
        for child in children(node) {
            self.walk(child, id, in_store);
        }
    }

    /// The subscripts of a stored member path are ordinary reads.
    fn walk_member_bases(&mut self, member: &'a Value, id: &str) {
        let mut node = member;
        while kind(node) == "MemberExpr" {
            let Some(base) = children(node).first() else {
                return;
            };
            node = strip_all(base);
        }
        if variable(node) != Some(id) {
            self.walk(node, id, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qcore_schemas::callback::CallbackMember;
    use qcore_schemas::constructed_state::{DiscriminantSet, MemberState};

    fn stable(rewrites: &[(i64, i64)], flags: &[u64]) -> Stable {
        Stable {
            rewrites: rewrites
                .iter()
                .map(|(from, to)| Rewrite {
                    mask: None,
                    from: BTreeSet::from([*from]),
                    to: *to,
                })
                .collect(),
            flags: flags.iter().copied().collect(),
        }
    }

    #[test]
    fn reachable_values_close_under_flags_and_rewrites() {
        assert_eq!(
            reachable(17, &stable(&[(17, 16)], &[])),
            BTreeSet::from([16, 17])
        );
        assert_eq!(reachable(9, &stable(&[(17, 16)], &[])), BTreeSet::from([9]));
        assert_eq!(
            reachable(6, &stable(&[], &[1 << 29, 1 << 30])),
            BTreeSet::from([6, 6 | (1 << 29), 6 | (1 << 30), 6 | (1 << 29) | (1 << 30)])
        );
    }

    fn facts(stores: &[(&str, StateStore)]) -> StateFacts {
        let mut out = StateFacts::default();
        for (key, store) in stores {
            out.stores
                .entry((*key).to_owned())
                .or_default()
                .insert(store.clone());
        }
        out
    }

    #[test]
    fn an_unclassified_store_withdraws_the_state() {
        let owned = |name: &str| name == "op_new0";
        let fresh = StateStore::Fresh {
            function: "make".into(),
            callees: BTreeSet::from(["op_new0".to_owned()]),
        };
        let unit = facts(&[("op.cb", fresh.clone())]);
        assert!(stability(&[Some(&unit)], "op.cb", false, &owned).is_ok());
        let popped = facts(&[(
            "op.cb",
            StateStore::Fresh {
                function: "take".into(),
                callees: BTreeSet::from(["pop".to_owned()]),
            },
        )]);
        assert!(stability(&[Some(&unit), Some(&popped)], "op.cb", false, &owned).is_err());
        let other = facts(&[(
            "op.cb",
            StateStore::Other {
                function: "f".into(),
                why: "a store into an existing object".into(),
            },
        )]);
        assert!(stability(&[Some(&unit), Some(&other)], "op.cb", false, &owned).is_err());
        assert!(stability(&[Some(&unit), None], "op.cb", false, &owned).is_err());
        let null = facts(&[(
            "op.cb",
            StateStore::Null {
                function: "call".into(),
            },
        )]);
        assert!(stability(&[Some(&null)], "op.cb", false, &owned).is_ok());
        let mut whole = StateFacts::default();
        whole
            .whole_writes
            .insert("op".into(), "a `memset` into it".into());
        assert!(stability(&[Some(&whole)], "op.cb", false, &owned).is_err());
        assert!(stores_into(&unit, "op.cb", "make"));
        assert!(!stores_into(&unit, "op.cb", "other"));
    }

    fn ast(text: &str) -> Value {
        serde_json::from_str(text).expect("json")
    }

    fn decl_ref(id: &str, var: &str) -> String {
        format!(
            r#"{{"kind":"ImplicitCastExpr","inner":[{{"kind":"DeclRefExpr","referencedDecl":{{"id":"{id}","kind":"{var}","name":"x"}}}}]}}"#
        )
    }

    fn arrow(base: &str, name: &str) -> String {
        format!(r#"{{"kind":"MemberExpr","name":"{name}","isArrow":true,"inner":[{base}]}}"#)
    }

    fn assign(left: &str, right: &str) -> String {
        format!(r#"{{"kind":"BinaryOperator","opcode":"=","inner":[{left},{right}]}}"#)
    }

    fn call(callee: &str) -> String {
        format!(
            r#"{{"kind":"CallExpr","inner":[{{"kind":"ImplicitCastExpr","inner":[{{"kind":"DeclRefExpr","referencedDecl":{{"kind":"FunctionDecl","name":"{callee}"}}}}]}}]}}"#
        )
    }

    fn constructor(statements: &[String]) -> Value {
        ast(&format!(
            r#"{{"kind":"FunctionDecl","name":"op_new0","inner":[
              {{"kind":"ParmVarDecl","id":"p0"}},{{"kind":"ParmVarDecl","id":"p1"}},
              {{"kind":"CompoundStmt","inner":[{}]}}]}}"#,
            statements.join(",")
        ))
    }

    fn state() -> ConstructedState {
        ConstructedState {
            live: true,
            discriminant: Some(DiscriminantSet {
                member: CallbackMember {
                    tag: "op".into(),
                    path: "type".into(),
                },
                parameter: 1,
                flags: 0,
            }),
            members: vec![MemberState {
                member: CallbackMember {
                    tag: "op".into(),
                    path: "cb".into(),
                },
                value: MemberValue::Absent,
            }],
        }
    }

    #[test]
    fn the_constructor_body_validates_the_absent_state() {
        let local = r#"{"kind":"DeclStmt","inner":[{"kind":"VarDecl","id":"v","type":{"qualType":"op_t *"}}]}"#.to_owned();
        let result = decl_ref("v", "VarDecl");
        let set = assign(&result, &call("rd_calloc"));
        let store_type = assign(&arrow(&result, "type"), &decl_ref("p1", "ParmVarDecl"));
        let ret = format!(r#"{{"kind":"ReturnStmt","inner":[{result}]}}"#);
        let good = constructor(&[local.clone(), set.clone(), store_type.clone(), ret.clone()]);
        assert_eq!(
            result_state_validates(&good, &state(), &BTreeMap::new()),
            Ok(())
        );
        // A store into the absent member withdraws it.
        let store_cb = assign(&arrow(&result, "cb"), &call("f"));
        let bad = constructor(&[
            local.clone(),
            set.clone(),
            store_type.clone(),
            store_cb,
            ret.clone(),
        ]);
        assert!(result_state_validates(&bad, &state(), &BTreeMap::new()).is_err());
        // An allocator that does not zero.
        let malloc = assign(&result, &call("rd_malloc"));
        let bad = constructor(&[local.clone(), malloc, store_type.clone(), ret.clone()]);
        assert!(result_state_validates(&bad, &state(), &BTreeMap::new()).is_err());
        // The result handed to a call.
        let handed = format!(
            r#"{{"kind":"CallExpr","inner":[{},{result}]}}"#,
            r#"{"kind":"ImplicitCastExpr","inner":[{"kind":"DeclRefExpr","referencedDecl":{"kind":"FunctionDecl","name":"g"}}]}"#
        );
        let bad = constructor(&[
            local.clone(),
            set.clone(),
            store_type.clone(),
            handed,
            ret.clone(),
        ]);
        assert!(result_state_validates(&bad, &state(), &BTreeMap::new()).is_err());
        // No store of the discriminant.
        let bad = constructor(&[local, set, ret]);
        assert!(result_state_validates(&bad, &state(), &BTreeMap::new()).is_err());
    }
}
