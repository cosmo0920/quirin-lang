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

//! Card **CG08-CASTS-AND-TYPE-QUERIES** (gateway rule 11b): C casts as
//! *representations* and `sizeof`/`_Alignof` operands by evaluation class,
//! over Clang's typed AST (gap entries E15, E16, M18, N08 (representation),
//! N11; evidence Q11, Q16, Q22, Q23, Q29, Q37, Q40 and example D).
//!
//! # Casts
//!
//! [`CAST_TABLE`] lists every Clang `CastKind` the route reads, each with two
//! explicit flags and one admission:
//!
//! * [`Preservation`]: whether every value of the source type is kept
//!   (`Preserving`) or the conversion may lose information (`PossiblyLossy`:
//!   narrowing, rounding, a many-to-one test);
//! * [`Provenance`]: whether the result still designates the object the
//!   operand designated (`Kept`, a pointer-to-pointer conversion), no longer
//!   carries it (`Lost`, a pointer converted to an integer or a truth value),
//!   or had none to carry (`NotApplicable`);
//! * the **custody effect of every row is none**: a cast never acquires,
//!   releases, moves or binds a resource. What a cast can do is *hide* one
//!   (a tracked pointer converted to an integer escapes the custody frame) or
//!   *invent* one (an integer converted to a pointer), and the admissions are
//!   written so that neither happens: a pointer-to-integer conversion is read
//!   only of a pointer the route proves carries no identity
//!   ([`Admission::PointerToInteger`]), and an integer-to-pointer conversion
//!   yields a pointer with **no** identity, never a guessed one
//!   ([`Admission::IntegerToPointer`]).
//!
//! `PointerToBoolean` is the null test (C11 6.3.1.2: the result is 0 exactly
//! when the pointer compares equal to a null pointer); its value carries no
//! pointer. Numeric transfer (ranges, overflow, rounding values) is not read
//! here (CG20): a lossy row adds the elaborator's `NoNumericClaim`. A kind the
//! table does not list is refused by name.
//!
//! # Type queries
//!
//! C11 6.5.3.4p2: the operand of `sizeof` is evaluated only when its type is a
//! variable length array type; `_Alignof` never evaluates. [`query_operand`]
//! reads the class from Clang's node: an operand whose type is not variably
//! modified is [`QueryOperand::Unevaluated`] (no event at all: `sizeof(*p)`
//! and `sizeof p->f` dereference nothing); a `sizeof` of a variably modified
//! type written in the query evaluates the size expressions Clang lists as
//! the node's children, **once** ([`QueryOperand::Evaluated`]); every other
//! shape (an expression operand of variably modified type other than a plain
//! name, `_Alignof` of one, the other traits) is refused.
//!
//! The value of a query is a scalar whose number no consumer here reads: the
//! custody routes never use it in a bound or a branch (constraint 5a, example
//! D). Where a number is consumed, it is Clang's own evaluation
//! ([`crate::type_query_value`]), or the consumer refuses.
//!
//! The Lean model is `QCore.CastRepresentation`, bound to [`CAST_TABLE`] and
//! [`query_operand`] by the shared-vector tests in
//! `tests/unit/cast_representation.rs`.

use serde_json::Value;

use crate::typed_c_ast::{children, kind};

/// Whether a conversion keeps every value of its source type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Preservation {
    Preserving,
    PossiblyLossy,
}

/// Whether the result still designates the operand's object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Provenance {
    /// A pointer converted to a pointer: the same address (C11 6.3.2.3p7,
    /// for a correctly aligned object pointer; the premise is stated).
    Kept,
    /// A pointer converted to an integer or a truth value.
    Lost,
    /// The operand carried no provenance (an integer, a floating value, the
    /// null constant, a discarded value).
    NotApplicable,
}

/// How the elaborator reads one row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Admission {
    /// Scalar to scalar: the operand's events, a computed scalar value.
    Scalar,
    /// `PointerToBoolean`: the pointer's events, a computed truth value
    /// (the null test).
    NullTest,
    /// `PointerToIntegral`: admitted only of a pointer the route proves
    /// carries no identity; otherwise an escape, refused by name.
    PointerToInteger,
    /// `IntegralToPointer`: a pointer value with no identity.
    IntegerToPointer,
    /// An object-pointer `BitCast` or `NoOp`: the same pointer value.
    PointerValue,
    /// `NullToPointer`: the null pointer, no identity.
    NullPointer,
    /// `ToVoid`: the operand's events, no value.
    Discard,
    /// Listed, read by no rule of this card; the owning card.
    Refused(&'static str),
}

