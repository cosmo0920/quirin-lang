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

//! Card **CAST-FORMALIZATION** (gateway rule 12h, amending rule 11b): one
//! formal cast semantics every route-G position consults.
//!
//! Rule 11b's [`CAST_TABLE`](crate::cast_representation::CAST_TABLE) reads a
//! cast by its Clang kind alone. A kind does not decide what a conversion
//! does to an identity or to an access: Clang spells a byte view
//! (`(char *) obj`), a typed view of a `void *` (`(struct T *) v`), a type pun
//! (`(struct B *) a`), a qualifier conversion and a function-pointer
//! conversion all as `BitCast` or `NoOp`. This module refines each kind by
//! the operand's and the result's type classes into one **conversion
//! class** (C11 §6.3, §6.5.4), and [`CONVERSION_TABLE`] states, once, for
//! each class:
//!
//! * its **identity** effect ([`Identity`]): data with no identity, the
//!   operand's identity preserved, the identity lost (provenance gone), no
//!   identity at all (an integer or the null constant made a pointer), no
//!   value (`(void) e`), or refused;
//! * the **access** an lvalue through the result may make ([`Access`]): the
//!   same object, a character view (C11 6.5p7, no premise; a `void *`
//!   result names no object to access), an effective-type view
//!   (`P-EFFECTIVE-TYPE`), a
//!   punned view (an incompatible effective type: refused, never certified),
//!   an untracked pointer, a callee (CG22), or refused;
//! * its **obligations**: the premises and owning rules an admission names.
//!
//! The table is the kind table refined, never forked: every class's kinds are
//! rows of `CAST_TABLE` (or the two decays it leaves to their owners), and
//! each class's identity agrees with the kind row's provenance flag
//! ([`agrees_with_cast_table`], `QCore.CastSemantics.agrees`).
//!
//! What a position does with the class is the position's own reading
//! (rule 12h): the class is decided here, from Clang's node, and nowhere else.
//! A class outside what a position admits refuses by name
//! ([`ConversionClass::refusal`]).
//!
//! The Lean model is `QCore.CastSemantics`, bound to [`CONVERSION_TABLE`] and
//! [`classify`] by the shared vectors [`CLASSIFY_VECTORS`] and the table test
//! in `tests/unit/cast_semantics.rs`.

use serde_json::Value;

use crate::typed_c_ast::{children, kind};

/// The class of one type text, read from Clang's desugared spelling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TypeClass {
    /// An arithmetic, enumeration or `_Bool` type.
    Scalar,
    /// `void` (the type of a discarded value).
    Void,
    /// A pointer, with its pointee's class.
    Pointer(Pointee),
    /// A record, array, union or a spelling this reader does not classify.
    Other,
}

/// The class of a pointer's pointee.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Pointee {
    /// `void` (any qualifiers).
    Void,
    /// `char`, `signed char` or `unsigned char` (C11 6.5p7's character
    /// type), with its spelling.
    Character(String),
    /// A function type: the pointer is a function pointer.
    Function,
    /// Any other object type, keyed by its unqualified spelling.
    Object(String),
}

/// Qualifiers of a pointer's pointee that change what an access may do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PointeeQualifiers {
    pub(crate) volatile: bool,
    pub(crate) atomic: bool,
    pub(crate) constant: bool,
}

const QUALIFIERS: [&str; 7] = [
    "const",
    "volatile",
    "restrict",
    "__restrict",
    "__restrict__",
    "_Atomic",
    "__unaligned",
];

const SCALAR_WORDS: [&str; 15] = [
    "_Bool",
    "bool",
    "char",
    "short",
    "int",
    "long",
    "signed",
    "unsigned",
    "float",
    "double",
    "__int128",
    "_Float16",
    "__fp16",
    "__int128_t",
    "__uint128_t",
];

