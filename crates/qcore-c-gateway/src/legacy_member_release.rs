//! Card **W45-CALLEE-SUMMARY-CASCADE**: the member-release shapes of corpus
//! destructor bodies, and the call-site guard that keeps a summary of such a
//! body from hiding a release from its caller.
//!
//! The card's cascade census ranks the blocker families in the bodies of the
//! uncontracted corpus callees by how many Unknown functions their removal
//! would unblock. Outside the sub-object stores and return expressions that
//! other wave-45 cards own, the largest family is the destructor body:
//!
//! ```c
//! void rd_kafka_acl_result_destroy(rd_kafka_acl_result_t *acl_res) {
//!         if (acl_res->error)
//!                 rd_kafka_error_destroy(acl_res->error);
//!         rd_free(acl_res);
//! }
//! ```
//!
//! Two statements refuse there, and the refusal of the first poisons the
//! formal, so the last line reads `unresolved resource identity`. This module
//! admits exactly those two shapes and adds the guard their summaries need.
//!
//! ## 1. A pointer member handed to a nullable release
//!
//! Card W44-INDIRECT-CALL-ARGUMENT-ACCESS admits a typed pointer read
//! (`B->F`, `B[I]`, `*B`) as a whole actual only at a `Borrowed` position or
//! in a declared data tail. [`admits_release`] adds one custody position:
//! `Released { nullable: true }`, for a pointer-valued read with at least one
//! dereferenced base (so never `S.F` over a local record). The value read out
//! of memory is an object the projection does not track, so the release is
//! **outside the resource projection** and changes no abstract map: the
//! admitted call is lowered with that position read as `Borrowed`
//! ([`rewrite_released_positions`]). What the read keeps is the existing
//! obligation block, unchanged: the typed read of every base (unbound, a
//! non-null untracked binding, or exactly one known live identity; a
//! released base is a `use_after_release` candidate), then `ArgumentReads`,
//! which refuses a base whose target owns sub-custody (a field link, a
//! contained child or an indexed collection) and a place that is itself bound.
//! Those two refusals are exactly the ways the read value could be a tracked
//! identity, so no tracked identity is ever released by this route.
//!
//! Premises, stated rather than hidden:
//!
//! * **Only `nullable`.** A non-null release contract demands a non-null
//!   actual, and the projection establishes nothing about the nullness of an
//!   untracked member, so `Released { nullable: false }` keeps the refusal.
//! * **Never an address.** `&B->F` is an interior pointer of `*B`; handing it
//!   to a release is the invalid release the derived-result rule accuses. The
//!   recogniser this rule reuses never reads `&`, so a caller never releases an
//!   interior pointer through it.
//! * **Untracked liveness is outside the property**, as it already is for the
//!   typed reads of card W43-INDIRECT-ACCESS-GENERAL: two releases of the same
//!   untracked member are not detected. Every tracked identity keeps its
//!   complete custody model.
//! * **A stored pointer is tracked only through a bound place or a
//!   sub-custody relation.** This is W44-INDIRECT-CALL-ARGUMENT-ACCESS's
//!   premise 4, unchanged; a card that stores a tracked identity into a member
//!   without recording either must refuse this read (see the report).
//!
//! ## 2. A pointer member null test in a controlling expression
//!
//! [`member_null_test`] reads `B->F`, `!B->F`, `B->F == N`, `B->F != N`,
//! `N == B->F` and `N != B->F` (after enclosing parentheses, `N` a null
//! pointer constant) with `B` declared `struct T *` and `F` a pointer member
//! of `T`'s single complete definition. It lowers to the same typed read
//! (`IndirectScalarAccess` over `B`, no destination, no frame) that a scalar
//! member test lowers to: both successors carry the state unchanged, so the
//! test refines nothing, and `B` keeps its dereference obligation at the
//! header. Reading a member's nullness releases, binds and transfers nothing.
//!
//! ## 3. The call-site guard (why a summary may carry rule 1)
//!
//! A W4 summary states what a body does to its formals. Rule 1 lets a
//! validated body also release an object reachable **through** a formal,
//! which no summary mode states. A caller whose actual owns sub-custody could
//! therefore hold that very object as a tracked child and release it again
//! after the call. [`guard_unit_callees`] closes that channel on the caller's
//! side, for every callee whose contract in force may be an inference: before
//! each call to a function **defined in this translation unit and not
//! declared** by the environment's trusted texts, it inserts `ArgumentReads`
//! over the call's plain-name actuals, which records `UnknownAlias` for any
//! actual whose target owns sub-custody. It is applied whether or not the
//! callee's body used rule 1, so it is a superset of the calls that need it,
//! and it only ever refuses. A callee with no body here has no derived summary
//! (W4 summaries never cross a unit), and a declared callee is governed by its
//! declaration, never by an inference (constraint 8); both keep the
//! declaration's stated premise (P-B08-SHALLOW) and carry no guard. The first
//! AFTER run also guarded declared header bodies, and `flb_input_chunk_create`
//! left `Safe` for a guard its `flb_free` call did not need. The inserted node
//! is appended and every edge into the call is redirected to it, so no
//! existing node's identity moves.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::{ResourceContracts, ResourceFunctionContract, ResourceParameterMode};

