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

//! Card **CG12-VOID-VIEW-READBACK** (gateway rule 11j): the view of a
//! `void *` value through an object-pointer cast, and the readback of an
//! identity stored in a `void *` member.
//!
//! Gap entry M05 (evidence Q18): a tracked pointer may be stored into a
//! `void *` member (card ALIAS-AGGREGATE's store destination), but reading it
//! back was refused, because `void *` names no pointee. This module supplies
//! the two missing facts, each read from the Clang AST only.
//!
//! ## 1. The view ([`classify_view`], [`view_position`])
//!
//! An explicit cast `(T *) E` whose Clang kind is `BitCast`, whose operand has
//! type `void *` (qualifiers `const`/`restrict` allowed) and whose type is an
//! object pointer (`T` is not `void`, a function, a union, `volatile` or
//! `_Atomic`) is a **view**: C11 6.3.2.3p1/p7 keep the pointer value, and
//! CG08's `CAST_TABLE` gives an object-pointer `BitCast` no custody effect and
//! a kept provenance. The pointee type of the place is the cast's target
//! type. Every other cast keeps its refusal: a cast from another object
//! pointer is a type pun (M22, card CG18), a cast to a function or `void`
//! pointer names no object, a `NoOp` or any other kind is not a view.
//!
//! In a typed access path ([`crate::typed_access_path::Reader::path`]) a view
//! may stand only where a pointer is read through (`*`, `->`, `[]`), is read
//! only over an **untracked** object, and is admitted in one of two positions:
//!
//! * over the **root** itself (`*(T *)a`, `((T *)a)->x`, `((T *)a)[i]`): the
//!   chain is marked `root_untracked_only`, so the flow reads the root only
//!   as never bound or bound-untracked (rule 11d) and refuses a tracked one:
//!   a tracked identity's object has the type its contract gives it, and a
//!   view of it would be a type claim this route cannot check;
//! * over a pointer **loaded** from a place of a root the custody closure
//!   proves never bound (`*(T *)f->p`): under rule 10k's premise every pointer
//!   loaded there is outside the tracked graph.
//!
//! A view of a root as `struct TAG *` where the body declares another name of
//! that record-pointer class is refused too: C02-P-ALIAS would relate that
//! name to a `struct TAG *` use, and the `void *` root has no class to carry
//! the obligation.
//!
//! A view over a pointer loaded from any other root (`((T *)s->v)->x` on a
//! tracked `s`) is refused (`view-over-link`): the loaded pointer is a field
//! link whose stored type this position does not check. That is the tracked
//! readback in place position, a named residual.
//!
//! ## 2. The readback ([`decide_readback`], the flow in `legacy_void_view.rs`)
//!
//! A pointer destination `T *c = E;` or `c = E;` whose value is a read of a
//! `void *` member `R->F`, directly, through the implicit `void *` bit-cast of
//! rule 10m, or through an explicit view to the destination's type, is a
//! **readback**. Where every premise below holds, the destination is bound to
//! the identity the member holds, instead of the target-free borrowed alias
//! with no identity every other pointer read gets (P2):
//!
//! 1. **Store base.** `R` is an admitted aggregate store base
//!    ([`crate::aggregate_store::admitted_field`]) and `F` is its `void *`
//!    member, so its stores are ALIAS-AGGREGATE's field-link stores.
//! 2. **Types agree.** Every stored value whose original type is known (a
//!    pointer to `U` converted to `void *`) and every view the body reads the
//!    member back as name one pointee type. A known stored type that differs
//!    from a view, or two different views, is **type punning** (M22): the
//!    statement is refused.
//! 3. **The owner stays in the body.** Every reference to `R` in the function
//!    is a member access (`R->G`, never `&R->G`), its own assignment, a null
//!    test, or a whole argument at a `Released`/`Consumed` position of a
//!    contracted call. No callee, alias or cast can reach `R->F`.
//! 4. **The member is only stored and read.** Every occurrence of `R->F` is
//!    the destination of a simple `=` or a read (`LValueToRValue`): never an
//!    address, an increment or a compound assignment.
//! 5. **Flow** (at the node, `bound_read_resolves`, O04-READ-BOUND): the owner
//!    is one live identity this body acquired, the member's binding is exactly
//!    the field link an admitted store recorded, on every path (no partial
//!    link, rule 11c), and the linked child is live with no unresolved effect.
//!    A refused statement or an uncontracted call on any reaching path makes
//!    the binding unknown; a later admitted store replaces the link. So the
//!    value read is the identity of the last store, or the read has no
//!    identity.
//!
//! Where premise 1, 3, 4 or 5 fails, nothing changes: the destination is the
//! target-free borrowed alias it was, never a guessed identity.
//!
//! Premises stated, not proved: faithful Clang extraction of cast kinds,
//! types, declaration and field ids (trust boundary 28), C11 6.3.2.3p7 for
//! the round trip through `void *`, and **P-CG12-VIEW-EFFECTIVE-TYPE**: an
//! access through a view of a root or of a never-bound load reads the object
//! at its effective type (C11 6.5p7). The view is checked against the stored
//! type only where this route itself knows it (premise 2).
//!
//! The Lean model is `QCore.VoidViewReadback`; its vectors are
//! [`VIEW_VECTORS`], [`POSITION_VECTORS`], [`DECISION_VECTORS`] and the
//! gateway-run [`EVENT_VECTORS`].