/// The pointee text of a pointer type and its qualifiers, when `text` is a
/// pointer: `T *`, `T *const`, `T (*)(args)`, `T (*)[N]`.
fn pointer_parts(text: &str) -> Option<(String, PointeeQualifiers)> {
    let text = text.trim();
    // A pointer to a function or an array: `R (*)(…)` / `R (*)[…]`, possibly
    // `R (*const)(…)`.
    if let Some(open) = text.find("(*") {
        let close = open + text[open..].find(')')?;
        let declarator = &text[open + 2..close];
        if declarator.contains('*') {
            // `R (**)(…)`: a pointer to a function pointer, an object.
            let inner = format!(
                "{}(*{}){}",
                &text[..open],
                &declarator[1..],
                &text[close + 1..]
            );
            return Some((inner, PointeeQualifiers::default()));
        }
        if !declarator
            .split_whitespace()
            .all(|word| QUALIFIERS.contains(&word))
        {
            return None;
        }
        let rest = text[close + 1..].trim_start();
        let pointee = format!("{}{}", text[..open].trim_end(), rest);
        let pointee = if rest.starts_with('(') {
            format!("fn:{pointee}")
        } else {
            pointee
        };
        return Some((pointee, PointeeQualifiers::default()));
    }
    let star = text.rfind('*')?;
    if !text[star + 1..]
        .split_whitespace()
        .all(|word| QUALIFIERS.contains(&word))
    {
        return None;
    }
    let inner = text[..star].trim_end();
    let mut qualifiers = PointeeQualifiers::default();
    // The pointee's own qualifiers are the words after its last `*` (or all
    // of its words when it is not itself a pointer).
    let own = inner.rfind('*').map_or(inner, |at| &inner[at + 1..]);
    for word in own.split_whitespace() {
        match word {
            "volatile" => qualifiers.volatile = true,
            "_Atomic" => qualifiers.atomic = true,
            "const" => qualifiers.constant = true,
            _ => {}
        }
    }
    if inner.contains("_Atomic(") {
        qualifiers.atomic = true;
    }
    Some((inner.to_owned(), qualifiers))
}

/// The unqualified key of a pointee text: its words with every qualifier
/// removed, so `const struct T` and `struct T` are one key.
fn unqualified_key(text: &str) -> String {
    let spaced = text.replace('*', " * ");
    spaced
        .split_whitespace()
        .filter(|word| !QUALIFIERS.contains(word))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The class of a type text and, for a pointer, its pointee's qualifiers.
pub(crate) fn type_class(text: &str) -> (TypeClass, PointeeQualifiers) {
    if let Some((pointee, qualifiers)) = pointer_parts(text) {
        if pointee.starts_with("fn:") {
            return (TypeClass::Pointer(Pointee::Function), qualifiers);
        }
        let key = unqualified_key(&pointee);
        let class = match key.as_str() {
            "void" => Pointee::Void,
            "char" | "signed char" | "unsigned char" => Pointee::Character(key),
            "" => return (TypeClass::Other, qualifiers),
            _ => Pointee::Object(key),
        };
        return (TypeClass::Pointer(class), qualifiers);
    }
    let key = unqualified_key(text);
    if key == "void" {
        return (TypeClass::Void, PointeeQualifiers::default());
    }
    let words = key.split_whitespace().collect::<Vec<_>>();
    if !words.is_empty()
        && (words.iter().all(|word| SCALAR_WORDS.contains(word)) || words[0] == "enum")
    {
        return (TypeClass::Scalar, PointeeQualifiers::default());
    }
    (TypeClass::Other, PointeeQualifiers::default())
}

/// One conversion class (C11 §6.3, §6.5.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ConversionClass {
    /// Scalar to scalar (6.3.1): data; its numbers are CG20's (rule 11q).
    Scalar,
    /// A pointer to a truth value (6.3.1.2): the null test.
    NullTest,
    /// A pointer to an integer (6.3.2.3p6): provenance lost.
    PointerToInteger,
    /// An integer to a pointer (6.3.2.3p5): never a tracked identity.
    IntegerToPointer,
    /// A null pointer constant to a pointer (6.3.2.3p3).
    NullPointer,
    /// An object pointer to `void *` or a character pointer (6.3.2.3p1,
    /// p7): the same object, read only as bytes (6.5p7) or not at all.
    ByteView,
    /// `void *` or a character pointer to another object pointer (6.3.2.3p1,
    /// p7): the same object, accessed at the target type only under
    /// `P-EFFECTIVE-TYPE` (6.5p6-7). The round trip of a [`Self::ByteView`].
    TypedView,
    /// An object pointer to a pointer to another, non-character object type:
    /// the same object, but an access through it reads an incompatible
    /// effective type (6.5p7) and is refused, never certified.
    PunnedView,
    /// The same pointee type with other qualifiers (6.3.2.3p2): the same
    /// object; an added `volatile` keeps rule 11h's limits.
    Qualifier,
    /// A function pointer to another function pointer type (6.3.2.3p8): a
    /// callee set (CG22); a call through an incompatible type is refused.
    FunctionPointer,
    /// A function pointer to an object pointer or back: not C (6.3.2.3 has
    /// no such conversion), refused.
    FunctionObject,
    /// `(void) e` (6.3.2.2): `e` evaluated for its effect only.
    Discard,
    /// `_Atomic` conversions: CG19's.
    Atomic,
    /// An address-space conversion: no model.
    AddressSpace,
    /// An array designator decayed to its first element's address (6.3.2.1p3):
    /// an address, CG16's.
    ArrayDecay,
    /// A function designator decayed to a function pointer (6.3.2.1p4): a
    /// callee, CG22's.
    FunctionDecay,
    /// A kind or type pair outside this table: refused by name.
    Unlisted,
}

