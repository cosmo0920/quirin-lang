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

//! Card **CG03-CALL-SUMMARY-INSTANTIATION** (gateway rule 10u): one
//! call-contract application point, and the typed per-formal substitution
//! of a call's actuals (Clang gap entries K12 and K03).
//!
//! # One application point
//!
//! A call contract (the summary in force for a callee) is applied by exactly
//! one piece of code: the statement classifier builds the call's
//! [`crate::legacy_c::LegacyInstruction::ContractCall`] — callee, the contract
//! in force, the per-formal substitution of its actuals, its produced sites
//! and its read obligations — and the flow applies it once. Every consumer
//! that admits a call (a statement, a desugared condition, a `return`, a
//! nested sibling, the elaborator's sequenced call) holds that one
//! instruction; none builds a second model.
//!
//! What differed between consumers before this card was the **acceptance**
//! of the instantiated call (gap entry K12): the condition desugaring took a
//! call to a locally defined callee outside the caller's recursive component,
//! the scalar `return` route (B01) took only an externally declared callee,
//! so `if (h(p) % n)` was decided and `return h(p) % n;` was not. This module
//! states the acceptance once, as a function of what the call *is* and of what
//! C does with its **value** ([`ResultUse`]), never of the consumer that asks
//! ([`accept`]). Two positions that use a call's value the same way accept the
//! same calls (`QCore.CallInstantiation.acceptance_is_position_independent`).
//!
//! * **Standing** ([`Standing`]): a callee in the enclosing function's own
//!   recursive component may carry constraint 4's provisional summary, which
//!   is not admission evidence; it is refused in every use this table
//!   decides. The statement consumer's older admission of such a call (the
//!   fixed point's own use) is kept and recorded as a parity disagreement,
//!   never widened to another position.
//! * **Use of the result** ([`ResultUse`]): a value tested or consumed by an
//!   operator (a condition leaf, a scalar `return` operand) must own nothing
//!   (`None` or a derived result); a discarded value likewise (the
//!   classifier's own rule); a whole returned value may carry only fresh
//!   custody (card C-GATEWAY-RETURN-CALL-W4); a value passed to another call
//!   is the identity on the abstract state (card B04), or, card
//!   CG05-NESTED-CALL-EFFECTS (rule 10y), any result but a reallocation: the
//!   nested-call consumer binds it to a fresh reserved place (or reads it as
//!   data) and applies it inside-out under its footprint order check.
//! * **Total or reject.** A clause the application does not support refuses
//!   the whole call by name: the classifier's refusal, or the flow's
//!   `arguments do not match resource contract` for a custody formal whose
//!   actual was not substituted. No position applies part of a contract.
//!
//! # Formal substitution (K03)
//!
//! [`substitute`] reads each actual from Clang's typed AST: value →
//! identity → formal. A named pointer object, through parentheses and
//! value-preserving pointer conversions only, is that object's identity; the
//! address of a named object at an `Out` formal is that slot; an expression
//! whose evaluated reads are all of scalar objects (with `sizeof`/`_Alignof`
//! operands unevaluated) is data; a member, subscript or indirection is a
//! place read by card CG10's common typed place ([`crate::typed_place`]),
//! and a place CG10 refuses stays refused. A place at a custody-taking formal
//! is refused by design (`W44-CALL-ARGUMENT-CUSTODY-POSITION`); pointer
//! arithmetic (CG16), a nested call (CG05), a conditional (CG06), any other
//! cast (CG08) and every unlisted form refuse by name with their owners.
//!
//! The Lean model is `QCore.CallInstantiation`; [`ACCEPTANCE_VECTORS`] and
//! [`SUBSTITUTION_VECTORS`] are shared with it.

use qcore_schemas::contract::{ResourceParameterMode, ResourceReturnMode};
use serde_json::Value;

use crate::elaboration::strip_parens;
use crate::typed_access_path::qual;
use crate::typed_c_ast::{children, kind};

