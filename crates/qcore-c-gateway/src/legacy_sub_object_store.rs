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

//! Card **W45-SUB-OBJECT-STORES**: stores into a sub-object through a
//! resolved, typed base.
//!
//! # What was refused
//!
//! The lexical indirect-access gate refuses every assignment whose destination
//! is a sub-object: a named `p->f`, `s.f` or `a[i]` destination, and, since
//! gateway constraint 5i (card W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE), every
//! left-hand side ending in a place the destination reader does not name
//! (`(p)->f`, `(&s)->f`, `a[i].f`). Card W43-INDIRECT-ACCESS-GENERAL's typed
//! route re-admits `p->f = e` and `s.f = e` when both sides are scalar and the
//! statement carries no call; every other sub-object store kept the refusal.
//!
//! # The rule
//!
//! One statement `DEST = VALUE` (a simple `=`, no call other than a whole
//! `VALUE`, no brace) whose gate refusal is the only reason it was refused,
//! and which no older typed route admits, is read here. Both sides must
//! resolve through declaration facts the analyzer's own reader committed to.
//!
//! **Destination** ([`read_destination`]), one selection of a member `F` of a
//! complete, unambiguous, non-`union` `struct T`:
//!
//! * `R -> F` or `( R ) -> F`, with `R` declared `T *` — a store through the
//!   pointer `R`, which keeps W43's dereference obligation;
//! * `R [ I ] . F` or `( R ) [ I ] . F`, with `R` declared `T *` (or a formal
//!   `T R [ N ]`) and `I` a scalar-only index that writes nothing — a store
//!   through `R` into the element `I`, with the same obligation (bounds are
//!   not decided);
//! * `S . F`, `( S ) . F`, `( & S ) -> F` or `A [ I ] . F`, with `S` declared
//!   `T` and `A` a local `T A [ N ]` — a store into this function's own
//!   record, which dereferences nothing.
//!
//! `T` resolves to one `struct` tag through the unit's typedefs, including a
//! typedef that defines its own `struct TAG { … }` ([`RecordRoots`], a side
//! reading that changes no declared type), and every declaration of the root
//! must agree. `F` must be a scalar member or a member holding exactly one
//! pointer (`single_pointer_field`, `void_pointer_field`,
//! `record_pointer_field`) of a complete, unambiguous, non-`union` record.
//! Casts, `(*R)`, chains, nested aggregates, typedefs the reader will not
//! resolve and undeclared roots all keep the refusal.
//!
//! **Value** ([`read_value`]):
//!
//! * into a scalar member, a scalar expression (W43's walker: literals,
//!   declared scalars, enumeration constants, scalar casts and typed scalar
//!   accesses, whose bases carry their own obligation);
//! * into a pointer member, the null constant, a string literal, a declared
//!   name (optionally cast) that is **not a formal parameter**, or one whole
//!   call. A formal's pointer is the caller's object: storing it links that
//!   object into a record, which a derived summary would misread as a borrow
//!   (`cfl_list_add`, `mk_list_init`), so it keeps the refusal.
//!
//! # The effect ([`super::transfer`], `SubObjectStore` arm)
//!
//! Every premise below is decided on the state at the node. When one fails the
//! node is exactly its **fallback**, the gate's own refusal: it reports the
//! refusal, keeps its write frame and, through card W44-LEAK-PIN-AUDIT's
//! refused hand-on rule, withdraws the leak claim of every identity it hands
//! on. So a store that stays refused still hands on.
//!
//! 1. The dereferenced base resolves exactly as W43's obligation requires
//!    (unbound, non-null untracked, or one exactly-live identity). A store of a
//!    pointer additionally requires the base to carry no tracked target: a
//!    member of a tracked aggregate may hold a field link, which only card
//!    O04-B08's aggregate store may rewrite.
//! 2. A scalar, null or literal value is custody-neutral. The member's place
//!    binding, if the projection holds one, is forgotten: its old fact no
//!    longer describes the member.
//! 3. A named or call-result pointer value is admitted only when it stores no
//!    identity this function tracks: no binding on any reaching path, or a
//!    binding with no target that is not unknown. **Every tracked identity —
//!    owned, derived, a caller's site, a descriptor — keeps the fallback.**
//!    Storing an owned identity into a record the caller reaches does move
//!    custody to the caller, but no summary mode states it: a derived summary
//!    would drop the obligation the caller now holds, so a caller's `Safe`
//!    would rest on a transfer it cannot see. That is the decision O04-B08
//!    (`a_store_into_a_borrowed_in_aggregate_is_never_admitted`) and
//!    W44-LEAK-PIN-AUDIT (`a_refused_store_of_the_identity_withdraws_the_leak`)
//!    pin, and this rule keeps it: the refusal hands the identity on, so no
//!    leak is claimed and the function stays `Unknown`.
//!
//! A call result is applied first (its own obligations and diagnostics are
//! the call's), bound to a temporary, decided as a named value and then
//! forgotten. The classifier admits a call value only when its contract in
//! force states a `None` result and borrows every parameter; every other
//! call keeps the refusal.
//!
//! # Composition
//!
//! * **Constraint 5i.** The destination is modelled as the sub-object it is;
//!   no name but the member's own place spelling (`R->F`, `S.F`) is ever
//!   written, and a `recs[i].name` store binds no local `name`.
//! * **Descriptors** are custody, never data: the descriptor guard
//!   (`admits_descriptor_read`) refuses this instruction whenever the
//!   statement reads a tracked descriptor, so such a store keeps the refusal.
//! * **Publication.** A root with static storage duration is not declared in
//!   the function, so it never resolves here; CTWIN-PUBLISH's refusal stands.

