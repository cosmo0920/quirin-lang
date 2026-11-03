//! Card **NULL-TRIAGE-UNRESOLVED-PATHS** (gateway rule 15q): integer result
//! ranges for the null path search.
//!
//! A call's integer result is never a free value of the search
//! ([`super::witness`]): its range is a **bound** that may decide a
//! comparison. The bound is the C type's range, narrowed by
//!
//! * a declared `ensures normal nonnull_result_range(LO, HI);` (bounds contract
//!   v1, null property only) that is in force: not held, and not refuted by
//!   this unit's own body of the function ([`declared_ranges`],
//!   [`evidence`]);
//! * this unit's **validated summary** of the callee's body ([`summary`]):
//!   every `return` is a constant or a local counter that starts at
//!   constants and only grows by non-negative constants
//!   ([`qcore_core::null_paths::counter_lower_bound`]; a defined execution
//!   never overflows a signed counter).
//!
//! The body check of a declaration is rule 13g's fold over the analysed
//! bodies ([`qcore_core::null_triage::result_standing`]): a body whose
//! summary lies inside the declared range validates it, a body that returns a
//! constant outside it refutes it (withdrawn for the run), any other body
//! leaves it a declared assumption.
//!
//! Card **CELL-GUARD-RESULT-RANGE-CHECKED** (contract specification C09.4,
//! gateway rule 38e) widens what a summary reads, never what it assumes: a
//! body's result set is the union of its `return`s' sets ([`Deriver`]),
//! where a constant is itself, a call to an in-unit helper is that helper's
//! derived set, a call to a function with a declared set **in force** is that
//! set (a C library `result_set`, flexible, is in force only where the unit
//! defines no function of that name and the call's C type is the declared
//! result type; a pinned `nonnull_result_range` only while not held and not
//! refuted here), and a local whose address is never taken and whose every
//! write is a plain assignment is the union of the values assigned to it (a
//! read before the first store is undefined, C11 6.3.2.1p2). Anything else is
//! its C type's whole range. The same derivation, over the configured unit's
//! AST with the C library's sets only, is `quirin-audit pin --bounds
//! --derive-results`' ([`derive_result_set`]).

use std::collections::{BTreeMap, BTreeSet};

use qcore_contract::bounds_contract::BoundsContracts;
use qcore_core::null_triage::ResultEvidence;
use qcore_core::result_set::ResultSet;
use serde_json::Value;

use super::ast::{self, kids, ty};
use super::witness::int_range;
use crate::typed_c_ast::kind;

/// A declared integer result set (a pinned `nonnull_result_range`, read as
/// its interval, or a C library `result_set`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RangeDecl {
    pub(super) lo: i128,
    pub(super) hi: i128,
    /// The declared set (`[lo, hi]` is its hull).
    pub(super) set: ResultSet,
    /// A C library result set (`requires bounds_revision(flexible);`).
    pub(super) flexible: bool,
    /// The hold, when the declaration is not in force (a review marker, a pin
    /// not in force, or a flexible set this unit does not bind).
    pub(super) held: Option<String>,
    /// This unit's own body of the function returns a constant outside it.
    pub(super) refuted: bool,
}

impl RangeDecl {
    /// In force here: neither held nor refuted.
    pub(super) fn in_force(&self) -> bool {
        self.held.is_none() && !self.refuted
    }
}

/// The declared result sets of the bounds environment, by function. A C
/// library set is unbound (held) in a unit that defines a function of the
/// same name: that function is not the C library's.
pub(super) fn declared_ranges(
    contracts: &BoundsContracts,
    unit: &ast::Unit,
) -> BTreeMap<String, RangeDecl> {
    let mut out = BTreeMap::new();
    for (name, contract) in contracts.null_declarations() {
        let Some(set) = contract.result_values() else {
            continue;
        };
        let Some((lo, hi)) = set.hull() else {
            continue;
        };
        let mut held = contract.held().then(|| contract.holds().join("; "));
        if contract.flexible && held.is_none() && unit.definitions.contains_key(name) {
            held = Some(format!(
                "unbound: this unit defines `{name}`, which is then not the C library's"
            ));
        }
        out.insert(
            name.clone(),
            RangeDecl {
                lo,
                hi,
                set,
                flexible: contract.flexible,
                held,
                refuted: false,
            },
        );
    }
    out
}