/// Where the callee's contract in force comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Standing {
    /// No body in this translation unit: a declared contract.
    Declared,
    /// A body in this unit outside the enclosing function's recursive
    /// component: a validated summary (constraint 4).
    Validated,
    /// A body in the enclosing function's recursive component: possibly the
    /// provisional `N(m)` of constraint 4.
    Recursive,
}

/// The contract's result mode, as the acceptance reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResultClass {
    None,
    Derived,
    Owned,
    Reallocated,
    Removed,
}

impl ResultClass {
    pub(crate) fn of(result: ResourceReturnMode) -> Self {
        match result {
            ResourceReturnMode::None => Self::None,
            ResourceReturnMode::Derived { .. } => Self::Derived,
            ResourceReturnMode::Owned { .. } => Self::Owned,
            ResourceReturnMode::Reallocated { .. } => Self::Reallocated,
            ResourceReturnMode::Removed { .. } => Self::Removed,
        }
    }

    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Derived => "derived",
            Self::Owned => "owned",
            Self::Reallocated => "reallocated",
            Self::Removed => "removed",
        }
    }

    /// Owns nothing: `None`, or a derived pointer (card W43-LIBC-RESULT-ALIAS).
    pub(crate) fn resource_free(self) -> bool {
        matches!(self, Self::None | Self::Derived)
    }
}

/// What C does with the call's value where it stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResultUse {
    /// An expression statement: the value is discarded.
    Discarded,
    /// `NAME = CALL(…)` as a statement: the value is stored in an object.
    Stored,
    /// The value is tested or consumed by an operator in the same full
    /// expression: a condition leaf, a scalar `return` operand, or a
    /// `(NAME = CALL(…))` group a condition then tests.
    Tested,
    /// The whole `return` value.
    Returned,
    /// An actual of another call.
    Passed,
    /// Card **CG04-CONDITION-ACQUISITION** (rule 10x): the value of a
    /// `(NAME = CALL(…))` group stored in `NAME` and tested, where it stands,
    /// against the contract's failure value on `NAME` (the NULL split of a
    /// pointer, the validity sentinel of a descriptor). The branch that
    /// follows is the statement position's own split, so every result class
    /// the stored use takes is taken; the carrier is checked by
    /// [`crate::condition_acquisition::carrier`].
    TestedSplit,
}

impl ResultUse {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Discarded => "discarded",
            Self::Stored => "stored",
            Self::Tested => "tested",
            Self::Returned => "returned",
            Self::Passed => "passed",
            Self::TestedSplit => "tested-split",
        }
    }
}

/// The consumer that asks. It never enters [`accept`]: it is recorded beside
/// each decision so that the parity stream can show that two consumers with
/// one use decided one way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Consumer {
    Statement,
    Return,
    Nested,
}

impl Consumer {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Statement => "statement",
            Self::Return => "return",
            Self::Nested => "nested",
        }
    }
}

/// Why the acceptance declined an instantiated call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decline {
    /// A callee in the caller's recursive component.
    RecursiveLocal,
    /// A value that must own nothing owns something.
    CustodyResult,
    /// A returned value whose custody is not fresh.
    ReturnedAlias,
    /// A passed value that is not the identity on the abstract state. Card
    /// CG05 widened the passed row: no decision reaches it any more; it is
    /// kept for the parity vocabulary.
    #[allow(dead_code, reason = "the parity vocabulary keeps B04's decline")]
    NotIdentity,
    /// Card CG05-NESTED-CALL-EFFECTS: a passed reallocated result (its
    /// replacement of an argument is not composed).
    PassedReallocation,
}

impl Decline {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::RecursiveLocal => "recursive-local",
            Self::CustodyResult => "custody-result",
            Self::ReturnedAlias => "returned-alias",
            Self::NotIdentity => "not-identity",
            Self::PassedReallocation => "passed-reallocation",
        }
    }
}

