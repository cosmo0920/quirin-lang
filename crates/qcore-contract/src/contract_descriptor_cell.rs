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

//! Card **MK-EVENT-DESCRIPTOR-QUANT** (contract spec §C04.6.2, grammar G05d,
//! gateway rule 38): descriptor cells, read total-or-reject like every other
//! clause.
//!
//! USER DECISION 2026-11-01 (verbatim in the spec and the card's report): "I
//! approve the changes of D1-D5." The shorthand and its lower level:
//!
//! ```text
//! quant live #data: mk_event.fd |-> #fresh(descriptor) when #result != -1;
//!     ensures normal resource_member_kind(data, mk_event.fd, descriptor);
//!     ensures normal resource_on_return(ne, -1, resource_member_out(data, mk_event.fd));
//! quant live #data: mk_event.fd |-> {#stale(descriptor), #kept} when #result == -1;
//!     ensures normal resource_on_return(eq, -1, resource_member_stale_or_kept(data, mk_event.fd));
//! quant live #data: mk_event.fd |-> #stale(descriptor);
//!     ensures normal resource_member_released(data, mk_event.fd, stale);
//! quant live #data: mk_event.fd |-> #same(r_fd) when #result == 0;
//!     ensures normal resource_on_return(eq, 0, resource_member_alias(data, mk_event.fd, r_fd));
//! quant live #r_fd: #pointee |-> #fresh(descriptor) when #result == 0;
//!     ensures normal resource_on_return(eq, 0, resource_out(r_fd, nonnull));
//!     ensures normal resource_on_return(ne, 0, resource_borrowed(r_fd));
//!     ensures normal resource_kind(r_fd, descriptor);
//! ensures normal resource_released(w_fd, valid) when #data: mk_event.fd == #r_fd;
//! quant live #loop: #holds(data) |-> #absent when #data: mk_event.fd == #r_fd;
//! ```
//!
//! Every spelling elaborates into one canonical
//! [`qcore_schemas::descriptor_cell::DescriptorCells`]: an agreeing overlap
//! merges, a disagreeing one rejects the module.

use std::collections::BTreeMap;

use qcore_frontend::lexer::{MAPS_TO, Symbol, TokenKind, lex};
use qcore_schemas::callback::CallbackMember;
use qcore_schemas::contract::{
    ResourceFunctionContract, ResourceKind, ResourceParameterMode, ResourceReturnMode,
};
use qcore_schemas::descriptor_cell::{
    CellPlace, CellState, CellValue, DescriptorCells, FRESH, MemberEquality, POINTEE, SAME,
    WitnessGuard,
};

/// The member-cell words of the lower level.
pub(super) const MEMBER_KIND: &str = "resource_member_kind";
pub(super) const MEMBER_OUT: &str = "resource_member_out";
pub(super) const MEMBER_RELEASED: &str = "resource_member_released";
pub(super) const MEMBER_ALIAS: &str = "resource_member_alias";
pub(super) const MEMBER_STALE_OR_KEPT: &str = "resource_member_stale_or_kept";

const STALE: &str = "stale";
const KEPT: &str = "kept";
const RESULT: &str = "result";

/// Where a written state stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum WrittenPlace {
    Member(CallbackMember),
    Pointee,
}

/// A written value; the shorthand carries its kind, the lower level does not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum WrittenValue {
    Fresh(Option<ResourceKind>),
    Stale(Option<ResourceKind>),
    StaleOrKept(Option<ResourceKind>),
    Same(String),
}

/// One cell clause as written, either level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Written {
    State {
        binder: String,
        place: WrittenPlace,
        value: WrittenValue,
        guard: Option<WitnessGuard>,
        text: String,
    },
    /// `resource_member_kind(P, TAG.FIELD, K)`, or (moved here by the
    /// loader) `resource_kind(S, K)` of a slot binder.
    Kind {
        binder: String,
        place: WrittenPlace,
        kind: ResourceKind,
        text: String,
    },
}

/// `when #P: TAG.FIELD == #X` as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WrittenEquality {
    pub binder: String,
    pub member: CallbackMember,
    pub against: String,
}

fn tokens(source: &str) -> Option<Vec<TokenKind>> {
    Some(
        lex(source)
            .ok()?
            .into_iter()
            .map(|token| token.kind)
            .filter(|kind| *kind != TokenKind::Eof)
            .collect(),
    )
}

fn is_identifier(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with(|ch: char| ch.is_ascii_digit())
        && text
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

struct Reader<'a> {
    tokens: &'a [TokenKind],
}

