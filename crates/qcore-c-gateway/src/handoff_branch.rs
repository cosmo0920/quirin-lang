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

//! Card **SUPPLIED-OWNED-RETURN-CODE-HANDOFF** (gateway rule 12m, amended
//! in place; contract specification §C04.19.5): the **caller branch check**.
//!
//! USER DECISION 2026-10-15: a conditional hand-off (`quant live #rko:
//! #owner |-> #callback when #result != @RD_KAFKA_OP_RES_PASS;`) is admitted
//! only when the caller's body branches on that same result, with the same
//! constant, and treats ownership consistently: it keeps or disposes of the
//! object where the guard fails, and does not touch it where the guard
//! holds. Any mismatch keeps today's refusal.
//!
//! # What is read
//!
//! Per function definition, from Clang's AST ([`function_handling`]): every
//! **site**, a call through a function-pointer formal or a member, a direct
//! call handing one of the function's function-pointer formals on, or a
//! direct call whose result is an enumeration. For each pointer variable a
//! site hands (a formal or a local), and for each class of the site's result
//! (each constant the body compares it with, then the rest), the walk from
//! the function's entry to every exit records whether the variable is named
//! again after the site while it still designates the handed object (a
//! *touch*; an assignment to the variable rebinds it and ends that), and what
//! the function returns there (the result itself, a constant, or something
//! else). A path through a noreturn call ends. A construct the walk does not
//! read leaves the site's handling unread, which refuses.
//!
//! # The check ([`admit`])
//!
//! From the declared invoker, the path along which the callback formal (an
//! invocation of a parameter) or the handed object (an invocation of a
//! member) flows is followed through direct sites to the sites that invoke
//! it. At each such site the guard's values are followed up the path: no
//! value the guard admits may touch the object, and what the function returns
//! for those values is what its caller's site is checked for. The path ends
//! where the object is a local (it was created there); at the invoker, a
//! handed parameter must never be returned to the invoker's caller on a path
//! the guard admits. Some function of the path must compare the result with
//! the guard's own constant. Every site found must pass; none found, a body
//! not in the facts, two different readings of one function, a callback
//! formal that escapes, or an unread handling refuses.
//!
//! Scope (stated, not proved): the check reads the invoker's own path, the
//! functions the callback formal or the handed object is passed through, as
//! the user's decision names them ("the bodies of `rd_kafka_op_handle` and
//! the serve loops"); that the invoker invokes the member only there is the
//! declaration's own assumption (rule 13i).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::typed_c_ast::{children, kind};
use qcore_schemas::handoff::{GuardOp, HandOffGuard};

/// The stable reason key of a hand-off whose invoker's branch does not admit
/// it (rule 12m).
pub const UNBRANCHED: &str = "callback-handoff-unbranched";
/// The stable reason key of a direct call of a hand-off declaration.
pub const DIRECT: &str = "callback-handoff-direct";

/// The bound on the label fixed point of one walk.
const MAX_ROUNDS: usize = 16;
/// The bound on the path search's depth.
const MAX_DEPTH: usize = 8;

/// What a site passes at one position.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteActual {
    /// The function's parameter at this index.
    Param(usize),
    /// An automatic local variable, by declaration id.
    Local(String),
    /// Anything else.
    Other,
}

/// What a site calls.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteCallee {
    /// A function by name.
    Direct(String),
    /// The function-pointer parameter at this index (whatever it holds).
    Formal(usize),
    /// A function-pointer member read from a variable: its member path.
    Member { object: SiteActual, path: String },
    /// Any other indirect call.
    Other,
}

/// What a path returns where the site's result is in a class.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ret {
    /// The site's result itself.
    Same,
    /// A constant.
    Value(i64),
    /// Anything else.
    Unknown,
}

/// The walk's outcome for one class of the result.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    /// The handed variable is named again while it designates the object.
    pub touched: bool,
    /// What the function returns on those paths.
    pub returns: BTreeSet<Ret>,
}

/// One handed variable's reading: an outcome per compared value, then the
/// rest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Handling {
    pub values: BTreeMap<i64, Outcome>,
    pub rest: Outcome,
}

/// One site of a function.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Site {
    pub callee: SiteCallee,
    pub actuals: Vec<SiteActual>,
    /// Per position whose actual is a pointer variable: its handling, `None`
    /// when the walk met a construct it does not read.
    pub handling: BTreeMap<usize, Option<Handling>>,
    /// The constants the function compares the site's result with.
    pub compared: BTreeMap<String, i64>,
    /// Card HANDOFF-CANDIDATE-OWN-CALLBACKS (rule 12m amended): the member
    /// the called variable is an exact copy of ([`exact_copy`]): the object
    /// and the member path. The site then also invokes that member.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy: Option<(SiteActual, String)>,
    /// Card HANDOFF-DIRECT-CALL-READING (rule 12m amended): the site's
    /// result is stored into a variable through a conversion (Clang wrote a
    /// cast between the call and the store), so a test of that variable reads
    /// a converted value, not the result.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub result_converted: bool,
}

/// One write of a called variable, as [`exact_copy`] reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyWrite {
    /// `v = o->m` (or the declaration's initializer), `o` a pointer variable
    /// of the function the body never writes, with no conversion but an
    /// lvalue read.
    FromMember,
    /// Any other write: another expression, a compound assignment, an
    /// increment or decrement.
    Other,
}

/// Whether the writes make an exact copy: exactly one, from the member, and
/// the variable's address never taken (`QCore.CandidateComposition.exactCopy`).
#[must_use]
pub fn exact_copy_class(writes: &[CopyWrite], address_taken: bool) -> bool {
    !address_taken && writes == [CopyWrite::FromMember]
}

/// `COPY_VECTORS`, shared with `QCore.CandidateComposition.copyVectors`:
/// (writes, address taken, exact).
pub const COPY_VECTORS: &[(&[CopyWrite], bool, bool)] = &[
    (&[CopyWrite::FromMember], false, true),
    (&[CopyWrite::FromMember], true, false),
    (
        &[CopyWrite::FromMember, CopyWrite::FromMember],
        false,
        false,
    ),
    (&[CopyWrite::FromMember, CopyWrite::Other], false, false),
    (&[CopyWrite::Other], false, false),
    (&[], false, false),
];

/// Peel the conversions an lvalue read adds (`LValueToRValue`, `NoOp`) and
/// parentheses, never a written cast.
fn peel_read(mut node: &Value) -> &Value {
    loop {
        match kind(node) {
            "ParenExpr" => match children(node).first() {
                Some(child) => node = child,
                None => return node,
            },
            "ImplicitCastExpr"
                if matches!(node["castKind"].as_str(), Some("LValueToRValue" | "NoOp")) =>
            {
                match children(node).first() {
                    Some(child) => node = child,
                    None => return node,
                }
            }
            _ => return node,
        }
    }
}

/// `o->a.b` read exactly (no cast), `o` a variable: the path and `o`'s
/// declaration kind and id.
fn exact_member(node: &Value) -> Option<(String, &str, &str)> {
    let mut node = peel_read(node);
    let mut segments = Vec::new();
    loop {
        if kind(node) != "MemberExpr" {
            return None;
        }
        segments.push(node["name"].as_str()?.to_owned());
        let arrow = node["isArrow"].as_bool().unwrap_or(false);
        node = peel_read(children(node).first()?);
        if arrow {
            break;
        }
    }
    if kind(node) != "DeclRefExpr" {
        return None;
    }
    let declared = node["referencedDecl"]["kind"].as_str()?;
    if !matches!(declared, "ParmVarDecl" | "VarDecl") {
        return None;
    }
    segments.reverse();
    Some((
        segments.join("."),
        declared,
        node["referencedDecl"]["id"].as_str()?,
    ))
}

/// Whether `node` names the variable `id` (through parentheses and lvalue
/// conversions only).
fn names_variable(node: &Value, id: &str) -> bool {
    let node = peel_read(node);
    kind(node) == "DeclRefExpr" && node["referencedDecl"]["id"].as_str() == Some(id)
}

/// The writes of the variable `id` in `body` (its declaration's initializer
/// included), each classified, and whether its address is taken; with the
/// member every exact write reads (`None` when two exact writes read
/// different members).
fn variable_writes(
    body: &Value,
    id: &str,
) -> (Vec<CopyWrite>, bool, Option<(String, String, String)>) {
    let mut writes = Vec::new();
    let mut taken = false;
    let mut sources = BTreeSet::new();
    let mut classify = |value: &Value, writes: &mut Vec<CopyWrite>| match exact_member(value) {
        Some((path, declared, object)) => {
            sources.insert((path, declared.to_owned(), object.to_owned()));
            writes.push(CopyWrite::FromMember);
        }
        None => writes.push(CopyWrite::Other),
    };
    each(body, &mut |node| match kind(node) {
        "VarDecl" if node["id"].as_str() == Some(id) => {
            if let Some(init) = children(node).first() {
                classify(init, &mut writes);
            }
        }
        "BinaryOperator" if node["opcode"] == "=" => {
            if let [target, value] = children(node)
                && names_variable(target, id)
            {
                classify(value, &mut writes);
            }
        }
        "CompoundAssignOperator" => {
            if children(node)
                .first()
                .is_some_and(|target| names_variable(target, id))
            {
                writes.push(CopyWrite::Other);
            }
        }
        "UnaryOperator"
            if children(node)
                .first()
                .is_some_and(|operand| names_variable(operand, id)) =>
        {
            match node["opcode"].as_str() {
                Some("&") => taken = true,
                Some("++" | "--") => writes.push(CopyWrite::Other),
                _ => {}
            }
        }
        _ => {}
    });
    let source = (sources.len() == 1)
        .then(|| sources.into_iter().next())
        .flatten();
    (writes, taken, source)
}

