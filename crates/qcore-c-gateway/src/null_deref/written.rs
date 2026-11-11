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

//! Card **CELL-RESULT-SET-WRITTEN-SET-DERIVATION** (contract specification
//! C09.4, grammar G05j as amended, gateway rule 38e as amended): a function's
//! slot written set, `written_set(SLOT[K], SET) when #result == C`, derived
//! from its body at `quirin-audit pin --bounds --derive-results` time.
//!
//! USER DECISIONS 2026-11-04 (verbatim): "Basically, Q1(a) and Q2(i) But we
//! must not handle descriptor 0 as a valid handle. So, we need to exclude the
//! 0 descriptor like: written_set(pipedes, [1, MAX]); a live descriptor is in
//! [1, MAX]"; "Write [1, MAX]".
//!
//! The body is walked path by path ([`Walker`]); each path that ends at a
//! `return` yields the set of the value it returns and the set of the `int`
//! at `SLOT[K]` ([`qcore_core::written_set::PathEnd`]), and
//! [`qcore_core::written_set::combine`] joins them. Two shapes derive a set:
//!
//! * **forwarding**, `return G(.., P, ..)`: `P[K]` holds `G`'s written set of
//!   that position for the class, since `F` returns exactly what `G` does (a
//!   C library declaration's set, or an in-unit helper's derived one);
//! * **a store after the C library call's failure exit**: `ret = G(.., A,
//!   ..)` links the local `ret` to the out array `A` (or to the slot itself);
//!   a test of `ret` on the path narrows it
//!   ([`qcore_core::written_set::narrow`]), and once it is exactly one class
//!   `c` of `G`'s result set, `A[k]` holds `G`'s written set of class `c`
//!   ([`qcore_core::written_set::resolve`]); a later `*P = A[k]` (or `P[K] =
//!   A[k]`) stores it. A `return ret` whose narrowed set excludes the class
//!   constrains nothing.
//!
//! Everything else reads nothing, fail-closed: a value the walk does not know
//! is its type's whole range; a call, or a store through another pointer of
//! another type, after the slot was written leaves it unknown; an `int` store
//! through any other pointer joins its value (it may alias the slot); a local
//! whose address is taken is never narrowed; an out array is read only when
//! every call it is handed to is a C library declaration stating a written
//! set of that position (which, under `libc-c-standard-abi`, writes it and
//! keeps nothing) and its every other mention is a read `A[k]`; a loop,
//! `switch`, `goto` or a statement shape the walk does not know refuses the
//! whole derivation. Only the C library's flexible declarations are read;
//! in-unit helpers are read from their bodies (their sites join the pin's
//! identity).

use std::collections::{BTreeMap, BTreeSet};

use qcore_core::result_set::{Cmp, ResultSet};
use qcore_core::written_set::{self, PathEnd, Resolution};
use serde_json::Value;

use super::ast::{self, kids, ty};
use super::range::{Deriver, RangeDecl};
use super::witness::int_range;
use crate::typed_c_ast::kind;

/// A declared written set's key: (function, parameter position, element,
/// class).
pub(super) type WrittenKey = (String, usize, u32, i128);

/// The paths one walk may hold at once; more refuses.
const MAX_PATHS: usize = 256;

/// A derived written set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrittenDerived {
    pub set: ResultSet,
    /// The in-unit helpers whose bodies the derivation read.
    pub helpers: BTreeSet<String>,
    /// The C library declarations whose result or written sets it read.
    pub rests_on: BTreeSet<String>,
}

/// The C library's declared written sets in force here, by key: read only
/// beside the declaration's result set in force (the same binding: not held,
/// not unbound by an in-unit definition of the name).
pub(super) fn declared_written(
    contracts: &qcore_contract::bounds_contract::BoundsContracts,
    declared: &BTreeMap<String, RangeDecl>,
) -> BTreeMap<WrittenKey, ResultSet> {
    use qcore_contract::bounds_contract::BoundsClause;
    let mut out = BTreeMap::new();
    for (name, contract) in contracts.null_declarations() {
        if !contract.flexible || !declared.get(name).is_some_and(RangeDecl::in_force) {
            continue;
        }
        for entry in &contract.clauses {
            if let BoundsClause::WrittenSet {
                parameter,
                index,
                set,
                class,
            } = &entry.clause
                && let Some(position) = contract
                    .parameters
                    .iter()
                    .position(|p| &p.name == parameter)
            {
                out.insert((name.clone(), position, *index, *class), set.clone());
            }
        }
    }
    out
}

