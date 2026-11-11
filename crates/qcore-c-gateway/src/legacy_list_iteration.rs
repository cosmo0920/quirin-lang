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

//! Card **W45-FB-BACKLOG-CUSTODY**: intrusive circular-list iteration and
//! element identity (gateway constraint 5l).
//!
//! # The refusals this replaces
//!
//! The Monkey and CFL list macros expand, before the gateway reads them, to
//!
//! ```text
//! for (C = (H)->next; C != (H); C = C->next)                       plain
//! for (C = (H)->next, N = C->next; C != (H); C = N, N = C->next)   safe
//! E = ({ const typeof(((T *)0)->F) *__mptr = (P);
//!        (T *)((char *)__mptr - __builtin_offsetof(T, F)); })         element
//! ```
//!
//! (and the `prev` forms). The header's init and increment read a pointer out
//! of list memory and were refused as indirect accesses; a head spelled
//! `&B->F` refused the condition; the element read is a GNU statement
//! expression and was refused as a braced expression. Each refusal poisoned
//! the reaching bindings, so every `mk_list_foreach` body was undecidable.
//!
//! # The rule
//!
//! 1. **Header** ([`recognise_header`]). Recognised on the header's
//!    **original** spelling, groups intact (`parenthesised_callee::for_clauses`),
//!    never on the stripped clauses rule 10c(3) distrusts. `C` and `N` are
//!    distinct plain names; both occurrences of `H` are the same token run,
//!    `& R -> L1 … -> F`, `& S . F` or a plain name `R`, whose root is
//!    declared a record pointer (or record object) and whose links are typed
//!    record pointers (`indirect_chain_access::parse_chain`).
//! 2. **Lowering.** The init is the typed read of `H`'s root (and chain), the
//!    condition is a comparison of two pointers (both successors, the exit
//!    edge kept), and every cursor assignment rebinds its name to list memory
//!    this function does not track: an `IndirectScalarAccess` whose
//!    destination is the cursor, so the name is unbound on every path and
//!    every read through it passes the ordinary dereference obligation.
//! 3. **Element** ([`element_binding`]). The statement expression above, with
//!    `P` an active cursor, a `(H)->next`/`(H)->prev` read of an admissible
//!    head, or a name this function never binds, binds `E` the same way: `E`
//!    is untracked memory, its reads keep every existing obligation, and a
//!    custody use of it (a release, an escape into an owner) finds no binding
//!    and refuses.
//! 4. **What the kinds admit** ([`Parser::list_iteration_refusal`]). In a
//!    *plain* iteration the step reads `C->next` after the body, so the body
//!    may not hand the current element's link node (`C`, `&E->F`) to any
//!    call, nor `C` or an element bound from it to a position its contract in
//!    force does not declare `Borrowed`: such a statement is refused with
//!    [`PLAIN_REFUSAL`]. A *safe* iteration saved `N` before the body, so the
//!    same restriction protects `N` and its elements instead ([`SAFE_REFUSAL`])
//!    and the current element may be unlinked and released.
//!
//! # Premises (stated, not proved; `QCore.ListIteration` models the rule)
//!
//! * **P-ILI-BODY.** The body does not write, increment or take the address
//!   of `C` or `N` (checked lexically; a body that does is declined).
//! * **P-ILI-UNTRACKED.** An element reached through an admitted iteration
//!   is not an identity this body tracks. The lexical guard declines every
//!   iteration of a function that hands a call the member address `&X->…`
//!   or `&X.…` of a name `X` it may bind to a tracked identity (the spelling
//!   of linking a tracked object into a list); a link through a wrapper call
//!   is the `Borrowed` declaration's shallow premise (P-B08-SHALLOW), and the
//!   liveness of a published object is `MK-LIST-ADD-BORROWED-PUBLICATION`'s.
//! * **P-ILI-CALLEE-FRAME.** A call in the body that receives the container
//!   (not the protected node) does not unlink the current element of a
//!   plain iteration or the saved successor of a safe one.
//! * **P-ILI-BRACED.** A plain iteration's body is a braced block (an
//!   unbraced body is declined).
//!
//! Only the header and element shapes above are admitted; every other shape
//! keeps its refusal byte for byte, and rule 4 only replaces an admission by
//! a refusal.

use std::collections::BTreeSet;

use qcore_schemas::contract::{ResourceContracts, ResourceParameterMode};

use super::{LegacyInstruction, Token};
use crate::borrowed_actual::UntrackedBases;
use crate::indirect_chain_access::AccessChain;
use crate::typed_field_access::{DeclaredType, FunctionTypes, TypedAccessModel, is_name};

/// Rule 4, plain form.
pub(crate) const PLAIN_REFUSAL: &str = "the current element of an intrusive-list iteration that \
                                        does not save its successor is unlinked or released";

