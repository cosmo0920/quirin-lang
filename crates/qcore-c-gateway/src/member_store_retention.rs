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

//! Card **CG10-MEMBER-STORE-RETAINED-BY** (gateway rule 70): a formal's
//! pointer stored into a member of another formal's object is a retention,
//! `retained_by(q, F.m…)`, published in the function's summary.
//!
//! USER DECISION 2026-11-20 on round 3's decision class 1, question 2: "Read
//! it as retention, option 1 with creating the relevant proof(s)."
//!
//! ## The store (rule 10v's missing owner premise, Q36's example C)
//!
//! Rule 10v refuses `s->p = q` for a formal `q` because nothing states who
//! owns `*s` or bounds `*q`'s lifetime by it. Rule 70 supplies that premise
//! the way the contract specification already spells it for a callee
//! (§C04.14.3, `resource_retained_by(P, FORMAL.FIELD)`): the holder is the
//! member path `F.m…` of the caller's object, the body takes **no custody**
//! of `q`, and the obligation that `*q` outlives the holder (or that the
//! holder is reset first) moves to every caller through the summary. The
//! store is read only when:
//!
//! 1. the destination is `F->m[.n…]`: a formal `F`'s pointee reached through
//!    no pointer but `F` (the member lies in `*F`);
//! 2. the value is a formal `q` of object-pointer type (rule 10v's
//!    `formal-pointer` class), distinct from `F`;
//! 3. neither `F` nor `q` is ever rebound in the body (no assignment,
//!    compound assignment, increment, decrement or address-of): the holder
//!    path and the retained pointer are the caller's actuals;
//! 4. on the flow, at the node: `F` and the member's object are untracked
//!    (rule 10v's owner premise; P-CG11-CALLER-SLOT for the old value), no
//!    binding of a place rooted at `F` carries a target or is unknown, and
//!    `q` is bound to no identity on any reaching path (the body holds no
//!    custody of it).
//!
//! A failing premise is rule 10v's own refusal, byte for byte.
//!
//! ## The summary (rule 10v's summary gate)
//!
//! Rule 10v withholds every summary from a body with a pointer store: a
//! caller would read `Borrowed` for the formal whose object was overwritten.
//! Rule 70 publishes, instead of an ordinary summary, a **retention summary**:
//! the body's validated W4 contract (every formal `Borrowed`, a `None`
//! result; the gate relaxed for exactly this body's admitted stores), plus a
//! path retention for each rule-70 store and an overwritten path for every
//! admitted pointer store into a member of a formal. It is installed as a
//! retaining declaration ([`ResourceContracts::insert_retaining`]), so every
//! consumer that knows nothing of retention sees an uncontracted callee, and
//! route G decides each call with rule 12t's table: the caller keeps custody
//! of `q`'s object and must show it outlives the holder (a static or null
//! actual), or a record is kept and refuses a release while held and an exit
//! while held (`retention-undecided`), never `Safe`.
//!
//! **Function pointers** (the subclass) carry no lifetime: a store of a value
//! rule 10v reads as function-pointer typed adds an overwritten path and no
//! retention. Every overwritten path, retained or not, must be a path holder
//! of the caller's own unrebound, untracked formal at the call (rule 12t's
//! premise P-HOLDER-PATH-STABLE); otherwise the call refuses whatever the
//! value, which carries P-CG11-CALLER-SLOT up one frame.
//!
//! A body with no rule-70 store keeps rule 10v's summary gate unchanged.
//!
//! The Lean model is `QCore.MemberStoreRetention`; [`DECISION_VECTORS`] are
//! shared with it.

use std::cell::RefCell;

use qcore_schemas::contract::{
    ResourceContracts, ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
};
use qcore_schemas::retained_by::{OverwrittenPath, PathRetention, RetainingContract};
use serde_json::Value;

use crate::pointer_store::{Decision, OwnerClass, ValueClass, ValueReading};
use crate::typed_c_ast::{children, kind};
use crate::typed_place::{Decision as PlaceDecision, RootKind, StepKind, TypedPlace};

