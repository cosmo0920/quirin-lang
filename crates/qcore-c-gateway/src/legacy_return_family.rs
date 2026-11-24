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
//! Card **RETURN-FAMILY-STAR-AFTER-UNRESOLVED-CAST** (gateway rule 74h): a
//! single `*` or `&` after a `)` is read by the group it closes. A group the
//! declaration reader resolves as a type ([`is_cast_group`]) is a cast; a
//! group rule 74f's one shared detector rules out (an argument list, a value
//! group, a declared object) is an operand; every other group is
//! **undecided** ([`Operator::Undecided`]), and family S keeps every refusal
//! and every base **either** reading raises: the `&` refusal, the starred
//! untracked base, and a may-bind root's one dereference counted under both
//! readings (`QCore.ReturnFamilyStarCast`).
//!
//! Card **RETURN-FAMILY-DEREF-THROUGH-RESOLVED-CAST** (gateway rule 74m): a
//! starred may-bind root counts its `*`s twice, once stopping at the nearest
//! cast group and once through every cast group (rule 12h's transparency),
//! and is admitted only when both counts are one. `(T) *q` is one
//! dereference; `*(T *)*q` and `*(T *)(U *)*q` are two and refuse;
//! `*(T *)q` dereferences the converted value and keeps its refusal
//! (`QCore.ReturnFamilyDerefCast`).
//!
//! Card **RETURN-FAMILY-UNTRACKED-DEREF-THROUGH-CAST** (gateway rule 74r): an
//! untracked root is a starred base also when a `*` that may be unary reaches
//! it through opening parentheses and cast groups
//! ([`dereferenced_through_groups`]), so `*(T *)p` makes `p` a base as `*p`
//! does. The change only adds bases (`QCore.ReturnFamilyUntrackedDerefCast`).
//!
//! Card **RETURN-FAMILY-UNTRACKED-MEMBER-THROUGH-CAST** (gateway rule 74t): an
//! untracked root is a member or subscript base, and a `->` root, also when
//! `->` or `[` reaches it through closing parentheses whose groups hold only
//! opening parentheses and cast groups before it ([`accessed_through_groups`]),
//! so `((T *)p)->f`, `((T *)p)[i]` and `(p)->f` make `p` a base as `p->f` and
//! `p[i]` do. The change only adds bases and `->` roots
//! (`QCore.ReturnFamilyUntrackedMemberCast`).
//!
//! Card **RETURN-FAMILY-UNTRACKED-ACCESS-THROUGH-ARITHMETIC** (gateway rule
//! 74w): an untracked, non-scalar root is a base, and a `->` root, also when
//! `->`, `[` or a `*` that may be unary reaches it through a parenthesised
//! additive group as well as cast groups and parentheses
//! ([`accessed_through_arithmetic`]), so `(p + n)->f`, `(n + p)[i]`,
//! `((T *)p - n)->f` and `*(n + p)` make `p` a base. An access through a sum
//! whose other operands are not literals, declared scalars or `sizeof` of a
//! resolved type (`(p - q)->f`) refuses. Bases and `->` roots only grow and
//! the one new refusal is the expression's, so nothing admits more
//! (`QCore.ReturnFamilyUntrackedArithAccess`).
//!
//! Card **RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS** (gateway rule 74x):
//! when `->`, `[` or a `*` that may be unary accesses a parenthesised
//! conditional, through cast groups, parentheses and classified additive
//! groups as rule 74w reads them, every untracked, non-scalar root of
//! **either** arm is a base, and a `->` root for `->`
//! ([`conditional_roots`]), so `(c ? p : q)->f`, `(0 ? p : q)->f`,
//! `*(c ? p : q)` and `((T *)(c ? p : q))[i]` make both `p` and `q` bases.
//! The condition is a read and is never decided. An arm the reader cannot
//! classify (a string, a unary `-`, a product) refuses. Bases and `->` roots
//! only grow and the one new refusal is the expression's, so nothing admits
//! more (`QCore.ReturnFamilyUntrackedConditionalAccess`).
//!
//! Card **RETURN-FAMILY-ARITH-WALK-CROSSES-CONDITIONAL** (gateway rule 82):
//! rule 74w's walk read a conditional group as the additive group around a
//! later term of an arm's sum (`(c ? 1 + p - n : a)->f`, `(c ? a : n +
//! p)[0]`) and refused. It now classifies such a group within the arm, the
//! arm's `?` or `:` standing in for the parentheses ([`arm_around`]); when
//! that is the only unclassified reading it returns
//! [`ArithmeticAccess::Arm`], and family S refuses unless rule 74x made the
//! root a base. A root in the condition and every other unclassified group
//! refuse as before (`QCore.ReturnFamilyArithWalkConditional`).
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

