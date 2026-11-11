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

//! Card **W45-RETURN-EXPRESSION-FAMILIES**: two families of `return`
//! expression the position used to refuse ("unmodeled call, aggregate or
//! indirect access in return expression"), each lowered to what C evaluates
//! with the returned value's custody stated exactly.
//!
//! Both are consulted only from `Parser::return_expression_instruction`, after
//! every older route — the strict and pointer-leaf rules (constraint 5b), the
//! cast return (W44-LEAK-PIN-AUDIT), the W4 whole call, both B01 routes and
//! W43's refused-whole-call lowering — has declined, so no expression an older
//! route decides is decided by different code.
//!
//! # S — the scalar-result read
//!
//! `return E;` in a function whose **definition declares a scalar value
//! result** (an arithmetic or enumeration type, or a typedef the unit's
//! declaration reader resolves to one, with no pointer level:
//! [`TypedAccessModel::is_scalar_value_type`]). C converts `E` to that type
//! (C11 6.8.6.4p3), so the returned value is a number: **it carries no
//! custody**, and the exit escapes nothing (`NoOp`). What remains is the
//! evaluation of `E`, and the rule admits only an evaluation that is a *read*:
//!
//! * no call (a call-shaped `NAME (` of any kind refuses, `_Generic` and the
//!   builtins included), no brace, no assignment, no increment or decrement,
//!   no comma, no GNU `?:` and no unary `&`;
//! * a type operator (`sizeof`, `_Alignof`) only over a parenthesised type
//!   name of identifiers the reader resolves and that is never variably
//!   modified ([`TypedAccessModel::never_variably_modified`]), so the operand
//!   is not evaluated (C11 6.5.3.4p2). A pointer type name (`sizeof(void *)`)
//!   keeps the fixed type-query rule's refusal, which its own pins hold;
//! * no name that may hold a tracked descriptor is mentioned: an integer
//!   identity is custody, never data;
//! * every name `E` evaluates is **untracked** — one this body can never bind
//!   to a tracked identity (`UntrackedBases::never_bound`, card C02-P premise
//!   4, the reading the chain route's root obligation uses) — or a declared
//!   scalar. A read through an untracked root reads an object no reaching
//!   path bound, and a member of such an object is a place this function never
//!   names (C02-P premise 3), so the links of a chain below it carry no
//!   obligation. A directly dereferenced untracked root is still a base of the
//!   read node, so the flow check stands behind the syntactic premise: such a
//!   name can never be released by this body, so the check can refuse
//!   (`unknown_alias` on a possibly-null entry binding) but never accuse;
//! * a name the body **may** bind is admitted only as the root of exactly one
//!   dereference — `B -> F`, `B -> F . G`, `B [ I ]`, `* B` — that C evaluates
//!   unconditionally (not after a `?`, `&&` or `||` of its group). It is a
//!   **base** of the read node and keeps the typed access route's obligation
//!   in transfer: unbound, a non-null untracked binding, or exactly one known,
//!   non-null, live identity, else `unknown_alias`; a base settled on a
//!   released identity is a `use_after_release` candidate. The position
//!   premise is what keeps that accusation to paths C actually takes. Any
//!   other use of such a name — as a value, under a second dereference,
//!   inside a parenthesised operand — refuses.
//!
//! The read node is `IndirectScalarAccess` with no destination, no frame and
//! no chain: it changes no binding, status, marker or link, and adds only the
//! bases' obligations and the C02-P-ALIAS obligations of every `->` root. It
//! is not a new instruction, so `descriptor_custody::admits_descriptor_read`
//! needs no new arm (and no descriptor reaches it).
//!
//! A **pointer-valued** result is not this rule: `p->buf` may be a load, an
//! array member's decay or, as `&p->f`, an interior address, which is a result
//! derived from `p`; telling them apart needs member typing, and publishing a
//! derived pointer as a result that carries nothing would let a caller free
//! `p` and keep using the result unaccused. It keeps the refusal.
//!
//! # K — a cast of a whole call
//!
//! `return ( T1 ) … ( Tn ) CALL ( ARGS );`, possibly parenthesised, with every
//! `Ti` a type name the reader resolves (so `(f)(x)`, a call of `f`, is never
//! read as a cast). The synthetic `PLACE = CALL ( ARGS );` is classified
//! exactly as card C-GATEWAY-RETURN-CALL-W4 classifies an uncast whole call:
//!
//! * a **refusal** is lowered to that refusal and a `NoOp` exit — card
//!   W43-RETURN-EXPRESSION's re-attribution, now under a cast. It decides
//!   nothing; the site counter is restored;
//! * an admitted call whose result is **`None`** carries nothing whatever the
//!   casts convert it to, and is taken as W4 takes it;
//! * an admitted **`Owned`** result is taken only when every cast targets a
//!   pointer type and the unit's own declaration of `CALL` returns an object
//!   pointer (`declared_abi::declares_object_pointer_result`): a
//!   pointer-to-pointer conversion designates the same object (C11
//!   6.3.2.3p7), so the exit escapes exactly the site the call created;
//! * with **no** cast (only parentheses) the answer is W4's own admission,
//!   derived results under `admit_derived_returns` included;
//! * every other classification — a `Reallocated` or `Derived` result under a
//!   cast, an `Owned` result under an integer cast, anything that is not the
//!   call into `PLACE` — keeps the return refusal, with the site counter
//!   restored.
//!
//! # Evaluation order
//!
//! Both families lower to one evaluation node before the exit node, at the
//! `return`'s own line, so every call and read of the expression happens
//! before the function returns, as C sequences it.
//!
//! The Lean model is `QCore.ReturnExpressionFamilies`; as for every gateway
//! module, the correspondence is maintained by review (constraint 28).