/// Rule 4, safe form.
pub(crate) const SAFE_REFUSAL: &str =
    "the saved successor of an intrusive-list iteration is unlinked or released";

/// Which iteration a header spells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IterationKind {
    /// `C = C->next` after the body.
    Plain,
    /// `N` saved before the body; `C = N, N = C->next` after it.
    Safe,
}

/// A recognised header, before typing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HeaderShape {
    pub(crate) kind: IterationKind,
    pub(crate) cursor: String,
    pub(crate) successor: Option<String>,
    /// `next` or `prev`.
    pub(crate) direction: String,
    /// The head's token texts, without the macro's grouping parentheses.
    pub(crate) head: Vec<String>,
}

/// The typed read of a head `H` (or of the list memory behind it).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct HeadRead {
    pub(crate) bases: Vec<String>,
    pub(crate) chains: Vec<AccessChain>,
}

/// One active iteration while its body is classified.
#[derive(Clone, Debug)]
pub(crate) struct Frame {
    pub(crate) kind: IterationKind,
    pub(crate) cursor: String,
    pub(crate) successor: Option<String>,
    /// The protected pointer (`C` for plain, `N` for safe) and every element
    /// name bound from it so far.
    pub(crate) protected: BTreeSet<String>,
    /// The protected link nodes, spelled `& E -> F` / `& E . F`.
    pub(crate) links: BTreeSet<Vec<String>>,
    /// Card W45-CONTAINER-ROOT-REACH: the head's token texts, for the
    /// container an element of this iteration is reached through.
    pub(crate) head: Vec<String>,
}

impl Frame {
    fn protected_cursor(&self) -> &str {
        match self.kind {
            IterationKind::Plain => &self.cursor,
            IterationKind::Safe => self.successor.as_deref().unwrap_or(&self.cursor),
        }
    }
}

/// Function-level facts read once per body.
#[derive(Clone, Debug, Default)]
pub(crate) struct FunctionFacts {
    /// P-ILI-UNTRACKED's lexical guard held: no call receives `&X->…` or
    /// `&X.…` for a name `X` this function may bind to a tracked identity.
    pub(crate) guard_holds: bool,
    /// Every name the body writes, steps or takes the whole address of,
    /// wherever it does (a `for` header included).
    pub(crate) written: BTreeSet<String>,
}

fn texts(tokens: &[Token]) -> Vec<&str> {
    tokens.iter().map(|token| token.text.as_str()).collect()
}

fn owned(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|text| (*text).to_owned()).collect()
}