use std::cell::OnceCell;
use std::collections::BTreeSet;

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
    /// Rule 74f's declared objects (`publication::value_names`): the formals,
    /// the body's declarations and the unit's file-scope objects, less every
    /// name a `typedef` spells. A `( name )` of one of them is an operand.
    fn objects(&self) -> &BTreeSet<String>;
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

/// Card RETURN-FAMILY-STAR-AFTER-UNRESOLVED-CAST (rule 74h): how a single
/// `&` or `*` reads by the token before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operator {
    /// Nothing, an operator or an opening group precedes it, or a cast the
    /// declaration reader resolves.
    Unary,
    /// An operand ends before it: a name, a number, `]`, or a `)` the shared
    /// detector rules out as a cast.
    Binary,
    /// A `)` neither reader decides: `(T)` with `T` unresolved, `(a)` with
    /// `a` no declared object. Either reading may be C's.
    Undecided,
}

/// Which reading an undecided `)` takes in one count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reading {
    /// The group is an operand: the operator is binary.
    Operand,
    /// The group is a cast: the operator is unary.
    Cast,
}

impl Operator {
    fn unary_under(self, reading: Reading) -> bool {
        match self {
            Self::Unary => true,
            Self::Binary => false,
            Self::Undecided => reading == Reading::Cast,
        }
    }

    /// Unary under at least one reading: what every refusal and every base
    /// keys on, so that neither reading's is lost.
    fn may_be_unary(self) -> bool {
        self.unary_under(Reading::Cast)
    }
}