/// Strip parentheses and conversions that keep every value: an implicit or
/// explicit integral conversion whose target range holds the source's.
fn strip_preserving(node: &Value) -> &Value {
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

fn returns<'v>(node: &'v Value, out: &mut Vec<&'v Value>) {
    if kind(node) == "ReturnStmt" {
        out.push(node);
    }
    for child in kids(node) {
        returns(child, out);
    }
}

/// The values a local counter `id` can take: `(least, greatest)`, when every
/// write to it is a constant start or a constant step and it starts
/// initialised.
fn counter(
    body: &Value,
    id: &str,
    spelled: &str,
    enums: &BTreeMap<String, i128>,
) -> Option<(i128, i128)> {
    fn names(node: &Value, id: &str) -> bool {
        let node = strip_preserving(node);
        kind(node) == "DeclRefExpr" && node["referencedDecl"]["id"].as_str() == Some(id)
    }
    fn scan(
        node: &Value,
        id: &str,
        enums: &BTreeMap<String, i128>,
        starts: &mut Vec<i128>,
        steps: &mut Vec<i128>,
        ok: &mut bool,
    ) {
        match kind(node) {
            "VarDecl" if node["id"].as_str() == Some(id) => {
                match kids(node).find(|c| !kind(c).ends_with("Attr")) {
                    Some(init) => match ast::constant_value(init, enums) {
                        Some(k) => starts.push(k),
                        None => *ok = false,
                    },
                    None => *ok = false,
                }
            }
            "BinaryOperator" if node["opcode"] == "=" => {
                let mut parts = kids(node);
                if let (Some(lhs), Some(rhs)) = (parts.next(), parts.next())
                    && names(lhs, id)
                {
                    match ast::constant_value(rhs, enums) {
                        Some(k) => starts.push(k),
                        None => *ok = false,
                    }
                }
            }
            "UnaryOperator" if matches!(node["opcode"].as_str(), Some("++" | "--")) => {
                if kids(node).next().is_some_and(|operand| names(operand, id)) {
                    steps.push(if node["opcode"] == "++" { 1 } else { -1 });
                }
            }
            "UnaryOperator" if node["opcode"] == "&" => {
                if kids(node).next().is_some_and(|operand| names(operand, id)) {
                    *ok = false;
                }
            }
            "CompoundAssignOperator" => {
                let mut parts = kids(node);
                if let (Some(lhs), Some(rhs)) = (parts.next(), parts.next())
                    && names(lhs, id)
                {
                    match (node["opcode"].as_str(), ast::constant_value(rhs, enums)) {
                        (Some("+="), Some(k)) => steps.push(k),
                        (Some("-="), Some(k)) => steps.push(-k),
                        _ => *ok = false,
                    }
                }
            }
            _ => {}
        }
        for child in kids(node) {
            scan(child, id, enums, starts, steps, ok);
        }
    }
    let (tlo, thi) = int_range(spelled)?;
    let (mut starts, mut steps, mut ok) = (Vec::new(), Vec::new(), true);
    scan(body, id, enums, &mut starts, &mut steps, &mut ok);
    if !ok || starts.iter().any(|k| *k < tlo || *k > thi) {
        return None;
    }
    // A positive step relies on the absence of overflow: only a signed
    // counter's overflow is undefined (an unsigned one wraps).
    if steps.iter().any(|k| *k > 0) && tlo >= 0 {
        return None;
    }
    let least = qcore_core::null_paths::counter_lower_bound(&starts, &steps)?;
    let greatest = if steps.iter().any(|k| *k > 0) {
        thi
    } else {
        *starts.iter().max()?
    };
    Some((least, greatest))
}

fn find_decl<'v>(node: &'v Value, id: &str, out: &mut Option<&'v Value>) {
    if kind(node) == "VarDecl" && node["id"].as_str() == Some(id) {
        *out = Some(node);
        return;
    }
    for child in kids(node) {
        find_decl(child, id, out);
    }
}

/// The derivation of one function's result set: the set, the in-unit
/// helpers whose bodies it read, and the declared sets it rested on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Derived {
    pub set: ResultSet,
    pub helpers: BTreeSet<String>,
    pub rests_on: BTreeSet<String>,
}

