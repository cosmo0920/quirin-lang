//! Body evidence for bounds declarations (card
//! **CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS**, gateway rules 14f–14g).
//!
//! Phase one of [`super::analyze_bounds_preprocessed`] analyses, in every
//! unit, the body of each declaration the unit defines (a `static inline`
//! header definition in every unit that includes it, or a main-source
//! definition) under that declaration. Before this card the result only
//! decided the in-unit refutation and was then dropped, so a header body's
//! check was invisible on the audit routes, which publish rows for main-source
//! definitions only. Here it becomes **evidence for the declaration** — never
//! a verdict row:
//!
//! * the body's own row under the declaration as loaded (no refutation table):
//!   `Safe` and not refuting covers the quant checks and preconditions
//!   ([`qcore_core::bounds_body_standing::covers`]);
//! * a **positive frame check** per pointer or slot parameter ([`frame_confirmed`]):
//!   the parameter is never assigned, never copied, never has its address
//!   taken, and every occurrence is a read through it (`P[k]`, `*P`, `P->f`
//!   read), a write through it when its declared mode is not `reads`, a
//!   comparison operand, an actual at a resolved, in-force closed frame
//!   position no stronger than its own, or (with a derived relation on it) a
//!   `return P` or `return P ± k`. Any other occurrence, and any inline
//!   assembly in the body, leaves the frame unconfirmed. The check reads the
//!   body's syntax only; it is narrow on purpose.
//!
//! Every row also carries the declarations its analysis could have consulted
//! ([`dependencies`]: its own name and every directly called name that has a
//! bounds declaration, held or not), an over-approximation from which the
//! audit rolls up the `Safe` rows (rule 13k's shape). Nothing here changes a
//! verdict, an access or a reason.

use std::collections::{BTreeMap, BTreeSet};

use qcore_contract::bounds_contract::{BoundsClause, BoundsContracts, BoundsFunctionContract};
use qcore_core::bounds_body_standing::{self as standing, BodyCheck, ClauseKind};
use qcore_core::index_extent_frames::FrameMode;
use serde_json::Value;

use super::{
    BoundsFunctionRow, call_abi_matches, callee_name, collect_calls, is_pointer, kids, kind, qual,
    strip,
};

/// One unit's body evidence for one declaration.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct DeclarationBodyEvidence {
    pub function: String,
    /// `crate:module` of the declaration.
    pub module: String,
    /// `held`, `failed`, `validated` or `unvalidated`.
    pub outcome: &'static str,
    /// The body's own row verdict under the declaration (phase one).
    pub body_verdict: &'static str,
    pub refutes: bool,
    /// Every clause of the declaration, by its text.
    pub clauses: Vec<String>,
    /// The clauses this unit covered, with the evidence that covered each.
    pub covered: BTreeMap<String, &'static str>,
}

/// What a clause is, for coverage.
#[must_use]
pub fn clause_kind(clause: &BoundsClause) -> ClauseKind {
    match clause {
        BoundsClause::QuantCheck { .. } => ClauseKind::QuantCheck,
        BoundsClause::Extent { .. } | BoundsClause::Index { .. } => ClauseKind::Precondition,
        BoundsClause::Frame { .. } => ClauseKind::Frame,
        BoundsClause::ReturnDerived { .. } => ClauseKind::Relation,
        BoundsClause::ReturnExtent { .. } => ClauseKind::ReturnExtent,
        BoundsClause::PreservesExtents => ClauseKind::Preserves,
        BoundsClause::GlobalExtent { .. } => ClauseKind::GlobalExtent,
        // Card NULL-SAFETY-PROPERTY: the bounds consumer reads the
        // environment without null-deref clauses (`for_bounds`); were one to
        // reach it, no bounds evidence would cover it, like a preservation.
        BoundsClause::NullDeref { .. } => ClauseKind::Preserves,
        // Card NULL-DEREF-QTT-GROUNDING: likewise the liveness part.
        BoundsClause::Live { .. } => ClauseKind::Preserves,
        // Card NULL-CANDIDATE-TRIAGE: likewise for the member and result
        // declarations only the null property reads.
        BoundsClause::NonnullMember { .. } | BoundsClause::NonnullResult => ClauseKind::Preserves,
        // Card NULL-MEMBER-INVARIANT-SCOPE: likewise the stable member and
        // the scope entry.
        BoundsClause::NonnullMemberStable { .. } | BoundsClause::NonnullMemberScope { .. } => {
            ClauseKind::Preserves
        }
        // Card NULL-TRIAGE-UNRESOLVED-PATHS: likewise the result range.
        BoundsClause::NonnullResultRange { .. } => ClauseKind::Preserves,
        // Card CELL-GUARD-RESULT-RANGE-CHECKED: likewise a C library result set.
        BoundsClause::ResultSet { .. } => ClauseKind::Preserves,
        // Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: likewise a member fact's
        // named premise.
        BoundsClause::Premise { .. } => ClauseKind::Preserves,
    }
}

