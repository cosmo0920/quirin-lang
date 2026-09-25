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
//! * `R -> F` or `( R ) -> F`, with `R` declared exactly `struct T *` — a
//!   store through the pointer `R`, which keeps W43's dereference obligation;
//! * `R [ I ] . F` or `( R ) [ I ] . F`, with `R` declared `struct T *` and
//!   `I` a scalar-only index that writes nothing — a store through `R` into
//!   the element `I`, with the same obligation (bounds are not decided);
//! * `S . F`, `( S ) . F` or `( & S ) -> F`, with `S` declared exactly
//!   `struct T` — a store into this function's own record object, which
//!   dereferences nothing.
//!
//! `F` must be a scalar member or a member holding exactly one pointer
//! (`single_pointer_field`, `void_pointer_field`, `record_pointer_field`).
//! Casts, `(*R)`, chains, nested aggregates, typedefs the reader will not
//! resolve and undeclared roots all keep the refusal.
//!
//! **Value** ([`read_value`]):
//!
//! * into a scalar member, a scalar expression (W43's walker: literals,
//!   declared scalars, enumeration constants, scalar casts and typed scalar
//!   accesses, whose bases carry their own obligation);
//! * into a pointer member, the null constant, a string literal, a declared
//!   name (optionally cast), or one whole call.
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
//! 3. A named or call-result pointer value is decided on its binding:
//!    * no binding on any reaching path, or a non-null binding with no target:
//!      no identity this function tracks is stored — custody-neutral;
//!    * exactly one known, exactly-live **heap** identity this body acquired
//!      (an identity binding, not a caller's site, not nullable-unresolved),
//!      stored through a base the caller can reach (a formal parameter this
//!      function never rebinds): the identity **escapes** — custody leaves
//!      with the caller's record, as it does through a return;
//!    * anything else — a derived pointer, a caller's own site, a released or
//!      unresolved identity, a descriptor, a store of an owned identity into a
//!      record the caller cannot reach — is the fallback.
//!
//! A call result is applied first (its own obligations and diagnostics are
//! the call's), bound to a temporary, decided as a named value and then
//! forgotten. The classifier admits a call value only when its contract in
//! force states a `None` result or an owned heap result, and the latter only
//! into a caller-reachable base; every other call keeps the refusal.
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
    DeclaredType, FunctionTypes, TypedAccessModel, is_name, is_scalar_value_expression,
    typed_scalar_value,
};

/// Whether the stored member holds a scalar or exactly one pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemberKind {
    Scalar,
    Pointer,
}

/// The destination of one admitted sub-object store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Destination {
    /// The pointer the store dereferences (`R` of `R->F` and `R[I].F`);
    /// `None` for a store into this function's own record object.
    pub base: Option<String>,
    /// The member's place spelling, the only place the store may write.
    pub place: String,
    pub member: MemberKind,
    /// Whether the store is through a formal parameter this function never
    /// rebinds, so that the record is one the caller can reach.
    pub caller_visible: bool,
}

/// The stored value, as the classifier read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoredValue {
    /// A scalar expression; the bases its typed accesses dereference carry
    /// W43's obligation.
    Scalar { bases: Vec<String> },
    /// The null constant or a string literal: no identity at all.
    Untracked,
    /// A declared name, decided on its binding.
    Name(String),
    /// One whole contracted call, applied first, whose result is bound to
    /// [`STORE_TEMPORARY`] and then decided as a name.
    Call(Box<super::LegacyInstruction>),
}

/// The temporary a stored call result is bound to between the call and the
/// store. Never a C identifier (a leading double underscore is reserved), and
/// forgotten after the store.
pub const STORE_TEMPORARY: &str = "__qcore_sub_object_value__";

/// Census commit only: whether the classifier consults the rule.
pub const WIRED: bool = false;

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