use std::collections::BTreeSet;

use serde_json::Value;

use qcore_schemas::contract::{ResourceContracts, ResourceParameterMode};

use crate::typed_access_path::{is_pointer_type, qual};
use crate::typed_c_ast::{children, kind};
use crate::typed_const_pointer_value::top_level_qualifiers;

// ---------------------------------------------------------------------------
// 1. The view
// ---------------------------------------------------------------------------

/// The class of a cast's operand type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OperandClass {
    /// `void *`, `const void *`, `void *restrict`.
    VoidPointer,
    /// A pointer to an object type.
    ObjectPointer,
    /// Anything else (a scalar, a function pointer, a qualified `void *`).
    Other,
}

/// The class of a cast's target type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TargetClass {
    /// A pointer to an object type: the view's pointee is that type.
    Object,
    /// `void *` again: no view.
    VoidPointer,
    /// A pointer to a function.
    Function,
    Other,
}

/// The class of a cast's Clang kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CastClass {
    BitCast,
    NoOp,
    Other,
}

/// Why a cast is not a view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ViewRefusal {
    /// Not an object-pointer `BitCast`.
    Kind,
    /// The operand is not `void *`: a cast between object pointers is a type
    /// pun (M22, card CG18).
    Operand,
    /// The target is not an object pointer.
    Target,
    /// A Clang fact is missing.
    Missing,
}

impl ViewRefusal {
    #[cfg(test)]
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Kind => "kind",
            Self::Operand => "operand",
            Self::Target => "target",
            Self::Missing => "missing",
        }
    }
}

/// The view decision: a pure function of the three classes.
/// `QCore.VoidViewReadback.viewOf` is the same function.
pub(crate) fn view_of(
    cast: CastClass,
    operand: OperandClass,
    target: TargetClass,
) -> Result<(), ViewRefusal> {
    match (cast, operand, target) {
        (CastClass::BitCast, OperandClass::VoidPointer, TargetClass::Object) => Ok(()),
        (CastClass::BitCast, OperandClass::VoidPointer, _) => Err(ViewRefusal::Target),
        (CastClass::BitCast, _, _) => Err(ViewRefusal::Operand),
        _ => Err(ViewRefusal::Kind),
    }
}

