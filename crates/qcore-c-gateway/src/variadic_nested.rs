//! Declared variadic data tails and one level of nested calls (card
//! **O04-TRACE**).
//!
//! These are the two families card O04-B09B10 measured to be jointly the whole
//! residual of the constructor's success exit **C6**, in one statement:
//!
//! ```c
//! flb_trace("… '%s' … '%s'", flb_input_chunk_get_name(ic), tag);
//! ```
//!
//! `flb_trace` is `void flb_trace(const char *, ...)`, which the contract
//! vocabulary could not spell (card O04-ARMS's **O04-ARMS-VARIADIC**), and its
//! second argument is a **call**, which card O04-B09B10 turned into an explicit
//! refusal (**O04-B09B10-NESTED**). Neither family is a variation of the other,
//! so both land here and must compose.
//!
//! # What each family admits
//!
//! ## The variadic data tail
//!
//! `qcore_contract::contract_loader` accepts
//! `ensures normal resource_variadic_data_tail();` on a declaration. It is a
//! **trusted assumption** that the callee takes no custody of, and retains no
//! pointer to, any argument beyond the ones the declaration names. It says
//! nothing about the callee's body, nothing about format strings, and it
//! **admits nothing on its own**: this module still requires, per call site,
//! that every tail actual really is data
//! ([`tail_arguments_are_data`], premises T1-T4). An actual carrying a tracked
//! resource identity refuses under the declaration exactly as it does without
//! one — the declaration covers what it covers.
//!
//! Without a declared tail, an actual beyond the declared parameter vector is
//! now **refused**. That is a fail-closed repair: the modeling zips the
//! contract's parameter vector against the actuals, so before this card an
//! excess actual was silently dropped, exactly as card O04-B09B10 found a
//! nested call's effects being dropped.
//!
//! ## One level of nesting
//!
//! [`admitted_nested_call`] admits exactly one nested call, under eight
//! conjunctive premises. The load-bearing pair is **N6** and **N7**: the nested
//! contract's result mode is `None` and every one of its parameter modes is
//! `Borrowed`. Under those two the nested call's **declared effects are the
//! identity on the abstract state**, which is what makes composing it with the
//! outer call well defined at all: C leaves the evaluation order of a call's
//! arguments unspecified (C11 6.5.2.2p10), so an inner call with any custody
//! effect would have to be ordered against the outer call's other arguments,
//! and this projection has no such order. An admitted inner call needs no
//! order because it changes nothing.
//!
//! The result mode is where "data-only or a borrowed handback" resolves: the v1
//! vocabulary has exactly **one** non-custody result mode, `resource_return_none()`,
//! so both phrasings are the same premise. A borrowed handback's aliasing into
//! the callee's argument is deliberately **not** modeled — the result is not
//! tracked at all — which is why premise **N8** requires the result to occupy a
//! position that needs no identity: a declared `Borrowed` parameter, or a
//! declared data tail. It is never bound to a name, so it cannot outlive the
//! statement.
//!
//! # What is refused, by construction
//!
//! Two or more nested calls (two-level nesting included), a nested call whose
//! callee has no contract in force, a nested result carrying custody, a nested
//! parameter carrying custody, a nested call that is only *part* of an
//! argument, a nested call inside the nested call's own arguments, a tail
//! actual mentioning a name that may hold a tracked identity, and every tail
//! actual carrying `&`, `*`, a subscript or a member access.
//!
//! # Inherited premises, not proved here
//!
//! C02's declaration completeness and C effective types; C02-P's
//! `untracked_bases` closure and its recorded may-alias boundary
//! (**C02-P-ALIAS**), which is what decides whether a tail actual can hold an
//! identity; and that a declared `Borrowed` mode really borrows.



use qcore_schemas::contract::{
    ResourceContracts, ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
};

use crate::borrowed_actual::UntrackedBases;
use crate::typed_field_access::{FunctionTypes, TypedAccessModel, is_name};

/// One admitted nested call: the inner call, the outer argument it *is*, and
/// its own modeled argument places.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NestedCallActual {
    /// The outer call's argument index this nested call occupies whole.
    pub(crate) argument: usize,
    pub(crate) function: String,
    pub(crate) contract: ResourceFunctionContract,
    /// One entry per declared parameter of the nested contract, in order.
    pub(crate) arguments: Vec<Option<String>>,
    /// Half-open token span of the whole nested call within the statement.
    pub(crate) start: usize,
    pub(crate) end: usize,
}