/// The evidence of one examined body (`function` is its `FunctionDecl`,
/// `row` its phase-one row under the declaration as loaded).
pub(super) fn evidence(
    contract: &BoundsFunctionContract,
    function: &Value,
    row: &BoundsFunctionRow,
    contracts: &BoundsContracts,
) -> DeclarationBodyEvidence {
    let clauses = contract
        .clauses
        .iter()
        .map(|e| e.text.clone())
        .collect::<Vec<_>>();
    let held = contract.held();
    let body = BodyCheck {
        safe: row.verdict == "Safe",
        refuted: row.refutes_own_contract,
    };
    let mut covered = BTreeMap::new();
    if !held {
        for entry in &contract.clauses {
            let frame = match &entry.clause {
                BoundsClause::Frame { parameter, mode } => {
                    frame_confirmed(function, contract, contracts, parameter, *mode)
                }
                _ => false,
            };
            if let Some(by) = standing::covers(clause_kind(&entry.clause), body, frame) {
                covered.insert(entry.text.clone(), by);
            }
        }
    }
    let outcome = standing::unit_outcome(held, body.refuted, &covered.keys().cloned().collect());
    DeclarationBodyEvidence {
        function: contract.name.clone(),
        module: contract.module.clone(),
        outcome: outcome.as_str(),
        body_verdict: row.verdict,
        refutes: body.refuted,
        clauses,
        covered,
    }
}

/// How the walker's current expression is used.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Use {
    /// Its value is read (or used in a way not listed below).
    Value,
    /// It is the target of a store, `++`/`--` or compound assignment.
    WriteTarget,
    /// Its address is taken.
    AddressOf,
    /// It is the pointer through which an lvalue is read.
    ReadThrough,
    /// It is the pointer through which an lvalue is written.
    WriteThrough,
    /// It is an operand of a comparison.
    Compare,
}

fn through(use_: Use) -> Use {
    match use_ {
        Use::WriteTarget => Use::WriteThrough,
        // `&P[k]` or `&*P`: a pointer derived from P escapes the check.
        Use::AddressOf => Use::AddressOf,
        _ => Use::ReadThrough,
    }
}

struct FrameWalk<'a> {
    parameter: &'a str,
    mode: FrameMode,
    contracts: &'a BoundsContracts,
    /// A derived relation names the parameter as a base.
    relation: bool,
    ok: bool,
}