/// A conversion's effect on the identity its operand carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Identity {
    /// Neither side is a pointer: data, no identity.
    Data,
    /// The result designates the operand's object (or its callee).
    Preserved,
    /// The operand's identity does not survive the conversion (an integer or
    /// a truth value carries none).
    Lost,
    /// The result is a pointer with no identity (null, or made of an integer).
    Absent,
    /// No value (`(void) e`).
    NoValue,
    /// Refused by name.
    Refused,
}

/// What an lvalue formed through the conversion's result may access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Access {
    /// The result is not a pointer.
    NotPointer,
    /// The object, at the operand's own type.
    Same,
    /// Bytes through a character type (6.5p7): no premise.
    Character,
    /// The object at the target type, under `P-EFFECTIVE-TYPE`.
    EffectiveType,
    /// An incompatible effective type: refused.
    Punned,
    /// A pointer with no tracked identity: never a custody access.
    Untracked,
    /// The null pointer: no object.
    Null,
    /// A callee, CG22's callee set.
    Callee,
    /// Refused by name.
    Refused,
}

/// One row of [`CONVERSION_TABLE`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Conversion {
    pub(crate) class: ConversionClass,
    /// The Clang kinds this class refines.
    pub(crate) kinds: &'static [&'static str],
    pub(crate) identity: Identity,
    pub(crate) access: Access,
    /// The premises and rules an admission of this class names.
    pub(crate) obligations: &'static [&'static str],
    /// The C11 clause.
    pub(crate) clause: &'static str,
}

/// `P-EFFECTIVE-TYPE` (rule 11k, the core's `EffectiveTypeDeclared`).
pub(crate) const P_EFFECTIVE_TYPE: &str = "P-EFFECTIVE-TYPE";
/// C11 6.3.2.3p7: the converted pointer is correctly aligned for its type
/// (rule 11b's stated premise for an object-pointer conversion).
pub(crate) const P_ALIGNED_CONVERSION: &str = "P-ALIGNED-POINTER-CONVERSION";

use ConversionClass as C;

const SCALAR_KINDS: &[&str] = &[
    "IntegralCast",
    "IntegralToBoolean",
    "FloatingToBoolean",
    "IntegralToFloating",
    "FloatingToIntegral",
    "FloatingCast",
    "NoOp",
];