use crate::typed_field_access::{
    FunctionTypes, TypedAccessModel, is_name, is_scalar_value_expression, typed_scalar_value,
};

/// Whether the stored member holds a scalar or exactly one pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MemberKind {
    Scalar,
    Pointer,
}

/// The destination of one admitted sub-object store.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Destination {
    /// The pointer the store dereferences (`R` of `R->F` and `R[I].F`);
    /// `None` for a store into this function's own record object.
    pub base: Option<String>,
    /// The member's place spelling, the only place the store may write.
    pub place: String,
    pub member: MemberKind,
}

/// The stored value, as the classifier read it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StoredValue {
    /// A scalar expression; the bases its typed accesses dereference carry
    /// W43's obligation.
    Scalar { bases: Vec<String> },
    /// The null constant or a string literal: no identity at all.
    Untracked,
    /// A declared name, decided on its binding.
    Name(String),
    /// One whole contracted call with a `None` result and every parameter
    /// `Borrowed`, applied first, whose result is bound to
    /// [`STORE_TEMPORARY`] and then decided as a name.
    Call(Box<super::LegacyInstruction>),
}

/// The temporary a stored call result is bound to between the call and the
/// store. Never a C identifier (a leading double underscore is reserved), and
/// forgotten after the store.
pub const STORE_TEMPORARY: &str = "__qcore_sub_object_value__";

/// Why [`read_destination`] or [`read_value`] refused, for the census.
pub type Refusal = &'static str;