/// The index of the `)` closing the `(` at `open`.
fn close_of(texts: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(open) {
        match *text {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return (*text == ")").then_some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// Split at top-level commas.
fn top_level_commas<'a, 'b>(texts: &'b [&'a str]) -> Vec<&'b [&'a str]> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (index, text) in texts.iter().enumerate() {
        match *text {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.saturating_sub(1),
            "," if depth == 0 => {
                parts.push(&texts[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(&texts[start..]);
    parts
}

/// `NAME = ( H ) -> DIR` → `(H, DIR)`.
fn head_step<'a, 'b>(clause: &'b [&'a str], cursor: &str) -> Option<(&'b [&'a str], &'a str)> {
    if clause.len() < 7 || clause[0] != cursor || clause[1] != "=" || clause[2] != "(" {
        return None;
    }
    let close = close_of(clause, 2)?;
    let rest = &clause[close + 1..];
    match rest {
        ["-", ">", direction] if matches!(*direction, "next" | "prev") => {
            Some((&clause[3..close], *direction))
        }
        _ => None,
    }
}

/// `DEST = SOURCE -> DIR`.
fn cursor_step(clause: &[&str], destination: &str, source: &str, direction: &str) -> bool {
    clause == [destination, "=", source, "-", ">", direction]
}

/// Rule 1's lexical half: the three clauses of a `for` header, groups intact.
pub(crate) fn recognise_header(clauses: &[Token]) -> Option<HeaderShape> {
    let [init, condition, increment] = super::parenthesised_callee::for_clauses(clauses)?;
    recognise_clauses(&texts(&init), &texts(&condition), &texts(&increment))
}

/// The header's three clauses split at top-level `;`, groups intact.
fn clause_texts<'a, 'b>(header: &'b [&'a str]) -> Option<[&'b [&'a str]; 3]> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (index, text) in header.iter().enumerate() {
        match *text {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.checked_sub(1)?,
            ";" if depth == 0 => {
                parts.push(&header[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(&header[start..]);
    parts.try_into().ok()
}

fn recognise_clauses(init: &[&str], condition: &[&str], increment: &[&str]) -> Option<HeaderShape> {
    let init_parts = top_level_commas(init);
    let increment_parts = top_level_commas(increment);
    let cursor = *init.first()?;
    if !is_name(cursor) {
        return None;
    }
    let (head, direction) = head_step(init_parts[0], cursor)?;
    if head.is_empty() {
        return None;
    }
    // The condition is `C != ( H )`.
    let mut expected_condition = vec![cursor, "!", "=", "("];
    expected_condition.extend_from_slice(head);
    expected_condition.push(")");
    if condition != expected_condition {
        return None;
    }
    let (kind, successor) = match (init_parts.len(), increment_parts.len()) {
        (1, 1) if cursor_step(increment_parts[0], cursor, cursor, direction) => {
            (IterationKind::Plain, None)
        }
        (2, 2) => {
            let successor = *init_parts[1].first()?;
            if !is_name(successor)
                || successor == cursor
                || !cursor_step(init_parts[1], successor, cursor, direction)
                || increment_parts[0] != [cursor, "=", successor]
                || !cursor_step(increment_parts[1], successor, cursor, direction)
            {
                return None;
            }
            (IterationKind::Safe, Some(successor.to_owned()))
        }
        _ => return None,
    };
    if head.contains(&cursor)
        || successor
            .as_deref()
            .is_some_and(|name| head.contains(&name))
    {
        return None;
    }
    Some(HeaderShape {
        kind,
        cursor: cursor.to_owned(),
        successor,
        direction: direction.to_owned(),
        head: owned(head),
    })
}

/// The typed read of an admissible head spelling: `R`, `& R -> … -> F` or
/// `& S . F`. `None` for every other spelling or an untyped root or link.
pub(crate) fn head_read(
    head: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
    bases: &UntrackedBases,
) -> Option<HeadRead> {
    match head {
        [root] if is_name(root) => matches!(
            types.declared_type(root),
            Some(DeclaredType::RecordPointer(_))
        )
        .then(|| HeadRead {
            bases: vec![(*root).to_owned()],
            chains: Vec::new(),
        }),
        ["&", root, ".", member] if is_name(root) && is_name(member) => {
            matches!(types.declared_type(root), Some(DeclaredType::Record(_)))
                .then(HeadRead::default)
        }
        ["&", root, "-", ">", member] if is_name(root) && is_name(member) => matches!(
            types.declared_type(root),
            Some(DeclaredType::RecordPointer(_))
        )
        .then(|| HeadRead {
            bases: vec![(*root).to_owned()],
            chains: Vec::new(),
        }),
        ["&", place @ ..] => {
            let chain = crate::indirect_chain_access::parse_chain(place, 0, model, types).ok()?;
            if chain.end != place.len() {
                return None;
            }
            let mut read = HeadRead::default();
            if chain.dereferenced_root {
                read.bases.push(chain.root.clone());
            }
            read.chains.push(AccessChain {
                root_never_bound: bases.never_bound(&chain.root),
                root_bound_untracked: false,
                root_untracked_only: false,
                root: chain.root,
                dereferenced_root: chain.dereferenced_root,
                links: chain.links,
                member: chain.member,
            });
            Some(read)
        }
        _ => None,
    }
}

/// The parts of an element statement expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ElementShape {
    pub(crate) destination: String,
    /// The pointer operand `P`, groups intact.
    pub(crate) pointer: Vec<String>,
    /// The link member `F`.
    pub(crate) member: String,
}

/// Skip a declaration prefix (`struct T *`, `T *`, qualifiers) and return the
/// index of the destination name, which is followed by `=`.
fn destination_index(texts: &[&str]) -> Option<usize> {
    let equal = texts.iter().position(|text| *text == "=")?;
    let destination = equal.checked_sub(1)?;
    if !is_name(texts[destination]) {
        return None;
    }
    let prefix = &texts[..destination];
    let declarator = prefix.iter().all(|text| *text == "*" || is_type_word(text));
    declarator.then_some(destination)
}

/// A word of a type name: an identifier or a keyword a type name may spell.
fn is_type_word(text: &str) -> bool {
    is_name(text)
        || matches!(
            text,
            "struct"
                | "union"
                | "enum"
                | "const"
                | "volatile"
                | "void"
                | "char"
                | "short"
                | "int"
                | "long"
                | "signed"
                | "unsigned"
        )
}

fn is_typeof(text: &str) -> bool {
    matches!(text, "typeof" | "__typeof__" | "__typeof")
}

/// `( ( T * ) 0 ) -> F` at `start`: the type tokens and the member.
fn null_member<'a>(texts: &[&'a str], start: usize) -> Option<(Vec<&'a str>, &'a str, usize)> {
    // ( ( T * ) 0 ) -> F
    if texts.get(start) != Some(&"(") || texts.get(start + 1) != Some(&"(") {
        return None;
    }
    let type_close = close_of(texts, start + 1)?;
    let type_tokens = &texts[start + 2..type_close];
    let (last, ty) = type_tokens.split_last()?;
    if *last != "*" || ty.is_empty() || !ty.iter().all(|text| is_type_word(text)) {
        return None;
    }
    if texts.get(type_close + 1) != Some(&"0") || texts.get(type_close + 2) != Some(&")") {
        return None;
    }
    let arrow = type_close + 3;
    if texts.get(arrow) != Some(&"-") || texts.get(arrow + 1) != Some(&">") {
        return None;
    }
    let member = *texts.get(arrow + 2)?;
    is_name(member).then(|| (ty.to_vec(), member, arrow + 3))
}

/// Rule 3's lexical half: `[DECL] E = ({ const? typeof(((T *)0)->F) *M =
/// (P); (T *)((char *)M - OFFSET); })`, with `OFFSET` either
/// `__builtin_offsetof(T, F)` or `((SIZE) &((T *)0)->F)`.
pub(crate) fn element_shape(tokens: &[&str]) -> Option<ElementShape> {
    let destination = destination_index(tokens)?;
    let at = destination + 2;
    let body = &tokens[at..];
    // ( { ... } )
    if body.first() != Some(&"(") || body.get(1) != Some(&"{") {
        return None;
    }
    if body.len() < 4 || body[body.len() - 2] != "}" || body[body.len() - 1] != ")" {
        return None;
    }
    let inner = &body[2..body.len() - 2];
    let mut index = 0usize;
    if inner.first() == Some(&"const") {
        index += 1;
    }
    if !inner.get(index).is_some_and(|text| is_typeof(text)) || inner.get(index + 1) != Some(&"(") {
        return None;
    }
    let typeof_close = close_of(inner, index + 1)?;
    let (ty, member, after) = null_member(inner, index + 2)?;
    if after != typeof_close {
        return None;
    }
    // * M = ( P ) ;
    let star = typeof_close + 1;
    if inner.get(star) != Some(&"*") {
        return None;
    }
    let temporary = *inner.get(star + 1)?;
    if !is_name(temporary) || inner.get(star + 2) != Some(&"=") || inner.get(star + 3) != Some(&"(")
    {
        return None;
    }
    let pointer_close = close_of(inner, star + 3)?;
    let pointer = &inner[star + 4..pointer_close];
    if pointer.is_empty() || inner.get(pointer_close + 1) != Some(&";") {
        return None;
    }
    // ( T * ) ( ( char * ) M - OFFSET ) ;
    let result = &inner[pointer_close + 2..];
    let mut expected_cast = vec!["("];
    expected_cast.extend_from_slice(&ty);
    expected_cast.extend_from_slice(&["*", ")", "(", "(", "char", "*", ")", temporary, "-"]);
    if !result.starts_with(&expected_cast) {
        return None;
    }
    let offset = &result[expected_cast.len()..];
    let (offset, tail) = match offset {
        ["__builtin_offsetof", "(", ..] => {
            let close = close_of(offset, 1)?;
            let mut expected = ty.clone();
            expected.extend_from_slice(&[",", member]);
            (offset[2..close] == expected[..], &offset[close + 1..])
        }
        ["(", "(", ..] => {
            let close = close_of(offset, 0)?;
            let inner_offset = &offset[1..close];
            let size_close = close_of(inner_offset, 0)?;
            let size = &inner_offset[1..size_close];
            let ok = !size.is_empty()
                && size.iter().all(|text| is_type_word(text))
                && inner_offset.get(size_close + 1) == Some(&"&")
                && null_member(inner_offset, size_close + 2)
                    .is_some_and(|(t, m, end)| t == ty && m == member && end == inner_offset.len());
            (ok, &offset[close + 1..])
        }
        _ => return None,
    };
    if !offset || tail != [")", ";"] {
        return None;
    }
    Some(ElementShape {
        destination: tokens[destination].to_owned(),
        pointer: owned(pointer),
        member: member.to_owned(),
    })
}

/// The criterion-2 declaration blocker: every element statement expression
/// `( { … } )` in `body`, as the inclusive index range of its outer
/// parentheses. The declaration reader skips the one declaration inside
/// each (`const typeof(…) *__mptr = (P);`, scoped to the expression and
/// read by nothing), where it used to refuse the whole function. Only a run
/// [`element_shape`] reads exactly is listed; every other statement
/// expression keeps the refusal.
pub(crate) fn element_expression_spans(body: &[&str]) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    for index in 0..body.len() {
        if body[index] != "(" || body.get(index + 1) != Some(&"{") {
            continue;
        }
        let Some(close) = close_of(body, index) else {
            continue;
        };
        let mut statement = vec!["__element", "="];
        statement.extend_from_slice(&body[index..=close]);
        if element_shape(&statement).is_some() {
            spans.push((index, close));
        }
    }
    spans
}

/// A call site: callee index and its argument spans (half-open, absolute).
fn call_sites(texts: &[&str]) -> Vec<(usize, Vec<(usize, usize)>)> {
    let mut sites = Vec::new();
    for index in 0..texts.len() {
        let text = texts[index];
        if !is_name(text)
            || texts.get(index + 1) != Some(&"(")
            || matches!(
                text,
                "sizeof"
                    | "typeof"
                    | "__typeof__"
                    | "__typeof"
                    | "_Alignof"
                    | "__builtin_offsetof"
                    | "__extension__"
            )
        {
            continue;
        }
        let Some(close) = close_of(texts, index + 1) else {
            continue;
        };
        let mut spans = Vec::new();
        let mut start = index + 2;
        let mut depth = 0usize;
        for (at, text) in texts.iter().enumerate().take(close).skip(index + 2) {
            match *text {
                "(" | "[" | "{" => depth += 1,
                ")" | "]" | "}" => depth = depth.saturating_sub(1),
                "," if depth == 0 => {
                    spans.push((start, at));
                    start = at + 1;
                }
                _ => {}
            }
        }
        if start < close {
            spans.push((start, close));
        }
        sites.push((index, spans));
    }
    sites
}

/// Strip grouping parentheses around a whole argument.
fn strip_groups<'a, 'b>(mut argument: &'b [&'a str]) -> &'b [&'a str] {
    while argument.len() >= 2
        && argument[0] == "("
        && close_of(argument, 0) == Some(argument.len() - 1)
    {
        argument = &argument[1..argument.len() - 1];
    }
    argument
}

