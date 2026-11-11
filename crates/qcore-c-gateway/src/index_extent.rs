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

//! The bounds consumer of route G (card **CG16-INDEX-AND-OFFSET-EXTENTS**,
//! gateway rule 11s, contract specification C09.1).
//!
//! A separately selected property: **index/access bounds**. Nothing in the
//! custody analysis calls this module; a caller asks for it explicitly
//! ([`analyze_bounds_c11`], [`analyze_bounds_preprocessed`], or the audit's
//! `QCORE_BOUNDS_PROPERTY` selection), and the custody result is unchanged
//! whether or not it does.
//!
//! # What is read
//!
//! Clang's typed AST of the whole translation unit (raw transport,
//! [`crate::typed_c_ast::translation_unit_ast`]) and the recorded target's
//! type sizes (Clang's predefined `__SIZEOF_*__`, `__CHAR_UNSIGNED__` and
//! `__PTRDIFF_MAX__`, under the unit's own frontend flags). For each selected
//! function definition the consumer **inventories every selected access** —
//! every subscript, `*` dereference, `->` member access, pointer arithmetic
//! (formation), pointer subtraction and ordering (same object), and every call
//! passing a pointer (a call-site obligation) — in every expression position,
//! loop bodies and conditions included, and judges each one through
//! [`qcore_core::index_extent`]. An expression or statement form it does not
//! read is a named function-level reason (Unknown), never skipped.
//!
//! # Extents and values
//!
//! * A **fixed array** (`T a[N]`, a member `s.a`/`p->a`, a string literal, a
//!   compound literal, a global with a complete type) has the extent Clang's
//!   type states, in elements, with no review: it is a source fact.
//! * A **declared** extent enters only from a bounds contract
//!   ([`qcore_contract::bounds_contract`]): the function's own preconditions
//!   (`bounds_extent`, `bounds_index`, `bounds_global_extent`) and a callee's
//!   `bounds_return_extent`, used only after that call's preconditions are
//!   established. A held declaration's facts carry their clauses; a decision
//!   resting on one is `Held` in the actual judgement and decided in the
//!   hypothetical one ([`qcore_core::index_extent::bounds_verdict`]). A
//!   declaration never overrides a source extent: a difference is a reported
//!   contradiction and the source extent decides.
//! * An **index** is known only in the bounded subset this card owns (CG20
//!   owns C numeric semantics): integer and character literals, enumeration
//!   constants, `+`/`-`/`*` of two known constants in their C type (signed
//!   overflow refuses, unsigned wraps), value-preserving integer conversions,
//!   a tracked local's or parameter's fact, guard facts from comparisons with
//!   a constant or a stable parameter, and a canonical counted `for`.
//!   Everything else is `bounds-unsupported-range`.
//!
//! # Facts and invalidation
//!
//! Facts live on locals and parameters that are not `static`, not `volatile`
//! and whose address the function never takes; a write withdraws the
//! variable's facts and every fact naming it, a call withdraws every pointer
//! fact on an object the source did not establish (unless the callee's
//! contract states `bounds_preserves_extents()`), a loop withdraws whatever it
//! writes, a label starts from no facts, and two incoming edges keep only the
//! facts they share.
//!
//! # Frames and derived relations (card CG24-ALIAS-EXTENT-FRAME-VOCABULARY)
//!
//! A callee whose bounds contract states a **closed frame** (gateway rule
//! 11e) is applied position by position ([`qcore_core::index_extent_frames`]):
//! a read position keeps every fact, a write position withdraws the facts of
//! the local whose address it receives, a move position also withdraws every
//! fact on an object the source did not establish. A local whose address
//! appears only as a direct actual of resolved closed frames (never a derived
//! base) is tracked. A `bounds_return_derived` result is inside its base when
//! the base decides containment, a contradiction is reported, otherwise the
//! declared view is used. A body that exceeds its declared frame or returns
//! something other than its declared base refutes its declaration, which the
//! unit's callers then do not see.
//!
//! # Reachability
//!
//! A violation is published `Unsafe` only when the access is **definitely
//! reached** from the function's entry: on the straight-line prefix, no call,
//! loop, branch, label or unresolved obligation before it, and not under a
//! `&&`, `||` or `?:` operand or beside a call in its own full expression.
//! Otherwise it is `bounds-violation-reachability-unresolved` (Unknown). An
//! access after `return`/`break`/`continue`/`goto` with no label between, or
//! in the arm of a constant-false `if`, is **excluded** by that explicit
//! check and stays in the inventory as excluded.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use qcore_contract::bounds_contract::{
    BOUNDS_CONTRACT_VERSION, BoundsClause, BoundsContracts, BoundsCount, BoundsFunctionContract,
    BoundsUnit, ParameterKind,
};
use qcore_core::index_extent::{
    self as core, BOUNDS_MODEL_VERSION, Count, Decision, EvidenceIdentity, Facts, IndexFact,
    Judgement, Outcome, PointerFact, Refusal, SameObject, ScalarFact, Standing,
};
use qcore_core::index_extent_frames::{
    self as frames, BodyEffect, CallFrame, FrameMode, FrameTouch,
};
use serde_json::Value;

/// Card QUANT-INDEX-CHECKS (gateway rule 12l): `quant checks` on both sides
/// of the call boundary.
mod quant_checks;
// Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (rules 14f-14g): body
// evidence for declarations, never a verdict.
mod body_evidence;
pub use body_evidence::DeclarationBodyEvidence;
/// The standing decisions the audit folds body evidence with.
pub use qcore_core::bounds_body_standing;

/// The property, its verdict values and the composition, for consumers that
/// reach the core through this gateway (the audit routes).
pub use qcore_core::index_extent::{BOUNDS_PROPERTY, BoundsVerdict, Request, compose};

use crate::legacy_c::C11TranslationUnit;
use crate::typed_c_ast::kind;

fn kids(node: &Value) -> std::slice::Iter<'_, Value> {
    crate::typed_c_ast::children(node).iter()
}

/// One judged obligation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct AccessRecord {
    /// `element-access`, `formation`, `dereference`, `member-access`,
    /// `same-object`, `call-precondition`.
    pub kind: &'static str,
    /// The expression position: `statement`, `declaration`, `condition`,
    /// `return`, `actual`, with a `loop-body ` prefix inside a loop.
    pub position: String,
    /// The object (or callee) and the index, as the consumer read them.
    pub subject: String,
    /// Line in the analysed (preprocessed) text.
    pub line: usize,
    pub actual: String,
    pub hypothetical: String,
}

/// The bounds row of one function.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct BoundsFunctionRow {
    pub function: String,
    pub property: &'static str,
    pub verdict: &'static str,
    pub reasons: Vec<String>,
    /// `unconditional`, or the function's own preconditions it is relative to.
    pub scope: String,
    /// Every declared clause a judgement relied on, with its standing.
    pub assumptions: Vec<String>,
    pub accesses: Vec<AccessRecord>,
    /// The body refutes the function's own `bounds_return_extent`.
    pub refutes_own_contract: bool,
    /// Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS: the bounds
    /// declarations the row's analysis could have consulted (its own name
    /// when declared, every directly called declared name); an
    /// over-approximation for the roll-up, read by no verdict. Absent when
    /// empty, so rows that consult none keep their bytes.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub contract_dependencies: Vec<String>,
}

/// The bounds report of one translation unit.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct BoundsUnitReport {
    pub property: &'static str,
    pub model_version: &'static str,
    pub contract_version: &'static str,
    pub source_sha256: String,
    pub target: String,
    pub contracts_sha256: String,
    pub functions: Vec<BoundsFunctionRow>,
    /// Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS: phase one's check of
    /// every declared body the unit defines (header definitions included),
    /// published as evidence for the declaration, never as a row.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub declaration_bodies: Vec<DeclarationBodyEvidence>,
}

impl BoundsUnitReport {
    pub fn identity(&self) -> EvidenceIdentity {
        EvidenceIdentity {
            property: self.property.to_owned(),
            model_version: self.model_version.to_owned(),
            source_sha256: self.source_sha256.clone(),
            target: self.target.clone(),
            contracts_sha256: self.contracts_sha256.clone(),
        }
    }

    pub fn row(&self, function: &str) -> Option<&BoundsFunctionRow> {
        self.functions.iter().find(|row| row.function == function)
    }
}

/// The verdict of a row, as the core's four values.
pub fn row_verdict(row: &BoundsFunctionRow) -> BoundsVerdict {
    match row.verdict {
        "Safe" => BoundsVerdict::Safe,
        "Pending" => BoundsVerdict::Pending,
        "Unsafe" => BoundsVerdict::Unsafe,
        _ => BoundsVerdict::Unknown,
    }
}

// ---------------------------------------------------------------------------
// Target facts
// ---------------------------------------------------------------------------

/// The recorded target's sizes the decisions need.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundsTarget {
    sizes: BTreeMap<String, u128>,
    char_unsigned: bool,
    ptrdiff_max: u128,
    identity: String,
}

static TARGETS: Mutex<BTreeMap<String, BoundsTarget>> = Mutex::new(BTreeMap::new());