/// How the operator at `index` (`&` or `*`) reads. A `)` is a resolved cast
/// when [`is_cast_group`] resolves its group, an operand when rule 74f's
/// shared detector rules the group out, and undecided otherwise.
fn operator_at(texts: &[&str], index: usize, names: &impl NameFacts) -> Operator {
    let Some(previous) = index.checked_sub(1).map(|at| texts[at]) else {
        return Operator::Unary;
    };
    match previous {
        ")" if is_cast_group(texts, index - 1, names) => Operator::Unary,
        ")" if crate::ampersand_runs::may_close_a_cast(texts, index - 1, names.objects()) => {
            Operator::Undecided
        }
        ")" | "]" => Operator::Binary,
        text if super::is_identifier(text) => Operator::Binary,
        text if text.as_bytes().first().is_some_and(u8::is_ascii_digit) => Operator::Binary,
        _ => Operator::Unary,
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
/// `usize::MAX` counts every one, which only refuses. Rule 74h: a `*` after
/// an undecided `)` counts under `reading`'s choice. The count is monotone in
/// that choice, so the operand and cast readings bound every mixed one.
/// Rule 74m: with `through`, a cast group does not stop the count at any
/// `closing` (`*(T *)*q` counts two), so the count with `through` is never
/// less than the count without it.
fn stars_before(
    texts: &[&str],
    index: usize,
    names: &impl NameFacts,
    closing: usize,
    reading: Reading,
    through: bool,
) -> usize {
    let mut at = index;
    let mut stars = 0;
    let mut opened = 0usize;
    while let Some(previous) = at.checked_sub(1) {
        match texts[previous] {
            "(" => {
                opened += 1;
                at = previous;
            }
            "*" if operator_at(texts, previous, names).unary_under(reading) => {
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
            // `*(T *)q->f` dereferences the value `q->f` reads. Card
            // RETURN-FAMILY-DEREF-THROUGH-RESOLVED-CAST (rule 74m): so is a
            // starred root's when `through` asks, so that `*(T *)*q`, which
            // dereferences the value `*q` reads, counts two.
            ")" if (closing == usize::MAX || through)
                && let Some(open) = crate::cast_semantics::type_group_open(texts, previous) =>
            {
                at = open;
            }
            _ => break,
        }
    }
    stars
}

/// Rule 74r: whether the name at `index` is a word of a cast's type name: it
/// stands in a parenthesised group of names and `*`s with only names before
/// it, and the group is no value group, that is, it holds more than one
/// token (`( long * )`, `( unsigned long )`, `( long * const )`) or its one
/// name is followed by what may start an operand (`( T ) x`, `( T ) * x`).
/// `( p )` before the end, a `)` or a binary-only operator is a value group,
/// and its name may be a root (`* ( long * ) ( p )`). A name this calls a
/// type word only keeps the base's reading.
fn cast_type_word(texts: &[&str], index: usize) -> bool {
    let mut first = index;
    while first
        .checked_sub(1)
        .is_some_and(|previous| super::is_identifier(texts[previous]))
    {
        first -= 1;
    }
    let Some(open) = first.checked_sub(1).filter(|open| texts[*open] == "(") else {
        return false;
    };
    let mut close = index + 1;
    while texts
        .get(close)
        .is_some_and(|text| *text == "*" || super::is_identifier(text))
    {
        close += 1;
    }
    texts.get(close) == Some(&")")
        && (close - open > 2
            || texts.get(close + 1).is_some_and(|text| {
                super::is_identifier(text)
                    || text.as_bytes().first().is_some_and(u8::is_ascii_digit)
                    || matches!(*text, "(" | "*" | "&" | "+" | "-" | "!" | "~")
            }))
}

/// Card RETURN-FAMILY-UNTRACKED-DEREF-THROUGH-CAST (rule 74r): whether a `*`
/// that may be unary under either of rule 74h's readings dereferences the
/// untracked root at `index` through opening parentheses and through groups
/// `cast_semantics::type_group_open` reads as a type (rule 12h's
/// transparency), so that `*(T *)p`, `*(T *)(U *)p` and `*((T *)p)` make `p`
/// a base as `*p` does. It only adds bases, each an obligation the flow check
/// must discharge (`QCore.ReturnFamilyUntrackedDerefCast`). A word of a
/// cast's type name ([`cast_type_word`]) is no root, so `long` in
/// `* ( long * ) p` is not made a base.
fn dereferenced_through_groups(texts: &[&str], index: usize, names: &impl NameFacts) -> bool {
    if cast_type_word(texts, index) {
        return false;
    }
    let mut at = index;
    while let Some(previous) = at.checked_sub(1) {
        match texts[previous] {
            "(" => at = previous,
            ")" => match crate::cast_semantics::type_group_open(texts, previous) {
                Some(open) => at = open,
                None => return false,
            },
            "*" => return operator_at(texts, previous, names).may_be_unary(),
            _ => return false,
        }
    }
    false
}

/// Card RETURN-FAMILY-UNTRACKED-MEMBER-THROUGH-CAST (rule 74t): the access
/// (`->` or `[`) that reaches the untracked root at `index` through closing
/// parentheses whose groups hold nothing before the root but opening
/// parentheses and groups `cast_semantics::type_group_open` reads as a type
/// (rule 12h's transparency), so that `((T *)p)->f`, `((T *)p)[i]`,
/// `((T *)(p))->f` and `(p)->f` make `p` a base as `p->f` and `p[i]` do.
/// `Some(true)` is a `->`, `Some(false)` a `[`, and `None` is no such access:
/// at least one `)` must stand between the root and the access (the direct
/// spellings are the old test's), and no more than the walk back opened, so
/// a group with an operand before the root (`( n + p )`, `( n + ( p ) )`) is
/// never read through. (`f ( p ) -> m` reads as an access of `p`: one more
/// base in an expression family S refuses as a call anyway.) A cast's type
/// word is followed inside its group by
/// another word, a `*` or the `)` before an operand, never by `)` then `->`
/// or `[`, so it is no root here. It only adds bases and `->` roots
/// (`QCore.ReturnFamilyUntrackedMemberCast`).
fn accessed_through_groups(texts: &[&str], index: usize) -> Option<bool> {
    let mut at = index;
    let mut opened = 0usize;
    while let Some(previous) = at.checked_sub(1) {
        match texts[previous] {
            "(" => {
                opened += 1;
                at = previous;
            }
            ")" => match crate::cast_semantics::type_group_open(texts, previous) {
                Some(open) => at = open,
                None => break,
            },
            _ => break,
        }
    }
    let closes = texts[index + 1..]
        .iter()
        .take(opened)
        .take_while(|text| **text == ")")
        .count();
    let after = index + 1 + closes;
    if closes == 0 {
        None
    } else if arrow_at(texts, after) {
        Some(true)
    } else if texts.get(after) == Some(&"[") {
        Some(false)
    } else {
        None
    }
}

/// Rule 74w: what [`accessed_through_arithmetic`] read past an untracked
/// root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArithmeticAccess {
    /// No additive group crossed, or nothing accesses what the walk reached:
    /// the other tests' reading stands.
    None,
    /// `( p + n ) - > f`: a base and a `->` root.
    Arrow,
    /// `( p + n ) [ i ]` or `* ( n + p )`: a base.
    Element,
    /// An access through an additive group whose other operands the reader
    /// cannot classify (`( p - q ) - > f`): family S refuses.
    Unclassified,
    /// Rule 82: `Unclassified` only because a crossed group is a conditional
    /// whose arm holds the span, every crossed group classified when the
    /// arm's `?` or `:` delimit it (`( c ? 1 + p - n : a ) - > f`): family S
    /// refuses unless rule 74x made the root a base, and a `->` root when
    /// `true` (`->` accesses what the walk reached).
    Arm(bool),
}

/// Rule 74w: whether `part`, the other operands of an additive group around
/// the root, holds only what can never be a pointer: numeric literals,
/// declared scalars (no pointer level, [`NameFacts::declared_scalar`]), a
/// type operator over a resolved type name, parentheses and `+ - * / %`.
fn integer_operands(part: &[&str], names: &impl NameFacts) -> bool {
    let mut at = 0usize;
    while at < part.len() {
        let text = part[at];
        if TYPE_OPERATORS.contains(&text) {
            let Some(close) = part
                .get(at + 1)
                .filter(|next| **next == "(")
                .and_then(|_| close_of(part, at + 1))
            else {
                return false;
            };
            let words = &part[at + 2..close];
            if words.is_empty()
                || !words.iter().all(|word| super::is_identifier(word))
                || !names.type_name(words)
            {
                return false;
            }
            at = close + 1;
            continue;
        }
        let integer = matches!(text, "(" | ")" | "+" | "-" | "*" | "/" | "%")
            || text.as_bytes().first().is_some_and(u8::is_ascii_digit)
            || (super::is_identifier(text) && names.declared_scalar(text));
        if !integer {
            return false;
        }
        at += 1;
    }
    true
}

/// Rule 74w: whether `text` may end an operand, so that an operator after it
/// is binary.
fn operand_end(text: &str) -> bool {
    matches!(text, ")" | "]")
        || super::is_identifier(text)
        || text.as_bytes().first().is_some_and(u8::is_ascii_digit)
}

/// Rule 74w: the `(` of the innermost group around `start..=end`, and its
/// matching `)`, when the innermost bracket around it is a `(`.
fn enclosing_group(texts: &[&str], start: usize) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    for at in (0..start).rev() {
        match texts[at] {
            ")" | "]" => depth += 1,
            "(" if depth == 0 => return close_of(texts, at).map(|close| (at, close)),
            "[" if depth == 0 => return None,
            "(" | "[" => depth -= 1,
            _ => {}
        }
    }
    None
}

/// Rule 82: the arm `a_lo..a_hi` of the conditional group `open..=close`
/// that holds the span `lo..=hi`, a term at the group's top level: the
/// tokens between the nearest top-level `?` or `:` before the span and the
/// nearest after it (or the group's `)`). `None` when the group holds no
/// top-level `?`, nothing delimits the span before it, or a `?` follows it
/// (the span is a condition, never an arm).
fn arm_around(
    texts: &[&str],
    open: usize,
    close: usize,
    lo: usize,
    hi: usize,
) -> Option<(usize, usize)> {
    let delimiter = |at: usize| matches!(texts[at], "?" | ":");
    top_level(texts, open + 1, close, |at| texts[at] == "?")?;
    let mut before = None;
    let mut from = open + 1;
    while let Some(at) = top_level(texts, from, lo, delimiter) {
        before = Some(at);
        from = at + 1;
    }
    let after = top_level(texts, hi + 1, close, delimiter);
    if after.is_some_and(|at| texts[at] == "?") {
        return None;
    }
    Some((before? + 1, after.unwrap_or(close)))
}

/// Card RETURN-FAMILY-UNTRACKED-ACCESS-THROUGH-ARITHMETIC (rule 74w): the
/// access that reaches the untracked, non-scalar root at `index` through
/// pointer arithmetic. The walk keeps a span `lo..=hi` of what it has read,
/// starting at the root, and widens it outward through a group
/// `cast_semantics::type_group_open` reads as a type before it (rule 12h's
/// transparency), through a `(` and `)` right around it, and through a
/// parenthesised **arithmetic group**: a binary `+`, `-`, `/` or `%` (never
/// `- >`, never `*`, which a type group spells too) next to the span, the
/// other neighbour such an operator or the group's own parenthesis, inside
/// the innermost `( … )` around the span, which becomes the span. Only that
/// group's matching `)` is read, so no closing parenthesis past the ones the
/// walk opened is. When at least one such group was crossed, a `->` or `[`
/// after the span, or a `*` before it that may be unary under either of
/// rule 74h's readings, accesses the root: `( p + n ) - > f`,
/// `( n + p ) [ i ]`, `( ( T * ) p - n ) - > f`, `( ( T * ) ( p + n ) ) - > f`,
/// `* ( n + p )`. A group is classified when the span is a term of a sum
/// (`+`, `-` or the group's parenthesis on each side) and
/// [`integer_operands`] accepts the rest; an access through any other
/// crossed group (`( p - q ) - > f`, `( n % p ) - > f`) is
/// [`ArithmeticAccess::Unclassified`] and family S refuses
/// (`QCore.ReturnFamilyUntrackedArithAccess`).
fn accessed_through_arithmetic(
    texts: &[&str],
    index: usize,
    names: &impl NameFacts,
) -> ArithmeticAccess {
    let (mut lo, mut hi) = (index, index);
    let mut crossed = false;
    let mut classified = true;
    // Rule 82: whether a conditional's arm delimited a crossed group, and
    // whether every crossed group is classified with the arm's `?` or `:`
    // standing in for a conditional group's parentheses.
    let mut delimited = false;
    let mut classified_in_arms = true;
    loop {
        if let Some(open) = lo
            .checked_sub(1)
            .filter(|previous| texts[*previous] == ")")
            .and_then(|previous| crate::cast_semantics::type_group_open(texts, previous))
        {
            lo = open;
            continue;
        }
        let left = lo.checked_sub(1).map(|previous| texts[previous]);
        let right = texts.get(hi + 1).copied();
        if left == Some("(") && right == Some(")") {
            lo -= 1;
            hi += 1;
            continue;
        }
        // An arithmetic operator next to the span, binary (after an
        // operand) on the left: a unary operator binds tighter, so its
        // operand, not the span, is the term (`( * p + 1 )`, `( - p )`).
        // A `*` is never read as a product: a type group's `*` reads the
        // same (`( long * const )`), and a product is no pointer.
        let left_binary =
            matches!(left, Some("+" | "-" | "/" | "%")) && lo >= 2 && operand_end(texts[lo - 2]);
        let right_operator =
            matches!(right, Some("+" | "-" | "/" | "%")) && !arrow_at(texts, hi + 1);
        if !((left_binary || left == Some("(")) && (right_operator || right == Some(")"))) {
            break;
        }
        let Some((open, close)) = enclosing_group(texts, lo) else {
            break;
        };
        // Classified: the span is a term of the sum (`+`, `-` or the
        // group's own parenthesis on each side, so no product holds it), and
        // the other operands can never be a pointer.
        let term_of = |from: usize, to: usize| {
            let clean_left = (left_binary && matches!(left, Some("+" | "-"))) || lo == from;
            let clean_right = (right_operator && matches!(right, Some("+" | "-"))) || hi + 1 == to;
            clean_left
                && clean_right
                && integer_operands(&texts[from..lo], names)
                && integer_operands(&texts[hi + 1..to], names)
        };
        if !term_of(open + 1, close) {
            classified = false;
        }
        let arm = arm_around(texts, open, close, lo, hi);
        delimited |= arm.is_some();
        if !arm.map_or_else(|| term_of(open + 1, close), |(from, to)| term_of(from, to)) {
            classified_in_arms = false;
        }
        crossed = true;
        lo = open;
        hi = close;
    }
    if !crossed {
        return ArithmeticAccess::None;
    }
    let arrow = arrow_at(texts, hi + 1);
    let element = texts.get(hi + 1) == Some(&"[")
        || lo.checked_sub(1).is_some_and(|star| {
            texts[star] == "*" && operator_at(texts, star, names).may_be_unary()
        });
    match (arrow || element, classified) {
        (false, _) => ArithmeticAccess::None,
        (true, false) if delimited && classified_in_arms => ArithmeticAccess::Arm(arrow),
        (true, false) => ArithmeticAccess::Unclassified,
        (true, true) if arrow => ArithmeticAccess::Arrow,
        (true, true) => ArithmeticAccess::Element,
    }
}

/// Rule 74x: the untracked roots of accessed conditional arms, by token
/// index: every one a base, and the `->` ones also `->` roots.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ConditionalRoots {
    bases: BTreeSet<usize>,
    arrows: BTreeSet<usize>,
}

