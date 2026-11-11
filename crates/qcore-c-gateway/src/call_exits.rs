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

//! Card **CG21-CALL-EXITS-AND-BOUNDARIES** (gateway rule 11o): how a call's
//! exit is established, and the explicit boundaries of the forms whose exit
//! or effect route G does not model.
//!
//! # The no-return path end
//!
//! A call ends its path, with no exit state and nothing after it checked, only
//! when **two** facts hold, each from its own authority:
//!
//! * **the declaration**: the callee's resource v1 contract carries
//!   `resource_no_return()` (`qcore_contract::contract_loader::no_return_callees`),
//!   which states that the call has no normal exit **and** that no frame of the
//!   calling thread resumes (the program terminates). Only that second half
//!   makes the path end exact for custody: a frame's live resources are not
//!   abandoned while the program goes on. The C attribute cannot say it:
//!   `longjmp` and `pthread_exit` are `noreturn` too, and the loader refuses
//!   to declare either ([`qcore_contract::contract_loader::UNDECLARABLE_C_NAMES`]);
//! * **the Clang declaration** the call resolves to is `noreturn`: every
//!   reference to the callee in this body is a `DeclRefExpr` to a
//!   `FunctionDecl` whose type carries `__attribute__((noreturn))` or which
//!   carries `NoReturnAttr`, `C11NoReturnAttr` or `CXX11NoReturnAttr`
//!   (inherited included), and nothing in the body declares the name as an
//!   object ([`resolve`]). A contract the declaration does not establish is
//!   refused at the call, never applied as a path end.
//!
//! The contract's own modes are still applied at the call (every parameter is
//! `resource_borrowed`, so they are the call's preconditions: the actuals'
//! liveness is checked as at any call). A no-return callee is never inferred
//! from its name, and a callee with the attribute but no such contract keeps
//! the refusal it had (an uncontracted call).
//!
//! # Boundaries
//!
//! A statement holding inline assembly (`asm`, `asm goto`, an `asm` label or
//! register binding), a Microsoft SEH keyword, a `cleanup` attribute or a
//! label's address (`&&label`) is an explicit, named, unbounded-write
//! boundary ([`statement_boundary`]), asked **first**, and so is a computed
//! `goto *E;`: the attribute's scope-exit call, the assembly's clobbers,
//! memory effects and label edges and a computed goto's target set are not
//! modelled, so no older route may
//! admit the statement without them. An uncontracted call to a name of
//! [`qcore_contract::contract_loader::UNDECLARABLE_C_NAMES`] (an extension
//! node, a returns-twice call or a non-local exit) is refused with that
//! reason instead of as an ordinary uncontracted call ([`undeclarable_call`]).

use std::collections::BTreeSet;

use qcore_schemas::contract::ResourceContracts;
use serde_json::Value;

use crate::typed_c_ast::{children, kind};

/// What this body's Clang AST says about every call of one callee name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Resolution {
    /// Every reference resolves to a `noreturn` function declaration.
    NoReturn,
    /// Every reference resolves to a function declaration that is not.
    Returns,
    /// No reference, a reference to something other than a function, the
    /// name declared as an object in the body, or references that disagree.
    Unresolved,
}

/// The lowering of one call statement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    /// The call is applied and its path continues (an ordinary contract).
    Continue,
    /// The call is applied and its path ends: no successor, no exit state.
    PathEnd,
    /// The call is refused (`Unknown`).
    Refused,
}

/// `QCore.CallExits.decide`: `contracted` the callee has a contract,
/// `declared` that contract carries `resource_no_return()`.
pub(crate) fn decide(contracted: bool, declared: bool, resolution: Resolution) -> Decision {
    if !contracted {
        // The existing classifier's uncontracted refusal.
        Decision::Refused
    } else if !declared {
        Decision::Continue
    } else if resolution == Resolution::NoReturn {
        Decision::PathEnd
    } else {
        Decision::Refused
    }
}

/// `DECISION_VECTORS` = `QCore.CallExits.decisionVectors`: every
/// `(contracted, declared, resolution, decision)`.
#[cfg(test)]
pub(crate) const DECISION_VECTORS: [(bool, bool, Resolution, Decision); 12] = [
    (false, false, Resolution::NoReturn, Decision::Refused),
    (false, false, Resolution::Returns, Decision::Refused),
    (false, false, Resolution::Unresolved, Decision::Refused),
    (false, true, Resolution::NoReturn, Decision::Refused),
    (false, true, Resolution::Returns, Decision::Refused),
    (false, true, Resolution::Unresolved, Decision::Refused),
    (true, false, Resolution::NoReturn, Decision::Continue),
    (true, false, Resolution::Returns, Decision::Continue),
    (true, false, Resolution::Unresolved, Decision::Continue),
    (true, true, Resolution::NoReturn, Decision::PathEnd),
    (true, true, Resolution::Returns, Decision::Refused),
    (true, true, Resolution::Unresolved, Decision::Refused),
];