fn target_facts(unit: &C11TranslationUnit<'_>) -> Result<BoundsTarget, String> {
    use std::io::Write;
    use std::process::Stdio;
    let mut command = crate::clang::command()?;
    unit.append_frontend_flags(&mut command);
    let dialect = unit.dialect.clang_invocation_argument()?;
    command.args(["-x", "c", dialect, "-dM", "-E", "-"]);
    let key = format!("{command:?}");
    if let Some(found) = TARGETS
        .lock()
        .map_err(|_| "target cache poisoned")?
        .get(&key)
    {
        return Ok(found.clone());
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Clang target facts unavailable: {error}"))?;
    child
        .stdin
        .take()
        .ok_or("Clang stdin unavailable")?
        .write_all(b"")
        .map_err(|error| format!("Clang input failed: {error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("Clang target facts failed: {error}"))?;
    if !output.status.success() {
        return Err("Clang refused the target-fact probe".into());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut sizes = BTreeMap::new();
    let mut char_unsigned = false;
    let mut ptrdiff_max = None;
    let mut lines = Vec::new();
    for line in text.lines() {
        let mut words = line.split_whitespace();
        let (Some("#define"), Some(name)) = (words.next(), words.next()) else {
            continue;
        };
        let value = words.next().unwrap_or("");
        if let Some(ty) = name
            .strip_prefix("__SIZEOF_")
            .and_then(|rest| rest.strip_suffix("__"))
        {
            if let Ok(size) = value.parse::<u128>() {
                sizes.insert(ty.to_owned(), size);
                lines.push(format!("{name}={value}"));
            }
        } else if name == "__CHAR_UNSIGNED__" {
            char_unsigned = true;
            lines.push(name.to_owned());
        } else if name == "__PTRDIFF_MAX__" {
            ptrdiff_max = value.trim_end_matches(['L', 'l']).parse::<u128>().ok();
            lines.push(format!("{name}={value}"));
        }
    }
    let ptrdiff_max = ptrdiff_max.ok_or("the target states no __PTRDIFF_MAX__")?;
    lines.sort();
    let target = BoundsTarget {
        sizes,
        char_unsigned,
        ptrdiff_max,
        identity: qcore_schemas::digest::sha256_hex(lines.join("\n").as_bytes()),
    };
    TARGETS
        .lock()
        .map_err(|_| "target cache poisoned")?
        .insert(key, target.clone());
    Ok(target)
}

/// An integer type's width (bits) and signedness, from its canonical spelling.
fn integer_type(target: &BoundsTarget, ty: &str) -> Option<(u32, bool)> {
    let ty = ty.trim();
    let size = |name: &str| target.sizes.get(name).map(|bytes| (*bytes * 8) as u32);
    Some(match ty {
        "_Bool" => (8, false),
        "char" => (8, !target.char_unsigned),
        "signed char" => (8, true),
        "unsigned char" => (8, false),
        "short" => (size("SHORT")?, true),
        "unsigned short" => (size("SHORT")?, false),
        "int" => (size("INT")?, true),
        "unsigned int" => (size("INT")?, false),
        "long" => (size("LONG")?, true),
        "unsigned long" => (size("LONG")?, false),
        "long long" => (size("LONG_LONG")?, true),
        "unsigned long long" => (size("LONG_LONG")?, false),
        _ => return None,
    })
}

fn type_range(width: u32, signed: bool) -> (i128, i128) {
    if signed {
        (-(1i128 << (width - 1)), (1i128 << (width - 1)) - 1)
    } else {
        (0, (1i128 << width) - 1)
    }
}

/// The size of an element type, when the target states it.
fn element_size(target: &BoundsTarget, ty: &str) -> Option<u128> {
    let ty = ty.trim();
    if ty.ends_with('*') {
        return target.sizes.get("POINTER").copied();
    }
    if let Some((bits, _)) = integer_type(target, ty) {
        return Some(u128::from(bits / 8));
    }
    match ty {
        "float" => target.sizes.get("FLOAT").copied(),
        "double" => target.sizes.get("DOUBLE").copied(),
        "long double" => target.sizes.get("LONG_DOUBLE").copied(),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

fn qual(node: &Value) -> String {
    let ty = &node["type"];
    ty["desugaredQualType"]
        .as_str()
        .or_else(|| ty["qualType"].as_str())
        .unwrap_or("")
        .to_owned()
}

fn strip_qualifiers(ty: &str) -> String {
    ty.split_whitespace()
        .filter(|word| !matches!(*word, "const" | "volatile" | "restrict" | "__restrict"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The outermost array dimension of a type spelling.
enum Shape {
    NotArray,
    Fixed { count: u128, element: String },
    Incomplete,
    Variable,
}

fn shape(ty: &str) -> Shape {
    let mut depth = 0i32;
    for (index, c) in ty.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            '[' if depth == 0 => {
                let Some(close) = ty[index..].find(']') else {
                    return Shape::NotArray;
                };
                let inside = ty[index + 1..index + close].trim();
                let element = format!("{}{}", ty[..index].trim_end(), &ty[index + close + 1..]);
                if inside.is_empty() {
                    return Shape::Incomplete;
                }
                return match inside.parse() {
                    Ok(count) => Shape::Fixed {
                        count,
                        element: element.trim().to_owned(),
                    },
                    Err(_) => Shape::Variable,
                };
            }
            _ => {}
        }
    }
    Shape::NotArray
}

/// The pointee of a pointer type spelling (`T *` → `T`), when simple.
fn pointee(ty: &str) -> Option<String> {
    let ty = strip_qualifiers(ty);
    let ty = ty.trim();
    if let Some(inner) = ty.strip_suffix('*')
        && !inner.contains('(')
    {
        return Some(strip_qualifiers(inner.trim()));
    }
    None
}

fn is_pointer(ty: &str) -> bool {
    let ty = strip_qualifiers(ty);
    ty.trim_end().ends_with('*') || ty.contains("(*)")
}

// ---------------------------------------------------------------------------
// The translation unit's declarations
// ---------------------------------------------------------------------------

struct TuIndex<'a> {
    functions: BTreeMap<String, &'a Value>,
    globals: BTreeMap<String, (String, String)>, // id -> (name, type)
    enums: BTreeMap<String, i128>,               // EnumConstantDecl id -> value
    prototypes: BTreeMap<String, Vec<String>>,   // function name -> parameter types
    /// Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (rule 14g): each
    /// file-scope typedef's underlying type as Clang desugars it, read only by
    /// the byte-view reading of a quant object.
    typedefs: BTreeMap<String, String>,
}

fn index_unit(ast: &Value) -> TuIndex<'_> {
    let mut unit = TuIndex {
        functions: BTreeMap::new(),
        globals: BTreeMap::new(),
        enums: BTreeMap::new(),
        prototypes: BTreeMap::new(),
        typedefs: BTreeMap::new(),
    };
    for decl in kids(ast) {
        match kind(decl) {
            "TypedefDecl" => {
                if let Some(name) = decl["name"].as_str() {
                    unit.typedefs.insert(name.to_owned(), qual(decl));
                }
            }
            "FunctionDecl" => {
                let name = decl["name"].as_str().unwrap_or("").to_owned();
                let parameters = kids(decl)
                    .filter(|c| kind(c) == "ParmVarDecl")
                    .map(qual)
                    .collect::<Vec<_>>();
                unit.prototypes.entry(name.clone()).or_insert(parameters);
                if kids(decl).any(|c| kind(c) == "CompoundStmt") {
                    unit.functions.insert(name, decl);
                }
            }
            "VarDecl" => {
                if let Some(id) = decl["id"].as_str() {
                    unit.globals.insert(
                        id.to_owned(),
                        (decl["name"].as_str().unwrap_or("").to_owned(), qual(decl)),
                    );
                }
            }
            "EnumDecl" => {
                let mut next = 0i128;
                for constant in kids(decl).filter(|c| kind(c) == "EnumConstantDecl") {
                    let explicit = kids(constant)
                        .find(|c| kind(c) == "ConstantExpr")
                        .and_then(|c| c["value"].as_str())
                        .and_then(|v| v.parse::<i128>().ok());
                    let has_initializer = kids(constant).next().is_some();
                    let value = match (explicit, has_initializer) {
                        (Some(value), _) => value,
                        (None, false) => next,
                        (None, true) => {
                            // An initializer Clang did not fold: no value.
                            next = i128::MIN;
                            continue;
                        }
                    };
                    if let Some(id) = constant["id"].as_str()
                        && (next != i128::MIN || explicit.is_some())
                    {
                        unit.enums.insert(id.to_owned(), value);
                    }
                    next = value.saturating_add(1);
                }
            }
            _ => {}
        }
    }
    unit
}

// ---------------------------------------------------------------------------
// Analysis state
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct State {
    facts: Facts,
    /// Held clauses each fact rests on (same keys as `facts`).
    held: BTreeMap<String, BTreeSet<String>>,
    /// Pointer facts' element types (same keys as `facts.pointers`).
    elements: BTreeMap<String, String>,
    /// Parameters not written on this path: their reads are stable symbols.
    stable: BTreeSet<String>,
    live: bool,
    definite: bool,
}

impl State {
    fn sync(&mut self) {
        let facts = &self.facts;
        self.held
            .retain(|k, _| facts.scalars.contains_key(k) || facts.pointers.contains_key(k));
        self.elements.retain(|k, _| facts.pointers.contains_key(k));
    }

    fn write(&mut self, id: &str) {
        self.facts.write(id);
        self.stable.remove(id);
        self.sync();
    }

    fn call(&mut self) {
        self.facts.call();
        self.sync();
    }

    fn join(&self, other: &Self) -> Self {
        if !self.live {
            return other.clone();
        }
        if !other.live {
            return self.clone();
        }
        let facts = self.facts.join(&other.facts);
        let mut joined = Self {
            facts,
            held: self.held.clone(),
            elements: self.elements.clone(),
            stable: self.stable.intersection(&other.stable).cloned().collect(),
            live: true,
            definite: false,
        };
        for (key, clauses) in &other.held {
            joined
                .held
                .entry(key.clone())
                .or_default()
                .extend(clauses.iter().cloned());
        }
        joined.sync();
        joined
    }

    fn dead(&self) -> Self {
        let mut dead = self.clone();
        dead.live = false;
        dead
    }
}

/// A value read at a use: its fact, the held clauses it rests on, and the
/// stable parameter it is exactly (if any).
#[derive(Clone, Debug)]
struct Val {
    fact: IndexFact,
    held: BTreeSet<String>,
    symbol: Option<String>,
}

impl Val {
    fn unknown() -> Self {
        Self {
            fact: IndexFact::Unknown,
            held: BTreeSet::new(),
            symbol: None,
        }
    }
    fn constant(value: i128) -> Self {
        Self {
            fact: IndexFact::Const(value),
            held: BTreeSet::new(),
            symbol: None,
        }
    }
}

/// What a pointer expression points into.
#[derive(Clone, Debug)]
enum Target {
    Known {
        object: String,
        count: Count,
        unit: Unit2,
        offset: IndexFact,
        element: String,
        held: BTreeSet<String>,
        source_object: bool,
        /// A whole named variable (distinct from every other one).
        distinct: bool,
    },
    Missing(String),
}

type Unit2 = qcore_core::index_extent::Unit;

struct Analysis<'a> {
    unit: &'a TuIndex<'a>,
    target: &'a BoundsTarget,
    contracts: &'a BoundsContracts,
    preprocessed: &'a str,
    /// Locals and parameters that may carry facts.
    tracked: BTreeSet<String>,
    params: BTreeMap<String, String>, // id -> name
    globals_declared: BTreeMap<String, (Count, Unit2, BTreeSet<String>)>,
    records: Vec<AccessRecord>,
    judgements: Vec<Judgement>,
    reasons: Vec<String>,
    assumptions: BTreeSet<String>,
    switch_entries: Vec<State>,
    loop_depth: usize,
    /// The current full expression is definitely reached and holds exactly
    /// one call: that call's preconditions are definitely reached.
    call_reach: bool,
    /// Depth of `&&`/`||`/`?:` operands evaluated conditionally.
    conditional: usize,
    /// The analysed function's own bounds contract (in force or held).
    own: Option<&'a BoundsFunctionContract>,
    /// The body refutes `own`'s return extent.
    refuted: bool,
    /// Card QUANT-INDEX-CHECKS: the function's own quant-checked pointer
    /// formals, by the object name their seeded fact carries.
    quant: BTreeMap<String, quant_checks::QuantObject>,
}

fn offset_of(node: &Value) -> Option<usize> {
    let range = &node["range"]["begin"];
    range["offset"]
        .as_u64()
        .or_else(|| range["expansionLoc"]["offset"].as_u64())
        .or_else(|| node["loc"]["offset"].as_u64())
        .map(|o| o as usize)
}

fn strip(node: &Value) -> &Value {
    let mut node = node;
    while kind(node) == "ParenExpr" {
        match kids(node).next() {
            Some(inner) => node = inner,
            None => break,
        }
    }
    node
}

fn decl_ref(node: &Value) -> Option<(&str, &str, &str)> {
    let node = strip(node);
    if kind(node) != "DeclRefExpr" {
        return None;
    }
    let decl = &node["referencedDecl"];
    Some((
        decl["id"].as_str()?,
        decl["kind"].as_str()?,
        decl["name"].as_str()?,
    ))
}

/// The variable a read `LValueToRValue(DeclRefExpr)` reads.
fn read_of(node: &Value) -> Option<&str> {
    let node = strip(node);
    if kind(node) == "ImplicitCastExpr" && node["castKind"] == "LValueToRValue" {
        let (id, decl_kind, _) = decl_ref(kids(node).next()?)?;
        if matches!(decl_kind, "VarDecl" | "ParmVarDecl") {
            return Some(id);
        }
    }
    None
}

fn call_count(node: &Value) -> usize {
    usize::from(kind(node) == "CallExpr") + kids(node).map(call_count).sum::<usize>()
}

fn has_call(node: &Value) -> bool {
    kind(node) == "CallExpr" || kids(node).any(has_call)
}

/// Variables a node writes: assignment and compound-assignment destinations,
/// increments and decrements, named by their declaration id.
fn writes(node: &Value, out: &mut BTreeSet<String>) {
    match kind(node) {
        "BinaryOperator" if node["opcode"] == "=" => {
            if let Some((id, _, _)) = kids(node).next().and_then(decl_ref) {
                out.insert(id.to_owned());
            }
        }
        "CompoundAssignOperator" => {
            if let Some((id, _, _)) = kids(node).next().and_then(decl_ref) {
                out.insert(id.to_owned());
            }
        }
        "UnaryOperator" if matches!(node["opcode"].as_str(), Some("++" | "--")) => {
            if let Some((id, _, _)) = kids(node).next().and_then(decl_ref) {
                out.insert(id.to_owned());
            }
        }
        "UnaryExprOrTypeTraitExpr" => return,
        _ => {}
    }
    for child in kids(node) {
        writes(child, out);
    }
}

/// Variables whose address the function takes (or that an array decay or a
/// member/element address roots).
fn address_taken(node: &Value, out: &mut BTreeSet<String>) {
    if let Some(id) = address_root(node) {
        out.insert(id.to_owned());
    }
    for child in kids(node) {
        address_taken(child, out);
    }
}

/// The variable one `&` expression roots: its own address, or the address of
/// a `.` member or an element of it.
fn address_root(node: &Value) -> Option<&str> {
    if kind(node) != "UnaryOperator" || node["opcode"] != "&" {
        return None;
    }
    let mut operand = kids(node).next();
    while let Some(inner) = operand {
        let inner = strip(inner);
        match kind(inner) {
            "DeclRefExpr" => return decl_ref(inner).map(|(id, _, _)| id),
            "MemberExpr" if inner["isArrow"] != true => operand = kids(inner).next(),
            "ArraySubscriptExpr" => {
                operand = kids(inner).find(|c| {
                    let c = strip(c);
                    kind(c) == "ImplicitCastExpr" && c["castKind"] == "ArrayToPointerDecay"
                });
                operand = operand.and_then(|c| kids(c).next());
            }
            _ => return None,
        }
    }
    None
}

/// Card CG24: how many times each variable's address is taken (the forms
/// [`address_taken`] reads).
fn address_counts(node: &Value, out: &mut BTreeMap<String, usize>) {
    if let Some(id) = address_root(node) {
        *out.entry(id.to_owned()).or_default() += 1;
    }
    for child in kids(node) {
        address_counts(child, out);
    }
}

/// The named callee of a direct call.
fn callee_name(call: &Value) -> Option<String> {
    let callee = strip(kids(call).next()?);
    (kind(callee) == "ImplicitCastExpr" && callee["castKind"] == "FunctionToPointerDecay")
        .then(|| kids(callee).next().and_then(decl_ref))
        .flatten()
        .filter(|d| d.1 == "FunctionDecl")
        .map(|d| d.2.to_owned())
}

/// The variable `&x` names, when the actual is exactly that address (through
/// parentheses and value-preserving pointer conversions).
fn address_actual(node: &Value) -> Option<&str> {
    let mut node = strip(node);
    while matches!(kind(node), "ImplicitCastExpr" | "CStyleCastExpr")
        && matches!(node["castKind"].as_str(), Some("NoOp" | "BitCast"))
    {
        node = strip(kids(node).next()?);
    }
    if kind(node) == "UnaryOperator" && node["opcode"] == "&" {
        let (id, decl_kind, _) = decl_ref(kids(node).next()?)?;
        if matches!(decl_kind, "VarDecl" | "ParmVarDecl") {
            return Some(id);
        }
    }
    None
}

/// A declared parameter kind against the C type of its actual (or formal).
fn kind_matches(kind: ParameterKind, ty: &str) -> bool {
    match kind {
        ParameterKind::Scalar => !is_pointer(ty),
        ParameterKind::Pointer => is_pointer(ty),
        ParameterKind::Slot => is_pointer(ty) && pointee(ty).is_some_and(|t| is_pointer(&t)),
    }
}

fn call_abi_matches(contract: &BoundsFunctionContract, arguments: &[&Value]) -> bool {
    contract.parameters.len() == arguments.len()
        && contract
            .parameters
            .iter()
            .zip(arguments)
            .all(|(p, a)| kind_matches(p.kind, &qual(a)))
}

fn collect_calls<'v>(node: &'v Value, out: &mut Vec<&'v Value>) {
    if kind(node) == "UnaryExprOrTypeTraitExpr" {
        return;
    }
    if kind(node) == "CallExpr" {
        out.push(node);
    }
    for child in kids(node) {
        collect_calls(child, out);
    }
}

/// Card CG24: the locals whose every address is a direct actual at a closed,
/// resolved frame position that is not a derived base. The frame then
/// decides what each call does to them, so they may carry facts.
fn frame_tracked(body: &Value, contracts: &BoundsContracts) -> BTreeSet<String> {
    let mut all = BTreeMap::new();
    address_counts(body, &mut all);
    let mut framed: BTreeMap<String, usize> = BTreeMap::new();
    let mut calls = Vec::new();
    collect_calls(body, &mut calls);
    for call in calls {
        let arguments = kids(call).skip(1).collect::<Vec<_>>();
        let Some(contract) = callee_name(call)
            .and_then(|name| contracts.contract(&name))
            .filter(|c| !c.held() && call_abi_matches(c, &arguments))
        else {
            continue;
        };
        let Some(modes) = contract.frame() else {
            continue;
        };
        let bases = contract.derived_bases();
        for ((mode, parameter), argument) in modes.iter().zip(&contract.parameters).zip(&arguments)
        {
            if mode.is_some()
                && !bases.contains(&parameter.name.as_str())
                && let Some(id) = address_actual(argument)
            {
                *framed.entry(id.to_owned()).or_default() += 1;
            }
        }
    }
    all.into_iter()
        .filter(|(id, n)| framed.get(id) == Some(n))
        .map(|(id, _)| id)
        .collect()
}

/// Card CG24: the parameter a pointer expression is derived from (a read of
/// it, through conversions, pointer arithmetic and array decay of a member).
fn pointer_root(node: &Value) -> Option<String> {
    let node = strip(node);
    match kind(node) {
        "ImplicitCastExpr" | "CStyleCastExpr" => match node["castKind"].as_str() {
            Some("LValueToRValue") => read_of(node).map(str::to_owned),
            Some("NoOp" | "BitCast") => pointer_root(kids(node).next()?),
            Some("ArrayToPointerDecay") => written_through(kids(node).next()?),
            _ => None,
        },
        "BinaryOperator" if matches!(node["opcode"].as_str(), Some("+" | "-")) => {
            let mut operands = kids(node);
            let (left, right) = (operands.next()?, operands.next()?);
            if is_pointer(&qual(left)) {
                pointer_root(left)
            } else {
                pointer_root(right)
            }
        }
        _ => None,
    }
}

/// Card CG24: the parameter an lvalue is reached through (`*p`, `p[i]`,
/// `p->f`, and their members), if any.
fn written_through(node: &Value) -> Option<String> {
    let node = strip(node);
    match kind(node) {
        "UnaryOperator" if node["opcode"] == "*" => pointer_root(kids(node).next()?),
        "ArraySubscriptExpr" => {
            let mut operands = kids(node);
            let (first, second) = (operands.next()?, operands.next()?);
            pointer_root(if is_pointer(&qual(first)) {
                first
            } else {
                second
            })
        }
        "MemberExpr" if node["isArrow"] == true => pointer_root(kids(node).next()?),
        "MemberExpr" => written_through(kids(node).next()?),
        _ => None,
    }
}

/// Card CG24: what a body does through its parameters: every write through
/// one, and every hand-off at a write or move position of a closed frame.
fn body_effects(node: &Value, contracts: &BoundsContracts, out: &mut Vec<(String, BodyEffect)>) {
    match kind(node) {
        "UnaryExprOrTypeTraitExpr" => return,
        "BinaryOperator" if node["opcode"] == "=" => {
            if let Some(id) = kids(node).next().and_then(written_through) {
                out.push((id, BodyEffect::Written));
            }
        }
        "CompoundAssignOperator" => {
            if let Some(id) = kids(node).next().and_then(written_through) {
                out.push((id, BodyEffect::Written));
            }
        }
        "UnaryOperator" if matches!(node["opcode"].as_str(), Some("++" | "--")) => {
            if let Some(id) = kids(node).next().and_then(written_through) {
                out.push((id, BodyEffect::Written));
            }
        }
        "CallExpr" => {
            let arguments = kids(node).skip(1).collect::<Vec<_>>();
            if let Some(modes) = callee_name(node)
                .and_then(|name| contracts.contract(&name))
                .filter(|c| call_abi_matches(c, &arguments))
                .and_then(|c| c.frame())
            {
                for (mode, argument) in modes.iter().zip(&arguments) {
                    let effect = match mode {
                        Some(FrameMode::Writes) => BodyEffect::Written,
                        Some(FrameMode::Moves) => BodyEffect::Moved,
                        _ => continue,
                    };
                    if let Some(id) = pointer_root(argument) {
                        out.push((id, effect));
                    }
                }
            }
        }
        _ => {}
    }
    for child in kids(node) {
        body_effects(child, contracts, out);
    }
}

fn declares_checked_relation(contract: &BoundsFunctionContract) -> bool {
    contract.clauses.iter().any(|e| {
        matches!(
            e.clause,
            BoundsClause::ReturnExtent { .. }
                | BoundsClause::Frame { .. }
                | BoundsClause::ReturnDerived { .. }
                | BoundsClause::QuantCheck { .. }
        )
    })
}

const READ_EXPRESSIONS: [&str; 22] = [
    "ParenExpr",
    "ImplicitCastExpr",
    "CStyleCastExpr",
    "BinaryOperator",
    "CompoundAssignOperator",
    "UnaryOperator",
    "CallExpr",
    "DeclRefExpr",
    "IntegerLiteral",
    "CharacterLiteral",
    "FloatingLiteral",
    "StringLiteral",
    "MemberExpr",
    "ArraySubscriptExpr",
    "ConditionalOperator",
    "InitListExpr",
    "ImplicitValueInitExpr",
    "CompoundLiteralExpr",
    "ConstantExpr",
    "PredefinedExpr",
    "UnaryExprOrTypeTraitExpr",
    "OffsetOfExpr",
];

fn outcome_of(decision: &Decision, reachable: bool, context: &str) -> Outcome {
    match decision {
        Decision::InBounds => Outcome::InBounds,
        Decision::OutOfBounds => Outcome::Violation { reachable },
        Decision::Undecided(refusal) => {
            Outcome::Unresolved(stable_text(&format!("{}: {context}", refusal.code())))
        }
    }
}

/// Clang's node and declaration ids are process addresses: never part of a
/// published text (they would make two runs of one unit differ).
fn stable_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'0' && bytes.get(i + 1) == Some(&b'x') {
            let mut j = i + 2;
            while j < bytes.len() && bytes[j].is_ascii_hexdigit() {
                j += 1;
            }
            if j > i + 2 {
                if out.ends_with('#') {
                    out.pop();
                } else {
                    out.push_str("id");
                }
                i = j;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn render_outcome(outcome: &Outcome) -> String {
    match outcome {
        Outcome::InBounds => "in-bounds".into(),
        Outcome::Excluded => "excluded: proved unreachable".into(),
        Outcome::Violation { reachable: true } => "violation".into(),
        Outcome::Violation { reachable: false } => core::UNREACHED_VIOLATION.into(),
        Outcome::Held(clauses) => format!("held: {}", clauses.join("; ")),
        Outcome::Unresolved(reason) => stable_text(reason),
    }
}

fn add_offsets(left: &IndexFact, right: &IndexFact) -> IndexFact {
    match (left, right) {
        (IndexFact::Const(a), IndexFact::Const(b)) => a
            .checked_add(*b)
            .map_or(IndexFact::Unknown, IndexFact::Const),
        (IndexFact::Const(0), other) | (other, IndexFact::Const(0)) => other.clone(),
        (
            IndexFact::Const(a),
            IndexFact::Range {
                lo,
                hi: Count::Const(hi),
            },
        )
        | (
            IndexFact::Range {
                lo,
                hi: Count::Const(hi),
            },
            IndexFact::Const(a),
        ) => match (lo.checked_add(*a), (*hi as i128).checked_add(*a)) {
            (Some(lo), Some(hi)) if hi >= 0 => IndexFact::Range {
                lo,
                hi: Count::Const(hi as u128),
            },
            _ => IndexFact::Unknown,
        },
        _ => IndexFact::Unknown,
    }
}

fn negate(fact: &IndexFact) -> IndexFact {
    match fact {
        IndexFact::Const(v) => IndexFact::Const(-v),
        _ => IndexFact::Unknown,
    }
}

impl<'a> Analysis<'a> {
    fn line(&self, node: &Value) -> usize {
        offset_of(node)
            .map(|offset| {
                self.preprocessed[..offset.min(self.preprocessed.len())]
                    .bytes()
                    .filter(|b| *b == b'\n')
                    .count()
                    + 1
            })
            .unwrap_or(0)
    }

    fn reason(&mut self, reason: String) {
        let reason = stable_text(&reason);
        if !self.reasons.contains(&reason) {
            self.reasons.push(reason);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn record(
        &mut self,
        node: &Value,
        kind: &'static str,
        position: &str,
        subject: String,
        decision: Decision,
        held: &BTreeSet<String>,
        reachable: bool,
        state: &State,
    ) {
        let (actual, hypothetical) = if !state.live {
            (Outcome::Excluded, Outcome::Excluded)
        } else {
            let decided = outcome_of(&decision, reachable, &subject);
            if held.is_empty() {
                (decided.clone(), decided)
            } else {
                (Outcome::Held(held.iter().cloned().collect()), decided)
            }
        };
        for clause in held {
            self.assumptions.insert(format!("held: {clause}"));
        }
        self.records.push(AccessRecord {
            kind,
            position: if self.loop_depth > 0 {
                format!("loop-body {position}")
            } else {
                position.to_owned()
            },
            subject: stable_text(&subject),
            line: self.line(node),
            actual: render_outcome(&actual),
            hypothetical: render_outcome(&hypothetical),
        });
        self.judgements.push(Judgement {
            actual,
            hypothetical,
        });
    }

    // -- values ------------------------------------------------------------

    fn eval(&self, node: &Value, state: &State) -> Val {
        let node = strip(node);
        let ty = qual(node);
        match kind(node) {
            "IntegerLiteral" => node["value"]
                .as_str()
                .and_then(|v| v.parse().ok())
                .map_or_else(Val::unknown, Val::constant),
            "CharacterLiteral" => node["value"]
                .as_i64()
                .map_or_else(Val::unknown, |v| Val::constant(v.into())),
            "ConstantExpr" => node["value"]
                .as_str()
                .and_then(|v| v.parse().ok())
                .map_or_else(
                    || {
                        kids(node)
                            .next()
                            .map_or_else(Val::unknown, |c| self.eval(c, state))
                    },
                    Val::constant,
                ),
            "DeclRefExpr" => match decl_ref(node) {
                Some((id, "EnumConstantDecl", _)) => self
                    .unit
                    .enums
                    .get(id)
                    .map_or_else(Val::unknown, |v| Val::constant(*v)),
                _ => Val::unknown(),
            },
            "ImplicitCastExpr" | "CStyleCastExpr" => {
                let Some(inner) = kids(node).next() else {
                    return Val::unknown();
                };
                match node["castKind"].as_str() {
                    Some("LValueToRValue") => {
                        let Some(id) = read_of(node) else {
                            return Val::unknown();
                        };
                        if !self.tracked.contains(id) {
                            return Val::unknown();
                        }
                        let symbol = (self.params.contains_key(id) && state.stable.contains(id))
                            .then(|| id.to_owned());
                        match state.facts.scalars.get(id) {
                            Some(fact) => Val {
                                fact: fact.value.clone(),
                                held: state.held.get(id).cloned().unwrap_or_default(),
                                symbol,
                            },
                            None => Val {
                                fact: IndexFact::Unknown,
                                held: BTreeSet::new(),
                                symbol,
                            },
                        }
                    }
                    Some("NoOp") => self.eval(inner, state),
                    Some("IntegralCast") => {
                        let value = self.eval(inner, state);
                        self.convert(value, &qual(inner), &ty)
                    }
                    _ => Val::unknown(),
                }
            }
            "UnaryOperator" => {
                let Some(inner) = kids(node).next() else {
                    return Val::unknown();
                };
                match node["opcode"].as_str() {
                    Some("+") => self.eval(inner, state),
                    Some("-") => {
                        let value = self.eval(inner, state);
                        let IndexFact::Const(v) = value.fact else {
                            return Val::unknown();
                        };
                        self.in_type(-v, &ty, value.held)
                    }
                    _ => Val::unknown(),
                }
            }
            "BinaryOperator" => {
                let mut operands = kids(node);
                let (Some(left), Some(right)) = (operands.next(), operands.next()) else {
                    return Val::unknown();
                };
                let (l, r) = (self.eval(left, state), self.eval(right, state));
                let (IndexFact::Const(a), IndexFact::Const(b)) = (&l.fact, &r.fact) else {
                    return Val::unknown();
                };
                let exact = match node["opcode"].as_str() {
                    Some("+") => a.checked_add(*b),
                    Some("-") => a.checked_sub(*b),
                    Some("*") => a.checked_mul(*b),
                    _ => None,
                };
                let held = l.held.union(&r.held).cloned().collect();
                exact.map_or_else(Val::unknown, |v| self.in_type(v, &ty, held))
            }
            _ => Val::unknown(),
        }
    }

    /// An exact result in its C type: signed overflow refuses (undefined),
    /// unsigned wraps (defined).
    fn in_type(&self, value: i128, ty: &str, held: BTreeSet<String>) -> Val {
        let Some((width, signed)) = integer_type(self.target, &strip_qualifiers(ty)) else {
            return Val::unknown();
        };
        let (min, max) = type_range(width, signed);
        let value = if signed {
            if value < min || value > max {
                return Val::unknown();
            }
            value
        } else {
            value.rem_euclid(max + 1)
        };
        Val {
            fact: IndexFact::Const(value),
            held,
            symbol: None,
        }
    }

    /// An integer conversion: exact into an unsigned type, value-preserving
    /// into a signed one; an out-of-range signed conversion is
    /// implementation-defined and refused.
    fn convert(&self, value: Val, from: &str, to: &str) -> Val {
        let (Some(from_type), Some((width, signed))) = (
            integer_type(self.target, &strip_qualifiers(from)),
            integer_type(self.target, &strip_qualifiers(to)),
        ) else {
            return Val::unknown();
        };
        let (min, max) = type_range(width, signed);
        let symbol = value.symbol.clone().filter(|_| {
            let (fmin, fmax) = type_range(from_type.0, from_type.1);
            fmin >= min && fmax <= max
        });
        match &value.fact {
            IndexFact::Const(v) => {
                if signed {
                    if *v < min || *v > max {
                        return Val::unknown();
                    }
                    Val { symbol, ..value }
                } else {
                    Val {
                        fact: IndexFact::Const(v.rem_euclid(max + 1)),
                        held: value.held,
                        symbol: if *v >= 0 && *v <= max { symbol } else { None },
                    }
                }
            }
            IndexFact::Range { lo, hi } => {
                let fits = *lo >= min
                    && match hi {
                        Count::Const(hi) => (*hi as i128) <= max + 1,
                        Count::Symbol(_) => {
                            let (fmin, fmax) = type_range(from_type.0, from_type.1);
                            fmin >= min && fmax <= max
                        }
                    };
                if fits {
                    Val { symbol, ..value }
                } else {
                    Val::unknown()
                }
            }
            IndexFact::Unknown => Val {
                fact: IndexFact::Unknown,
                held: value.held,
                symbol,
            },
        }
    }

    // -- pointer targets ---------------------------------------------------

    fn object_name(&self, node: &Value) -> (String, bool) {
        let node = strip(node);
        match kind(node) {
            "DeclRefExpr" => match decl_ref(node) {
                Some((id, _, name)) => (format!("{name}#{id}"), true),
                None => (
                    format!("object#{}", node["id"].as_str().unwrap_or("?")),
                    false,
                ),
            },
            "MemberExpr" => {
                let base = kids(node)
                    .next()
                    .map(|b| self.object_name(b).0)
                    .unwrap_or_default();
                let arrow = if node["isArrow"] == true { "->" } else { "." };
                (
                    format!("{base}{arrow}{}", node["name"].as_str().unwrap_or("?")),
                    false,
                )
            }
            "ImplicitCastExpr" => kids(node)
                .next()
                .map(|c| self.object_name(c))
                .unwrap_or_else(|| ("?".into(), false)),
            _ => (
                format!("{}#{}", kind(node), node["id"].as_str().unwrap_or("?")),
                false,
            ),
        }
    }

    fn target(&mut self, node: &Value, state: &State) -> Target {
        let node = strip(node);
        match kind(node) {
            "ImplicitCastExpr" | "CStyleCastExpr" => {
                let Some(inner) = kids(node).next() else {
                    return Target::Missing("unreadable pointer".into());
                };
                match node["castKind"].as_str() {
                    Some("ArrayToPointerDecay") => self.array_target(inner, state),
                    Some("LValueToRValue") => {
                        let Some(id) = read_of(node) else {
                            return Target::Missing(
                                "no extent established for this pointer".into(),
                            );
                        };
                        match state.facts.pointers.get(id) {
                            Some(fact) => Target::Known {
                                object: fact.object.clone(),
                                count: fact.count.clone(),
                                unit: fact.unit,
                                offset: fact.offset.clone(),
                                element: state.elements.get(id).cloned().unwrap_or_default(),
                                held: state.held.get(id).cloned().unwrap_or_default(),
                                source_object: fact.source_object,
                                distinct: false,
                            },
                            None => Target::Missing(format!(
                                "no extent established for `{}`",
                                decl_ref(kids(node).next().unwrap_or(node)).map_or("?", |d| d.2)
                            )),
                        }
                    }
                    Some("NoOp" | "BitCast") => {
                        let target = self.target(inner, state);
                        let same = pointee(&qual(node)).map(|t| strip_qualifiers(&t))
                            == pointee(&qual(inner)).map(|t| strip_qualifiers(&t));
                        if same || self.byte_view_of_quant_object(node, inner, &target) {
                            target
                        } else {
                            Target::Missing("a pointer conversion changes the element type".into())
                        }
                    }
                    _ => Target::Missing("unsupported pointer conversion".into()),
                }
            }
            "BinaryOperator" if matches!(node["opcode"].as_str(), Some("+" | "-")) => {
                let mut operands = kids(node);
                let (Some(left), Some(right)) = (operands.next(), operands.next()) else {
                    return Target::Missing("unreadable pointer arithmetic".into());
                };
                let (pointer, integer, negative) = if is_pointer(&qual(left)) {
                    (left, right, node["opcode"] == "-")
                } else {
                    (right, left, false)
                };
                let base = self.target(pointer, state);
                let value = self.eval(integer, state);
                let step = if negative {
                    negate(&value.fact)
                } else {
                    value.fact
                };
                match base {
                    Target::Known {
                        object,
                        count,
                        unit,
                        offset,
                        element,
                        mut held,
                        source_object,
                        distinct,
                    } => {
                        held.extend(value.held);
                        Target::Known {
                            object,
                            count,
                            unit,
                            offset: add_offsets(&offset, &step),
                            element,
                            held,
                            source_object,
                            distinct,
                        }
                    }
                    missing => missing,
                }
            }
            "UnaryOperator" if node["opcode"] == "&" => {
                let Some(inner) = kids(node).next().map(strip) else {
                    return Target::Missing("unreadable address".into());
                };
                match kind(inner) {
                    "DeclRefExpr" => {
                        let (object, distinct) = self.object_name(inner);
                        Target::Known {
                            object,
                            count: Count::Const(1),
                            unit: Unit2::Elements,
                            offset: IndexFact::Const(0),
                            element: strip_qualifiers(&qual(inner)),
                            held: BTreeSet::new(),
                            source_object: true,
                            distinct,
                        }
                    }
                    "ArraySubscriptExpr" => {
                        // `&a[i]` is `a + i`.
                        let (base, index) = self.subscript_parts(inner);
                        let (Some(base), Some(index)) = (base, index) else {
                            return Target::Missing("unreadable subscript".into());
                        };
                        let value = self.eval(index, state);
                        match self.target(base, state) {
                            Target::Known {
                                object,
                                count,
                                unit,
                                offset,
                                element,
                                mut held,
                                source_object,
                                distinct,
                            } => {
                                held.extend(value.held);
                                Target::Known {
                                    object,
                                    count,
                                    unit,
                                    offset: add_offsets(&offset, &value.fact),
                                    element,
                                    held,
                                    source_object,
                                    distinct,
                                }
                            }
                            missing => missing,
                        }
                    }
                    _ => Target::Missing("the address of this object carries no extent".into()),
                }
            }
            "DeclRefExpr" => match decl_ref(node) {
                Some((id, "VarDecl" | "ParmVarDecl", name)) => match state.facts.pointers.get(id) {
                    Some(fact) => Target::Known {
                        object: fact.object.clone(),
                        count: fact.count.clone(),
                        unit: fact.unit,
                        offset: fact.offset.clone(),
                        element: state.elements.get(id).cloned().unwrap_or_default(),
                        held: state.held.get(id).cloned().unwrap_or_default(),
                        source_object: fact.source_object,
                        distinct: false,
                    },
                    None => Target::Missing(format!("no extent established for `{name}`")),
                },
                _ => Target::Missing("unsupported pointer form".into()),
            },
            _ => Target::Missing("unsupported pointer form".into()),
        }
    }

    fn array_target(&mut self, array: &Value, state: &State) -> Target {
        let ty = qual(array);
        let (object, distinct) = self.object_name(array);
        let declared = decl_ref(array).and_then(|(id, _, _)| {
            self.unit
                .globals
                .get(id)
                .and_then(|(name, _)| self.globals_declared.get(name).cloned())
        });
        let _ = state;
        match shape(&ty) {
            Shape::Fixed { count, element } => {
                if let Some((declared_count, _, held)) = declared {
                    let standing = if held.is_empty() {
                        Standing::Resolved
                    } else {
                        Standing::Held
                    };
                    if let core::ExtentResolution::Conflict { source, declared } =
                        core::resolve_extent(Some(count), Some((declared_count, standing)))
                    {
                        self.reason(format!(
                            "bounds-declaration-contradicted: `{object}` has {source} elements in the source; a declaration states {declared:?}"
                        ));
                    }
                }
                Target::Known {
                    object,
                    count: Count::Const(count),
                    unit: Unit2::Elements,
                    offset: IndexFact::Const(0),
                    element: strip_qualifiers(&element),
                    held: BTreeSet::new(),
                    source_object: true,
                    distinct: distinct && decl_ref(array).is_some(),
                }
            }
            Shape::Incomplete => match declared {
                Some((count, unit, held)) => Target::Known {
                    object,
                    count,
                    unit,
                    offset: IndexFact::Const(0),
                    element: match shape(&ty.replacen("[]", "[1]", 1)) {
                        Shape::Fixed { element, .. } => strip_qualifiers(&element),
                        _ => String::new(),
                    },
                    held,
                    source_object: false,
                    distinct: false,
                },
                None => Target::Missing(format!(
                    "`{object}` has an incomplete array type and no declared extent"
                )),
            },
            Shape::Variable => Target::Missing(format!(
                "`{object}` is a variable-length array; its extent is CG17's"
            )),
            Shape::NotArray => Target::Missing(format!("`{object}` has no array extent")),
        }
    }

    fn subscript_parts<'n>(&self, node: &'n Value) -> (Option<&'n Value>, Option<&'n Value>) {
        let mut operands = kids(node);
        let (Some(first), Some(second)) = (operands.next(), operands.next()) else {
            return (None, None);
        };
        if is_pointer(&qual(first)) {
            (Some(first), Some(second))
        } else {
            (Some(second), Some(first))
        }
    }

    // -- obligations --------------------------------------------------------

    fn decide_access(
        &self,
        target: &Target,
        index: &IndexFact,
        formation: bool,
    ) -> (Decision, String) {
        match target {
            Target::Missing(reason) => {
                (Decision::Undecided(Refusal::MissingExtent), reason.clone())
            }
            Target::Known {
                object,
                count,
                unit,
                offset,
                element,
                ..
            } => {
                let at = add_offsets(offset, index);
                let subject = format!("{object}[{at:?}] of {count:?} {}", unit.as_str());
                let decision = match unit {
                    Unit2::Elements => {
                        if formation {
                            core::formation(Some(count), &at)
                        } else {
                            core::element_access(Some(count), &at)
                        }
                    }
                    Unit2::Bytes => match (count, &at, element_size(self.target, element)) {
                        (Count::Const(bytes), IndexFact::Const(i), Some(width)) => {
                            match core::scale(*i, width, self.target.ptrdiff_max) {
                                Err(refusal) => Decision::Undecided(refusal),
                                Ok(byte_offset) if formation => {
                                    if byte_offset >= 0 && (byte_offset as u128) <= *bytes {
                                        Decision::InBounds
                                    } else {
                                        Decision::OutOfBounds
                                    }
                                }
                                Ok(byte_offset) => core::byte_access(
                                    *bytes,
                                    byte_offset,
                                    width,
                                    self.target.ptrdiff_max,
                                ),
                            }
                        }
                        (_, _, None) => Decision::Undecided(Refusal::UnsupportedRange),
                        _ => Decision::Undecided(Refusal::UnsupportedRange),
                    },
                };
                (self.quant_lower_bound_only(target, decision), subject)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn access(
        &mut self,
        node: &Value,
        kind: &'static str,
        position: &str,
        target: Target,
        index: Val,
        formation: bool,
        reach: bool,
        state: &State,
    ) -> bool {
        // Card QUANT-INDEX-CHECKS: through a quant-checked formal, the index
        // is decided against the admitted set first.
        if let Some(clear) =
            self.quant_access(node, kind, position, &target, &index, formation, state)
        {
            return clear;
        }
        let (decision, subject) = self.decide_access(&target, &index.fact, formation);
        let mut held = index.held.clone();
        if let Target::Known { held: h, .. } = &target {
            held.extend(h.iter().cloned());
        }
        let clear = decision == Decision::InBounds && held.is_empty();
        self.record(node, kind, position, subject, decision, &held, reach, state);
        clear
    }

    /// Walk one expression, recording every obligation. `reach` says whether
    /// an access here is definitely reached; returns whether every recorded
    /// obligation was in bounds with no held premise.
    fn expr(&mut self, node: &Value, state: &State, position: &str, reach: bool) -> bool {
        let node = strip(node);
        let node_kind = kind(node);
        if node_kind.is_empty() {
            return true;
        }
        if !READ_EXPRESSIONS.contains(&node_kind) {
            self.reason(format!("bounds-unsupported-form: expression `{node_kind}`"));
            return false;
        }
        let mut clear = true;
        match node_kind {
            "UnaryExprOrTypeTraitExpr" | "OffsetOfExpr" => {
                // Unevaluated operands, except a variable-length type.
                if matches!(
                    shape(&qual(kids(node).next().unwrap_or(node))),
                    Shape::Variable
                ) || node["argType"]["qualType"]
                    .as_str()
                    .is_some_and(|t| matches!(shape(t), Shape::Variable))
                {
                    self.reason(
                        "bounds-unsupported-form: evaluated `sizeof` of a variable-length type"
                            .into(),
                    );
                    return false;
                }
                return true;
            }
            "ArraySubscriptExpr" => {
                let (base, index) = self.subscript_parts(node);
                let (Some(base), Some(index)) = (base, index) else {
                    self.reason("bounds-unsupported-form: unreadable subscript".into());
                    return false;
                };
                clear &= self.expr(base, state, position, reach);
                clear &= self.expr(index, state, position, reach);
                let target = self.target(base, state);
                let value = self.eval(index, state);
                clear &= self.access(
                    node,
                    "element-access",
                    position,
                    target,
                    value,
                    false,
                    reach,
                    state,
                );
                return clear;
            }
            "UnaryOperator" => {
                let opcode = node["opcode"].as_str().unwrap_or("");
                let operand = kids(node).next();
                match (opcode, operand) {
                    ("&", Some(operand)) => {
                        let operand = strip(operand);
                        match kind(operand) {
                            // `&a[i]` forms `a + i`; `&*p` is `p` (C11 6.5.3.2p3).
                            "ArraySubscriptExpr" => {
                                let (base, index) = self.subscript_parts(operand);
                                let (Some(base), Some(index)) = (base, index) else {
                                    return false;
                                };
                                clear &= self.expr(base, state, position, reach);
                                clear &= self.expr(index, state, position, reach);
                                let target = self.target(base, state);
                                let value = self.eval(index, state);
                                clear &= self.access(
                                    node,
                                    "formation",
                                    position,
                                    target,
                                    value,
                                    true,
                                    reach,
                                    state,
                                );
                                return clear;
                            }
                            "UnaryOperator" if operand["opcode"] == "*" => {
                                return kids(operand)
                                    .next()
                                    .is_none_or(|p| self.expr(p, state, position, reach));
                            }
                            _ => return self.expr(operand, state, position, reach),
                        }
                    }
                    ("*", Some(pointer)) => {
                        clear &= self.expr(pointer, state, position, reach);
                        let target = self.target(pointer, state);
                        clear &= self.access(
                            node,
                            "dereference",
                            position,
                            target,
                            Val::constant(0),
                            false,
                            reach,
                            state,
                        );
                        return clear;
                    }
                    ("++" | "--", Some(operand)) if is_pointer(&qual(operand)) => {
                        clear &= self.expr(operand, state, position, reach);
                        let target = self.target(operand, state);
                        let step = Val::constant(if opcode == "++" { 1 } else { -1 });
                        clear &= self.access(
                            node,
                            "formation",
                            position,
                            target,
                            step,
                            true,
                            reach,
                            state,
                        );
                        return clear;
                    }
                    _ => {}
                }
            }
            "MemberExpr" if node["isArrow"] == true => {
                let Some(base) = kids(node).next() else {
                    return false;
                };
                clear &= self.expr(base, state, position, reach);
                let target = self.target(base, state);
                clear &= self.access(
                    node,
                    "member-access",
                    position,
                    target,
                    Val::constant(0),
                    false,
                    reach,
                    state,
                );
                return clear;
            }
            "BinaryOperator" | "CompoundAssignOperator" => {
                let opcode = node["opcode"].as_str().unwrap_or("");
                let mut operands = kids(node);
                let (Some(left), Some(right)) = (operands.next(), operands.next()) else {
                    return false;
                };
                let (lp, rp) = (is_pointer(&qual(left)), is_pointer(&qual(right)));
                match opcode {
                    "&&" | "||" => {
                        clear &= self.expr(left, state, position, reach);
                        self.conditional += 1;
                        clear &= self.expr(right, state, position, false);
                        self.conditional -= 1;
                        return clear;
                    }
                    "+" | "-" | "+=" | "-=" if lp != rp => {
                        clear &= self.expr(left, state, position, reach);
                        clear &= self.expr(right, state, position, reach);
                        let (pointer, integer) = if lp { (left, right) } else { (right, left) };
                        let target = self.target(pointer, state);
                        let mut value = self.eval(integer, state);
                        if opcode.starts_with('-') {
                            value.fact = negate(&value.fact);
                        }
                        clear &= self.access(
                            node,
                            "formation",
                            position,
                            target,
                            value,
                            true,
                            reach,
                            state,
                        );
                        return clear;
                    }
                    "-" | "<" | "<=" | ">" | ">=" if lp && rp => {
                        clear &= self.expr(left, state, position, reach);
                        clear &= self.expr(right, state, position, reach);
                        let (a, b) = (self.target(left, state), self.target(right, state));
                        clear &= self.same_object(node, position, &a, &b, reach, state);
                        return clear;
                    }
                    _ => {}
                }
            }
            "ConditionalOperator" => {
                let mut operands = kids(node);
                if let Some(condition) = operands.next() {
                    clear &= self.expr(condition, state, position, reach);
                }
                self.conditional += 1;
                for arm in operands {
                    clear &= self.expr(arm, state, position, false);
                }
                self.conditional -= 1;
                return clear;
            }
            "CallExpr" => return self.call(node, state, position, reach),
            _ => {}
        }
        for child in kids(node) {
            clear &= self.expr(child, state, position, reach);
        }
        clear
    }

    fn same_object(
        &mut self,
        node: &Value,
        position: &str,
        a: &Target,
        b: &Target,
        reach: bool,
        state: &State,
    ) -> bool {
        let (oa, da, ha) = match a {
            Target::Known {
                object,
                distinct,
                held,
                ..
            } => (Some(object.clone()), *distinct, held.clone()),
            Target::Missing(_) => (None, false, BTreeSet::new()),
        };
        let (ob, db, hb) = match b {
            Target::Known {
                object,
                distinct,
                held,
                ..
            } => (Some(object.clone()), *distinct, held.clone()),
            Target::Missing(_) => (None, false, BTreeSet::new()),
        };
        let decision = match core::same_object(oa.as_ref(), ob.as_ref(), da && db) {
            SameObject::Same => Decision::InBounds,
            SameObject::Different => Decision::OutOfBounds,
            SameObject::Unresolved => Decision::Undecided(Refusal::MissingExtent),
        };
        let held: BTreeSet<String> = ha.union(&hb).cloned().collect();
        let clear = decision == Decision::InBounds && held.is_empty();
        let subject = format!(
            "{} vs {}",
            oa.as_deref().unwrap_or("an unresolved object"),
            ob.as_deref().unwrap_or("an unresolved object")
        );
        self.record(
            node,
            "same-object",
            position,
            subject,
            decision,
            &held,
            reach,
            state,
        );
        clear
    }

    /// A call: its callee and actuals are walked; every pointer actual is a
    /// call-site obligation against the callee's bounds contract.
    fn call(&mut self, node: &Value, state: &State, position: &str, reach: bool) -> bool {
        let mut parts = kids(node);
        let callee = parts.next();
        let arguments = parts.collect::<Vec<_>>();
        let mut clear = true;
        let name = callee.and_then(|c| {
            let c = strip(c);
            (kind(c) == "ImplicitCastExpr" && c["castKind"] == "FunctionToPointerDecay")
                .then(|| kids(c).next().and_then(decl_ref))
                .flatten()
                .filter(|d| d.1 == "FunctionDecl")
                .map(|d| d.2.to_owned())
        });
        if name.is_none()
            && let Some(callee) = callee
        {
            clear &= self.expr(callee, state, position, reach);
        }
        for argument in &arguments {
            clear &= self.expr(argument, state, "actual", reach);
        }
        let contract = name.as_deref().and_then(|n| self.contracts.contract(n));
        let reach = reach || (self.call_reach && self.conditional == 0);
        let callee_label = name.clone().unwrap_or_else(|| "an indirect callee".into());
        let Some(contract) = contract else {
            for (position_index, argument) in arguments.iter().enumerate() {
                if !is_pointer(&qual(argument)) {
                    continue;
                }
                self.record(
                    argument,
                    "call-precondition",
                    "actual",
                    format!(
                        "`{callee_label}` argument {} is a pointer and `{callee_label}` states no bounds contract",
                        position_index + 1
                    ),
                    Decision::Undecided(Refusal::MissingExtent),
                    &BTreeSet::new(),
                    reach,
                    state,
                );
                clear = false;
            }
            return clear;
        };
        clear &= self.quant_escapes(contract, &arguments, state, reach);
        clear & self.callee_preconditions(node, contract, &arguments, state, reach)
    }

    fn callee_holds(&self, contract: &BoundsFunctionContract) -> BTreeSet<String> {
        if contract.held() {
            contract.holds().into_iter().collect()
        } else {
            BTreeSet::new()
        }
    }

    fn actual_value(
        &self,
        contract: &BoundsFunctionContract,
        arguments: &[&Value],
        count: &BoundsCount,
        state: &State,
    ) -> Option<Val> {
        match count {
            BoundsCount::Const(n) => Some(Val::constant(*n as i128)),
            BoundsCount::Parameter(name) => {
                let (index, _) = contract.parameter(name)?;
                Some(self.eval(arguments.get(index)?, state))
            }
        }
    }

    fn callee_preconditions(
        &mut self,
        node: &Value,
        contract: &BoundsFunctionContract,
        arguments: &[&Value],
        state: &State,
        reach: bool,
    ) -> bool {
        let holds = self.callee_holds(contract);
        let mut clear = true;
        if !call_abi_matches(contract, arguments) {
            self.record(
                node,
                "call-precondition",
                "actual",
                format!(
                    "`{}`'s bounds contract does not match the call's arguments",
                    contract.name
                ),
                Decision::Undecided(Refusal::SymbolMismatch),
                &BTreeSet::new(),
                reach,
                state,
            );
            return false;
        }
        for entry in &contract.clauses {
            match &entry.clause {
                BoundsClause::Extent {
                    parameter,
                    count,
                    unit,
                } => {
                    let Some((index, _)) = contract.parameter(parameter) else {
                        continue;
                    };
                    let target = self.target(arguments[index], state);
                    let required = self
                        .actual_value(contract, arguments, count, state)
                        .unwrap_or_else(Val::unknown);
                    let decision = self.covers(&target, &required, *unit);
                    let mut held = holds.clone();
                    held.extend(required.held.iter().cloned());
                    if let Target::Known { held: h, .. } = &target {
                        held.extend(h.iter().cloned());
                    }
                    clear &= decision == Decision::InBounds && held.is_empty();
                    self.record(
                        arguments[index],
                        "call-precondition",
                        "actual",
                        format!("`{}` requires {}", contract.name, entry.text),
                        decision,
                        &held,
                        reach,
                        state,
                    );
                }
                BoundsClause::Index { parameter, count } => {
                    let Some((index, _)) = contract.parameter(parameter) else {
                        continue;
                    };
                    let value = self.eval(arguments[index], state);
                    let bound = self
                        .actual_value(contract, arguments, count, state)
                        .unwrap_or_else(Val::unknown);
                    let count = match (&bound.fact, &bound.symbol) {
                        (IndexFact::Const(n), _) if *n >= 0 => Some(Count::Const(*n as u128)),
                        (_, Some(symbol)) => Some(Count::Symbol(symbol.clone())),
                        _ => None,
                    };
                    let decision = core::element_access(count.as_ref(), &value.fact);
                    let mut held = holds.clone();
                    held.extend(value.held.iter().cloned());
                    held.extend(bound.held.iter().cloned());
                    clear &= decision == Decision::InBounds && held.is_empty();
                    self.record(
                        arguments[index],
                        "call-precondition",
                        "actual",
                        format!("`{}` requires {}", contract.name, entry.text),
                        decision,
                        &held,
                        reach,
                        state,
                    );
                }
                BoundsClause::QuantCheck {
                    parameter,
                    predicate,
                    ..
                } => {
                    clear &= self.quant_call_obligation(
                        contract, entry, parameter, predicate, arguments, &holds, state, reach,
                    );
                }
                _ => {}
            }
        }
        clear
    }

    /// The target can hold `required` units from its offset: forming
    /// `target + required` is valid (a quant object's extent is a lower
    /// bound, card QUANT-INDEX-CHECKS).
    fn covers(&self, target: &Target, required: &Val, unit: BoundsUnit) -> Decision {
        self.quant_lower_bound_only(target, self.covers_extent(target, required, unit))
    }

    fn covers_extent(&self, target: &Target, required: &Val, unit: BoundsUnit) -> Decision {
        let Target::Known {
            count,
            unit: have,
            offset,
            element,
            ..
        } = target
        else {
            return Decision::Undecided(Refusal::MissingExtent);
        };
        let want = match unit {
            BoundsUnit::Elements => Unit2::Elements,
            BoundsUnit::Bytes => Unit2::Bytes,
        };
        // A stable symbol at offset zero against the same symbol.
        if let (Some(symbol), Count::Symbol(have_symbol), IndexFact::Const(0)) =
            (&required.symbol, count, offset)
            && symbol == have_symbol
            && *have == want
        {
            return Decision::InBounds;
        }
        let IndexFact::Const(r) = required.fact else {
            return Decision::Undecided(Refusal::UnsupportedRange);
        };
        if r < 0 {
            return Decision::Undecided(Refusal::UnsupportedRange);
        }
        if *have == want {
            return core::formation(Some(count), &add_offsets(offset, &IndexFact::Const(r)));
        }
        // Elements held, bytes required (or the reverse): convert by the
        // element size when every quantity is a constant.
        let (Some(width), Count::Const(n), IndexFact::Const(o)) =
            (element_size(self.target, element), count, offset)
        else {
            return Decision::Undecided(Refusal::UnsupportedRange);
        };
        let limit = self.target.ptrdiff_max;
        match have {
            Unit2::Elements => {
                let (Ok(capacity), Ok(start)) = (
                    core::scale(*n as i128, width, limit),
                    core::scale(*o, width, limit),
                ) else {
                    return Decision::Undecided(Refusal::Overflow);
                };
                match start.checked_add(r) {
                    Some(end) if start >= 0 && end <= capacity => Decision::InBounds,
                    Some(_) => Decision::OutOfBounds,
                    None => Decision::Undecided(Refusal::Overflow),
                }
            }
            Unit2::Bytes => Decision::Undecided(Refusal::UnsupportedRange),
        }
    }

    // -- statements -----------------------------------------------------------

    fn full_expression(&mut self, node: &Value, state: &mut State, position: &str) {
        self.check_own_frame(node, state);
        let written = self.written(node);
        // A whole-statement step `v++`, `v += k` reads the old value of `v`
        // and writes it last: its own fact stays readable for the step.
        let step_of = {
            let top = strip(node);
            let is_step = matches!(
                (kind(top), top["opcode"].as_str()),
                ("UnaryOperator", Some("++" | "--"))
                    | ("CompoundAssignOperator", Some("+=" | "-="))
            );
            is_step
                .then(|| kids(top).next().and_then(decl_ref).map(|d| d.0.to_owned()))
                .flatten()
                .filter(|v| written.len() == 1 && written.contains(v))
        };
        let mut reading = state.clone();
        for id in &written {
            if step_of.as_deref() != Some(id.as_str()) {
                reading.write(id);
            }
        }
        let calls = has_call(node);
        let reach = state.definite && state.live && !calls;
        self.call_reach = state.definite && state.live && call_count(node) == 1;
        let clear = self.expr(node, &reading, position, reach);
        self.call_reach = false;
        // The new facts of a simple assignment or increment.
        let mut updates: Vec<(String, Update)> = Vec::new();
        self.assignment_updates(node, state, &reading, &mut updates);
        for id in &written {
            state.write(id);
        }
        if calls {
            self.apply_calls(node, state);
        }
        for (id, update) in updates {
            self.apply(state, &id, update);
        }
        if calls || !clear {
            state.definite = false;
        }
    }

    /// Card CG24: the frame a call applies, and the holds of a held one.
    fn frame_of(&self, call: &Value) -> (CallFrame, BTreeSet<String>) {
        let arguments = kids(call).skip(1).collect::<Vec<_>>();
        let Some(contract) = callee_name(call)
            .and_then(|name| self.contracts.contract(&name))
            .filter(|c| call_abi_matches(c, &arguments))
        else {
            return (CallFrame::Unstated, BTreeSet::new());
        };
        match contract.frame() {
            Some(modes) => {
                let touches = modes
                    .iter()
                    .zip(&arguments)
                    .filter_map(|(mode, argument)| {
                        mode.map(|mode| FrameTouch {
                            mode,
                            addr: address_actual(argument)
                                .filter(|id| self.tracked.contains(*id))
                                .map(str::to_owned),
                        })
                    })
                    .collect();
                (
                    CallFrame::Closed {
                        touches,
                        held: contract.held(),
                    },
                    self.callee_holds(contract),
                )
            }
            None if !contract.held()
                && contract
                    .clauses
                    .iter()
                    .any(|e| e.clause == BoundsClause::PreservesExtents) =>
            {
                (CallFrame::PreservesExtents, BTreeSet::new())
            }
            None => (CallFrame::Unstated, BTreeSet::new()),
        }
    }

    /// Every call of a full expression, in order: its frame withdraws (or
    /// keeps, on a held premise) the facts it may touch.
    fn apply_calls(&self, node: &Value, state: &mut State) {
        let mut calls = Vec::new();
        collect_calls(node, &mut calls);
        for call in calls {
            let (frame, holds) = self.frame_of(call);
            if let CallFrame::Closed { touches, .. } = &frame {
                for touch in touches {
                    if touch.mode != FrameMode::Reads
                        && let Some(id) = &touch.addr
                    {
                        state.write(id);
                    }
                }
            }
            for name in state.facts.call_frame(&frame) {
                state
                    .held
                    .entry(name)
                    .or_default()
                    .extend(holds.iter().cloned());
            }
            state.sync();
        }
    }

    /// The variables a node writes: assignments and steps (CG16), and the
    /// locals whose address a write or move frame position receives (CG24).
    fn written(&self, node: &Value) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        writes(node, &mut out);
        let mut calls = Vec::new();
        collect_calls(node, &mut calls);
        for call in calls {
            if let (CallFrame::Closed { touches, .. }, _) = self.frame_of(call) {
                out.extend(
                    touches
                        .into_iter()
                        .filter(|t| t.mode != FrameMode::Reads)
                        .filter_map(|t| t.addr),
                );
            }
        }
        out
    }

    /// Card CG24: a body that exceeds the frame its own contract declares
    /// (a write through a parameter declared read, a hand-off of a parameter
    /// to a stronger frame position) refutes the declaration.
    fn check_own_frame(&mut self, node: &Value, state: &State) {
        let Some(own) = self.own else {
            return;
        };
        let Some(modes) = own.frame() else {
            return;
        };
        if !state.live {
            return;
        }
        let mut effects = Vec::new();
        body_effects(node, self.contracts, &mut effects);
        for (id, effect) in effects {
            if !state.stable.contains(&id) {
                continue;
            }
            let Some(name) = self.params.get(&id) else {
                continue;
            };
            let Some(mode) = own.parameter(name).and_then(|(index, _)| modes[index]) else {
                continue;
            };
            if frames::refutes(mode, effect) {
                self.refuted = true;
                self.reason(format!(
                    "bounds-contract-refuted-by-body: `{}` {} `{name}`, declared bounds_frame_{}",
                    own.name,
                    match effect {
                        BodyEffect::Moved => "hands to a moving frame position",
                        _ => "writes through",
                    },
                    mode.spelling()
                ));
            }
        }
    }

    fn assignment_updates(
        &mut self,
        node: &Value,
        pre: &State,
        reading: &State,
        out: &mut Vec<(String, Update)>,
    ) {
        let node = strip(node);
        match kind(node) {
            "BinaryOperator" if node["opcode"] == "=" => {
                let mut operands = kids(node);
                let (Some(left), Some(right)) = (operands.next(), operands.next()) else {
                    return;
                };
                let Some((id, _, _)) = decl_ref(left) else {
                    return;
                };
                let inner = self.written(right);
                if !inner.is_empty() || !self.tracked.contains(id) {
                    return;
                }
                let update = self.value_update(left, right, reading);
                out.push((id.to_owned(), update));
            }
            "UnaryOperator" | "CompoundAssignOperator"
                if matches!(node["opcode"].as_str(), Some("++" | "--" | "+=" | "-=")) =>
            {
                let mut operands = kids(node);
                let Some(left) = operands.next() else {
                    return;
                };
                let Some((id, _, _)) = decl_ref(left) else {
                    return;
                };
                if !self.tracked.contains(id) {
                    return;
                }
                let step = match node["opcode"].as_str() {
                    Some("++") => Val::constant(1),
                    Some("--") => Val::constant(-1),
                    Some("+=") => operands
                        .next()
                        .map_or_else(Val::unknown, |r| self.eval(r, reading)),
                    _ => {
                        let v = operands
                            .next()
                            .map_or_else(Val::unknown, |r| self.eval(r, reading));
                        Val {
                            fact: negate(&v.fact),
                            ..v
                        }
                    }
                };
                // A step from the variable's own fact before the write.
                let update = if let Some(fact) = pre.facts.pointers.get(id) {
                    let mut held = pre.held.get(id).cloned().unwrap_or_default();
                    held.extend(step.held.iter().cloned());
                    match add_offsets(&fact.offset, &step.fact) {
                        IndexFact::Unknown => Update::None,
                        offset => Update::Pointer(
                            PointerFact {
                                offset,
                                ..fact.clone()
                            },
                            pre.elements.get(id).cloned().unwrap_or_default(),
                            held,
                        ),
                    }
                } else if let (
                    Some(ScalarFact {
                        value: IndexFact::Const(v),
                        ..
                    }),
                    IndexFact::Const(d),
                ) = (pre.facts.scalars.get(id), &step.fact)
                {
                    let mut held = pre.held.get(id).cloned().unwrap_or_default();
                    held.extend(step.held.iter().cloned());
                    match v.checked_add(*d) {
                        Some(sum) => {
                            let value = self.in_type(sum, &qual(left), held);
                            match value.fact {
                                IndexFact::Unknown => Update::None,
                                fact => Update::Scalar(fact, value.held),
                            }
                        }
                        None => Update::None,
                    }
                } else {
                    Update::None
                };
                out.push((id.to_owned(), update));
            }
            _ => {}
        }
    }

    fn value_update(&mut self, left: &Value, right: &Value, reading: &State) -> Update {
        if is_pointer(&qual(left)) {
            match self.target(right, reading) {
                Target::Known {
                    object,
                    count,
                    unit,
                    offset,
                    element,
                    held,
                    source_object,
                    ..
                } => {
                    let element_now = pointee(&qual(left)).unwrap_or_default();
                    // Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (rule
                    // 14g): a byte view of a quant object keeps its target.
                    let element = if strip_qualifiers(&element_now) == element {
                        element
                    } else if self.quant.contains_key(&object)
                        && quant_checks::byte_types(
                            &self.desugared(&element_now),
                            &self.desugared(&element),
                        )
                    {
                        strip_qualifiers(&element_now)
                    } else {
                        return Update::None;
                    };
                    Update::Pointer(
                        PointerFact {
                            object,
                            count,
                            unit,
                            offset,
                            standing: if held.is_empty() {
                                Standing::Source
                            } else {
                                Standing::Held
                            },
                            source_object,
                        },
                        element,
                        held,
                    )
                }
                Target::Missing(_) => self
                    .return_extent(right, reading)
                    .or_else(|| self.return_derived(right, reading))
                    .unwrap_or(Update::None),
            }
        } else {
            let value = self.eval(right, reading);
            let value = self.convert(value, &qual(right), &qual(left));
            match value.fact {
                IndexFact::Unknown => Update::None,
                fact => Update::Scalar(fact, value.held),
            }
        }
    }

    /// `p = f(...)` with `f`'s `bounds_return_extent`, used only when every
    /// precondition of the call is established (held premises carried).
    fn return_extent(&mut self, right: &Value, reading: &State) -> Option<Update> {
        let call = strip(right);
        let call = if kind(call) == "ImplicitCastExpr" && call["castKind"] == "NoOp" {
            kids(call).next().map(strip)?
        } else {
            call
        };
        if kind(call) != "CallExpr" {
            return None;
        }
        let mut parts = kids(call);
        let callee = strip(parts.next()?);
        let name = kids(callee).next().and_then(decl_ref)?.2.to_owned();
        let arguments = parts.collect::<Vec<_>>();
        let contract = self.contracts.contract(&name)?;
        let (count, unit) = contract.clauses.iter().find_map(|e| match &e.clause {
            BoundsClause::ReturnExtent { count, unit } => Some((count.clone(), *unit)),
            _ => None,
        })?;
        // Preconditions first: re-decide them silently.
        let mut held = self.callee_holds(contract);
        self.preconditions_established(contract, &arguments, reading, &mut held)?;
        let value = self.actual_value(contract, &arguments, &count, reading)?;
        held.extend(value.held.iter().cloned());
        let count = match (&value.fact, &value.symbol) {
            (IndexFact::Const(n), _) if *n >= 0 => Count::Const(*n as u128),
            (_, Some(symbol)) => Count::Symbol(symbol.clone()),
            _ => return None,
        };
        let element = pointee(&qual(call))
            .map(|t| strip_qualifiers(&t))
            .unwrap_or_default();
        for clause in contract
            .clauses
            .iter()
            .filter(|e| matches!(e.clause, BoundsClause::ReturnExtent { .. }))
        {
            self.assumptions.insert(format!(
                "{}: `{}` {}",
                if held.is_empty() { "declared" } else { "held" },
                contract.name,
                clause.text
            ));
        }
        Some(Update::Pointer(
            PointerFact {
                object: format!("result of `{name}`#{}", call["id"].as_str().unwrap_or("?")),
                count,
                unit: match unit {
                    BoundsUnit::Elements => Unit2::Elements,
                    BoundsUnit::Bytes => Unit2::Bytes,
                },
                offset: IndexFact::Const(0),
                standing: if held.is_empty() {
                    Standing::Resolved
                } else {
                    Standing::Held
                },
                source_object: false,
            },
            element,
            held,
        ))
    }

    /// The callee's preconditions at this call, re-decided silently: `None`
    /// unless every one is established (held premises carried in `held`).
    fn preconditions_established(
        &mut self,
        contract: &BoundsFunctionContract,
        arguments: &[&Value],
        reading: &State,
        held: &mut BTreeSet<String>,
    ) -> Option<()> {
        for entry in &contract.clauses {
            let established = match &entry.clause {
                BoundsClause::Extent {
                    parameter,
                    count,
                    unit,
                } => {
                    let (index, _) = contract.parameter(parameter)?;
                    let target = self.target(arguments.get(index)?, reading);
                    if let Target::Known { held: h, .. } = &target {
                        held.extend(h.iter().cloned());
                    }
                    let required = self.actual_value(contract, arguments, count, reading)?;
                    held.extend(required.held.iter().cloned());
                    self.covers(&target, &required, *unit) == Decision::InBounds
                }
                BoundsClause::Index { parameter, count } => {
                    let (index, _) = contract.parameter(parameter)?;
                    let value = self.eval(arguments.get(index)?, reading);
                    let bound = self.actual_value(contract, arguments, count, reading)?;
                    held.extend(value.held.iter().cloned());
                    let count = match (&bound.fact, &bound.symbol) {
                        (IndexFact::Const(n), _) if *n >= 0 => Count::Const(*n as u128),
                        (_, Some(symbol)) => Count::Symbol(symbol.clone()),
                        _ => return None,
                    };
                    core::element_access(Some(&count), &value.fact) == Decision::InBounds
                }
                BoundsClause::QuantCheck {
                    parameter,
                    predicate,
                    ..
                } => {
                    self.quant_established(contract, parameter, predicate, arguments, reading, held)
                }
                _ => true,
            };
            if !established {
                return None;
            }
        }
        Some(())
    }

    /// Card CG24: `p = f(...)` with `f`'s `bounds_return_derived` relations,
    /// used only when every precondition of the call is established. One
    /// base whose extent decides containment: the result is inside it (unless
    /// the call may move it). A base too small: a reported contradiction and
    /// no fact. Otherwise the declared view, when every relation states the
    /// same one.
    fn return_derived(&mut self, right: &Value, reading: &State) -> Option<Update> {
        let call = strip(right);
        let call = if kind(call) == "ImplicitCastExpr" && call["castKind"] == "NoOp" {
            kids(call).next().map(strip)?
        } else {
            call
        };
        if kind(call) != "CallExpr" {
            return None;
        }
        let name = callee_name(call)?;
        let arguments = kids(call).skip(1).collect::<Vec<_>>();
        let contract = self.contracts.contract(&name)?;
        if !call_abi_matches(contract, &arguments) {
            return None;
        }
        let relations = contract
            .clauses
            .iter()
            .filter_map(|e| match &e.clause {
                BoundsClause::ReturnDerived {
                    parameter,
                    offset,
                    length,
                    unit,
                } => Some((
                    parameter.clone(),
                    offset.clone(),
                    length.clone(),
                    *unit,
                    e.text.clone(),
                )),
                _ => None,
            })
            .collect::<Vec<_>>();
        if relations.is_empty() {
            return None;
        }
        let mut held = self.callee_holds(contract);
        self.preconditions_established(contract, &arguments, reading, &mut held)?;
        let element = pointee(&qual(call))
            .map(|t| strip_qualifiers(&t))
            .unwrap_or_default();
        let may_move = match self.frame_of(call).0 {
            CallFrame::Unstated => true,
            CallFrame::PreservesExtents => false,
            CallFrame::Closed { touches, .. } => touches.iter().any(|t| t.mode == FrameMode::Moves),
        };
        let mut decisions = Vec::new();
        let mut views = BTreeSet::new();
        let mut inside = None;
        for (parameter, offset, length, unit, text) in &relations {
            let (index, _) = contract.parameter(parameter)?;
            let base = self.target(arguments.get(index)?, reading);
            let off = self.actual_value(contract, &arguments, offset, reading)?;
            let len = self.actual_value(contract, &arguments, length, reading)?;
            held.extend(off.held.iter().cloned());
            held.extend(len.held.iter().cloned());
            let length = match (&len.fact, &len.symbol) {
                (IndexFact::Const(n), _) if *n >= 0 => Count::Const(*n as u128),
                (_, Some(symbol)) => Count::Symbol(symbol.clone()),
                _ => return None,
            };
            let want = match unit {
                BoundsUnit::Elements => Unit2::Elements,
                BoundsUnit::Bytes => Unit2::Bytes,
            };
            views.insert((length.clone(), want));
            let decision = match &base {
                Target::Known {
                    count,
                    unit: have,
                    offset: base_offset,
                    ..
                } if *have == want => {
                    let start = add_offsets(base_offset, &off.fact);
                    let decision = frames::derived_within(count, &start, &length);
                    if decision == Decision::InBounds && relations.len() == 1 {
                        inside = Some((base.clone(), start));
                    }
                    decision
                }
                Target::Known { .. } => Decision::Undecided(Refusal::UnsupportedRange),
                Target::Missing(_) => Decision::Undecided(Refusal::MissingExtent),
            };
            if decision == Decision::OutOfBounds {
                self.reason(format!(
                    "bounds-declaration-contradicted: `{name}`'s derived result {text} exceeds the extent of its base"
                ));
            }
            decisions.push(decision);
        }
        if !frames::multi_base_view(&decisions) {
            return None;
        }
        if let Some((
            Target::Known {
                object,
                count,
                unit,
                element: base_element,
                held: base_held,
                source_object,
                ..
            },
            start,
        )) = inside
            && (source_object || !may_move)
            && base_element == element
        {
            held.extend(base_held);
            self.relation_assumptions(contract, &held);
            return Some(Update::Pointer(
                PointerFact {
                    object,
                    count,
                    unit,
                    offset: start,
                    standing: if held.is_empty() {
                        Standing::Resolved
                    } else {
                        Standing::Held
                    },
                    source_object,
                },
                element,
                held,
            ));
        }
        if views.len() != 1 {
            return None;
        }
        let (count, unit) = views.into_iter().next()?;
        self.relation_assumptions(contract, &held);
        Some(Update::Pointer(
            PointerFact {
                object: format!(
                    "derived result of `{name}`#{}",
                    call["id"].as_str().unwrap_or("?")
                ),
                count,
                unit,
                offset: IndexFact::Const(0),
                standing: if held.is_empty() {
                    Standing::Resolved
                } else {
                    Standing::Held
                },
                source_object: false,
            },
            element,
            held,
        ))
    }

    fn relation_assumptions(&mut self, contract: &BoundsFunctionContract, held: &BTreeSet<String>) {
        for clause in contract
            .clauses
            .iter()
            .filter(|e| matches!(e.clause, BoundsClause::ReturnDerived { .. }))
        {
            self.assumptions.insert(format!(
                "{}: `{}` {}",
                if held.is_empty() { "declared" } else { "held" },
                contract.name,
                clause.text
            ));
        }
    }

    fn apply(&self, state: &mut State, id: &str, update: Update) {
        match update {
            Update::None => {}
            Update::Scalar(fact, held) => {
                state.facts.scalars.insert(
                    id.to_owned(),
                    ScalarFact {
                        value: fact,
                        standing: if held.is_empty() {
                            Standing::Source
                        } else {
                            Standing::Held
                        },
                    },
                );
                if !held.is_empty() {
                    state.held.insert(id.to_owned(), held);
                }
            }
            Update::Pointer(fact, element, held) => {
                state.facts.pointers.insert(id.to_owned(), fact);
                state.elements.insert(id.to_owned(), element);
                if !held.is_empty() {
                    state.held.insert(id.to_owned(), held);
                }
            }
        }
    }

    fn stmt(&mut self, node: &Value, state: &mut State) {
        let node_kind = kind(node);
        if node_kind.is_empty() {
            return;
        }
        if !state.live && !has_entry(node) {
            // Proved unreachable (no label or case can enter it): excluded,
            // still inventoried.
            self.excluded(node, state);
            return;
        }
        match node_kind {
            "CompoundStmt" => {
                for child in kids(node) {
                    self.stmt(child, state);
                }
            }
            "NullStmt" => {}
            "AttributedStmt" => {
                for child in kids(node).filter(|c| !kind(c).ends_with("Attr")) {
                    self.stmt(child, state);
                }
            }
            "DeclStmt" => {
                for decl in kids(node) {
                    match kind(decl) {
                        "VarDecl" => self.declaration(decl, state),
                        "RecordDecl" | "EnumDecl" | "TypedefDecl" | "StaticAssertDecl" => {}
                        other => {
                            self.reason(format!("bounds-unsupported-form: declaration `{other}`"))
                        }
                    }
                }
            }
            "ReturnStmt" => {
                if let Some(value) = kids(node).next() {
                    let before = state.clone();
                    self.full_expression(value, state, "return");
                    self.check_return_extent(value, &before);
                    self.check_return_derived(value, &before);
                }
                *state = state.dead();
            }
            "BreakStmt" | "ContinueStmt" | "GotoStmt" => *state = state.dead(),
            "IfStmt" => self.if_stmt(node, state),
            "WhileStmt" | "DoStmt" | "ForStmt" => self.loop_stmt(node, state),
            "SwitchStmt" => self.switch_stmt(node, state),
            "CaseStmt" | "DefaultStmt" => {
                let entry = self.switch_entries.last().cloned();
                match entry {
                    Some(entry) => *state = state.join(&entry),
                    None => {
                        self.reason("bounds-unsupported-form: a case label outside a switch".into())
                    }
                }
                state.definite = false;
                let substatement = if node_kind == "CaseStmt" {
                    kids(node).nth(1)
                } else {
                    kids(node).next()
                };
                if let Some(substatement) = substatement {
                    self.stmt(substatement, state);
                }
            }
            "LabelStmt" => {
                *state = self.top_state();
                if let Some(substatement) = kids(node).next() {
                    self.stmt(substatement, state);
                }
            }
            kind if READ_EXPRESSIONS.contains(&kind) => {
                self.full_expression(node, state, "statement")
            }
            other => {
                self.reason(format!("bounds-unsupported-form: statement `{other}`"));
                state.definite = false;
            }
        }
    }

    /// Inventory a proved-unreachable statement's obligations as excluded.
    /// A function's own `bounds_return_extent` against what each `return`
    /// returns. A return the body establishes as smaller **refutes** the
    /// declaration: the defining row keeps the refutation, and every caller
    /// sees the callee as uncontracted (the declaration is withdrawn from
    /// dependent uses; re-approving the text cannot discharge it).
    fn check_return_extent(&mut self, value: &Value, state: &State) {
        let Some(own) = self.own else {
            return;
        };
        let Some((count, unit)) = own.clauses.iter().find_map(|e| match &e.clause {
            BoundsClause::ReturnExtent { count, unit } => Some((count.clone(), *unit)),
            _ => None,
        }) else {
            return;
        };
        if !state.live || !is_pointer(&qual(value)) {
            return;
        }
        let required = match &count {
            BoundsCount::Const(n) => Val::constant(*n as i128),
            BoundsCount::Parameter(name) => {
                let id = self
                    .params
                    .iter()
                    .find(|(_, p)| *p == name)
                    .map(|(id, _)| id.clone());
                match id {
                    Some(id) if state.stable.contains(&id) => Val {
                        fact: IndexFact::Unknown,
                        held: BTreeSet::new(),
                        symbol: Some(id),
                    },
                    _ => Val::unknown(),
                }
            }
        };
        let target = self.target(value, state);
        let source_only =
            matches!(&target, Target::Known { source_object: true, held, .. } if held.is_empty());
        match self.covers(&target, &required, unit) {
            Decision::OutOfBounds if source_only => {
                self.refuted = true;
                self.reason(format!(
                    "bounds-contract-refuted-by-body: `{}` returns an object the source establishes as smaller than its declared bounds_return_extent",
                    own.name
                ));
            }
            // Not refuted: the declaration stays a `Declared` assumption for
            // its callers, exactly as an unchecked custody declaration does.
            _ => {}
        }
    }

    /// Card CG24: a function's own `bounds_return_derived` against what each
    /// `return` returns. A source object, a parameter that is no declared
    /// base, or a declared base at a constant element offset no relation on
    /// it states **refutes** the declaration (withdrawn from the unit's
    /// callers). Anything the body does not establish stays a `Declared`
    /// assumption.
    fn check_return_derived(&mut self, value: &Value, state: &State) {
        let Some(own) = self.own else {
            return;
        };
        let relations = own
            .clauses
            .iter()
            .filter_map(|e| match &e.clause {
                BoundsClause::ReturnDerived {
                    parameter,
                    offset,
                    unit,
                    ..
                } => Some((parameter.clone(), offset.clone(), *unit)),
                _ => None,
            })
            .collect::<Vec<_>>();
        if relations.is_empty() || !state.live || !is_pointer(&qual(value)) {
            return;
        }
        let returned = match self.returned_base(value, state) {
            Some((id, offset)) => {
                let name = self.params.get(&id).cloned().unwrap_or_default();
                let on_base = relations
                    .iter()
                    .filter(|(parameter, ..)| *parameter == name)
                    .collect::<Vec<_>>();
                if on_base.is_empty() {
                    Some(format!("`{name}`, which is no declared base"))
                } else if let Some(offset) = offset
                    && on_base.iter().all(|(_, declared, unit)| {
                        *unit == BoundsUnit::Elements
                            && matches!(declared, BoundsCount::Const(c) if i128::try_from(*c).ok() != Some(offset))
                    })
                {
                    Some(format!("`{name}` + {offset}, at no declared offset"))
                } else {
                    None
                }
            }
            None => matches!(
                self.target(value, state),
                Target::Known { source_object: true, held, .. } if held.is_empty()
            )
            .then(|| "a source object".to_owned()),
        };
        if let Some(what) = returned {
            self.refuted = true;
            self.reason(format!(
                "bounds-contract-refuted-by-body: `{}` returns {what}, contradicting its bounds_return_derived",
                own.name
            ));
        }
    }

    /// The stable parameter a returned pointer is, and its constant element
    /// offset when known.
    fn returned_base(&self, value: &Value, state: &State) -> Option<(String, Option<i128>)> {
        let node = strip(value);
        match kind(node) {
            "ImplicitCastExpr" | "CStyleCastExpr" => match node["castKind"].as_str() {
                Some("LValueToRValue") => {
                    let id = read_of(node)?;
                    (self.params.contains_key(id) && state.stable.contains(id))
                        .then(|| (id.to_owned(), Some(0)))
                }
                Some("NoOp" | "BitCast") => self.returned_base(kids(node).next()?, state),
                _ => None,
            },
            "BinaryOperator" if matches!(node["opcode"].as_str(), Some("+" | "-")) => {
                let mut operands = kids(node);
                let (left, right) = (operands.next()?, operands.next()?);
                let (pointer, integer, negative) = if is_pointer(&qual(left)) {
                    (left, right, node["opcode"] == "-")
                } else {
                    (right, left, false)
                };
                let (id, base) = self.returned_base(pointer, state)?;
                let step = match self.eval(integer, state).fact {
                    IndexFact::Const(c) => Some(if negative { -c } else { c }),
                    _ => None,
                };
                Some((id, base.zip(step).and_then(|(a, b)| a.checked_add(b))))
            }
            "UnaryOperator" if node["opcode"] == "&" => {
                let inner = strip(kids(node).next()?);
                if kind(inner) != "ArraySubscriptExpr" {
                    return None;
                }
                let (base, index) = self.subscript_parts(inner);
                let (id, at) = self.returned_base(base?, state)?;
                let step = match self.eval(index?, state).fact {
                    IndexFact::Const(c) => Some(c),
                    _ => None,
                };
                Some((id, at.zip(step).and_then(|(a, b)| a.checked_add(b))))
            }
            _ => None,
        }
    }

    /// Walk a proved-unreachable statement with a dead state: every
    /// obligation records `Excluded`; an unsupported form keeps its reason.
    fn excluded(&mut self, node: &Value, state: &State) {
        let dead = state.dead();
        match kind(node) {
            kind if READ_EXPRESSIONS.contains(&kind) => {
                self.expr(node, &dead, "statement", false);
            }
            _ => {
                for child in kids(node) {
                    self.excluded(child, &dead);
                }
            }
        }
    }

    /// A label may be entered from anywhere: no fact survives it.
    fn top_state(&self) -> State {
        State {
            facts: Facts::default(),
            held: BTreeMap::new(),
            elements: BTreeMap::new(),
            stable: BTreeSet::new(),
            live: true,
            definite: false,
        }
    }

    fn declaration(&mut self, decl: &Value, state: &mut State) {
        let id = decl["id"].as_str().unwrap_or("").to_owned();
        let static_storage = matches!(decl["storageClass"].as_str(), Some("static" | "extern"));
        if matches!(shape(&qual(decl)), Shape::Variable) {
            // The bound expression is evaluated; its extent is CG17's.
            self.reason(format!(
                "bounds-unsupported-form: variable-length array `{}` (CG17)",
                decl["name"].as_str().unwrap_or("?")
            ));
            state.definite = false;
        }
        let Some(init) = kids(decl).last().filter(|c| !kind(c).ends_with("Attr")) else {
            state.write(&id);
            return;
        };
        if static_storage {
            // A static initializer runs once, before any call: no fact.
            return;
        }
        self.check_own_frame(init, state);
        let calls = has_call(init);
        let reach = state.definite && state.live && !calls;
        self.call_reach = state.definite && state.live && call_count(init) == 1;
        let clear = self.expr(init, state, "declaration", reach);
        self.call_reach = false;
        let inner = self.written(init);
        let update = if self.tracked.contains(&id) && inner.is_empty() {
            self.value_update(decl, init, state)
        } else {
            Update::None
        };
        for written in &inner {
            state.write(written);
        }
        state.write(&id);
        if calls {
            self.apply_calls(init, state);
        }
        self.apply(state, &id, update);
        if calls || !clear {
            state.definite = false;
        }
    }

    // -- guards --------------------------------------------------------------

    fn guard(&self, condition: &Value, truth: bool, state: &mut State) {
        let condition = strip(condition);
        match kind(condition) {
            "BinaryOperator" => {
                let opcode = condition["opcode"].as_str().unwrap_or("");
                let mut operands = kids(condition);
                let (Some(left), Some(right)) = (operands.next(), operands.next()) else {
                    return;
                };
                match (opcode, truth) {
                    ("&&", true) | ("||", false) => {
                        self.guard(left, truth, state);
                        self.guard(right, truth, state);
                    }
                    ("<" | "<=" | ">" | ">=" | "==" | "!=", _) => {
                        self.comparison(opcode, left, right, truth, state);
                    }
                    _ => {}
                }
            }
            "UnaryOperator" if condition["opcode"] == "!" => {
                if let Some(inner) = kids(condition).next() {
                    self.guard(inner, !truth, state);
                }
            }
            "ImplicitCastExpr" if condition["castKind"] == "IntegralCast" => {
                if let Some(inner) = kids(condition).next() {
                    self.guard(inner, truth, state);
                }
            }
            _ => {}
        }
    }

    /// The tracked variable an operand reads, through value-preserving
    /// conversions only, and its type.
    fn guarded_variable(&self, operand: &Value) -> Option<(String, String)> {
        let operand = strip(operand);
        if let Some(id) = read_of(operand) {
            if self.tracked.contains(id) {
                return Some((id.to_owned(), strip_qualifiers(&qual(operand))));
            }
            return None;
        }
        if kind(operand) == "ImplicitCastExpr" && operand["castKind"] == "IntegralCast" {
            let inner = kids(operand).next()?;
            let (id, inner_type) = self.guarded_variable(inner)?;
            let (fw, fs) = integer_type(self.target, &inner_type)?;
            let (tw, ts) = integer_type(self.target, &strip_qualifiers(&qual(operand)))?;
            let (fmin, fmax) = type_range(fw, fs);
            let (tmin, tmax) = type_range(tw, ts);
            if fmin >= tmin && fmax <= tmax {
                return Some((id, inner_type));
            }
        }
        None
    }

    fn comparison(
        &self,
        opcode: &str,
        left: &Value,
        right: &Value,
        truth: bool,
        state: &mut State,
    ) {
        let usable = |v: Val| !matches!(v.fact, IndexFact::Unknown) || v.symbol.is_some();
        let (variable, other, opcode) =
            match (self.guarded_variable(left), self.guarded_variable(right)) {
                (Some(v), _) if usable(self.eval(right, state)) => (v, right, opcode.to_owned()),
                (_, Some(v)) => (
                    v,
                    left,
                    match opcode {
                        "<" => ">",
                        "<=" => ">=",
                        ">" => "<",
                        ">=" => "<=",
                        other => other,
                    }
                    .to_owned(),
                ),
                _ => return,
            };
        let opcode = if truth {
            opcode
        } else {
            match opcode.as_str() {
                "<" => ">=",
                "<=" => ">",
                ">" => "<=",
                ">=" => "<",
                "==" => "!=",
                _ => "==",
            }
            .to_owned()
        };
        let (id, ty) = variable;
        let Some((width, signed)) = integer_type(self.target, &ty) else {
            return;
        };
        let (min, max) = type_range(width, signed);
        let bound = self.eval(other, state);
        let (lo, hi): (Option<i128>, Option<Count>) =
            match (&bound.fact, &bound.symbol, opcode.as_str()) {
                (IndexFact::Const(c), _, "<") => {
                    (None, (*c > min).then(|| Count::Const((*c).max(0) as u128)))
                }
                (IndexFact::Const(c), _, "<=") => {
                    (None, Some(Count::Const((c + 1).max(0) as u128)))
                }
                (IndexFact::Const(c), _, ">") => (Some(c + 1), None),
                (IndexFact::Const(c), _, ">=") => (Some(*c), None),
                (IndexFact::Const(c), _, "==") => {
                    (Some(*c), Some(Count::Const((c + 1).max(0) as u128)))
                }
                (_, Some(symbol), "<") => (None, Some(Count::Symbol(symbol.clone()))),
                _ => return,
            };
        let previous = state.facts.scalars.get(&id).map(|f| f.value.clone());
        let previous = previous.unwrap_or(IndexFact::Range {
            lo: min,
            hi: Count::Const((max + 1).max(0) as u128),
        });
        let narrowed = core::narrow(Some(&previous), lo, hi);
        let mut held = state.held.get(&id).cloned().unwrap_or_default();
        held.extend(bound.held.iter().cloned());
        state.facts.scalars.insert(
            id.clone(),
            ScalarFact {
                value: narrowed,
                standing: if held.is_empty() {
                    Standing::Source
                } else {
                    Standing::Held
                },
            },
        );
        if !held.is_empty() {
            state.held.insert(id, held);
        }
    }

    fn constant_condition(&self, condition: &Value, state: &State) -> Option<bool> {
        let condition = strip(condition);
        if has_call(condition) {
            return None;
        }
        let value = self.eval(condition, state);
        match value.fact {
            IndexFact::Const(v) if value.held.is_empty() => Some(v != 0),
            _ => None,
        }
    }

    fn if_stmt(&mut self, node: &Value, state: &mut State) {
        let parts = kids(node).collect::<Vec<_>>();
        let Some(condition) = parts.first() else {
            return;
        };
        self.full_expression(condition, state, "condition");
        let constant = self.constant_condition(condition, state);
        let mut then_state = state.clone();
        then_state.definite = false;
        let mut else_state = then_state.clone();
        self.guard(condition, true, &mut then_state);
        self.guard(condition, false, &mut else_state);
        if constant == Some(false) {
            then_state.live = false;
        }
        if constant == Some(true) {
            else_state.live = false;
        }
        if let Some(then_branch) = parts.get(1) {
            self.stmt(then_branch, &mut then_state);
        }
        if let Some(else_branch) = parts.get(2) {
            self.stmt(else_branch, &mut else_state);
        }
        *state = then_state.join(&else_state);
        state.definite = false;
    }

    fn loop_stmt(&mut self, node: &Value, state: &mut State) {
        let node_kind = kind(node);
        let parts = kids(node).collect::<Vec<_>>();
        let (init, condition, increment, body) = match node_kind {
            "ForStmt" => (
                parts.first().copied(),
                parts.get(2).copied(),
                parts.get(3).copied(),
                parts.get(4).copied(),
            ),
            "WhileStmt" => (None, parts.first().copied(), None, parts.get(1).copied()),
            _ => (None, parts.get(1).copied(), None, parts.first().copied()),
        };
        // `do … while (0)`: the body runs once.
        if node_kind == "DoStmt"
            && condition.is_some_and(|c| self.constant_condition(c, state) == Some(false))
        {
            let mut once = state.clone();
            if let Some(body) = body {
                self.stmt(body, &mut once);
            }
            let jumps = body.is_some_and(loop_jumps);
            if jumps {
                let mut after = state.clone();
                let written = body.map(|body| self.written(body)).unwrap_or_default();
                for id in &written {
                    after.write(id);
                }
                if body.is_some_and(has_call) {
                    after.call();
                }
                *state = after.join(&once);
                state.live = true;
            } else {
                *state = once;
            }
            if let Some(condition) = condition {
                self.full_expression(condition, state, "condition");
            }
            return;
        }
        if let Some(init) = init.filter(|i| !kind(i).is_empty()) {
            match kind(init) {
                "DeclStmt" => self.stmt(init, state),
                _ => self.full_expression(init, state, "statement"),
            }
        }
        let mut written = BTreeSet::new();
        for part in [condition, increment, body].into_iter().flatten() {
            written.extend(self.written(part));
        }
        let mut entry = state.clone();
        for id in &written {
            entry.write(id);
        }
        if [condition, increment, body]
            .into_iter()
            .flatten()
            .any(has_call)
        {
            entry.call();
        }
        entry.definite = false;
        // A canonical counted `for (i = c0; i < E; i++)` with `i` unwritten in
        // the body: `i` is in `[c0, E)` throughout the body.
        let counted = (node_kind == "ForStmt")
            .then(|| self.counted(init?, condition?, increment?, body?, &entry))
            .flatten();
        self.loop_depth += 1;
        if let Some(condition) = condition.filter(|c| !kind(c).is_empty()) {
            let mut at_condition = entry.clone();
            self.full_expression(condition, &mut at_condition, "condition");
        }
        let mut body_state = entry.clone();
        if let Some(condition) = condition.filter(|c| !kind(c).is_empty()) {
            self.guard(condition, true, &mut body_state);
            if self.constant_condition(condition, &entry) == Some(false) {
                body_state.live = false;
            }
        }
        if let Some((id, fact)) = counted {
            body_state.facts.scalars.insert(
                id,
                ScalarFact {
                    value: fact,
                    standing: Standing::Source,
                },
            );
        }
        if let Some(body) = body {
            self.stmt(body, &mut body_state);
        }
        if let Some(increment) = increment.filter(|i| !kind(i).is_empty()) {
            let mut at_increment = entry.clone();
            self.full_expression(increment, &mut at_increment, "statement");
        }
        self.loop_depth -= 1;
        *state = entry;
        state.definite = false;
    }

    fn counted(
        &self,
        init: &Value,
        condition: &Value,
        increment: &Value,
        body: &Value,
        entry: &State,
    ) -> Option<(String, IndexFact)> {
        // init: `i = c0` or a declaration `T i = c0`.
        let (id, start) = match kind(init) {
            "BinaryOperator" if init["opcode"] == "=" => {
                let mut operands = kids(init);
                let (id, _, _) = decl_ref(operands.next()?)?;
                (id.to_owned(), self.eval(operands.next()?, entry).fact)
            }
            "DeclStmt" => {
                let decl = kids(init).next()?;
                let value = kids(decl).last()?;
                (
                    decl["id"].as_str()?.to_owned(),
                    self.eval(value, entry).fact,
                )
            }
            _ => return None,
        };
        let IndexFact::Const(start) = start else {
            return None;
        };
        if !self.tracked.contains(&id) || start < 0 {
            return None;
        }
        // condition: `i < E`.
        let condition = strip(condition);
        if kind(condition) != "BinaryOperator" || condition["opcode"] != "<" {
            return None;
        }
        let mut operands = kids(condition);
        let (variable, _) = self.guarded_variable(operands.next()?)?;
        if variable != id {
            return None;
        }
        let bound = self.eval(operands.next()?, entry);
        let hi = match (&bound.fact, &bound.symbol) {
            (IndexFact::Const(c), _) if *c > start && bound.held.is_empty() => {
                Count::Const(*c as u128)
            }
            (_, Some(symbol)) if bound.held.is_empty() => Count::Symbol(symbol.clone()),
            _ => return None,
        };
        // increment: `i++`, `++i` or `i += 1`.
        let increment = strip(increment);
        let steps = match kind(increment) {
            "UnaryOperator" => increment["opcode"] == "++",
            "CompoundAssignOperator" => {
                increment["opcode"] == "+="
                    && kids(increment)
                        .nth(1)
                        .is_some_and(|r| matches!(self.eval(r, entry).fact, IndexFact::Const(1)))
            }
            _ => false,
        };
        if !steps || kids(increment).next().and_then(decl_ref).map(|d| d.0) != Some(id.as_str()) {
            return None;
        }
        let written = self.written(body);
        if written.contains(&id) {
            return None;
        }
        if let Count::Symbol(symbol) = &hi
            && written.contains(symbol)
        {
            return None;
        }
        Some((id, IndexFact::Range { lo: start, hi }))
    }

    fn switch_stmt(&mut self, node: &Value, state: &mut State) {
        let parts = kids(node).collect::<Vec<_>>();
        let (Some(selector), Some(body)) = (parts.first(), parts.get(1)) else {
            self.reason("bounds-unsupported-form: unreadable switch".into());
            return;
        };
        self.full_expression(selector, state, "condition");
        let written = self.written(body);
        let mut entry = state.clone();
        for id in &written {
            entry.write(id);
        }
        if has_call(body) {
            entry.call();
        }
        entry.definite = false;
        self.switch_entries.push(entry.clone());
        let mut inside = entry.dead();
        self.stmt(body, &mut inside);
        self.switch_entries.pop();
        *state = entry;
        state.definite = false;
    }
}

fn has_entry(node: &Value) -> bool {
    matches!(kind(node), "LabelStmt" | "CaseStmt" | "DefaultStmt") || kids(node).any(has_entry)
}

fn loop_jumps(node: &Value) -> bool {
    match kind(node) {
        "BreakStmt" | "ContinueStmt" => true,
        "WhileStmt" | "DoStmt" | "ForStmt" | "SwitchStmt" => false,
        _ => kids(node).any(loop_jumps),
    }
}

#[derive(Clone, Debug)]
enum Update {
    None,
    Scalar(IndexFact, BTreeSet<String>),
    Pointer(PointerFact, String, BTreeSet<String>),
}

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

fn analyze_function(
    unit: &TuIndex<'_>,
    target: &BoundsTarget,
    contracts: &BoundsContracts,
    preprocessed: &str,
    name: &str,
) -> BoundsFunctionRow {
    let Some(function) = unit.functions.get(name) else {
        return BoundsFunctionRow {
            function: name.to_owned(),
            property: BOUNDS_PROPERTY,
            verdict: "Unknown",
            reasons: vec![
                "bounds-import-failure: no definition of the function in the unit's AST".into(),
            ],
            scope: "unconditional".into(),
            assumptions: Vec::new(),
            accesses: Vec::new(),
            refutes_own_contract: false,
            contract_dependencies: body_evidence::dependencies(name, None, contracts),
        };
    };
    let body = kids(function).find(|c| kind(c) == "CompoundStmt");
    let mut taken = BTreeSet::new();
    let mut params = BTreeMap::new();
    let mut tracked = BTreeSet::new();
    if let Some(body) = body {
        address_taken(body, &mut taken);
        // Card CG24: an address handed only to closed frames does not escape.
        for id in frame_tracked(body, contracts) {
            taken.remove(&id);
        }
    }
    let collect = |decl: &Value, tracked: &mut BTreeSet<String>| {
        let Some(id) = decl["id"].as_str() else {
            return;
        };
        let ty = qual(decl);
        let scalar_or_pointer = matches!(shape(&ty), Shape::NotArray);
        if scalar_or_pointer
            && !ty.contains("volatile")
            && !matches!(decl["storageClass"].as_str(), Some("static" | "extern"))
            && !taken.contains(id)
        {
            tracked.insert(id.to_owned());
        }
    };
    for parameter in kids(function).filter(|c| kind(c) == "ParmVarDecl") {
        collect(parameter, &mut tracked);
        if let (Some(id), Some(pname)) = (parameter["id"].as_str(), parameter["name"].as_str()) {
            params.insert(id.to_owned(), pname.to_owned());
        }
    }
    fn locals(node: &Value, out: &mut Vec<Value>) {
        if kind(node) == "VarDecl" {
            out.push(node.clone());
        }
        for child in kids(node) {
            locals(child, out);
        }
    }
    let mut declared = Vec::new();
    if let Some(body) = body {
        locals(body, &mut declared);
    }
    for decl in &declared {
        collect(decl, &mut tracked);
    }
    let contract = contracts.own(name);
    let mut analysis = Analysis {
        unit,
        target,
        contracts,
        preprocessed,
        tracked,
        params: params.clone(),
        globals_declared: BTreeMap::new(),
        records: Vec::new(),
        judgements: Vec::new(),
        reasons: Vec::new(),
        assumptions: BTreeSet::new(),
        switch_entries: Vec::new(),
        loop_depth: 0,
        call_reach: false,
        conditional: 0,
        own: contract,
        refuted: false,
        quant: BTreeMap::new(),
    };
    let mut state = State {
        facts: Facts::default(),
        held: BTreeMap::new(),
        elements: BTreeMap::new(),
        stable: params.keys().cloned().collect(),
        live: true,
        definite: true,
    };
    // The function's own bounds contract: its preconditions at entry.
    let mut scope = "unconditional".to_owned();
    if let Some(contract) = contract {
        let holds: BTreeSet<String> = if contract.held() {
            contract.holds().into_iter().collect()
        } else {
            BTreeSet::new()
        };
        let parameters = kids(function)
            .filter(|c| kind(c) == "ParmVarDecl")
            .collect::<Vec<_>>();
        let abi_matches = parameters.len() == contract.parameters.len()
            && parameters
                .iter()
                .zip(&contract.parameters)
                .all(|(c, p)| kind_matches(p.kind, &qual(c)));
        if !abi_matches {
            analysis.reason(format!(
                "bounds-declaration-withdrawn: `{name}`'s bounds contract does not match its C declaration"
            ));
        } else {
            let id_of = |pname: &str| {
                contract
                    .parameter(pname)
                    .and_then(|(index, _)| parameters.get(index))
                    .and_then(|p| p["id"].as_str())
                    .map(str::to_owned)
            };
            let count_of = |count: &BoundsCount| match count {
                BoundsCount::Const(n) => Some(Count::Const(*n)),
                BoundsCount::Parameter(p) => id_of(p).map(Count::Symbol),
            };
            let preconditions = contract
                .clauses
                .iter()
                .filter(|e| e.clause.is_precondition())
                .map(|e| e.text.clone())
                .collect::<Vec<_>>();
            if !preconditions.is_empty() {
                scope = format!(
                    "conditional on `{name}`'s bounds preconditions ({}): {}",
                    if contract.held() { "held" } else { "declared" },
                    preconditions.join("; ")
                );
            }
            for entry in &contract.clauses {
                let standing = if holds.is_empty() {
                    Standing::Resolved
                } else {
                    Standing::Held
                };
                match &entry.clause {
                    BoundsClause::Extent {
                        parameter,
                        count,
                        unit: u,
                    } => {
                        let (Some(id), Some(count)) = (id_of(parameter), count_of(count)) else {
                            continue;
                        };
                        let element = parameters
                            .iter()
                            .find(|p| p["id"].as_str() == Some(id.as_str()))
                            .and_then(|p| pointee(&qual(p)))
                            .map(|t| strip_qualifiers(&t))
                            .unwrap_or_default();
                        if !analysis.tracked.contains(&id) {
                            analysis.reason(format!(
                                "bounds-declaration-withdrawn: `{parameter}`'s address is taken, so its extent cannot be tracked"
                            ));
                            continue;
                        }
                        state.facts.pointers.insert(
                            id.clone(),
                            PointerFact {
                                object: format!("{parameter}@entry"),
                                count,
                                unit: match u {
                                    BoundsUnit::Elements => Unit2::Elements,
                                    BoundsUnit::Bytes => Unit2::Bytes,
                                },
                                offset: IndexFact::Const(0),
                                standing,
                                source_object: false,
                            },
                        );
                        state.elements.insert(id.clone(), element);
                        if !holds.is_empty() {
                            state.held.insert(id, holds.clone());
                        }
                    }
                    BoundsClause::Index { parameter, count } => {
                        let (Some(id), Some(count)) = (id_of(parameter), count_of(count)) else {
                            continue;
                        };
                        state.facts.scalars.insert(
                            id.clone(),
                            ScalarFact {
                                value: IndexFact::Range { lo: 0, hi: count },
                                standing,
                            },
                        );
                        if !holds.is_empty() {
                            state.held.insert(id, holds.clone());
                        }
                    }
                    BoundsClause::GlobalExtent {
                        object,
                        count,
                        unit: u,
                    } => {
                        analysis.globals_declared.insert(
                            object.clone(),
                            (
                                Count::Const(*count),
                                match u {
                                    BoundsUnit::Elements => Unit2::Elements,
                                    BoundsUnit::Bytes => Unit2::Bytes,
                                },
                                holds.clone(),
                            ),
                        );
                    }
                    _ => {}
                }
                analysis.assumptions.insert(format!(
                    "{}: `{name}` {}",
                    if holds.is_empty() { "declared" } else { "held" },
                    entry.text
                ));
            }
            // Card QUANT-INDEX-CHECKS: the quant-checked formals.
            analysis.seed_quant_checks(contract, &parameters, &mut state, &holds);
        }
    }
    if let Some(body) = body {
        analysis.stmt(body, &mut state);
    } else {
        analysis.reason("bounds-import-failure: the definition has no body".into());
    }
    let (verdict, reasons) = core::bounds_verdict(&analysis.judgements, &analysis.reasons);
    let mut reasons = reasons;
    reasons.dedup();
    BoundsFunctionRow {
        function: name.to_owned(),
        property: BOUNDS_PROPERTY,
        verdict: verdict.as_str(),
        reasons,
        scope,
        assumptions: analysis.assumptions.into_iter().collect(),
        accesses: analysis.records,
        refutes_own_contract: analysis.refuted,
        contract_dependencies: body_evidence::dependencies(name, Some(function), contracts),
    }
}

/// The bounds report of the selected functions of an already preprocessed
/// unit (the text the custody analysis read).
pub fn analyze_bounds_preprocessed(
    preprocessed: &str,
    unit: &C11TranslationUnit<'_>,
    functions: &[String],
    contracts: &BoundsContracts,
) -> Result<BoundsUnitReport, String> {
    // A deep `else if` chain nests the AST deeper than serde's default limit
    // and than a worker's default stack: read and walk it on a stack sized
    // for it, as the typed dispatch reader does.
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(ANALYSIS_STACK)
            .spawn_scoped(scope, || {
                analyze_on_stack(preprocessed, unit, functions, contracts)
            })
            .map_err(|error| format!("cannot start the bounds reader: {error}"))?
            .join()
            .map_err(|_| "the bounds reader panicked".to_owned())?
    })
}

const ANALYSIS_STACK: usize = 256 << 20;

/// Keep only what the consumer reads of each top-level declaration: the
/// selected (and return-extent-declaring) definitions whole, every other
/// function's parameter types, every global's name and type, and the
/// enumerations. Everything else is dropped as it is read, so a unit's memory
/// is its selected bodies, not its whole AST.
struct TopSeed<'a> {
    wanted: &'a BTreeSet<String>,
    kept: &'a mut Vec<Value>,
}

impl<'de> serde::de::DeserializeSeed<'de> for TopSeed<'_> {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de> serde::de::Visitor<'de> for TopSeed<'_> {
    type Value = ();
    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a Clang translation-unit AST")
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        while let Some(key) = map.next_key::<String>()? {
            if key == "inner" {
                map.next_value_seed(InnerSeed {
                    wanted: self.wanted,
                    kept: &mut *self.kept,
                })?;
            } else {
                map.next_value::<serde::de::IgnoredAny>()?;
            }
        }
        Ok(())
    }
}

struct InnerSeed<'a> {
    wanted: &'a BTreeSet<String>,
    kept: &'a mut Vec<Value>,
}

impl<'de> serde::de::DeserializeSeed<'de> for InnerSeed<'_> {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_seq(self)
    }
}

