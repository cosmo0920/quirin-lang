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

//! Card **CG10-SMALL-BUNDLE** (gateway rule 74b): a formal declared
//! `R (*f)(A)` in the shared W4 lexical parameter reader.
//!
//! The token reader's `function_parameters` refused every declarator list
//! holding `(`, so a function with a function-pointer formal
//! (`rd_kafka_buf_set_maker`'s `void (*free_make_opaque_cb)(void *)`)
//! published no summary and gave every body reading an empty formal set.
//!
//! Where the old reader refuses a list holding `(`, [`read`] splits it at its
//! top-level commas and reads it only when every group holding `(` is
//! exactly `SPEC+ ( * NAME ) ( BALANCED )` ([`function_pointer_name`]): one
//! or more specifier or `*` tokens, then the parenthesised `* NAME`, then one
//! balanced parameter list with no brace. Each such group names the formal
//! `NAME`, a function pointer (C11 6.7.6.3; a pointer to a function type
//! designates no object, 6.3.2.3p8, so it carries no lifetime: rule 10v's
//! class); every other group is read by the old reader alone, so a list with
//! no such group reads exactly as before. The whole list stays refused when
//!
//! * any parenthesised group has another shape (`int (cmp)(…)`, `void
//!   (*log_cb)`, `void (* const f)(void)`, a nested declarator, a macro such
//!   as `STACK_OF(X509) *`), the list is variadic, or a name repeats;
//! * **the body spells a call through a read formal** (`NAME (` or `( * NAME
//!   ) (`): an invocation of a caller-chosen function can release what it is
//!   handed (contract spec §C04.19's callback shape), and every consumer
//!   resolves a call by its callee's spelling, so a formal named like a
//!   contracted function would be read as that function.
//!
//! The positions are kept: the result has one entry per top-level group, so
//! every consumer that zips the names with a contract's parameters or a
//! call's actuals sees the formal at its index. `QCore.FunctionPointerFormal`
//! is the model, bound to [`DECISION_VECTORS`].

use std::collections::BTreeSet;

use super::{DECLARATION_SPECIFIERS, Token, is_identifier, parse_parameter_declarators};

/// The class of one top-level parameter group, as the reading decides it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Group {
    /// No `(`: the old reader's group (named, unnamed or refused by it).
    Plain,
    /// `SPEC+ ( * NAME ) ( BALANCED )`, `NAME` never called in the body.
    FunctionPointer,
    /// `SPEC+ ( * NAME ) ( BALANCED )`, `NAME` called in the body.
    CalledFunctionPointer,
    /// Any other group holding `(`.
    Other,
}

/// Rule 74b's decision over a list's groups (the old reader's refusal of a
/// plain group, a variadic list and a repeated name are decided beside it).
pub(crate) fn admits(groups: &[Group]) -> bool {
    groups
        .iter()
        .all(|group| matches!(group, Group::Plain | Group::FunctionPointer))
}

#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(&[Group], bool)] = &[
    (&[Group::Plain, Group::FunctionPointer], true),
    (&[Group::FunctionPointer], true),
    (&[Group::Plain, Group::Plain], true),
    (
        &[Group::FunctionPointer, Group::CalledFunctionPointer],
        false,
    ),
    (&[Group::Plain, Group::Other], false),
    (&[Group::Other], false),
    (&[Group::CalledFunctionPointer], false),
    (&[], true),
];