/// The value code a rule-70 store carries ([`ValueClass::RetainedFormal`]).
pub(crate) const RETAINED: &str = "retained-formal";

/// The refusal of a call whose derived retention summary overwrote a member
/// path that is not a path holder of the calling function.
pub(crate) fn overwrite_description(callee: &str, holder: &str) -> String {
    format!(
        "call to `{callee}` overwrites `{holder}`, which is not a member path of an unrebound \
         formal this body holds no custody of ({})",
        crate::retained_by::UNDECIDED
    )
}

/// The flow's decision for a rule-70 store: whether the retained formal
/// carries an identity on some reaching path, and the destination's owner.
/// `QCore.MemberStoreRetention.decideRetained` is the same function.
pub(crate) fn decide(carries: bool, owner: OwnerClass) -> Decision {
    if carries {
        return Decision::NamedValue;
    }
    match owner {
        OwnerClass::Untracked => Decision::Admit,
        OwnerClass::Tracked => Decision::OwnedField,
        OwnerClass::Unresolved => Decision::UnresolvedOwner,
    }
}

/// The decision vectors shared with `proof/QCore/MemberStoreRetention.lean`
/// (`decisionVectors`): `(carries, owner, decision)`.
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(bool, &str, &str)] = &[
    (false, "untracked", "admit"),
    (false, "tracked", "owned-field"),
    (false, "unresolved", "unresolved-owner"),
    (true, "untracked", "named-value"),
    (true, "tracked", "named-value"),
    (true, "unresolved", "named-value"),
];

/// Reader premises 1–3: whether the classified store `destination = value`
/// is a rule-70 store (the retained formal's name is `reading.name`).
pub(crate) fn retained_formal(
    function: Option<&Value>,
    destination: &TypedPlace,
    reading: &ValueReading<'_>,
) -> bool {
    let Some(function) = function else {
        return false;
    };
    if reading.class != ValueClass::FormalPointer {
        return false;
    }
    let Some((_, value_id)) = &reading.name else {
        return false;
    };
    holder_path(destination).is_some()
        && destination.decl_kind == "ParmVarDecl"
        && *value_id != destination.decl
        && unrebound(function, &destination.decl)
        && unrebound(function, value_id)
}

/// Premise 1: `F->m[.n…]`, a formal's pointee reached through no pointer but
/// the root; the member names in order.
pub(crate) fn holder_path(destination: &TypedPlace) -> Option<Vec<String>> {
    if destination.root != RootKind::Pointer {
        return None;
    }
    let kinds = destination
        .steps
        .iter()
        .map(|step| step.kind)
        .collect::<Vec<_>>();
    if crate::typed_place::decide(destination.root, &kinds) != PlaceDecision::RootOnly {
        return None;
    }
    let (first, rest) = destination.steps.split_first()?;
    if first.kind != StepKind::Arrow || rest.iter().any(|step| step.kind != StepKind::Dot) {
        return None;
    }
    let fields = destination
        .steps
        .iter()
        .map(|step| step.member.clone())
        .collect::<Vec<_>>();
    fields.iter().all(|member| !member.is_empty()).then_some(fields)
}

/// Premise 3: the declaration `id` is never written or address-taken in
/// `function`.
fn unrebound(function: &Value, id: &str) -> bool {
    let mut unrebound = true;
    visit(function, &mut |node| {
        let target = match kind(node) {
            "BinaryOperator" if node["opcode"].as_str() == Some("=") => children(node).first(),
            "CompoundAssignOperator" => children(node).first(),
            "UnaryOperator" if matches!(node["opcode"].as_str(), Some("&" | "++" | "--")) => {
                children(node).first()
            }
            _ => None,
        };
        if target.is_some_and(|target| names(target, id)) {
            unrebound = false;
        }
    });
    unrebound
}

/// `node`, through parentheses, is a reference to the declaration `id`.
fn names(mut node: &Value, id: &str) -> bool {
    while kind(node) == "ParenExpr" {
        match children(node).first() {
            Some(inner) => node = inner,
            None => return false,
        }
    }
    kind(node) == "DeclRefExpr" && node["referencedDecl"]["id"].as_str() == Some(id)
}