impl Reader<'_> {
    fn word(&self, at: usize) -> Option<&str> {
        match self.tokens.get(at) {
            Some(TokenKind::Word(word)) => Some(word.as_str()),
            _ => None,
        }
    }

    fn symbol(&self, at: usize, expected: Symbol) -> bool {
        self.tokens.get(at) == Some(&TokenKind::Symbol(expected))
    }

    /// `TAG . SEG (. SEG)*` at `at`: the member and the index after it.
    fn member(&self, mut at: usize) -> Option<(CallbackMember, usize)> {
        let mut segments = Vec::new();
        loop {
            let segment = self.word(at).filter(|segment| is_identifier(segment))?;
            segments.push(segment.to_owned());
            at += 1;
            if self.symbol(at, Symbol::Dot) {
                at += 1;
                continue;
            }
            break;
        }
        if segments.len() < 2 {
            return None;
        }
        let tag = segments.remove(0);
        Some((
            CallbackMember {
                tag,
                path: segments.join("."),
            },
            at,
        ))
    }

    /// `# KEYWORD ( ARG )` at `at`: the argument word and the index after it.
    fn application(&self, at: usize, keyword: &str) -> Option<(String, usize)> {
        (self.symbol(at, Symbol::Hash)
            && self.word(at + 1) == Some(keyword)
            && self.symbol(at + 2, Symbol::LeftParen)
            && self.symbol(at + 4, Symbol::RightParen))
        .then(|| self.word(at + 3).map(|word| (word.to_owned(), at + 5)))
        .flatten()
    }
}

fn cell_kind(word: &str) -> Result<ResourceKind, String> {
    match ResourceKind::from_spelling(word) {
        Some(ResourceKind::Descriptor) => Ok(ResourceKind::Descriptor),
        _ => Err(format!(
            "quant-live-cell-kind: a descriptor cell's kind is `descriptor`, not `{word}`"
        )),
    }
}

/// Whether the text after `quant live` is a cell state: a `#pointee` left
/// side, or a value `#fresh(…)`, `#stale(…)`, `{#stale(…), …}` or `#same(…)`.
pub(super) fn is_cell_clause(source: &str) -> bool {
    let Some(tokens) = tokens(source) else {
        return false;
    };
    let reader = Reader { tokens: &tokens };
    if !(reader.symbol(0, Symbol::Hash) && reader.symbol(2, Symbol::Colon)) {
        return false;
    }
    if reader.symbol(3, Symbol::Hash) && reader.word(4) == Some(POINTEE) {
        return true;
    }
    let Some(maps) = tokens
        .iter()
        .position(|kind| *kind == TokenKind::Symbol(Symbol::MapsTo))
    else {
        return false;
    };
    let at = if reader.symbol(maps + 1, Symbol::LeftBrace) {
        maps + 2
    } else {
        maps + 1
    };
    reader.symbol(at, Symbol::Hash)
        && match reader.word(at + 1) {
            Some(FRESH | SAME) => true,
            Some(STALE) => reader.symbol(at + 2, Symbol::LeftParen),
            _ => false,
        }
}

/// `when # result (== | !=) [-] INTEGER` from `at` to the end.
fn witness_guard(
    reader: &Reader<'_>,
    at: usize,
    shape: &dyn Fn() -> String,
) -> Result<WitnessGuard, String> {
    let guard_error = || {
        format!(
            "quant-live-cell-guard: a cell state is guarded by `when #result != -1`, `== -1`, \
             `== 0` or `!= 0`: {}",
            shape()
        )
    };
    if reader.word(at) != Some("when")
        || !reader.symbol(at + 1, Symbol::Hash)
        || reader.word(at + 2) != Some(RESULT)
    {
        return Err(guard_error());
    }
    let equal = if reader.symbol(at + 3, Symbol::EqualEqual) {
        true
    } else if reader.symbol(at + 3, Symbol::NotEqual) {
        false
    } else {
        return Err(guard_error());
    };
    let (negative, digits, end) = match (reader.tokens.get(at + 4), reader.tokens.get(at + 5)) {
        (Some(TokenKind::Symbol(Symbol::Minus)), Some(TokenKind::Integer(digits))) => {
            (true, digits, at + 6)
        }
        (Some(TokenKind::Integer(digits)), _) => (false, digits, at + 5),
        _ => return Err(guard_error()),
    };
    if end != reader.tokens.len() {
        return Err(guard_error());
    }
    let value: i64 = match digits.as_str() {
        "0" if !negative => 0,
        "1" if negative => -1,
        _ => return Err(guard_error()),
    };
    WitnessGuard::from_comparison(equal, value).ok_or_else(guard_error)
}