/// The acceptance of one instantiated call: a function of its standing, its
/// result class, whether its effect is the identity on the abstract state,
/// and the use C makes of its value. `QCore.CallInstantiation.accept` is the
/// same function.
pub(crate) fn accept(
    standing: Standing,
    result: ResultClass,
    identity: bool,
    use_: ResultUse,
) -> Result<(), Decline> {
    if standing == Standing::Recursive {
        return Err(Decline::RecursiveLocal);
    }
    match use_ {
        ResultUse::Stored => Ok(()),
        ResultUse::Discarded | ResultUse::Tested => {
            if result.resource_free() {
                Ok(())
            } else {
                Err(Decline::CustodyResult)
            }
        }
        ResultUse::Returned => {
            if matches!(result, ResultClass::None | ResultClass::Owned) {
                Ok(())
            } else {
                Err(Decline::ReturnedAlias)
            }
        }
        ResultUse::Passed => {
            if identity && result == ResultClass::None {
                // Card B04: the identity with no result.
                Ok(())
            } else if !matches!(result, ResultClass::Reallocated | ResultClass::Removed) {
                // Card CG05-NESTED-CALL-EFFECTS (rule 10y): a non-identity
                // call whose value is bound to a fresh reserved place (or is
                // data), applied inside-out under the footprint order check.
                Ok(())
            } else {
                Err(Decline::PassedReallocation)
            }
        }
        // Card CG04-CONDITION-ACQUISITION (rule 10x): the tested row widened
        // by the NULL split. The value is bound to its destination and the
        // leaf splits on its failure value, exactly the statement pair
        // `NAME = CALL(…); if (NAME …)`.
        ResultUse::TestedSplit => Ok(()),
    }
}

/// `(standing, result, identity, use, decision)`, shared with
/// `proof/QCore/CallInstantiation.lean` (`acceptanceVectors`).
#[cfg(test)]
pub(crate) const ACCEPTANCE_VECTORS: &[(&str, &str, bool, &str, &str)] = &[
    ("declared", "removed", false, "stored", "accept"),
    (
        "declared",
        "removed",
        false,
        "passed",
        "passed-reallocation",
    ),
    ("declared", "removed", false, "returned", "returned-alias"),
    ("declared", "removed", false, "discarded", "custody-result"),
    ("declared", "none", true, "tested", "accept"),
    ("validated", "none", false, "tested", "accept"),
    ("validated", "derived", true, "tested", "accept"),
    ("recursive", "none", true, "tested", "recursive-local"),
    ("declared", "owned", false, "tested", "custody-result"),
    (
        "validated",
        "reallocated",
        false,
        "discarded",
        "custody-result",
    ),
    ("declared", "derived", true, "discarded", "accept"),
    ("validated", "owned", false, "stored", "accept"),
    ("recursive", "owned", false, "stored", "recursive-local"),
    ("declared", "owned", false, "returned", "accept"),
    ("declared", "derived", true, "returned", "returned-alias"),
    (
        "declared",
        "reallocated",
        false,
        "returned",
        "returned-alias",
    ),
    ("declared", "none", true, "passed", "accept"),
    ("declared", "none", false, "passed", "accept"),
    ("declared", "derived", true, "passed", "accept"),
    // CG04-CONDITION-ACQUISITION (rule 10x): the NULL split.
    ("declared", "owned", false, "tested-split", "accept"),
    ("validated", "reallocated", false, "tested-split", "accept"),
    ("declared", "derived", true, "tested-split", "accept"),
    ("declared", "none", true, "tested-split", "accept"),
    (
        "recursive",
        "owned",
        false,
        "tested-split",
        "recursive-local",
    ),
    ("validated", "owned", false, "passed", "accept"),
    (
        "declared",
        "reallocated",
        false,
        "passed",
        "passed-reallocation",
    ),
    ("recursive", "owned", false, "passed", "recursive-local"),
];

/// The typed form of one actual, read from Clang's AST.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Actual {
    /// A named object's value (through parentheses and value-preserving
    /// pointer conversions): its identity. `cast` records a conversion.
    Identity { name: String, cast: bool },
    /// `&NAME`: the address of a named object.
    Address { name: String },
    /// A value whose evaluated reads are all of scalar objects.
    Data,
    /// A member, subscript or indirection CG10's place admits.
    Place { spelling: String },
}