fn visit(node: &Value, f: &mut impl FnMut(&Value)) {
    f(node);
    for child in children(node) {
        visit(child, f);
    }
}

/// What a body's admitted pointer stores state for its callers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Stated {
    pub(crate) retentions: Vec<PathRetention>,
    pub(crate) overwritten: Vec<OverwrittenPath>,
}

thread_local! {
    /// Set only while [`infer`] validates one body; the summary gate reads it.
    static DERIVING: RefCell<Option<Option<Stated>>> = const { RefCell::new(None) };
}

/// The summary gate's question (rule 10v's `has_pointer_store` relaxed): while
/// [`infer`] validates a body, whether every admitted pointer store of `cfg`
/// is a rule-70 store or a store into a member of a formal (or of a frame
/// record), with at least one rule-70 store. The answer's statement is kept
/// for [`infer`]. Outside [`infer`] it is always `false`.
pub(crate) fn summary_admits(
    cfg: &crate::legacy_c::LegacyCfg,
    parameters: &[Option<String>],
) -> bool {
    DERIVING.with(|cell| {
        let mut slot = cell.borrow_mut();
        let Some(kept) = slot.as_mut() else {
            return false;
        };
        let stated = statement(cfg, parameters);
        let admitted = stated.is_some();
        *kept = stated;
        admitted
    })
}

/// Each admitted pointer store of `cfg` as its callers must see it, or
/// `None` when one is not a store rule 70 can state or none retains.
fn statement(cfg: &crate::legacy_c::LegacyCfg, parameters: &[Option<String>]) -> Option<Stated> {
    let index = |name: &str| parameters.iter().position(|p| p.as_deref() == Some(name));
    let mut stated = Stated::default();
    for node in &cfg.nodes {
        let crate::legacy_c::LegacyInstruction::PointerStore { store, .. } = &node.instruction
        else {
            continue;
        };
        let Some(root) = index(&store.root) else {
            // A member of this frame's own record object: no caller sees it.
            if store.owner.is_none() && store.value != RETAINED {
                continue;
            }
            return None;
        };
        let fields = store
            .place
            .strip_prefix(store.root.as_str())
            .and_then(|rest| rest.strip_prefix("->"))
            .map(|rest| rest.split('.').map(str::to_owned).collect::<Vec<_>>())?;
        if fields.iter().any(|field| {
            field.is_empty() || !field.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
        }) {
            return None;
        }
        if store.value == RETAINED {
            let retained = index(store.name.as_deref()?)?;
            let retention = PathRetention {
                retained,
                root,
                fields: fields.clone(),
            };
            if !stated.retentions.contains(&retention) {
                stated.retentions.push(retention);
            }
        }
        let path = OverwrittenPath { root, fields };
        if !stated.overwritten.contains(&path) {
            stated.overwritten.push(path);
        }
    }
    // A body whose every pointer store is rule 10v's keeps 10v's gate.
    (!stated.retentions.is_empty()).then_some(stated)
}

/// The retention summary of `name`: the validated W4 contract of the body
/// with rule 10v's gate relaxed for its rule-70 stores, every formal
/// `Borrowed` and a `None` result, with the stores' retentions and
/// overwritten paths; `None` otherwise (the call stays uncontracted).
pub(crate) fn infer(
    tokens: &[crate::legacy_c::Token],
    name: &str,
    environment: &ResourceContracts,
    validate: impl Fn(
        &[crate::legacy_c::Token],
        &str,
        &ResourceContracts,
    ) -> Option<ResourceFunctionContract>,
) -> Option<RetainingContract> {
    DERIVING.with(|cell| *cell.borrow_mut() = Some(None));
    let contract = validate(tokens, name, environment);
    let stated = DERIVING.with(|cell| cell.borrow_mut().take().flatten());
    let (contract, stated) = (contract?, stated?);
    let neutral = ResourceFunctionContract {
        parameters: vec![ResourceParameterMode::Borrowed; contract.parameters.len()],
        result: ResourceReturnMode::None,
        ..ResourceFunctionContract::default()
    };
    if contract != neutral
        || stated
            .retentions
            .iter()
            .any(|r| r.retained >= contract.parameters.len() || r.root >= contract.parameters.len())
    {
        return None;
    }
    let mut retaining = RetainingContract::custody_only(contract);
    retaining.path_retentions = stated.retentions;
    retaining.overwritten_paths = stated.overwritten;
    Some(retaining)
}