use super::call_argument_access::{AdmittedArgument, ArgumentRead};
use super::{
    LegacyCfg, LegacyCfgNode, LegacyInstruction, Token, find_function_body, is_null_constant,
    strip_parentheses,
};
use crate::typed_field_access::{DeclaredType, FunctionTypes, TypedAccessModel, is_name};

/// Rule 1: whether a recognised actual at `index` of a call under `contract`
/// is admitted as a pointer member handed to a nullable release.
pub(super) fn admits_release(
    contract: Option<&ResourceFunctionContract>,
    index: usize,
    read: &ArgumentRead,
) -> bool {
    read.pointer_valued
        && !read.bases.is_empty()
        && matches!(
            contract.and_then(|contract| contract.parameters.get(index)),
            Some(ResourceParameterMode::Released { nullable: true })
        )
}

/// Whether every used actual admitted by rule 1 sits at a `Released
/// { nullable: true }` position of the contracted call the probe classified.
/// Anything else (an element store around the call, a contract the
/// classification rewrote) declines the whole admission, which keeps the
/// original refusal.
pub(super) fn released_positions_rewritable(
    result: &LegacyInstruction,
    used: &[AdmittedArgument],
) -> bool {
    let releases = used
        .iter()
        .filter(|entry| entry.release)
        .collect::<Vec<_>>();
    if releases.is_empty() {
        return true;
    }
    let LegacyInstruction::ContractCall {
        function, contract, ..
    } = result
    else {
        return false;
    };
    releases.iter().all(|entry| {
        entry.callee == *function
            && matches!(
                contract.parameters.get(entry.index),
                Some(ResourceParameterMode::Released { nullable: true })
            )
    })
}

/// Rule 1's lowering: each position a used rule-1 actual occupies is read as
/// `Borrowed`, because the object released there is outside the projection.
pub(super) fn rewrite_released_positions(
    result: &mut LegacyInstruction,
    used: &[AdmittedArgument],
) {
    let LegacyInstruction::ContractCall { contract, .. } = result else {
        return;
    };
    for entry in used.iter().filter(|entry| entry.release) {
        if let Some(mode) = contract.parameters.get_mut(entry.index) {
            *mode = ResourceParameterMode::Borrowed;
        }
    }
}