/// Card HANDOFF-CANDIDATE-OWN-CALLBACKS (rule 12m amended): the member the
/// variable `id` (a formal, or an automatic local) is an exact copy of: the
/// body writes it exactly once, `v = o->m` (or `T v = o->m;`) with no
/// conversion but an lvalue read, never takes its address, and never writes
/// `o` (a formal or an automatic local pointer). A formal so copied holds,
/// where it is called, the member's value or the value it arrived with
/// (`QCore.CandidateComposition.an_exact_copy_calls_a_member_value`); the
/// site invokes the member on the paths that copied it. Anything else is
/// `None`, and the member's invocation through the variable is not read.
fn exact_copy(body: &Value, id: &str) -> Option<(String, String, String)> {
    let (writes, taken, source) = variable_writes(body, id);
    if !exact_copy_class(&writes, taken) {
        return None;
    }
    let (path, declared, object) = source?;
    // The object is a formal the body never writes (a local object, whose
    // write could follow the copy, is not followed).
    if declared != "ParmVarDecl" {
        return None;
    }
    let (object_writes, object_taken, _) = variable_writes(body, &object);
    (object_writes.is_empty() && !object_taken).then_some((path, declared, object))
}

/// A function's sites and the function-pointer formals that escape.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionHandling {
    pub parameters: usize,
    pub sites: Vec<Site>,
    /// Function-pointer formals named other than as a site's callee or
    /// actual, a null test or an assignment's target.
    pub escaping: BTreeSet<usize>,
    /// The function-pointer formals (a callback is followed only through
    /// one of these).
    pub formals: BTreeSet<usize>,
}

fn strip(mut node: &Value) -> &Value {
    while matches!(
        kind(node),
        "ImplicitCastExpr" | "ParenExpr" | "CStyleCastExpr" | "ConstantExpr"
    ) {
        match children(node).first() {
            Some(child) => node = child,
            None => break,
        }
    }
    node
}

fn reference(node: &Value) -> Option<(&str, &str)> {
    let node = strip(node);
    (kind(node) == "DeclRefExpr").then(|| {
        (
            node["referencedDecl"]["kind"].as_str().unwrap_or(""),
            node["referencedDecl"]["id"].as_str().unwrap_or(""),
        )
    })
}

fn is_pointer(type_text: &str) -> bool {
    type_text.trim_end().ends_with('*')
}

fn is_function_pointer(type_text: &str) -> bool {
    type_text.contains("(*")
}

fn no_return_callee(call: &Value) -> bool {
    let Some(callee) = children(call).first() else {
        return false;
    };
    let callee = strip(callee);
    kind(callee) == "DeclRefExpr"
        && callee["referencedDecl"]["type"]["qualType"]
            .as_str()
            .is_some_and(|text| text.ends_with("__attribute__((noreturn))") && !text.contains("(*"))
}

/// A dotted member path read from a variable (`rko->rko_u.leaders.cb`:
/// `rko_u.leaders.cb`), with the variable's expression.
fn member_path(node: &Value) -> Option<(String, &Value)> {
    let mut node = strip(node);
    let mut segments = Vec::new();
    while kind(node) == "MemberExpr" {
        segments.push(node["name"].as_str()?.to_owned());
        let arrow = node["isArrow"].as_bool().unwrap_or(false);
        node = strip(children(node).first()?);
        if arrow {
            break;
        }
    }
    if segments.is_empty() {
        return None;
    }
    segments.reverse();
    Some((segments.join("."), node))
}

struct Params<'a> {
    ids: Vec<&'a str>,
    types: Vec<&'a str>,
    /// Clang's desugared spelling (a pointer to a function typedef reads
    /// `R (*)(...)` there).
    desugared: Vec<&'a str>,
}

impl Params<'_> {
    fn index(&self, id: &str) -> Option<usize> {
        self.ids.iter().position(|candidate| *candidate == id)
    }

    fn actual(&self, node: &Value) -> SiteActual {
        match reference(node) {
            Some(("ParmVarDecl", id)) => {
                self.index(id).map_or(SiteActual::Other, SiteActual::Param)
            }
            Some(("VarDecl", id)) => SiteActual::Local(id.to_owned()),
            _ => SiteActual::Other,
        }
    }
}

/// Whether the local `id` is an automatic pointer variable of `function`.
fn local_pointer(function: &Value, id: &str) -> bool {
    fn find<'a>(node: &'a Value, id: &str) -> Option<&'a Value> {
        if kind(node) == "VarDecl" && node["id"].as_str() == Some(id) {
            return Some(node);
        }
        children(node).iter().find_map(|child| find(child, id))
    }
    find(function, id).is_some_and(|decl| {
        let spelled = decl["type"]["qualType"].as_str().unwrap_or("");
        let desugared = decl["type"]["desugaredQualType"]
            .as_str()
            .unwrap_or(spelled);
        decl["storageClass"].is_null()
            && (is_pointer(spelled) || is_pointer(desugared))
            && !is_function_pointer(spelled)
            && !is_function_pointer(desugared)
    })
}

/// Whether the local `id` is an automatic variable of `function` (a call
/// through it reads it as a function pointer).
fn local_function_pointer(function: &Value, id: &str) -> bool {
    let mut automatic = false;
    each(function, &mut |node| {
        if kind(node) == "VarDecl" && node["id"].as_str() == Some(id) {
            automatic = node["storageClass"].is_null();
        }
    });
    automatic
}

fn each<'a>(node: &'a Value, visit: &mut dyn FnMut(&'a Value)) {
    visit(node);
    for child in children(node) {
        each(child, visit);
    }
}

/// Whether a call's result is an enumeration (`enum T` or a typedef the unit
/// names one by).
fn enumerated_result(call: &Value, enum_typedefs: &BTreeSet<String>) -> bool {
    call["type"]["qualType"]
        .as_str()
        .is_some_and(|text| text.starts_with("enum "))
        || call["type"]["typeAliasDeclId"]
            .as_str()
            .is_some_and(|id| enum_typedefs.contains(id))
}