/// Names an argument mentions as a place root.
fn root_names<'a>(argument: &[&'a str]) -> Vec<&'a str> {
    let mut names = Vec::new();
    for index in 0..argument.len() {
        let text = argument[index];
        if !is_name(text) {
            continue;
        }
        match argument.get(index + 1).copied() {
            Some("(") | Some(".") => continue,
            Some("-") if argument.get(index + 2) == Some(&">") => continue,
            _ => {}
        }
        if index > 0 && matches!(argument[index - 1], "." | ">") {
            continue;
        }
        names.push(text);
    }
    names
}

/// P-ILI-UNTRACKED's lexical guard and the element-only names, over one body.
pub(crate) fn function_facts(body: &[&str], bases: &UntrackedBases) -> FunctionFacts {
    // Element-only names: every `X =` assignment of them is `X = ( {`.
    let mut element = BTreeSet::new();
    let mut other = BTreeSet::new();
    for index in 0..body.len() {
        let text = body[index];
        if !is_name(text) {
            continue;
        }
        let next = body.get(index + 1).copied();
        let after = body.get(index + 2).copied();
        if next == Some("=") && after != Some("=") {
            let previous = index.checked_sub(1).map(|at| body[at]);
            if matches!(previous, Some("." | ">")) {
                continue;
            }
            if after == Some("(") && body.get(index + 3) == Some(&"{") {
                element.insert(text.to_owned());
            } else {
                other.insert(text.to_owned());
            }
        } else if index > 0 && body[index - 1] == "&" {
            let continues = matches!(next, Some("-") if after == Some(">"))
                || matches!(next, Some(".") | Some("["));
            if !continues {
                other.insert(text.to_owned());
            }
        }
    }
    let element_names: BTreeSet<String> = element.difference(&other).cloned().collect();
    // The head spelling of every list iteration header of the body.
    let mut heads: BTreeSet<Vec<&str>> = BTreeSet::new();
    for index in 0..body.len() {
        if body[index] != "for" || body.get(index + 1) != Some(&"(") {
            continue;
        }
        let Some(close) = close_of(body, index + 1) else {
            continue;
        };
        if let Some([init, condition, increment]) = clause_texts(&body[index + 2..close])
            && let Some(shape) = recognise_clauses(init, condition, increment)
        {
            let head = shape.head.clone();
            if let Some(found) = body[index + 2..close]
                .windows(head.len())
                .find(|window| window.iter().copied().eq(head.iter().map(String::as_str)))
            {
                heads.insert(found.to_vec());
            }
        }
    }
    // P-ILI-UNTRACKED's guard: no call receives both the member address of a
    // name that may hold a tracked identity and the head of a list the body
    // iterates — the spelling of linking a tracked object into that list.
    let mut guard_holds = true;
    for (_, spans) in call_sites(body) {
        let arguments = spans
            .iter()
            .map(|(start, end)| strip_groups(&body[*start..*end]))
            .collect::<Vec<_>>();
        // The linked object and the list are two different actuals: a call
        // handed only the head (`mk_list_size(&ctx->backlogs)`) links
        // nothing into it.
        let links_tracked = |argument: &[&str]| {
            matches!(argument, ["&", root, rest @ ..]
                if is_name(root)
                    && matches!(rest.first(), Some(&"-") | Some(&"."))
                    && !bases.never_bound(root)
                    && !element_names.contains(*root))
        };
        let names_iterated_head =
            |argument: &[&str]| heads.iter().any(|head| head.as_slice() == argument);
        for (linked, object) in arguments.iter().enumerate() {
            if links_tracked(object)
                && arguments
                    .iter()
                    .enumerate()
                    .any(|(other, head)| other != linked && names_iterated_head(head))
            {
                guard_holds = false;
            }
        }
    }
    let written = (0..body.len())
        .filter(|index| is_name(body[*index]) && written_at(body, *index))
        .map(|index| body[index].to_owned())
        .collect();
    FunctionFacts {
        guard_holds,
        written,
    }
}