fn close_of(texts: &[&str], open: usize, left: &str, right: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(open) {
        if *text == left {
            depth += 1;
        } else if *text == right {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

/// How a record-typed root is declared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootKind {
    /// `T * name`, or a formal spelled `T name [ N ]` (C11 6.7.6.3p7): a
    /// pointer the store dereferences.
    Pointer,
    /// `T name`: this function's own record object.
    Object,
    /// A local `T name [ N ]`: this function's own array of records.
    Array,
}

/// Card **W45-SUB-OBJECT-STORES**: the record-typed roots of one function —
/// every name each of whose declarations (formals and locals, read by the
/// same total-or-reject declarator reader as [`FunctionTypes::read`]) is
/// `T name`, `T * name` or `T name [ N ]` with `T` resolving to one `struct`
/// tag ([`TypedAccessModel::record_tag`], which also reads a typedef that
/// defines its own `struct TAG { … }`). Two disagreeing declarations of one
/// name make it unusable. Empty when the reader refuses the function.
#[derive(Clone, Debug, Default)]
pub struct RecordRoots {
    roots: std::collections::BTreeMap<String, Option<(String, RootKind)>>,
}

impl RecordRoots {
    pub fn read(
        model: &TypedAccessModel,
        parameters: &[&str],
        body: &[&str],
        formals: &std::collections::BTreeSet<String>,
    ) -> Self {
        let Some(facts) = crate::typed_field_access::declarator_facts(parameters, body) else {
            return Self::default();
        };
        let mut roots = std::collections::BTreeMap::new();
        for fact in facts {
            let specifiers = fact
                .specifiers
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            let formal = formals.contains(&fact.name);
            let kind = match (fact.stars, fact.dimensions) {
                (1, 0) => Some(RootKind::Pointer),
                (0, 0) => Some(RootKind::Object),
                (0, 1) if formal => Some(RootKind::Pointer),
                (0, 1) => Some(RootKind::Array),
                _ => None,
            };
            let reading = kind.and_then(|kind| Some((model.record_tag(&specifiers)?, kind)));
            match roots.get(&fact.name) {
                Some(existing) if *existing != reading => {
                    roots.insert(fact.name, None);
                }
                Some(_) => {}
                None => {
                    roots.insert(fact.name, reading);
                }
            }
        }
        Self { roots }
    }

    fn root(&self, name: &str) -> Option<&(String, RootKind)> {
        self.roots.get(name)?.as_ref()
    }
}

fn member_kind(model: &TypedAccessModel, tag: &str, field: &str) -> Option<MemberKind> {
    if model.scalar_field(tag, field) {
        Some(MemberKind::Scalar)
    } else if model.single_pointer_field(tag, field)
        || model.void_pointer_field(tag, field)
        || model.record_pointer_field(tag, field).is_some()
    {
        Some(MemberKind::Pointer)
    } else {
        None
    }
}

/// Read one left-hand side as a store into one member through a resolved,
/// typed base. `roots` is [`RecordRoots::read`]'s answer for the function.
pub fn read_destination(
    left: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
    roots: &RecordRoots,
) -> Result<Destination, Refusal> {
    if !types.is_readable() {
        return Err("declarations-unreadable");
    }
    // The root: `N`, `( N )` or `( & N )`.
    let (root, address, mut at) = match left {
        [name, ..] if is_name(name) => (*name, false, 1),
        ["(", name, ")", ..] if is_name(name) => (*name, false, 3),
        ["(", "&", name, ")", ..] if is_name(name) => (*name, true, 4),
        _ => return Err("prefix"),
    };
    let Some((tag, kind)) = roots.root(root) else {
        return Err("root-untyped");
    };
    let pointer = *kind == RootKind::Pointer;
    let array = *kind == RootKind::Array;
    if address && !matches!(kind, RootKind::Object) {
        return Err("address-of-pointer");
    }
    // An optional subscript through a record pointer, or into a local array.
    let mut subscripted = false;
    if left.get(at) == Some(&"[") {
        if !(pointer || array) || address {
            return Err("subscript-of-object");
        }
        let close = close_of(left, at, "[", "]").ok_or("subscript")?;
        let index = &left[at + 1..close];
        if !is_scalar_value_expression(index, model, types) {
            return Err("subscript-index");
        }
        subscripted = true;
        at = close + 1;
    }
    // Exactly one selection: `->` through a pointer (or `( & S )`), `.` on an
    // object or an element.
    let arrow = left.get(at..at + 2) == Some(&["-", ">"][..]);
    let (field, selector_len) = if arrow {
        (left.get(at + 2).copied(), 3)
    } else if left.get(at) == Some(&".") {
        (left.get(at + 1).copied(), 2)
    } else {
        return Err("selection");
    };
    let field = field.filter(|field| is_name(field)).ok_or("selection")?;
    if at + selector_len != left.len() {
        return Err("chain");
    }
    let through_pointer = match (kind, address, subscripted, arrow) {
        // `R -> F`, `R [ I ] . F`
        (RootKind::Pointer, false, false, true) | (RootKind::Pointer, false, true, false) => true,
        // `S . F`, `( & S ) -> F`, a local `A [ I ] . F`
        (RootKind::Object, false, false, false)
        | (RootKind::Object, true, false, true)
        | (RootKind::Array, false, true, false) => false,
        _ => return Err("selector-mismatch"),
    };
    let member = member_kind(model, tag, field).ok_or("member")?;
    let place = if subscripted {
        format!("{root}[].{field}")
    } else if through_pointer {
        format!("{root}->{field}")
    } else {
        format!("{root}.{field}")
    };
    Ok(Destination {
        base: through_pointer.then(|| root.to_owned()),
        place,
        member,
    })
}

/// Whether `right` is the null constant: `0`, `NULL`, or `( void * ) 0` with
/// any number of enclosing parentheses.
fn is_null_constant(mut right: &[&str]) -> bool {
    while right.len() >= 2
        && right[0] == "("
        && close_of(right, 0, "(", ")") == Some(right.len() - 1)
    {
        right = &right[1..right.len() - 1];
    }
    matches!(right, ["0"] | ["NULL"] | ["(", "void", "*", ")", "0"])
}

/// Strip one leading pointer cast `( T * )` whose type carries no
/// parenthesis, and enclosing parentheses.
fn strip_pointer_cast<'a, 'b>(mut right: &'b [&'a str]) -> &'b [&'a str] {
    loop {
        if right.len() >= 2
            && right[0] == "("
            && close_of(right, 0, "(", ")") == Some(right.len() - 1)
        {
            right = &right[1..right.len() - 1];
            continue;
        }
        if right.first() == Some(&"(")
            && let Some(close) = close_of(right, 0, "(", ")")
            && close + 1 < right.len()
            && right[close - 1] == "*"
            && right[1..close].iter().all(|text| {
                *text == "*"
                    || (!text.is_empty()
                        && text
                            .chars()
                            .all(|character| character.is_ascii_alphanumeric() || character == '_'))
            })
        {
            right = &right[close + 1..];
            continue;
        }
        return right;
    }
}