/// Reads result sets from bodies (see the module docs).
pub(super) struct Deriver<'a> {
    unit: &'a ast::Unit,
    declared: &'a BTreeMap<String, RangeDecl>,
    /// Functions being derived (a recursive call reads its type's range).
    stack: Vec<String>,
    /// Locals being read (a cyclic assignment reads its type's range).
    locals: BTreeSet<String>,
    helpers: BTreeSet<String>,
    rests_on: BTreeSet<String>,
}

impl<'a> Deriver<'a> {
    pub(super) fn new(unit: &'a ast::Unit, declared: &'a BTreeMap<String, RangeDecl>) -> Self {
        Self {
            unit,
            declared,
            stack: Vec::new(),
            locals: BTreeSet::new(),
            helpers: BTreeSet::new(),
            rests_on: BTreeSet::new(),
        }
    }

    /// Derive `name`'s result set; `None` for a function the unit does not
    /// define, one without an integer result or a `return` value, or one
    /// being derived already.
    pub(super) fn derive(mut self, name: &str) -> Option<Derived> {
        let set = self.function(name)?;
        Some(Derived {
            set,
            helpers: self.helpers,
            rests_on: self.rests_on,
        })
    }

    fn function(&mut self, name: &str) -> Option<ResultSet> {
        if self.stack.iter().any(|f| f == name) {
            return None;
        }
        let definition = self.unit.definitions.get(name)?;
        let body = kids(definition).find(|c| kind(c) == "CompoundStmt")?;
        let mut found = Vec::new();
        returns(body, &mut found);
        let mut taken = BTreeSet::new();
        ast::address_taken(body, &mut taken);
        self.stack.push(name.to_owned());
        let mut out: Option<ResultSet> = None;
        for ret in found {
            let Some(value) = kids(ret).next() else {
                out = None;
                break;
            };
            let Some(set) = self.expr(definition, body, value, &taken) else {
                out = None;
                break;
            };
            out = Some(match out {
                Some(acc) => acc.union(&set),
                None => set,
            });
        }
        self.stack.pop();
        out
    }

    /// The set of one integer expression; `None` when its type has no
    /// integer range.
    fn expr(
        &mut self,
        definition: &Value,
        body: &Value,
        value: &Value,
        taken: &BTreeSet<String>,
    ) -> Option<ResultSet> {
        let universe = int_range(ty(value))?;
        let full = ResultSet::full(universe);
        let inner = strip_preserving(value);
        if let Some(k) = ast::constant_value(inner, &self.unit.enum_values) {
            return Some(if universe.0 <= k && k <= universe.1 {
                ResultSet::interval(universe, k, k)
            } else {
                full
            });
        }
        let clip = |set: ResultSet| ResultSet::from_intervals(universe, set.intervals());
        match kind(inner) {
            "CallExpr" => {
                let Some(callee) = ast::callee_name(inner) else {
                    return Some(full);
                };
                if self.unit.definitions.contains_key(&callee) {
                    // An in-unit helper: its body's set.
                    if let Some(set) = self.function(&callee) {
                        self.helpers.insert(callee);
                        return Some(clip(set));
                    }
                    return Some(full);
                }
                match self.declared.get(&callee) {
                    Some(decl)
                        if decl.in_force() && int_range(ty(inner)) == Some(decl.set.universe()) =>
                    {
                        self.rests_on.insert(callee);
                        Some(clip(decl.set.clone()))
                    }
                    _ => Some(full),
                }
            }
            "DeclRefExpr" if inner["referencedDecl"]["kind"] == "VarDecl" => {
                let Some(id) = inner["referencedDecl"]["id"].as_str() else {
                    return Some(full);
                };
                if taken.contains(id) {
                    return Some(full);
                }
                let mut local = None;
                find_decl(definition, id, &mut local);
                let Some(decl) = local else {
                    return Some(full);
                };
                if matches!(decl["storageClass"].as_str(), Some("static" | "extern"))
                    || ty(decl).contains("volatile")
                {
                    return Some(full);
                }
                if let Some((lo, hi)) = counter(body, id, ty(decl), &self.unit.enum_values) {
                    return Some(clip(ResultSet::interval((lo, hi), lo, hi)));
                }
                Some(
                    self.assigned(definition, body, decl, id, taken)
                        .map_or(full, clip),
                )
            }
            _ => Some(full),
        }
    }