/// One row of [`CAST_TABLE`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Representation {
    pub(crate) kind: &'static str,
    pub(crate) preservation: Preservation,
    pub(crate) provenance: Provenance,
    pub(crate) admission: Admission,
}

const fn row(
    kind: &'static str,
    preservation: Preservation,
    provenance: Provenance,
    admission: Admission,
) -> Representation {
    Representation {
        kind,
        preservation,
        provenance,
        admission,
    }
}

use Admission as A;
use Preservation::{PossiblyLossy as Lossy, Preserving};
use Provenance::{Kept, Lost, NotApplicable as None_};

/// Every cast kind the route reads (`QCore.CastRepresentation.castTable`).
/// `LValueToRValue` is a read, not a representation, and stays the
/// elaborator's own arm; `NoOp`/`IntegralCast` between scalars and
/// `FunctionToPointerDecay` keep their CG02 arms, which are asked first.
pub(crate) const CAST_TABLE: [Representation; 17] = [
    row("IntegralCast", Lossy, None_, A::Scalar),
    row("IntegralToBoolean", Lossy, None_, A::Scalar),
    row("FloatingToBoolean", Lossy, None_, A::Scalar),
    row("IntegralToFloating", Lossy, None_, A::Scalar),
    row("FloatingToIntegral", Lossy, None_, A::Scalar),
    row("FloatingCast", Lossy, None_, A::Scalar),
    row("PointerToBoolean", Lossy, Lost, A::NullTest),
    row("PointerToIntegral", Lossy, Lost, A::PointerToInteger),
    row("IntegralToPointer", Lossy, None_, A::IntegerToPointer),
    row("BitCast", Preserving, Kept, A::PointerValue),
    row("NoOp", Preserving, Kept, A::PointerValue),
    row("NullToPointer", Preserving, None_, A::NullPointer),
    row("ToVoid", Preserving, None_, A::Discard),
    row("AtomicToNonAtomic", Preserving, Kept, A::Refused("CG19")),
    row("NonAtomicToAtomic", Preserving, Kept, A::Refused("CG19")),
    row(
        "AddressSpaceConversion",
        Preserving,
        Kept,
        A::Refused("CG08"),
    ),
    row("ArrayToPointerDecay", Preserving, Kept, A::Refused("CG16")),
];

/// The row of a cast kind, `None` for an unlisted one (refused).
pub(crate) fn representation(kind: &str) -> Option<Representation> {
    CAST_TABLE.iter().find(|row| row.kind == kind).copied()
}

/// The implicit conversions this card adds to the elaborator's gate: the
/// ones CG02's arms refused and the table admits with a value (object-pointer
/// `BitCast`/`NoOp` and `NullToPointer` are everywhere and change no
/// admission on their own, so they do not open a consultation).
const GATE_KINDS: [&str; 8] = [
    "IntegralToBoolean",
    "FloatingToBoolean",
    "IntegralToFloating",
    "FloatingToIntegral",
    "FloatingCast",
    "PointerToBoolean",
    "PointerToIntegral",
    "IntegralToPointer",
];

/// The CG08 form a node is, when it is one: an explicit cast, one of the
/// implicit conversions of [`GATE_KINDS`], or a type query. Route G consults
/// the elaborator for a refused statement, `return` or call-free condition
/// holding one (rule 11b), beside CG02's value forms.
pub(crate) fn form(node: &Value) -> Option<&'static str> {
    match kind(node) {
        "CStyleCastExpr" => Some("explicit cast"),
        "ImplicitCastExpr" => node["castKind"]
            .as_str()
            .and_then(|cast| GATE_KINDS.iter().find(|gate| **gate == cast))
            .copied(),
        "UnaryExprOrTypeTraitExpr" => Some("type query"),
        _ => None,
    }
}