/// P-ILI-BODY: the body writes, steps or takes the address of none of `names`.
pub(crate) fn body_leaves_cursors(body: &[&str], names: &[&str]) -> bool {
    (0..body.len()).all(|index| !names.contains(&body[index]) || !written_at(body, index))
}

/// Whether the name at `index` is written (`x =`, `x op=`), stepped or has
/// its whole address taken there. Over-approximates: `x <= y` counts.
fn written_at(body: &[&str], index: usize) -> bool {
    let next = body.get(index + 1).copied();
    let after = body.get(index + 2).copied();
    let previous = index.checked_sub(1).map(|at| body[at]);
    let before = index.checked_sub(2).map(|at| body[at]);
    if matches!(previous, Some("." | ">")) {
        return false;
    }
    let assigned = next == Some("=") && after != Some("=");
    let compound = matches!(
        next,
        Some("+" | "-" | "*" | "/" | "%" | "&" | "|" | "^" | "<" | ">")
    ) && (after == Some("=")
        || matches!(
            (next, after),
            (Some("<"), Some("<")) | (Some(">"), Some(">"))
        ));
    let stepped = matches!(
        (next, after),
        (Some("+"), Some("+")) | (Some("-"), Some("-"))
    ) || matches!(
        (before, previous),
        (Some("+"), Some("+")) | (Some("-"), Some("-"))
    );
    let continues =
        matches!(next, Some("-") if after == Some(">")) || matches!(next, Some(".") | Some("["));
    let addressed = previous == Some("&") && !continues;
    assigned || compound || stepped || addressed
}