/// Derives written sets from bodies (see the module docs).
pub(super) struct WrittenDeriver<'a> {
    unit: &'a ast::Unit,
    declared: &'a BTreeMap<String, RangeDecl>,
    written: &'a BTreeMap<WrittenKey, ResultSet>,
    stack: Vec<WrittenKey>,
    helpers: BTreeSet<String>,
    rests_on: BTreeSet<String>,
}

impl<'a> WrittenDeriver<'a> {
    pub(super) fn new(
        unit: &'a ast::Unit,
        declared: &'a BTreeMap<String, RangeDecl>,
        written: &'a BTreeMap<WrittenKey, ResultSet>,
    ) -> Self {
        Self {
            unit,
            declared,
            written,
            stack: Vec::new(),
            helpers: BTreeSet::new(),
            rests_on: BTreeSet::new(),
        }
    }

    /// Derive `function`'s written set of `parameter[element]` when it
    /// returns `class`.
    ///
    /// # Errors
    ///
    /// Why nothing is derived.
    pub(super) fn derive(
        mut self,
        function: &str,
        parameter: usize,
        element: u32,
        class: i128,
    ) -> Result<WrittenDerived, String> {
        if !self.unit.definitions.contains_key(function) {
            return Err(format!("the unit defines no `{function}`"));
        }
        let set = self.slot_set(function, parameter, element, class)?;
        Ok(WrittenDerived {
            set,
            helpers: self.helpers,
            rests_on: self.rests_on,
        })
    }

    /// Whether a C library declaration states a written set of `position`.
    fn states_written(&self, callee: &str, position: usize) -> bool {
        !self.unit.definitions.contains_key(callee)
            && self
                .written
                .keys()
                .any(|(name, at, _, _)| name == callee && *at == position)
    }

    /// The written set of one function's slot: a C library declaration's,
    /// or one derived from an in-unit body.
    fn slot_set(
        &mut self,
        function: &str,
        parameter: usize,
        element: u32,
        class: i128,
    ) -> Result<ResultSet, String> {
        let key = (function.to_owned(), parameter, element, class);
        if self.unit.definitions.contains_key(function) {
            if self.stack.contains(&key) {
                return Err(format!("`{function}` is reached recursively"));
            }
            self.stack.push(key);
            let derived = self.body(function, parameter, element, class);
            self.stack.pop();
            if derived.is_ok() && !self.stack.is_empty() {
                self.helpers.insert(function.to_owned());
            }
            return derived;
        }
        match self.written.get(&key) {
            Some(set) => {
                self.rests_on.insert(function.to_owned());
                Ok(set.clone())
            }
            None => Err(format!(
                "`{function}` states no written set of its argument {parameter}[{element}] when it returns {class}, and the unit does not define it"
            )),
        }
    }

    /// The result set of a call to `callee` (a C library set in force, an
    /// in-unit helper's derived set), or `None`.
    fn result_of(&mut self, callee: &str, call: &Value) -> Option<ResultSet> {
        if self.unit.definitions.contains_key(callee) {
            let derived = Deriver::new(self.unit, self.declared).derive(callee)?;
            self.helpers.insert(callee.to_owned());
            self.helpers.extend(derived.helpers);
            self.rests_on.extend(derived.rests_on);
            return Some(derived.set);
        }
        match self.declared.get(callee) {
            Some(decl) if decl.in_force() && int_range(ty(call)) == Some(decl.set.universe()) => {
                self.rests_on.insert(callee.to_owned());
                Some(decl.set.clone())
            }
            _ => None,
        }
    }