/// Read one function definition's sites (`None` for a declaration, or a
/// function with no site).
pub fn function_handling(
    function: &Value,
    enumerators: &BTreeMap<String, i64>,
    enum_typedefs: &BTreeSet<String>,
    function_pointer: &dyn Fn(&Value) -> bool,
) -> Option<FunctionHandling> {
    let body = children(function)
        .iter()
        .find(|child| kind(child) == "CompoundStmt")?;
    let parameters = children(function)
        .iter()
        .filter(|child| kind(child) == "ParmVarDecl")
        .collect::<Vec<_>>();
    let params = Params {
        ids: parameters
            .iter()
            .map(|parameter| parameter["id"].as_str().unwrap_or(""))
            .collect(),
        types: parameters
            .iter()
            .map(|parameter| parameter["type"]["qualType"].as_str().unwrap_or(""))
            .collect(),
        desugared: parameters
            .iter()
            .map(|parameter| {
                parameter["type"]["desugaredQualType"]
                    .as_str()
                    .or_else(|| parameter["type"]["qualType"].as_str())
                    .unwrap_or("")
            })
            .collect(),
    };
    // A function pointer as the unit's typedefs read it (a pointer to a
    // function typedef, `rd_kafka_q_serve_cb_t *`, has no desugared form).
    let function_formal = |index: usize| {
        params
            .types
            .get(index)
            .is_some_and(|text| is_function_pointer(text))
            || params
                .desugared
                .get(index)
                .is_some_and(|text| is_function_pointer(text))
            || parameters
                .get(index)
                .is_some_and(|parameter| function_pointer(parameter))
    };
    let mut calls = Vec::new();
    each(body, &mut |node| {
        if kind(node) == "CallExpr" {
            calls.push(node);
        }
    });
    // Locals by their order in the body: a declaration id is a pointer of
    // one Clang run, so two units' readings of one function compare equal
    // only by position.
    let mut locals = BTreeMap::new();
    each(body, &mut |node| {
        if kind(node) == "VarDecl"
            && let Some(id) = node["id"].as_str()
        {
            let next = locals.len();
            locals.entry(id.to_owned()).or_insert(next);
        }
    });
    let normal = |actual: SiteActual| match actual {
        SiteActual::Local(id) => locals.get(&id).map_or(SiteActual::Other, |at| {
            SiteActual::Local(format!("local{at}"))
        }),
        other => other,
    };
    let mut sites = Vec::new();
    let mut site_ids = BTreeSet::new();
    for call in calls {
        let Some(callee) = children(call).first() else {
            continue;
        };
        let stripped = strip(callee);
        let callee_kind = match reference(callee) {
            Some(("FunctionDecl", _)) => {
                SiteCallee::Direct(stripped["referencedDecl"]["name"].as_str()?.to_owned())
            }
            Some(("ParmVarDecl", id)) => match params.index(id) {
                Some(index) => SiteCallee::Formal(index),
                None => SiteCallee::Other,
            },
            _ => match member_path(callee) {
                Some((path, object)) => SiteCallee::Member {
                    object: params.actual(object),
                    path,
                },
                None => SiteCallee::Other,
            },
        };
        // Card HANDOFF-CANDIDATE-OWN-CALLBACKS: a call through a formal or an
        // automatic local that is an exact copy of a member.
        let copy = match reference(callee) {
            Some(("ParmVarDecl", id)) if params.index(id).is_some() => exact_copy(body, id),
            Some(("VarDecl", id)) if local_function_pointer(function, id) => exact_copy(body, id),
            _ => None,
        }
        .and_then(|(path, _, object)| {
            params
                .index(&object)
                .map(|index| (SiteActual::Param(index), path))
        });
        let actuals = children(call)[1..]
            .iter()
            .map(|actual| params.actual(actual))
            .collect::<Vec<_>>();
        let hands_a_formal = actuals
            .iter()
            .any(|actual| matches!(actual, SiteActual::Param(index) if function_formal(*index)));
        let recorded = match &callee_kind {
            SiteCallee::Direct(_) => hands_a_formal || enumerated_result(call, enum_typedefs),
            SiteCallee::Formal(_) | SiteCallee::Member { .. } | SiteCallee::Other => {
                enumerated_result(call, enum_typedefs) || hands_a_formal
            }
        };
        if !recorded {
            continue;
        }
        let Some(id) = call["id"].as_str() else {
            continue;
        };
        site_ids.insert(id.to_owned());
        let (result, result_converted) = match result_variable(body, id) {
            Some((result, converted)) => (Some(result), converted),
            None => (None, false),
        };
        let compared = compared_constants(body, id, result.as_deref(), enumerators);
        let mut handling = BTreeMap::new();
        for (position, actual) in actuals.iter().enumerate() {
            let object = match actual {
                SiteActual::Param(index)
                    if (params
                        .types
                        .get(*index)
                        .is_some_and(|text| is_pointer(text))
                        || params
                            .desugared
                            .get(*index)
                            .is_some_and(|text| is_pointer(text)))
                        && !function_formal(*index) =>
                {
                    params.ids[*index].to_owned()
                }
                SiteActual::Local(id) if local_pointer(function, id) => id.clone(),
                _ => continue,
            };
            handling.insert(
                position,
                walk_all(body, id, &object, result.as_deref(), &compared, enumerators),
            );
        }
        sites.push(Site {
            callee: match callee_kind {
                SiteCallee::Member { object, path } => SiteCallee::Member {
                    object: normal(object),
                    path,
                },
                other => other,
            },
            actuals: actuals.into_iter().map(normal).collect(),
            handling,
            compared,
            copy,
            result_converted,
        });
    }
    if sites.is_empty() {
        return None;
    }
    Some(FunctionHandling {
        formals: (0..params.ids.len())
            .filter(|index| function_formal(*index))
            .collect(),
        parameters: params.ids.len(),
        escaping: escaping_formals(body, &params, &function_formal, &site_ids),
        sites,
    })
}

/// The function-pointer formals named other than as a recorded site's
/// callee or actual, a null test or an assignment's target.
fn escaping_formals(
    body: &Value,
    params: &Params<'_>,
    function_formal: &dyn Fn(usize) -> bool,
    sites: &BTreeSet<String>,
) -> BTreeSet<usize> {
    fn visit(
        node: &Value,
        allowed: bool,
        params: &Params<'_>,
        function_formal: &dyn Fn(usize) -> bool,
        sites: &BTreeSet<String>,
        out: &mut BTreeSet<usize>,
    ) {
        if let Some(("ParmVarDecl", id)) = reference(node)
            && let Some(index) = params.index(id)
            && function_formal(index)
            && !allowed
        {
            out.insert(index);
            return;
        }
        let inner = children(node);
        match kind(node) {
            // A recorded site's callee and actuals.
            "CallExpr" if node["id"].as_str().is_some_and(|id| sites.contains(id)) => {
                for child in inner {
                    visit(child, true, params, function_formal, sites, out);
                }
            }
            // A null test (`if (callback)`, `!callback`, `callback == NULL`).
            "IfStmt" | "WhileStmt" => {
                let condition =
                    usize::from(node["hasInit"] == true) + usize::from(node["hasVar"] == true);
                for (at, child) in inner.iter().enumerate() {
                    visit(child, at == condition, params, function_formal, sites, out);
                }
            }
            "UnaryOperator" if node["opcode"] == "!" => {
                for child in inner {
                    visit(child, true, params, function_formal, sites, out);
                }
            }
            "BinaryOperator" if matches!(node["opcode"].as_str(), Some("==" | "!=")) => {
                for child in inner {
                    visit(child, true, params, function_formal, sites, out);
                }
            }
            // An assignment's target (`callback = rko->rko_serve`).
            "BinaryOperator" if node["opcode"] == "=" => {
                if let [target, value] = inner {
                    visit(target, true, params, function_formal, sites, out);
                    visit(value, false, params, function_formal, sites, out);
                }
            }
            "ImplicitCastExpr" | "ParenExpr" => {
                for child in inner {
                    visit(child, allowed, params, function_formal, sites, out);
                }
            }
            _ => {
                for child in inner {
                    visit(child, false, params, function_formal, sites, out);
                }
            }
        }
    }
    let mut out = BTreeSet::new();
    visit(body, false, params, function_formal, sites, &mut out);
    out
}

/// The local the site's result is assigned to (`r = CALL;` or `T r =
/// CALL;`), if any, and whether the store converts it (card
/// HANDOFF-DIRECT-CALL-READING: a cast Clang wrote between the call and the
/// store, below any parentheses).
fn result_variable(body: &Value, site: &str) -> Option<(String, bool)> {
    let is_site = |node: &Value| strip(node)["id"].as_str() == Some(site);
    let converted = |node: &Value| {
        let mut node = node;
        while kind(node) == "ParenExpr" {
            match children(node).first() {
                Some(inner) => node = inner,
                None => break,
            }
        }
        node["id"].as_str() != Some(site)
    };
    let mut found = None;
    each(body, &mut |node| match kind(node) {
        "BinaryOperator" if node["opcode"] == "=" => {
            if let [target, value] = children(node)
                && is_site(value)
                && let Some(("VarDecl", id)) = reference(target)
            {
                found = Some((id.to_owned(), converted(value)));
            }
        }
        "VarDecl" => {
            if let Some(init) = children(node).first()
                && is_site(init)
                && let Some(id) = node["id"].as_str()
            {
                found = Some((id.to_owned(), converted(init)));
            }
        }
        _ => {}
    });
    found
}

/// Whether `node` is the site's result: the site call itself, or a read of
/// its result variable.
fn is_result(node: &Value, site: &str, result: Option<&str>) -> bool {
    let node = strip(node);
    node["id"].as_str() == Some(site)
        || result
            .is_some_and(|result| matches!(reference(node), Some(("VarDecl", id)) if id == result))
}

fn unexpect(node: &Value) -> &Value {
    let stripped = strip(node);
    if kind(stripped) == "CallExpr"
        && let Some(callee) = children(stripped).first()
        && strip(callee)["referencedDecl"]["name"] == "__builtin_expect"
        && let Some(argument) = children(stripped).get(1)
    {
        return strip(argument);
    }
    stripped
}

/// Every constant the body compares the site's result with (`==`, `!=` and
/// `switch` labels), by name where it is an enumerator.
fn compared_constants(
    body: &Value,
    site: &str,
    result: Option<&str>,
    enumerators: &BTreeMap<String, i64>,
) -> BTreeMap<String, i64> {
    let mut out = BTreeMap::new();
    let mut note = |node: &Value| {
        let stripped = strip(node);
        if let Some(value) = crate::arm_release_check::fold_with(node, enumerators) {
            let name = match reference(stripped) {
                Some(("EnumConstantDecl", _)) => stripped["referencedDecl"]["name"]
                    .as_str()
                    .unwrap_or("")
                    .to_owned(),
                _ => format!("{value}"),
            };
            out.insert(name, value);
        }
    };
    each(body, &mut |node| match kind(node) {
        "BinaryOperator" if matches!(node["opcode"].as_str(), Some("==" | "!=")) => {
            if let [left, right] = children(node) {
                if is_result(left, site, result) {
                    note(right);
                } else if is_result(right, site, result) {
                    note(left);
                }
            }
        }
        "SwitchStmt"
            if children(node)
                .iter()
                .rev()
                .nth(1)
                .is_some_and(|condition| is_result(condition, site, result)) =>
        {
            each(node, &mut |inner| {
                if kind(inner) == "CaseStmt"
                    && let Some(label) = children(inner).first()
                {
                    note(label);
                }
            });
        }
        _ => {}
    });
    out
}