/// The pointee words of a pointer type with every `const`/`restrict`
/// removed, when the text is a pointer the positive test admits and its own
/// qualifiers are only `const`/`restrict`.
fn pointee_words(text: &str) -> Option<Vec<&str>> {
    let (unqualified, qualifiers) = top_level_qualifiers(text)?;
    if qualifiers
        .iter()
        .any(|word| !matches!(*word, "const" | "restrict" | "__restrict"))
        || !is_pointer_type(unqualified)
    {
        return None;
    }
    let inner = &unqualified[..unqualified.len() - 1];
    let words = inner
        .split_whitespace()
        .filter(|word| !matches!(*word, "const" | "restrict" | "__restrict"))
        .collect::<Vec<_>>();
    (!words.is_empty()).then_some(words)
}

/// The operand class of a type's text.
pub(crate) fn operand_class(text: &str) -> OperandClass {
    match pointee_words(text) {
        Some(words) if words == ["void"] => OperandClass::VoidPointer,
        Some(_) => OperandClass::ObjectPointer,
        None => OperandClass::Other,
    }
}

/// The target class of a type's text.
pub(crate) fn target_class(text: &str) -> TargetClass {
    if text.contains('(') && text.contains('*') {
        return TargetClass::Function;
    }
    match pointee_words(text) {
        Some(words) if words == ["void"] => TargetClass::VoidPointer,
        Some(_) => TargetClass::Object,
        None => TargetClass::Other,
    }
}

fn cast_class(kind: &str) -> CastClass {
    match kind {
        "BitCast" => CastClass::BitCast,
        "NoOp" => CastClass::NoOp,
        _ => CastClass::Other,
    }
}

/// The pointee of a pointer type, qualifiers removed, as one comparable key
/// (`struct T`, `char *`). `None` for `void` or a non-pointer.
pub(crate) fn pointee_key(text: &str) -> Option<String> {
    let words = pointee_words(text)?;
    (words != ["void"]).then(|| words.join(" "))
}

/// Classify the cast `node` (a `CStyleCastExpr` or `ImplicitCastExpr`).
pub(crate) fn classify_view(node: &Value) -> Result<&Value, ViewRefusal> {
    if !matches!(kind(node), "CStyleCastExpr" | "ImplicitCastExpr") {
        return Err(ViewRefusal::Kind);
    }
    let cast = cast_class(node["castKind"].as_str().ok_or(ViewRefusal::Missing)?);
    let [operand] = children(node) else {
        return Err(ViewRefusal::Missing);
    };
    let operand_type = qual(operand).ok_or(ViewRefusal::Missing)?;
    let target_type = qual(node).ok_or(ViewRefusal::Missing)?;
    view_of(cast, operand_class(operand_type), target_class(target_type))?;
    Ok(operand)
}

/// An explicit view in a path: the operand of a `CStyleCastExpr` view, or
/// `None` when `node` (parentheses peeled) is not one.
pub(crate) fn explicit_view(mut node: &Value) -> Option<Result<&Value, ViewRefusal>> {
    while kind(node) == "ParenExpr" {
        node = children(node).first()?;
    }
    // Card CAST-FORMALIZATION (rule 12h): which conversion is a view is the
    // conversion table's decision (`cast_semantics::access_view`), of which
    // `classify_view`'s `void *` row is one case.
    (kind(node) == "CStyleCastExpr").then(|| crate::cast_semantics::access_view(node))
}

/// A pointer destination's value that is an explicit view to the
/// destination's own pointee type (`T *c = (T *) E;`, a qualification
/// conversion allowed on top): the operand `E` as the path reader reads it
/// (`strip`), or `None`. Rule 10m's implicit conversion is
/// `pointer_source`'s; this is its explicit spelling.
pub(crate) fn explicit_source<'a>(value: &'a Value, destination: &Value) -> Option<&'a Value> {
    let mut node = value;
    while kind(node) == "ParenExpr" || is_cast(node, &["NoOp"]) {
        node = children(node).first()?;
    }
    if kind(node) != "CStyleCastExpr" {
        return None;
    }
    let operand = classify_view(node).ok()?;
    let destination = crate::typed_access_path::strip(destination).unwrap_or(destination);
    let wanted = pointee_key(qual(destination)?)?;
    if pointee_key(qual(node)?)? != wanted {
        return None;
    }
    crate::typed_access_path::strip(operand)
}