/// The nested call this statement's outer call admits, if any.
///
/// `call_index` is the outer callee's token index — the index
/// `legacy_c::find_call` returned — and `arguments` its raw argument spans.
/// Returning `None` leaves card O04-B09B10's refusal in place unchanged.
#[allow(clippy::too_many_arguments)]
pub(crate) fn admitted_nested_call(
    texts: &[&str],
    call_index: usize,
    argument_spans: &[(usize, usize)],
    outer: &ResourceFunctionContract,
    outer_tail: bool,
    contracts: &ResourceContracts,
    types: &FunctionTypes,
) -> Option<NestedCallActual> {
    // Premise 0: a function whose declarations this reader refused admits
    // nothing at all, exactly as in every other typed family.
    if !types.is_readable() {
        return None;
    }
    // Premise N2: *exactly one* nested call in the whole statement. Two calls
    // beyond the outer one — `f(g(x), h(y))` — or a call inside the nested
    // call's own arguments — `f(g(h(x)))` — both leave more than one, so
    // two-level nesting is refused here and needs no separate rule.
    let mut nested_callees = texts.windows(2).enumerate().filter_map(|(index, pair)| {
        (index != call_index
            && is_name(pair[0])
            && pair[1] == "("
            && !crate::legacy_c::TYPE_OPERATORS.contains(&pair[0]))
        .then_some(index)
    });
    let nested_index = nested_callees.next()?;
    if nested_callees.next().is_some() {
        return None;
    }
    // Premise N4: the nested call is exactly one *whole* argument of the outer
    // call — its callee is the argument's first token and its closing `)` the
    // argument's last. A cast, an arithmetic combination, a comparison or any
    // other surrounding expression refuses.
    let (argument, &(start, end)) = argument_spans
        .iter()
        .enumerate()
        .find(|(_, (start, _))| *start == nested_index)?;
    let span = texts.get(start..end)?;
    if span.last().copied() != Some(")") {
        return None;
    }
    // Premise N5: the nested call's own argument list carries no call. N2
    // already implies it; asserting it here keeps the premise local and keeps
    // the depth bound explicit at one.
    if span[1..]
        .windows(2)
        .any(|pair| is_name(pair[0]) && pair[1] == "(")
    {
        return None;
    }
    // Premise N3: the nested callee has a contract in force. An uncontracted
    // inner call is refused — nothing models its effects, which is the whole
    // reason the outer statement was refused in the first place.
    let function = texts[nested_index];
    let contract = contracts.contract(function)?.clone();
    // Premises N6 and N7: the nested call's declared effects are the identity.
    if !matches!(contract.result, ResourceReturnMode::None)
        || !contract
            .parameters
            .iter()
            .all(|mode| matches!(mode, ResourceParameterMode::Borrowed))
    {
        return None;
    }
    // Premise N8: the position the nested result occupies needs no identity.
    let position_is_data = match outer.parameters.get(argument) {
        Some(ResourceParameterMode::Borrowed) => true,
        Some(_) => false,
        None => outer_tail,
    };
    if !position_is_data {
        return None;
    }
    // The nested call's own actuals, arity-exact: a nested call whose actual
    // count differs from its declared parameter count is refused rather than
    // zipped, the same repair the outer arity gate makes.
    let inner_arguments = crate::legacy_c::call_arguments(span, 0)?;
    if inner_arguments.len() != contract.parameters.len() {
        return None;
    }
    let arguments = inner_arguments
        .iter()
        .map(|argument| crate::legacy_c::borrowed_argument_place(argument))
        .collect::<Vec<_>>();
    Some(NestedCallActual {
        argument,
        function: function.to_owned(),
        contract,
        arguments,
        start,
        end,
    })
}

/// Whether every actual beyond the declared parameter vector carries no custody
/// (premises T1-T4).
///
/// `nested` is the one admitted nested call, which is data by premises N6-N8
/// and is therefore skipped by this walker rather than re-examined as text.
pub(crate) fn tail_arguments_are_data(
    arguments: &[&[&str]],
    named: usize,
    nested: Option<&NestedCallActual>,
    model: &TypedAccessModel,
    types: &FunctionTypes,
    bases: &UntrackedBases,
) -> bool {
    if !types.is_readable() {
        return false;
    }
    arguments
        .iter()
        .enumerate()
        .skip(named)
        .all(|(index, argument)| {
            if nested.is_some_and(|nested| nested.argument == index) {
                return true;
            }
            is_data_only_actual(argument, model, types, bases)
        })
}

/// One tail actual, checked conjunctively.
///
/// * **T1** — nonempty. An empty argument is a shape this rule does not read.
/// * **T2** — no `&`, `*`, `[`, `]`, `.` and no `->`. No address escapes, no
///   dereference, no subscript and no member access: each of those denotes
///   storage this projection has no model for, and the first of them is exactly
///   how a caller hands a callee a handle it could keep.
/// * **T3** — no call. The one admitted nested call is handled by its own
///   premises; every other call is refused.
/// * **T4** — every identifier is an enumeration constant, a declared scalar
///   variable, or a name C02-P's closure says this function can **never** bind
///   to a tracked resource identity. String and character literals reach this
///   walker as the tokenizer's single literal token, which is never bound and
///   is therefore admitted by the same clause rather than by a special case.
fn is_data_only_actual(
    argument: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
    bases: &UntrackedBases,
) -> bool {
    if argument.is_empty() {
        return false;
    }
    for (index, text) in argument.iter().enumerate() {
        if matches!(*text, "&" | "*" | "[" | "]" | ".") {
            return false;
        }
        if *text == "-" && argument.get(index + 1) == Some(&">") {
            return false;
        }
        if is_name(text) {
            if argument.get(index + 1) == Some(&"(") {
                return false;
            }
            if !types.declared_is_scalar(text)
                && !model.is_enum_constant(text)
                && !bases.never_bound(text)
            {
                return false;
            }
        }
    }
    true
}