    fn body(
        &mut self,
        function: &str,
        parameter: usize,
        element: u32,
        class: i128,
    ) -> Result<ResultSet, String> {
        let unit = self.unit;
        let definition = unit
            .definitions
            .get(function)
            .ok_or_else(|| format!("the unit defines no `{function}`"))?;
        let result_universe = int_range(ty(definition).split('(').next().unwrap_or("").trim())
            .ok_or_else(|| format!("`{function}` has no integer result"))?;
        let parameters = kids(definition)
            .filter(|c| kind(c) == "ParmVarDecl")
            .collect::<Vec<_>>();
        let slot = parameters
            .get(parameter)
            .ok_or_else(|| format!("`{function}` has no parameter {parameter}"))?;
        if pointee_universe(ty(slot)) != Some(written_set::INT) {
            return Err(format!(
                "`{function}`'s parameter {parameter} is no `int *`"
            ));
        }
        let slot_id = slot["id"].as_str().unwrap_or("").to_owned();
        let body = kids(definition)
            .find(|c| kind(c) == "CompoundStmt")
            .ok_or_else(|| format!("`{function}` has no body"))?;
        let mut taken = BTreeSet::new();
        ast::address_taken(body, &mut taken);
        let mut automatic = parameters
            .iter()
            .filter_map(|p| p["id"].as_str().map(str::to_owned))
            .collect::<BTreeSet<_>>();
        automatic_locals(body, &mut automatic);
        let arrays = out_arrays(body, |callee, position| {
            self.states_written(callee, position)
        });
        let mut start = Path::default();
        for parameter in &parameters {
            if let (Some(id), Some(universe)) = (parameter["id"].as_str(), int_range(ty(parameter)))
                && id != slot_id
                && !taken.contains(id)
            {
                start.vals.insert(id.to_owned(), ResultSet::full(universe));
            }
        }
        let mut walker = Walker {
            deriver: self,
            slot_id,
            element,
            class,
            result_universe,
            taken,
            automatic,
            arrays,
            ends: Vec::new(),
        };
        let fallthrough = walker.statement(body, vec![start])?;
        if !fallthrough.is_empty() {
            return Err(format!(
                "a path of `{function}` reaches its end without a `return`"
            ));
        }
        let ends = std::mem::take(&mut walker.ends);
        written_set::combine(written_set::INT, class, &ends).map_err(|refusal| {
            format!(
                "`{function}`: {} (argument {parameter}[{element}] when it returns {class})",
                refusal.code()
            )
        })
    }
}

/// The `int` range a pointer spelling points to.
fn pointee_universe(spelled: &str) -> Option<(i128, i128)> {
    int_range(spelled.trim().strip_suffix('*')?.trim())
}

/// Every automatic (non-`static`, non-`extern`) local a body declares.
fn automatic_locals(node: &Value, out: &mut BTreeSet<String>) {
    if kind(node) == "VarDecl"
        && !matches!(node["storageClass"].as_str(), Some("static" | "extern"))
        && let Some(id) = node["id"].as_str()
    {
        out.insert(id.to_owned());
    }
    for child in kids(node) {
        automatic_locals(child, out);
    }
}

/// Where a call's out argument wrote.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Target {
    /// The slot parameter itself.
    Slot,
    /// A local out array.
    Array(String),
}

/// A local linked to the call whose result it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Link {
    callee: String,
    results: ResultSet,
    /// The out arguments: (callee position, what it is).
    targets: Vec<(usize, Target)>,
}

/// One path's state.
#[derive(Clone, Debug, Default)]
struct Path {
    /// Narrowable `int` locals (address never taken), by declaration id.
    vals: BTreeMap<String, ResultSet>,
    links: BTreeMap<String, Link>,
    /// Known elements of local out arrays.
    known: BTreeMap<(String, u32), ResultSet>,
    /// The slot's set as the path left it.
    slot: Option<ResultSet>,
}

impl Path {
    /// A call or an unknown write may have changed the slot.
    fn clobber_slot(&mut self) {
        self.slot = None;
        for link in self.links.values_mut() {
            link.targets.retain(|(_, target)| *target != Target::Slot);
        }
    }

    /// A call wrote `array`.
    fn clobber_array(&mut self, array: &str) {
        self.known.retain(|(id, _), _| id != array);
        for link in self.links.values_mut() {
            link.targets
                .retain(|(_, target)| !matches!(target, Target::Array(id) if id == array));
        }
    }

    fn forget(&mut self, local: &str) {
        self.vals.remove(local);
        self.links.remove(local);
    }
}

/// Parentheses and conversions that keep the designated object.
fn strip(node: &Value) -> &Value {
    let mut node = node;
    loop {
        match kind(node) {
            "ParenExpr" => {}
            "ImplicitCastExpr" | "CStyleCastExpr"
                if matches!(node["castKind"].as_str(), Some("LValueToRValue" | "NoOp")) => {}
            _ => return node,
        }
        match kids(node).next() {
            Some(inner) => node = inner,
            None => return node,
        }
    }
}