/// Read a cell state, the text after `quant live`:
/// `#P: TAG.FIELD |-> V [when #result OP K]` or `#S: #pointee |-> V [when …]`,
/// `V` one of `#fresh(K)`, `#stale(K)`, `{#stale(K), #kept}`, `#same(S)`.
pub(super) fn quant_cell_clause(source: &str) -> Result<Written, String> {
    let shape = || {
        format!(
            "quant-live-cell-shape: a descriptor cell state is `quant live #P: TAG.FIELD {MAPS_TO} \
             #fresh(descriptor) when #result != -1;` (or `#S: #pointee`; values `#fresh(K)`, \
             `#stale(K)`, `{{#stale(K), #kept}}`, `#same(S)`), not `quant live {source};`"
        )
    };
    let tokens = tokens(source).ok_or_else(shape)?;
    let reader = Reader { tokens: &tokens };
    if !reader.symbol(0, Symbol::Hash) || !reader.symbol(2, Symbol::Colon) {
        return Err(shape());
    }
    let binder = reader
        .word(1)
        .filter(|name| is_identifier(name))
        .ok_or_else(shape)?;
    if qcore_schemas::handoff::RESERVED_KEYWORDS.contains(&binder) {
        return Err(format!(
            "quant-live-cell-binder: `#{binder}` is a reserved keyword, not a formal's binder"
        ));
    }
    let (place, mut at) = if reader.symbol(3, Symbol::Hash) {
        if reader.word(4) != Some(POINTEE) {
            return Err(shape());
        }
        (WrittenPlace::Pointee, 5)
    } else {
        let (member, at) = reader.member(3).ok_or_else(|| {
            format!(
                "quant-live-path-member: the left of `{MAPS_TO}` is a dotted `TAG.FIELD` or \
                 `#pointee`: {}",
                shape()
            )
        })?;
        (WrittenPlace::Member(member), at)
    };
    if reader.symbol(at, Symbol::Arrow) {
        return Err(format!(
            "quant-live-maps-to: `->` is C member access, not maps-to: write `{MAPS_TO}`"
        ));
    }
    if !reader.symbol(at, Symbol::MapsTo) {
        return Err(shape());
    }
    at += 1;
    let value_error = || {
        format!(
            "quant-live-cell-value: a descriptor cell maps to `#fresh(K)`, `#stale(K)`, \
             `{{#stale(K), #kept}}` or `#same(S)`: {}",
            shape()
        )
    };
    let value = if reader.symbol(at, Symbol::LeftBrace) {
        let (kind, after) = reader.application(at + 1, STALE).ok_or_else(value_error)?;
        if !reader.symbol(after, Symbol::Comma)
            || !reader.symbol(after + 1, Symbol::Hash)
            || reader.word(after + 2) != Some(KEPT)
            || !reader.symbol(after + 3, Symbol::RightBrace)
        {
            return Err(format!(
                "quant-live-path-set: a cell set is exactly `{{#stale(K), #kept}}`: {}",
                shape()
            ));
        }
        at = after + 4;
        WrittenValue::StaleOrKept(Some(cell_kind(&kind)?))
    } else if let Some((kind, after)) = reader.application(at, FRESH) {
        at = after;
        WrittenValue::Fresh(Some(cell_kind(&kind)?))
    } else if let Some((kind, after)) = reader.application(at, STALE) {
        at = after;
        WrittenValue::Stale(Some(cell_kind(&kind)?))
    } else if let Some((slot, after)) = reader.application(at, SAME) {
        if !is_identifier(&slot) {
            return Err(value_error());
        }
        at = after;
        WrittenValue::Same(slot)
    } else {
        return Err(value_error());
    };
    let guard = if at == tokens.len() {
        None
    } else {
        Some(witness_guard(&reader, at, &shape)?)
    };
    Ok(Written::State {
        binder: binder.to_owned(),
        place,
        value,
        guard,
        text: format!("quant live {source}"),
    })
}

/// `when # P : TAG . FIELD == # X` from `at` to the end.
fn equality(reader: &Reader<'_>, at: usize) -> Option<WrittenEquality> {
    if reader.word(at) != Some("when")
        || !reader.symbol(at + 1, Symbol::Hash)
        || !reader.symbol(at + 3, Symbol::Colon)
    {
        return None;
    }
    let binder = reader.word(at + 2).filter(|name| is_identifier(name))?;
    let (member, after) = reader.member(at + 4)?;
    if !reader.symbol(after, Symbol::EqualEqual)
        || !reader.symbol(after + 1, Symbol::Hash)
        || after + 3 != reader.tokens.len()
    {
        return None;
    }
    let against = reader.word(after + 2).filter(|name| is_identifier(name))?;
    Some(WrittenEquality {
        binder: binder.to_owned(),
        member,
        against: against.to_owned(),
    })
}

/// Whether the text after `quant live` is a guarded removal,
/// `#H: #holds(P) |-> #absent when …`.
pub(super) fn is_guarded_removal(source: &str) -> bool {
    tokens(source).is_some_and(|tokens| {
        let reader = Reader { tokens: &tokens };
        reader.symbol(0, Symbol::Hash)
            && reader.symbol(3, Symbol::Hash)
            && reader.word(4) == Some("holds")
            && reader.word(11) == Some("when")
    })
}

