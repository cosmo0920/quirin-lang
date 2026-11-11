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

//! Card **CG26-VARIADICS** (gateway rule 11w), gap entry K08: trailing actuals
//! read per position under a declared variadic data tail.
//!
//! `resource_variadic_data_tail()` declares that the callee takes no custody
//! of, and retains no pointer to, any actual beyond the named parameters
//! (`qcore_contract::contract_loader`). That is the `Borrowed` mode, stated
//! for every trailing position at once. Before this card the gateway admitted
//! a trailing actual only when it was *data* (premises T1-T6 of
//! [`crate::variadic_nested`]): an actual that may carry a tracked identity,
//! or that takes an address, refused the whole call.
//!
//! This module reads the tail the way a declared `Borrowed` parameter is read,
//! and only that way. It is asked only by the statement fallback of
//! `legacy_variadics.rs`, after every older route refused the statement with
//! the tail refusal, and only for a callee whose declaration carries the tail.
//! Two further shapes are admitted beside T1-T6:
//!
//! * **T7** — `& NAME`, where `NAME` is a declared scalar (an arithmetic or
//!   enumeration object) the projection never binds to an integer-carried
//!   identity (not one of the function's descriptor names), and not a
//!   protected formal. C02-P's closure is not asked: it marks every
//!   address-taken name, which is this shape's own premise. The address denotes no
//!   identity, so the position is `Borrowed` with no modeled place; the callee
//!   may write the scalar (`fcntl`'s fourth actual is never read at `F_SETFD`,
//!   but the rule does not depend on that), so the statement's lowering
//!   forgets `NAME`'s scalar fact after the call (`ForgetScalar`). This is
//!   W46-FCNTL-SETFD-ADDRESS-TAIL's discharge.
//! * **T8** — one plain name, only for a callee whose declaration carries
//!   `resource_variadic_borrowed_tail()`. The data tail alone keeps O04-TRACE's
//!   refusal of an actual that may carry an identity ("the declaration covers
//!   what it covers"); the borrowed tail is the contract stating that trailing
//!   arguments are borrowed and not retained. The position is `Borrowed` with
//!   the name as its modeled place, so the flow checks it exactly as it checks a declared
//!   `Borrowed` parameter: a released identity there is a use-after-release
//!   candidate, an unresolved binding is `UnknownAlias`, and nothing is
//!   acquired, released or transferred.
//!
//! The extension is the contract's own parameter vector followed by one
//! `Borrowed` per trailing actual ([`borrowed_tail_contract`];
//! `QCore.Variadics.borrowed_tail_is_the_identity`). Default argument
//! promotions are read from Clang, not re-derived: the statement fallback
//! requires every trailing actual's promoted type (the type of the call's
//! argument node) to be an arithmetic, enumeration or pointer type
//! ([`promoted_scalar_or_pointer`]); a record, union, vector or complex
//! actual keeps the refusal.
//!
//! What stays refused: everything T2 refused except T7's exact shape (a
//! dereference, a subscript, a member address, the address of a pointer, of a
//! record or of a name the body may bind), a cast, arithmetic over a name
//! that may bind, a call T3 refused, and every trailing actual of a callee
//! without the declared tail. Format agreement (gap entry K10) is not checked:
//! nothing here reads a format string.
//!
//! Card CG26-TAIL-RESIDUAL-ACTUALS (rule 12g) reads a position these shapes
//! refuse once more, from its Clang node ([`crate::variadic_tail_actuals`]).

use qcore_schemas::contract::{ResourceFunctionContract, ResourceParameterMode};
use serde_json::Value;

use crate::typed_field_access::{FunctionTypes, is_name};

/// The shape of one trailing actual the widening admits beyond T1-T6.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WidenedTail {
    /// T7: `& NAME` of a never-bound scalar.
    ScalarAddress(String),
    /// T8: one plain name, read as a `Borrowed` place.
    Borrowed(String),
}

/// T7 or T8 (T8 only for a callee declaring the borrowed tail) for one
/// trailing actual, or `None`. `holds_identity` answers
/// whether the projection may bind a name to an integer-carried identity (the
/// function's descriptor names): such a scalar's address refuses, since the
/// callee could rewrite the identity's carrier.
pub(crate) fn widened_actual(
    argument: &[&str],
    types: &FunctionTypes,
    holds_identity: &dyn Fn(&str) -> bool,
    protected: &std::collections::BTreeSet<String>,
    borrowed_tail: bool,
) -> Option<WidenedTail> {
    if !types.is_readable() {
        return None;
    }
    match argument {
        ["&", name]
            if is_name(name)
                && types.declared_is_scalar(name)
                && !holds_identity(name)
                && !protected.contains(*name) =>
        {
            Some(WidenedTail::ScalarAddress((*name).to_owned()))
        }
        [name]
            if borrowed_tail
                && is_name(name)
                && !crate::legacy_c::TYPE_OPERATORS.contains(name)
                && !name.starts_with("__qcore_") =>
        {
            Some(WidenedTail::Borrowed((*name).to_owned()))
        }
        _ => None,
    }
}

/// A definition's parameter tokens without its trailing `, ...` (three `.`
/// tokens after a comma): the ellipsis declares no object, so the named
/// parameters are the whole declared set (C11 6.7.6.3p9). Any other
/// parameter list is returned unchanged, and a lone `...` (no named
/// parameter, not C before C23) is left for the reader to refuse.
pub(crate) fn named_parameters<'t, 'a>(parameters: &'t [&'a str]) -> &'t [&'a str] {
    match parameters {
        [named @ .., ",", ".", ".", "."] if !named.is_empty() => named,
        _ => parameters,
    }
}

/// The contract a widened call is applied with: the declared parameter vector
/// followed by `Borrowed` for each of `tail` trailing positions. Only the
/// parameter vector grows; the result, kinds and every other declared effect
/// are the declaration's own.
pub(crate) fn borrowed_tail_contract(
    contract: &ResourceFunctionContract,
    tail: usize,
) -> ResourceFunctionContract {
    let mut extended = contract.clone();
    extended
        .parameters
        .extend(std::iter::repeat_n(ResourceParameterMode::Borrowed, tail));
    extended
}

/// Whether a trailing actual's Clang argument node has, after the default
/// argument promotions Clang applied (C11 6.5.2.2p7), an arithmetic,
/// enumeration or object/function pointer type.
pub(crate) fn promoted_scalar_or_pointer(argument: &Value) -> bool {
    let text = argument["type"]["desugaredQualType"]
        .as_str()
        .or_else(|| argument["type"]["qualType"].as_str())
        .unwrap_or("");
    let text = text.trim();
    if text.ends_with('*') || text.ends_with("*const") || text.ends_with("* const") {
        return true;
    }
    if text.ends_with(')') && text.contains("(*)") {
        return true;
    }
    let base = text
        .trim_start_matches("const ")
        .trim_start_matches("volatile ");
    matches!(
        base,
        "int"
            | "unsigned int"
            | "long"
            | "unsigned long"
            | "long long"
            | "unsigned long long"
            | "double"
            | "long double"
            | "__int128"
            | "unsigned __int128"
    ) || (base.starts_with("enum ") && !base.contains(['*', '[', '(']))
}

#[cfg(test)]
#[path = "../tests/unit/variadic_tail.rs"]
mod tests;