/// The conversion table (`QCore.CastSemantics.conversionTable`): one row per
/// class, every position's single authority.
pub(crate) const CONVERSION_TABLE: [Conversion; 17] = [
    Conversion {
        class: C::Scalar,
        kinds: SCALAR_KINDS,
        identity: Identity::Data,
        access: Access::NotPointer,
        obligations: &["rule 11q (CG20): the numeric conversion's obligations"],
        clause: "6.3.1",
    },
    Conversion {
        class: C::NullTest,
        kinds: &["PointerToBoolean"],
        identity: Identity::Lost,
        access: Access::NotPointer,
        obligations: &[],
        clause: "6.3.1.2",
    },
    Conversion {
        class: C::PointerToInteger,
        kinds: &["PointerToIntegral"],
        identity: Identity::Lost,
        access: Access::NotPointer,
        obligations: &["rule 11b: the pointer provably carries no tracked identity"],
        clause: "6.3.2.3p6",
    },
    Conversion {
        class: C::IntegerToPointer,
        kinds: &["IntegralToPointer"],
        identity: Identity::Absent,
        access: Access::Untracked,
        obligations: &[],
        clause: "6.3.2.3p5",
    },
    Conversion {
        class: C::NullPointer,
        kinds: &["NullToPointer"],
        identity: Identity::Absent,
        access: Access::Null,
        obligations: &[],
        clause: "6.3.2.3p3",
    },
    Conversion {
        class: C::ByteView,
        kinds: &["BitCast", "NoOp"],
        identity: Identity::Preserved,
        access: Access::Character,
        obligations: &[],
        clause: "6.3.2.3p1, p7; 6.5p7",
    },
    Conversion {
        class: C::TypedView,
        kinds: &["BitCast", "NoOp"],
        identity: Identity::Preserved,
        access: Access::EffectiveType,
        obligations: &[P_EFFECTIVE_TYPE, P_ALIGNED_CONVERSION],
        clause: "6.3.2.3p1, p7; 6.5p6-7",
    },
    Conversion {
        class: C::PunnedView,
        kinds: &["BitCast"],
        identity: Identity::Preserved,
        access: Access::Punned,
        obligations: &[P_ALIGNED_CONVERSION],
        clause: "6.3.2.3p7; 6.5p7",
    },
    Conversion {
        class: C::Qualifier,
        kinds: &["NoOp", "BitCast"],
        identity: Identity::Preserved,
        access: Access::Same,
        obligations: &["rule 11h: an added volatile keeps its limits"],
        clause: "6.3.2.3p2",
    },
    Conversion {
        class: C::FunctionPointer,
        kinds: &["BitCast", "NoOp"],
        identity: Identity::Preserved,
        access: Access::Callee,
        obligations: &["rule 11p (CG22): a call through an incompatible type is refused"],
        clause: "6.3.2.3p8",
    },
    Conversion {
        class: C::FunctionObject,
        kinds: &["BitCast"],
        identity: Identity::Refused,
        access: Access::Refused,
        obligations: &[],
        clause: "6.3.2.3 (no such conversion)",
    },
    Conversion {
        class: C::Discard,
        kinds: &["ToVoid"],
        identity: Identity::NoValue,
        access: Access::NotPointer,
        obligations: &[],
        clause: "6.3.2.2",
    },
    Conversion {
        class: C::Atomic,
        kinds: &["AtomicToNonAtomic", "NonAtomicToAtomic"],
        identity: Identity::Refused,
        access: Access::Refused,
        obligations: &["rule 11i (CG19)"],
        clause: "6.2.5p27",
    },
    Conversion {
        class: C::AddressSpace,
        kinds: &["AddressSpaceConversion"],
        identity: Identity::Refused,
        access: Access::Refused,
        obligations: &[],
        clause: "(extension)",
    },
    Conversion {
        class: C::ArrayDecay,
        kinds: &["ArrayToPointerDecay"],
        identity: Identity::Refused,
        access: Access::Refused,
        obligations: &["rule 11s (CG16): an address"],
        clause: "6.3.2.1p3",
    },
    Conversion {
        class: C::FunctionDecay,
        kinds: &["FunctionToPointerDecay"],
        identity: Identity::Preserved,
        access: Access::Callee,
        obligations: &["rule 11p (CG22)"],
        clause: "6.3.2.1p4",
    },
    Conversion {
        class: C::Unlisted,
        kinds: &[],
        identity: Identity::Refused,
        access: Access::Refused,
        obligations: &[],
        clause: "",
    },
];

/// The row of a class.
pub(crate) fn conversion(class: ConversionClass) -> &'static Conversion {
    CONVERSION_TABLE
        .iter()
        .find(|row| row.class == class)
        .unwrap_or(&CONVERSION_TABLE[CONVERSION_TABLE.len() - 1])
}

impl ConversionClass {
    /// The class's stable name (census rows, refusals, Lean vectors).
    pub(crate) fn name(self) -> &'static str {
        match self {
            C::Scalar => "scalar",
            C::NullTest => "null-test",
            C::PointerToInteger => "pointer-to-integer",
            C::IntegerToPointer => "integer-to-pointer",
            C::NullPointer => "null-pointer",
            C::ByteView => "byte-view",
            C::TypedView => "typed-view",
            C::PunnedView => "punned-view",
            C::Qualifier => "qualifier",
            C::FunctionPointer => "function-pointer",
            C::FunctionObject => "function-object",
            C::Discard => "discard",
            C::Atomic => "atomic",
            C::AddressSpace => "address-space",
            C::ArrayDecay => "array-decay",
            C::FunctionDecay => "function-decay",
            C::Unlisted => "unlisted",
        }
    }

    /// Whether the class's result designates the operand's object, so a
    /// position may read the cast as its operand's identity.
    pub(crate) fn preserves_identity(self) -> bool {
        conversion(self).identity == Identity::Preserved
    }
}

/// Card CAST-FORMALIZATION, H-V (rule 12h): whether an explicit cast standing
/// where a pointer is read through (`((T *)p)->f`, `*(T *)p`, `((T *)p)[i]`)
/// is a **view**, by the table's access rule: a `typed-view` (from `void *`
/// or a character pointer; `P-EFFECTIVE-TYPE`) and a `byte-view` to a
/// character pointer (C11 6.5p7, no premise) are; a `punned-view` (an
/// incompatible effective type), a function/object conversion and every other
/// class are not. The type gate is CG12's: both sides pass the positive
/// pointer test (no union, `volatile`, `_Atomic` or function pointee) and the
/// target names an object (`void_view::pointee_key`). Returns the operand.
pub(crate) fn access_view(node: &Value) -> Result<&Value, crate::void_view::ViewRefusal> {
    use crate::void_view::{OperandClass, ViewRefusal, operand_class, pointee_key};
    let class = classify_node(node).ok_or(ViewRefusal::Missing)?;
    let [operand] = children(node) else {
        return Err(ViewRefusal::Missing);
    };
    let from = crate::typed_access_path::qual(operand).ok_or(ViewRefusal::Missing)?;
    let to = crate::typed_access_path::qual(node).ok_or(ViewRefusal::Missing)?;
    match conversion(class).access {
        Access::EffectiveType | Access::Character => {
            if operand_class(from) == OperandClass::Other {
                return Err(ViewRefusal::Operand);
            }
            if pointee_key(to).is_none() || type_class(to).1.volatile {
                return Err(ViewRefusal::Target);
            }
            Ok(operand)
        }
        Access::Punned => Err(ViewRefusal::Operand),
        Access::Callee | Access::Refused => Err(ViewRefusal::Target),
        _ => Err(ViewRefusal::Kind),
    }
}