/// The class of the result one walk follows.
#[derive(Clone, Copy)]
enum Atom<'a> {
    Value(i64),
    Rest(&'a BTreeSet<i64>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum St {
    /// Before the site.
    Inactive,
    /// After the site: whether the variable still designates the object,
    /// and whether the result variable still holds the site's result.
    Active { alive: bool, valid: bool },
}

type States = BTreeSet<St>;

#[derive(Default)]
struct Flow {
    normal: States,
    breaks: States,
    continues: States,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tri {
    True,
    False,
    Unknown,
}

impl Tri {
    fn not(self) -> Self {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
        }
    }
    fn may_be(self, value: bool) -> bool {
        match self {
            Self::True => value,
            Self::False => !value,
            Self::Unknown => true,
        }
    }
}

struct Walk<'a> {
    site: &'a str,
    object: &'a str,
    result: Option<&'a str>,
    atom: Atom<'a>,
    enumerators: &'a BTreeMap<String, i64>,
    outcome: Outcome,
    unread: bool,
    labels: BTreeMap<String, States>,
    gotos: BTreeMap<String, States>,
}

fn walk_all(
    body: &Value,
    site: &str,
    object: &str,
    result: Option<&str>,
    compared: &BTreeMap<String, i64>,
    enumerators: &BTreeMap<String, i64>,
) -> Option<Handling> {
    let values = compared.values().copied().collect::<BTreeSet<_>>();
    let mut handling = Handling {
        values: BTreeMap::new(),
        rest: Outcome::default(),
    };
    for value in &values {
        handling.values.insert(
            *value,
            walk_one(body, site, object, result, Atom::Value(*value), enumerators)?,
        );
    }
    handling.rest = walk_one(body, site, object, result, Atom::Rest(&values), enumerators)?;
    Some(handling)
}

fn walk_one(
    body: &Value,
    site: &str,
    object: &str,
    result: Option<&str>,
    atom: Atom<'_>,
    enumerators: &BTreeMap<String, i64>,
) -> Option<Outcome> {
    let mut walk = Walk {
        site,
        object,
        result,
        atom,
        enumerators,
        outcome: Outcome::default(),
        unread: false,
        labels: BTreeMap::new(),
        gotos: BTreeMap::new(),
    };
    for _ in 0..MAX_ROUNDS {
        walk.gotos.clear();
        let flow = walk.stmt(body, BTreeSet::from([St::Inactive]));
        // Falling off the end of the body returns no value the caller reads.
        if flow
            .normal
            .iter()
            .any(|state| matches!(state, St::Active { .. }))
        {
            walk.outcome.returns.insert(Ret::Unknown);
        }
        let mut next = walk.labels.clone();
        for (label, states) in &walk.gotos {
            next.entry(label.clone()).or_default().extend(states);
        }
        if next == walk.labels {
            return (!walk.unread).then_some(walk.outcome);
        }
        walk.labels = next;
    }
    None
}