/// The callees an environment declares `resource_no_return()` for, rederived
/// from its retained trusted texts (empty for an environment without them,
/// and for a source set the loader rejects: the fail-closed direction).
pub(crate) fn declared_no_return(contracts: &ResourceContracts) -> BTreeSet<String> {
    // A source that does not spell the clause cannot declare it.
    if !contracts
        .source_assumptions()
        .values()
        .any(|source| source.contains("resource_no_return"))
    {
        return BTreeSet::new();
    }
    qcore_contract::contract_loader::declared_no_return_callees(contracts).unwrap_or_default()
}

/// The attribute kinds that make a function declaration `noreturn`.
const NO_RETURN_ATTRIBUTES: [&str; 3] = ["NoReturnAttr", "C11NoReturnAttr", "CXX11NoReturnAttr"];

/// Whether a `FunctionDecl` node carries a `noreturn` attribute.
pub(crate) fn declares_no_return(node: &Value) -> bool {
    kind(node) == "FunctionDecl"
        && children(node)
            .iter()
            .any(|child| NO_RETURN_ATTRIBUTES.contains(&kind(child)))
}

/// Whether a function type spelling carries the `noreturn` type attribute of
/// the function itself. A spelling that holds a pointer declarator is not
/// read (the attribute could belong to a returned function pointer type).
fn type_is_no_return(spelling: &str) -> bool {
    spelling.ends_with(" __attribute__((noreturn))") && !spelling.contains("(*")
}

/// [`Resolution`] of every reference to `callee` in `function` (a Clang
/// `FunctionDecl` with its body). `no_return_declarations` holds the ids of the
/// unit's function declarations carrying a `noreturn` attribute; block-scope
/// declarations in the body are read here.
pub(crate) fn resolve(
    function: &Value,
    callee: &str,
    no_return_declarations: &BTreeSet<String>,
) -> Resolution {
    let mut local = BTreeSet::new();
    let mut references = Vec::new();
    let mut shadowed = false;
    walk(function, &mut |node| match kind(node) {
        "VarDecl" | "ParmVarDecl" if node["name"].as_str() == Some(callee) => shadowed = true,
        "FunctionDecl" if declares_no_return(node) => {
            if let Some(id) = node["id"].as_str() {
                local.insert(id.to_owned());
            }
        }
        "DeclRefExpr" if node["referencedDecl"]["name"].as_str() == Some(callee) => {
            references.push(node["referencedDecl"].clone());
        }
        _ => {}
    });
    if shadowed || references.is_empty() {
        return Resolution::Unresolved;
    }
    let mut answers = BTreeSet::new();
    for declaration in &references {
        if declaration["kind"].as_str() != Some("FunctionDecl") {
            return Resolution::Unresolved;
        }
        let by_type = declaration["type"]["qualType"]
            .as_str()
            .is_some_and(type_is_no_return);
        let by_attribute = declaration["id"]
            .as_str()
            .is_some_and(|id| no_return_declarations.contains(id) || local.contains(id));
        answers.insert(by_type || by_attribute);
    }
    match answers.into_iter().collect::<Vec<_>>().as_slice() {
        [true] => Resolution::NoReturn,
        [false] => Resolution::Returns,
        _ => Resolution::Unresolved,
    }
}

fn walk<'a>(node: &'a Value, visit: &mut impl FnMut(&'a Value)) {
    visit(node);
    for child in children(node) {
        walk(child, visit);
    }
}

/// A statement form route G bounds explicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Boundary {
    /// `asm` / `__asm__` statement, or an `asm` label or register binding.
    InlineAssembly,
    /// `asm goto`: assembly with label edges.
    AssemblyGoto,
    /// `__try` / `__except` / `__finally` / `__leave`.
    StructuredExceptions,
    /// `__attribute__((cleanup(f)))` on a declaration.
    CleanupAttribute,
    /// `goto *E;`: a computed goto, whose target set is not read.
    ComputedGoto,
    /// `&&label`: a label's address (GNU), the operand of a computed goto.
    LabelAddress,
}