/// Card CAST-FORMALIZATION, H-V (rule 12h): the operand of an explicit
/// `qualifier` conversion (the same pointee type, C11 6.3.2.3p2) standing
/// where a pointer is read through: the same object at the same type, so the
/// path reads through it with no view. An added `volatile` or `_Atomic`
/// keeps its refusal (rule 11h; CG19), and so does a pointee the positive
/// pointer test refuses.
pub(crate) fn transparent_cast(mut node: &Value) -> Option<&Value> {
    while kind(node) == "ParenExpr" {
        node = children(node).first()?;
    }
    if kind(node) != "CStyleCastExpr" || classify_node(node)? != ConversionClass::Qualifier {
        return None;
    }
    let [operand] = children(node) else {
        return None;
    };
    let to = crate::typed_access_path::qual(node)?;
    let from = crate::typed_access_path::qual(operand)?;
    let (_, to_qualifiers) = type_class(to);
    let (_, from_qualifiers) = type_class(from);
    if to_qualifiers.volatile || to_qualifiers.atomic || from_qualifiers.atomic {
        return None;
    }
    crate::void_view::pointee_key(to)?;
    crate::void_view::pointee_key(from)?;
    Some(operand)
}

/// Card CAST-FORMALIZATION (rule 12h), in rule 12g's trailing-actual reader:
/// whether a class is read as its operand at a declared variadic tail
/// position. A `scalar` conversion is (the callee reads a value; an
/// integer-carried identity in the operand is still refused by the reader's
/// own name answer), and so is an identity-preserving pointer conversion
/// (`byte-view`, `typed-view`, `punned-view`, `qualifier`: the same object,
/// `QCore.CastSemantics.view_preserves_identity`). Every other class (a
/// pointer made an integer or back, null, a function pointer) is not.
pub(crate) fn tail_transparent(class: ConversionClass) -> bool {
    matches!(
        class,
        ConversionClass::Scalar
            | ConversionClass::ByteView
            | ConversionClass::TypedView
            | ConversionClass::PunnedView
            | ConversionClass::Qualifier
    )
}

/// Card CAST-FORMALIZATION, H-SR5: the opening index of the parenthesised
/// group closing at `close` when it spells a type name (identifiers, then
/// `*`s, at least one identifier): a cast group the dereference count reads
/// through. Whether the words resolve to a type is not asked: the caller
/// only refuses more by reading through it.
pub(crate) fn type_group_open(texts: &[&str], close: usize) -> Option<usize> {
    if texts.get(close) != Some(&")") {
        return None;
    }
    let open = (0..close).rev().find(|&at| texts[at] == "(")?;
    let group = &texts[open + 1..close];
    let stars = group.iter().rev().take_while(|text| **text == "*").count();
    let words = &group[..group.len() - stars];
    (!words.is_empty()
        && words.iter().all(|text| {
            text.chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        }))
    .then_some(open)
}

/// The scalar kinds whose `NoOp` reading is a scalar identity conversion.
fn scalar_kind(kind: &str) -> bool {
    SCALAR_KINDS.contains(&kind) || kind == "BooleanToSignedIntegral"
}

/// Classify one conversion from Clang's kind and the operand's and the
/// result's type texts (`QCore.CastSemantics.classify`).
pub(crate) fn classify(kind: &str, from: &str, to: &str) -> ConversionClass {
    let (from_class, from_qualifiers) = type_class(from);
    let (to_class, to_qualifiers) = type_class(to);
    classify_classes(kind, &from_class, from_qualifiers, &to_class, to_qualifiers)
}