impl FrameWalk<'_> {
    fn is_parameter(&self, node: &Value) -> bool {
        let node = strip(node);
        kind(node) == "DeclRefExpr" && node["referencedDecl"]["id"].as_str() == Some(self.parameter)
    }

    /// Through parentheses and every implicit or explicit cast: the
    /// expression a use is a use of.
    fn operand<'v>(&self, node: &'v Value) -> &'v Value {
        let mut node = strip(node);
        while matches!(kind(node), "ImplicitCastExpr" | "CStyleCastExpr") {
            match kids(node).next() {
                Some(inner) => node = strip(inner),
                None => break,
            }
        }
        node
    }

    fn walk(&mut self, node: &Value, use_: Use) {
        if !self.ok {
            return;
        }
        match kind(node) {
            "GCCAsmStmt" | "MSAsmStmt" => self.ok = false,
            "UnaryExprOrTypeTraitExpr" => {}
            "DeclRefExpr" => {
                if self.is_parameter(node) {
                    self.ok = match use_ {
                        Use::ReadThrough | Use::Compare => true,
                        Use::WriteThrough => self.mode != FrameMode::Reads,
                        Use::Value | Use::WriteTarget | Use::AddressOf => false,
                    };
                }
            }
            "ParenExpr" | "ImplicitCastExpr" | "CStyleCastExpr" => {
                for child in kids(node) {
                    self.walk(child, use_);
                }
            }
            "ArraySubscriptExpr" => {
                for child in kids(node) {
                    let child_use = if is_pointer(&qual(child)) {
                        through(use_)
                    } else {
                        Use::Value
                    };
                    self.walk(child, child_use);
                }
            }
            "UnaryOperator" => {
                let child_use = match node["opcode"].as_str() {
                    Some("*") => through(use_),
                    Some("&") => Use::AddressOf,
                    Some("++" | "--") => Use::WriteTarget,
                    _ => Use::Value,
                };
                for child in kids(node) {
                    self.walk(child, child_use);
                }
            }
            "MemberExpr" => {
                let child_use = if node["isArrow"] == true {
                    through(use_)
                } else {
                    use_
                };
                for child in kids(node) {
                    self.walk(child, child_use);
                }
            }
            "BinaryOperator"
                if matches!(
                    node["opcode"].as_str(),
                    Some("==" | "!=" | "<" | "<=" | ">" | ">=")
                ) =>
            {
                for child in kids(node) {
                    self.walk(child, Use::Compare);
                }
            }
            "BinaryOperator" if node["opcode"] == "=" => {
                let mut operands = kids(node);
                if let Some(left) = operands.next() {
                    self.walk(left, Use::WriteTarget);
                }
                for right in operands {
                    self.walk(right, Use::Value);
                }
            }
            "CompoundAssignOperator" => {
                let mut operands = kids(node);
                if let Some(left) = operands.next() {
                    self.walk(left, Use::WriteTarget);
                }
                for right in operands {
                    self.walk(right, Use::Value);
                }
            }
            "CallExpr" => self.call(node),
            "ReturnStmt" => {
                for child in kids(node) {
                    if self.relation && self.returns_parameter(child) {
                        continue;
                    }
                    self.walk(child, Use::Value);
                }
            }
            _ => {
                for child in kids(node) {
                    self.walk(child, Use::Value);
                }
            }
        }
    }

    /// `P`, or `P ± k` with `k` an integer literal, under conversions.
    fn returns_parameter(&self, value: &Value) -> bool {
        let node = self.operand(value);
        if self.is_parameter(node) {
            return true;
        }
        if kind(node) == "BinaryOperator" && matches!(node["opcode"].as_str(), Some("+" | "-")) {
            let operands = kids(node).collect::<Vec<_>>();
            if let [left, right] = operands[..] {
                let literal = |n: &Value| kind(self.operand(n)) == "IntegerLiteral";
                return (self.is_parameter(self.operand(left)) && literal(right))
                    || (node["opcode"] == "+"
                        && literal(left)
                        && self.is_parameter(self.operand(right)));
            }
        }
        false
    }

    fn call(&mut self, call: &Value) {
        let mut children = kids(call);
        if let Some(callee) = children.next() {
            self.walk(callee, Use::Value);
        }
        let arguments = children.collect::<Vec<_>>();
        let callee = callee_name(call)
            .and_then(|name| self.contracts.contract(&name))
            .filter(|c| !c.held() && call_abi_matches(c, &arguments));
        let modes = callee.and_then(BoundsFunctionContract::frame);
        // A position the callee's result is derived from hands back a pointer
        // into the parameter's object: a copy the walk cannot follow.
        let bases = callee.map(|c| c.derived_bases()).unwrap_or_default();
        for (index, argument) in arguments.iter().enumerate() {
            if self.is_parameter(self.operand(argument)) {
                let derived = callee
                    .and_then(|c| c.parameters.get(index))
                    .is_some_and(|p| bases.contains(&p.name.as_str()));
                let allowed = !derived
                    && modes
                        .as_ref()
                        .and_then(|modes| modes.get(index).copied().flatten())
                        .is_some_and(|callee| callee <= self.mode);
                if !allowed {
                    self.ok = false;
                    return;
                }
                continue;
            }
            self.walk(argument, Use::Value);
        }
    }
}

/// The positive frame check of one parameter (module documentation).
#[must_use]
pub(super) fn frame_confirmed(
    function: &Value,
    contract: &BoundsFunctionContract,
    contracts: &BoundsContracts,
    parameter: &str,
    mode: FrameMode,
) -> bool {
    let Some(id) = kids(function)
        .filter(|c| kind(c) == "ParmVarDecl")
        .find(|c| c["name"].as_str() == Some(parameter))
        .and_then(|c| c["id"].as_str())
    else {
        return false;
    };
    let Some(body) = kids(function).find(|c| kind(c) == "CompoundStmt") else {
        return false;
    };
    let mut walk = FrameWalk {
        parameter: id,
        mode,
        contracts,
        relation: contract.clauses.iter().any(|e| {
            matches!(&e.clause, BoundsClause::ReturnDerived { parameter: base, .. } if base == parameter)
        }),
        ok: true,
    };
    walk.walk(body, Use::Value);
    walk.ok
}

/// The declarations a row's analysis could have consulted: its own name when
/// declared, and every directly called name with a bounds declaration (held,
/// refuted or in force). An over-approximation, never an omission of a
/// declaration the analysis read: every lookup the consumer makes is by the
/// callee's name, of a call in the body, or of the function itself.
#[must_use]
pub(super) fn dependencies(
    name: &str,
    function: Option<&Value>,
    contracts: &BoundsContracts,
) -> Vec<String> {
    let mut out = BTreeSet::new();
    if contracts.functions.contains_key(name) {
        out.insert(name.to_owned());
    }
    if let Some(function) = function {
        let mut calls = Vec::new();
        collect_calls(function, &mut calls);
        for call in calls {
            if let Some(callee) = callee_name(call)
                && contracts.functions.contains_key(&callee)
            {
                out.insert(callee);
            }
        }
    }
    out.into_iter().collect()
}