/// The record tag a declared name has, and whether it is a pointer to it.
fn record_of(types: &FunctionTypes, name: &str) -> Option<(String, bool)> {
    match types.declared_type(name)? {
        DeclaredType::RecordPointer(tag) => Some((tag.clone(), true)),
        DeclaredType::Record(tag) => Some((tag.clone(), false)),
        DeclaredType::Scalar | DeclaredType::Opaque => None,
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

/// The formal parameters whose record the caller can reach through them: a
/// formal no statement of `body` writes (`p = …`, `p op= …`, `p++`, `--p`)
/// and, per `unaliased` (the frame-protected names, which exclude every
/// address-taken one), whose slot no other lvalue designates. Such a formal
/// holds, at every point of the body, the pointer the caller passed.
pub fn caller_bases(
    body: &[&str],
    formals: &std::collections::BTreeSet<String>,
    unaliased: &std::collections::BTreeSet<String>,
) -> std::collections::BTreeSet<String> {
    let mut bases = formals
        .intersection(unaliased)
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    for (index, text) in body.iter().enumerate() {
        if !bases.contains(*text) {
            continue;
        }
        let previous = index.checked_sub(1).map(|at| body[at]);
        let before = index.checked_sub(2).map(|at| body[at]);
        let next = body.get(index + 1).copied();
        let after = body.get(index + 2).copied();
        // A member name (`q->p`, `s.p`) is not the formal.
        if previous == Some(".") || (previous == Some(">") && before == Some("-")) {
            continue;
        }
        let written = (next == Some("=") && after != Some("="))
            || (matches!(
                next,
                Some("+" | "-" | "*" | "/" | "%" | "&" | "|" | "^")
            ) && after == Some("="))
            || matches!((next, after), (Some("+"), Some("+")) | (Some("-"), Some("-")))
            || matches!((next, after), (Some("<"), Some("<")) | (Some(">"), Some(">")))
            || matches!((before, previous), (Some("+"), Some("+")) | (Some("-"), Some("-")))
            || previous == Some("&");
        if written {
            bases.remove(*text);
        }
    }
    bases
}

/// Read one left-hand side as a store into one member through a resolved,
/// typed base. `caller_bases` is [`caller_bases`]' answer for the function.
pub fn read_destination(
    left: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
    caller_bases: &std::collections::BTreeSet<String>,
) -> Result<Destination, Refusal> {
    if !types.is_readable() {
        return Err("declarations-unreadable");
    }
    // The root: `N`, `( N )` or `( & N )`.
    let (root, address, mut at) = match left {
        [name, ..] if is_name(name) => (*name, false, 1),
        ["(", name, ")", ..] if is_name(name) => (*name, false, 3),
        ["(", "&", name, ")", ..] if is_name(name) => (*name, true, 4),
        ["(", ..] => return Err("prefix"),
        _ => return Err("prefix"),
    };
    let Some((tag, pointer)) = record_of(types, root) else {
        return Err("root-untyped");
    };
    if address && pointer {
        return Err("address-of-pointer");
    }
    // An optional subscript through a record pointer.
    let mut subscripted = false;
    if left.get(at) == Some(&"[") {
        if !pointer || address {
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
    let through_pointer = match (pointer, address, subscripted, arrow) {
        // `R -> F`
        (true, false, false, true) => true,
        // `R [ I ] . F`
        (true, false, true, false) => true,
        // `S . F`, `( & S ) -> F`
        (false, false, false, false) | (false, true, false, true) => false,
        _ => return Err("selector-mismatch"),
    };
    let member = member_kind(model, &tag, field).ok_or("member")?;
    let place = if through_pointer {
        if subscripted {
            format!("{root}[].{field}")
        } else {
            format!("{root}->{field}")
        }
    } else {
        format!("{root}.{field}")
    };
    Ok(Destination {
        base: through_pointer.then(|| root.to_owned()),
        place,
        member,
        caller_visible: through_pointer && caller_bases.contains(root),
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
            && right[1..close].iter().all(|text| is_name(text) || *text == "*")
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
                    if is_name(callee)
                        && close_of(core, 1, "(", ")") == Some(core.len() - 1) =>
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
#[allow(clippy::too_many_arguments)]
pub(super) fn transfer(
    function: &str,
    node: &super::LegacyCfgNode,
    state: super::AbstractState,
    acquisition_lines: &std::collections::BTreeMap<usize, usize>,
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
    let refuse = |state: super::AbstractState,
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
    if let Some(pointer_base) = &destination.base {
        let sanctioned = context
            .sanctioned
            .contains(format!("*{pointer_base}").as_str());
        let tracked = state
            .pointers
            .get(pointer_base)
            .is_some_and(|pointer| !pointer.targets.is_empty());
        if sanctioned
            || !super::indirect_scalar_base_resolves(&state, pointer_base)
            || (destination.member == MemberKind::Pointer && tracked)
        {
            return refuse(state, diagnostics);
        }
    }
    if value_bases
        .iter()
        .any(|read| !super::indirect_scalar_base_resolves(&state, read))
    {
        return refuse(state, diagnostics);
    }
    // 2. and 3. The value.
    let mut state = state;
    match value {
        StoredValue::Scalar { .. } | StoredValue::Untracked => {}
        StoredValue::Name(name) => match decide(&state, name, destination.caller_visible, context) {
            Decision::Neutral => {}
            Decision::Escape => super::apply_resource_action(
                function,
                node.line,
                name,
                &mut state,
                acquisition_lines,
                diagnostics,
                true,
                true,
            ),
            Decision::Refuse => return refuse(state, diagnostics),
        },
        StoredValue::Call(call) => {
            state = base(&super::with_instruction(node, call), state, diagnostics);
            match decide(
                &state,
                STORE_TEMPORARY,
                destination.caller_visible,
                context,
            ) {
                Decision::Neutral => {}
                Decision::Escape => super::apply_resource_action(
                    function,
                    node.line,
                    STORE_TEMPORARY,
                    &mut state,
                    acquisition_lines,
                    diagnostics,
                    true,
                    true,
                ),
                Decision::Refuse => {
                    // The call's result reached a store this rule refuses. The
                    // refused hand-on rule reads the statement's own tokens,
                    // which never spell the temporary, so its identities are
                    // marked here: they may have been stored.
                    if let Some(pointer) = state.pointers.remove(STORE_TEMPORARY) {
                        super::mark_unresolved_effect(&mut state, &pointer);
                    }
                    return refuse(state, diagnostics);
                }
            }
            state.pointers.remove(STORE_TEMPORARY);
        }
    }
    // The member's old fact no longer describes it.
    state.pointers.remove(&destination.place);
    state
}

/// Whether [`transfer`] applies a stored call's effects at this state: its
/// base premises (step 1) hold. The settled-state passes then examine that
/// call exactly as they examine any other, so every borrowed actual keeps its
/// liveness precondition. The out-slot sanction is not visible here; a base
/// it covers is examined too, which can only report more.
pub(super) fn call_runs(instruction: &super::LegacyInstruction, state: &super::AbstractState) -> bool {
    let super::LegacyInstruction::SubObjectStore {
        destination,
        value_bases,
        ..
    } = instruction
    else {
        return false;
    };
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

enum Decision {
    Neutral,
    Escape,
    Refuse,
}

/// What storing the pointer `name` holds does to custody.
fn decide(
    state: &super::AbstractState,
    name: &str,
    caller_visible: bool,
    context: &super::TransferContext<'_>,
) -> Decision {
    let Some(pointer) = state.pointers.get(name) else {
        // No reaching path bound it: it holds nothing this function tracks.
        return Decision::Neutral;
    };
    if pointer.targets.is_empty() {
        return if pointer.unknown {
            Decision::Refuse
        } else {
            Decision::Neutral
        };
    }
    let escapes = caller_visible
        && !pointer.unknown
        && pointer.derivation.is_identity()
        && pointer.targets.len() == 1
        && pointer.targets.iter().all(|site| {
            !context.caller_sites.contains(site)
                && state.resources.get(site) == Some(&super::LIVE)
                && !state.unresolved_effect_sites.contains(site)
                && !state.collections.contains_key(site)
                && super::kind_of(&context.site_kinds, *site).is_heap()
        });
    if escapes {
        Decision::Escape
    } else {
        Decision::Refuse
    }
}

#[cfg(test)]
#[path = "../tests/unit/legacy_sub_object_store.rs"]
mod tests;