/// [`classify`] over already classified types.
pub(crate) fn classify_classes(
    kind: &str,
    from: &TypeClass,
    from_qualifiers: PointeeQualifiers,
    to: &TypeClass,
    to_qualifiers: PointeeQualifiers,
) -> ConversionClass {
    use Pointee as P;
    use TypeClass as T;
    match kind {
        "PointerToBoolean" if matches!(from, T::Pointer(_)) => C::NullTest,
        "PointerToIntegral" if matches!(from, T::Pointer(_)) => C::PointerToInteger,
        "IntegralToPointer" if matches!(to, T::Pointer(_)) => C::IntegerToPointer,
        "NullToPointer" if matches!(to, T::Pointer(_)) => C::NullPointer,
        "ToVoid" => C::Discard,
        "AtomicToNonAtomic" | "NonAtomicToAtomic" => C::Atomic,
        "AddressSpaceConversion" => C::AddressSpace,
        "ArrayToPointerDecay" => C::ArrayDecay,
        "FunctionToPointerDecay" => C::FunctionDecay,
        "BitCast" | "NoOp" => match (from, to) {
            (T::Scalar, T::Scalar) if kind == "NoOp" => C::Scalar,
            (T::Pointer(a), T::Pointer(b)) => {
                if from_qualifiers.atomic || to_qualifiers.atomic {
                    return C::Atomic;
                }
                match (a, b) {
                    (P::Function, P::Function) => C::FunctionPointer,
                    (P::Function, _) | (_, P::Function) => C::FunctionObject,
                    _ if a == b => C::Qualifier,
                    (_, P::Void | P::Character(_)) => C::ByteView,
                    (P::Void | P::Character(_), P::Object(_)) => C::TypedView,
                    (P::Object(_), P::Object(_)) => C::PunnedView,
                }
            }
            _ => C::Unlisted,
        },
        _ if scalar_kind(kind) && *from == T::Scalar && *to == T::Scalar => C::Scalar,
        _ => C::Unlisted,
    }
}

/// The class of a cast node (`CStyleCastExpr` or `ImplicitCastExpr`) from
/// Clang's kind and both type spellings; `None` when `node` is not a cast or
/// a Clang fact is missing.
pub(crate) fn classify_node(node: &Value) -> Option<ConversionClass> {
    if !matches!(kind(node), "CStyleCastExpr" | "ImplicitCastExpr") {
        return None;
    }
    let cast = node["castKind"].as_str()?;
    let [operand] = children(node) else {
        return None;
    };
    let from = crate::typed_access_path::qual(operand)?;
    let to = crate::typed_access_path::qual(node)?;
    Some(classify(cast, from, to))
}

/// Whether the class table agrees with rule 11b's kind table: every kind a
/// class refines is a `CAST_TABLE` row or one of the two decays, and a class
/// that preserves an identity refines only rows whose provenance is kept,
/// a class that loses one only rows whose provenance is lost.
#[cfg(test)]
pub(crate) fn agrees_with_cast_table() -> bool {
    use crate::cast_representation::{Provenance, representation};
    CONVERSION_TABLE.iter().all(|row| {
        row.kinds.iter().all(|kind| {
            if matches!(*kind, "FunctionToPointerDecay") {
                return true;
            }
            let Some(kind_row) = representation(kind) else {
                return false;
            };
            match row.identity {
                Identity::Preserved if row.class != C::FunctionPointer => {
                    kind_row.provenance == Provenance::Kept
                }
                Identity::Lost => kind_row.provenance == Provenance::Lost,
                Identity::Absent | Identity::NoValue => {
                    kind_row.provenance == Provenance::NotApplicable
                }
                Identity::Data => {
                    kind_row.provenance == Provenance::NotApplicable
                        || (*kind == "NoOp" && kind_row.provenance == Provenance::Kept)
                }
                _ => true,
            }
        })
    })
}

/// The shared classification vectors (`QCore.CastSemantics.classifyVectors`):
/// `(kind, operand type, result type, class)`.
#[cfg(test)]
pub(crate) const CLASSIFY_VECTORS: [(&str, &str, &str, &str); 28] = [
    ("IntegralCast", "int", "unsigned long", "scalar"),
    ("NoOp", "int", "int", "scalar"),
    ("FloatingToIntegral", "double", "int", "scalar"),
    ("IntegralToBoolean", "int", "_Bool", "scalar"),
    ("PointerToBoolean", "char *", "_Bool", "null-test"),
    (
        "PointerToIntegral",
        "void *",
        "unsigned long",
        "pointer-to-integer",
    ),
    (
        "IntegralToPointer",
        "unsigned long",
        "char *",
        "integer-to-pointer",
    ),
    ("NullToPointer", "int", "void *", "null-pointer"),
    ("BitCast", "struct S *", "void *", "byte-view"),
    ("BitCast", "struct S *", "char *", "byte-view"),
    ("BitCast", "struct S *", "unsigned char *", "byte-view"),
    (
        "BitCast",
        "const struct S *",
        "const unsigned char *",
        "byte-view",
    ),
    ("BitCast", "void *", "struct S *", "typed-view"),
    ("BitCast", "char *", "struct S *", "typed-view"),
    ("BitCast", "unsigned char *", "int *", "typed-view"),
    ("BitCast", "struct A *", "struct B *", "punned-view"),
    ("BitCast", "int *", "long *", "punned-view"),
    ("BitCast", "void **", "char **", "punned-view"),
    ("NoOp", "struct S *", "const struct S *", "qualifier"),
    ("NoOp", "const char *", "char *", "qualifier"),
    ("BitCast", "struct S *", "volatile struct S *", "qualifier"),
    (
        "BitCast",
        "int (*)(void *)",
        "void (*)(void)",
        "function-pointer",
    ),
    ("BitCast", "int (*)(void *)", "void *", "function-object"),
    ("BitCast", "void *", "void (*)(int)", "function-object"),
    ("ToVoid", "int", "void", "discard"),
    ("ArrayToPointerDecay", "char[16]", "char *", "array-decay"),
    ("BitCast", "_Atomic(int) *", "int *", "atomic"),
    ("Dependent", "int", "int", "unlisted"),
];