/// Rule 74x: the first index in `lo..hi` where `wanted` holds at bracket
/// depth zero.
fn top_level(
    texts: &[&str],
    lo: usize,
    hi: usize,
    mut wanted: impl FnMut(usize) -> bool,
) -> Option<usize> {
    let mut depth = 0usize;
    for (at, text) in texts.iter().enumerate().take(hi).skip(lo) {
        match *text {
            "(" | "[" => depth += 1,
            ")" | "]" => depth = depth.saturating_sub(1),
            _ if depth == 0 && wanted(at) => return Some(at),
            _ => {}
        }
    }
    None
}

/// Rule 74x: the `:` in `lo..hi` that closes the conditional whose `?` is
/// just before `lo`, counting the nested conditionals of its middle arm.
fn matching_colon(texts: &[&str], lo: usize, hi: usize) -> Option<usize> {
    let mut open = 0usize;
    top_level(texts, lo, hi, |at| match texts[at] {
        "?" => {
            open += 1;
            false
        }
        ":" if open == 0 => true,
        ":" => {
            open -= 1;
            false
        }
        _ => false,
    })
}

/// Rule 74x: whether `lo..hi` is a value C loads from memory: a unary `*`
/// of an operand, or a name or parenthesised group followed by at least one
/// `- > F`, `. F` or `[ … ]` that ends at `hi`. Its roots are read by the
/// old tests (a member, subscript or starred root is a base already), and
/// what it loads is a place of an untracked object the function never
/// names, so it yields no root here.
fn loaded_value(texts: &[&str], lo: usize, hi: usize) -> bool {
    if texts[lo] == "*" {
        return hi > lo + 1;
    }
    let mut at = if texts[lo] == "(" {
        match close_of(texts, lo) {
            Some(close) if close < hi => close + 1,
            _ => return false,
        }
    } else if super::is_identifier(texts[lo]) {
        lo + 1
    } else {
        return false;
    };
    let mut postfix = false;
    while at < hi {
        if arrow_at(texts, at) || texts[at] == "." {
            let member = at + if texts[at] == "." { 1 } else { 2 };
            if member >= hi || !super::is_identifier(texts[member]) {
                return false;
            }
            at = member + 1;
        } else if texts[at] == "[" {
            match close_of(texts, at) {
                Some(close) if close < hi => at = close + 1,
                _ => return false,
            }
        } else {
            return false;
        }
        postfix = true;
    }
    postfix
}