/// Parentheses and value-preserving conversions.
fn preserving(node: &Value) -> &Value {
    let mut node = node;
    loop {
        match kind(node) {
            "ParenExpr" | "ConstantExpr" | "FullExpr" | "ExprWithCleanups" => {}
            "ImplicitCastExpr" | "CStyleCastExpr"
                if matches!(node["castKind"].as_str(), Some("LValueToRValue" | "NoOp")) => {}
            "ImplicitCastExpr" | "CStyleCastExpr" if node["castKind"] == "IntegralCast" => {
                let Some(inner) = kids(node).next() else {
                    return node;
                };
                match (int_range(ty(inner)), int_range(ty(node))) {
                    (Some((lo, hi)), Some((tlo, thi))) if tlo <= lo && hi <= thi => {}
                    _ => return node,
                }
            }
            _ => return node,
        }
        match kids(node).next() {
            Some(inner) => node = inner,
            None => return node,
        }
    }
}

/// A local array an argument decays from.
fn decayed_array(argument: &Value) -> Option<&str> {
    if kind(argument) != "ImplicitCastExpr" || argument["castKind"] != "ArrayToPointerDecay" {
        return None;
    }
    let array = strip(kids(argument).next()?);
    (kind(array) == "DeclRefExpr" && array["referencedDecl"]["kind"] == "VarDecl")
        .then(|| array["referencedDecl"]["id"].as_str())
        .flatten()
}

/// `A[k]`'s local array id and constant element.
fn constant_element(subscript: &Value) -> Option<(String, u32)> {
    let mut parts = kids(subscript);
    let array = decayed_array(parts.next()?)?;
    let index = parts.next()?;
    let k = ast::constant_value(index, &BTreeMap::new())?;
    Some((array.to_owned(), u32::try_from(k).ok()?))
}

/// Local `int` arrays (by id, with their length) whose every mention is a
/// read `A[k]` at a constant or an argument of a call to a C library
/// declaration that states a written set of that position (`states`).
fn out_arrays(body: &Value, states: impl Fn(&str, usize) -> bool) -> BTreeMap<String, u32> {
    fn scan(
        node: &Value,
        states: &dyn Fn(&str, usize) -> bool,
        bad: &mut BTreeSet<String>,
        decls: &mut BTreeMap<String, u32>,
    ) {
        match kind(node) {
            "VarDecl" => {
                let spelled = ty(node);
                if let (Some(id), Some(open), Some(close)) =
                    (node["id"].as_str(), spelled.find('['), spelled.find(']'))
                    && int_range(spelled[..open].trim()) == Some(written_set::INT)
                    && !matches!(node["storageClass"].as_str(), Some("static" | "extern"))
                    && kids(node).all(|c| kind(c).ends_with("Attr"))
                    && let Ok(length) = spelled[open + 1..close].trim().parse::<u32>()
                {
                    decls.insert(id.to_owned(), length);
                }
            }
            "CallExpr" => {
                let callee = ast::callee_name(node);
                for (position, argument) in kids(node).skip(1).enumerate() {
                    if decayed_array(argument).is_some()
                        && callee
                            .as_deref()
                            .is_some_and(|callee| states(callee, position))
                    {
                        continue;
                    }
                    scan(argument, states, bad, decls);
                }
                if let Some(callee) = kids(node).next() {
                    scan(callee, states, bad, decls);
                }
                return;
            }
            "ImplicitCastExpr" if node["castKind"] == "LValueToRValue" => {
                if let Some(inner) = kids(node).next()
                    && kind(inner) == "ArraySubscriptExpr"
                    && constant_element(inner).is_some()
                {
                    return;
                }
            }
            "DeclRefExpr" => {
                if let Some(id) = node["referencedDecl"]["id"].as_str() {
                    bad.insert(id.to_owned());
                }
            }
            _ => {}
        }
        for child in kids(node) {
            scan(child, states, bad, decls);
        }
    }
    let (mut bad, mut decls) = (BTreeSet::new(), BTreeMap::new());
    scan(body, &states, &mut bad, &mut decls);
    decls.retain(|id, _| !bad.contains(id));
    decls
}

/// What a store's left side designates.
enum Place {
    /// A narrowable local.
    Local(String),
    /// `P[k]` (`*P` is `P[0]`); `None` for an index the walk does not know.
    Slot(Option<u32>),
    /// An automatic local whose address is never taken and that is not
    /// narrowed (a pointer, an aggregate): no memory a pointer reaches.
    Plain,
    /// Anything else: memory that may alias the slot.
    Memory,
}

/// The walk over one body.
struct Walker<'w, 'a> {
    deriver: &'w mut WrittenDeriver<'a>,
    slot_id: String,
    element: u32,
    class: i128,
    result_universe: (i128, i128),
    taken: BTreeSet<String>,
    automatic: BTreeSet<String>,
    arrays: BTreeMap<String, u32>,
    ends: Vec<PathEnd>,
}

