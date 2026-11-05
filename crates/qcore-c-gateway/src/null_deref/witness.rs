//! Card **NULL-CANDIDATE-TRIAGE** (gateway rule 14r): the reachability
//! validation of a strong-source NULL-dereference candidate.
//!
//! The walk ([`super::walk`]) joins paths, so a candidate says only that
//! *some* reaching path may carry a null pointer to the dereference. This
//! module looks for one path, from the function's entry, that reaches the
//! dereference with the pointer NULL under a **named condition**: an
//! allocation that fails, a lookup that misses, a null constant, or the null
//! arm of the function's own NULL test of a value. It runs the body one path
//! at a time: every branch whose condition the path does not decide is a
//! decision ([`qcore_core::null_triage::admit`]), and a driver enumerates
//! the decisions depth first under a budget.
//!
//! A decision is admitted only when every value the condition reads is
//! unconstrained on the path (a formal, a call's result, a value read from
//! memory since the last call or store); a condition over a value an earlier
//! decision constrained (a local assigned from an expression, a memory value
//! re-read after a call when the same spelling was decided before) is
//! *correlated* and ends the path unresolved. A condition the path decides
//! (constants, a NULL test of a pointer the path fixed, the same condition
//! over unchanged values) takes its one feasible arm.
//!
//! The result is a [`Witness`] (a finding, with its decisions, its NULL
//! event and its class), `infeasible` (every path explored, none reaches the
//! dereference NULL under a named condition), `unresolved` (a correlated
//! condition, an unread form, the budget) or `indeterminate` (a path reaches
//! it with the pointer uninitialised). Nothing here changes a check the walk
//! made; the consumer records the result beside the candidate.
//!
//! Card **NULL-TRIAGE-UNRESOLVED-PATHS** (gateway rules 15q–15u) adds the
//! *inexact* reading ([`qcore_core::null_paths`]): a condition the search
//! cannot read exactly (a call's integer result its bound does not decide, a
//! correlated value, a value it does not follow, an ordered pointer
//! comparison) no longer ends the path. Both arms are explored, which
//! over-approximates the executions, and the path is marked inexact: it is
//! never a witness, but an exploration that ends every path without reaching
//! the dereference NULL proves the candidate infeasible. A call's integer
//! result is never free: its range is a bound (its type, a checked
//! result-range contract, this unit's validated summary of the callee).
//!
//! Card **NULL-WITNESS-GUARD-CORRELATION** (gateway rule 39,
//! [`qcore_core::null_lookup`]): a lookup's miss is a function of the state
//! its arguments reach, so it is named only when feasible on the path. A
//! lookup whose body (in the unit) is one read of a place over its
//! parameters returns that place: NULL only where the path's own test fixed
//! it. Any other lookup misses freely only over constants and formals or
//! globals no earlier decision, store, assignment or call touched. A refused
//! miss is a constrained value ending its path unresolved, so it validates no
//! witness and proves no candidate infeasible.

use std::collections::{BTreeMap, BTreeSet};

use qcore_core::null_triage::{Condition, FindingClass, Triage};
use qcore_schemas::contract::{DerivedExtent, ResourceReturnMode};
use serde_json::Value;

use super::ast::{self, callee_name, kids, render, ty};
use super::walk::{Entry, Env};
use crate::typed_c_ast::kind;

/// Paths tried per candidate.
const MAX_RUNS: usize = 1024;
/// Nodes one path may visit.
const MAX_STEPS: usize = 40_000;
/// Iterations one loop may run on one path.
const LOOP_BOUND: usize = 64;

/// One step a reviewer reads.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct PathStep {
    pub line: usize,
    /// `decision`, `null-event` or `dereference`.
    pub step: &'static str,
    pub text: String,
}

/// A validated path.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Witness {
    pub class: &'static str,
    /// The NULL condition, in words.
    pub condition: String,
    pub path: Vec<PathStep>,
}