impl<'de> serde::de::Visitor<'de> for InnerSeed<'_> {
    type Value = ();
    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("the translation unit's declarations")
    }
    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        while let Some(decl) = seq.next_element::<Value>()? {
            let name = decl["name"].as_str().unwrap_or("");
            match kind(&decl) {
                "FunctionDecl" if self.wanted.contains(name) => self.kept.push(decl),
                "FunctionDecl" => {
                    let parameters = kids(&decl)
                        .filter(|c| kind(c) == "ParmVarDecl")
                        .map(|c| serde_json::json!({"kind": "ParmVarDecl", "type": c["type"].clone()}))
                        .collect::<Vec<_>>();
                    self.kept.push(serde_json::json!({
                        "kind": "FunctionDecl", "name": name, "inner": parameters,
                    }));
                }
                "VarDecl" => self.kept.push(serde_json::json!({
                    "kind": "VarDecl", "id": decl["id"].clone(), "name": name, "type": decl["type"].clone(),
                })),
                "EnumDecl" => self.kept.push(decl),
                // Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (rule
                // 14g): the typedef's name and type only.
                "TypedefDecl" => self.kept.push(serde_json::json!({
                    "kind": "TypedefDecl", "name": name, "type": decl["type"].clone(),
                })),
                _ => {}
            }
        }
        Ok(())
    }
}