impl Walker<'_, '_> {
    fn is_slot(&self, node: &Value) -> bool {
        let node = strip(node);
        kind(node) == "DeclRefExpr" && node["referencedDecl"]["id"].as_str() == Some(&self.slot_id)
    }

    /// An automatic `int` local (or parameter) whose address is never taken.
    fn narrowable(&self, decl_ref: &Value) -> bool {
        let Some(id) = decl_ref["referencedDecl"]["id"].as_str() else {
            return false;
        };
        kind(decl_ref) == "DeclRefExpr"
            && id != self.slot_id
            && self.automatic.contains(id)
            && !self.taken.contains(id)
            && int_range(decl_ref["referencedDecl"]["t"].as_str().unwrap_or("")).is_some()
            && !decl_ref["referencedDecl"]["t"]
                .as_str()
                .unwrap_or("")
                .contains("volatile")
    }

    fn place(&self, lhs: &Value) -> Place {
        let node = strip(lhs);
        match kind(node) {
            "DeclRefExpr" => {
                let id = node["referencedDecl"]["id"].as_str().unwrap_or("");
                if self.narrowable(node) {
                    Place::Local(id.to_owned())
                } else if self.automatic.contains(id)
                    && !self.taken.contains(id)
                    && id != self.slot_id
                {
                    Place::Plain
                } else {
                    Place::Memory
                }
            }
            "UnaryOperator" if node["opcode"] == "*" => match kids(node).next() {
                Some(operand) if self.is_slot(operand) => Place::Slot(Some(0)),
                _ => Place::Memory,
            },
            "ArraySubscriptExpr" => {
                let mut parts = kids(node);
                match (parts.next(), parts.next()) {
                    (Some(base), Some(index)) if self.is_slot(base) => Place::Slot(
                        ast::constant_value(index, &self.deriver.unit.enum_values)
                            .and_then(|k| u32::try_from(k).ok()),
                    ),
                    _ => Place::Memory,
                }
            }
            _ => Place::Memory,
        }
    }

    /// The set of an integer expression on a path (its effects applied
    /// before).
    fn value(&mut self, node: &Value, path: &Path) -> ResultSet {
        let universe = int_range(ty(node)).unwrap_or(written_set::INT);
        let full = ResultSet::full(universe);
        let clip = |set: &ResultSet| ResultSet::from_intervals(universe, set.intervals());
        if let Some(k) = ast::constant_value(node, &self.deriver.unit.enum_values) {
            return if universe.0 <= k && k <= universe.1 {
                ResultSet::interval(universe, k, k)
            } else {
                full
            };
        }
        let inner = preserving(node);
        match kind(inner) {
            "DeclRefExpr" if self.narrowable(inner) => inner["referencedDecl"]["id"]
                .as_str()
                .and_then(|id| path.vals.get(id))
                .map_or(full.clone(), clip),
            "ArraySubscriptExpr" => constant_element(inner)
                .filter(|(array, _)| self.arrays.contains_key(array))
                .and_then(|key| path.known.get(&key))
                .map_or(full.clone(), clip),
            "CallExpr" => ast::callee_name(inner)
                .and_then(|callee| self.deriver.result_of(&callee, inner))
                .map_or(full.clone(), |set| clip(&set)),
            _ => full,
        }
    }