impl Actual {
    #[cfg(test)]
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::Identity { cast: false, .. } => "identity",
            Self::Identity { cast: true, .. } => "identity-cast",
            Self::Address { .. } => "address",
            Self::Data => "data",
            Self::Place { .. } => "place",
        }
    }
}

/// Why an actual has no substitution, with the card that owns the form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SubstitutionRefusal {
    /// A form this reader does not list, with its owner.
    Form(&'static str, &'static str),
    /// CG10's place refused (its code).
    Place(String),
    /// A place, or an address, at a custody-taking formal.
    CustodyPosition(&'static str),
    /// Fewer or more actuals than declared formals.
    Arity,
    /// A missing Clang fact.
    Missing,
}

impl SubstitutionRefusal {
    pub(crate) fn reason(&self) -> String {
        match self {
            Self::Form(form, owner) => format!("{form} ({owner})"),
            Self::Place(code) => format!("place: {code} (CG10)"),
            Self::CustodyPosition(what) => {
                format!("{what} at a custody-taking formal (W44-CALL-ARGUMENT-CUSTODY-POSITION)")
            }
            Self::Arity => "actuals do not match the declared formals".into(),
            Self::Missing => "missing Clang fact".into(),
        }
    }

    #[cfg(test)]
    pub(crate) fn code(&self) -> String {
        match self {
            Self::Form(form, _) => format!("form:{form}"),
            Self::Place(code) => format!("place:{code}"),
            Self::CustodyPosition(what) => format!("custody-position:{what}"),
            Self::Arity => "arity".into(),
            Self::Missing => "missing".into(),
        }
    }
}

/// One formal's substitution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Substituted {
    pub(crate) formal: usize,
    pub(crate) mode: ResourceParameterMode,
    pub(crate) actual: Actual,
}

/// Whether a formal takes custody (or writes a slot) rather than observing.
pub(crate) fn custody_taking(mode: ResourceParameterMode) -> bool {
    !matches!(mode, ResourceParameterMode::Borrowed)
}

/// Whether a Clang type spelling is an object pointer.
fn object_pointer(ty: &str) -> bool {
    let ty = ty.trim_end();
    ty.ends_with('*') && !ty.contains("(*")
}

/// The Clang type of a node's value.
fn value_type(node: &Value) -> Option<&str> {
    qual(node)
}

/// Read one actual (value → identity).
pub(crate) fn read_actual(
    node: &Value,
    fields: &std::collections::BTreeMap<String, Option<crate::typed_access_path::FieldFacts>>,
) -> Result<Actual, SubstitutionRefusal> {
    let mut cursor = strip_parens(node);
    let mut cast = false;
    // Value-preserving conversions of a pointer value: a load, a
    // qualification conversion, and a bit cast between object pointers
    // (C11 6.3.2.3p7 keeps the address; the projection tracks addresses).
    loop {
        match kind(cursor) {
            "ParenExpr" => cursor = strip_parens(cursor),
            "ImplicitCastExpr" | "CStyleCastExpr" => {
                let cast_kind = cursor["castKind"]
                    .as_str()
                    .ok_or(SubstitutionRefusal::Missing)?;
                let inner = children(cursor)
                    .first()
                    .ok_or(SubstitutionRefusal::Missing)?;
                match cast_kind {
                    "LValueToRValue" | "NoOp" => {}
                    "BitCast"
                        if value_type(cursor).is_some_and(object_pointer)
                            && value_type(inner).is_some_and(object_pointer) =>
                    {
                        cast = true;
                    }
                    _ => break,
                }
                cursor = inner;
            }
            _ => break,
        }
    }
    match kind(cursor) {
        "DeclRefExpr" => {
            let reference = &cursor["referencedDecl"];
            match reference["kind"].as_str() {
                Some("VarDecl" | "ParmVarDecl") => {
                    let name = reference["name"]
                        .as_str()
                        .ok_or(SubstitutionRefusal::Missing)?
                        .to_owned();
                    if value_type(cursor).is_some_and(object_pointer) {
                        return Ok(Actual::Identity { name, cast });
                    }
                    // A scalar object's value (a pointer cast of one never
                    // reaches here: its cast is not a bit cast of pointers).
                    Ok(Actual::Data)
                }
                Some("EnumConstantDecl") => Ok(Actual::Data),
                _ => Err(SubstitutionRefusal::Form(
                    "reference to a non-object",
                    "CG22",
                )),
            }
        }
        "UnaryOperator" if cursor["opcode"].as_str() == Some("&") => {
            let operand = strip_parens(
                children(cursor)
                    .first()
                    .ok_or(SubstitutionRefusal::Missing)?,
            );
            if kind(operand) == "DeclRefExpr"
                && matches!(
                    operand["referencedDecl"]["kind"].as_str(),
                    Some("VarDecl" | "ParmVarDecl")
                )
            {
                return Ok(Actual::Address {
                    name: operand["referencedDecl"]["name"]
                        .as_str()
                        .ok_or(SubstitutionRefusal::Missing)?
                        .to_owned(),
                });
            }
            Err(SubstitutionRefusal::Form("address of a place", "CG11"))
        }
        "MemberExpr" | "ArraySubscriptExpr" => place_actual(cursor, fields),
        "UnaryOperator" if cursor["opcode"].as_str() == Some("*") => place_actual(cursor, fields),
        _ => data_actual(cursor),
    }
}