/// The outcome of the search for one candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Search {
    pub triage: Triage,
    pub reason: String,
    pub witness: Option<Witness>,
    pub runs: usize,
    /// Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15t): every declaration the
    /// explored paths read (`member:`, `result:`, `range:`, `declared:`,
    /// `inferred:` and declared-clause names); an infeasibility proof rests
    /// on all of them.
    pub consulted: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Origin {
    Allocation(String, usize),
    Lookup(String, usize),
    Formal(String),
    Memory(String),
    Uncontracted(String),
    Other(String),
    /// Card NULL-WITNESS-GUARD-CORRELATION (rule 39): a lookup's result
    /// whose miss is not a NULL event on the path (the callee, the call's
    /// line, why). Always held `constrained`.
    Refused(String, usize, String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NullWhy {
    allocation: bool,
    condition: String,
    line: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum P {
    Null(NullWhy),
    NonNull,
    /// Possibly null; `constrained` when its spelling was tested before on
    /// the path and it was re-read since.
    Opaque(Origin, bool),
    Uninit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum S {
    Const(i128),
    Atom(usize),
    /// Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15s): `atom + k`, built only
    /// when no value of the atom's range wraps or overflows in the
    /// expression's type, so a comparison of it is a comparison of the atom
    /// with a shifted constant.
    Off(usize, i128),
    /// Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15s): `a + b` of two distinct
    /// atoms whose sum interval lies inside the expression's type.
    Sum(usize, usize),
    /// A value the search does not follow (an expression of atoms, an
    /// indeterminate local, a non-integer).
    Opaque(String),
}

/// The atom and constant offset of a linear value.
fn lin(s: &S) -> Option<(usize, i128)> {
    match s {
        S::Atom(a) => Some((*a, 0)),
        S::Off(a, k) => Some((*a, *k)),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum V {
    Ptr(P, usize),
    Int(S),
}

/// One unconstrained value the path reads (a formal, a call's result, a
/// value read from memory). Decisions comparing it with a constant narrow
/// its interval (`range`, minus the excluded points `holes`): an interval of
/// one integer is exact, so a later comparison is decided only when the
/// interval decides it. Any other decision over it (a comparison with
/// another value, a mask, a conversion) marks it `constrained`, and a later
/// condition reading it is correlated.
#[derive(Clone, Debug)]
struct AtomInfo {
    constrained: bool,
    range: Option<(i128, i128)>,
    holes: BTreeSet<i128>,
    /// Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15q): a call's result (the
    /// callee's name). Its range is a **bound** (the C type's range, a
    /// checked result-range contract, a validated in-unit summary): it
    /// decides a comparison it excludes, and is never evidence that the
    /// callee returns any value in it, so no other comparison over it is
    /// taken on a witness path.
    call: Option<String>,
}

/// The values of one arm of `x OP k`: an interval, or everything but `k`.
#[derive(Clone, Copy, Debug)]
enum ArmSet {
    Interval(i128, i128),
    Except(i128),
}

fn arm_set(op: &str, k: i128, arm: bool) -> Option<ArmSet> {
    let (min, max) = (i128::MIN, i128::MAX);
    Some(match (op, arm) {
        ("==", true) | ("!=", false) => ArmSet::Interval(k, k),
        ("==", false) | ("!=", true) => ArmSet::Except(k),
        ("<", true) | (">=", false) => ArmSet::Interval(min, k.checked_sub(1)?),
        ("<", false) | (">=", true) => ArmSet::Interval(k, max),
        ("<=", true) | (">", false) => ArmSet::Interval(min, k),
        ("<=", false) | (">", true) => ArmSet::Interval(k.checked_add(1)?, max),
        _ => return None,
    })
}

impl AtomInfo {
    /// Whether some value of the atom lies in `set`.
    fn admits(&self, set: ArmSet) -> bool {
        let Some((lo, hi)) = self.range else {
            return false;
        };
        let count = |a: i128, b: i128| -> i128 {
            if a > b {
                return 0;
            }
            let holes = self.holes.range(a..=b).count() as i128;
            (b - a).saturating_add(1) - holes
        };
        match set {
            ArmSet::Interval(a, b) => count(a.max(lo), b.min(hi)) > 0,
            ArmSet::Except(k) => {
                let excluded = i128::from(k >= lo && k <= hi && !self.holes.contains(&k));
                count(lo, hi) - excluded > 0
            }
        }
    }

    fn narrow(&mut self, set: ArmSet) {
        let Some((lo, hi)) = self.range else {
            return;
        };
        match set {
            ArmSet::Interval(a, b) => {
                let (lo, hi) = (a.max(lo), b.min(hi));
                self.range = Some((lo, hi));
                self.holes.retain(|h| *h >= lo && *h <= hi);
            }
            ArmSet::Except(k) => {
                self.holes.insert(k);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PlaceKey {
    Var(String),
    Mem(String),
}

/// How one run ended.
enum End {
    /// The run needs one more decision; `prefer` is the arm tried first.
    Need(bool),
    Witness(Witness),
    Indeterminate(String),
    /// The path ended without reaching the dereference NULL.
    Ended,
    /// Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15r): a path that took an
    /// inexact arm reached the dereference NULL: no witness.
    InexactReach(String),
    /// The path faulted at an earlier dereference of a null pointer.
    Faulted,
    Unresolved(String),
}

enum Flow {
    Normal,
    Break,
    Continue,
    Return,
    Goto(String),
}

type R<T> = Result<T, End>;

struct Run<'a, 'e> {
    env: &'a Env<'e>,
    function: &'a str,
    target: &'a str,
    contains: &'a BTreeSet<*const Value>,
    /// The followed variables: declaration id -> name.
    tracked: &'a BTreeMap<String, String>,
    decisions: &'a [bool],
    used: usize,
    steps: usize,
    vars: BTreeMap<String, V>,
    memory: BTreeMap<String, V>,
    atoms: Vec<AtomInfo>,
    next_id: usize,
    /// Condition spellings decided on the path, with their atoms and arm.
    decided: BTreeMap<String, (Vec<usize>, bool)>,
    /// Memory spellings a decided condition read.
    decided_texts: BTreeSet<String>,
    needs_allocation_failure: bool,
    path: Vec<PathStep>,
    /// The arm a branch prefers to try first (the one holding the target).
    steer: Option<bool>,
    /// Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15r): the first inexact arm
    /// the path took (a call's result, a correlated value, a value the
    /// search does not follow): the path is explored, never a witness.
    inexact: Option<String>,
    /// Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15t): every declaration the
    /// run read (an infeasibility proof rests on all of them).
    consulted: BTreeSet<String>,
    /// Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15r): the second phase, which
    /// explores inexact arms; the first (exact) phase ends a path at one.
    explore_inexact: bool,
    /// Card NULL-WITNESS-GUARD-CORRELATION (rule 39): the searched function's
    /// formals (declaration ids).
    formal_ids: &'a BTreeSet<String>,
    /// Rule 39: what the path did that a lookup's state may depend on:
    /// memory spellings a pointer test read through an alias, the memory
    /// spelling (and starting range) each integer atom was read from, the
    /// stores (`None`: a place the search does not spell), the assignments
    /// of followed variables and the calls, each with its line.
    decided_reads: BTreeSet<String>,
    atom_reads: BTreeMap<usize, (String, Option<(i128, i128)>)>,
    stores: Vec<(usize, Option<String>)>,
    assigned: Vec<(usize, String)>,
    calls: Vec<(usize, String, Vec<String>)>,
}

/// Card NULL-WITNESS-GUARD-CORRELATION (rule 39): how a lookup's result is
/// read on one path.
enum LookupReading {
    /// The miss is named (`null_lookup::MissEvent::Named`).
    Named(P),
    /// The miss is refused, with the reason.
    Refused(String),
}

/// Rule 39: where a lookup argument is rooted.
enum Rooted {
    Constant,
    Decl {
        root: qcore_core::null_lookup::Root,
        id: String,
        name: String,
        pointer: bool,
    },
    Unrooted(String),
}

/// Rule 39: the place a lookup's body ties its result to, spelled as the
/// caller's own read of it would be (each parameter replaced by its
/// argument's spelling): the unit holds the lookup's definition, its body is
/// one `return` of a load of a place built only from the parameters, member
/// accesses, `&`, `*` and casts.
fn tied_place(env: &Env<'_>, callee: &str, arguments: &[&Value]) -> Option<String> {
    let definition = env.unit.definitions.get(callee)?;
    let parameters = kids(definition)
        .filter(|c| kind(c) == "ParmVarDecl")
        .map(|p| p["id"].as_str())
        .collect::<Option<Vec<_>>>()?;
    if parameters.len() != arguments.len() {
        return None;
    }
    let body = kids(definition).find(|c| kind(c) == "CompoundStmt")?;
    let mut statements = kids(body);
    let only = statements.next()?;
    if statements.next().is_some() || kind(only) != "ReturnStmt" {
        return None;
    }
    let place = loaded_place(kids(only).next()?)?;
    let substitution = parameters
        .into_iter()
        .zip(arguments.iter().map(|a| render(a)))
        .collect::<BTreeMap<_, _>>();
    tied_render(place, &substitution)
}

/// The place a returned expression loads: the value casts and parentheses
/// stripped, a member access or a dereference.
fn loaded_place(node: &Value) -> Option<&Value> {
    let mut node = node;
    loop {
        match kind(node) {
            "ParenExpr" => node = kids(node).next()?,
            "ImplicitCastExpr"
                if matches!(
                    node["castKind"].as_str(),
                    Some("LValueToRValue" | "NoOp" | "BitCast")
                ) =>
            {
                node = kids(node).next()?;
            }
            "MemberExpr" => return Some(node),
            "UnaryOperator" if node["opcode"] == "*" => return Some(node),
            _ => return None,
        }
    }
}

/// `ast::render` of a place over the lookup's parameters, each parameter
/// spelled as its argument; `None` for any other form.
fn tied_render(node: &Value, substitution: &BTreeMap<&str, String>) -> Option<String> {
    match kind(node) {
        "ParenExpr" | "ImplicitCastExpr" => tied_render(kids(node).next()?, substitution),
        "CStyleCastExpr" => Some(format!(
            "({}){}",
            node["q"].as_str().unwrap_or(""),
            tied_render(kids(node).next()?, substitution)?
        )),
        "DeclRefExpr" => substitution
            .get(node["referencedDecl"]["id"].as_str()?)
            .cloned(),
        "MemberExpr" => {
            let base = tied_render(kids(node).next()?, substitution)?;
            let op = if node["isArrow"] == true { "->" } else { "." };
            Some(format!("{base}{op}{}", node["name"].as_str()?))
        }
        "UnaryOperator"
            if node["isPostfix"] != true && matches!(node["opcode"].as_str(), Some("&" | "*")) =>
        {
            Some(format!(
                "{}{}",
                node["opcode"].as_str()?,
                tied_render(kids(node).next()?, substitution)?
            ))
        }
        _ => None,
    }
}

fn strip(node: &Value) -> &Value {
    let mut node = node;
    while matches!(
        kind(node),
        "ParenExpr" | "ConstantExpr" | "ExprWithCleanups" | "FullExpr"
    ) || (kind(node) == "UnaryOperator" && node["opcode"] == "__extension__")
    {
        match kids(node).next() {
            Some(inner) => node = inner,
            None => break,
        }
    }
    node
}

/// A C pointer type, function pointers included (`int (*)(void)`).
fn is_pointer(spelled: &str) -> bool {
    ast::is_pointer(spelled) || spelled.replace(' ', "").contains("(*)")
}

fn present(node: &Value) -> bool {
    node.as_object().is_some_and(|o| o.contains_key("kind"))
}

fn node_key(node: &Value) -> String {
    node["id"]
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("@{}", node["o"]))
}

/// The value range of a C integer type, from its spelling.
pub(super) fn int_range(spelled: &str) -> Option<(i128, i128)> {
    let words = spelled
        .split_whitespace()
        .filter(|w| !matches!(*w, "const" | "volatile"))
        .collect::<Vec<_>>()
        .join(" ");
    let range = match words.as_str() {
        "_Bool" => (0, 1),
        "char" | "signed char" => (-128, 127),
        "unsigned char" => (0, 255),
        "short" | "signed short" | "short int" => (-32_768, 32_767),
        "unsigned short" | "unsigned short int" => (0, 65_535),
        "int" | "signed int" | "signed" => (i128::from(i32::MIN), i128::from(i32::MAX)),
        "unsigned int" | "unsigned" => (0, i128::from(u32::MAX)),
        "long" | "long int" | "long long" | "long long int" | "signed long" => {
            (i128::from(i64::MIN), i128::from(i64::MAX))
        }
        "unsigned long" | "unsigned long int" | "unsigned long long" | "unsigned long long int" => {
            (0, i128::from(u64::MAX))
        }
        other if other.starts_with("enum ") => (i128::from(i32::MIN), i128::from(i32::MAX)),
        _ => return None,
    };
    Some(range)
}

fn literal(node: &Value) -> Option<i128> {
    match &node["value"] {
        Value::String(text) => {
            if let Ok(value) = text.parse() {
                return Some(value);
            }
        }
        Value::Number(n) => {
            if let Some(value) = n.as_i64() {
                return Some(i128::from(value));
            }
        }
        _ => {}
    }
    let inner = strip(node);
    if !std::ptr::eq(inner, node) {
        return literal(inner);
    }
    match kind(node) {
        "IntegerLiteral" | "CharacterLiteral" => match &node["value"] {
            Value::String(text) => text.parse().ok(),
            Value::Number(n) => n.as_i64().map(i128::from),
            _ => None,
        },
        _ => node["value"].as_str().and_then(|t| t.parse().ok()),
    }
}

fn flip(op: &str) -> &str {
    match op {
        "<" => ">",
        ">" => "<",
        "<=" => ">=",
        ">=" => "<=",
        other => other,
    }
}

fn compare(op: &str, a: i128, b: i128) -> Option<bool> {
    Some(match op {
        "==" => a == b,
        "!=" => a != b,
        "<" => a < b,
        "<=" => a <= b,
        ">" => a > b,
        ">=" => a >= b,
        _ => return None,
    })
}

impl<'a, 'e> Run<'a, 'e> {
    /// A C integer type's range, an enumeration typedef's included.
    fn int_range(&self, spelled: &str) -> Option<(i128, i128)> {
        let bare = spelled
            .split_whitespace()
            .filter(|w| !matches!(*w, "const" | "volatile"))
            .collect::<Vec<_>>()
            .join(" ");
        // Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15s): an enumeration type
        // (spelled `enum TAG` or through a typedef) has its compatible
        // integer type's range; one whose range the unit does not state is
        // not followed (before the card it read as `int`, which an
        // enumeration with no negative enumerator is not).
        if let Some(range) = self.env.unit.enum_ranges.get(&bare) {
            return Some(*range);
        }
        if bare.starts_with("enum ") || self.env.unit.enum_typedefs.contains(&bare) {
            return None;
        }
        int_range(spelled)
    }

    fn tick(&mut self) -> R<()> {
        self.steps += 1;
        if self.steps > MAX_STEPS {
            return Err(End::Unresolved("the path's step budget".into()));
        }
        Ok(())
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

    fn fresh_id(&mut self) -> usize {
        self.next_id += 1;
        self.next_id
    }

    fn new_atom(&mut self, range: Option<(i128, i128)>, constrained: bool) -> usize {
        self.new_value(range, constrained, None)
    }

    /// A new atom; `call` names the callee whose result it is (rule 15q).
    fn new_value(
        &mut self,
        range: Option<(i128, i128)>,
        constrained: bool,
        call: Option<String>,
    ) -> usize {
        self.atoms.push(AtomInfo {
            constrained,
            range,
            holes: BTreeSet::new(),
            call,
        });
        self.atoms.len() - 1
    }

    /// Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15r): take an arm of a
    /// condition the search cannot read exactly. Both arms are explored (an
    /// over-approximation, sound for an infeasibility proof); the path is no
    /// longer a witness.
    fn inexact_take(&mut self, node: &Value, reason: String) -> R<bool> {
        // The exact phase keeps rule 14r's reading: the path ends unresolved.
        if !self.explore_inexact {
            return Err(End::Unresolved(reason));
        }
        let prefer = self.steer.unwrap_or(false);
        let arm = self.decide(prefer)?;
        self.note(node, arm);
        debug_assert_eq!(
            qcore_core::null_paths::explore(qcore_core::null_paths::Reading::Inexact, arm),
            qcore_core::null_paths::Step::TakenInexact
        );
        self.inexact.get_or_insert(reason);
        Ok(arm)
    }

    /// Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15u): a tracked variable is
    /// assigned: a cached memory value whose spelling names it no longer
    /// designates the same object.
    fn forget_mentions(&mut self, id: &str) {
        let Some(name) = self.tracked.get(id).cloned() else {
            self.forget_memory();
            return;
        };
        self.memory.retain(|text, _| !mentions(text, &name));
    }

    /// The next decision, or the request for one (`prefer` first).
    fn decide(&mut self, prefer: bool) -> R<bool> {
        match self.decisions.get(self.used) {
            Some(d) => {
                self.used += 1;
                Ok(*d)
            }
            None => Err(End::Need(prefer)),
        }
    }

    /// Whether the subtree contains the target.
    fn holds_target(&self, node: &Value) -> bool {
        self.contains.contains(&std::ptr::from_ref(node))
    }

    // -------------------------------------------------------------------
    // Places
    // -------------------------------------------------------------------

    fn var_id(&self, node: &Value) -> Option<String> {
        let node = strip(node);
        if kind(node) != "DeclRefExpr" {
            return None;
        }
        let id = node["referencedDecl"]["id"].as_str()?;
        self.tracked.contains_key(id).then(|| id.to_owned())
    }

    /// Give every holder of value `id` the pointer value `p`.
    fn set_ident(&mut self, id: usize, p: &P) {
        for value in self.vars.values_mut().chain(self.memory.values_mut()) {
            if let V::Ptr(held, held_id) = value
                && *held_id == id
            {
                *held = p.clone();
            }
        }
    }

    fn forget_memory(&mut self) {
        self.memory.clear();
    }

    fn member_declared(&self, node: &Value) -> Option<String> {
        let node = strip(node);
        if kind(node) != "MemberExpr" {
            return None;
        }
        let field = node["name"].as_str()?;
        let base = kids(node).next()?;
        let tag = super::walk::record_tag(self.env.unit, ty(base), node["isArrow"] == true)?;
        let key = (tag, field.to_owned());
        self.env
            .members
            .get(&key)
            .filter(|m| m.held.is_none())
            .map(|_| format!("{}.{}", key.0, key.1))
    }

    /// Read a memory lvalue spelled `text` of type `spelled`.
    fn read_memory(&mut self, text: &str, spelled: &str, declared: bool) -> V {
        if let Some(value) = self.memory.get(text) {
            return value.clone();
        }
        let constrained = self.decided_texts.contains(text);
        let value = if is_pointer(spelled) {
            let id = self.fresh_id();
            if declared {
                V::Ptr(P::NonNull, id)
            } else {
                V::Ptr(P::Opaque(Origin::Memory(text.to_owned()), constrained), id)
            }
        } else {
            match self.int_range(spelled) {
                Some(range) => {
                    let atom = self.new_atom(Some(range), constrained);
                    self.atom_reads.insert(atom, (text.to_owned(), Some(range)));
                    V::Int(S::Atom(atom))
                }
                None => V::Int(S::Opaque(format!("a value of type `{spelled}`"))),
            }
        };
        self.memory.insert(text.to_owned(), value.clone());
        value
    }

    /// The place an lvalue designates, after the dereferences its
    /// evaluation makes.
    fn lvalue(&mut self, node: &Value) -> R<Option<PlaceKey>> {
        let node = strip(node);
        match kind(node) {
            "DeclRefExpr" => {
                if let Some(id) = self.var_id(node) {
                    return Ok(Some(PlaceKey::Var(id)));
                }
                Ok(Some(PlaceKey::Mem(render(node))))
            }
            "MemberExpr" => {
                let Some(base) = kids(node).next() else {
                    return Err(End::Unresolved("a member without a base".into()));
                };
                if node["isArrow"] == true {
                    self.dereference(node, base)?;
                } else {
                    self.lvalue(base)?;
                }
                Ok(Some(PlaceKey::Mem(render(node))))
            }
            "UnaryOperator" if node["opcode"] == "*" => {
                let Some(operand) = kids(node).next() else {
                    return Err(End::Unresolved("a dereference without an operand".into()));
                };
                self.dereference(node, operand)?;
                Ok(Some(PlaceKey::Mem(render(node))))
            }
            "ArraySubscriptExpr" => {
                self.element(node)?;
                Ok(Some(PlaceKey::Mem(render(node))))
            }
            "CompoundLiteralExpr" => {
                for child in kids(node) {
                    self.eval(child)?;
                }
                Ok(None)
            }
            other => Err(End::Unresolved(format!(
                "an lvalue the search does not read: {other}"
            ))),
        }
    }

    fn read_place(&mut self, place: &PlaceKey, node: &Value) -> V {
        match place {
            PlaceKey::Var(id) => self.vars.get(id).cloned().unwrap_or_else(|| {
                if is_pointer(ty(node)) {
                    let fresh = self.fresh_id();
                    V::Ptr(P::Uninit, fresh)
                } else {
                    V::Int(S::Opaque("an indeterminate local".into()))
                }
            }),
            PlaceKey::Mem(text) => {
                let member = self.member_declared(node);
                if let Some(member) = &member
                    && is_pointer(ty(node))
                    && !self.memory.contains_key(text)
                {
                    self.consulted.insert(format!("member:{member}"));
                }
                self.read_memory(text, ty(node), member.is_some())
            }
        }
    }

    fn write(&mut self, place: Option<PlaceKey>, value: V, at: &Value) {
        let line = self.line(at);
        match place {
            Some(PlaceKey::Var(id)) => {
                self.assigned.push((line, id.clone()));
                self.forget_mentions(&id);
                self.vars.insert(id, value);
            }
            Some(PlaceKey::Mem(text)) => {
                self.stores.push((line, Some(text.clone())));
                self.forget_memory();
                self.memory.insert(text, value);
            }
            None => {
                self.stores.push((line, None));
                self.forget_memory();
            }
        }
    }

    // -------------------------------------------------------------------
    // Dereferences
    // -------------------------------------------------------------------

    /// The pointer `value` is dereferenced at `node` (the target or not).
    fn judge(&mut self, node: &Value, value: V) -> R<()> {
        let V::Ptr(p, id) = value else {
            return Ok(());
        };
        if node_key(node) == self.target {
            let line = self.line(node);
            let why = match &p {
                P::Null(why) => Some(why.clone()),
                P::Opaque(Origin::Allocation(name, at), _) => Some(NullWhy {
                    allocation: true,
                    condition: format!("the allocation `{name}` at line {at} fails (returns NULL)"),
                    line: *at,
                }),
                P::Opaque(Origin::Lookup(name, at), _) => Some(NullWhy {
                    allocation: false,
                    condition: format!("the lookup `{name}` at line {at} misses (returns NULL)"),
                    line: *at,
                }),
                // Card NULL-WITNESS-GUARD-CORRELATION (rule 39): a refused
                // miss ends the path unresolved, never `Ended`, so it neither
                // validates a witness nor proves the candidate infeasible.
                P::Opaque(Origin::Refused(name, at, reason), _) => {
                    return Err(End::Unresolved(format!(
                        "the lookup `{name}` at line {at} reaches the dereference at line {line}, but its miss is not a NULL event on this path (rule 39): {reason}"
                    )));
                }
                P::Uninit => {
                    let why =
                        format!("the pointer is uninitialised on a path reaching line {line}");
                    // Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15r): only an
                    // exact path shows an indeterminate value.
                    return Err(match &self.inexact {
                        Some(inexact) => {
                            End::InexactReach(format!("{why}, but only along {inexact}"))
                        }
                        None => End::Indeterminate(why),
                    });
                }
                _ => None,
            };
            if let Some(why) = why {
                // Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15r): a path that
                // took an inexact arm is no witness.
                if let Some(inexact) = &self.inexact {
                    return Err(End::InexactReach(format!(
                        "{} at line {line}, but only along {inexact}",
                        why.condition
                    )));
                }
                let allocation = why.allocation || self.needs_allocation_failure;
                let mut path = self.path.clone();
                if !path
                    .iter()
                    .any(|s| s.step == "null-event" && s.line == why.line)
                {
                    path.push(PathStep {
                        line: why.line,
                        step: "null-event",
                        text: why.condition.clone(),
                    });
                }
                path.push(PathStep {
                    line,
                    step: "dereference",
                    text: render(node),
                });
                return Err(End::Witness(Witness {
                    class: if allocation {
                        FindingClass::AllocationFailure.as_str()
                    } else {
                        FindingClass::NormalPath.as_str()
                    },
                    condition: why.condition,
                    path,
                }));
            }
        }
        match p {
            P::Null(_) => Err(End::Faulted),
            P::Uninit => Err(End::Unresolved(
                "an uninitialised pointer is dereferenced earlier on the path".into(),
            )),
            P::Opaque(..) => {
                self.set_ident(id, &P::NonNull);
                Ok(())
            }
            P::NonNull => Ok(()),
        }
    }

    fn dereference(&mut self, node: &Value, pointer: &Value) -> R<()> {
        let value = self.eval(pointer)?;
        self.judge(node, value)
    }

    fn element(&mut self, node: &Value) -> R<()> {
        let parts = kids(node).collect::<Vec<_>>();
        let (Some(first), Some(second)) = (parts.first(), parts.get(1)) else {
            return Err(End::Unresolved("an element access".into()));
        };
        let (base, index) = if is_pointer(ty(strip(first))) || !is_pointer(ty(strip(second))) {
            (*first, *second)
        } else {
            (*second, *first)
        };
        let stripped = strip(base);
        if kind(stripped) == "ImplicitCastExpr" && stripped["castKind"] == "ArrayToPointerDecay" {
            if let Some(array) = kids(stripped).next() {
                self.lvalue(array)?;
            }
            self.eval(index)?;
            return Ok(());
        }
        let value = self.eval(base)?;
        self.eval(index)?;
        self.judge(node, value)
    }

    // -------------------------------------------------------------------
    // Expressions
    // -------------------------------------------------------------------

    fn eval(&mut self, node: &Value) -> R<V> {
        self.tick()?;
        match kind(node) {
            "ParenExpr"
            | "ConstantExpr"
            | "ExprWithCleanups"
            | "FullExpr"
            | "OpaqueValueExpr"
            | "MaterializeTemporaryExpr" => match kids(node).next() {
                Some(inner) => self.eval(inner),
                None => Ok(V::Int(S::Opaque("an empty expression".into()))),
            },
            "ImplicitCastExpr" | "CStyleCastExpr" | "BuiltinBitCastExpr" => self.cast(node),
            "DeclRefExpr" => match node["referencedDecl"]["kind"].as_str() {
                Some("FunctionDecl") => Ok(V::Ptr(P::NonNull, self.fresh_id())),
                Some("EnumConstantDecl") => Ok(V::Int(
                    node["referencedDecl"]["id"]
                        .as_str()
                        .and_then(|id| self.env.unit.enum_values.get(id))
                        .copied()
                        .map_or_else(|| S::Opaque("an enumeration constant".into()), S::Const),
                )),
                Some("VarDecl" | "ParmVarDecl") => {
                    let place = self.lvalue(node)?;
                    Ok(match place {
                        Some(place) => self.read_place(&place, node),
                        None => V::Int(S::Opaque("a variable".into())),
                    })
                }
                _ => Ok(V::Int(S::Opaque("a declaration".into()))),
            },
            "IntegerLiteral" | "CharacterLiteral" => Ok(V::Int(
                literal(node).map_or_else(|| S::Opaque("a literal".into()), S::Const),
            )),
            "FloatingLiteral" | "ImaginaryLiteral" | "FixedPointLiteral" => {
                Ok(V::Int(S::Opaque("a floating value".into())))
            }
            "StringLiteral" | "PredefinedExpr" | "AddrLabelExpr" => {
                Ok(V::Ptr(P::NonNull, self.fresh_id()))
            }
            "UnaryOperator" => self.unary(node),
            "BinaryOperator" => self.binary(node),
            "CompoundAssignOperator" => {
                let mut parts = kids(node);
                let (Some(lhs), Some(rhs)) = (parts.next(), parts.next()) else {
                    return Err(End::Unresolved("a compound assignment".into()));
                };
                let place = self.lvalue(lhs)?;
                let current = match &place {
                    Some(place) => self.read_place(place, lhs),
                    None => V::Int(S::Opaque("an rvalue".into())),
                };
                let right = self.eval(rhs)?;
                let op = node["opcode"].as_str().unwrap_or("");
                let updated = match (current, right) {
                    (V::Ptr(p, id), _) => V::Ptr(
                        match p {
                            P::Null(_) | P::Uninit => {
                                P::Opaque(Origin::Other("offset".into()), true)
                            }
                            other => other,
                        },
                        id,
                    ),
                    (V::Int(S::Const(a)), V::Int(S::Const(b))) => V::Int(
                        arith(op.trim_end_matches('='), a, b)
                            .map_or_else(|| S::Opaque("an arithmetic result".into()), S::Const),
                    ),
                    _ => V::Int(S::Opaque("an updated value".into())),
                };
                self.write(place, updated.clone(), node);
                Ok(updated)
            }
            "ConditionalOperator" => {
                let parts = kids(node).collect::<Vec<_>>();
                let [condition, yes, no] = parts.as_slice() else {
                    return Err(End::Unresolved("a conditional operator".into()));
                };
                if self.test(condition)? {
                    self.eval(yes)
                } else {
                    self.eval(no)
                }
            }
            "BinaryConditionalOperator" => {
                let parts = kids(node).collect::<Vec<_>>();
                let (Some(common), Some(no)) = (parts.first(), parts.last()) else {
                    return Err(End::Unresolved("a binary conditional".into()));
                };
                let value = self.eval(common)?;
                let truth = self.truth(common, &value)?;
                if truth { Ok(value) } else { self.eval(no) }
            }
            "CallExpr" => self.call(node),
            "MemberExpr" | "ArraySubscriptExpr" => {
                let place = self.lvalue(node)?;
                Ok(match place {
                    Some(place) => self.read_place(&place, node),
                    None => V::Int(S::Opaque("an rvalue member".into())),
                })
            }
            "UnaryExprOrTypeTraitExpr" | "OffsetOfExpr" | "TypeTraitExpr" => Ok(V::Int(
                literal(node).map_or_else(|| S::Opaque("a size".into()), S::Const),
            )),
            "InitListExpr" | "ParenListExpr" | "DesignatedInitExpr" | "CompoundLiteralExpr" => {
                for child in kids(node) {
                    self.eval(child)?;
                }
                Ok(V::Int(S::Opaque("an aggregate".into())))
            }
            "ImplicitValueInitExpr" | "NoInitExpr" => Ok(V::Int(S::Const(0))),
            "StmtExpr" => {
                let Some(body) = kids(node).next() else {
                    return Ok(V::Int(S::Opaque("a statement expression".into())));
                };
                let parts = kids(body).collect::<Vec<_>>();
                let Some((last, rest)) = parts.split_last() else {
                    return Ok(V::Int(S::Opaque("a statement expression".into())));
                };
                for part in rest {
                    match self.stmt(part)? {
                        Flow::Normal => {}
                        _ => {
                            return Err(End::Unresolved(
                                "a jump out of a statement expression".into(),
                            ));
                        }
                    }
                }
                if super::walk::is_expression_kind(last) {
                    self.eval(last)
                } else {
                    match self.stmt(last)? {
                        Flow::Normal => Ok(V::Int(S::Opaque("a statement expression".into()))),
                        _ => Err(End::Unresolved(
                            "a jump out of a statement expression".into(),
                        )),
                    }
                }
            }
            "GenericSelectionExpr" => {
                match kids(node)
                    .find(|c| c["selected"] == true)
                    .and_then(|a| kids(a).next_back())
                {
                    Some(selected) => self.eval(selected),
                    None => Err(End::Unresolved("a generic selection".into())),
                }
            }
            "VAArgExpr" => {
                for child in kids(node) {
                    self.eval(child)?;
                }
                let fresh = self.fresh_id();
                Ok(if is_pointer(ty(node)) {
                    V::Ptr(P::Opaque(Origin::Other("va_arg".into()), false), fresh)
                } else {
                    V::Int(S::Opaque("va_arg".into()))
                })
            }
            kind_name if kind_name.ends_with("Type") => Ok(V::Int(S::Opaque("a type".into()))),
            other => Err(End::Unresolved(format!("an unread form: {other}"))),
        }
    }

    fn cast(&mut self, node: &Value) -> R<V> {
        let Some(operand) = kids(node).next() else {
            return Err(End::Unresolved("a cast without an operand".into()));
        };
        match node["castKind"].as_str() {
            Some("LValueToRValue") => {
                let place = self.lvalue(operand)?;
                Ok(match place {
                    Some(place) => self.read_place(&place, operand),
                    None => V::Int(S::Opaque("an rvalue".into())),
                })
            }
            Some("ArrayToPointerDecay") => {
                let array = strip(operand);
                if kind(array) == "MemberExpr"
                    && array["isArrow"] == true
                    && let Some(base) = kids(array).next()
                {
                    // `&B->G` is no dereference of `B`: its null-ness is B's.
                    return self.eval(base);
                }
                if matches!(kind(array), "MemberExpr" | "ArraySubscriptExpr")
                    || (kind(array) == "UnaryOperator" && array["opcode"] == "*")
                {
                    return self.address(array);
                }
                if matches!(kind(array), "CompoundLiteralExpr" | "StmtExpr" | "CallExpr") {
                    self.eval(array)?;
                }
                Ok(V::Ptr(P::NonNull, self.fresh_id()))
            }
            Some("FunctionToPointerDecay" | "BuiltinFnToFnPtr") => {
                Ok(V::Ptr(P::NonNull, self.fresh_id()))
            }
            Some("NullToPointer") => {
                self.eval(operand)?;
                let line = self.line(node);
                Ok(V::Ptr(
                    P::Null(NullWhy {
                        allocation: false,
                        condition: format!("a null pointer constant at line {line}"),
                        line,
                    }),
                    self.fresh_id(),
                ))
            }
            Some("IntegralToPointer") => {
                let value = self.eval(operand)?;
                let line = self.line(node);
                Ok(match value {
                    V::Int(S::Const(0)) => V::Ptr(
                        P::Null(NullWhy {
                            allocation: false,
                            condition: format!("a null pointer constant at line {line}"),
                            line,
                        }),
                        self.fresh_id(),
                    ),
                    _ => V::Ptr(
                        P::Opaque(Origin::Other("integer-to-pointer".into()), true),
                        self.fresh_id(),
                    ),
                })
            }
            Some("PointerToBoolean") => {
                let value = self.eval(operand)?;
                Ok(match value {
                    V::Ptr(P::Null(_), _) => V::Int(S::Const(0)),
                    V::Ptr(P::NonNull, _) => V::Int(S::Const(1)),
                    _ => V::Int(S::Opaque("a pointer's truth".into())),
                })
            }
            Some("PointerToIntegral") => {
                self.eval(operand)?;
                Ok(V::Int(S::Opaque("a pointer's address".into())))
            }
            _ => {
                let value = self.eval(operand)?;
                if is_pointer(ty(node)) {
                    match value {
                        V::Ptr(..) => Ok(value),
                        V::Int(S::Const(0)) => {
                            let line = self.line(node);
                            Ok(V::Ptr(
                                P::Null(NullWhy {
                                    allocation: false,
                                    condition: format!("a null pointer constant at line {line}"),
                                    line,
                                }),
                                self.fresh_id(),
                            ))
                        }
                        V::Int(_) => Ok(V::Ptr(
                            P::Opaque(Origin::Other("integer-to-pointer".into()), true),
                            self.fresh_id(),
                        )),
                    }
                } else {
                    Ok(match value {
                        V::Int(S::Atom(atom)) => {
                            // A conversion whose target admits every value of
                            // the atom's range keeps it; any other is a new
                            // value of the target's range, its source
                            // constrained.
                            let target = self.int_range(ty(node));
                            match (self.atoms[atom].range, target) {
                                (Some((lo, hi)), Some((tlo, thi))) if tlo <= lo && hi <= thi => {
                                    V::Int(S::Atom(atom))
                                }
                                (_, Some(range)) => {
                                    let constrained = self.atoms[atom].constrained;
                                    let call = self.atoms[atom].call.clone();
                                    self.atoms[atom].constrained = true;
                                    V::Int(S::Atom(self.new_value(Some(range), constrained, call)))
                                }
                                (_, None) => V::Int(S::Opaque("a conversion".into())),
                            }
                        }
                        // Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15s): an
                        // offset or a sum survives a conversion whose target
                        // admits every value it may take.
                        V::Int(S::Off(atom, k)) => {
                            let target = self.int_range(ty(node));
                            match (self.atoms[atom].range, target) {
                                (Some((lo, hi)), Some((tlo, thi)))
                                    if lo.checked_add(k).is_some_and(|a| tlo <= a)
                                        && hi.checked_add(k).is_some_and(|b| b <= thi) =>
                                {
                                    V::Int(S::Off(atom, k))
                                }
                                _ => V::Int(S::Opaque("a conversion of an offset".into())),
                            }
                        }
                        V::Int(S::Sum(a, b)) => {
                            let target = self.int_range(ty(node));
                            match (self.atoms[a].range, self.atoms[b].range, target) {
                                (Some((alo, ahi)), Some((blo, bhi)), Some((tlo, thi)))
                                    if tlo <= alo + blo && ahi + bhi <= thi =>
                                {
                                    V::Int(S::Sum(a, b))
                                }
                                _ => V::Int(S::Opaque("a conversion of a sum".into())),
                            }
                        }
                        V::Int(s) => V::Int(s),
                        V::Ptr(..) => V::Int(S::Opaque("a pointer conversion".into())),
                    })
                }
            }
        }
    }

    /// `&operand`.
    fn address(&mut self, operand: &Value) -> R<V> {
        let operand = strip(operand);
        match kind(operand) {
            "MemberExpr" => {
                let Some(base) = kids(operand).next() else {
                    return Ok(V::Ptr(P::NonNull, self.fresh_id()));
                };
                if operand["isArrow"] == true {
                    // `&B->G` is not a dereference of `B` (decision 1).
                    let value = self.eval(base)?;
                    Ok(offset_of(value))
                } else {
                    let value = self.address(base)?;
                    Ok(offset_of(value))
                }
            }
            "ArraySubscriptExpr" => {
                let parts = kids(operand).collect::<Vec<_>>();
                let (Some(first), Some(second)) = (parts.first(), parts.get(1)) else {
                    return Ok(V::Ptr(P::NonNull, self.fresh_id()));
                };
                let (base, index) =
                    if is_pointer(ty(strip(first))) || !is_pointer(ty(strip(second))) {
                        (*first, *second)
                    } else {
                        (*second, *first)
                    };
                let value = self.eval(base)?;
                self.eval(index)?;
                Ok(offset_of(value))
            }
            "UnaryOperator" if operand["opcode"] == "*" => match kids(operand).next() {
                Some(inner) => self.eval(inner),
                None => Ok(V::Ptr(P::NonNull, self.fresh_id())),
            },
            "CompoundLiteralExpr" => {
                for child in kids(operand) {
                    self.eval(child)?;
                }
                Ok(V::Ptr(P::NonNull, self.fresh_id()))
            }
            "DeclRefExpr" | "StringLiteral" | "PredefinedExpr" => {
                Ok(V::Ptr(P::NonNull, self.fresh_id()))
            }
            other => Err(End::Unresolved(format!("an address of a {other}"))),
        }
    }

    fn unary(&mut self, node: &Value) -> R<V> {
        let Some(operand) = kids(node).next() else {
            return Err(End::Unresolved("a unary operator".into()));
        };
        match node["opcode"].as_str().unwrap_or("") {
            "*" => {
                let place = self.lvalue(node)?;
                Ok(match place {
                    Some(place) => self.read_place(&place, node),
                    None => V::Int(S::Opaque("a dereference".into())),
                })
            }
            "&" => self.address(operand),
            "++" | "--" => {
                let place = self.lvalue(operand)?;
                let current = match &place {
                    Some(place) => self.read_place(place, operand),
                    None => V::Int(S::Opaque("an rvalue".into())),
                };
                let updated = match &current {
                    V::Ptr(p, id) => V::Ptr(
                        match p {
                            P::Null(_) | P::Uninit => {
                                P::Opaque(Origin::Other("offset".into()), true)
                            }
                            other => other.clone(),
                        },
                        *id,
                    ),
                    V::Int(S::Const(a)) => {
                        V::Int(S::Const(if node["opcode"] == "++" { a + 1 } else { a - 1 }))
                    }
                    V::Int(_) => V::Int(S::Opaque("an incremented value".into())),
                };
                self.write(place, updated.clone(), node);
                Ok(if node["isPostfix"] == true {
                    current
                } else {
                    updated
                })
            }
            "!" => {
                let value = self.eval(operand)?;
                Ok(match value {
                    V::Int(S::Const(a)) => V::Int(S::Const(i128::from(a == 0))),
                    V::Ptr(P::Null(_), _) => V::Int(S::Const(1)),
                    V::Ptr(P::NonNull, _) => V::Int(S::Const(0)),
                    _ => V::Int(S::Opaque("a negation".into())),
                })
            }
            "-" => {
                let value = self.eval(operand)?;
                Ok(match value {
                    V::Int(S::Const(a)) => V::Int(S::Const(-a)),
                    _ => V::Int(S::Opaque("a negation".into())),
                })
            }
            "~" => {
                let value = self.eval(operand)?;
                Ok(match value {
                    V::Int(S::Const(a)) => V::Int(S::Const(!a)),
                    _ => V::Int(S::Opaque("a complement".into())),
                })
            }
            "__extension__" | "+" => self.eval(operand),
            _ => {
                self.eval(operand)?;
                Ok(V::Int(S::Opaque("a unary result".into())))
            }
        }
    }

    fn binary(&mut self, node: &Value) -> R<V> {
        let mut parts = kids(node);
        let (Some(lhs), Some(rhs)) = (parts.next(), parts.next()) else {
            return Err(End::Unresolved("a binary operator".into()));
        };
        let op = node["opcode"].as_str().unwrap_or("");
        match op {
            "=" => {
                let place = self.lvalue(lhs)?;
                let value = self.eval(rhs)?;
                self.write(place, value.clone(), node);
                Ok(value)
            }
            "," => {
                self.eval(lhs)?;
                self.eval(rhs)
            }
            "&&" | "||" | "==" | "!=" | "<" | "<=" | ">" | ">=" => {
                let truth = self.test(node)?;
                Ok(V::Int(S::Const(i128::from(truth))))
            }
            _ => {
                let left = self.eval(lhs)?;
                let right = self.eval(rhs)?;
                Ok(match (left, right) {
                    (V::Ptr(p, id), V::Int(_)) if is_pointer(ty(node)) => offset_of(V::Ptr(p, id)),
                    (V::Int(_), V::Ptr(p, id)) if is_pointer(ty(node)) => offset_of(V::Ptr(p, id)),
                    (V::Int(S::Const(a)), V::Int(S::Const(b))) => V::Int(
                        arith(op, a, b)
                            .map_or_else(|| S::Opaque("an arithmetic result".into()), S::Const),
                    ),
                    // Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15s): `x + k`,
                    // `x - k`, `k + x` of a linear value, and `a + b` of two
                    // atoms, followed when no value wraps or overflows.
                    (V::Int(x), V::Int(S::Const(k)))
                        if matches!(op, "+" | "-") && lin(&x).is_some() =>
                    {
                        let k = if op == "+" { Some(k) } else { k.checked_neg() };
                        V::Int(self.offset(&x, k, ty(node), node))
                    }
                    (V::Int(S::Const(k)), V::Int(x)) if op == "+" && lin(&x).is_some() => {
                        V::Int(self.offset(&x, Some(k), ty(node), node))
                    }
                    (V::Int(S::Atom(a)), V::Int(S::Atom(b))) if op == "+" && a != b => {
                        V::Int(self.sum(a, b, ty(node), node))
                    }
                    // `a & K` of an unconstrained `a`: a new value in
                    // [0, K] (every one reachable from some `a`), `a`
                    // itself constrained.
                    (V::Int(S::Atom(a)), V::Int(S::Const(k)))
                    | (V::Int(S::Const(k)), V::Int(S::Atom(a)))
                        if op == "&"
                            && k > 0
                            && self.atoms[a]
                                .range
                                .is_some_and(|(lo, hi)| lo <= 0 && hi >= k) =>
                    {
                        let constrained = self.atoms[a].constrained;
                        let call = self.atoms[a].call.clone();
                        self.atoms[a].constrained = true;
                        if k & (k - 1) == 0 {
                            // A single bit: the result is 0 or K.
                            V::Int(S::Atom(self.new_value(Some((0, k)), constrained, call)))
                        } else {
                            V::Int(S::Opaque(format!("a mask `{}`", render(node))))
                        }
                    }
                    _ => V::Int(S::Opaque(format!("an expression `{}`", render(node)))),
                })
            }
        }
    }

    fn call(&mut self, node: &Value) -> R<V> {
        let parts = kids(node).collect::<Vec<_>>();
        let Some((callee, arguments)) = parts.split_first() else {
            return Err(End::Unresolved("a call".into()));
        };
        let name = callee_name(node);
        match name.as_deref() {
            Some("__builtin_expect" | "__builtin_expect_with_probability") => {
                let value = match arguments.first() {
                    Some(a) => self.eval(a)?,
                    None => V::Int(S::Opaque("an expectation".into())),
                };
                for rest in arguments.iter().skip(1) {
                    self.eval(rest)?;
                }
                return Ok(value);
            }
            Some(
                "__builtin_constant_p"
                | "__builtin_object_size"
                | "__builtin_dynamic_object_size"
                | "__builtin_classify_type",
            ) => return Ok(V::Int(S::Opaque("a builtin's answer".into()))),
            _ => {}
        }
        if name.is_none() {
            self.eval(callee)?;
        }
        let mut values = Vec::new();
        for argument in arguments {
            values.push(self.eval(argument)?);
        }
        // Clause positions: the callee dereferences each such actual.
        let mut positions = BTreeSet::new();
        if let Some(callee_name) = &name {
            if let Some(declared) = self.env.declared.get(callee_name) {
                if declared.held.is_none() && declared.parameters == values.len() {
                    positions.extend(declared.positions.keys().copied());
                    self.consulted.insert(callee_name.clone());
                }
            } else if let Some(summary) = self.env.summaries.get(callee_name)
                && callee_name != self.function
            {
                positions.extend(summary.derefs.iter().copied());
                if !summary.derefs.is_empty() {
                    self.consulted.extend(summary.dependencies.iter().cloned());
                    self.consulted.insert(format!("inferred:{callee_name}"));
                }
            }
        }
        for position in positions {
            if let (Some(argument), Some(value)) = (arguments.get(position), values.get(position)) {
                self.judge(argument, value.clone())?;
            }
        }
        // Card NULL-WITNESS-GUARD-CORRELATION (rule 39): a lookup is read
        // before the call forgets the cached memory values and before the
        // call joins the path's record of calls.
        let line = self.line(node);
        let lookup = match &name {
            Some(callee_name) if self.is_lookup(callee_name) => {
                Some(self.read_lookup(callee_name, line, arguments))
            }
            _ => None,
        };
        self.calls.push((
            line,
            name.clone().unwrap_or_else(|| render(callee)),
            arguments.iter().map(|a| render(a)).collect(),
        ));
        self.forget_memory();
        if let Some(callee_name) = &name
            && (self.env.unit.no_return.contains(callee_name)
                || ast::builtin_no_return(callee_name))
        {
            return Err(End::Ended);
        }
        let id = self.fresh_id();
        if !is_pointer(ty(node)) {
            // Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15q): a call's integer
            // result is bounded (its type, a checked result-range contract,
            // an in-unit validated summary), never free.
            return Ok(match self.int_range(ty(node)) {
                Some(range) => {
                    let callee = name.clone().unwrap_or_else(|| "an indirect call".into());
                    let bound = self.result_bound(&callee, range);
                    V::Int(S::Atom(self.new_value(Some(bound), false, Some(callee))))
                }
                None => V::Int(S::Opaque(format!("a result of type `{}`", ty(node)))),
            });
        }
        let Some(callee_name) = name else {
            return Ok(V::Ptr(
                P::Opaque(Origin::Uncontracted("indirect call".into()), false),
                id,
            ));
        };
        if matches!(
            callee_name.as_str(),
            "__builtin_alloca" | "__builtin_alloca_with_align"
        ) || ast::is_errno_location(&callee_name)
        {
            return Ok(V::Ptr(P::NonNull, id));
        }
        if self
            .env
            .results
            .get(&callee_name)
            .is_some_and(|r| r.held.is_none())
        {
            self.consulted.insert(format!("result:{callee_name}"));
            return Ok(V::Ptr(P::NonNull, id));
        }
        if let Some(summary) = self.env.summaries.get(&callee_name)
            && summary.returns_non_null
            && callee_name != self.function
        {
            self.consulted.extend(summary.dependencies.iter().cloned());
            return Ok(V::Ptr(P::NonNull, id));
        }
        let contract = crate::contract_dependencies::unrecorded(|| {
            self.env
                .resource
                .contract(&callee_name)
                .filter(|_| !self.env.checked_results.contains(&callee_name))
                .cloned()
        });
        let p = match contract.map(|c| c.result) {
            Some(
                ResourceReturnMode::Owned { nullable: true }
                | ResourceReturnMode::Reallocated { nullable: true, .. },
            ) => P::Opaque(Origin::Allocation(callee_name, line), false),
            Some(
                ResourceReturnMode::Owned { nullable: false }
                | ResourceReturnMode::Reallocated {
                    nullable: false, ..
                },
            ) => {
                self.consulted.insert(format!("declared:{callee_name}"));
                P::NonNull
            }
            // Rule 39: a lookup's miss is named only when it is feasible on
            // the path by facts the search reads.
            Some(ResourceReturnMode::Derived { nullable: true, .. }) => match lookup {
                Some(LookupReading::Named(p)) => p,
                Some(LookupReading::Refused(reason)) => {
                    P::Opaque(Origin::Refused(callee_name, line, reason), true)
                }
                None => P::Opaque(
                    Origin::Refused(
                        callee_name,
                        line,
                        "the search did not read the lookup".into(),
                    ),
                    true,
                ),
            },
            Some(ResourceReturnMode::Derived {
                nullable: false,
                parameter,
                extent,
                ..
            }) => {
                // Rule 36: relative non-nullability is not an absolute
                // non-null result. Exact aliases preserve a known NULL base.
                // An unresolved base cuts the search; it cannot prove the
                // target infeasible by losing its possible NULL value.
                self.consulted.insert(format!("declared:{callee_name}"));
                match values.get(parameter) {
                    Some(V::Ptr(P::NonNull, _)) => P::NonNull,
                    Some(V::Ptr(P::Null(why), _)) if extent == DerivedExtent::Exact => {
                        P::Null(why.clone())
                    }
                    _ => {
                        return Err(End::Unresolved(format!(
                            "the nullness of derived result `{callee_name}` is unresolved from its base"
                        )));
                    }
                }
            }
            Some(ResourceReturnMode::None | ResourceReturnMode::Removed { .. }) | None => {
                P::Opaque(Origin::Uncontracted(callee_name), false)
            }
        };
        Ok(V::Ptr(p, id))
    }

    /// Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15q): the bound of a call's
    /// integer result: its type's range, narrowed by a result-range
    /// declaration in force (not held, not refuted by this unit's body) and
    /// by this unit's validated summary of the callee's body.
    fn result_bound(&mut self, callee: &str, (lo, hi): (i128, i128)) -> (i128, i128) {
        let (mut lo, mut hi) = (lo, hi);
        // Card CELL-GUARD-RESULT-RANGE-CHECKED: a C library result set binds
        // only a call of the declared result type (its signature).
        if let Some(declared) = self.env.ranges.get(callee)
            && declared.in_force()
            && (!declared.flexible || declared.set.universe() == (lo, hi))
            && declared.lo.max(lo) <= declared.hi.min(hi)
        {
            lo = lo.max(declared.lo);
            hi = hi.min(declared.hi);
            self.consulted.insert(format!("range:{callee}"));
        }
        if callee != self.function
            && let Some((slo, shi)) = self.env.range_summaries.get(callee).copied()
            && slo.max(lo) <= shi.min(hi)
        {
            lo = lo.max(slo);
            hi = hi.min(shi);
        }
        (lo, hi)
    }

    // -------------------------------------------------------------------
    // Lookups (card NULL-WITNESS-GUARD-CORRELATION, rule 39)
    // -------------------------------------------------------------------

    /// A callee whose declared result is borrowed from an argument and
    /// nullable (the same reading the result's classification below makes).
    fn is_lookup(&self, callee: &str) -> bool {
        crate::contract_dependencies::unrecorded(|| {
            self.env
                .resource
                .contract(callee)
                .filter(|_| !self.env.checked_results.contains(callee))
                .is_some_and(|c| {
                    matches!(c.result, ResourceReturnMode::Derived { nullable: true, .. })
                })
        })
    }

    /// Whether the miss of the lookup `callee` at `line` is a NULL event on
    /// this path: the body's tie when the unit holds one, otherwise the state
    /// its arguments reach (`qcore_core::null_lookup`).
    fn read_lookup(&self, callee: &str, line: usize, arguments: &[&Value]) -> LookupReading {
        use qcore_core::null_lookup::{MissEvent, State, Tie, lookup_state, miss_event};
        if let Some(place) = tied_place(self.env, callee, arguments) {
            let cached = self.memory.get(&place);
            let tie = match cached {
                Some(V::Ptr(P::Null(_), _)) => Tie::DecidedNull,
                _ => Tie::Other,
            };
            return match (miss_event(tie, State::Correlated), cached) {
                (MissEvent::Named, Some(V::Ptr(P::Null(why), _))) => {
                    LookupReading::Named(P::Null(NullWhy {
                        allocation: why.allocation,
                        condition: format!(
                            "the lookup `{callee}` at line {line} returns `{place}` (its body), which is NULL on this path: {}",
                            why.condition
                        ),
                        line,
                    }))
                }
                _ => {
                    let detail = match cached {
                        Some(V::Ptr(P::NonNull, _)) => {
                            "which the path already fixed non-null, so the miss is excluded"
                        }
                        _ if self.decided_texts.contains(&place)
                            || self.decided_reads.contains(&place) =>
                        {
                            "which the path tested before a call or store: correlated"
                        }
                        _ => {
                            "a memory value no condition on the path names NULL (rule 14r), not a miss of the lookup's own"
                        }
                    };
                    LookupReading::Refused(format!("its body returns `{place}`, {detail}"))
                }
            };
        }
        let mut states = Vec::new();
        let mut first_reason = None;
        for argument in arguments {
            let (state, reason) = self.argument_state(argument);
            if state == State::Correlated && first_reason.is_none() {
                first_reason = Some(format!(
                    "it reads the state of `{}`: {}",
                    render(argument),
                    reason.unwrap_or_default()
                ));
            }
            states.push(state);
        }
        match miss_event(Tie::None, lookup_state(&states)) {
            MissEvent::Named => {
                LookupReading::Named(P::Opaque(Origin::Lookup(callee.to_owned(), line), false))
            }
            MissEvent::Refused => LookupReading::Refused(
                first_reason.unwrap_or_else(|| "a correlated argument".into()),
            ),
        }
    }

    /// The state one lookup argument contributes, with the reason when it is
    /// correlated (`null_lookup::argument_state`).
    fn argument_state(&self, argument: &Value) -> (qcore_core::null_lookup::State, Option<String>) {
        use qcore_core::null_lookup::{Root, State, argument_state};
        let (root, id, name, pointer) = match self.argument_root(argument) {
            Rooted::Constant => return (State::Free, None),
            Rooted::Unrooted(what) => {
                return (
                    State::Correlated,
                    Some(format!(
                        "an argument the search does not root at an entry value ({what})"
                    )),
                );
            }
            Rooted::Decl {
                root,
                id,
                name,
                pointer,
            } => (root, id, name, pointer),
        };
        if root == Root::Local {
            return (
                State::Correlated,
                Some(format!(
                    "`{name}` is a local of this function, whose value or object the path establishes"
                )),
            );
        }
        let touched = self.touched(&id, &name, pointer);
        let call_before = self
            .calls
            .first()
            .map(|(line, callee, _)| (*line, callee.clone()));
        let state = argument_state(root, touched.is_some(), call_before.is_some());
        let reason = match (state, touched, call_before) {
            (State::Free, _, _) => None,
            (State::Correlated, Some(touched), _) => Some(touched),
            (State::Correlated, None, Some((line, callee))) => Some(format!(
                "the call `{callee}` at line {line} precedes it on the path and may write the global `{name}`"
            )),
            (State::Correlated, None, None) => Some(format!("`{name}` is correlated")),
        };
        (state, reason)
    }

    /// Whether an earlier decision, store, assignment or call on the path
    /// read or wrote through the root `name` (declaration `id`); a NULL test
    /// of a pointer root itself reads the pointer, not what it reaches.
    fn touched(&self, id: &str, name: &str, pointer: bool) -> Option<String> {
        let decided_atoms = self.atom_reads.iter().filter_map(|(atom, (text, start))| {
            let info = &self.atoms[*atom];
            (info.constrained || !info.holes.is_empty() || info.range != *start).then_some(text)
        });
        for text in self
            .decided_texts
            .iter()
            .chain(self.decided_reads.iter())
            .chain(decided_atoms)
        {
            if mentions(text, name) && !(pointer && text == name) {
                return Some(format!(
                    "the path tested `{text}`, which reads through `{name}`"
                ));
            }
        }
        for (line, text) in &self.stores {
            match text {
                None => {
                    return Some(format!(
                        "the path wrote a place the search does not spell at line {line}"
                    ));
                }
                Some(text) if mentions(text, name) => {
                    return Some(format!("the path stored `{text}` at line {line}"));
                }
                Some(_) => {}
            }
        }
        if let Some((line, _)) = self.assigned.iter().find(|(_, var)| var == id) {
            return Some(format!("the path assigned `{name}` at line {line}"));
        }
        for (line, callee, arguments) in &self.calls {
            if let Some(argument) = arguments.iter().find(|a| mentions(a, name)) {
                return Some(format!(
                    "the call `{callee}` at line {line} received `{argument}`"
                ));
            }
        }
        None
    }

    /// Where a lookup argument is rooted: the variable its place or address
    /// expression starts from.
    fn argument_root(&self, node: &Value) -> Rooted {
        use qcore_core::null_lookup::Root;
        let node = strip(node);
        match kind(node) {
            "ImplicitCastExpr" | "CStyleCastExpr" | "MemberExpr" => match kids(node).next() {
                Some(inner) => self.argument_root(inner),
                None => Rooted::Unrooted(kind(node).to_owned()),
            },
            "UnaryOperator" if matches!(node["opcode"].as_str(), Some("&" | "*")) => {
                match kids(node).next() {
                    Some(inner) => self.argument_root(inner),
                    None => Rooted::Unrooted("an operator".into()),
                }
            }
            "ArraySubscriptExpr" => {
                let parts = kids(node).collect::<Vec<_>>();
                match (parts.first(), parts.get(1)) {
                    (Some(base), Some(index)) if literal(index).is_some() => {
                        self.argument_root(base)
                    }
                    _ => Rooted::Unrooted(format!(
                        "`{}`, an element at an index the search does not fix",
                        render(node)
                    )),
                }
            }
            "IntegerLiteral"
            | "CharacterLiteral"
            | "FloatingLiteral"
            | "StringLiteral"
            | "PredefinedExpr"
            | "UnaryExprOrTypeTraitExpr"
            | "OffsetOfExpr" => Rooted::Constant,
            "DeclRefExpr" => {
                let decl = &node["referencedDecl"];
                match decl["kind"].as_str() {
                    Some("EnumConstantDecl" | "FunctionDecl") => Rooted::Constant,
                    Some("ParmVarDecl" | "VarDecl") => {
                        let id = decl["id"].as_str().unwrap_or("").to_owned();
                        let root = if self.formal_ids.contains(&id) {
                            Root::Formal
                        } else if self.env.unit.globals.contains(&id) {
                            Root::Global
                        } else {
                            Root::Local
                        };
                        Rooted::Decl {
                            root,
                            id,
                            name: decl["name"].as_str().unwrap_or("?").to_owned(),
                            pointer: is_pointer(decl["t"].as_str().unwrap_or("")),
                        }
                    }
                    _ => Rooted::Unrooted(format!("`{}`", render(node))),
                }
            }
            other => Rooted::Unrooted(format!("`{}` ({other})", render(node))),
        }
    }

    // -------------------------------------------------------------------
    // Conditions
    // -------------------------------------------------------------------

    /// Record a decision taken on the path.
    fn note(&mut self, node: &Value, arm: bool) {
        let line = self.line(node);
        self.path.push(PathStep {
            line,
            step: "decision",
            text: format!(
                "`{}` is {}",
                render(node),
                if arm { "true" } else { "false" }
            ),
        });
    }

    /// The truth of a value already evaluated at `node` (a pointer or an
    /// integer in a boolean context).
    fn truth(&mut self, node: &Value, value: &V) -> R<bool> {
        match value {
            V::Ptr(p, id) => self.pointer_test(node, p.clone(), *id),
            V::Int(s) => self.integer_test(node, s.clone(), "!=", S::Const(0)),
        }
    }

    fn pointer_test(&mut self, node: &Value, p: P, id: usize) -> R<bool> {
        let line = self.line(node);
        match p {
            P::Null(_) => Ok(false),
            P::NonNull => Ok(true),
            P::Uninit => Err(End::Unresolved("a test of an uninitialised pointer".into())),
            P::Opaque(origin, constrained) => {
                // Card NULL-WITNESS-GUARD-CORRELATION (rule 39): a memory
                // value tested through an alias is a read a lookup's state
                // may depend on.
                if let Origin::Memory(text) = &origin {
                    self.decided_reads.insert(text.clone());
                }
                let arm = if let Origin::Refused(name, at, reason) = &origin {
                    // Rule 39: a refused lookup's result is tested inexactly.
                    self.inexact_take(
                        node,
                        format!(
                            "a test at line {line} of the lookup `{name}`'s result (line {at}), whose miss is not a NULL event on this path (rule 39): {reason}"
                        ),
                    )?
                } else if constrained {
                    // Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15r): explored
                    // on both arms, inexactly.
                    self.inexact_take(
                        node,
                        format!(
                            "a correlated test at line {line}: `{}` was tested before a call on the path",
                            render(node)
                        ),
                    )?
                } else {
                    let prefer = self.steer.unwrap_or(false);
                    let arm = self.decide(prefer)?;
                    self.note(node, arm);
                    arm
                };
                if arm {
                    self.set_ident(id, &P::NonNull);
                } else {
                    let what = render(node);
                    let (allocation, condition) = match &origin {
                        Origin::Allocation(name, at) => (
                            true,
                            format!(
                                "the allocation `{name}` at line {at} fails, as the test at line {line} admits"
                            ),
                        ),
                        Origin::Lookup(name, at) => (
                            false,
                            format!(
                                "the lookup `{name}` at line {at} misses, as the test at line {line} admits"
                            ),
                        ),
                        Origin::Formal(formal) => (
                            false,
                            format!(
                                "the formal `{formal}` is NULL at entry, as the function's own test at line {line} admits"
                            ),
                        ),
                        Origin::Memory(text) => (
                            false,
                            format!("`{text}` reads NULL, as the test at line {line} admits"),
                        ),
                        Origin::Uncontracted(name) => (
                            false,
                            format!("`{name}` returns NULL, as the test at line {line} admits"),
                        ),
                        Origin::Other(other) => (
                            false,
                            format!(
                                "`{what}` ({other}) is NULL, as the test at line {line} admits"
                            ),
                        ),
                        // Rule 39: reached only along the inexact arm above,
                        // so never a witness.
                        Origin::Refused(name, at, _) => (
                            false,
                            format!(
                                "the lookup `{name}` at line {at} misses, as the test at line {line} admits (an inexact arm, rule 39)"
                            ),
                        ),
                    };
                    if allocation {
                        self.needs_allocation_failure = true;
                    }
                    self.set_ident(
                        id,
                        &P::Null(NullWhy {
                            allocation,
                            condition,
                            line,
                        }),
                    );
                }
                self.decided_texts.insert(render(strip(node)));
                Ok(arm)
            }
        }
    }

    /// The branch arm a path takes for the condition `node`.
    fn test(&mut self, node: &Value) -> R<bool> {
        self.tick()?;
        let node = strip(node);
        if is_pointer(ty(node)) {
            let value = self.eval(node)?;
            return self.truth(node, &value);
        }
        match kind(node) {
            "UnaryOperator" if node["opcode"] == "!" => {
                let Some(operand) = kids(node).next() else {
                    return Err(End::Unresolved("a negation".into()));
                };
                Ok(!self.test(operand)?)
            }
            "BinaryOperator" => {
                let mut parts = kids(node);
                let (Some(lhs), Some(rhs)) = (parts.next(), parts.next()) else {
                    return Err(End::Unresolved("a condition".into()));
                };
                let op = node["opcode"].as_str().unwrap_or("");
                match op {
                    "&&" => Ok(self.test(lhs)? && self.test(rhs)?),
                    "||" => Ok(self.test(lhs)? || self.test(rhs)?),
                    "," => {
                        self.eval(lhs)?;
                        self.test(rhs)
                    }
                    "==" | "!=" | "<" | "<=" | ">" | ">=" => {
                        let left = self.eval(lhs)?;
                        let right = self.eval(rhs)?;
                        match (left, right) {
                            (V::Ptr(p, id), V::Ptr(q, qid)) => {
                                let null_left = matches!(p, P::Null(_));
                                let null_right = matches!(q, P::Null(_));
                                if matches!(op, "==" | "!=") && (null_left || null_right) {
                                    let (tested, tid, at) = if null_right {
                                        (p, id, lhs)
                                    } else {
                                        (q, qid, rhs)
                                    };
                                    let non_null = self.pointer_test(at, tested, tid)?;
                                    return Ok(if op == "!=" { non_null } else { !non_null });
                                }
                                self.pointer_compare(node, lhs, rhs, op, (p, id), (q, qid))
                            }
                            (V::Int(a), V::Int(b)) => self.integer_test(node, a, op, b),
                            (V::Ptr(p, id), V::Int(S::Const(0))) if matches!(op, "==" | "!=") => {
                                let non_null = self.pointer_test(lhs, p, id)?;
                                Ok(if op == "!=" { non_null } else { !non_null })
                            }
                            (V::Int(S::Const(0)), V::Ptr(q, qid)) if matches!(op, "==" | "!=") => {
                                let non_null = self.pointer_test(rhs, q, qid)?;
                                Ok(if op == "!=" { non_null } else { !non_null })
                            }
                            _ => Err(End::Unresolved(format!(
                                "a comparison the search does not read at line {}",
                                self.line(node)
                            ))),
                        }
                    }
                    _ => {
                        let value = self.eval(node)?;
                        self.truth(node, &value)
                    }
                }
            }
            "ConditionalOperator" => {
                let parts = kids(node).collect::<Vec<_>>();
                let [condition, yes, no] = parts.as_slice() else {
                    return Err(End::Unresolved("a conditional operator".into()));
                };
                if self.test(condition)? {
                    self.test(yes)
                } else {
                    self.test(no)
                }
            }
            "ImplicitCastExpr" | "CStyleCastExpr"
                if matches!(
                    node["castKind"].as_str(),
                    Some("IntegralCast" | "IntegralToBoolean" | "NoOp" | "PointerToBoolean")
                ) =>
            {
                match kids(node).next() {
                    Some(operand) => self.test(operand),
                    None => Err(End::Unresolved("a cast".into())),
                }
            }
            "CallExpr"
                if matches!(
                    callee_name(node).as_deref(),
                    Some("__builtin_expect" | "__builtin_expect_with_probability")
                ) =>
            {
                let parts = kids(node).collect::<Vec<_>>();
                let Some(first) = parts.get(1) else {
                    return Err(End::Unresolved("an expectation".into()));
                };
                let arm = self.test(first)?;
                for rest in parts.iter().skip(2) {
                    self.eval(rest)?;
                }
                Ok(arm)
            }
            _ => {
                let value = self.eval(node)?;
                self.truth(node, &value)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn pointer_compare(
        &mut self,
        node: &Value,
        lhs: &Value,
        rhs: &Value,
        op: &str,
        (p, pid): (P, usize),
        (q, qid): (P, usize),
    ) -> R<bool> {
        let line = self.line(node);
        let (left, right) = (render(strip(lhs)), render(strip(rhs)));
        if left == right && matches!(op, "==" | "!=" | "<=" | ">=" | "<" | ">") {
            return Ok(matches!(op, "==" | "<=" | ">="));
        }
        if !matches!(op, "==" | "!=") {
            // Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15r): explored on both
            // arms, inexactly (no same-object reading exists here).
            return self.inexact_take(
                node,
                format!("an ordered pointer comparison at line {line}"),
            );
        }
        if matches!(p, P::Uninit) || matches!(q, P::Uninit) {
            return Err(End::Unresolved(format!(
                "a comparison of an uninitialised pointer at line {line}"
            )));
        }
        if matches!(p, P::Opaque(_, true)) || matches!(q, P::Opaque(_, true)) {
            return self.inexact_take(
                node,
                format!("a correlated pointer comparison at line {line}"),
            );
        }
        // A side's identity: its value (a read of a variable or of memory)
        // or, for an address it forms afresh, its spelling.
        let identity = |side: &Value, id: usize| {
            let side = strip(side);
            let formed = (kind(side) == "UnaryOperator" && side["opcode"] == "&")
                || matches!(kind(side), "StringLiteral" | "PredefinedExpr");
            if formed {
                render(side)
            } else {
                format!("{}#{id}", render(side))
            }
        };
        let key = format!("{} {op} {}", identity(lhs, pid), identity(rhs, qid));
        if let Some((_, arm)) = self.decided.get(&key) {
            return Ok(*arm);
        }
        // A value compared before with a different value: correlated.
        let compared = |side: &str| {
            self.decided
                .keys()
                .any(|k| k.contains(" == ") || k.contains(" != "))
                && self.decided.keys().any(|k| k.split(' ').any(|w| w == side))
        };
        let (lk, rk) = (identity(lhs, pid), identity(rhs, qid));
        if (lk.contains('#') && compared(&lk)) || (rk.contains('#') && compared(&rk)) {
            let arm = self.inexact_take(
                node,
                format!("a correlated pointer comparison at line {line}"),
            )?;
            self.decided.insert(key, (Vec::new(), arm));
            return Ok(arm);
        }
        // Two distinct, unconstrained pointer values may be equal or not.
        let prefer = self.steer.unwrap_or(false);
        let arm = self.decide(prefer)?;
        self.note(node, arm);
        self.decided.insert(key, (Vec::new(), arm));
        self.decided_texts.insert(left);
        self.decided_texts.insert(right);
        Ok(arm)
    }

    /// The atoms an integer value reads (empty for a constant).
    fn atoms_of(s: &S) -> Option<Vec<usize>> {
        match s {
            S::Const(_) => Some(Vec::new()),
            S::Atom(a) | S::Off(a, _) => Some(vec![*a]),
            S::Sum(a, b) => Some(vec![*a, *b]),
            S::Opaque(_) => None,
        }
    }

    /// What an unfollowed side is, for the reason.
    fn describe(a: &S, b: &S) -> String {
        [a, b]
            .iter()
            .filter_map(|s| match s {
                S::Opaque(what) => Some(what.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("; ")
    }

    fn integer_test(&mut self, node: &Value, a: S, op: &str, b: S) -> R<bool> {
        let line = self.line(node);
        if let (S::Const(x), S::Const(y)) = (&a, &b) {
            return compare(op, *x, *y)
                .ok_or_else(|| End::Unresolved("an unread comparison".into()));
        }
        if !matches!(op, "==" | "!=" | "<" | "<=" | ">" | ">=") {
            return Err(End::Unresolved(format!(
                "an unread comparison at line {line}"
            )));
        }
        // Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15r): a value the search
        // does not follow is explored on both arms, inexactly.
        let (Some(mut atoms), Some(more)) = (Self::atoms_of(&a), Self::atoms_of(&b)) else {
            let what = Self::describe(&a, &b);
            for atom in Self::atoms_of(&a)
                .into_iter()
                .chain(Self::atoms_of(&b))
                .flatten()
            {
                self.atoms[atom].constrained = true;
            }
            return self.inexact_take(
                node,
                format!(
                    "a condition over a value the search does not follow at line {line}: `{}` ({what})",
                    render(node)
                ),
            );
        };
        atoms.extend(more);
        atoms.sort_unstable();
        atoms.dedup();
        let key = render(node);
        if let Some((seen, arm)) = self.decided.get(&key)
            && *seen == atoms
        {
            return Ok(*arm);
        }
        for side in kids(node) {
            self.decided_texts.insert(render(strip(side)));
        }
        self.decided_texts.insert(render(strip(node)));
        if atoms.iter().any(|atom| self.atoms[*atom].constrained) {
            let arm = self.inexact_take(
                node,
                format!(
                    "a correlated condition at line {line}: `{key}` reads a value an earlier decision constrained"
                ),
            )?;
            self.decided.insert(key, (atoms, arm));
            return Ok(arm);
        }
        // One linear value against a constant: `x + k OP c` is `x OP c - k`.
        let single = match (&a, &b) {
            (x, S::Const(c)) => lin(x).map(|(atom, k)| (atom, op, c.checked_sub(k))),
            (S::Const(c), x) => lin(x).map(|(atom, k)| (atom, flip(op), c.checked_sub(k))),
            _ => None,
        };
        if let Some((x, op, c)) = single {
            let arms = c.and_then(|c| Some((arm_set(op, c, true)?, arm_set(op, c, false)?)));
            let Some((on_true, on_false)) = arms else {
                return Err(End::Unresolved(format!(
                    "an unread comparison at line {line}"
                )));
            };
            if self.atoms[x].range.is_none() {
                return self.inexact_take(
                    node,
                    format!("a comparison over a value of unknown range at line {line}"),
                );
            }
            let can_true = self.atoms[x].admits(on_true);
            let can_false = self.atoms[x].admits(on_false);
            let arm = match (&self.atoms[x].call, can_true && can_false) {
                // Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15q): a call's
                // result is bounded, never free: an arm its bound does not
                // decide is inexact.
                (Some(callee), true) => {
                    let callee = callee.clone();
                    self.inexact_take(
                        node,
                        format!(
                            "a condition over the result of `{callee}` at line {line}: its range is a bound, not a range the callee is known to return"
                        ),
                    )?
                }
                _ => self.take(node, can_true, can_false)?,
            };
            self.atoms[x].narrow(if arm { on_true } else { on_false });
            return Ok(arm);
        }
        // The same atom on both sides: `x + k1 OP x + k2`.
        if let (Some((x, k1)), Some((y, k2))) = (lin(&a), lin(&b))
            && x == y
        {
            return compare(op, k1, k2)
                .ok_or_else(|| End::Unresolved("an unread comparison".into()));
        }
        // Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15s): two independent
        // values over hole-free intervals: `x + k1 OP y + k2` is
        // `x - y OP k2 - k1`, and `a + b OP c` a sum; an arm is taken only
        // when the difference (sum) interval admits it.
        let relation = match (lin(&a), lin(&b), &a, &b) {
            (Some((x, k1)), Some((y, k2)), _, _) => {
                k2.checked_sub(k1).map(|c| (x, y, false, op, c))
            }
            (_, _, S::Sum(x, y), S::Const(c)) => Some((*x, *y, true, op, *c)),
            (_, _, S::Const(c), S::Sum(x, y)) => Some((*x, *y, true, flip(op), *c)),
            _ => None,
        };
        let exact = relation.and_then(|(x, y, sum, op, c)| {
            let (xi, yi) = (&self.atoms[x], &self.atoms[y]);
            if xi.call.is_some()
                || yi.call.is_some()
                || !xi.holes.is_empty()
                || !yi.holes.is_empty()
            {
                return None;
            }
            qcore_core::null_paths::relation_arms(xi.range?, yi.range?, sum, op, c)
        });
        let arm = match exact {
            Some((can_true, can_false)) => self.take(node, can_true, can_false)?,
            None => self.inexact_take(
                node,
                format!(
                    "a relation at line {line} the search does not read exactly: `{key}` (a call's result, an excluded value or a value of unknown range)"
                ),
            )?,
        };
        for atom in &atoms {
            self.atoms[*atom].constrained = true;
        }
        self.decided.insert(key, (atoms, arm));
        Ok(arm)
    }

    /// The arm a path takes when `can_true` and `can_false` say which are
    /// feasible: a decided condition its one arm, a free one a decision.
    fn take(&mut self, node: &Value, can_true: bool, can_false: bool) -> R<bool> {
        let condition = match (can_true, can_false) {
            (true, true) => Condition::Free,
            (true, false) => Condition::Decided(true),
            (false, true) => Condition::Decided(false),
            (false, false) => return Err(End::Ended),
        };
        let arm = match condition {
            Condition::Decided(value) => value,
            _ => {
                let prefer = self.steer.unwrap_or(false);
                let arm = self.decide(prefer)?;
                self.note(node, arm);
                arm
            }
        };
        debug_assert_eq!(
            qcore_core::null_triage::admit(condition, arm),
            qcore_core::null_triage::Admission::Taken
        );
        Ok(arm)
    }

    // -------------------------------------------------------------------
    // Statements
    // -------------------------------------------------------------------

    fn block(&mut self, children: &[&Value], from: usize) -> R<Flow> {
        let mut index = from;
        while index < children.len() {
            match self.stmt(children[index])? {
                Flow::Normal => index += 1,
                Flow::Goto(label) => match find_label(children, &label) {
                    Some(at) => index = at,
                    None => return Ok(Flow::Goto(label)),
                },
                other => return Ok(other),
            }
        }
        Ok(Flow::Normal)
    }

    fn stmt(&mut self, node: &Value) -> R<Flow> {
        self.tick()?;
        match kind(node) {
            "CompoundStmt" => {
                let children = kids(node).collect::<Vec<_>>();
                self.block(&children, 0)
            }
            "DeclStmt" => {
                for decl in kids(node) {
                    if kind(decl) != "VarDecl" {
                        continue;
                    }
                    let id = decl["id"].as_str().unwrap_or("").to_owned();
                    let init = kids(decl).find(|c| !kind(c).ends_with("Attr") && present(c));
                    let static_storage =
                        matches!(decl["storageClass"].as_str(), Some("static" | "extern"));
                    if static_storage {
                        continue;
                    }
                    match init {
                        Some(init) => {
                            let value = self.eval(init)?;
                            if self.tracked.contains_key(&id) {
                                self.forget_mentions(&id);
                                self.vars.insert(id, value);
                            } else {
                                let name = decl["name"].as_str().unwrap_or("?").to_owned();
                                self.memory.insert(name, value);
                            }
                        }
                        None => {
                            self.forget_mentions(&id);
                            self.vars.remove(&id);
                        }
                    }
                }
                Ok(Flow::Normal)
            }
            "NullStmt" => Ok(Flow::Normal),
            "IfStmt" => {
                let parts = kids(node).filter(|c| present(c)).collect::<Vec<_>>();
                let (Some(cond), Some(then)) = (parts.first(), parts.get(1)) else {
                    return Err(End::Unresolved("an if statement".into()));
                };
                let otherwise = parts.get(2);
                let arm = self.test_toward(cond, then, otherwise.copied())?;
                if arm {
                    self.stmt(then)
                } else {
                    match otherwise {
                        Some(otherwise) => self.stmt(otherwise),
                        None => Ok(Flow::Normal),
                    }
                }
            }
            "WhileStmt" => {
                let parts = kids(node).filter(|c| present(c)).collect::<Vec<_>>();
                let (Some(cond), Some(body)) = (parts.first(), parts.last()) else {
                    return Err(End::Unresolved("a while statement".into()));
                };
                self.loop_stmt(Some(cond), body, None, false)
            }
            "DoStmt" => {
                let parts = kids(node).collect::<Vec<_>>();
                let (Some(body), Some(cond)) = (parts.first(), parts.get(1)) else {
                    return Err(End::Unresolved("a do statement".into()));
                };
                self.loop_stmt(Some(cond), body, None, true)
            }
            "ForStmt" => {
                let parts = kids(node).collect::<Vec<_>>();
                if parts.len() != 5 {
                    return Err(End::Unresolved("a for statement".into()));
                }
                if present(parts[0]) {
                    match self.stmt(parts[0])? {
                        Flow::Normal => {}
                        other => return Ok(other),
                    }
                }
                let cond = present(parts[2]).then_some(parts[2]);
                let inc = present(parts[3]).then_some(parts[3]);
                self.loop_stmt(cond, parts[4], inc, false)
            }
            "SwitchStmt" => self.switch_stmt(node),
            "CaseStmt" | "DefaultStmt" => {
                // Reached by falling through: the label is no jump.
                match kids(node).rfind(|c| present(c)) {
                    Some(sub)
                        if kind(node) == "DefaultStmt"
                            || kids(node).filter(|c| present(c)).count() > 1 =>
                    {
                        self.stmt(sub)
                    }
                    _ => Ok(Flow::Normal),
                }
            }
            "BreakStmt" => Ok(Flow::Break),
            "ContinueStmt" => Ok(Flow::Continue),
            "ReturnStmt" => {
                if let Some(value) = kids(node).next() {
                    self.eval(value)?;
                }
                Ok(Flow::Return)
            }
            "LabelStmt" => match kids(node).next() {
                Some(sub) => self.stmt(sub),
                None => Ok(Flow::Normal),
            },
            "GotoStmt" => Ok(Flow::Goto(
                node["targetLabelDeclId"].as_str().unwrap_or("").to_owned(),
            )),
            "AttributedStmt" => {
                let children = kids(node)
                    .filter(|c| !kind(c).ends_with("Attr"))
                    .collect::<Vec<_>>();
                self.block(&children, 0)
            }
            "GCCAsmStmt" | "MSAsmStmt" => {
                for child in kids(node) {
                    self.eval(child)?;
                }
                // Rule 39: an asm statement may write any place.
                let line = self.line(node);
                self.stores.push((line, None));
                self.forget_memory();
                Ok(Flow::Normal)
            }
            "IndirectGotoStmt" => Err(End::Unresolved("an indirect goto".into())),
            _ => {
                self.eval(node)?;
                Ok(Flow::Normal)
            }
        }
    }

    /// A branch condition, preferring the arm that holds the target.
    fn test_toward(&mut self, cond: &Value, then: &Value, otherwise: Option<&Value>) -> R<bool> {
        // The preference is read by `decide` through `holds_target` of the
        // condition itself; steer it here with the arms.
        let toward = if self.holds_target(then) {
            Some(true)
        } else if otherwise.is_some_and(|o| self.holds_target(o)) {
            Some(false)
        } else {
            None
        };
        self.steer = toward;
        let arm = self.test(cond);
        self.steer = None;
        arm
    }

    fn loop_stmt(
        &mut self,
        cond: Option<&Value>,
        body: &Value,
        inc: Option<&Value>,
        body_first: bool,
    ) -> R<Flow> {
        let mut first = true;
        for _ in 0..LOOP_BOUND {
            if !(body_first && first)
                && let Some(cond) = cond
            {
                self.steer = Some(self.holds_target(body));
                let arm = self.test(cond);
                self.steer = None;
                if !arm? {
                    return Ok(Flow::Normal);
                }
            }
            first = false;
            match self.stmt(body)? {
                Flow::Normal | Flow::Continue => {}
                Flow::Break => return Ok(Flow::Normal),
                other => return Ok(other),
            }
            if let Some(inc) = inc {
                self.eval(inc)?;
            }
        }
        Err(End::Unresolved("a loop's iteration bound".into()))
    }

    fn switch_stmt(&mut self, node: &Value) -> R<Flow> {
        let parts = kids(node).filter(|c| present(c)).collect::<Vec<_>>();
        let (Some(cond), Some(body)) = (parts.first(), parts.last()) else {
            return Err(End::Unresolved("a switch statement".into()));
        };
        let value = self.eval(cond)?;
        let line = self.line(cond);
        if kind(body) != "CompoundStmt" {
            return Err(End::Unresolved("a switch whose body is not a block".into()));
        }
        let children = kids(body).collect::<Vec<_>>();
        // Each top-level child's case values (a chain `case A: case B: s`).
        let mut cases: Vec<(usize, i128)> = Vec::new();
        let mut default = None;
        for (index, child) in children.iter().enumerate() {
            let mut label = *child;
            loop {
                match kind(label) {
                    "CaseStmt" => {
                        let present_kids = kids(label).filter(|c| present(c)).collect::<Vec<_>>();
                        if present_kids.len() != 2 {
                            return Err(End::Unresolved("a case range".into()));
                        }
                        let Some(k) = literal(present_kids[0]).or_else(|| {
                            ast::constant_value(present_kids[0], &self.env.unit.enum_values)
                        }) else {
                            return Err(End::Unresolved("a case value".into()));
                        };
                        cases.push((index, k));
                        label = present_kids[1];
                    }
                    "DefaultStmt" => {
                        default = Some(index);
                        match kids(label).rfind(|c| present(c)) {
                            Some(sub) => label = sub,
                            None => break,
                        }
                    }
                    _ => break,
                }
            }
        }
        if nested_label(body) {
            return Err(End::Unresolved(
                "a case label inside a nested statement".into(),
            ));
        }
        let start = match value {
            V::Int(S::Const(k)) => cases
                .iter()
                .find(|(_, case)| *case == k)
                .map(|(index, _)| *index)
                .or(default),
            V::Int(s) => {
                // Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15r): a linear value
                // of an unconstrained entry atom is read exactly (case `k`
                // is `atom == k - offset`); any other value (a call's result
                // its bound does not decide, a correlated or unfollowed
                // value) is explored case by case, inexactly.
                let linear = lin(&s).filter(|(atom, _)| self.atoms[*atom].range.is_some());
                let exact = linear.is_some_and(|(atom, _)| {
                    !self.atoms[atom].constrained && self.atoms[atom].call.is_none()
                });
                let why = match (&s, linear) {
                    (_, Some((atom, _))) if self.atoms[atom].constrained => {
                        format!("a correlated switch at line {line}")
                    }
                    (_, Some((atom, _))) => format!(
                        "a switch over the result of `{}` at line {line}: its range is a bound",
                        self.atoms[atom].call.clone().unwrap_or_default()
                    ),
                    (S::Opaque(what), None) => format!(
                        "a switch over a value the search does not follow at line {line} ({what})"
                    ),
                    _ => format!("a switch over a value the search does not follow at line {line}"),
                };
                let mut chosen = None;
                for (index, k) in &cases {
                    let (can_take, can_pass) = match linear {
                        Some((atom, off)) => match k.checked_sub(off) {
                            Some(at) => (
                                self.atoms[atom].admits(ArmSet::Interval(at, at)),
                                self.atoms[atom].admits(ArmSet::Except(at)),
                            ),
                            None => (false, true),
                        },
                        None => (true, true),
                    };
                    let take = match (can_take, can_pass) {
                        (false, _) => false,
                        (true, false) => true,
                        (true, true) if exact => {
                            let prefer = self.holds_target(children[*index]);
                            self.decide(prefer)?
                        }
                        (true, true) => {
                            if !self.explore_inexact {
                                return Err(End::Unresolved(why));
                            }
                            let prefer = self.holds_target(children[*index]);
                            let take = self.decide(prefer)?;
                            self.inexact.get_or_insert(why.clone());
                            take
                        }
                    };
                    if let Some((atom, off)) = linear
                        && let Some(at) = k.checked_sub(off)
                    {
                        if take {
                            self.atoms[atom].narrow(ArmSet::Interval(at, at));
                        } else {
                            self.atoms[atom].narrow(ArmSet::Except(at));
                        }
                    }
                    if take {
                        self.path.push(PathStep {
                            line,
                            step: "decision",
                            text: format!("`{}` is {k}", render(cond)),
                        });
                        if let Some(id) = self.var_id(cond_operand(cond)) {
                            self.vars.insert(id, V::Int(S::Const(*k)));
                        } else if linear.is_some_and(|(_, off)| off == 0) {
                            let text = render(strip(cond_operand(cond)));
                            self.memory.insert(text.clone(), V::Int(S::Const(*k)));
                            self.decided_texts.insert(text);
                        }
                        chosen = Some(*index);
                        break;
                    }
                }
                if chosen.is_none() {
                    self.path.push(PathStep {
                        line,
                        step: "decision",
                        text: format!("`{}` matches no case", render(cond)),
                    });
                }
                self.decided_texts.insert(render(strip(cond)));
                chosen.or(default)
            }
            V::Ptr(..) => {
                return Err(End::Unresolved(format!(
                    "a switch over a value the search does not follow at line {line}"
                )));
            }
        };
        let Some(start) = start else {
            return Ok(Flow::Normal);
        };
        match self.block_from_label(&children, start)? {
            Flow::Break | Flow::Normal => Ok(Flow::Normal),
            other => Ok(other),
        }
    }

    /// Run the switch body from a labelled child (its labels are no jumps).
    fn block_from_label(&mut self, children: &[&Value], start: usize) -> R<Flow> {
        self.block(children, start)
    }
}

/// The operand under value-preserving casts.
fn cond_operand(node: &Value) -> &Value {
    let mut node = strip(node);
    while kind(node) == "ImplicitCastExpr"
        && matches!(
            node["castKind"].as_str(),
            Some("LValueToRValue" | "IntegralCast" | "NoOp")
        )
    {
        match kids(node).next() {
            Some(inner) => node = strip(inner),
            None => break,
        }
    }
    node
}

impl Run<'_, '_> {
    /// Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15s): `x + k` of a linear
    /// value `x`, exact only when every value of `x`'s range plus `k` lies
    /// in the expression's type (no wrap, no overflow).
    fn offset(&self, x: &S, k: Option<i128>, spelled: &str, node: &Value) -> S {
        let opaque = || S::Opaque(format!("an expression `{}`", render(node)));
        let (Some((atom, base)), Some(k)) = (lin(x), k) else {
            return opaque();
        };
        let Some(total) = base.checked_add(k) else {
            return opaque();
        };
        let (Some((lo, hi)), Some((tlo, thi))) = (self.atoms[atom].range, self.int_range(spelled))
        else {
            return opaque();
        };
        match (lo.checked_add(total), hi.checked_add(total)) {
            (Some(a), Some(b)) if tlo <= a && b <= thi => {
                if total == 0 {
                    S::Atom(atom)
                } else {
                    S::Off(atom, total)
                }
            }
            _ => S::Opaque(format!(
                "an expression `{}` that may wrap or overflow",
                render(node)
            )),
        }
    }

    /// Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15s): `a + b` of two atoms,
    /// exact only when the sum interval lies in the expression's type.
    fn sum(&self, a: usize, b: usize, spelled: &str, node: &Value) -> S {
        let (Some((alo, ahi)), Some((blo, bhi)), Some((tlo, thi))) = (
            self.atoms[a].range,
            self.atoms[b].range,
            self.int_range(spelled),
        ) else {
            return S::Opaque(format!("an expression `{}`", render(node)));
        };
        match (alo.checked_add(blo), ahi.checked_add(bhi)) {
            (Some(lo), Some(hi)) if tlo <= lo && hi <= thi => S::Sum(a.min(b), a.max(b)),
            _ => S::Opaque(format!(
                "an expression `{}` that may overflow",
                render(node)
            )),
        }
    }
}

fn arith(op: &str, a: i128, b: i128) -> Option<i128> {
    match op {
        "+" => a.checked_add(b),
        "-" => a.checked_sub(b),
        "*" => a.checked_mul(b),
        "/" => (b != 0).then(|| a / b),
        "%" => (b != 0).then(|| a % b),
        "|" => Some(a | b),
        "&" => Some(a & b),
        "^" => Some(a ^ b),
        "<<" => u32::try_from(b).ok().and_then(|s| a.checked_shl(s)),
        ">>" => u32::try_from(b).ok().and_then(|s| a.checked_shr(s)),
        "==" | "!=" | "<" | "<=" | ">" | ">=" => compare(op, a, b).map(i128::from),
        "&&" => Some(i128::from(a != 0 && b != 0)),
        "||" => Some(i128::from(a != 0 || b != 0)),
        _ => None,
    }
}

fn offset_of(value: V) -> V {
    match value {
        V::Ptr(P::Null(_) | P::Uninit, id) => {
            V::Ptr(P::Opaque(Origin::Other("offset".into()), true), id)
        }
        other => other,
    }
}

/// The index of the top-level child labelled `label` (through label chains).
fn find_label(children: &[&Value], label: &str) -> Option<usize> {
    children.iter().position(|child| {
        let mut node = *child;
        while kind(node) == "LabelStmt" {
            if node["declId"].as_str() == Some(label) {
                return true;
            }
            match kids(node).next() {
                Some(sub) => node = sub,
                None => break,
            }
        }
        false
    })
}

/// Whether a case or default label sits below a statement other than a
/// label chain (the search reads only top-level labels).
fn nested_label(body: &Value) -> bool {
    fn below(node: &Value) -> bool {
        match kind(node) {
            "SwitchStmt" => false,
            "CaseStmt" | "DefaultStmt" => true,
            _ => kids(node).any(below),
        }
    }
    kids(body).any(|child| {
        let mut node = child;
        while matches!(kind(node), "CaseStmt" | "DefaultStmt" | "LabelStmt") {
            match kids(node).rfind(|c| present(c)) {
                Some(sub) => node = sub,
                None => return false,
            }
        }
        below(node)
    })
}

/// Every node on the way down to the target, by address.
fn ancestors(node: &Value, target: &str, out: &mut BTreeSet<*const Value>) -> bool {
    let mut found = node_key(node) == target;
    for child in kids(node) {
        if ancestors(child, target, out) {
            found = true;
        }
    }
    if found {
        out.insert(std::ptr::from_ref(node));
    }
    found
}

/// Search the body of `definition` for a path reaching the check whose node
/// key is `target` with its pointer NULL under a named condition.
pub(super) fn search(
    env: &Env<'_>,
    function: &str,
    definition: &Value,
    entries: &BTreeMap<usize, Entry>,
    target: &str,
) -> Search {
    let unresolved_without = |reason: &str| Search {
        triage: Triage::Unresolved,
        reason: reason.into(),
        witness: None,
        runs: 0,
        consulted: BTreeSet::new(),
    };
    let Some(body) = kids(definition).find(|c| kind(c) == "CompoundStmt") else {
        return unresolved_without("the definition has no body");
    };
    let mut contains = BTreeSet::new();
    if !ancestors(body, target, &mut contains) {
        return unresolved_without("the dereference is not in the body");
    }
    let tracked = tracked_variables(definition, body, &env.unit.enum_typedefs);
    let context = Context {
        env,
        function,
        definition,
        body,
        entries,
        target,
        contains: &contains,
        tracked: &tracked,
    };
    // Phase 1 (rule 14r as amended by rule 15q): exact paths only; an
    // inexact condition ends its path unresolved.
    let exact = phase(&context, false);
    if let Some(found) = exact.finding {
        return found;
    }
    if let Some(reason) = exact.indeterminate {
        return Search {
            triage: Triage::Indeterminate,
            reason,
            witness: None,
            runs: exact.runs,
            consulted: exact.consulted,
        };
    }
    if exact.unresolved.is_none() {
        // Every exact path ended without reaching it NULL.
        return Search {
            triage: Triage::Infeasible,
            reason: exact.infeasible_reason(),
            witness: None,
            runs: exact.runs,
            consulted: exact.consulted,
        };
    }
    // Phase 2 (rule 15r): inexact arms explored on both sides; only an
    // infeasibility proof is drawn from it.
    let explored = phase(&context, true);
    let runs = exact.runs + explored.runs;
    // An exact path the first phase's budget did not reach is still a
    // witness (its arms are the same in both phases).
    if let Some(mut found) = explored.finding {
        found.runs = runs;
        return found;
    }
    if let Some(reason) = explored.indeterminate {
        return Search {
            triage: Triage::Indeterminate,
            reason,
            witness: None,
            runs,
            consulted: explored.consulted,
        };
    }
    match qcore_core::null_paths::conclude(explored.explored) {
        "infeasible" => Search {
            triage: Triage::Infeasible,
            reason: explored.infeasible_reason(),
            witness: None,
            runs,
            consulted: explored.consulted,
        },
        _ => Search {
            triage: Triage::Unresolved,
            reason: explored
                .inexact_reach
                .or(exact.unresolved)
                .unwrap_or_else(|| "the search did not conclude".into()),
            witness: None,
            runs,
            consulted: BTreeSet::new(),
        },
    }
}

/// What one search reads, for both phases.
struct Context<'c, 'e> {
    env: &'c Env<'e>,
    function: &'c str,
    definition: &'c Value,
    body: &'c Value,
    entries: &'c BTreeMap<usize, Entry>,
    target: &'c str,
    contains: &'c BTreeSet<*const Value>,
    tracked: &'c BTreeMap<String, String>,
}

/// One phase's result.
#[derive(Default)]
struct Phase {
    finding: Option<Search>,
    indeterminate: Option<String>,
    unresolved: Option<String>,
    inexact_reach: Option<String>,
    explored: qcore_core::null_paths::Explored,
    consulted: BTreeSet<String>,
    runs: usize,
    ended: usize,
    faulted: usize,
    inexact_paths: usize,
}

impl Phase {
    fn infeasible_reason(&self) -> String {
        format!(
            "every path explored ({}; {} end without reaching it NULL, {} fault earlier, {} through an inexact arm): no path reaches the dereference with the pointer NULL under a named condition",
            self.runs, self.ended, self.faulted, self.inexact_paths
        )
    }
}

/// Run every path of one phase (depth first, under the budget).
fn phase(context: &Context<'_, '_>, explore_inexact: bool) -> Phase {
    let formals = kids(context.definition)
        .filter(|c| kind(c) == "ParmVarDecl")
        .collect::<Vec<_>>();
    let formal_ids = formals
        .iter()
        .filter_map(|f| f["id"].as_str().map(str::to_owned))
        .collect::<BTreeSet<_>>();
    let mut out = Phase {
        explored: qcore_core::null_paths::Explored {
            complete: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut stack: Vec<Vec<bool>> = vec![Vec::new()];
    let mut allocation_witness: Option<Witness> = None;
    while let Some(decisions) = stack.pop() {
        if out.runs >= MAX_RUNS {
            out.unresolved
                .get_or_insert_with(|| format!("the search budget ({MAX_RUNS} paths)"));
            out.explored.complete = false;
            break;
        }
        out.runs += 1;
        let mut run = Run {
            env: context.env,
            function: context.function,
            target: context.target,
            contains: context.contains,
            tracked: context.tracked,
            decisions: &decisions,
            used: 0,
            steps: 0,
            vars: BTreeMap::new(),
            memory: BTreeMap::new(),
            atoms: Vec::new(),
            next_id: 0,
            decided: BTreeMap::new(),
            decided_texts: BTreeSet::new(),
            needs_allocation_failure: false,
            path: Vec::new(),
            steer: None,
            inexact: None,
            consulted: BTreeSet::new(),
            explore_inexact,
            formal_ids: &formal_ids,
            decided_reads: BTreeSet::new(),
            atom_reads: BTreeMap::new(),
            stores: Vec::new(),
            assigned: Vec::new(),
            calls: Vec::new(),
        };
        for (index, formal) in formals.iter().enumerate() {
            let id = formal["id"].as_str().unwrap_or("").to_owned();
            let name = formal["name"].as_str().unwrap_or("?").to_owned();
            let spelled = ty(formal);
            let value = if is_pointer(spelled) {
                let fresh = run.fresh_id();
                match context.entries.get(&index) {
                    Some(Entry::Declared(_) | Entry::Inferred(_)) => V::Ptr(P::NonNull, fresh),
                    _ => V::Ptr(P::Opaque(Origin::Formal(name.clone()), false), fresh),
                }
            } else {
                match run.int_range(spelled) {
                    Some(range) => V::Int(S::Atom(run.new_atom(Some(range), false))),
                    None => V::Int(S::Opaque(format!("a formal of type `{spelled}`"))),
                }
            };
            if context.tracked.contains_key(&id) {
                run.vars.insert(id, value);
            } else {
                run.memory.insert(name, value);
            }
        }
        let end = match run.stmt(context.body) {
            Ok(Flow::Goto(_)) => {
                End::Unresolved("a jump to a label the search cannot enter".into())
            }
            Ok(_) => End::Ended,
            Err(end) => end,
        };
        out.consulted.append(&mut run.consulted);
        if run.inexact.is_some() && !matches!(end, End::Need(_)) {
            out.inexact_paths += 1;
        }
        match end {
            End::Need(prefer) => {
                let mut other = decisions.clone();
                other.push(!prefer);
                let mut first = decisions;
                first.push(prefer);
                stack.push(other);
                stack.push(first);
            }
            End::Witness(witness) => {
                out.explored.exact_reach = true;
                if witness.class == FindingClass::NormalPath.as_str() {
                    out.finding = Some(Search {
                        triage: Triage::Finding(FindingClass::NormalPath),
                        reason: witness.condition.clone(),
                        witness: Some(witness),
                        runs: out.runs,
                        consulted: std::mem::take(&mut out.consulted),
                    });
                    return out;
                }
                allocation_witness.get_or_insert(witness);
            }
            End::Indeterminate(reason) => {
                out.explored.complete = false;
                out.indeterminate.get_or_insert(reason);
            }
            End::Ended => out.ended += 1,
            End::Faulted => out.faulted += 1,
            End::InexactReach(reason) => {
                out.explored.inexact_reach = true;
                out.inexact_reach.get_or_insert_with(|| {
                    format!("an inexact path reaches the dereference NULL: {reason}")
                });
            }
            End::Unresolved(reason) => {
                out.explored.complete = false;
                out.unresolved.get_or_insert(reason);
            }
        }
    }
    if let Some(witness) = allocation_witness {
        out.finding = Some(Search {
            triage: Triage::Finding(FindingClass::AllocationFailure),
            reason: witness.condition.clone(),
            witness: Some(witness),
            runs: out.runs,
            consulted: std::mem::take(&mut out.consulted),
        });
    }
    out
}

/// The formals and automatic locals the search follows as values: pointers
/// and integers whose address the body never takes, not `volatile`.
fn tracked_variables(
    definition: &Value,
    body: &Value,
    enums: &BTreeSet<String>,
) -> BTreeMap<String, String> {
    fn locals<'v>(node: &'v Value, out: &mut Vec<&'v Value>) {
        if kind(node) == "VarDecl" {
            out.push(node);
        }
        for child in kids(node) {
            locals(child, out);
        }
    }
    let mut taken = BTreeSet::new();
    ast::address_taken(body, &mut taken);
    let mut declared = kids(definition)
        .filter(|c| kind(c) == "ParmVarDecl")
        .collect::<Vec<_>>();
    locals(body, &mut declared);
    declared
        .into_iter()
        .filter(|decl| !matches!(decl["storageClass"].as_str(), Some("static" | "extern")))
        .filter(|decl| {
            let spelled = ty(decl);
            !spelled.contains("volatile")
                && (is_pointer(spelled) || int_range(spelled).is_some() || enums.contains(spelled))
        })
        .filter_map(|decl| Some((decl["id"].as_str()?, decl["name"].as_str().unwrap_or(""))))
        .filter(|(id, _)| !taken.contains(*id))
        .map(|(id, name)| (id.to_owned(), name.to_owned()))
        .collect()
}

/// Whether the spelling `text` names the identifier `name` (as a whole
/// token).
fn mentions(text: &str, name: &str) -> bool {
    !name.is_empty()
        && text
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .any(|token| token == name)
}