/// Read a guarded removal as `(H, P, guard)`.
pub(super) fn guarded_removal_clause(
    source: &str,
) -> Result<(String, String, WrittenEquality), String> {
    let shape = || {
        format!(
            "quant-live-holds-shape: a guarded removal is `quant live #H: #holds(P) {MAPS_TO} \
             #absent when #D: TAG.FIELD == #X;`, not `quant live {source};`"
        )
    };
    let tokens = tokens(source).ok_or_else(shape)?;
    let reader = Reader { tokens: &tokens };
    if !reader.symbol(0, Symbol::Hash)
        || !reader.symbol(2, Symbol::Colon)
        || !reader.symbol(3, Symbol::Hash)
        || reader.word(4) != Some("holds")
        || !reader.symbol(5, Symbol::LeftParen)
        || !reader.symbol(7, Symbol::RightParen)
        || !reader.symbol(8, Symbol::MapsTo)
        || !reader.symbol(9, Symbol::Hash)
        || reader.word(10) != Some("absent")
    {
        return Err(shape());
    }
    let holder = reader
        .word(1)
        .filter(|name| is_identifier(name))
        .ok_or_else(shape)?;
    let retained = reader
        .word(6)
        .filter(|name| is_identifier(name))
        .ok_or_else(shape)?;
    let guard = equality(&reader, 11).ok_or_else(|| {
        format!(
            "quant-live-cell-equality: the guard is `when #D: TAG.FIELD == #X`: {}",
            shape()
        )
    })?;
    Ok((holder.to_owned(), retained.to_owned(), guard))
}

/// `resource_released(P, valid) when #D: TAG.FIELD == #X`: `Some((P,
/// guard))`, `None` for any text that is not a release with a member
/// equality, an error for a release with a malformed one.
pub(super) fn guarded_release(
    expression: &str,
) -> Result<Option<(String, WrittenEquality)>, String> {
    let Some(tokens) = tokens(expression) else {
        return Ok(None);
    };
    let reader = Reader { tokens: &tokens };
    if reader.word(0) != Some("resource_released")
        || !reader.symbol(1, Symbol::LeftParen)
        || reader.word(6) != Some("when")
        || !reader.symbol(7, Symbol::Hash)
        || !reader.symbol(9, Symbol::Colon)
    {
        return Ok(None);
    }
    let shape = || {
        format!(
            "quant-live-cell-equality: a guarded release is `resource_released(P, valid) when \
             #D: TAG.FIELD == #X`, not `{expression}`"
        )
    };
    if !reader.symbol(3, Symbol::Comma)
        || reader.word(4) != Some("valid")
        || !reader.symbol(5, Symbol::RightParen)
    {
        return Err(shape());
    }
    let released = reader
        .word(2)
        .filter(|name| is_identifier(name))
        .ok_or_else(shape)?;
    let guard = equality(&reader, 6).ok_or_else(shape)?;
    Ok(Some((released.to_owned(), guard)))
}

/// The arguments of `NAME ( A , B , … )`, each argument's tokens.
fn arguments(tokens: &[TokenKind]) -> Option<(&str, Vec<&[TokenKind]>)> {
    let TokenKind::Word(name) = tokens.first()? else {
        return None;
    };
    if tokens.get(1) != Some(&TokenKind::Symbol(Symbol::LeftParen))
        || tokens.last() != Some(&TokenKind::Symbol(Symbol::RightParen))
    {
        return None;
    }
    let inner = &tokens[2..tokens.len() - 1];
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (at, token) in inner.iter().enumerate() {
        match token {
            TokenKind::Symbol(Symbol::LeftParen) => depth += 1,
            TokenKind::Symbol(Symbol::RightParen) => depth = depth.checked_sub(1)?,
            TokenKind::Symbol(Symbol::Comma) if depth == 0 => {
                out.push(&inner[start..at]);
                start = at + 1;
            }
            _ => {}
        }
    }
    out.push(&inner[start..]);
    Some((name.as_str(), out))
}

fn one_word(tokens: &[TokenKind]) -> Option<&str> {
    match tokens {
        [TokenKind::Word(word)] if is_identifier(word) => Some(word.as_str()),
        _ => None,
    }
}

fn member_argument(tokens: &[TokenKind]) -> Option<CallbackMember> {
    let reader = Reader { tokens };
    let (member, end) = reader.member(0)?;
    (end == tokens.len()).then_some(member)
}