/// Whether a Clang type spelling is a variable-length array: a bracket whose
/// bound is neither empty nor a decimal constant.
fn variable_length(ty: &str) -> bool {
    let mut rest = ty;
    while let Some(open) = rest.find('[') {
        let Some(close) = rest[open..].find(']') else {
            return true;
        };
        let bound = rest[open + 1..open + close].trim();
        if !bound.is_empty() && !bound.bytes().all(|byte| byte.is_ascii_digit()) {
            return true;
        }
        rest = &rest[open + close + 1..];
    }
    false
}

/// A constant address: a string literal decaying, or a null pointer constant.
fn constant_address(node: &Value) -> bool {
    let node = strip_parens(node);
    match kind(node) {
        "StringLiteral" => true,
        "ImplicitCastExpr" | "CStyleCastExpr" => {
            matches!(
                node["castKind"].as_str(),
                Some("ArrayToPointerDecay" | "NullToPointer" | "NoOp" | "BitCast")
            ) && children(node).first().is_some_and(constant_address)
                || node["castKind"].as_str() == Some("NullToPointer")
        }
        _ => false,
    }
}

fn place_actual(
    node: &Value,
    fields: &std::collections::BTreeMap<String, Option<crate::typed_access_path::FieldFacts>>,
) -> Result<Actual, SubstitutionRefusal> {
    match crate::typed_place::read_place(node, fields) {
        Ok(place) => {
            let mut spelling = place.name.clone();
            for step in &place.steps {
                match step.kind {
                    crate::typed_place::StepKind::Arrow => {
                        spelling.push_str("->");
                        spelling.push_str(&step.member);
                    }
                    crate::typed_place::StepKind::Dot => {
                        spelling.push('.');
                        spelling.push_str(&step.member);
                    }
                    crate::typed_place::StepKind::Index { .. } => spelling.push_str("[]"),
                    crate::typed_place::StepKind::Deref { .. } => spelling.insert(0, '*'),
                }
            }
            Ok(Actual::Place { spelling })
        }
        Err(refusal) => Err(SubstitutionRefusal::Place(format!("{refusal:?}"))),
    }
}