/// Rule 74x: the untracked, non-scalar roots of the value `lo..hi` (an arm
/// of an accessed conditional) yields, pushed onto `roots`; `false` when the
/// reader cannot classify the arm, which refuses the expression. An arm is
/// classified when it is, after its parentheses: a conditional, whose
/// condition is a read and both of whose arms are classified (neither is
/// ever pruned, even under a constant condition); a sum whose binary `+` and
/// `-` terms are all [`integer_operands`] but at most one, itself
/// classified and not subtracted; a cast of a classified operand; a numeric
/// literal or a declared scalar (no root); a name this body never binds (a
/// root); or a [`loaded_value`]. A string, a unary `-`, `!` or `~`, a
/// product or comparison of the whole arm, a may-bind name and every other
/// spelling refuse.
fn arm_roots(
    texts: &[&str],
    lo: usize,
    hi: usize,
    names: &impl NameFacts,
    roots: &mut Vec<usize>,
) -> bool {
    if lo >= hi {
        return false;
    }
    if texts[lo] == "(" && close_of(texts, lo) == Some(hi - 1) {
        return arm_roots(texts, lo + 1, hi - 1, names, roots);
    }
    if let Some(question) = top_level(texts, lo, hi, |at| texts[at] == "?") {
        let Some(colon) = matching_colon(texts, question + 1, hi) else {
            return false;
        };
        return question > lo
            && arm_roots(texts, question + 1, colon, names, roots)
            && arm_roots(texts, colon + 1, hi, names, roots);
    }
    let binary = |at: usize| {
        matches!(texts[at], "+" | "-")
            && at > lo
            && operand_end(texts[at - 1])
            && !arrow_at(texts, at)
    };
    if top_level(texts, lo, hi, binary).is_some() {
        let mut pointer: Option<(usize, usize)> = None;
        let mut start = lo;
        while start < hi {
            let end = top_level(texts, start + 1, hi, binary).unwrap_or(hi);
            let term = if start == lo { start } else { start + 1 };
            if term >= end {
                return false;
            }
            if !integer_operands(&texts[term..end], names) {
                if pointer.is_some() || (texts[start] == "-" && start != lo) {
                    return false;
                }
                pointer = Some((term, end));
            }
            start = end;
        }
        return pointer.is_none_or(|(term, end)| arm_roots(texts, term, end, names, roots));
    }
    if loaded_value(texts, lo, hi) {
        return true;
    }
    if texts[lo] == "("
        && let Some(close) = close_of(texts, lo)
        && close + 1 < hi
        && crate::cast_semantics::type_group_open(texts, close) == Some(lo)
    {
        return arm_roots(texts, close + 1, hi, names, roots);
    }
    if hi == lo + 1 {
        let text = texts[lo];
        if text.as_bytes().first().is_some_and(u8::is_ascii_digit)
            || (super::is_identifier(text) && names.declared_scalar(text))
        {
            return true;
        }
        if super::is_identifier(text) && names.untracked(text) {
            roots.push(lo);
            return true;
        }
    }
    false
}