impl Boundary {
    /// The named refusal.
    pub(crate) fn description(self) -> &'static str {
        match self {
            Boundary::InlineAssembly => {
                "inline assembly is an opaque boundary: its clobbers and memory effects are not \
                 modelled"
            }
            Boundary::AssemblyGoto => {
                "`asm goto` is an opaque boundary: its clobbers, memory effects and label edges \
                 are not modelled"
            }
            Boundary::StructuredExceptions => {
                "structured exception handling (`__try`/`__except`/`__finally`) is not modelled"
            }
            Boundary::CleanupAttribute => {
                "a `cleanup` attribute's call at every exit of its scope is not modelled"
            }
            Boundary::ComputedGoto => "a computed `goto *` has no modelled target set",
            Boundary::LabelAddress => {
                "a label's address (`&&label`) is the operand of a computed goto, which is not \
                 modelled"
            }
        }
    }
}

/// `BOUNDARIES` = `QCore.CallExits.boundaryTable`: every boundary, and
/// whether route G admits it (never).
#[cfg(test)]
pub(crate) const BOUNDARIES: [(Boundary, bool); 6] = [
    (Boundary::InlineAssembly, false),
    (Boundary::AssemblyGoto, false),
    (Boundary::StructuredExceptions, false),
    (Boundary::CleanupAttribute, false),
    (Boundary::ComputedGoto, false),
    (Boundary::LabelAddress, false),
];

const ASSEMBLY: [&str; 3] = ["asm", "__asm__", "__asm"];
const STRUCTURED_EXCEPTIONS: [&str; 4] = ["__try", "__except", "__finally", "__leave"];

/// The boundary a statement's tokens hold, if any. Read before any route.
pub(crate) fn statement_boundary(texts: &[&str]) -> Option<Boundary> {
    if let Some(at) = texts.iter().position(|text| ASSEMBLY.contains(text)) {
        // `asm goto (` / `asm volatile goto (` / `asm inline goto (`.
        let goto = texts[at + 1..]
            .iter()
            .take_while(|text| **text != "(")
            .any(|text| *text == "goto");
        return Some(if goto {
            Boundary::AssemblyGoto
        } else {
            Boundary::InlineAssembly
        });
    }
    if texts
        .iter()
        .any(|text| STRUCTURED_EXCEPTIONS.contains(text))
    {
        return Some(Boundary::StructuredExceptions);
    }
    // `&&label` (the token stream spells it `&`, `&`, `label`) where an
    // operand starts: at the start, or after an operator or an opening token
    // (a binary `&&` follows an operand; `& &x` is no C operand).
    if texts.windows(3).enumerate().any(|(at, triple)| {
        triple[0] == "&"
            && triple[1] == "&"
            && is_identifier(triple[2])
            && (at == 0
                || matches!(
                    texts[at - 1],
                    "=" | "(" | "," | "{" | "?" | ":" | "return" | "[" | "!"
                ))
    }) {
        return Some(Boundary::LabelAddress);
    }
    attribute_lists(texts)
        .any(|list| {
            list.windows(2)
                .any(|pair| matches!(pair[0], "cleanup" | "__cleanup__") && pair[1] == "(")
        })
        .then_some(Boundary::CleanupAttribute)
}

fn is_identifier(text: &str) -> bool {
    text.chars()
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && text.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
}

/// The token lists inside every `__attribute__ (( … ))` of a statement (an
/// unterminated one runs to the end, which only widens the search).
fn attribute_lists<'a>(texts: &'a [&'a str]) -> impl Iterator<Item = &'a [&'a str]> {
    texts
        .iter()
        .enumerate()
        .filter(|(_, text)| matches!(**text, "__attribute__" | "__attribute"))
        .map(|(at, _)| {
            let mut depth = 0usize;
            let mut end = texts.len();
            for (offset, text) in texts[at + 1..].iter().enumerate() {
                match *text {
                    "(" => depth += 1,
                    ")" => {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            end = at + 1 + offset;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            &texts[at + 1..end]
        })
}

/// The named refusal of an uncontracted call to a name no v1 declaration may
/// carry: an extension node, a returns-twice call or a non-local exit.
pub(crate) fn undeclarable_call(function: &str) -> Option<String> {
    qcore_contract::contract_loader::undeclarable_c_name(function).map(|reason| {
        format!("`{function}` is {reason}, which no resource contract or route models")
    })
}

/// The named refusal of a declared no-return call the Clang declaration does
/// not establish.
pub(crate) fn unestablished_description(function: &str, resolution: Resolution) -> String {
    let why = match resolution {
        Resolution::Returns => "its declaration here is not `noreturn`",
        Resolution::NoReturn | Resolution::Unresolved => {
            "its Clang declaration here is not resolved"
        }
    };
    format!("no-return contract of `{function}` not established at this call: {why}")
}

#[cfg(test)]
#[path = "../tests/unit/call_exits.rs"]
mod tests;