impl Walk<'_> {
    fn active(states: &States) -> bool {
        states
            .iter()
            .any(|state| matches!(state, St::Active { .. }))
    }

    fn contains_site(&self, node: &Value) -> bool {
        if node["id"].as_str() == Some(self.site) {
            return true;
        }
        children(node).iter().any(|child| self.contains_site(child))
    }

    fn map(states: &States, f: impl Fn(St) -> St) -> States {
        states.iter().map(|state| f(*state)).collect()
    }

    fn touch(&mut self, states: &States) {
        if states
            .iter()
            .any(|state| matches!(state, St::Active { alive: true, .. }))
        {
            self.outcome.touched = true;
        }
    }

    /// Decide a condition in one state.
    fn decide(&self, condition: &Value, state: St) -> Tri {
        let node = unexpect(condition);
        let valid = matches!(state, St::Active { valid: true, .. });
        let result_here = |node: &Value| {
            let stripped = strip(node);
            stripped["id"].as_str() == Some(self.site) && matches!(state, St::Active { .. })
                || (valid && is_result(node, self.site, self.result))
        };
        match kind(node) {
            "IntegerLiteral" => match node["value"].as_str().and_then(|v| v.parse::<i64>().ok()) {
                Some(0) => Tri::False,
                Some(_) => Tri::True,
                None => Tri::Unknown,
            },
            "UnaryOperator" if node["opcode"] == "!" => children(node)
                .first()
                .map_or(Tri::Unknown, |child| self.decide(child, state).not()),
            "BinaryOperator" => {
                let [left, right] = children(node) else {
                    return Tri::Unknown;
                };
                match node["opcode"].as_str() {
                    Some("&&") => match (self.decide(left, state), self.decide(right, state)) {
                        (Tri::False, _) | (_, Tri::False) => Tri::False,
                        (Tri::True, Tri::True) => Tri::True,
                        _ => Tri::Unknown,
                    },
                    Some("||") => match (self.decide(left, state), self.decide(right, state)) {
                        (Tri::True, _) | (_, Tri::True) => Tri::True,
                        (Tri::False, Tri::False) => Tri::False,
                        _ => Tri::Unknown,
                    },
                    Some(operator @ ("==" | "!=")) => {
                        let constant = if result_here(left) {
                            crate::arm_release_check::fold_with(right, self.enumerators)
                        } else if result_here(right) {
                            crate::arm_release_check::fold_with(left, self.enumerators)
                        } else {
                            None
                        };
                        let Some(constant) = constant else {
                            return Tri::Unknown;
                        };
                        let equal = match self.atom {
                            Atom::Value(value) => {
                                if value == constant {
                                    Tri::True
                                } else {
                                    Tri::False
                                }
                            }
                            Atom::Rest(values) => {
                                if values.contains(&constant) {
                                    Tri::False
                                } else {
                                    Tri::Unknown
                                }
                            }
                        };
                        if operator == "==" { equal } else { equal.not() }
                    }
                    _ => Tri::Unknown,
                }
            }
            _ => Tri::Unknown,
        }
    }

    /// Split `states` by a condition: (may be true, may be false).
    fn split(&self, condition: &Value, states: &States) -> (States, States) {
        let mut yes = BTreeSet::new();
        let mut no = BTreeSet::new();
        for state in states {
            let decided = self.decide(condition, *state);
            if decided.may_be(true) {
                yes.insert(*state);
            }
            if decided.may_be(false) {
                no.insert(*state);
            }
        }
        (yes, no)
    }

    fn assigns(&self, target: &Value) -> Option<bool> {
        match reference(target) {
            Some((_, id)) if id == self.object => Some(true),
            Some(("VarDecl", id)) if Some(id) == self.result => Some(false),
            _ => None,
        }
    }

    fn expr(&mut self, node: &Value, states: States) -> States {
        if states.is_empty() {
            return states;
        }
        if !Self::active(&states) && !self.contains_site(node) {
            return states;
        }
        match kind(node) {
            "DeclRefExpr" => {
                if node["referencedDecl"]["id"].as_str() == Some(self.object) {
                    self.touch(&states);
                }
                states
            }
            "UnaryExprOrTypeTraitExpr" => states,
            "BinaryOperator" | "CompoundAssignOperator" => {
                let [left, right] = children(node) else {
                    self.unread = true;
                    return states;
                };
                match node["opcode"].as_str() {
                    Some("=") => match self.assigns(left) {
                        Some(true) => {
                            let states = self.expr(right, states);
                            Self::map(&states, |state| match state {
                                St::Active { valid, .. } => St::Active {
                                    alive: false,
                                    valid,
                                },
                                other => other,
                            })
                        }
                        Some(false) => {
                            let site = strip(right)["id"].as_str() == Some(self.site);
                            let states = self.expr(right, states);
                            Self::map(&states, |state| match state {
                                St::Active { alive, .. } => St::Active { alive, valid: site },
                                other => other,
                            })
                        }
                        None => {
                            let states = self.expr(left, states);
                            self.expr(right, states)
                        }
                    },
                    Some("&&" | "||") => {
                        let after_left = self.expr(left, states);
                        let after_right = self.expr(right, after_left.clone());
                        after_left.union(&after_right).copied().collect()
                    }
                    _ => {
                        if kind(node) == "CompoundAssignOperator"
                            && self.assigns(left) == Some(false)
                        {
                            let states = self.expr(right, states);
                            return Self::map(&states, |state| match state {
                                St::Active { alive, .. } => St::Active {
                                    alive,
                                    valid: false,
                                },
                                other => other,
                            });
                        }
                        let states = self.expr(left, states);
                        self.expr(right, states)
                    }
                }
            }
            "ConditionalOperator" => {
                let [condition, yes, no] = children(node) else {
                    self.unread = true;
                    return states;
                };
                let states = self.expr(condition, states);
                let (when_yes, when_no) = self.split(condition, &states);
                let mut out = self.expr(yes, when_yes);
                out.extend(self.expr(no, when_no));
                out
            }
            "CallExpr" => {
                let mut states = states;
                for child in children(node) {
                    states = self.expr(child, states);
                }
                if node["id"].as_str() == Some(self.site) {
                    return BTreeSet::from([St::Active {
                        alive: true,
                        valid: false,
                    }])
                    .into_iter()
                    .filter(|_| !states.is_empty())
                    .collect();
                }
                if no_return_callee(node) {
                    return BTreeSet::new();
                }
                states
            }
            "StmtExpr" => {
                let Some(body) = children(node).first() else {
                    return states;
                };
                let flow = self.stmt(body, states);
                if !flow.breaks.is_empty() || !flow.continues.is_empty() {
                    self.unread = true;
                }
                flow.normal
            }
            "UnaryOperator" => {
                let states = children(node)
                    .iter()
                    .fold(states, |states, child| self.expr(child, states));
                if matches!(node["opcode"].as_str(), Some("++" | "--"))
                    && children(node)
                        .first()
                        .is_some_and(|child| self.assigns(child) == Some(false))
                {
                    return Self::map(&states, |state| match state {
                        St::Active { alive, .. } => St::Active {
                            alive,
                            valid: false,
                        },
                        other => other,
                    });
                }
                states
            }
            _ => children(node)
                .iter()
                .fold(states, |states, child| self.expr(child, states)),
        }
    }

    fn record_return(&mut self, value: Option<&Value>, states: &States) {
        if !Self::active(states) {
            return;
        }
        let ret = match value {
            None => Ret::Unknown,
            Some(value) => {
                let valid = states
                    .iter()
                    .all(|state| matches!(state, St::Active { valid: true, .. } | St::Inactive));
                if strip(value)["id"].as_str() == Some(self.site)
                    || (valid && is_result(value, self.site, self.result))
                {
                    Ret::Same
                } else if let Some(constant) =
                    crate::arm_release_check::fold_with(value, self.enumerators)
                {
                    Ret::Value(constant)
                } else {
                    Ret::Unknown
                }
            }
        };
        self.outcome.returns.insert(ret);
    }

    fn stmt(&mut self, node: &Value, states: States) -> Flow {
        let mut flow = Flow::default();
        if states.is_empty() {
            return flow;
        }
        let inner = children(node);
        match kind(node) {
            "CompoundStmt" => {
                let mut current = states;
                for child in inner {
                    let step = self.stmt(child, current);
                    flow.breaks.extend(step.breaks);
                    flow.continues.extend(step.continues);
                    current = step.normal;
                }
                flow.normal = current;
            }
            "DeclStmt" => {
                let mut current = states;
                for declaration in inner {
                    if kind(declaration) != "VarDecl" {
                        continue;
                    }
                    let id = declaration["id"].as_str().unwrap_or("");
                    if let Some(init) = children(declaration).first() {
                        let site = strip(init)["id"].as_str() == Some(self.site);
                        current = self.expr(init, current);
                        if Some(id) == self.result {
                            current = Self::map(&current, |state| match state {
                                St::Active { alive, .. } => St::Active { alive, valid: site },
                                other => other,
                            });
                        }
                    }
                    if id == self.object {
                        current = Self::map(&current, |state| match state {
                            St::Active { valid, .. } => St::Active {
                                alive: false,
                                valid,
                            },
                            other => other,
                        });
                    }
                }
                flow.normal = current;
            }
            "ReturnStmt" => {
                let value = inner.first();
                let states = match value {
                    Some(value) => self.expr(value, states),
                    None => states,
                };
                self.record_return(value, &states);
            }
            "IfStmt" => {
                let has_else = node["hasElse"] == true;
                let has_init = node["hasInit"] == true;
                let has_var = node["hasVar"] == true;
                let mut at = 0;
                let mut current = states;
                if has_init {
                    current = self.stmt(&inner[at], current).normal;
                    at += 1;
                }
                if has_var {
                    current = self.stmt(&inner[at], current).normal;
                    at += 1;
                }
                let (Some(condition), Some(then)) = (inner.get(at), inner.get(at + 1)) else {
                    self.unread = true;
                    flow.normal = current;
                    return flow;
                };
                let current = self.expr(condition, current);
                let (yes, no) = self.split(condition, &current);
                let then_flow = self.stmt(then, yes);
                let else_flow = if has_else {
                    match inner.get(at + 2) {
                        Some(otherwise) => self.stmt(otherwise, no),
                        None => {
                            self.unread = true;
                            Flow::default()
                        }
                    }
                } else {
                    Flow {
                        normal: no,
                        ..Flow::default()
                    }
                };
                flow.normal = then_flow.normal.union(&else_flow.normal).copied().collect();
                flow.breaks = then_flow.breaks.union(&else_flow.breaks).copied().collect();
                flow.continues = then_flow
                    .continues
                    .union(&else_flow.continues)
                    .copied()
                    .collect();
            }
            "WhileStmt" | "ForStmt" | "DoStmt" => return self.looping(node, states),
            "SwitchStmt" => return self.switch(node, states),
            "BreakStmt" => flow.breaks = states,
            "ContinueStmt" => flow.continues = states,
            "GotoStmt" => {
                let label = node["targetLabelDeclId"].as_str().unwrap_or("").to_owned();
                self.gotos.entry(label).or_default().extend(states);
            }
            "LabelStmt" => {
                let label = node["declId"].as_str().unwrap_or("");
                let mut incoming = states;
                if let Some(earlier) = self.labels.get(label) {
                    incoming.extend(earlier);
                }
                match inner.first() {
                    Some(child) => return self.stmt(child, incoming),
                    None => flow.normal = incoming,
                }
            }
            "NullStmt" => flow.normal = states,
            "CaseStmt" | "DefaultStmt" | "AttributedStmt" => match inner.last() {
                Some(child) => return self.stmt(child, states),
                None => flow.normal = states,
            },
            "GCCAsmStmt" | "MSAsmStmt" | "IndirectGotoStmt" => {
                self.unread = true;
                flow.normal = states;
            }
            other if other.ends_with("Stmt") && other != "StmtExpr" => {
                self.unread = true;
                flow.normal = states;
            }
            // An expression statement.
            _ => flow.normal = self.expr(node, states),
        }
        flow
    }

    fn looping(&mut self, node: &Value, states: States) -> Flow {
        let inner = children(node);
        let named = |node: &Value| !kind(node).is_empty();
        let (init, condition, increment, body) = match kind(node) {
            "WhileStmt" => {
                let body = inner.last();
                let condition = inner.len().checked_sub(2).and_then(|at| inner.get(at));
                (None, condition, None, body)
            }
            "DoStmt" => (None, inner.get(1), None, inner.first()),
            // `for (init; condVar; cond; inc) body`: absent parts are empty.
            _ => (
                inner.first().filter(|n| named(n)),
                inner.get(2).filter(|n| named(n)),
                inner.get(3).filter(|n| named(n)),
                inner.get(4),
            ),
        };
        let Some(body) = body else {
            self.unread = true;
            return Flow {
                normal: states,
                ..Flow::default()
            };
        };
        let mut entry = states;
        if let Some(init) = init {
            entry = self.stmt(init, entry).normal;
        }
        let mut head = entry.clone();
        let mut exits = BTreeSet::new();
        let mut breaks = BTreeSet::new();
        for _ in 0..MAX_ROUNDS {
            let (enter, leave) = if kind(node) == "DoStmt" {
                (head.clone(), BTreeSet::new())
            } else {
                match condition {
                    Some(condition) => {
                        let evaluated = self.expr(condition, head.clone());
                        self.split(condition, &evaluated)
                    }
                    None => (head.clone(), BTreeSet::new()),
                }
            };
            exits.extend(leave);
            let flow = self.stmt(body, enter);
            breaks.extend(flow.breaks);
            let mut back: States = flow.normal.union(&flow.continues).copied().collect();
            if kind(node) == "DoStmt" {
                let Some(condition) = condition else {
                    self.unread = true;
                    break;
                };
                let evaluated = self.expr(condition, back);
                let (again, leave) = self.split(condition, &evaluated);
                exits.extend(leave);
                back = again;
            } else if let Some(increment) = increment {
                back = self.expr(increment, back);
            }
            let next: States = entry.union(&back).copied().collect();
            if next == head {
                return Flow {
                    normal: exits.union(&breaks).copied().collect(),
                    ..Flow::default()
                };
            }
            head = next;
        }
        self.unread = true;
        Flow {
            normal: exits.union(&breaks).copied().collect(),
            ..Flow::default()
        }
    }

    fn switch(&mut self, node: &Value, states: States) -> Flow {
        let inner = children(node);
        let (Some(condition), Some(body)) = (
            inner.len().checked_sub(2).and_then(|at| inner.get(at)),
            inner.last(),
        ) else {
            self.unread = true;
            return Flow {
                normal: states,
                ..Flow::default()
            };
        };
        let states = self.expr(condition, states);
        let statements: &[Value] = if kind(body) == "CompoundStmt" {
            children(body)
        } else {
            std::slice::from_ref(body)
        };
        // Every label must be a direct statement of the body (no Duff).
        let mut nested_label = false;
        for statement in statements {
            let mut depth_label = false;
            let mut first = true;
            each(statement, &mut |inner| {
                if !first && matches!(kind(inner), "CaseStmt" | "DefaultStmt") {
                    depth_label = true;
                }
                first = false;
            });
            // `case A: case B: stmt` nests a label in a label: allowed.
            if depth_label && !matches!(kind(statement), "CaseStmt" | "DefaultStmt") {
                nested_label = true;
            }
        }
        if nested_label {
            self.unread = true;
        }
        let enumerators = self.enumerators;
        let labels_of = |statement: &Value| {
            let mut labels = Vec::new();
            let mut cursor = statement;
            loop {
                match kind(cursor) {
                    "CaseStmt" => {
                        labels.push(
                            children(cursor)
                                .first()
                                .and_then(|label| {
                                    crate::arm_release_check::fold_with(label, enumerators)
                                })
                                .ok_or(()),
                        );
                    }
                    "DefaultStmt" => labels.push(Ok(i64::MIN)),
                    _ => break,
                }
                match children(cursor).last() {
                    Some(next) => cursor = next,
                    None => break,
                }
            }
            labels
        };
        let mut values = BTreeSet::new();
        let mut has_default = false;
        for statement in statements {
            for label in labels_of(statement) {
                match label {
                    Ok(i64::MIN) => has_default = true,
                    Ok(value) => {
                        values.insert(value);
                    }
                    Err(()) => self.unread = true,
                }
            }
        }
        // Whether a state may enter at a label set.
        let (site, result, atom) = (self.site, self.result, self.atom);
        let condition_is_result = |state: St| match state {
            St::Active { valid, .. } => {
                strip(condition)["id"].as_str() == Some(site)
                    || (valid && is_result(condition, site, result))
            }
            St::Inactive => false,
        };
        let may_match = |state: St, value: i64| {
            !condition_is_result(state)
                || match atom {
                    Atom::Value(atom) => atom == value,
                    Atom::Rest(rest) => !rest.contains(&value),
                }
        };
        let may_miss = |state: St| {
            !condition_is_result(state)
                || match atom {
                    Atom::Value(atom) => !values.contains(&atom),
                    Atom::Rest(_) => true,
                }
        };
        let mut current = BTreeSet::new();
        let mut breaks = BTreeSet::new();
        let mut continues = BTreeSet::new();
        for statement in statements {
            for label in labels_of(statement) {
                for state in &states {
                    let enters = match label {
                        Ok(i64::MIN) => may_miss(*state),
                        Ok(value) => may_match(*state, value),
                        Err(()) => true,
                    };
                    if enters {
                        current.insert(*state);
                    }
                }
            }
            let step = self.stmt(statement, current);
            breaks.extend(step.breaks);
            continues.extend(step.continues);
            current = step.normal;
        }
        let mut normal: States = current.union(&breaks).copied().collect();
        if !has_default {
            normal.extend(states.iter().filter(|state| may_miss(**state)));
        }
        Flow {
            normal,
            continues,
            ..Flow::default()
        }
    }
}