    /// Apply every effect of an expression: calls, stores and writes inside
    /// it, in evaluation order where C fixes one.
    fn effects(&mut self, node: &Value, paths: &mut [Path]) -> Result<(), String> {
        match kind(node) {
            "CallExpr" => {
                for child in kids(node).skip(1) {
                    self.effects(child, paths)?;
                }
                for path in paths.iter_mut() {
                    call_effect(node, path);
                }
                return Ok(());
            }
            "BinaryOperator" if node["opcode"] == "=" => {
                let mut parts = kids(node);
                let (Some(lhs), Some(rhs)) = (parts.next(), parts.next()) else {
                    return Err("an assignment without operands".into());
                };
                self.effects(rhs, paths)?;
                self.effects_of_place(lhs, paths)?;
                for path in paths.iter_mut() {
                    self.assign(lhs, rhs, path);
                }
                return Ok(());
            }
            "CompoundAssignOperator" => {
                let mut parts = kids(node);
                if let (Some(lhs), Some(rhs)) = (parts.next(), parts.next()) {
                    self.effects(rhs, paths)?;
                    self.effects_of_place(lhs, paths)?;
                    for path in paths.iter_mut() {
                        self.unknown_write(lhs, path);
                    }
                }
                return Ok(());
            }
            "UnaryOperator" if matches!(node["opcode"].as_str(), Some("++" | "--")) => {
                if let Some(operand) = kids(node).next() {
                    self.effects_of_place(operand, paths)?;
                    for path in paths.iter_mut() {
                        self.unknown_write(operand, path);
                    }
                }
                return Ok(());
            }
            "StmtExpr" | "BinaryConditionalOperator" | "ConditionalOperator" => {
                return Err(format!("a `{}` expression", kind(node)));
            }
            "BinaryOperator" if matches!(node["opcode"].as_str(), Some("&&" | "||")) => {
                // The right operand may not run: its effects are refused
                // rather than guessed.
                let mut parts = kids(node);
                if let Some(left) = parts.next() {
                    self.effects(left, paths)?;
                }
                if parts.next().is_some_and(has_effect) {
                    return Err("an effect under `&&` or `||`".into());
                }
                return Ok(());
            }
            _ => {}
        }
        for child in kids(node) {
            self.effects(child, paths)?;
        }
        Ok(())
    }

    /// The effects inside a store's left side (its index, its pointer).
    fn effects_of_place(&mut self, lhs: &Value, paths: &mut [Path]) -> Result<(), String> {
        let node = strip(lhs);
        match kind(node) {
            "DeclRefExpr" => Ok(()),
            _ => {
                for child in kids(node) {
                    self.effects(child, paths)?;
                }
                Ok(())
            }
        }
    }

    /// The out arguments of a call: the slot itself, or a local out array.
    fn targets(&self, call: &Value) -> Vec<(usize, Target)> {
        let mut out = Vec::new();
        for (position, argument) in kids(call).skip(1).enumerate() {
            if self.is_slot(argument) {
                out.push((position, Target::Slot));
            } else if let Some(id) = decayed_array(argument)
                && self.arrays.contains_key(id)
            {
                out.push((position, Target::Array(id.to_owned())));
            }
        }
        out
    }

    /// `local = rhs` (its effects applied): its set, and the link to the
    /// call whose result it now holds.
    fn bind(&mut self, local: &str, rhs: &Value, path: &mut Path) {
        let value = self.value(rhs, path);
        path.forget(local);
        path.vals.insert(local.to_owned(), value);
        let call = preserving(rhs);
        if kind(call) == "CallExpr"
            && let Some(callee) = ast::callee_name(call)
        {
            let targets = self.targets(call);
            if !targets.is_empty()
                && let Some(results) = self.deriver.result_of(&callee, call)
            {
                path.links.insert(
                    local.to_owned(),
                    Link {
                        callee,
                        results,
                        targets,
                    },
                );
            }
        }
    }

    fn assign(&mut self, lhs: &Value, rhs: &Value, path: &mut Path) {
        match self.place(lhs) {
            Place::Local(id) => self.bind(&id, rhs, path),
            Place::Slot(Some(k)) if k == self.element => {
                let value = self.value(rhs, path);
                path.clobber_slot();
                path.slot = Some(ResultSet::from_intervals(
                    written_set::INT,
                    value.intervals(),
                ));
            }
            Place::Slot(Some(_)) | Place::Plain => {}
            Place::Slot(None) | Place::Memory => {
                let slot = if int_range(ty(lhs)).is_some() {
                    let value = self.value(rhs, path);
                    written_set::after_store(path.slot.as_ref(), &value)
                } else {
                    None
                };
                path.clobber_slot();
                path.slot = slot;
            }
        }
    }

    fn unknown_write(&self, lhs: &Value, path: &mut Path) {
        match self.place(lhs) {
            Place::Local(id) => path.forget(&id),
            Place::Plain => {}
            Place::Slot(Some(k)) if k != self.element => {}
            Place::Slot(_) | Place::Memory => path.clobber_slot(),
        }
    }