fn analyze_on_stack(
    preprocessed: &str,
    unit: &C11TranslationUnit<'_>,
    functions: &[String],
    contracts: &BoundsContracts,
) -> Result<BoundsUnitReport, String> {
    use serde::de::DeserializeSeed;
    let target = target_facts(unit)?;
    let mut wanted = functions.iter().cloned().collect::<BTreeSet<_>>();
    for (name, contract) in &contracts.functions {
        if declares_checked_relation(contract) {
            wanted.insert(name.clone());
        }
    }
    let mut kept = Vec::new();
    crate::typed_c_ast::translation_unit_ast_with(preprocessed, unit, |bytes| {
        let mut deserializer = serde_json::Deserializer::from_slice(bytes);
        deserializer.disable_recursion_limit();
        TopSeed {
            wanted: &wanted,
            kept: &mut kept,
        }
        .deserialize(&mut deserializer)
    })?;
    let ast = serde_json::json!({ "kind": "TranslationUnitDecl", "inner": kept });
    let index = index_unit(&ast);
    // Phase one: every body in the unit that declares a return extent is
    // checked against it; a refuted declaration is withdrawn from every
    // dependent use (C06.4's two phases, in this unit; a caller composing
    // units passes the union through `BoundsContracts::refuted`).
    let mut refuted = contracts.refuted.clone();
    // Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (rule 14f): the body
    // evidence is phase one's check under the declarations as loaded (no
    // refutation table), so a route's second phase publishes the evidence its
    // first phase found.
    let loaded;
    let as_loaded = if contracts.refuted.is_empty() {
        contracts
    } else {
        loaded = BoundsContracts {
            refuted: BTreeSet::new(),
            ..contracts.clone()
        };
        &loaded
    };
    let selected = functions.iter().collect::<BTreeSet<_>>();
    let mut declaration_bodies = Vec::new();
    for (name, contract) in &contracts.functions {
        let checked = declares_checked_relation(contract);
        let Some(function) = index.functions.get(name) else {
            continue;
        };
        if !checked && !selected.contains(name) {
            continue;
        }
        let row = analyze_function(&index, &target, as_loaded, preprocessed, name);
        let refutes = if contracts.refuted.is_empty() {
            row.refutes_own_contract
        } else {
            analyze_function(&index, &target, contracts, preprocessed, name).refutes_own_contract
        };
        if checked && refutes {
            refuted.insert(name.clone());
        }
        declaration_bodies.push(body_evidence::evidence(contract, function, &row, as_loaded));
    }
    let withdrawn;
    let contracts = if refuted == contracts.refuted {
        contracts
    } else {
        withdrawn = BoundsContracts {
            refuted,
            ..contracts.clone()
        };
        &withdrawn
    };
    let rows = functions
        .iter()
        .map(|name| analyze_function(&index, &target, contracts, preprocessed, name))
        .collect();
    Ok(BoundsUnitReport {
        property: BOUNDS_PROPERTY,
        model_version: BOUNDS_MODEL_VERSION,
        contract_version: BOUNDS_CONTRACT_VERSION,
        source_sha256: qcore_schemas::digest::sha256_hex(preprocessed.as_bytes()),
        target: target.identity.clone(),
        contracts_sha256: contracts.digest(),
        functions: rows,
        declaration_bodies,
    })
}