/// The values a guard admits, or a forwarded set.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Values {
    Only(BTreeSet<i64>),
    Except(BTreeSet<i64>),
}

impl Values {
    fn is_empty(&self) -> bool {
        matches!(self, Self::Only(values) if values.is_empty())
    }
    fn contains(&self, value: i64) -> bool {
        match self {
            Self::Only(values) => values.contains(&value),
            Self::Except(values) => !values.contains(&value),
        }
    }
    /// Whether some admitted value lies outside `compared`.
    fn meets_rest(&self, compared: &BTreeSet<i64>) -> bool {
        match self {
            Self::Only(values) => values.iter().any(|value| !compared.contains(value)),
            Self::Except(_) => true,
        }
    }
    fn union(self, other: Self) -> Self {
        match (self, other) {
            (Self::Only(mut a), Self::Only(b)) => {
                a.extend(b);
                Self::Only(a)
            }
            (Self::Except(a), Self::Except(b)) => {
                Self::Except(a.intersection(&b).copied().collect())
            }
            (Self::Only(only), Self::Except(except)) | (Self::Except(except), Self::Only(only)) => {
                Self::Except(except.difference(&only).copied().collect())
            }
        }
    }
    /// `self` restricted to the rest class of `compared`.
    fn rest(&self, compared: &BTreeSet<i64>) -> Self {
        match self {
            Self::Only(values) => Self::Only(values.difference(compared).copied().collect()),
            Self::Except(values) => Self::Except(values.union(compared).copied().collect()),
        }
    }
}

/// Follow `values` through one site's handling of the variable at
/// `position`: `Err` when an admitted value touches it; otherwise what the
/// function forwards (`None`: nothing returns on those paths).
fn forward(site: &Site, position: usize, values: &Values) -> Result<Values, String> {
    let handling = site
        .handling
        .get(&position)
        .ok_or("the handed object is not a variable the site passes")?
        .as_ref()
        .ok_or("a construct the branch reading does not read")?;
    let compared = handling.values.keys().copied().collect::<BTreeSet<_>>();
    let mut out = Values::Only(BTreeSet::new());
    let mut take = |outcome: &Outcome, class: Values| -> Result<(), String> {
        if outcome.touched {
            return Err("the object is named again where the guard admits the result".into());
        }
        for ret in &outcome.returns {
            out = std::mem::replace(&mut out, Values::Only(BTreeSet::new())).union(match ret {
                Ret::Same => class.clone(),
                Ret::Value(value) => Values::Only(BTreeSet::from([*value])),
                Ret::Unknown => Values::Except(BTreeSet::new()),
            });
        }
        Ok(())
    };
    for (value, outcome) in &handling.values {
        if values.contains(*value) {
            take(outcome, Values::Only(BTreeSet::from([*value])))?;
        }
    }
    if values.meets_rest(&compared) {
        take(&handling.rest, values.rest(&compared))?;
    }
    Ok(out)
}

/// The function definitions the check reads, from every unit in scope.
pub trait Bodies {
    /// The readings of `function` (one per unit that defines it).
    fn readings(&self, function: &str) -> Vec<&FunctionHandling>;
}

/// The invocation the clause states.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Invocation {
    /// The invoker's function-pointer parameter at this index.
    Formal(usize),
    /// A member of the handed object (or of the invoker's parameter), by
    /// its path below the record (`rko_op_cb`).
    Member { object: Option<usize>, path: String },
}

/// What the clause hands at the hand-off's position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handed {
    /// `supplied_owned` or `_`: an object the invoker owns.
    Own,
    /// The invoker's parameter at this index.
    Parameter(usize),
}

/// One query: the invoker, the invocation, the position of the handed
/// object among the callback's actuals, what is handed there, the guard and
/// its resolved value.
#[derive(Clone, Debug)]
pub struct Query<'a> {
    pub invoker: &'a str,
    pub invocation: Invocation,
    pub position: usize,
    pub handed: Handed,
    pub guard: &'a HandOffGuard,
    pub value: i64,
}

fn one_reading<'a>(bodies: &'a dyn Bodies, function: &str) -> Result<&'a FunctionHandling, String> {
    let readings = bodies.readings(function);
    let Some(first) = readings.first() else {
        return Err(format!("the body of `{function}` is not read"));
    };
    if readings.iter().any(|reading| *reading != *first) {
        return Err(format!("`{function}` is defined differently by two units"));
    }
    Ok(first)
}

/// One step of a path from the invoker down: the function and the site it
/// calls the next function (or the invocation) through.
#[derive(Clone)]
struct Step<'a> {
    name: String,
    site: &'a Site,
}

/// Rule 12m's caller branch check (module documentation).
///
/// # Errors
/// Why the hand-off is not admitted.
pub fn admit(bodies: &dyn Bodies, query: &Query<'_>) -> Result<(), String> {
    let values = match query.guard.op {
        GuardOp::Eq => Values::Only(BTreeSet::from([query.value])),
        GuardOp::Ne => Values::Except(BTreeSet::from([query.value])),
    };
    let invoker = one_reading(bodies, query.invoker)?;
    // Paths from the invoker to every invocation site.
    let mut paths: Vec<Vec<Step<'_>>> = Vec::new();
    let mut seen = BTreeSet::new();
    search(
        bodies,
        query.invoker,
        invoker,
        &query.invocation,
        Vec::new(),
        &mut paths,
        &mut seen,
        0,
    )?;
    if paths.is_empty() {
        return Err(format!(
            "no invocation is read on `{}`'s path",
            query.invoker
        ));
    }
    for path in &paths {
        check_path(path, query, &values)?;
    }
    Ok(())
}