    /// Narrow on one edge of `cond`; `None` when the edge cannot occur.
    fn narrow_edge(&mut self, cond: &Value, true_edge: bool, mut path: Path) -> Option<Path> {
        let Some((local, cmp, k)) = self.test_of(cond) else {
            return Some(path);
        };
        let Some(current) = path.vals.get(&local).cloned() else {
            return Some(path);
        };
        let narrowed = written_set::narrow(&current, cmp, k, true_edge);
        if narrowed.is_empty() {
            return None;
        }
        if let Some(link) = path.links.get(&local).cloned() {
            for (position, target) in &link.targets {
                let elements: Vec<u32> = match target {
                    Target::Slot => vec![self.element],
                    Target::Array(array) => (0..self.arrays[array]).collect(),
                };
                for element in elements {
                    let resolution = written_set::resolve(&narrowed, &link.results, |c| {
                        self.deriver
                            .slot_set(&link.callee, *position, element, c)
                            .ok()
                    });
                    match (resolution, target) {
                        (Resolution::Infeasible, _) => return None,
                        (Resolution::Known(set), Target::Slot) => path.slot = Some(set),
                        (Resolution::Known(set), Target::Array(array)) => {
                            path.known.insert((array.clone(), element), set);
                        }
                        (Resolution::Unknown, _) => {}
                    }
                }
            }
        }
        path.vals.insert(local, narrowed);
        Some(path)
    }

    /// `L OP k`, `k OP L`, `L` (`!= 0`) or `!L` (`== 0`) over a narrowable
    /// local `L` (or over `(L = e)`).
    fn test_of(&self, cond: &Value) -> Option<(String, Cmp, i128)> {
        let node = preserving(cond);
        let enums = &self.deriver.unit.enum_values;
        match kind(node) {
            "BinaryOperator" => {
                let cmp = match node["opcode"].as_str()? {
                    "==" => Cmp::Eq,
                    "!=" => Cmp::Ne,
                    "<" => Cmp::Lt,
                    "<=" => Cmp::Le,
                    ">" => Cmp::Gt,
                    ">=" => Cmp::Ge,
                    _ => return None,
                };
                let mut parts = kids(node);
                let (a, b) = (parts.next()?, parts.next()?);
                if let Some(k) = ast::constant_value(b, enums) {
                    Some((self.narrowed_local(a)?, cmp, k))
                } else {
                    let k = ast::constant_value(a, enums)?;
                    Some((self.narrowed_local(b)?, cmp.mirrored(), k))
                }
            }
            "UnaryOperator" if node["opcode"] == "!" => {
                Some((self.narrowed_local(kids(node).next()?)?, Cmp::Eq, 0))
            }
            "DeclRefExpr" | "ImplicitCastExpr" => Some((self.narrowed_local(node)?, Cmp::Ne, 0)),
            _ => None,
        }
    }

    /// The local a test reads: `L`, or the left side of `(L = e)`, its type
    /// holding every value of the test's own operand.
    fn narrowed_local(&self, node: &Value) -> Option<String> {
        let node = preserving(node);
        let node = if kind(node) == "BinaryOperator" && node["opcode"] == "=" {
            strip(kids(node).next()?)
        } else {
            node
        };
        (kind(node) == "DeclRefExpr" && self.narrowable(node))
            .then(|| node["referencedDecl"]["id"].as_str().map(str::to_owned))
            .flatten()
    }