/// A cast's position in its statement (the census's second axis).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Position {
    /// The base of `->` or `.`: `((T *)p)->f`.
    MemberRoot,
    /// The operand of a unary `*`.
    Dereference,
    /// The base of a subscript.
    SubscriptBase,
    /// An operand of pointer arithmetic: `(char *)q + n`.
    PointerStep,
    /// The callee of a call.
    Callee,
    /// A call's fixed actual.
    CallActual,
    /// A call's trailing actual beyond the declared parameters.
    VariadicTail,
    /// A `return` value.
    Return,
    /// The condition of `if`, a loop or `?:`.
    Condition,
    /// A declaration's initializer.
    Initializer,
    /// The value of `=`.
    AssignmentValue,
    /// Inside the destination of an assignment or increment.
    AssignmentDestination,
    /// Inside a `sizeof`/`_Alignof` operand.
    TypeQueryOperand,
    /// An expression statement (`(void) e;`).
    Statement,
    /// Any other operand (arithmetic, comparison, a nested cast).
    Operand,
}

impl Position {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Position::MemberRoot => "member-root",
            Position::Dereference => "dereference",
            Position::SubscriptBase => "subscript-base",
            Position::PointerStep => "pointer-step",
            Position::Callee => "callee",
            Position::CallActual => "call-actual",
            Position::VariadicTail => "variadic-tail",
            Position::Return => "return",
            Position::Condition => "condition",
            Position::Initializer => "initializer",
            Position::AssignmentValue => "assignment-value",
            Position::AssignmentDestination => "assignment-destination",
            Position::TypeQueryOperand => "type-query-operand",
            Position::Statement => "statement",
            Position::Operand => "operand",
        }
    }
}

/// The number of declared parameters of a callee type text and whether it is
/// variadic: `int (*)(const char *, ...)` → `(1, true)`.
fn callee_arity(text: &str) -> Option<(usize, bool)> {
    // The parameter list is the last parenthesised group, matched from its
    // closing parenthesis.
    let text = text.trim_end();
    let bytes = text.as_bytes();
    if bytes.last() != Some(&b')') {
        return None;
    }
    let mut depth = 0i32;
    let mut open = None;
    for (at, byte) in bytes.iter().enumerate().rev() {
        match byte {
            b')' => depth += 1,
            b'(' => {
                depth -= 1;
                if depth == 0 {
                    open = Some(at);
                    break;
                }
            }
            _ => {}
        }
    }
    let open = open?;
    let list = text[open + 1..text.len() - 1].trim();
    if list.is_empty() || list == "void" {
        return Some((0, false));
    }
    let mut depth = 0i32;
    let mut parameters = 1usize;
    for byte in list.bytes() {
        match byte {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b',' if depth == 0 => parameters += 1,
            _ => {}
        }
    }
    let variadic = list.ends_with("...");
    Some((parameters - usize::from(variadic), variadic))
}