    /// The union of every value a local is assigned, when its every write is
    /// its initializer or a plain assignment `id = VALUE`.
    fn assigned(
        &mut self,
        definition: &Value,
        body: &Value,
        decl: &Value,
        id: &str,
        taken: &BTreeSet<String>,
    ) -> Option<ResultSet> {
        fn names(node: &Value, id: &str) -> bool {
            let node = strip_preserving(node);
            kind(node) == "DeclRefExpr" && node["referencedDecl"]["id"].as_str() == Some(id)
        }
        fn writes<'v>(node: &'v Value, id: &str, out: &mut Vec<&'v Value>, ok: &mut bool) {
            match kind(node) {
                "BinaryOperator" if node["opcode"] == "=" => {
                    let mut parts = kids(node);
                    if let (Some(lhs), Some(rhs)) = (parts.next(), parts.next())
                        && names(lhs, id)
                    {
                        out.push(rhs);
                    }
                }
                "UnaryOperator"
                    if matches!(node["opcode"].as_str(), Some("++" | "--"))
                        && kids(node).next().is_some_and(|operand| names(operand, id)) =>
                {
                    *ok = false;
                }
                "CompoundAssignOperator" if kids(node).next().is_some_and(|lhs| names(lhs, id)) => {
                    *ok = false;
                }
                _ => {}
            }
            for child in kids(node) {
                writes(child, id, out, ok);
            }
        }
        if !self.locals.insert(id.to_owned()) {
            return None;
        }
        let mut values = Vec::new();
        if let Some(init) = kids(decl).find(|c| !kind(c).ends_with("Attr")) {
            values.push(init);
        }
        let mut ok = true;
        writes(body, id, &mut values, &mut ok);
        let mut out: Option<ResultSet> = None;
        if ok {
            for value in values {
                let Some(set) = self.expr(definition, body, value, taken) else {
                    out = None;
                    break;
                };
                out = Some(match out {
                    Some(acc) => acc.union(&set),
                    None => set,
                });
            }
        } else {
            out = None;
        }
        self.locals.remove(id);
        out
    }
}

/// This unit's validated summary of `name`'s integer result range: the hull
/// of its derived set, when that is narrower than its type's range.
pub(super) fn summary(
    name: &str,
    unit: &ast::Unit,
    declared: &BTreeMap<String, RangeDecl>,
) -> Option<(i128, i128)> {
    let derived = Deriver::new(unit, declared).derive(name)?;
    let hull = derived.set.hull()?;
    (hull != derived.set.universe()).then_some(hull)
}

/// This unit's evidence on a declared range from its own body of the
/// function: `Refuted` when a `return` gives a constant outside it,
/// `Validated` when the derived set lies inside it, otherwise `Unvalidated`.
pub(super) fn evidence(
    name: &str,
    definition: &Value,
    unit: &ast::Unit,
    declared: &BTreeMap<String, RangeDecl>,
    decl: &RangeDecl,
) -> ResultEvidence {
    if let Some(body) = kids(definition).find(|c| kind(c) == "CompoundStmt") {
        let mut found = Vec::new();
        returns(body, &mut found);
        let refuted = found.iter().any(|ret| {
            kids(ret).next().is_some_and(|value| {
                ast::constant_value(strip_preserving(value), &unit.enum_values)
                    .is_some_and(|k| !decl.set.contains(k))
            })
        });
        if refuted {
            return ResultEvidence::Refuted;
        }
    }
    match Deriver::new(unit, declared).derive(name) {
        Some(derived) if derived.set.is_subset(&decl.set) => ResultEvidence::Validated,
        _ => ResultEvidence::Unvalidated,
    }
}

/// The spelling of a range evidence in the unit's export.
pub(super) fn evidence_name(evidence: ResultEvidence) -> &'static str {
    match evidence {
        ResultEvidence::Validated => "validated",
        ResultEvidence::Unvalidated => "unvalidated",
        ResultEvidence::Refuted => "refuted",
    }
}