/// Rule 74x: what accesses the conditional group `lo..=hi`, widened outward
/// as rule 74w's walk widens a root: through a group
/// `cast_semantics::type_group_open` reads as a type before it, a `(` and
/// `)` right around it, and a parenthesised arithmetic group around it,
/// which is classified as in [`accessed_through_arithmetic`]. Unlike that
/// walk no arithmetic group needs to be crossed: the conditional group is
/// already a value of its own.
fn conditional_group_access(
    texts: &[&str],
    mut lo: usize,
    mut hi: usize,
    names: &impl NameFacts,
) -> ArithmeticAccess {
    let mut classified = true;
    loop {
        if let Some(open) = lo
            .checked_sub(1)
            .filter(|previous| texts[*previous] == ")")
            .and_then(|previous| crate::cast_semantics::type_group_open(texts, previous))
        {
            lo = open;
            continue;
        }
        let left = lo.checked_sub(1).map(|previous| texts[previous]);
        let right = texts.get(hi + 1).copied();
        if left == Some("(") && right == Some(")") {
            lo -= 1;
            hi += 1;
            continue;
        }
        let left_binary =
            matches!(left, Some("+" | "-" | "/" | "%")) && lo >= 2 && operand_end(texts[lo - 2]);
        let right_operator =
            matches!(right, Some("+" | "-" | "/" | "%")) && !arrow_at(texts, hi + 1);
        if !((left_binary || left == Some("(")) && (right_operator || right == Some(")"))) {
            break;
        }
        let Some((open, close)) = enclosing_group(texts, lo) else {
            break;
        };
        let clean_left = (left_binary && matches!(left, Some("+" | "-"))) || lo == open + 1;
        let clean_right = (right_operator && matches!(right, Some("+" | "-"))) || hi + 1 == close;
        if !(clean_left
            && clean_right
            && integer_operands(&texts[open + 1..lo], names)
            && integer_operands(&texts[hi + 1..close], names))
        {
            classified = false;
        }
        lo = open;
        hi = close;
    }
    let arrow = arrow_at(texts, hi + 1);
    let element = texts.get(hi + 1) == Some(&"[")
        || lo.checked_sub(1).is_some_and(|star| {
            texts[star] == "*" && operator_at(texts, star, names).may_be_unary()
        });
    match (arrow || element, classified) {
        (false, _) => ArithmeticAccess::None,
        (true, false) => ArithmeticAccess::Unclassified,
        (true, true) if arrow => ArithmeticAccess::Arrow,
        (true, true) => ArithmeticAccess::Element,
    }
}