/// Rule 4 over one statement's texts: the description of the refusal, if
/// the statement hands a frame's protected node or element to a position it
/// may unlink or release through.
pub(crate) fn frame_refusal(
    texts: &[&str],
    frames: &[Frame],
    contracts: &ResourceContracts,
    variadic_tails: &BTreeSet<String>,
) -> Option<&'static str> {
    if frames.is_empty() {
        return None;
    }
    for (callee_index, spans) in call_sites(texts) {
        let callee = texts[callee_index];
        let contract = contracts.contract(callee);
        for (position, (start, end)) in spans.iter().copied().enumerate() {
            let argument = strip_groups(&texts[start..end]);
            let borrowed =
                contract.is_some_and(|contract| match contract.parameters.get(position) {
                    Some(ResourceParameterMode::Borrowed) => true,
                    Some(_) => false,
                    None => variadic_tails.contains(callee),
                });
            for frame in frames {
                let description = match frame.kind {
                    IterationKind::Plain => PLAIN_REFUSAL,
                    IterationKind::Safe => SAFE_REFUSAL,
                };
                let node = argument == [frame.protected_cursor()]
                    || frame
                        .links
                        .iter()
                        .any(|link| link.iter().map(String::as_str).eq(argument.iter().copied()));
                if node {
                    return Some(description);
                }
                if !borrowed
                    && root_names(argument)
                        .iter()
                        .any(|name| frame.protected.contains(*name))
                {
                    return Some(description);
                }
            }
        }
    }
    None
}