use qcore_schemas::contract::ResourceReturnMode;

use super::{LegacyInstruction, Parser, Token, descriptor_custody, return_call};
use crate::typed_field_access::TypedAccessModel;

/// Facts about the enclosing function, read once from its definition.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FunctionFacts {
    /// The definition declares a scalar value result (family S's premise).
    pub(crate) scalar_result: bool,
}

impl FunctionFacts {
    /// Read `function`'s definition — the first `NAME ( … ) {`, the one
    /// `find_function_body` delimits — and classify its declared result.
    pub(crate) fn read(tokens: &[Token], function: &str, model: &TypedAccessModel) -> Self {
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        Self {
            scalar_result: definition_result(&texts, function)
                .is_some_and(|specifiers| model.is_scalar_value_type(&specifiers)),
        }
    }
}

/// Words that end the backward walk over a definition's prefix: a
/// declaration's specifiers never continue past them, and a `typedef` prefix
/// is not a function definition's.
const PREFIX_WORDS_ENDING_A_WALK: &[&str] = &["typedef", "return"];

/// Attribute introducers whose parenthesised group carries no type.
const ATTRIBUTES: &[&str] = &["__attribute__", "__attribute", "__declspec"];

/// The specifier words of `function`'s definition, when the result has no
/// pointer level and the prefix is readable: walked backwards from the name
/// over identifiers and skipping attribute groups, and required to begin a
/// declaration (after `;`, `{`, `}` or at the start of the stream). `None`
/// for a pointer result, a declarator this walk does not read (a function
/// returning a function pointer, a K&R definition) and no definition.
pub(crate) fn definition_result<'a>(texts: &[&'a str], function: &str) -> Option<Vec<&'a str>> {
    let name = (0..texts.len()).find(|&index| {
        texts[index] == function
            && texts.get(index + 1) == Some(&"(")
            && close_of(texts, index + 1).is_some_and(|close| texts.get(close + 1) == Some(&"{"))
    })?;
    let mut words = Vec::new();
    let mut start = name;
    while start > 0 {
        let previous = texts[start - 1];
        if previous == ")" {
            let open = open_of(texts, start - 1)?;
            if open == 0 || !ATTRIBUTES.contains(&texts[open - 1]) {
                break;
            }
            start = open - 1;
            continue;
        }
        if previous == "*" {
            // A pointer result.
            return None;
        }
        if !super::is_identifier(previous) || PREFIX_WORDS_ENDING_A_WALK.contains(&previous) {
            break;
        }
        words.push(previous);
        start -= 1;
    }
    if start > 0 && !matches!(texts[start - 1], ";" | "{" | "}") {
        return None;
    }
    if words.is_empty() {
        return None;
    }
    words.reverse();
    Some(words)
}