/// The list `texts` split at its top-level commas; `None` when a bracket is
/// unbalanced.
fn split_top_level<'a>(texts: &'a [&'a str]) -> Option<Vec<&'a [&'a str]>> {
    let mut groups = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (index, text) in texts.iter().enumerate() {
        match *text {
            "(" | "[" => depth += 1,
            ")" | "]" => depth = depth.checked_sub(1)?,
            "," if depth == 0 => {
                groups.push(&texts[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    (depth == 0).then_some(())?;
    groups.push(&texts[start..]);
    Some(groups)
}

/// `NAME` when `group` is exactly `SPEC+ ( * NAME ) ( BALANCED )`.
pub(crate) fn function_pointer_name<'a>(group: &[&'a str]) -> Option<&'a str> {
    let open = group.iter().position(|text| *text == "(")?;
    let specifiers = &group[..open];
    if specifiers.is_empty()
        || !specifiers
            .iter()
            .all(|text| *text == "*" || is_identifier(text))
        || !specifiers.iter().any(|text| is_identifier(text))
    {
        return None;
    }
    let [_, "*", name, ")", "(", list @ ..] = &group[open..] else {
        return None;
    };
    if !is_identifier(name) || DECLARATION_SPECIFIERS.contains(name) || list.last() != Some(&")") {
        return None;
    }
    // The parameter list is one balanced group ending the declarator.
    let mut depth = 1usize;
    for (index, text) in list.iter().enumerate() {
        match *text {
            "{" | "}" => return None,
            "(" => depth += 1,
            ")" => {
                depth -= 1;
                if depth == 0 && index + 1 != list.len() {
                    return None;
                }
            }
            _ => {}
        }
    }
    (depth == 0).then_some(name)
}

/// Whether `body` spells a call through `name`: `name (` or `( * name ) (`.
pub(crate) fn called_in(body: &[&str], name: &str) -> bool {
    body.iter().enumerate().any(|(index, text)| {
        *text == name
            && (body.get(index + 1) == Some(&"(")
                || (index >= 2
                    && body[index - 1] == "*"
                    && body[index - 2] == "("
                    && body.get(index + 1) == Some(&")")
                    && body.get(index + 2) == Some(&"(")))
    })
}

/// Rule 74b: the formal names of the list `parameters` (the tokens between
/// the declarator's parentheses) whose body is `body`, when every group
/// holding `(` is a function-pointer formal the body never calls; `None`
/// otherwise. Asked only where the old reader refused the list.
pub(crate) fn read(parameters: &[&str], body: &[&str]) -> Option<Vec<Option<String>>> {
    if !parameters.contains(&"(") || parameters.iter().any(|text| matches!(*text, "{" | "}")) {
        return None;
    }
    let groups = split_top_level(parameters)?;
    let mut classes = Vec::with_capacity(groups.len());
    let mut names = Vec::with_capacity(groups.len());
    for group in &groups {
        if group.contains(&"(") {
            match function_pointer_name(group) {
                Some(name) if called_in(body, name) => {
                    classes.push(Group::CalledFunctionPointer);
                    names.push(None);
                }
                Some(name) => {
                    classes.push(Group::FunctionPointer);
                    names.push(Some(name.to_owned()));
                }
                None => {
                    classes.push(Group::Other);
                    names.push(None);
                }
            }
            continue;
        }
        // A plain group is the old reader's, alone; `void` alone or a
        // variadic `...` is no formal here.
        let tokens = group
            .iter()
            .map(|text| Token {
                text: (*text).to_owned(),
                line: 0,
            })
            .collect::<Vec<_>>();
        if group.is_empty() || *group == ["void"] || group.iter().any(|text| text.starts_with('.'))
        {
            return None;
        }
        let parsed = parse_parameter_declarators(&tokens)?;
        let [name] = parsed.as_slice() else {
            return None;
        };
        classes.push(Group::Plain);
        names.push(name.clone());
    }
    record(parameters, &classes);
    if !admits(&classes) {
        return None;
    }
    let mut seen = BTreeSet::new();
    if !names.iter().flatten().all(|name| seen.insert(name.clone())) {
        return None;
    }
    Some(names)
}

/// Parity only: one row per list read.
fn record(parameters: &[&str], classes: &[Group]) {
    if !crate::typed_access_path::parity_enabled() {
        return;
    }
    crate::typed_access_path::record_row(serde_json::json!({
        "position": "function-pointer-formal",
        "rule": "74b",
        "texts": parameters,
        "groups": classes.iter().map(|class| format!("{class:?}")).collect::<Vec<_>>(),
        "admitted": admits(classes),
    }));
}

#[cfg(test)]
#[path = "../tests/unit/function_pointer_formal.rs"]
mod tests;