/// Read the value of one store into a member of kind `member`.
///
/// `call` is `Some` exactly when `right` is one whole call after an optional
/// pointer cast; the caller classifies that call itself.
pub fn read_value(
    right: &[&str],
    member: MemberKind,
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> Result<ValueSyntax, Refusal> {
    if right.is_empty() {
        return Err("value-empty");
    }
    match member {
        MemberKind::Scalar => {
            if is_scalar_value_expression(right, model, types) {
                return Ok(ValueSyntax::Scalar(Vec::new()));
            }
            if let Some(plan) = typed_scalar_value(right, model, types) {
                return Ok(ValueSyntax::Scalar(plan.bases));
            }
            Err("value-not-scalar")
        }
        MemberKind::Pointer => {
            if is_null_constant(right) {
                return Ok(ValueSyntax::Untracked);
            }
            let core = strip_pointer_cast(right);
            match core {
                ["__qcore_literal__"] => Ok(ValueSyntax::Untracked),
                [name] if is_name(name) => {
                    if types.declared_type(name).is_none() {
                        return Err("value-undeclared");
                    }
                    if types.declared_is_scalar(name) {
                        return Err("value-scalar-into-pointer");
                    }
                    Ok(ValueSyntax::Name((*name).to_owned()))
                }
                [callee, "(", .., ")"]
                    if is_name(callee) && close_of(core, 1, "(", ")") == Some(core.len() - 1) =>
                {
                    Ok(ValueSyntax::Call(core.len()))
                }
                _ => Err("value-shape"),
            }
        }
    }
}

/// The syntactic reading of a value, before the classifier decides a call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueSyntax {
    Scalar(Vec<String>),
    Untracked,
    Name(String),
    /// One whole call: the number of trailing tokens of the right-hand side
    /// that spell it (after any cast).
    Call(usize),
}