fn close_of(texts: &[&str], open: usize) -> Option<usize> {
    let (opener, closer) = match *texts.get(open)? {
        "(" => ("(", ")"),
        "[" => ("[", "]"),
        _ => return None,
    };
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(open) {
        if *text == opener {
            depth += 1;
        } else if *text == closer {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn open_of(texts: &[&str], close: usize) -> Option<usize> {
    let (opener, closer) = match *texts.get(close)? {
        ")" => ("(", ")"),
        "]" => ("[", "]"),
        _ => return None,
    };
    let mut depth = 0usize;
    for index in (0..=close).rev() {
        if texts[index] == closer {
            depth += 1;
        } else if texts[index] == opener {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

/// What family S needs to know about a name, from the parser's facts.
pub(crate) trait NameFacts {
    /// Never bound by this body to a tracked identity:
    /// `UntrackedBases::never_bound` (card C02-P premise 4), the reading the
    /// chain route's root obligation uses.
    fn untracked(&self, name: &str) -> bool;
    /// Declared a scalar by this function's declaration reader.
    fn declared_scalar(&self, name: &str) -> bool;
    /// `words` (identifiers, no `*`) name a type the reader resolves and that
    /// is never variably modified.
    fn type_name(&self, words: &[&str]) -> bool;
}

/// Family S's read: the may-bind bases it dereferences and the `->` roots
/// whose C02-P-ALIAS obligation it carries, each once, in textual order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ScalarRead {
    pub(crate) bases: Vec<String>,
    pub(crate) arrow_roots: Vec<String>,
}

const TYPE_OPERATORS: &[&str] = &["sizeof", "_Alignof", "alignof", "__alignof__", "__alignof"];

fn push_once(list: &mut Vec<String>, name: &str) {
    if !list.iter().any(|known| known == name) {
        list.push(name.to_owned());
    }
}

/// Whether `texts[index]` (`=`) is an assignment operator rather than part of
/// `==`, `!=`, `<=` or `>=` (`<<=` and `>>=` are assignments).
fn is_assignment(texts: &[&str], index: usize) -> bool {
    let previous = index.checked_sub(1).map(|at| texts[at]);
    texts.get(index + 1) != Some(&"=")
        && (!matches!(previous, Some("=" | "!" | "<" | ">"))
            || (matches!(previous, Some("<" | ">"))
                && index.checked_sub(2).map(|at| texts[at]) == previous))
}

/// Whether `( … )` closing at `close` is a cast: identifiers the reader
/// resolves to a type, then any number of `*`.
fn is_cast_group(texts: &[&str], close: usize, names: &impl NameFacts) -> bool {
    let Some(open) = open_of(texts, close) else {
        return false;
    };
    cast_target(&texts[open + 1..close], names).is_some()
}

/// The target of a cast whose parenthesised type is `group`: `Some(true)` for
/// a pointer type, `Some(false)` for another type, `None` when `group` is not
/// a type name the reader resolves.
fn cast_target(group: &[&str], names: &impl NameFacts) -> Option<bool> {
    let stars = group.iter().rev().take_while(|text| **text == "*").count();
    let words = &group[..group.len() - stars];
    (!words.is_empty()
        && words.iter().all(|text| super::is_identifier(text))
        && names.type_name(words))
    .then_some(stars > 0)
}

/// Whether the operator at `index` (`&` or `*`) is unary: nothing, an
/// operator or an opening group precedes it, or a cast does.
fn unary_position(texts: &[&str], index: usize, names: &impl NameFacts) -> bool {
    let Some(previous) = index.checked_sub(1).map(|at| texts[at]) else {
        return true;
    };
    match previous {
        ")" => is_cast_group(texts, index - 1, names),
        "]" => false,
        text if super::is_identifier(text) => false,
        text if text.as_bytes().first().is_some_and(u8::is_ascii_digit) => false,
        _ => true,
    }
}

/// Token indices C may skip: every token after a `?`, `&&` or `||` up to the
/// end of the group that operator is in. A superset of the conditionally
/// evaluated tokens, which only narrows the may-bind admission.
fn conditional_positions(texts: &[&str]) -> Vec<bool> {
    let mut conditional = vec![false; texts.len()];
    let mut depth = 0usize;
    let mut starts: Vec<usize> = Vec::new();
    for (index, text) in texts.iter().enumerate() {
        match *text {
            "(" | "[" => depth += 1,
            ")" | "]" => {
                depth = depth.saturating_sub(1);
                starts.retain(|start| *start <= depth);
            }
            _ => {}
        }
        let connective = *text == "?"
            || (matches!(*text, "&" | "|")
                && (texts.get(index + 1) == Some(text)
                    || index.checked_sub(1).map(|at| texts[at]) == Some(*text)));
        if connective {
            starts.push(depth);
        }
        conditional[index] = starts.iter().any(|start| *start <= depth);
    }
    conditional
}

/// Is `texts[index]` followed by a member selection `- >` or `.`?
fn arrow_at(texts: &[&str], index: usize) -> bool {
    texts.get(index) == Some(&"-") && texts.get(index + 1) == Some(&">")
}

/// After a may-bind root's one dereference ends at `after`, may only `.`
/// selections and closing parentheses follow, and then nothing that
/// continues the access? Card W45-SR5-SOUNDNESS
/// (`SCALAR-READ-PARENTHESISED-DOUBLE-DEREF`): a closing parenthesis no
/// longer ends the reading, so `(q->next)->len` and `(*q)[0]` are the second
/// dereferences they are.
fn ends_one_dereference(texts: &[&str], mut after: usize) -> bool {
    loop {
        if texts.get(after) == Some(&")") {
            after += 1;
        } else if texts.get(after) == Some(&".")
            && texts
                .get(after + 1)
                .is_some_and(|text| super::is_identifier(text))
        {
            after += 2;
        } else {
            break;
        }
    }
    !arrow_at(texts, after) && !matches!(texts.get(after), Some(&"[") | Some(&"(") | Some(&"."))
}

/// Card W45-SR5-SOUNDNESS: how many unary `*` apply to the name at `index`,
/// read back through opening parentheses (`*q`, `(*q)`, `**q`, `*(q->f)`).
/// A `*` behind more opening parentheses than `closing` does not apply to the
/// name alone (`*(T *)p` applies to the cast, never to the type name `T`);
/// `usize::MAX` counts every one, which only refuses.
fn stars_before(texts: &[&str], index: usize, names: &impl NameFacts, closing: usize) -> usize {
    let mut at = index;
    let mut stars = 0;
    let mut opened = 0usize;
    while let Some(previous) = at.checked_sub(1) {
        match texts[previous] {
            "(" => {
                opened += 1;
                at = previous;
            }
            "*" if unary_position(texts, previous, names) => {
                if opened > closing {
                    break;
                }
                stars += 1;
                at = previous;
            }
            // Card CAST-FORMALIZATION (rule 12h,
            // `W45-SR5-SCALAR-READ-STAR-THROUGH-CAST`): where every `*` counts
            // (before a member or subscript root), a cast group is
            // transparent to the count (`QCore.CastSemantics.cast_transparent`):
            // `*(T *)q->f` dereferences the value `q->f` reads.
            ")" if closing == usize::MAX
                && let Some(open) = crate::cast_semantics::type_group_open(texts, previous) =>
            {
                at = open;
            }
            _ => break,
        }
    }
    stars
}

/// Family S's shape and name premises over one `return` expression. `None`
/// keeps the refusal. The declared-result premise and the descriptor premise
/// are the caller's.
pub(crate) fn scalar_read(texts: &[&str], names: &impl NameFacts) -> Option<ScalarRead> {
    if texts.is_empty() {
        return None;
    }
    let conditional = conditional_positions(texts);
    let mut read = ScalarRead::default();
    let mut index = 0usize;
    while index < texts.len() {
        let text = texts[index];
        let next = texts.get(index + 1).copied();
        match text {
            "{" | "}" | "," | ";" => return None,
            "=" if is_assignment(texts, index) => return None,
            "+" | "-" if next == Some(text) => return None,
            "?" if next == Some(":") => return None,
            "&" if next == Some("&") => {
                index += 2;
                continue;
            }
            "&" if unary_position(texts, index, names) => return None,
            "." | "-" if text == "." || next == Some(">") => {
                // A member name is not a value name; a member called is a
                // call.
                let member = index + if text == "." { 1 } else { 2 };
                if !texts
                    .get(member)
                    .is_some_and(|text| super::is_identifier(text))
                    || texts.get(member + 1) == Some(&"(")
                {
                    return None;
                }
                index = member + 1;
                continue;
            }
            // A call through a parenthesised or subscripted operand; a cast
            // group followed by a parenthesised operand is not a call.
            ")" if next == Some("(") && !is_cast_group(texts, index, names) => return None,
            "]" if next == Some("(") => return None,
            _ => {}
        }
        if TYPE_OPERATORS.contains(&text) {
            // Only over a parenthesised, resolved, never variably modified
            // type name: an operand C does not evaluate.
            if next != Some("(") {
                return None;
            }
            let close = close_of(texts, index + 1)?;
            let words = &texts[index + 2..close];
            if words.is_empty()
                || !words.iter().all(|text| super::is_identifier(text))
                || !names.type_name(words)
            {
                return None;
            }
            index = close + 1;
            continue;
        }
        if !super::is_identifier(text) {
            index += 1;
            continue;
        }
        if next == Some("(") {
            // A call of any kind.
            return None;
        }
        if names.declared_scalar(text) && !names.untracked(text) {
            // A scalar binds no identity; it is not dereferenced.
            index += 1;
            continue;
        }
        if names.untracked(text) {
            // Anything goes; a directly dereferenced root is still a base, so
            // the flow check stands behind the syntactic premise.
            if arrow_at(texts, index + 1) {
                push_once(&mut read.arrow_roots, text);
            }
            let starred =
                index >= 1 && texts[index - 1] == "*" && unary_position(texts, index - 1, names);
            if arrow_at(texts, index + 1) || next == Some("[") || starred {
                push_once(&mut read.bases, text);
            }
            index += 1;
            continue;
        }
        // A name this body may bind: the root of exactly one dereference C
        // always evaluates, and nothing else.
        if conditional[index] {
            return None;
        }
        // Card W45-SR5-SOUNDNESS: a `*` counts through parentheses, and the
        // one dereference must end after them too. Before a member or
        // subscript root every `*` counts (it only refuses); a starred root
        // counts the `*`s whose parentheses close right after the name.
        let root = if arrow_at(texts, index + 1) {
            stars_before(texts, index, names, usize::MAX) == 0
                && texts
                    .get(index + 3)
                    .is_some_and(|text| super::is_identifier(text))
                && ends_one_dereference(texts, index + 4)
        } else if next == Some("[") {
            stars_before(texts, index, names, usize::MAX) == 0
                && close_of(texts, index + 1)
                    .is_some_and(|close| ends_one_dereference(texts, close + 1))
        } else {
            let closing = texts[index + 1..]
                .iter()
                .take_while(|text| **text == ")")
                .count();
            stars_before(texts, index, names, closing) == 1
                && next != Some(".")
                && ends_one_dereference(texts, index + 1)
        };
        if !root {
            return None;
        }
        if arrow_at(texts, index + 1) {
            push_once(&mut read.arrow_roots, text);
        }
        push_once(&mut read.bases, text);
        index += 1;
    }
    Some(read)
}

/// Family K's shape: the casts (`true` for a pointer target), in order, and
/// the half-open token span of the whole call they convert.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CastCall {
    pub(crate) pointer_targets: Vec<bool>,
    pub(crate) call: (usize, usize),
}

/// `start..end` with every enclosing pair of parentheses removed.
fn strip_group(texts: &[&str], mut start: usize, mut end: usize) -> (usize, usize) {
    while end > start + 1 && texts[start] == "(" && close_of(texts, start) == Some(end - 1) {
        start += 1;
        end -= 1;
    }
    (start, end)
}

/// Read `( T1 ) … ( Tn ) CALL ( … )`, possibly parenthesised at any level,
/// with at least one cast or one pair of parentheses (the bare whole call is
/// W4's and W43's), every `Ti` a type name the reader resolves, and `CALL` a
/// name followed by its argument list closing at the end.
pub(crate) fn cast_call(texts: &[&str], names: &impl NameFacts) -> Option<CastCall> {
    let (mut start, mut end) = strip_group(texts, 0, texts.len());
    let parenthesised = start > 0;
    let mut pointer_targets = Vec::new();
    while texts.get(start) == Some(&"(") {
        let close = close_of(texts, start)?;
        if close + 1 >= end {
            return None;
        }
        pointer_targets.push(cast_target(&texts[start + 1..close], names)?);
        (start, end) = strip_group(texts, close + 1, end);
    }
    if pointer_targets.is_empty() && !parenthesised {
        return None;
    }
    let callee = *texts.get(start)?;
    if !super::is_identifier(callee)
        || TYPE_OPERATORS.contains(&callee)
        || texts.get(start + 1) != Some(&"(")
        || close_of(texts, start + 1)? + 1 != end
    {
        return None;
    }
    Some(CastCall {
        pointer_targets,
        call: (start, end),
    })
}

/// The parser's facts, as family S and K read them.
struct ParserNames<'p, 'a> {
    parser: &'p Parser<'a>,
}

impl NameFacts for ParserNames<'_, '_> {
    fn untracked(&self, name: &str) -> bool {
        self.parser.typed_bases.never_bound(name)
    }

    fn declared_scalar(&self, name: &str) -> bool {
        self.parser.typed_types.declared_is_scalar(name)
    }

    fn type_name(&self, words: &[&str]) -> bool {
        self.parser.typed_model.never_variably_modified(words)
    }
}

impl Parser<'_> {
    /// The hook `return_expression_instruction` calls last. `Some` is the
    /// evaluation node and whether the exit escapes the reserved place; `None`
    /// keeps the return-position refusal.
    pub(super) fn return_family_instruction(
        &mut self,
        expression: &[Token],
        texts: &[&str],
        line: usize,
    ) -> Option<(LegacyInstruction, bool)> {
        let decided = self.return_family_decision(expression, texts, line);
        #[cfg(feature = "indirect-census")]
        if !self.admit_derived_returns
            && let Some((instruction, escapes)) = &decided
        {
            census::record(&self.census_function, line, texts, instruction, *escapes);
        }
        decided
    }

    fn return_family_decision(
        &mut self,
        expression: &[Token],
        texts: &[&str],
        line: usize,
    ) -> Option<(LegacyInstruction, bool)> {
        let shape = cast_call(texts, &ParserNames { parser: self });
        if let Some(shape) = shape {
            return self.cast_call_instruction(expression, texts, line, &shape);
        }
        if !self.return_facts.scalar_result
            || descriptor_custody::value_carries_descriptor(texts, &self.descriptor_names).is_some()
        {
            return None;
        }
        let read = scalar_read(texts, &ParserNames { parser: self })?;
        Some((
            LegacyInstruction::IndirectScalarAccess {
                typed_alternative: None,
                alias_reads: read
                    .arrow_roots
                    .iter()
                    .filter_map(|root| self.read_alias(Some(root)))
                    .collect(),
                bases: read.bases,
                destination: None,
                frame: None,
                chains: Vec::new(),
                borrowed_destination: false,
                readback: None,
            },
            false,
        ))
    }

    /// Family K: classify the converted call as W4 classifies a whole call,
    /// and take the answer only on the grounds in the module header.
    fn cast_call_instruction(
        &mut self,
        expression: &[Token],
        texts: &[&str],
        line: usize,
        shape: &CastCall,
    ) -> Option<(LegacyInstruction, bool)> {
        if !return_call::place_is_unspelled(self.tokens, return_call::RETURN_RESULT_PLACE) {
            return None;
        }
        let (start, end) = shape.call;
        let callee = texts[start];
        let call = &expression[start..end];
        let call_texts = &texts[start..end];
        // The callee spellings W43 keeps out of the lowering (type operators
        // and constants) are kept out here too.
        if !return_call::lowers_refused_whole_call(call, call_texts, true) {
            return None;
        }
        let mut synthetic = Vec::with_capacity(call.len() + 2);
        synthetic.extend(return_call::assignment_prefix(line));
        synthetic.extend_from_slice(call);
        // Card GAPO-RETURN-CORRECTNESS (rule 15x): the whole probe state.
        let restore = self.probe_state();
        let instruction = self.classify_simple(&synthetic);
        // Card GAPO-RETURN-CORRECTNESS (rule 15z): rule 12h's H-U0.
        let instruction = self
            .uncast_returned_call(
                &return_call::assignment_prefix(line),
                call,
                &instruction,
                &restore,
            )
            .unwrap_or(instruction);
        let admitted = match &instruction {
            LegacyInstruction::Unknown { .. } => {
                // Re-attribution: the classifier's own refusal, deciding
                // nothing, with the probe state restored.
                self.restore_probe_state(&restore);
                return Some((instruction, false));
            }
            LegacyInstruction::ContractCall {
                destination: Some(destination),
                contract,
                ..
            } if destination == return_call::RETURN_RESULT_PLACE => {
                if shape.pointer_targets.is_empty() {
                    // Parentheses only: W4's own admission.
                    return_call::escapes_only_fresh_custody(contract.result)
                        || (self.admit_derived_returns
                            && matches!(contract.result, ResourceReturnMode::Derived { .. }))
                } else {
                    match contract.result {
                        ResourceReturnMode::None => true,
                        ResourceReturnMode::Owned { .. } => {
                            shape.pointer_targets.iter().all(|pointer| *pointer)
                                && super::declared_abi::declares_object_pointer_result(
                                    &self
                                        .tokens
                                        .iter()
                                        .map(|token| token.text.as_str())
                                        .collect::<Vec<_>>(),
                                    callee,
                                )
                        }
                        ResourceReturnMode::Reallocated { .. }
                        | ResourceReturnMode::Removed { .. }
                        | ResourceReturnMode::Derived { .. } => false,
                    }
                }
            }
            _ => false,
        };
        if admitted {
            Some((instruction, true))
        } else {
            self.restore_probe_state(&restore);
            None
        }
    }
}

/// Census only (card W45-RETURN-EXPRESSION-FAMILIES): one JSON line per
/// `return` this module decided, written after the decision and read by
/// nothing but the card's attribution scripts. Compiled out with the
/// `indirect-census` feature off; with it on, the audit's artifacts are
/// unchanged.
#[cfg(feature = "indirect-census")]
pub(crate) mod census {
    use std::fs::File;
    use std::io::{BufWriter, Write};
    use std::sync::Mutex;

    use super::LegacyInstruction;

    /// The environment variable naming the stream.
    pub const RETURN_FAMILY_CENSUS_PATH: &str = "QCORE_RETURN_FAMILY_CENSUS";

    static SINK: Mutex<Option<Option<BufWriter<File>>>> = Mutex::new(None);

    #[derive(serde::Serialize)]
    struct Record<'a> {
        unit_seq: u64,
        function: &'a str,
        line: usize,
        /// `scalar-read`, `cast-call-lowered` or `cast-call-admitted`.
        family: &'static str,
        tokens: &'a [&'a str],
        /// The read node's bases, or the admitted call's callee.
        subjects: Vec<String>,
    }

    pub(crate) fn record(
        function: &str,
        line: usize,
        tokens: &[&str],
        instruction: &LegacyInstruction,
        escapes: bool,
    ) {
        if std::env::var_os(RETURN_FAMILY_CENSUS_PATH).is_none() {
            return;
        }
        let (family, subjects) = match instruction {
            LegacyInstruction::IndirectScalarAccess { bases, .. } => ("scalar-read", bases.clone()),
            LegacyInstruction::ContractCall { function, .. } if escapes => {
                ("cast-call-admitted", vec![function.clone()])
            }
            _ => ("cast-call-lowered", Vec::new()),
        };
        let record = Record {
            unit_seq: crate::indirect_census::unit_seq(),
            function,
            line,
            family,
            tokens,
            subjects,
        };
        let Ok(mut guard) = SINK.lock() else {
            return;
        };
        let opened = guard.get_or_insert_with(|| {
            let path = std::env::var_os(RETURN_FAMILY_CENSUS_PATH)?;
            File::create(path).ok().map(BufWriter::new)
        });
        let Some(writer) = opened.as_mut() else {
            return;
        };
        if let Ok(line) = serde_json::to_string(&record) {
            let _ = writeln!(writer, "{line}");
            let _ = writer.flush();
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/legacy_return_family.rs"]
mod tests;