/// The readback candidate a pointer destination's source names: exactly
/// `R->F` with `R` a declared object and `F` a member of type `void *`.
pub(crate) fn readback_of(source: &Value) -> Option<Readback> {
    let member = crate::typed_access_path::strip(source)?;
    if kind(member) != "MemberExpr"
        || member["isArrow"].as_bool() != Some(true)
        || operand_class(qual(member)?) != OperandClass::VoidPointer
    {
        return None;
    }
    let base = crate::typed_access_path::strip(children(member).first()?)?;
    if kind(base) != "DeclRefExpr"
        || !matches!(
            base["referencedDecl"]["kind"].as_str(),
            Some("VarDecl" | "ParmVarDecl")
        )
    {
        return None;
    }
    Some(Readback {
        root: base["referencedDecl"]["name"].as_str()?.to_owned(),
        root_decl: base["referencedDecl"]["id"].as_str()?.to_owned(),
        member: member["name"].as_str()?.to_owned(),
        field: member["referencedMemberDecl"].as_str()?.to_owned(),
    })
}

/// The destination of `c = E` is an automatic local of `function` (not a
/// formal, a `static` or a file-scope object, whose slot a binding of this
/// frame would not describe).
pub(crate) fn local_destination(lhs: &Value, function: Option<&Value>) -> bool {
    let Some(name) = crate::typed_access_path::strip(lhs) else {
        return false;
    };
    let Some(id) = name["referencedDecl"]["id"].as_str() else {
        return false;
    };
    if kind(name) != "DeclRefExpr" || name["referencedDecl"]["kind"].as_str() != Some("VarDecl") {
        return false;
    }
    let mut found = false;
    let mut parents = Vec::new();
    if let Some(function) = function {
        walk(function, &mut parents, &mut |node, _| {
            if kind(node) == "VarDecl" && node["id"].as_str() == Some(id) {
                found = local_declaration(node);
            }
        });
    }
    found
}

/// The declarator of `T *c = E;` is an automatic local.
pub(crate) fn local_declaration(decl: &Value) -> bool {
    kind(decl) == "VarDecl" && decl["storageClass"].is_null()
}

/// Whether `function` declares a name (other than the root `root_decl`) whose
/// pointee is the record `key` (`struct TAG`): C02-P-ALIAS relates such a
/// name to a `struct TAG *` use, and a view of a `void *` root as `struct TAG
/// *` has no declared class to carry that obligation, so the reader refuses
/// it there. Any other key names no may-alias class.
pub(crate) fn view_has_alias_peers(
    function: Option<&Value>,
    root_decl: Option<&str>,
    key: &str,
) -> bool {
    if !key.starts_with("struct ") {
        return false;
    }
    let Some(function) = function else {
        return true;
    };
    let mut found = false;
    let mut parents = Vec::new();
    walk(function, &mut parents, &mut |node, _| {
        if matches!(kind(node), "VarDecl" | "ParmVarDecl")
            && node["id"].as_str() != root_decl
            && qual(node).and_then(pointee_key).as_deref() == Some(key)
        {
            found = true;
        }
    });
    found
}

/// Where a view stands in a path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ViewPosition {
    /// Over the root pointer itself.
    Root,
    /// Over a pointer loaded from a place of the root.
    Load,
}

/// The position decision: a view is read only over an untracked object. A
/// view over the root is admitted where the root carries no tracked identity
/// (never bound, or rule 11d's bound-untracked premise at the node); a view
/// over a loaded pointer only when the root is never bound. The reader asks
/// it with `root_tracked` false and marks the chain `root_untracked_only`, so
/// the flow supplies the tracked half. `QCore.VoidViewReadback.positionAdmits`
/// is the same function.
pub(crate) fn view_position(
    position: ViewPosition,
    root_never_bound: bool,
    root_tracked: bool,
) -> bool {
    !root_tracked
        && match position {
            ViewPosition::Root => true,
            ViewPosition::Load => root_never_bound,
        }
}