/// The flow decision of one admitted sub-object store: the `SubObjectStore`
/// arm of [`super::transfer`]. `base` is the ordinary transfer, used for the
/// fallback refusal and for a stored call.
pub(super) fn transfer(
    function: &str,
    node: &super::LegacyCfgNode,
    state: super::AbstractState,
    context: &super::TransferContext<'_>,
    alias: &super::NodeAliasFacts,
    diagnostics: &mut std::collections::BTreeSet<super::LegacyDiagnostic>,
    base: &dyn Fn(
        &super::LegacyCfgNode,
        super::AbstractState,
        &mut std::collections::BTreeSet<super::LegacyDiagnostic>,
    ) -> super::AbstractState,
) -> super::AbstractState {
    let super::LegacyInstruction::SubObjectStore {
        destination,
        value,
        value_bases,
        alias_read,
        fallback,
    } = &node.instruction
    else {
        return base(node, state, diagnostics);
    };
    let refuse =
        |state: super::AbstractState,
         diagnostics: &mut std::collections::BTreeSet<super::LegacyDiagnostic>| {
            base(&super::with_instruction(node, fallback), state, diagnostics)
        };
    if let Some(read) = alias_read {
        super::require_unaliased_reads(
            function,
            node.line,
            std::slice::from_ref(read),
            &state,
            &context.caller_sites,
            alias,
            diagnostics,
        );
    }
    // 1. The dereferenced base, and every base the scalar value reads through.
    let sanctioned = destination.base.as_ref().is_some_and(|pointer_base| {
        context
            .sanctioned
            .contains(format!("*{pointer_base}").as_str())
    });
    if sanctioned || !bases_resolve(destination, value_bases, &state) {
        return refuse(state, diagnostics);
    }
    // 2. and 3. The value.
    let mut state = state;
    match value {
        StoredValue::Scalar { .. } | StoredValue::Untracked => {}
        StoredValue::Name(name) => {
            if !stores_no_identity(&state, name) {
                return refuse(state, diagnostics);
            }
        }
        StoredValue::Call(call) => {
            state = base(&super::with_instruction(node, call), state, diagnostics);
            if !stores_no_identity(&state, STORE_TEMPORARY) {
                // The call's result reached a store this rule refuses. The
                // refused hand-on rule reads the statement's own tokens, which
                // never spell the temporary, so its identities are marked here:
                // they may have been stored.
                if let Some(pointer) = state.pointers.remove(STORE_TEMPORARY) {
                    super::mark_unresolved_effect(&mut state, &pointer);
                }
                return refuse(state, diagnostics);
            }
            state.pointers.remove(STORE_TEMPORARY);
        }
    }
    // The member's old fact no longer describes it.
    state.pointers.remove(&destination.place);
    state
}

/// Step 1: the dereferenced base resolves as W43's obligation requires, a
/// pointer member's base carries no tracked target, and every base a scalar
/// value reads through resolves.
fn bases_resolve(
    destination: &Destination,
    value_bases: &[String],
    state: &super::AbstractState,
) -> bool {
    destination.base.as_ref().is_none_or(|pointer_base| {
        super::indirect_scalar_base_resolves(state, pointer_base)
            && !(destination.member == MemberKind::Pointer
                && state
                    .pointers
                    .get(pointer_base)
                    .is_some_and(|pointer| !pointer.targets.is_empty()))
    }) && value_bases
        .iter()
        .all(|read| super::indirect_scalar_base_resolves(state, read))
}

/// Whether [`transfer`] applies a stored call's effects at this state: its
/// base premises (step 1) hold. The settled-state passes then examine that
/// call exactly as they examine any other, so every borrowed actual keeps its
/// liveness precondition. The out-slot sanction is not visible here; a base
/// it covers is examined too, which can only report more.
pub(super) fn call_runs(
    instruction: &super::LegacyInstruction,
    state: &super::AbstractState,
) -> bool {
    let super::LegacyInstruction::SubObjectStore {
        destination,
        value_bases,
        ..
    } = instruction
    else {
        return false;
    };
    bases_resolve(destination, value_bases, state)
}

/// Step 3: whether storing the pointer `name` holds stores no identity this
/// function tracks — no binding on any reaching path, or a binding with no
/// target that is not unknown (a null or untracked value). Any target, and an
/// unknown binding that may denote any identity, is custody the store would
/// hand on unseen, so the store keeps its refusal.
fn stores_no_identity(state: &super::AbstractState, name: &str) -> bool {
    state
        .pointers
        .get(name)
        .is_none_or(|pointer| pointer.targets.is_empty() && !pointer.unknown)
}

#[cfg(test)]
#[path = "../tests/unit/legacy_sub_object_store.rs"]
mod tests;