/// Why a direct call's reading of the calling body does not admit it (card
/// HANDOFF-DIRECT-CALL-READING, rule 12m amended).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DirectRefusal {
    /// The reading is not made: no site, an unread construct, a handed
    /// expression that is no variable, a converted store, or a body that
    /// never compares the result with the guard's own constant.
    NotRead(String),
    /// The reading is made and disagrees: the body names the object again on
    /// a result the guard gives the callback.
    Disagrees(String),
}

/// Card **HANDOFF-DIRECT-CALL-READING** (rule 12m amended): the caller branch
/// check applied inside the calling body, for a direct call of `callee`
/// handing its parameter `position` under `guard` (resolved to `value`).
/// Every direct site of `callee` in the body must be read: the handed actual
/// a pointer variable whose handling the walk read, the result stored (if at
/// all) without a conversion, no class the guard admits naming the object
/// again before a rebinding or a noreturn call (rule 12m's `forward`), and
/// the result compared with the guard's own constant. Where the result
/// leads after the call (what the body returns) is not the object's custody
/// and is not read here; the per-path custody state is split at the caller's
/// test by the route.
///
/// # Errors
/// Why the direct call is not admitted.
pub fn direct_admission(
    handling: &FunctionHandling,
    callee: &str,
    position: usize,
    guard: &HandOffGuard,
    value: i64,
) -> Result<(), DirectRefusal> {
    let values = match guard.op {
        GuardOp::Eq => Values::Only(BTreeSet::from([value])),
        GuardOp::Ne => Values::Except(BTreeSet::from([value])),
    };
    let sites = handling
        .sites
        .iter()
        .filter(|site| site.callee == SiteCallee::Direct(callee.to_owned()))
        .collect::<Vec<_>>();
    if sites.is_empty() {
        return Err(DirectRefusal::NotRead(format!(
            "no direct call of `{callee}` is read in the calling body"
        )));
    }
    for site in sites {
        match site.handling.get(&position) {
            None => {
                return Err(DirectRefusal::NotRead(
                    "the handed object is not a pointer variable the reading follows".into(),
                ));
            }
            Some(None) => {
                return Err(DirectRefusal::NotRead(
                    "a construct the branch reading does not read".into(),
                ));
            }
            Some(Some(_)) => {}
        }
        forward(site, position, &values).map_err(DirectRefusal::Disagrees)?;
        if site.compared.get(&guard.constant) != Some(&value) {
            return Err(DirectRefusal::NotRead(format!(
                "the calling body does not compare the result with `{}`",
                guard.constant
            )));
        }
        if site.result_converted {
            return Err(DirectRefusal::NotRead(
                "the result is stored through a conversion".into(),
            ));
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn search<'a>(
    bodies: &'a dyn Bodies,
    name: &str,
    function: &'a FunctionHandling,
    invocation: &Invocation,
    path: Vec<Step<'a>>,
    paths: &mut Vec<Vec<Step<'a>>>,
    seen: &mut BTreeSet<(String, String)>,
    depth: usize,
) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err("the invocation's path is deeper than the reading follows".into());
    }
    if !seen.insert((name.to_owned(), format!("{invocation:?}"))) {
        // A recursive forward is the same site set.
        return Ok(());
    }
    if let Invocation::Formal(index) = invocation
        && !function.formals.contains(index)
    {
        return Err(format!(
            "`{name}`'s parameter {index} is not read as a function pointer"
        ));
    }
    if let Invocation::Formal(index) = invocation
        && function.escaping.contains(index)
    {
        return Err(format!(
            "`{name}` passes its callback where the reading does not follow it"
        ));
    }
    for site in &function.sites {
        let mut step_path = path.clone();
        step_path.push(Step {
            name: name.to_owned(),
            site,
        });
        // Card HANDOFF-CANDIDATE-OWN-CALLBACKS (rule 12m amended): a call
        // through an exact copy of the member invokes the member.
        if let (
            Some((object, member)),
            Invocation::Member {
                object: owner,
                path: wanted,
            },
        ) = (&site.copy, invocation)
            && member == wanted
            && match owner {
                Some(index) => *object == SiteActual::Param(*index),
                None => true,
            }
        {
            paths.push(step_path);
            continue;
        }
        match (&site.callee, invocation) {
            (SiteCallee::Formal(index), Invocation::Formal(formal)) if index == formal => {
                paths.push(step_path);
            }
            (
                SiteCallee::Member {
                    object,
                    path: member,
                },
                Invocation::Member {
                    object: owner,
                    path: wanted,
                },
            ) if member == wanted
                && match owner {
                    Some(index) => *object == SiteActual::Param(*index),
                    None => true,
                } =>
            {
                paths.push(step_path);
            }
            (SiteCallee::Direct(callee), _) => {
                // Follow the callback formal, or every variable that may
                // carry the handed object (a member invocation's object).
                let next = match invocation {
                    Invocation::Formal(formal) => site
                        .actuals
                        .iter()
                        .position(|actual| *actual == SiteActual::Param(*formal))
                        .map(Invocation::Formal)
                        .into_iter()
                        .collect::<Vec<_>>(),
                    Invocation::Member { object, path } => site
                        .actuals
                        .iter()
                        .enumerate()
                        .filter(|(position, actual)| match object {
                            Some(index) => **actual == SiteActual::Param(*index),
                            None => site.handling.contains_key(position),
                        })
                        .map(|(position, _)| Invocation::Member {
                            object: Some(position),
                            path: path.clone(),
                        })
                        .collect(),
                };
                if next.is_empty() {
                    continue;
                }
                // A body not read may invoke the callback or the member
                // there: the reading refuses.
                let callee_body = one_reading(bodies, callee)?;
                for next in &next {
                    search(
                        bodies,
                        callee,
                        callee_body,
                        next,
                        step_path.clone(),
                        paths,
                        seen,
                        depth + 1,
                    )?;
                }
            }
            (SiteCallee::Other, Invocation::Formal(_)) => {}
            _ => {}
        }
    }
    Ok(())
}

fn check_path(path: &[Step<'_>], query: &Query<'_>, guard: &Values) -> Result<(), String> {
    let constant = &query.guard.constant;
    let mut compared = false;
    // The invocation site's handed actual, then up the path.
    let mut values = guard.clone();
    let mut position = query.position;
    for (level, step) in path.iter().enumerate().rev() {
        if step.site.result_converted {
            return Err(format!(
                "`{}` stores the result through a conversion",
                step.name
            ));
        }
        if let Some(value) = step.site.compared.get(constant) {
            if *value != query.value {
                return Err(format!(
                    "`{}` compares with `{constant}` = {value}, not {}",
                    step.name, query.value
                ));
            }
            compared = true;
        }
        let actual = step
            .site
            .actuals
            .get(position)
            .cloned()
            .unwrap_or(SiteActual::Other);
        let forwarded = forward(step.site, position, &values)
            .map_err(|why| format!("`{}`: {why}", step.name))?;
        match actual {
            SiteActual::Local(_) => {
                // The object was created here: no caller holds it.
                if level == 0 && query.handed != Handed::Own {
                    return Err(format!(
                        "`{}` hands a local where the clause names a parameter",
                        step.name
                    ));
                }
                return if compared {
                    Ok(())
                } else {
                    Err(format!("no function on the path branches on `{constant}`"))
                };
            }
            SiteActual::Param(index) => {
                if level == 0 {
                    // The invoker: a handed parameter is never returned to
                    // the invoker's caller on a path the guard admits.
                    if let Handed::Parameter(expected) = query.handed {
                        if expected != index {
                            return Err(format!(
                                "`{}` hands its parameter {index}, not {expected}",
                                step.name
                            ));
                        }
                        if !forwarded.is_empty() {
                            return Err(format!(
                                "`{}` returns to its caller where the callback took its parameter",
                                step.name
                            ));
                        }
                    } else {
                        return Err(format!(
                            "`{}` hands its own parameter where the clause names an object it owns",
                            step.name
                        ));
                    }
                    return if compared {
                        Ok(())
                    } else {
                        Err(format!("no function on the path branches on `{constant}`"))
                    };
                }
                if forwarded.is_empty() {
                    // Nothing returns where the guard admits the result.
                    return if compared {
                        Ok(())
                    } else {
                        Err(format!("no function on the path branches on `{constant}`"))
                    };
                }
                values = forwarded;
                position = index;
            }
            SiteActual::Other => {
                return Err(format!(
                    "`{}` hands an object the reading does not name",
                    step.name
                ));
            }
        }
    }
    Err("the invocation's path does not reach the invoker".into())
}

/// The readings of every unit in scope: the analysing unit's own facts, or
/// the run's (the pre-pass).
pub struct UnitBodies<'a>(pub Vec<&'a crate::callback_member_closure::MemberFacts>);

impl Bodies for UnitBodies<'_> {
    fn readings(&self, function: &str) -> Vec<&FunctionHandling> {
        self.0
            .iter()
            .filter_map(|facts| facts.handling.get(function))
            .collect()
    }
}