/// Card RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS (rule 74x): the roots of
/// every parenthesised conditional group that `->`, `[` or a `*` that may be
/// unary accesses ([`conditional_group_access`]), from both arms
/// ([`arm_roots`]): `( c ? p : q ) - > f`, `( c ? p : q ) [ i ]`,
/// `* ( c ? p : q )`, `( ( T * ) ( c ? p : q ) ) - > f`,
/// `( ( c ? p : q ) + n ) - > f`, `( c ? ( d ? p : r ) : q ) - > f`,
/// `( c ? p + n : q ) - > f`. The condition is a read and is never decided.
/// `None` refuses the expression: an accessed group with an arm the reader
/// cannot classify, or reached through an unclassified arithmetic group.
/// A conditional group nothing accesses (`( c ? p : q ) != 0`,
/// `( c ? p - > n : q - > n )`) yields nothing
/// (`QCore.ReturnFamilyUntrackedConditionalAccess`).
fn conditional_roots(texts: &[&str], names: &impl NameFacts) -> Option<ConditionalRoots> {
    let mut found = ConditionalRoots::default();
    for open in 0..texts.len() {
        if texts[open] != "(" {
            continue;
        }
        let Some(close) = close_of(texts, open) else {
            continue;
        };
        if top_level(texts, open + 1, close, |at| texts[at] == "?").is_none() {
            continue;
        }
        let access = conditional_group_access(texts, open, close, names);
        let arrow = match access {
            ArithmeticAccess::None => continue,
            ArithmeticAccess::Unclassified | ArithmeticAccess::Arm(_) => return None,
            ArithmeticAccess::Arrow => true,
            ArithmeticAccess::Element => false,
        };
        let mut roots = Vec::new();
        if !arm_roots(texts, open + 1, close, names, &mut roots) {
            return None;
        }
        for root in roots {
            found.bases.insert(root);
            if arrow {
                found.arrows.insert(root);
            }
        }
    }
    Some(found)
}