/// An expression is data when every value it *evaluates* is a scalar read of
/// a named object, a literal or an enumerator combined by scalar operators;
/// the operand of `sizeof`/`_Alignof` is not evaluated.
fn data_actual(node: &Value) -> Result<Actual, SubstitutionRefusal> {
    fn walk(node: &Value) -> Result<(), SubstitutionRefusal> {
        match kind(node) {
            "UnaryExprOrTypeTraitExpr" => {
                // `sizeof(T)` / `sizeof E`: E is not evaluated unless its type
                // is a variable-length array (C11 6.5.3.4p2).
                let queried = node["argType"]["qualType"]
                    .as_str()
                    .map(str::to_owned)
                    .or_else(|| children(node).first().and_then(qual).map(str::to_owned));
                if queried.as_deref().is_none_or(variable_length) {
                    return Err(SubstitutionRefusal::Form(
                        "variable-length type query",
                        "CG08",
                    ));
                }
                Ok(())
            }
            "IntegerLiteral" | "CharacterLiteral" | "FloatingLiteral" | "StringLiteral" => Ok(()),
            "DeclRefExpr" => match node["referencedDecl"]["kind"].as_str() {
                Some("EnumConstantDecl") => Ok(()),
                Some("VarDecl" | "ParmVarDecl") => {
                    if qual(node).is_some_and(|ty| ty.contains('*') || ty.contains('[')) {
                        Err(SubstitutionRefusal::Form("pointer arithmetic", "CG16"))
                    } else {
                        Ok(())
                    }
                }
                _ => Err(SubstitutionRefusal::Form(
                    "reference to a non-object",
                    "CG22",
                )),
            },
            "ParenExpr" => children(node).iter().try_for_each(walk),
            "ImplicitCastExpr" | "CStyleCastExpr" => {
                match node["castKind"].as_str() {
                    Some(
                        "LValueToRValue" | "NoOp" | "IntegralCast" | "IntegralToFloating"
                        | "FloatingToIntegral" | "FloatingCast" | "IntegralToBoolean",
                    ) => {}
                    // Card CG08-CASTS-AND-TYPE-QUERIES (rule 11b): the
                    // remaining scalar representation, and the null test of
                    // a named pointer object, whose truth value carries no
                    // identity (`QCore.CastRepresentation`).
                    Some("FloatingToBoolean") => {}
                    Some("PointerToBoolean") => {
                        let operand = crate::elaboration::pointer_operand(
                            children(node).first().unwrap_or(node),
                        );
                        return match (kind(operand), operand["referencedDecl"]["kind"].as_str()) {
                            ("DeclRefExpr", Some("VarDecl" | "ParmVarDecl")) => Ok(()),
                            _ => Err(SubstitutionRefusal::Form("cast", "CG08")),
                        };
                    }
                    Some("ArrayToPointerDecay" | "NullToPointer" | "BitCast") => {
                        // A constant address: a string literal's decay or a
                        // null pointer constant.
                        if !constant_address(node) {
                            return Err(SubstitutionRefusal::Form("pointer arithmetic", "CG16"));
                        }
                        return Ok(());
                    }
                    _ => return Err(SubstitutionRefusal::Form("cast", "CG08")),
                }
                children(node).iter().try_for_each(walk)
            }
            "UnaryOperator" => match node["opcode"].as_str() {
                Some("-" | "+" | "~" | "!") => children(node).iter().try_for_each(walk),
                Some("++" | "--") => Err(SubstitutionRefusal::Form("effectful actual", "CG03")),
                _ => Err(SubstitutionRefusal::Form("indirect access", "CG10")),
            },
            "BinaryOperator" => match node["opcode"].as_str() {
                Some("&&" | "||") => Err(SubstitutionRefusal::Form("value && ||", "CG06")),
                Some("=" | ",") => Err(SubstitutionRefusal::Form("effectful actual", "CG03")),
                _ if qual(node).is_some_and(|ty| ty.contains('*')) => {
                    Err(SubstitutionRefusal::Form("pointer arithmetic", "CG16"))
                }
                _ => children(node).iter().try_for_each(walk),
            },
            "CompoundAssignOperator" => Err(SubstitutionRefusal::Form("effectful actual", "CG03")),
            "ConditionalOperator" | "BinaryConditionalOperator" => {
                Err(SubstitutionRefusal::Form("conditional", "CG06"))
            }
            "CallExpr" => Err(SubstitutionRefusal::Form("nested call", "CG05")),
            "MemberExpr" | "ArraySubscriptExpr" => Err(SubstitutionRefusal::Form(
                "place inside an expression",
                "CG10",
            )),
            _ => Err(SubstitutionRefusal::Form("unlisted form", "CG03")),
        }
    }
    walk(node)?;
    // A pointer-typed value that is no named object is data only as a
    // constant address.
    if qual(node).is_some_and(|ty| ty.contains('*')) && !constant_address(node) {
        return Err(SubstitutionRefusal::Form(
            "pointer-valued expression",
            "CG16",
        ));
    }
    Ok(Actual::Data)
}