impl super::Parser<'_> {
    /// Rules 1 and 2: the whole `for` statement whose header `clauses` spell
    /// an admissible list iteration, with its body parsed under the
    /// iteration's frame. `Ok(None)` leaves the header to the ordinary
    /// lowering, byte for byte.
    pub(super) fn list_iteration_statement(
        &mut self,
        clauses: &[Token],
        line: usize,
    ) -> Result<Option<super::Statement>, usize> {
        let Some(shape) = recognise_header(clauses) else {
            return Ok(None);
        };
        if !self.list_facts.guard_holds {
            return Ok(None);
        }
        let head = shape.head.iter().map(String::as_str).collect::<Vec<_>>();
        let Some(read) = head_read(
            &head,
            &self.typed_model,
            &self.typed_types,
            &self.typed_bases,
        ) else {
            return Ok(None);
        };
        // P-ILI-BRACED and P-ILI-BODY.
        if self.text() != Some("{") {
            return Ok(None);
        }
        let Some(body_end) = super::matching(self.tokens, self.cursor, "{", "}") else {
            return Ok(None);
        };
        let mut cursors = vec![shape.cursor.as_str()];
        cursors.extend(shape.successor.as_deref());
        if cursors
            .iter()
            .any(|name| self.formal_parameters.contains(*name))
        {
            return Ok(None);
        }
        let body = texts(&self.tokens[self.cursor..=body_end]);
        if !body_leaves_cursors(&body, &cursors) {
            return Ok(None);
        }
        let read_access = |parser: &Self, bases: Vec<String>, chains, destination: &str| {
            LegacyInstruction::IndirectScalarAccess {
                typed_alternative: None,
                alias_reads: bases
                    .iter()
                    .filter_map(|base| parser.read_alias(Some(base)))
                    .collect(),
                bases,
                destination: Some(destination.to_owned()),
                frame: None,
                chains,
                borrowed_destination: false,
                readback: None,
            }
        };
        let cursor = shape.cursor.clone();
        let head_init = read_access(self, read.bases, read.chains, &cursor);
        let (prefix, init, increment, list_step) = match &shape.successor {
            None => (
                None,
                head_init,
                read_access(self, vec![cursor.clone()], Vec::new(), &cursor),
                None,
            ),
            Some(successor) => (
                Some(head_init),
                read_access(self, vec![cursor.clone()], Vec::new(), successor),
                // `C = N`: `N` is unbound on every path (P-ILI-BODY), so the
                // copy reads nothing and leaves `C` unbound.
                read_access(self, Vec::new(), Vec::new(), &cursor),
                Some(read_access(
                    self,
                    vec![cursor.clone()],
                    Vec::new(),
                    successor,
                )),
            ),
        };
        let protected = match shape.kind {
            IterationKind::Plain => cursor.clone(),
            IterationKind::Safe => shape.successor.clone().unwrap_or_else(|| cursor.clone()),
        };
        self.list_frames.push(Frame {
            kind: shape.kind,
            cursor: cursor.clone(),
            successor: shape.successor.clone(),
            protected: BTreeSet::from([protected]),
            links: BTreeSet::new(),
            head: shape.head.clone(),
        });
        let body = self.parse_statement_body();
        self.list_frames.pop();
        let body = body?;
        let statement = super::Statement::For {
            line,
            counted: None,
            // The condition is supported, so its refusal frame is never read.
            writes: super::RefusedWrites::Unbounded,
            init: Some(Box::new((init, line))),
            condition_always_true: false,
            condition_supported: true,
            condition_call: None,
            condition_access: None,
            null_condition: None,
            condition_plan: None,
            increment: Some(Box::new((increment, line))),
            list_step: list_step.map(|step| Box::new((step, line))),
            body,
        };
        Ok(Some(match prefix {
            None => statement,
            Some(prefix) => super::Statement::Block(vec![
                super::Statement::Instruction(prefix, line),
                statement,
            ]),
        }))
    }

    /// Rule 3: an element statement expression over an active cursor, an
    /// admissible head's first or last node, or a never-bound name.
    pub(super) fn list_element_binding(&mut self, texts: &[&str]) -> Option<LegacyInstruction> {
        let shape = element_shape(texts)?;
        // A formal's own slot is never rebound by this rule.
        if self.formal_parameters.contains(&shape.destination) {
            return None;
        }
        let pointer = shape.pointer.iter().map(String::as_str).collect::<Vec<_>>();
        // Card W45-CONTAINER-ROOT-REACH: the container this element is reached
        // through — an iteration's current cursor or a head's first or last
        // node — and nothing for a saved successor or a never-bound formal.
        let mut origin = None;
        let (bases, chains) = match pointer.as_slice() {
            [name]
                if self.list_frames.iter().any(|frame| {
                    frame.cursor == *name || frame.successor.as_deref() == Some(*name)
                }) =>
            {
                origin = self
                    .list_frames
                    .iter()
                    .rev()
                    .find(|frame| {
                        frame.cursor == *name || frame.successor.as_deref() == Some(*name)
                    })
                    .filter(|frame| frame.cursor == *name)
                    .map(|frame| frame.head.clone());
                for frame in &mut self.list_frames {
                    if frame.protected_cursor() == *name {
                        frame.protected.insert(shape.destination.clone());
                        frame.links.insert(
                            [
                                "&",
                                shape.destination.as_str(),
                                "-",
                                ">",
                                shape.member.as_str(),
                            ]
                            .iter()
                            .map(|text| (*text).to_owned())
                            .collect(),
                        );
                    }
                }
                (Vec::new(), Vec::new())
            }
            // A formal the body never writes, never binds and never
            // addresses: the callback idiom `ctx = container_of(arg, …)`.
            [name]
                if is_name(name)
                    && self.formal_parameters.contains(*name)
                    && !self.list_facts.written.contains(*name)
                    && self.typed_bases.never_bound(name) =>
            {
                // Card W45-SR5-SOUNDNESS: the element is the caller's object.
                self.element_origins.note_over_formal(&shape.destination);
                (Vec::new(), Vec::new())
            }
            ["(", ..] => {
                let close = close_of(&pointer, 0)?;
                match &pointer[close + 1..] {
                    ["-", ">", direction] if matches!(*direction, "next" | "prev") => {
                        let read = head_read(
                            &pointer[1..close],
                            &self.typed_model,
                            &self.typed_types,
                            &self.typed_bases,
                        )?;
                        origin = Some(owned(&pointer[1..close]));
                        (read.bases, read.chains)
                    }
                    _ => return None,
                }
            }
            _ => return None,
        };
        self.element_origins.propose(&shape.destination, origin);
        Some(LegacyInstruction::IndirectScalarAccess {
            typed_alternative: None,
            alias_reads: bases
                .iter()
                .filter_map(|base| self.read_alias(Some(base)))
                .collect(),
            bases,
            destination: Some(shape.destination),
            frame: None,
            chains,
            borrowed_destination: false,
            readback: None,
        })
    }

    /// Rule 4 over one classified statement.
    pub(super) fn list_iteration_refusal(&self, texts: &[&str]) -> Option<&'static str> {
        frame_refusal(
            texts,
            &self.list_frames,
            self.contracts,
            &self.variadic_tails,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(text: &str) -> Vec<Token> {
        super::super::tokenize(text)
    }

    #[test]
    fn the_plain_header_is_recognised_on_its_original_spelling() {
        let header = tokens("it = (&q->items)->next; it != (&q->items); it = it->next");
        let shape = recognise_header(&header).expect("plain header");
        assert_eq!(shape.kind, IterationKind::Plain);
        assert_eq!(shape.cursor, "it");
        assert_eq!(shape.head, ["&", "q", "-", ">", "items"]);
    }

    #[test]
    fn the_safe_header_is_recognised() {
        let header =
            tokens("curr = (head)->next, n = curr->next; curr != (head); curr = n, n = curr->next");
        let shape = recognise_header(&header).expect("safe header");
        assert_eq!(shape.kind, IterationKind::Safe);
        assert_eq!(shape.successor.as_deref(), Some("n"));
        assert_eq!(shape.head, ["head"]);
    }

    #[test]
    fn a_different_head_or_step_is_not_a_list_iteration() {
        for header in [
            "it = (&q->items)->next; it != (&q->other); it = it->next",
            "it = (&q->items)->next; it != (&q->items); it = it->prev",
            "it = (&q->items)->next; it != (&q->items); it = n",
            "it = (&q->items)->next, n = it->next; it != (&q->items); it = it->next",
            "it = q->items.next; it != &q->items; it = it->next",
            "i = 0; i < n; i++",
        ] {
            assert_eq!(recognise_header(&tokens(header)), None, "{header}");
        }
    }

    #[test]
    fn the_element_statement_expression_is_read() {
        let statement = tokens(
            "e = ({ const typeof( ((struct item *)0)->_head ) *__mptr = (it); \
             (struct item *)( (char *)__mptr - __builtin_offsetof(struct item, _head) );})",
        );
        let texts = texts(&statement);
        let shape = element_shape(&texts).expect("element");
        assert_eq!(shape.destination, "e");
        assert_eq!(shape.pointer, ["it"]);
        assert_eq!(shape.member, "_head");
        let declared = tokens(
            "struct item *e = ({ const typeof( ((struct item *)0)->_head ) *__mptr = ((&q->items)->next); \
             (struct item *)( (char *)__mptr - ((size_t) &((struct item *)0)->_head) );})",
        );
        let texts = super::texts(&declared);
        let shape = element_shape(&texts).expect("declared element");
        assert_eq!(
            shape.pointer,
            ["(", "&", "q", "-", ">", "items", ")", "-", ">", "next"]
        );
    }

    #[test]
    fn a_mismatched_member_or_type_is_not_an_element() {
        for statement in [
            "e = ({ const typeof( ((struct item *)0)->_head ) *__mptr = (it); \
             (struct item *)( (char *)__mptr - __builtin_offsetof(struct item, other) );})",
            "e = ({ const typeof( ((struct item *)0)->_head ) *__mptr = (it); \
             (struct other *)( (char *)__mptr - __builtin_offsetof(struct item, _head) );})",
            "e = ({ const typeof( ((struct item *)0)->_head ) *__mptr = (it); q_free(it); \
             (struct item *)( (char *)__mptr - __builtin_offsetof(struct item, _head) );})",
        ] {
            let tokens = tokens(statement);
            assert_eq!(element_shape(&texts(&tokens)), None, "{statement}");
        }
    }

    #[test]
    fn a_written_cursor_declines_the_body() {
        let body = ["{", "it", "=", "0", ";", "}"];
        assert!(!body_leaves_cursors(&body, &["it"]));
        let body = ["{", "q_use", "(", "&", "it", ")", ";", "}"];
        assert!(!body_leaves_cursors(&body, &["it"]));
        let body = ["{", "x", "=", "it", "-", ">", "next", ";", "}"];
        assert!(body_leaves_cursors(&body, &["it"]));
    }
}