/// The view vectors shared with `proof/QCore/VoidViewReadback.lean`
/// (`viewVectors`): the operand type, the cast kind, the target type as
/// Clang prints them, and the decision. The Rust test binds each row's
/// classes to [`view_of`] and a C spelling of each to Clang's reading.
#[cfg(test)]
pub(crate) const VIEW_VECTORS: &[(&str, &str, &str, &str)] = &[
    ("void *", "BitCast", "struct T *", "view"),
    ("const void *", "BitCast", "const char **", "view"),
    ("const void *", "BitCast", "const struct T *", "view"),
    ("void *", "BitCast", "int *", "view"),
    ("int *", "BitCast", "char *", "operand"),
    ("struct T *", "BitCast", "struct A *", "operand"),
    ("void *", "BitCast", "void (*)(void)", "target"),
    ("void *", "NoOp", "const void *", "kind"),
    ("volatile void *", "BitCast", "int *", "operand"),
    ("void *", "BitCast", "volatile int *", "target"),
    ("long", "IntegralToPointer", "int *", "kind"),
];

/// The position vectors (`positionVectors`): a path spelling, the view's
/// position, whether the root is never bound, whether it carries a tracked
/// identity at the access, and whether it is admitted. The Rust test runs
/// each through the gateway (`a`: a `void *` formal; `f`: a `struct S *`
/// formal; `o`: a local bound to an untracked copy, or to `q_alloc()` where
/// tracked; `h`: a tracked handle; `s`: a store base holding a tracked child).
#[cfg(test)]
pub(crate) const POSITION_VECTORS: &[(&str, &str, bool, bool, bool)] = &[
    ("*(int *)a", "root", true, false, true),
    ("((struct T *)a)->x", "root", true, false, true),
    ("((char *)a)[1]", "root", true, false, true),
    ("((struct T *)o)->x", "root", false, false, true),
    ("((struct T *)o)->x", "root", false, true, false),
    ("*(int *)h", "root", false, true, false),
    ("*(int *)f->p", "load", true, false, true),
    ("((struct T *)s->v)->x", "load", false, true, false),
];

// ---------------------------------------------------------------------------
// 2. The readback
// ---------------------------------------------------------------------------

/// A readback candidate the typed reader found at a pointer destination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Readback {
    /// The owner's name and Clang declaration id.
    pub(crate) root: String,
    pub(crate) root_decl: String,
    /// The `void *` member's name and `FieldDecl` id.
    pub(crate) member: String,
    pub(crate) field: String,
}

impl Readback {
    /// The canonical place an admitted store binds, `R->F`.
    pub(crate) fn place(&self) -> String {
        format!("{}->{}", self.root, self.member)
    }
}

/// The static readback decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadbackDecision {
    /// Every static premise holds: the flow decides the identity.
    Identity,
    /// Premise 1 fails: no admitted store names the member.
    NoStoreBase,
    /// Premise 2 fails: type punning. The statement is refused.
    Pun,
    /// Premise 3 fails: the owner reaches something that could rewrite it.
    OwnerEscapes,
    /// Premise 4 fails: the member is used otherwise than stored or read.
    MemberEscapes,
}

impl ReadbackDecision {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::NoStoreBase => "no-store-base",
            Self::Pun => "pun",
            Self::OwnerEscapes => "owner-escapes",
            Self::MemberEscapes => "member-escapes",
        }
    }
}

/// The static decision. `QCore.VoidViewReadback.decideReadback` is the same
/// function.
pub(crate) fn decide_readback(
    store_base: bool,
    types_agree: bool,
    owner_escapes: bool,
    member_escapes: bool,
) -> ReadbackDecision {
    if !store_base {
        ReadbackDecision::NoStoreBase
    } else if !types_agree {
        ReadbackDecision::Pun
    } else if owner_escapes {
        ReadbackDecision::OwnerEscapes
    } else if member_escapes {
        ReadbackDecision::MemberEscapes
    } else {
        ReadbackDecision::Identity
    }
}