/// [`analyze_bounds_preprocessed`] after the gateway's own validation and
/// preprocessing of `unit`.
pub fn analyze_bounds_c11(
    unit: &C11TranslationUnit<'_>,
    functions: &[String],
    contracts: &BoundsContracts,
) -> Result<BoundsUnitReport, String> {
    let preprocessed =
        crate::legacy_c::validate_unrestricted_c11(unit).map_err(|e| format!("{e:?}"))?;
    analyze_bounds_preprocessed(&preprocessed, unit, functions, contracts)
}

/// Admission of a recorded bounds report: it is re-derived from the same unit
/// and contracts, and every identity component, the access inventory and the
/// verdict of every row must agree; a missing row refuses.
pub fn check_bounds_report(
    recorded: &BoundsUnitReport,
    unit: &C11TranslationUnit<'_>,
    functions: &[String],
    contracts: &BoundsContracts,
) -> Result<(), String> {
    let fresh = analyze_bounds_c11(unit, functions, contracts)?;
    for name in functions {
        let Some(row) = recorded.row(name) else {
            return Err(format!(
                "bounds evidence omits the selected function `{name}`"
            ));
        };
        let Some(fresh_row) = fresh.row(name) else {
            return Err(format!("bounds evidence cannot be re-derived for `{name}`"));
        };
        core::admit(
            (&recorded.identity(), row.accesses.len(), row_verdict(row)),
            (
                &fresh.identity(),
                fresh_row.accesses.len(),
                row_verdict(fresh_row),
            ),
        )
        .map_err(|reason| format!("`{name}`: {reason}"))?;
        if row.accesses != fresh_row.accesses {
            return Err(format!(
                "`{name}`: bounds evidence's accesses differ from their derivation"
            ));
        }
    }
    Ok(())
}