/// A lower-level member-cell clause: `resource_member_kind(P, TAG.FIELD, K)`,
/// `resource_member_released(P, TAG.FIELD, stale)` or `resource_on_return(OP,
/// V, resource_member_{out,stale_or_kept,alias}(P, TAG.FIELD[, S]))`. `None`
/// for any other text.
pub(super) fn lower_clause(expression: &str) -> Result<Option<Written>, String> {
    let Some(all) = tokens(expression) else {
        return Ok(None);
    };
    let malformed = || {
        format!(
            "quant-live-cell-shape: a member-cell clause is `{MEMBER_KIND}(P, TAG.FIELD, \
             descriptor)`, `{MEMBER_RELEASED}(P, TAG.FIELD, stale)` or `resource_on_return(OP, \
             V, {MEMBER_OUT}|{MEMBER_STALE_OR_KEPT}|{MEMBER_ALIAS}(P, TAG.FIELD[, S]))`, not \
             `{expression}`"
        )
    };
    let Some((name, args)) = arguments(&all) else {
        return Ok(None);
    };
    let text = expression.split_whitespace().collect::<Vec<_>>().join(" ");
    match name {
        MEMBER_KIND => {
            let [binder, member, kind] = args.as_slice() else {
                return Err(malformed());
            };
            Ok(Some(Written::Kind {
                binder: one_word(binder).ok_or_else(malformed)?.to_owned(),
                place: WrittenPlace::Member(member_argument(member).ok_or_else(malformed)?),
                kind: cell_kind(one_word(kind).ok_or_else(malformed)?)?,
                text,
            }))
        }
        MEMBER_RELEASED => {
            let [binder, member, state] = args.as_slice() else {
                return Err(malformed());
            };
            if one_word(state) != Some(STALE) {
                return Err(malformed());
            }
            Ok(Some(Written::State {
                binder: one_word(binder).ok_or_else(malformed)?.to_owned(),
                place: WrittenPlace::Member(member_argument(member).ok_or_else(malformed)?),
                value: WrittenValue::Stale(None),
                guard: None,
                text,
            }))
        }
        "resource_on_return" => {
            let [op, value, inner] = args.as_slice() else {
                return Ok(None);
            };
            let Some((inner_name, inner_args)) = arguments(inner) else {
                return Ok(None);
            };
            if ![MEMBER_OUT, MEMBER_STALE_OR_KEPT, MEMBER_ALIAS].contains(&inner_name) {
                return Ok(None);
            }
            let equal = match one_word(op) {
                Some("eq") => true,
                Some("ne") => false,
                _ => return Err(malformed()),
            };
            let constant = match value {
                [TokenKind::Integer(digits)] if digits == "0" => 0,
                [TokenKind::Symbol(Symbol::Minus), TokenKind::Integer(digits)] if digits == "1" => {
                    -1
                }
                _ => return Err(malformed()),
            };
            let guard = WitnessGuard::from_comparison(equal, constant).ok_or_else(malformed)?;
            let (binder, member, rest) = match inner_args.as_slice() {
                [binder, member, rest @ ..] => (binder, member, rest),
                _ => return Err(malformed()),
            };
            let written_value = match (inner_name, rest) {
                (MEMBER_OUT, []) => WrittenValue::Fresh(None),
                (MEMBER_STALE_OR_KEPT, []) => WrittenValue::StaleOrKept(None),
                (MEMBER_ALIAS, [slot]) => {
                    WrittenValue::Same(one_word(slot).ok_or_else(malformed)?.to_owned())
                }
                _ => return Err(malformed()),
            };
            Ok(Some(Written::State {
                binder: one_word(binder).ok_or_else(malformed)?.to_owned(),
                place: WrittenPlace::Member(member_argument(member).ok_or_else(malformed)?),
                value: written_value,
                guard: Some(guard),
                text,
            }))
        }
        _ => Ok(None),
    }
}

/// The written slot binders (`#S: #pointee`), by name.
pub(super) fn slot_binders(written: &[Written]) -> Vec<String> {
    written
        .iter()
        .filter_map(|clause| match clause {
            Written::State {
                binder,
                place: WrittenPlace::Pointee,
                ..
            }
            | Written::Kind {
                binder,
                place: WrittenPlace::Pointee,
                ..
            } => Some(binder.clone()),
            _ => None,
        })
        .collect()
}

/// The lower level of a slot cell: a parameter whose kind is a cell kind and
/// whose return-code dispositions are exactly `resource_out(S, nonnull)` in one
/// witness class and `resource_borrowed(S)` in its complement. Returns the
/// written clauses and the dispositions it consumed (their indices in
/// `conditional`), or nothing when the shape does not match (the ordinary
/// rules then read, and reject, them).
pub(super) fn slot_lower_level(
    parameter: &str,
    kind: ResourceKind,
    dispositions: &[(
        usize,
        qcore_schemas::relational_post::ReturnRelation,
        String,
    )],
) -> Option<Vec<Written>> {
    use qcore_schemas::relational_post::RelationOp;
    let [first, second] = dispositions else {
        return None;
    };
    let read = |(_, relation, inner): &(
        usize,
        qcore_schemas::relational_post::ReturnRelation,
        String,
    )| {
        let guard = WitnessGuard::from_comparison(
            match relation.op {
                RelationOp::Eq => true,
                RelationOp::Ne => false,
                _ => return None,
            },
            relation.constant,
        )?;
        let words = inner
            .replace(['(', ')', ','], " ")
            .split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let out = match words
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice()
        {
            ["resource_out", p, "nonnull"] if *p == parameter => true,
            ["resource_borrowed", p] if *p == parameter => false,
            _ => return None,
        };
        Some((guard, out))
    };
    let (left, right) = (read(first)?, read(second)?);
    let (out, borrowed) = match (left, right) {
        ((guard, true), (other, false)) | ((other, false), (guard, true)) => (guard, other),
        _ => return None,
    };
    (borrowed == out.complement() && kind == ResourceKind::Descriptor).then(|| {
        vec![
            Written::State {
                binder: parameter.to_owned(),
                place: WrittenPlace::Pointee,
                value: WrittenValue::Fresh(None),
                guard: Some(out),
                text: format!("resource_on_return({}) for {parameter}", out.spelling()),
            },
            Written::Kind {
                binder: parameter.to_owned(),
                place: WrittenPlace::Pointee,
                kind,
                text: format!("resource_kind({parameter}, descriptor)"),
            },
        ]
    })
}