/// The decision vectors (`decisionVectors`): a C body (with `struct S *s`
/// the owner and `v` its `void *` member), the four premises and the
/// decision. The Rust test binds each body's premises to the AST reading
/// and its verdict to the gateway's.
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(&str, bool, bool, bool, bool, &str)] = &[
    (
        "struct S *s = q_alloc(); struct T *t = q_alloc(); s->v = t; struct T *c = s->v; q_free(c); q_free(s);",
        true,
        true,
        false,
        false,
        "identity",
    ),
    (
        "struct S *s = q_alloc(); struct T *t = q_alloc(); s->v = t; struct T *c = (struct T *) s->v; q_free(c); q_free(s);",
        true,
        true,
        false,
        false,
        "identity",
    ),
    (
        "struct S *s = q_alloc(); s->v = q_alloc(); char *c = s->v; q_free(c); q_free(s);",
        true,
        true,
        false,
        false,
        "identity",
    ),
    (
        "struct S *s = q_alloc(); struct T *t = q_alloc(); s->v = t; struct A *c = (struct A *) s->v; q_free(c); q_free(s);",
        true,
        false,
        false,
        false,
        "pun",
    ),
    (
        "struct S *s = q_alloc(); struct T *t = q_alloc(); s->v = t; char *c = s->v; q_free(c); q_free(s);",
        true,
        false,
        false,
        false,
        "pun",
    ),
    (
        "struct S *s = q_alloc(); struct T *t = q_alloc(); s->v = t; q_observe(s); struct T *c = s->v; q_free(c); q_free(s);",
        true,
        true,
        true,
        false,
        "owner-escapes",
    ),
    (
        "struct S *s = q_alloc(); struct T *t = q_alloc(); s->v = t; void **w = &s->v; struct T *c = s->v; q_free(c); q_free(s);",
        true,
        true,
        false,
        true,
        "member-escapes",
    ),
];

/// What the function body does with one owner and its member, read from the
/// Clang AST.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct BodyFacts {
    /// The known pointee types stored into the member and read back as.
    pub(crate) types: BTreeSet<String>,
    /// Premise 3 fails.
    pub(crate) owner_escapes: bool,
    /// Premise 4 fails.
    pub(crate) member_escapes: bool,
}

impl BodyFacts {
    pub(crate) fn types_agree(&self) -> bool {
        self.types.len() <= 1
    }
}

/// Walk `node` with its ancestors (nearest last).
fn walk<'a>(
    node: &'a Value,
    parents: &mut Vec<&'a Value>,
    f: &mut impl FnMut(&'a Value, &[&'a Value]),
) {
    f(node, parents);
    parents.push(node);
    for child in children(node) {
        walk(child, parents, f);
    }
    parents.pop();
}

fn is_ref_to(node: &Value, decl: &str) -> bool {
    kind(node) == "DeclRefExpr" && node["referencedDecl"]["id"].as_str() == Some(decl)
}

/// The nearest ancestor that is not a parenthesis, with its index.
fn enclosing<'a>(parents: &[&'a Value], mut from: usize) -> Option<(usize, &'a Value)> {
    while from > 0 {
        from -= 1;
        if kind(parents[from]) != "ParenExpr" {
            return Some((from, parents[from]));
        }
    }
    None
}

fn is_cast(node: &Value, kinds: &[&str]) -> bool {
    kind(node) == "ImplicitCastExpr"
        && node["castKind"]
            .as_str()
            .is_some_and(|k| kinds.contains(&k))
}