/// The two-byte punctuators the lexer emits byte by byte, as rule 12t's
/// `merged_punctuators` reads them (the indices below are that reading's).
const PAIRS: [&str; 15] = [
    "->", "==", "!=", "<=", ">=", "++", "--", "+=", "-=", "*=", "/=", "|=", "&=", "^=", "%=",
];

fn merged_with_lines(tokens: &[crate::legacy_c::Token]) -> Vec<(String, usize)> {
    let mut merged = Vec::with_capacity(tokens.len());
    let mut at = 0;
    while at < tokens.len() {
        if let Some(next) = tokens.get(at + 1) {
            let pair = format!("{}{}", tokens[at].text, next.text);
            if tokens[at].text.len() == 1 && next.text.len() == 1 && PAIRS.contains(&pair.as_str())
            {
                merged.push((pair, tokens[at].line));
                at += 2;
                continue;
            }
        }
        merged.push((tokens[at].text.clone(), tokens[at].line));
        at += 1;
    }
    merged
}

/// Neither member path is a prefix of the other.
fn disjoint(a: &[String], b: &[String]) -> bool {
    !a.iter().zip(b).all(|(x, y)| x == y)
}

/// Rule 12t's premise P-HOLDER-PATH-STABLE at a call of a derived retention
/// summary: the bare mentions of `root` in `caller`'s body (indices of the
/// merged body texts) that are the root actual of a call of a derived summary
/// at an overwritten position, where that call is the one being decided
/// (`own`: its callee and line, when that call is the only one there) or
/// every path it overwrites below that position is disjoint from `fields`.
/// Such a call states exactly what it writes below `root`; every other
/// whole mention of `root` keeps the premise's refusal.
pub(crate) fn framed(
    unit: &[crate::legacy_c::Token],
    caller: &str,
    root: &str,
    fields: &[String],
    contracts: &ResourceContracts,
    own: (&str, usize),
) -> std::collections::BTreeSet<usize> {
    let mut framed = std::collections::BTreeSet::new();
    let Some((start, _)) = crate::legacy_c::find_function_body(unit, caller) else {
        return framed;
    };
    let Some(end) = crate::legacy_c::matching(unit, start, "{", "}") else {
        return framed;
    };
    let merged = merged_with_lines(&unit[start + 1..end]);
    let texts = merged.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>();
    let calls = |at: usize| texts.get(at + 1) == Some(&"(");
    let own_calls = (0..texts.len())
        .filter(|&at| texts[at] == own.0 && calls(at) && merged[at].1 == own.1)
        .collect::<Vec<_>>();
    let own_at = match own_calls.as_slice() {
        [at] => Some(*at),
        _ => None,
    };
    for at in 0..texts.len() {
        if !calls(at) {
            continue;
        }
        let Some(declared) = contracts.retaining_contract(texts[at]) else {
            continue;
        };
        if declared.overwritten_paths.is_empty() {
            continue;
        }
        let Some(spans) = crate::legacy_c::call_argument_spans(&texts, at) else {
            continue;
        };
        for (position, &(first, last)) in spans.iter().enumerate() {
            if texts.get(first..last) != Some(&[root][..]) {
                continue;
            }
            let mut below = declared
                .overwritten_paths
                .iter()
                .filter(|path| path.root == position)
                .peekable();
            if below.peek().is_none() {
                continue;
            }
            if own_at == Some(at) || below.all(|path| disjoint(&path.fields, fields)) {
                framed.insert(first);
            }
        }
    }
    framed
}

#[cfg(test)]
#[path = "../tests/unit/member_store_retention.rs"]
mod tests;