/// Substitute every formal of a call (`node` a `CallExpr`) against `modes`:
/// total or rejected. A place or an address at a custody-taking formal other
/// than `Out` is refused by design.
pub(crate) fn substitute(
    node: &Value,
    modes: &[ResourceParameterMode],
    fields: &std::collections::BTreeMap<String, Option<crate::typed_access_path::FieldFacts>>,
) -> Result<Vec<Substituted>, (usize, SubstitutionRefusal)> {
    let call = strip_parens(node);
    if kind(call) != "CallExpr" {
        return Err((0, SubstitutionRefusal::Form("not a call", "CG03")));
    }
    let actuals = children(call).get(1..).unwrap_or_default();
    if actuals.len() != modes.len() {
        return Err((0, SubstitutionRefusal::Arity));
    }
    let mut out = Vec::with_capacity(modes.len());
    for (formal, (actual, mode)) in actuals.iter().zip(modes).enumerate() {
        let read = read_actual(actual, fields).map_err(|refusal| (formal, refusal))?;
        match (&read, mode) {
            (
                Actual::Address { .. },
                ResourceParameterMode::Out { .. } | ResourceParameterMode::Borrowed,
            ) => {}
            (Actual::Address { .. }, _) => {
                return Err((formal, SubstitutionRefusal::CustodyPosition("address")));
            }
            (Actual::Place { .. }, mode) if custody_taking(*mode) => {
                return Err((formal, SubstitutionRefusal::CustodyPosition("place")));
            }
            (Actual::Data, ResourceParameterMode::Out { .. }) => {
                return Err((
                    formal,
                    SubstitutionRefusal::Form("data at an out slot", "CG03"),
                ));
            }
            // A scalar value at a custody-taking formal is a descriptor's
            // identity, which the lexical descriptor route owns.
            (Actual::Data, mode) if custody_taking(*mode) => {
                return Err((formal, SubstitutionRefusal::CustodyPosition("value")));
            }
            _ => {}
        }
        out.push(Substituted {
            formal,
            mode: *mode,
            actual: read,
        });
    }
    Ok(out)
}

/// `(C actual, formal mode, expected form or refusal code)`, shared with
/// `proof/QCore/CallInstantiation.lean` (`substitutionVectors`). The Rust
/// test binds each spelling to Clang's reading of it.
#[cfg(test)]
pub(crate) const SUBSTITUTION_VECTORS: &[(&str, &str, &str)] = &[
    ("p", "released", "identity"),
    ("(struct T *)p", "released", "identity-cast"),
    ("(void *)(char *)p", "released", "identity-cast"),
    ("&p", "out", "address"),
    ("&n", "borrowed", "address"),
    ("n * 2 + sizeof(*p)", "borrowed", "data"),
    ("sizeof(struct T)", "borrowed", "data"),
    ("\"text\"", "borrowed", "data"),
    ("s->p", "borrowed", "place"),
    ("s->p", "released", "custody-position:place"),
    ("&n", "released", "custody-position:address"),
    ("n", "released", "custody-position:value"),
    ("(char *)p + 1", "borrowed", "form:pointer arithmetic"),
    ("i++", "borrowed", "form:effectful actual"),
    ("n ? p : 0", "borrowed", "form:conditional"),
    ("(long)p", "borrowed", "form:cast"),
];

#[cfg(test)]
#[path = "../tests/unit/call_instantiation.rs"]
mod tests;