/// The known pointee type of a value stored into a `void *` member: the
/// value under its conversions to `void *`, when that is an object pointer.
fn stored_type(mut value: &Value) -> Option<String> {
    loop {
        match kind(value) {
            "ParenExpr" => value = children(value).first()?,
            "ImplicitCastExpr" | "CStyleCastExpr"
                if matches!(value["castKind"].as_str(), Some("BitCast" | "NoOp"))
                    && qual(value)
                        .is_some_and(|t| operand_class(t) == OperandClass::VoidPointer) =>
            {
                value = children(value).first()?;
            }
            _ => break,
        }
    }
    pointee_key(qual(value)?)
}

/// The view a member read is converted to: the first cast above the read
/// that is not a qualification conversion.
fn read_view(parents: &[&Value], load: usize) -> Option<String> {
    let mut at = load;
    while let Some((index, parent)) = enclosing(parents, at) {
        match kind(parent) {
            "ImplicitCastExpr" | "CStyleCastExpr" => match parent["castKind"].as_str() {
                Some("NoOp") => at = index,
                Some("BitCast") => return pointee_key(qual(parent)?),
                _ => return None,
            },
            _ => return None,
        }
    }
    None
}

/// Whether the call `call` passes its argument `argument` (the whole of it,
/// under parentheses and pointer conversions) at a position that releases
/// or consumes it, through a direct callee with a contract.
fn released_argument(call: &Value, argument: usize, contracts: &ResourceContracts) -> bool {
    let Some(callee) = children(call).first() else {
        return false;
    };
    let mut callee = callee;
    while matches!(kind(callee), "ParenExpr" | "ImplicitCastExpr") {
        let Some(inner) = children(callee).first() else {
            return false;
        };
        callee = inner;
    }
    if kind(callee) != "DeclRefExpr"
        || callee["referencedDecl"]["kind"].as_str() != Some("FunctionDecl")
    {
        return false;
    }
    let Some(name) = callee["referencedDecl"]["name"].as_str() else {
        return false;
    };
    contracts.contract(name).is_some_and(|contract| {
        matches!(
            contract.parameters.get(argument),
            Some(ResourceParameterMode::Released { .. } | ResourceParameterMode::Consumed { .. })
        )
    })
}

/// Premises 2–4 over the function's AST for the owner `root_decl` and the
/// member `field`.
pub(crate) fn body_facts(
    function: &Value,
    root_decl: &str,
    field: &str,
    contracts: &ResourceContracts,
) -> BodyFacts {
    let mut facts = BodyFacts::default();
    let mut parents = Vec::new();
    walk(function, &mut parents, &mut |node, parents| {
        if is_ref_to(node, root_decl) {
            if !owner_use_admitted(node, parents, contracts) {
                facts.owner_escapes = true;
            }
            return;
        }
        if kind(node) != "MemberExpr" || node["referencedMemberDecl"].as_str() != Some(field) {
            return;
        }
        let Some(base) = children(node).first() else {
            facts.member_escapes = true;
            return;
        };
        let mut base = base;
        while matches!(kind(base), "ParenExpr") || is_cast(base, &["LValueToRValue", "NoOp"]) {
            let Some(inner) = children(base).first() else {
                break;
            };
            base = inner;
        }
        if !is_ref_to(base, root_decl) {
            // The same member of another object: not this owner's.
            return;
        }
        let here = parents.len();
        match enclosing(parents, here) {
            Some((_, parent))
                if kind(parent) == "BinaryOperator"
                    && parent["opcode"].as_str() == Some("=")
                    && children(parent)
                        .first()
                        .is_some_and(|lhs| std::ptr::eq(strip_parens(lhs), node)) =>
            {
                if let Some(ty) = children(parent).get(1).and_then(stored_type) {
                    facts.types.insert(ty);
                }
            }
            Some((index, parent)) if is_cast(parent, &["LValueToRValue"]) => {
                if let Some(ty) = read_view(parents, index) {
                    facts.types.insert(ty);
                }
            }
            _ => facts.member_escapes = true,
        }
    });
    facts
}