/// Validate one declaration's cell clauses and elaborate them into the
/// canonical set.
///
/// # Errors
/// The module's rejection.
#[allow(clippy::too_many_arguments)]
pub(super) fn declaration(
    written: &[Written],
    releases: &[(String, WrittenEquality)],
    removal_guards: &[WrittenEquality],
    removal_count: usize,
    contract: &ResourceFunctionContract,
    parameter_types: &[String],
    names: &BTreeMap<&str, usize>,
    return_type: &str,
) -> Result<DescriptorCells, String> {
    let index = |name: &str, text: &str| {
        names.get(name).copied().ok_or_else(|| {
            format!("quant-live-cell-binder: `{text}`: `#{name}` names no parameter")
        })
    };
    let pointer_binder = |at: usize, text: &str, slot: bool| -> Result<(), String> {
        let types: &[&str] = if slot {
            &["ptr<mut, u8>"]
        } else {
            &["ptr<mut, u8>", "ptr<const, u8>"]
        };
        if !types.contains(&parameter_types[at].as_str())
            || contract.parameters[at] != ResourceParameterMode::Borrowed
            || contract.parameter_kinds.contains_key(&at)
        {
            return Err(format!(
                "quant-live-cell-binder: `{text}`: the formal is a depth-one pointer{}, \
                 `resource_borrowed`, with no kind",
                if slot { " to mutable storage" } else { "" }
            ));
        }
        Ok(())
    };
    let place_of = |binder: &str, place: &WrittenPlace, text: &str| -> Result<CellPlace, String> {
        let at = index(binder, text)?;
        Ok(match place {
            WrittenPlace::Member(member) => {
                pointer_binder(at, text, false)?;
                CellPlace::Member {
                    parameter: at,
                    member: member.clone(),
                }
            }
            WrittenPlace::Pointee => {
                pointer_binder(at, text, true)?;
                CellPlace::Slot { parameter: at }
            }
        })
    };
    let mut kinds: Vec<(CellPlace, ResourceKind)> = Vec::new();
    fn add_kind(
        kinds: &mut Vec<(CellPlace, ResourceKind)>,
        place: CellPlace,
        kind: ResourceKind,
        text: &str,
    ) -> Result<(), String> {
        match kinds.iter().find(|(stated, _)| *stated == place) {
            Some((_, stated)) if *stated != kind => Err(format!(
                "quant-live-cell-kind: `{text}`: the cell already has another kind"
            )),
            Some(_) => Ok(()),
            None => {
                kinds.push((place, kind));
                Ok(())
            }
        }
    }
    let mut states: Vec<CellState> = Vec::new();
    let mut same_texts: Vec<(CellState, String)> = Vec::new();
    for clause in written {
        match clause {
            Written::Kind {
                binder,
                place,
                kind,
                text,
            } => {
                let place = place_of(binder, place, text)?;
                add_kind(&mut kinds, place, *kind, text)?;
            }
            Written::State {
                binder,
                place,
                value,
                guard,
                text,
            } => {
                let place = place_of(binder, place, text)?;
                let (value, kind) = match value {
                    WrittenValue::Fresh(kind) => (CellValue::Fresh, *kind),
                    WrittenValue::Stale(kind) => (CellValue::Stale, *kind),
                    WrittenValue::StaleOrKept(kind) => (CellValue::StaleOrKept, *kind),
                    WrittenValue::Same(slot) => {
                        let slot = index(slot, text)?;
                        if !matches!(place, CellPlace::Member { .. }) {
                            return Err(format!(
                                "quant-live-cell-value: `{text}`: `#same(S)` names a second \
                                 name of a slot's descriptor at a member"
                            ));
                        }
                        (CellValue::Same { slot }, None)
                    }
                };
                if let Some(kind) = kind {
                    add_kind(&mut kinds, place.clone(), kind, text)?;
                }
                if matches!(value, CellValue::Fresh | CellValue::Same { .. }) && guard.is_none() {
                    return Err(format!(
                        "quant-live-cell-guard: `{text}`: a creation is guarded by its witness, \
                         the call's result"
                    ));
                }
                let state = CellState {
                    place,
                    value,
                    guard: *guard,
                };
                if let CellValue::Same { .. } = value {
                    same_texts.push((state.clone(), text.clone()));
                }
                if let Some(other) = states.iter().find(|other| {
                    other.place == state.place && other.guard == state.guard && **other != state
                }) {
                    return Err(format!(
                        "quant-live-cell-duplicate: `{text}`: the cell already has the state \
                         {:?} in this class",
                        other.value
                    ));
                }
                if !states.contains(&state) {
                    states.push(state);
                }
            }
        }
    }
    // One witness: every guard of one pair, a place unguarded or guarded,
    // never both.
    let guards = states
        .iter()
        .filter_map(|state| state.guard)
        .collect::<Vec<_>>();
    if let Some(first) = guards.first()
        && guards
            .iter()
            .any(|guard| *guard != *first && *guard != first.complement())
    {
        return Err(
            "quant-live-cell-guard: every cell state of a declaration is guarded by one witness \
             pair (`!= -1`/`== -1` or `== 0`/`!= 0`)"
                .into(),
        );
    }
    for state in &states {
        if states.iter().any(|other| {
            other.place == state.place && other.guard.is_some() != state.guard.is_some()
        }) {
            return Err(
                "quant-live-cell-duplicate: a cell is stated for every return or per class, not \
                 both"
                    .into(),
            );
        }
    }
    if !guards.is_empty()
        && (contract.result != ResourceReturnMode::None
            || !matches!(return_type, "i8" | "i16" | "i32" | "i64" | "isize"))
    {
        return Err(
            "quant-live-cell-guard: a witness is a signed integer result with \
             `resource_return_none()`"
                .into(),
        );
    }
    for (state, text) in &same_texts {
        let CellValue::Same { slot } = state.value else {
            continue;
        };
        let slot_place = CellPlace::Slot { parameter: slot };
        if !states.iter().any(|other| {
            other.place == slot_place
                && other.value == CellValue::Fresh
                && other.guard == state.guard
        }) {
            return Err(format!(
                "quant-live-cell-value: `{text}`: `#same(S)` names a slot stated `#fresh` in the \
                 same class"
            ));
        }
        let slot_kind = kinds
            .iter()
            .find(|(place, _)| *place == slot_place)
            .map(|(_, kind)| *kind);
        if let Some(kind) = slot_kind {
            add_kind(&mut kinds, state.place.clone(), kind, text)?;
        }
    }
    for state in &states {
        if !kinds.iter().any(|(place, _)| *place == state.place) {
            return Err(format!(
                "quant-live-cell-kind: the cell {:?} states no kind (`#fresh(K)`, `#stale(K)` or \
                 a kind clause)",
                state.place
            ));
        }
    }
    if let Some((place, _)) = kinds
        .iter()
        .find(|(place, _)| !states.iter().any(|state| state.place == *place))
    {
        return Err(format!(
            "quant-live-cell-kind: a kind is stated for the cell {place:?}, which no state names"
        ));
    }
    // The member-equality guard: one per declaration.
    let mut guard: Option<MemberEquality> = None;
    let mut read_guard = |written: &WrittenEquality| -> Result<(), String> {
        let text = format!(
            "when #{}: {} == #{}",
            written.binder,
            written.member.spelling(),
            written.against
        );
        let parameter = index(&written.binder, &text)?;
        pointer_binder(parameter, &text, false)?;
        let against = index(&written.against, &text)?;
        if parameter_types[against] != "i32" {
            return Err(format!(
                "quant-live-cell-equality: `{text}`: `#{}` is a descriptor formal (`i32`)",
                written.against
            ));
        }
        let read = MemberEquality {
            parameter,
            member: written.member.clone(),
            against,
        };
        match &guard {
            Some(existing) if *existing != read => Err(format!(
                "quant-live-cell-equality: `{text}`: a declaration states one member-equality guard"
            )),
            _ => {
                guard = Some(read);
                Ok(())
            }
        }
    };
    let mut guarded_releases = Vec::new();
    for (released, written) in releases {
        read_guard(written)?;
        let at = index(released, "resource_released")?;
        if parameter_types[at] != "i32" || guarded_releases.contains(&at) {
            return Err(format!(
                "quant-live-cell-equality: `{released}`: a guarded release names one descriptor \
                 formal (`i32`) once"
            ));
        }
        guarded_releases.push(at);
    }
    for written in removal_guards {
        read_guard(written)?;
    }
    let guarded_removal = !removal_guards.is_empty();
    if guarded_removal && removal_guards.len() != removal_count {
        return Err(
            "quant-live-cell-equality: every removal of a declaration with a guarded removal is \
             guarded"
                .into(),
        );
    }
    if let Some(guard) = &guard
        && !guarded_releases.contains(&guard.against)
    {
        return Err(
            "quant-live-cell-equality: the guard's descriptor formal is one the declaration \
             releases under it"
                .into(),
        );
    }
    states.sort();
    kinds.sort();
    guarded_releases.sort_unstable();
    Ok(DescriptorCells {
        kinds,
        states,
        guarded_releases,
        guarded_removal,
        guard,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shorthand_value_reads() {
        for (text, value) in [
            (
                "#data: mk_event.fd |-> #fresh(descriptor) when #result != -1",
                "fresh",
            ),
            ("#data: mk_event.fd |-> #stale(descriptor)", "stale"),
            (
                "#data: mk_event.fd |-> {#stale(descriptor), #kept} when #result == -1",
                "set",
            ),
            (
                "#data: mk_event.fd |-> #same(r_fd) when #result == 0",
                "same",
            ),
            (
                "#r_fd: #pointee |-> #fresh(descriptor) when #result == 0",
                "fresh",
            ),
        ] {
            assert!(is_cell_clause(text), "{text}");
            let read = quant_cell_clause(text).expect(text);
            let Written::State { value: read, .. } = read else {
                panic!("{text}");
            };
            let word = match read {
                WrittenValue::Fresh(_) => "fresh",
                WrittenValue::Stale(_) => "stale",
                WrittenValue::StaleOrKept(_) => "set",
                WrittenValue::Same(_) => "same",
            };
            assert_eq!(word, value, "{text}");
        }
        assert!(!is_cell_clause("#rko: t.a |-> #stale"));
        assert!(!is_cell_clause("#rko: t.a |-> {#stale, #kept}"));
    }

    #[test]
    fn every_other_shape_rejects_with_a_named_diagnostic() {
        for (text, diagnostic) in [
            (
                "#data: mk_event.fd |-> #fresh(heap) when #result != -1",
                "quant-live-cell-kind:",
            ),
            (
                "#data: mk_event.fd |-> #fresh(stream) when #result != -1",
                "quant-live-cell-kind:",
            ),
            (
                "#data: mk_event.fd |-> #fresh(descriptor) when #result != 1",
                "quant-live-cell-guard:",
            ),
            (
                "#data: mk_event.fd |-> #fresh(descriptor) when #result < 0",
                "quant-live-cell-guard:",
            ),
            (
                "#data: mk_event.fd |-> #fresh(descriptor) when #x != -1",
                "quant-live-cell-guard:",
            ),
            (
                "#data: mk_event.fd |-> {#stale(descriptor), #fresh}",
                "quant-live-path-set:",
            ),
            (
                "#data: mk_event |-> #fresh(descriptor) when #result != -1",
                "quant-live-path-member:",
            ),
            (
                "#data: mk_event.fd -> #fresh(descriptor)",
                "quant-live-maps-to:",
            ),
            (
                "#data: #target |-> #fresh(descriptor)",
                "quant-live-cell-shape:",
            ),
            (
                "#fresh: mk_event.fd |-> #stale(descriptor)",
                "quant-live-cell-binder:",
            ),
        ] {
            let error = quant_cell_clause(text).expect_err(text);
            assert!(error.starts_with(diagnostic), "{text}: {error}");
        }
    }

    #[test]
    fn the_guards_and_the_lower_level_read() {
        let (holder, retained, guard) = guarded_removal_clause(
            "#loop: #holds(data) |-> #absent when #data: mk_event.fd == #r_fd",
        )
        .expect("reads");
        assert_eq!((holder.as_str(), retained.as_str()), ("loop", "data"));
        assert_eq!(guard.against, "r_fd");
        assert!(is_guarded_removal(
            "#loop: #holds(data) |-> #absent when #data: mk_event.fd == #r_fd"
        ));
        assert!(!is_guarded_removal("#loop: #holds(data) |-> #absent"));
        let (released, _) =
            guarded_release("resource_released(w_fd, valid) when #data: mk_event.fd == #r_fd")
                .expect("reads")
                .expect("a guarded release");
        assert_eq!(released, "w_fd");
        assert_eq!(guarded_release("resource_released(w_fd, valid)"), Ok(None));
        assert!(
            guarded_release("resource_released(w_fd, sentinel) when #data: mk_event.fd == #r_fd")
                .is_err()
        );
        for text in [
            "resource_member_kind(data, mk_event.fd, descriptor)",
            "resource_member_released(data, mk_event.fd, stale)",
            "resource_on_return(ne, -1, resource_member_out(data, mk_event.fd))",
            "resource_on_return(eq, -1, resource_member_stale_or_kept(data, mk_event.fd))",
            "resource_on_return(eq, 0, resource_member_alias(data, mk_event.fd, r_fd))",
        ] {
            assert!(lower_clause(text).expect(text).is_some(), "{text}");
        }
        assert_eq!(
            lower_clause("resource_on_return(eq, 0, resource_consumed(p, nonnull))"),
            Ok(None)
        );
        assert!(lower_clause("resource_on_return(lt, 0, resource_member_out(d, t.f))").is_err());
        assert!(lower_clause("resource_member_released(d, t.f, kept)").is_err());
    }
}