/// The evaluation class of a type query's operand.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum QueryOperand<'a> {
    /// Not evaluated (C11 6.5.3.4p2): no event.
    Unevaluated,
    /// A `sizeof` of a variably modified type: these size expressions are
    /// evaluated, each once.
    Evaluated(Vec<&'a Value>),
    /// Refused by name.
    Refused(&'static str),
}

/// Whether a type's text is variably modified: a bracketed extent that is not
/// a decimal constant (`int[n]`, `int (*)[n]`). An incomplete `[]` is not.
pub(crate) fn variably_modified(text: &str) -> bool {
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        let Some(close) = rest[open..].find(']') else {
            return true;
        };
        let extent = rest[open + 1..open + close].trim();
        if !extent.bytes().all(|byte| byte.is_ascii_digit()) {
            return true;
        }
        rest = &rest[open + close + 1..];
    }
    false
}

/// Every spelling of a type Clang gives (a variably modified typedef shows
/// only in its desugared spelling).
fn type_texts(ty: &Value) -> impl Iterator<Item = &str> {
    ["qualType", "desugaredQualType"]
        .into_iter()
        .filter_map(move |key| ty[key].as_str())
}

fn modified(ty: &Value) -> bool {
    type_texts(ty).any(variably_modified) || ty.is_null()
}

/// The evaluation class of a `UnaryExprOrTypeTraitExpr`'s operand.
pub(crate) fn query_operand(node: &Value) -> QueryOperand<'_> {
    let name = node["name"].as_str().unwrap_or("");
    if !matches!(name, "sizeof" | "alignof") {
        return QueryOperand::Refused("a type trait other than sizeof or _Alignof");
    }
    let operands = children(node);
    if !node["argType"].is_null() {
        // A type operand. Its size expressions, when written in the query,
        // are the node's children.
        if !modified(&node["argType"]) {
            return if operands.is_empty() {
                QueryOperand::Unevaluated
            } else {
                QueryOperand::Refused("a constant type operand with children")
            };
        }
        return if name == "sizeof" {
            QueryOperand::Evaluated(operands.iter().collect())
        } else {
            QueryOperand::Refused("_Alignof of a variably modified type")
        };
    }
    let [operand] = operands else {
        return QueryOperand::Refused("an expression operand that is not one node");
    };
    if !modified(&operand["type"]) {
        return QueryOperand::Unevaluated;
    }
    // An expression of variably modified type is evaluated; only a plain
    // name (an array designation, which reads nothing) is read here.
    if name == "sizeof" && kind(crate::elaboration::strip_parens(operand)) == "DeclRefExpr" {
        QueryOperand::Evaluated(Vec::new())
    } else {
        QueryOperand::Refused("an evaluated expression operand")
    }
}

/// The probe enumerator [`clang_constant`] reads.
const PROBE: &str = "__qcore_cg08_value";

/// Clang's own evaluation of the integer constant expression `expression`
/// over `source` (N11, example D): the value Clang's constant evaluator gives
/// an enumerator initialised with it (C11 6.6, 6.7.2.2), never a host
/// computation. A consumer that needs a query's or an enumerator's number
/// takes this or refuses (`QCore.CastRepresentation.consume`): an expression
/// that is not an integer constant expression over the unit's file scope (a
/// variably modified `sizeof`, a block-scope name) is a Clang error, and so
/// an `Err`.
pub(crate) fn clang_constant(
    source: &str,
    unit: &crate::legacy_c::C11TranslationUnit<'_>,
    expression: &str,
) -> Result<i128, String> {
    let probe = format!("{source}\nenum __qcore_cg08_probe {{ {PROBE} = ({expression}) }};\n");
    let declarations = crate::typed_c_ast::ast_declarations(&probe, unit, PROBE)?;
    fn find(node: &Value) -> Option<&Value> {
        if kind(node) == "EnumConstantDecl" && node["name"] == PROBE {
            return children(node).first();
        }
        children(node).iter().find_map(find)
    }
    let mut value = declarations
        .iter()
        .find_map(find)
        .ok_or("Clang evaluated no probe enumerator")?;
    // Clang converts the initializer to the enumerator's type around its
    // evaluated `ConstantExpr`.
    while kind(value) == "ImplicitCastExpr" {
        value = children(value)
            .first()
            .ok_or("a conversion without an operand")?;
    }
    if kind(value) != "ConstantExpr" {
        return Err("Clang gave no evaluated constant".into());
    }
    value["value"]
        .as_str()
        .ok_or("Clang gave no evaluated value")?
        .parse()
        .map_err(|_| "Clang's value is not an integer".into())
}

#[cfg(test)]
#[path = "../tests/unit/cast_representation.rs"]
mod tests;