fn strip_parens(mut node: &Value) -> &Value {
    while kind(node) == "ParenExpr" {
        match children(node).first() {
            Some(inner) => node = inner,
            None => break,
        }
    }
    node
}

/// Premise 3 for one reference to the owner.
fn owner_use_admitted(node: &Value, parents: &[&Value], contracts: &ResourceContracts) -> bool {
    let here = parents.len();
    let Some((index, parent)) = enclosing(parents, here) else {
        return false;
    };
    // Its own assignment: `R = …`.
    if kind(parent) == "BinaryOperator"
        && parent["opcode"].as_str() == Some("=")
        && children(parent)
            .first()
            .is_some_and(|lhs| std::ptr::eq(strip_parens(lhs), node))
    {
        return true;
    }
    if !is_cast(parent, &["LValueToRValue"]) {
        return false;
    }
    // The value of `R`, above its load and any pointer conversion.
    let mut at = index;
    let mut value = parent;
    while let Some((next, above)) = enclosing(parents, at) {
        if is_cast(above, &["NoOp", "BitCast"]) {
            at = next;
            value = above;
            continue;
        }
        match kind(above) {
            // A member access: `R->G`, never its address.
            "MemberExpr" if above["isArrow"].as_bool() == Some(true) => {
                return !enclosing(parents, next).is_some_and(|(_, over)| {
                    kind(over) == "UnaryOperator" && over["opcode"].as_str() == Some("&")
                });
            }
            // A null test.
            "ImplicitCastExpr" if above["castKind"].as_str() == Some("PointerToBoolean") => {
                return true;
            }
            "BinaryOperator" if matches!(above["opcode"].as_str(), Some("==" | "!=")) => {
                return children(above)
                    .iter()
                    .any(|side| !std::ptr::eq(strip_parens(side), value) && is_null(side));
            }
            "UnaryOperator" if above["opcode"].as_str() == Some("!") => return true,
            // A whole argument that releases or consumes the owner.
            "CallExpr" => {
                let Some(position) = children(above)
                    .iter()
                    .position(|child| std::ptr::eq(strip_parens(child), value))
                else {
                    return false;
                };
                return position > 0 && released_argument(above, position - 1, contracts);
            }
            _ => return false,
        }
    }
    false
}

fn is_null(node: &Value) -> bool {
    let mut node = node;
    loop {
        match kind(node) {
            "ParenExpr" | "ImplicitCastExpr" | "CStyleCastExpr" => match children(node).first() {
                Some(inner) => node = inner,
                None => return false,
            },
            "IntegerLiteral" => return node["value"].as_str() == Some("0"),
            "GNUNullExpr" => return true,
            _ => return false,
        }
    }
}

/// The flow's answer for one admitted readback: the identity the member
/// holds, or none. `QCore.VoidViewReadback.readback` is the event model it
/// implements; `resolves` is `bound_read_resolves` at the node.
pub(crate) fn flow_binds(decision: ReadbackDecision, resolves: bool) -> bool {
    decision == ReadbackDecision::Identity && resolves
}

/// The event vectors (`eventVectors`): the statements between the
/// acquisitions `struct S *s = q_alloc(); struct T *t = q_alloc(); struct T
/// *u = q_alloc();` and the readback `struct T *c = s->v; q_free(c);
/// q_free(s);` of `void subject(int n)`, and whether the readback yields the
/// stored identity. The Rust test runs each through the gateway: a `true` row
/// is `Safe`, a `false` row never is.
#[cfg(test)]
pub(crate) const EVENT_VECTORS: &[(&str, bool)] = &[
    ("s->v = t; q_free(u);", true),
    ("s->v = t; s->v = u; q_free(t);", true),
    ("s->v = t; q_free(u); opaque();", false),
    ("s->v = t; q_free(t); q_free(u);", false),
    ("if (n) s->v = t; else q_free(t); q_free(u);", false),
    ("q_free(t); q_free(u);", false),
];

#[cfg(test)]
#[path = "../tests/unit/void_view.rs"]
mod tests;