    fn statement(&mut self, node: &Value, paths: Vec<Path>) -> Result<Vec<Path>, String> {
        if paths.len() > MAX_PATHS {
            return Err(format!("more than {MAX_PATHS} paths"));
        }
        if paths.is_empty() {
            return Ok(paths);
        }
        let mut paths = paths;
        match kind(node) {
            "CompoundStmt" => {
                for child in kids(node) {
                    paths = self.statement(child, paths)?;
                }
                Ok(paths)
            }
            "NullStmt" => Ok(paths),
            "DeclStmt" => {
                for decl in kids(node) {
                    if kind(decl) != "VarDecl" {
                        continue;
                    }
                    let id = decl["id"].as_str().unwrap_or("").to_owned();
                    let Some(init) = kids(decl).find(|c| !kind(c).ends_with("Attr")) else {
                        // A declaration without an initializer: its value is
                        // not known (C11 6.7.9p10).
                        for path in &mut paths {
                            path.forget(&id);
                        }
                        continue;
                    };
                    self.effects(init, &mut paths)?;
                    let narrowable = self.automatic.contains(&id)
                        && !self.taken.contains(&id)
                        && int_range(ty(decl)).is_some()
                        && !ty(decl).contains("volatile");
                    for path in &mut paths {
                        if narrowable {
                            self.bind(&id, init, path);
                        } else {
                            path.forget(&id);
                        }
                    }
                }
                Ok(paths)
            }
            "IfStmt" => {
                let mut parts = kids(node);
                let cond = parts.next().ok_or("an `if` without a condition")?;
                let then = parts.next().ok_or("an `if` without a branch")?;
                let otherwise = parts.next();
                self.effects(cond, &mut paths)?;
                let (mut on_true, mut on_false) = (Vec::new(), Vec::new());
                for path in paths {
                    if let Some(p) = self.narrow_edge(cond, true, path.clone()) {
                        on_true.push(p);
                    }
                    if let Some(p) = self.narrow_edge(cond, false, path) {
                        on_false.push(p);
                    }
                }
                let mut out = self.statement(then, on_true)?;
                match otherwise {
                    Some(other) => out.extend(self.statement(other, on_false)?),
                    None => out.extend(on_false),
                }
                Ok(out)
            }
            "DoStmt" => {
                let mut parts = kids(node);
                let body = parts.next().ok_or("a `do` without a body")?;
                let cond = parts.next().ok_or("a `do` without a condition")?;
                if ast::constant_value(cond, &self.deriver.unit.enum_values) != Some(0)
                    || jumps(body)
                {
                    return Err("a loop".into());
                }
                self.statement(body, paths)
            }
            "ReturnStmt" => {
                let Some(value) = kids(node).next() else {
                    return Err("a `return` without a value".into());
                };
                self.return_statement(value, paths)?;
                Ok(Vec::new())
            }
            "ForStmt" | "WhileStmt" | "SwitchStmt" | "GotoStmt" | "LabelStmt" | "BreakStmt"
            | "ContinueStmt" | "IndirectGotoStmt" | "AttributedStmt" => {
                Err(format!("a `{}`", kind(node)))
            }
            other if other.ends_with("Stmt") => Err(format!("a `{other}` the walk does not read")),
            _ => {
                // An expression statement.
                self.effects(node, &mut paths)?;
                Ok(paths)
            }
        }
    }

    fn return_statement(&mut self, value: &Value, mut paths: Vec<Path>) -> Result<(), String> {
        let call = preserving(value);
        let forwarded = if kind(call) == "CallExpr" {
            ast::callee_name(call).and_then(|callee| {
                kids(call)
                    .skip(1)
                    .position(|a| self.is_slot(a))
                    .map(|position| (callee, position))
            })
        } else {
            None
        };
        if let Some((callee, position)) = forwarded {
            // Forwarding: `F` returns exactly `G`'s value, and `G`'s written
            // set of the slot's position is the slot's.
            for argument in kids(call).skip(1) {
                self.effects(argument, &mut paths)?;
            }
            let returns = self
                .deriver
                .result_of(&callee, call)
                .unwrap_or_else(|| ResultSet::full(self.result_universe));
            let slot = if returns.contains(self.class) {
                Some(
                    self.deriver
                        .slot_set(&callee, position, self.element, self.class)?,
                )
            } else {
                None
            };
            let returns = ResultSet::from_intervals(self.result_universe, returns.intervals());
            for _ in &paths {
                self.ends.push(PathEnd {
                    returns: returns.clone(),
                    slot: slot.clone(),
                });
            }
            return Ok(());
        }
        self.effects(value, &mut paths)?;
        for path in &paths {
            let returns = self.value(value, path);
            self.ends.push(PathEnd {
                returns: ResultSet::from_intervals(self.result_universe, returns.intervals()),
                slot: path.slot.clone(),
            });
        }
        Ok(())
    }
}

/// A call may write any memory: the slot is unknown, and so are the local
/// out arrays it is handed until its result is narrowed.
fn call_effect(call: &Value, path: &mut Path) {
    path.clobber_slot();
    for argument in kids(call).skip(1) {
        if let Some(id) = decayed_array(argument) {
            path.clobber_array(id);
        }
    }
}

/// Whether an expression calls or writes.
fn has_effect(node: &Value) -> bool {
    matches!(
        kind(node),
        "CallExpr" | "CompoundAssignOperator" | "StmtExpr"
    ) || (kind(node) == "BinaryOperator" && node["opcode"] == "=")
        || (kind(node) == "UnaryOperator" && matches!(node["opcode"].as_str(), Some("++" | "--")))
        || kids(node).any(has_effect)
}

/// Whether a `do { } while (0)` body jumps (`break`, `continue`, `goto`).
fn jumps(node: &Value) -> bool {
    matches!(
        kind(node),
        "BreakStmt" | "ContinueStmt" | "GotoStmt" | "IndirectGotoStmt"
    ) || kids(node).any(jumps)
}