/// Rule 13l: the value of `@constant` as an enumerator of `candidate`'s own
/// result type, read through Clang in every unit in scope that declares the
/// candidate. `Err` when no unit declares it, when its result type has no
/// enumerator so named (a typo, a constant of another type), or when two
/// units disagree.
///
/// # Errors
/// Why the constant does not resolve.
pub fn resolve(
    units: &[&crate::callback_member_closure::MemberFacts],
    candidate: &str,
    constant: &str,
) -> Result<i64, String> {
    let mut values = BTreeSet::new();
    let mut declared = false;
    for facts in units {
        let Some(result) = facts.result_types.get(candidate) else {
            continue;
        };
        declared = true;
        let value = facts
            .typed_constants
            .get(constant)
            .and_then(|types| types.get(result))
            .ok_or_else(|| {
                format!(
                    "`@{constant}` is not an enumerator of `{candidate}`'s result type `{result}`"
                )
            })?;
        values.insert(*value);
    }
    if !declared {
        return Err(format!(
            "no unit in scope declares `{candidate}`, so `@{constant}` is not resolved"
        ));
    }
    match values.into_iter().collect::<Vec<_>>().as_slice() {
        [value] => Ok(*value),
        _ => Err(format!(
            "`@{constant}` has two values in the units in scope"
        )),
    }
}

/// Card FB-TEARDOWN-DESTROY-DECLARATIONS (rule 30's parameter guard, rule
/// 13l): the value of `@constant` as an enumerator of the type of
/// `callee`'s parameter `index`, read through Clang in every unit in scope
/// that declares the callee. `Err` as [`resolve`].
///
/// # Errors
/// Why the constant does not resolve.
pub fn resolve_parameter(
    units: &[&crate::callback_member_closure::MemberFacts],
    callee: &str,
    index: usize,
    constant: &str,
) -> Result<i64, String> {
    let mut values = BTreeSet::new();
    let mut declared = false;
    for facts in units {
        let Some(parameter) = facts
            .parameter_types
            .get(callee)
            .and_then(|types| types.get(index))
        else {
            continue;
        };
        declared = true;
        let value = facts
            .typed_constants
            .get(constant)
            .and_then(|types| types.get(parameter))
            .ok_or_else(|| {
                format!(
                    "`@{constant}` is not an enumerator of `{callee}`'s parameter type \
                     `{parameter}`"
                )
            })?;
        values.insert(*value);
    }
    if !declared {
        return Err(format!(
            "no unit in scope declares `{callee}`'s parameter {index}, so `@{constant}` is not \
             resolved"
        ));
    }
    match values.into_iter().collect::<Vec<_>>().as_slice() {
        [value] => Ok(*value),
        _ => Err(format!(
            "`@{constant}` has two values in the units in scope"
        )),
    }
}

/// One row of [`BRANCH_VECTORS`].
pub type BranchVector = (bool, i64, &'static [(i64, bool)], bool, bool);

/// `BRANCH_VECTORS`: (guard is `!=`, guard value, the site's compared values
/// with whether each touches, whether the rest touches, admitted), shared
/// with `QCore.ReturnCodeHandOff.branchVectors`: a site is admitted exactly
/// when no value the guard admits touches the object.
pub const BRANCH_VECTORS: &[BranchVector] = &[
    // `!= PASS` at the serve loop: PASS crashes (no touch recorded), every
    // other value leaves the op alone.
    (true, 0, &[(0, false)], false, true),
    // `!= PASS` at the pop serve: HANDLED/KEEP/YIELD untouched, the rest
    // (PASS and any value outside the enum) returns the op.
    (true, 0, &[(1, false), (2, false), (3, false)], true, false),
    // `== KEEP` at `rd_kafka_op_call`: KEEP untouched, the rest clears the
    // member.
    (false, 2, &[(2, false), (3, false)], true, true),
    // `== KEEP` where KEEP touches the object.
    (false, 2, &[(2, true)], false, false),
    // A guard the body never compares: the rest decides.
    (false, 2, &[], true, false),
    (false, 2, &[], false, true),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_vectors_hold() {
        for (ne, value, compared, rest_touches, admitted) in BRANCH_VECTORS {
            let handling = Handling {
                values: compared
                    .iter()
                    .map(|(value, touched)| {
                        (
                            *value,
                            Outcome {
                                touched: *touched,
                                returns: BTreeSet::new(),
                            },
                        )
                    })
                    .collect(),
                rest: Outcome {
                    touched: *rest_touches,
                    returns: BTreeSet::new(),
                },
            };
            let site = Site {
                callee: SiteCallee::Other,
                actuals: vec![SiteActual::Param(0)],
                handling: BTreeMap::from([(0, Some(handling))]),
                compared: BTreeMap::new(),
                copy: None,
                result_converted: false,
            };
            let values = if *ne {
                Values::Except(BTreeSet::from([*value]))
            } else {
                Values::Only(BTreeSet::from([*value]))
            };
            assert_eq!(
                forward(&site, 0, &values).is_ok(),
                *admitted,
                "{ne} {value} {compared:?} {rest_touches}"
            );
        }
    }

    #[test]
    fn a_forwarded_value_follows_its_class() {
        let site = Site {
            callee: SiteCallee::Other,
            actuals: vec![SiteActual::Param(0)],
            handling: BTreeMap::from([(
                0,
                Some(Handling {
                    values: BTreeMap::from([
                        (
                            2,
                            Outcome {
                                touched: false,
                                returns: BTreeSet::from([Ret::Same, Ret::Value(3)]),
                            },
                        ),
                        (
                            3,
                            Outcome {
                                touched: false,
                                returns: BTreeSet::from([Ret::Value(3)]),
                            },
                        ),
                    ]),
                    rest: Outcome {
                        touched: true,
                        returns: BTreeSet::from([Ret::Same]),
                    },
                }),
            )]),
            compared: BTreeMap::new(),
            copy: None,
            result_converted: false,
        };
        assert_eq!(
            forward(&site, 0, &Values::Only(BTreeSet::from([2]))),
            Ok(Values::Only(BTreeSet::from([2, 3])))
        );
        assert!(forward(&site, 0, &Values::Except(BTreeSet::from([0]))).is_err());
    }

    /// `check_path` refuses a step whose site stores the result through a
    /// conversion, even when the compared constant and per-class handling
    /// would otherwise admit it.
    #[test]
    fn check_path_refuses_a_converted_result_store() {
        let guard = HandOffGuard {
            op: GuardOp::Ne,
            constant: "H_PASS".into(),
        };
        let query = Query {
            invoker: "h_serve",
            invocation: Invocation::Formal(1),
            position: 1,
            handed: Handed::Own,
            guard: &guard,
            value: 0,
        };
        let make_site = |result_converted| Site {
            callee: SiteCallee::Formal(1),
            actuals: vec![SiteActual::Param(0), SiteActual::Local("op".into())],
            handling: BTreeMap::from([(
                1,
                Some(Handling {
                    values: BTreeMap::from([(
                        0,
                        Outcome {
                            touched: true,
                            returns: BTreeSet::new(),
                        },
                    )]),
                    rest: Outcome {
                        touched: false,
                        returns: BTreeSet::new(),
                    },
                }),
            )]),
            compared: BTreeMap::from([("H_PASS".into(), 0)]),
            copy: None,
            result_converted,
        };
        let values = Values::Except(BTreeSet::from([0]));
        let exact = make_site(false);
        assert_eq!(
            check_path(
                &[Step {
                    name: "h_serve".into(),
                    site: &exact
                }],
                &query,
                &values
            ),
            Ok(())
        );
        let converted = make_site(true);
        let err = check_path(
            &[Step {
                name: "h_serve".into(),
                site: &converted,
            }],
            &query,
            &values,
        )
        .expect_err("a converted result store is refused");
        assert!(
            err.contains("stores the result through a conversion"),
            "{err}"
        );
    }

    /// Card HANDOFF-CANDIDATE-OWN-CALLBACKS: the classifier's vectors hold,
    /// and are `QCore.CandidateComposition.copyVectors` entry for entry.
    #[test]
    fn copy_vectors_hold() {
        let lean = include_str!("../../../proof/QCore/CandidateComposition.lean");
        let start = lean.find("def copyVectors").expect("the Lean vectors");
        let end = start + lean[start..].find("theorem").expect("the vectors end");
        let entries = lean[start..end]
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with("[ ([") || line.starts_with("(["))
            .map(|line| {
                line.trim_start_matches("[ ")
                    .trim_end_matches(" ]")
                    .trim_end_matches(',')
                    .to_owned()
            })
            .collect::<Vec<_>>();
        let rendered = COPY_VECTORS
            .iter()
            .map(|(writes, taken, exact)| {
                assert_eq!(
                    exact_copy_class(writes, *taken),
                    *exact,
                    "{writes:?} {taken}"
                );
                format!(
                    "([{}], {taken}, {exact})",
                    writes
                        .iter()
                        .map(|write| match write {
                            CopyWrite::FromMember => ".fromMember",
                            CopyWrite::Other => ".other",
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(entries, rendered);
    }
}