/// The position of a cast from its ancestors (innermost last), the cast
/// itself not included. Parentheses and implicit conversions between the
/// cast and its consumer are looked through.
pub(crate) fn position(ancestors: &[&Value], cast: &Value) -> Position {
    let mut child = cast;
    for parent in ancestors.iter().rev() {
        let parent = *parent;
        match kind(parent) {
            "ParenExpr" | "ImplicitCastExpr" => {
                child = parent;
                continue;
            }
            "MemberExpr" => return Position::MemberRoot,
            "UnaryOperator" => {
                return match parent["opcode"].as_str() {
                    Some("*") => Position::Dereference,
                    Some("++" | "--") => Position::AssignmentDestination,
                    _ => Position::Operand,
                };
            }
            "ArraySubscriptExpr" => {
                let base = children(parent).first();
                return if base.is_some_and(|base| std::ptr::eq(base, child))
                    && crate::typed_access_path::qual(child)
                        .is_some_and(|text| matches!(type_class(text).0, TypeClass::Pointer(_)))
                {
                    Position::SubscriptBase
                } else {
                    Position::Operand
                };
            }
            "BinaryOperator" | "CompoundAssignOperator" => {
                let opcode = parent["opcode"].as_str().unwrap_or("");
                let operands = children(parent);
                let left = operands
                    .first()
                    .is_some_and(|left| std::ptr::eq(left, child));
                if opcode == "=" {
                    return if left {
                        Position::AssignmentDestination
                    } else {
                        Position::AssignmentValue
                    };
                }
                if kind(parent) == "CompoundAssignOperator" && left {
                    return Position::AssignmentDestination;
                }
                if matches!(opcode, "+" | "-" | "+=" | "-=")
                    && crate::typed_access_path::qual(parent)
                        .is_some_and(|text| matches!(type_class(text).0, TypeClass::Pointer(_)))
                {
                    return Position::PointerStep;
                }
                if matches!(opcode, "&&" | "||") {
                    // A connective's operand: its consumer decides.
                    child = parent;
                    continue;
                }
                return Position::Operand;
            }
            "CallExpr" => {
                let operands = children(parent);
                let at = operands
                    .iter()
                    .position(|operand| std::ptr::eq(operand, child))
                    .unwrap_or(0);
                if at == 0 {
                    return Position::Callee;
                }
                let arity = operands
                    .first()
                    .and_then(crate::typed_access_path::qual)
                    .and_then(callee_arity);
                return match arity {
                    Some((fixed, true)) if at > fixed => Position::VariadicTail,
                    _ => Position::CallActual,
                };
            }
            "ReturnStmt" => return Position::Return,
            "IfStmt" | "WhileStmt" | "DoStmt" | "ForStmt" | "SwitchStmt" => {
                return Position::Condition;
            }
            "ConditionalOperator" | "BinaryConditionalOperator" => {
                let first = children(parent).first();
                if first.is_some_and(|first| std::ptr::eq(first, child)) {
                    return Position::Condition;
                }
                child = parent;
                continue;
            }
            "VarDecl" => return Position::Initializer,
            "UnaryExprOrTypeTraitExpr" => return Position::TypeQueryOperand,
            "CompoundStmt" | "CaseStmt" | "DefaultStmt" | "LabelStmt" => {
                return Position::Statement;
            }
            "CStyleCastExpr" => return Position::Operand,
            _ => return Position::Operand,
        }
    }
    Position::Statement
}

/// One explicit cast of a function body, for the census.
#[derive(Clone, Debug)]
pub(crate) struct CastSite<'a> {
    pub(crate) node: &'a Value,
    pub(crate) class: ConversionClass,
    pub(crate) position: Position,
    /// The line of the innermost enclosing statement (a child of a compound
    /// statement or a control statement), the line route G's leaves carry.
    pub(crate) statement: &'a Value,
}

/// Every explicit cast (`CStyleCastExpr`) of `function`, outermost first,
/// with its class, position and enclosing statement.
pub(crate) fn explicit_casts(function: &Value) -> Vec<CastSite<'_>> {
    fn walk<'a>(
        node: &'a Value,
        ancestors: &mut Vec<&'a Value>,
        statement: Option<&'a Value>,
        out: &mut Vec<CastSite<'a>>,
    ) {
        let parent_is_block = ancestors.last().is_some_and(|parent| {
            matches!(
                kind(parent),
                "CompoundStmt"
                    | "IfStmt"
                    | "WhileStmt"
                    | "DoStmt"
                    | "ForStmt"
                    | "SwitchStmt"
                    | "CaseStmt"
                    | "DefaultStmt"
                    | "LabelStmt"
            )
        });
        let statement = if parent_is_block && kind(node) != "CompoundStmt" {
            Some(node)
        } else {
            statement
        };
        if kind(node) == "CStyleCastExpr" {
            out.push(CastSite {
                node,
                class: classify_node(node).unwrap_or(ConversionClass::Unlisted),
                position: position(ancestors, node),
                statement: statement.unwrap_or(node),
            });
        }
        ancestors.push(node);
        for child in children(node) {
            walk(child, ancestors, statement, out);
        }
        ancestors.pop();
    }
    let mut out = Vec::new();
    walk(function, &mut Vec::new(), None, &mut out);
    out
}

#[cfg(test)]
#[path = "../tests/unit/cast_semantics.rs"]
mod tests;