/// Family S's shape and name premises over one `return` expression. `None`
/// keeps the refusal. The declared-result premise and the descriptor premise
/// are the caller's.
pub(crate) fn scalar_read(texts: &[&str], names: &impl NameFacts) -> Option<ScalarRead> {
    if texts.is_empty() {
        return None;
    }
    let conditional = conditional_positions(texts);
    // Rule 74x: the roots of accessed conditional arms; an arm the reader
    // cannot classify refuses.
    let conditional_arms = conditional_roots(texts, names)?;
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
            // Rule 74h: refused when either reading takes an address.
            "&" if operator_at(texts, index, names).may_be_unary() => return None,
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
            // Rule 74h: a base when either reading dereferences it, the
            // union of both readings' bases. Rule 74r: also when a `*`
            // dereferences it through parentheses and cast groups.
            let starred = (index >= 1
                && texts[index - 1] == "*"
                && operator_at(texts, index - 1, names).may_be_unary())
                || dereferenced_through_groups(texts, index, names);
            // Rule 74t: a member or subscript root also through closing
            // parentheses and cast groups, with its `->` reading.
            let accessed = accessed_through_groups(texts, index);
            if accessed == Some(true) {
                push_once(&mut read.arrow_roots, text);
            }
            // Rule 74w: an access through pointer arithmetic on the root; a
            // declared scalar is an index, never the pointer. An access
            // through a sum the reader cannot classify refuses.
            let arithmetic = if names.declared_scalar(text) {
                ArithmeticAccess::None
            } else {
                accessed_through_arithmetic(texts, index, names)
            };
            match arithmetic {
                ArithmeticAccess::Unclassified => return None,
                // Rule 82: the arm's refusal stands unless rule 74x read the
                // arm and made this root a base.
                ArithmeticAccess::Arm(_) if !conditional_arms.bases.contains(&index) => {
                    return None;
                }
                ArithmeticAccess::Arrow | ArithmeticAccess::Arm(true) => {
                    push_once(&mut read.arrow_roots, text)
                }
                ArithmeticAccess::None
                | ArithmeticAccess::Element
                | ArithmeticAccess::Arm(false) => {}
            }
            // Rule 74x: an arm of an accessed conditional, either arm.
            if conditional_arms.arrows.contains(&index) {
                push_once(&mut read.arrow_roots, text);
            }
            if arrow_at(texts, index + 1)
                || next == Some("[")
                || starred
                || accessed.is_some()
                || arithmetic != ArithmeticAccess::None
                || conditional_arms.bases.contains(&index)
            {
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
        // Rule 74h: a member or subscript root counts an undecided `*` (the
        // cast reading, the larger count); a starred root needs exactly one
        // under both readings, so neither reading's refusal is lost.
        let root = if arrow_at(texts, index + 1) {
            stars_before(texts, index, names, usize::MAX, Reading::Cast, true) == 0
                && texts
                    .get(index + 3)
                    .is_some_and(|text| super::is_identifier(text))
                && ends_one_dereference(texts, index + 4)
        } else if next == Some("[") {
            stars_before(texts, index, names, usize::MAX, Reading::Cast, true) == 0
                && close_of(texts, index + 1)
                    .is_some_and(|close| ends_one_dereference(texts, close + 1))
        } else {
            let closing = texts[index + 1..]
                .iter()
                .take_while(|text| **text == ")")
                .count();
            // Rule 74m: exactly one `*` up to the nearest cast group (so
            // `*(T *)q`, a dereference of the converted value, refuses) and
            // exactly one through every cast group (so `*(T *)*q`, a second
            // dereference, refuses).
            [Reading::Operand, Reading::Cast]
                .into_iter()
                .all(|reading| {
                    [false, true].into_iter().all(|through| {
                        stars_before(texts, index, names, closing, reading, through) == 1
                    })
                })
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
    /// Rule 74h: read only when a `)` before a `*` or `&` is no resolved cast.
    objects: OnceCell<BTreeSet<String>>,
}

impl<'p, 'a> ParserNames<'p, 'a> {
    fn new(parser: &'p Parser<'a>) -> Self {
        Self {
            parser,
            objects: OnceCell::new(),
        }
    }
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

    /// Rule 74f's set, read from the unit's token stream as the publication
    /// facts read it. With no body located it is empty, so every `( name )`
    /// is undecided, which keeps both readings' refusals.
    fn objects(&self) -> &BTreeSet<String> {
        self.objects.get_or_init(|| {
            let parser = self.parser;
            let start = parser.owned_out_body_start;
            let Some(end) = super::matching(parser.tokens, start, "{", "}") else {
                return BTreeSet::new();
            };
            let unit = parser
                .tokens
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>();
            let (declared, _) = crate::publication::body_declarations(&unit[start + 1..end]);
            crate::publication::value_names(
                &unit,
                &declared,
                &crate::publication::file_scope_objects(&unit),
                &parser.formal_parameters,
            )
        })
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
        let shape = cast_call(texts, &ParserNames::new(self));
        if let Some(shape) = shape {
            return self.cast_call_instruction(expression, texts, line, &shape);
        }
        if !self.return_facts.scalar_result
            || descriptor_custody::value_carries_descriptor(texts, &self.descriptor_names).is_some()
        {
            return None;
        }
        let read = scalar_read(texts, &ParserNames::new(self))?;
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

#[cfg(test)]
#[path = "../tests/unit/legacy_return_family_star_cast.rs"]
mod star_cast_tests;

#[cfg(test)]
#[path = "../tests/unit/legacy_return_family_deref_cast.rs"]
mod deref_cast_tests;

#[cfg(test)]
#[path = "../tests/unit/legacy_return_family_untracked_deref_cast.rs"]
mod untracked_deref_cast_tests;

#[cfg(test)]
#[path = "../tests/unit/legacy_return_family_untracked_member_cast.rs"]
mod untracked_member_cast_tests;

#[cfg(test)]
#[path = "../tests/unit/legacy_return_family_untracked_arith_access.rs"]
mod untracked_arith_access_tests;

#[cfg(test)]
#[path = "../tests/unit/legacy_return_family_untracked_conditional_access.rs"]
mod untracked_conditional_access_tests;

#[cfg(test)]
#[path = "../tests/unit/legacy_return_family_arith_walk_conditional.rs"]
mod arith_walk_conditional_tests;