/// Rule 2: the bases of a pointer member null test, or `None`.
pub(super) fn member_null_test(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> Option<String> {
    let texts = strip_parentheses(texts);
    if let Some(rest) = texts.strip_prefix(&["!"]) {
        return member_null_test(rest, model, types);
    }
    if let Some(split) = comparison(texts) {
        let (left, right) = (&texts[..split], &texts[split + 2..]);
        if is_null_constant(right) {
            return pointer_member_base(left, model, types);
        }
        if is_null_constant(left) {
            return pointer_member_base(right, model, types);
        }
        return None;
    }
    pointer_member_base(texts, model, types)
}

/// The index of the only top-level `==` or `!=` in `texts`.
fn comparison(texts: &[&str]) -> Option<usize> {
    let mut depth = 0usize;
    let mut found = None;
    for index in 0..texts.len() {
        match texts[index] {
            "(" => depth += 1,
            ")" => depth = depth.checked_sub(1)?,
            "=" | "!"
                if depth == 0
                    && texts.get(index + 1) == Some(&"=")
                    && (index == 0 || !matches!(texts[index - 1], "=" | "!" | "<" | ">")) =>
            {
                if found.is_some() {
                    return None;
                }
                found = Some(index);
            }
            _ => {}
        }
    }
    found
}

/// `B -> F` with `B` declared `struct T *` and `F` a pointer member of `T`.
fn pointer_member_base(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> Option<String> {
    let texts = strip_parentheses(texts);
    let [base, "-", ">", field] = texts else {
        return None;
    };
    if !is_name(base) || !is_name(field) {
        return None;
    }
    let Some(DeclaredType::RecordPointer(tag)) = types.declared_type(base) else {
        return None;
    };
    (model.borrowable_field(tag, field) && !model.scalar_field(tag, field))
        .then(|| (*base).to_owned())
}

/// Rule 3: before every call to a function defined in this translation unit
/// and not declared by the environment,
/// the sub-custody guard over the call's plain-name actuals.
pub(super) fn guard_unit_callees(
    cfg: &mut LegacyCfg,
    tokens: &[Token],
    contracts: &ResourceContracts,
) {
    let mut defined = BTreeMap::<String, bool>::new();
    // Read lazily: most CFGs call no function this unit defines.
    let mut declared = None::<BTreeSet<String>>;
    let mut is_defined = |name: &str| {
        *defined.entry(name.to_owned()).or_insert_with(|| {
            find_function_body(tokens, name).is_some()
                && !declared
                    .get_or_insert_with(|| super::descriptor_custody::declared_callees(contracts))
                    .contains(name)
        })
    };
    let count = cfg.nodes.len();
    for id in 0..count {
        let mut actuals = Vec::<String>::new();
        for (function, arguments) in guarded_calls(&cfg.nodes[id].instruction) {
            if !is_defined(function) {
                continue;
            }
            for argument in arguments.iter().flatten() {
                if is_name(argument) && !actuals.contains(argument) {
                    actuals.push(argument.clone());
                }
            }
        }
        if actuals.is_empty() {
            continue;
        }
        let guard = cfg.nodes.len();
        let line = cfg.nodes[id].line;
        cfg.nodes.push(LegacyCfgNode {
            instruction: LegacyInstruction::ArgumentReads {
                names: Vec::new(),
                pointer_bases: actuals,
                pointer_places: Vec::new(),
            },
            successors: vec![id],
            line,
            is_exit: false,
            loop_exit_edge: None,
        });
        for (other, node) in cfg.nodes.iter_mut().enumerate() {
            if other == guard {
                continue;
            }
            for successor in &mut node.successors {
                if *successor == id {
                    *successor = guard;
                }
            }
        }
        if cfg.entry == id {
            cfg.entry = guard;
        }
    }
}

/// Every call an instruction makes, with its actuals: a contracted call and
/// its admitted nested calls, and the call inside an element store.
fn guarded_calls(instruction: &LegacyInstruction) -> Vec<(&str, &[Option<String>])> {
    match instruction {
        LegacyInstruction::ContractCall {
            function,
            arguments,
            nested,
            ..
        } => {
            let mut calls = vec![(function.as_str(), arguments.as_slice())];
            calls.extend(
                nested
                    .iter()
                    .map(|call| (call.function.as_str(), call.arguments.as_slice())),
            );
            calls
        }
        LegacyInstruction::ElementStore {
            call: Some(call), ..
        } => guarded_calls(call),
        _ => Vec::new(),
    }
}

/// The distinct callees a set of guarded calls names (tests only).
#[cfg(test)]
pub(super) fn guarded_callees(cfg: &LegacyCfg) -> BTreeSet<String> {
    cfg.nodes
        .iter()
        .filter_map(|node| match &node.instruction {
            LegacyInstruction::ArgumentReads {
                names,
                pointer_places,
                ..
            } if names.is_empty() && pointer_places.is_empty() => node
                .successors
                .first()
                .map(|next| &cfg.nodes[*next].instruction),
            _ => None,
        })
        .flat_map(|instruction| {
            guarded_calls(instruction)
                .into_iter()
                .map(|(function, _)| function.to_owned())
        })
        .collect()
}

#[cfg(test)]
#[path = "../tests/unit/legacy_member_release.rs"]
mod tests;
