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

//! Exact source lowering for `source.loop.v1` — the checked bounded counted
//! loop (card `LOOP-V1`).
//!
//! ```text
//! stmt  ::= [attr] "for" NAME "in" "0" ".." BOUND "{" stmt* "}"
//! attr  ::= "[|" "#" "unroll" "," "where" "#" "bound" "<" INTEGER "|]"
//! BOUND ::= INTEGER | NAME
//! ```
//!
//! # The unrolling attribute (card `LOOP-UNROLL-BUDGET`)
//!
//! `attr` is a **statement-adjacent** clause list: it licenses the single `for`
//! immediately following it and nothing else. Its brackets are the **existing**
//! `[|`/`|]` of the quant-index type's readable alias, borrowed rather than
//! invented; its `#` is a new one-byte token, promoted from a lexer error on
//! card `OR-AND-LOWERING`'s precedent for `^`, and census-clean at **zero**
//! occurrences in code position across every `.qcore`/`.qc` file in the tree.
//! The head is recognised as `[|` **followed by `#`**, so
//! `quant[| n where n := 5 |]` — eight code occurrences in the pinned corpus —
//! is untouched.
//!
//! The three-layer decision the clause list feeds is
//! [`qcore_core::source_loop::admit_unroll`]'s and is stated there. This module
//! reads the list and nothing else.
//!
//! # One reserved word, zero new tokens, and one contextual clause word
//!
//! **`for`** is the only word this card reserves. The constraint-14 census over
//! every `.qcore`/`.qc` file in the tree — 418 of them — found `for` **zero**
//! times outside a `//` comment, so `for x …` at a statement head was a `Q000`
//! parse failure before this card and the reservation collides with nothing.
//!
//! **`in` is deliberately not reserved.** The same census found it six times in
//! code position: four are parameter names that the referenced-names filter
//! already shields (a word followed by `:`), but two are genuine **references**
//! inside the pinned contract corpus —
//! `ensures normal resource_borrowed(in);` in
//! `providers/quirin-provider-fluent-bit/contracts/fluent_bit/input_chunk_helpers.qv` and in a gateway test
//! fixture, both naming the `in` parameter of
//! `extern fn generate_chunk_name(in: ptr<mut, u8>, …)`. That is exactly the
//! shape card `STRING-SLICE` measured for `len`, and it takes exactly that
//! card's answer: **narrow the rule rather than edit a pinned corpus**. `in` is
//! skipped by `frontend::referenced_names` at the one position a `for` header
//! puts it — immediately after `for NAME` — and is an ordinary word everywhere
//! else.
//!
//! **`..`** is two existing `Symbol::Dot` tokens read positionally, on the
//! precedent that `||` was two `Symbol::Pipe` tokens before card
//! `OR-AND-LOWERING` gave it a token of its own. The lexer is untouched.
//!
//! # `0..BOUND` and not `LOW..HIGH`
//!
//! The low end is the literal `0`, written out rather than implied. Spelling it
//! keeps the form readable as a range while admitting exactly one range per
//! bound, so the iteration values are `0, 1, …, BOUND-1` with nothing to derive
//! — and a reader who writes `1..n` gets a refusal that names the rule instead
//! of a loop that runs a different number of times than it looks like it does.
//! A general low end is the follow-on `LOOP-RANGE-LOW`.
//!
//! # What this module does not decide
//!
//! It reads a header and nothing else. How many times the body runs, what one
//! iteration may do to custody, and what the body's statements *are* belong to
//! [`qcore_core::source_loop`] and to the admitting route respectively — a loop
//! body's statements are the route's own statements, read by the route's own
//! reader, so a shape one admits and the other refuses is not expressible.

use qcore_core::scalar_types::{RecordedTarget, ScalarType};
use qcore_core::source_loop::{
    BoundRefusal, LoopBudget, LoopRefusal, MAX_DECLARED_LIMIT, MAX_ITERATIONS, UnrollLicense,
    budget_refusal, typed_bound,
};
use qcore_core::std_buffer::LoopIndex;

use crate::frontend::{
    DeclarationKind, SemanticPass, SourceBlock, SourceDiagnostic, SourceProgram, StatementKind,
};
use crate::lexer::{Span, Symbol, Token, TokenKind};

/// The statement head. The one word this card reserves.
pub const FOR_KEYWORD: &str = "for";

/// The range clause word. **Contextual**, never globally reserved — see the
/// module header for the census site that forbids a global reservation.
pub const IN_KEYWORD: &str = "in";

/// The attribute's own clause word: `[| #unroll, … |]` (card
/// `LOOP-UNROLL-BUDGET`).
///
/// Census-clean at **zero** occurrences anywhere in the repository — no
/// `.qcore`/`.qc` file, in code or in comment text, and no Rust string literal.
/// It is nevertheless **not** in `is_reserved_source_word`, because it needs no
/// reservation: a `[| … |]` clause list contributes no referenced name at all
/// (`frontend::lexical_references`), so a program may still bind `unroll`.
pub const UNROLL_CLAUSE: &str = "unroll";

/// The condition clause's introducer, `where`. **Contextual**, and deliberately
/// not reserved: the constraint-14 census found it **eight** times in code
/// position inside `quant<n where n := 5>` clauses of the pinned corpus, which
/// is the `len`/`in` shape again, and the answer is the same one — the word is
/// read only inside this clause list and is an ordinary word everywhere else.
pub const WHERE_CLAUSE: &str = "where";

/// The quantity the condition constrains, `#bound`. The word `bound` is
/// **already** globally reserved (the epoch profile's `bound N effort M;`
/// declaration clause), so this card reserves nothing new for it; the `#`
/// marker is what distinguishes the attribute's use from that one.
pub const BOUND_CLAUSE: &str = "bound";

/// The attribute's clause-list tokens: everything from `[|` through `|]`.
#[must_use]
pub fn attribute_window(tokens: &[Token], span: Span) -> Vec<&Token> {
    tokens
        .iter()
        .filter(|token| span.start <= token.span.start && token.span.end <= span.end)
        .collect()
}

/// Read one `[| #unroll, where #bound < N |]` clause list.
///
/// Every refusal here is a **shape** refusal and lands at `Q097`, a condition
/// extension of the band card `LOOP-V1` allocated, per the standing Q-space
/// note (`Q-NAMESPACE-DECISION`): a new refusal is a new condition of an
/// existing code, never a new code. The one exception is the declared numeral
/// itself — a value problem is the *bound* band's, `Q098`, and is decided by
/// [`qcore_core::source_loop::admit_unroll`] rather than here.
///
/// The `where` clause is **required**. `[| #unroll |]` alone would have to mean
/// "license up to the ceiling", which is a declaration with nothing in it to
/// check — and the whole point of the form is that the condition a programmer
/// writes is compared against the bound the checker computes.
pub fn read_loop_attribute(
    window: &[&Token],
    span: Span,
) -> Result<UnrollLicense, SourceDiagnostic> {
    let shape = |message: &str| diagnostic("Q097", span, message.to_owned());

    if !symbol(window, 0, Symbol::QuantOpen) || !symbol(window, 1, Symbol::Hash) {
        return Err(shape(
            "`source.loop.v1` reads an unrolling attribute as `[| #unroll, where #bound < N |]`",
        ));
    }
    match word(window, 2) {
        Some(UNROLL_CLAUSE) => {}
        Some(other) => {
            return Err(diagnostic(
                "Q097",
                span,
                format!(
                    "`#{other}` is not a clause `source.loop.v1` reads; the one attribute this \
                     revision spells is `[| #unroll, where #bound < N |]`, and an unknown clause \
                     word is refused rather than ignored"
                ),
            ));
        }
        None => {
            return Err(shape(
                "`source.loop.v1` reads an unrolling attribute as `[| #unroll, where #bound < N \
                 |]`; `#` must be followed by a clause word",
            ));
        }
    }
    if !symbol(window, 3, Symbol::Comma) {
        return Err(shape(
            "`[| #unroll |]` licenses nothing, because there is no condition in it to check \
             against the bound the checker computes; `source.loop.v1` requires the full form \
             `[| #unroll, where #bound < N |]`",
        ));
    }
    if word(window, 4) != Some(WHERE_CLAUSE) {
        return Err(shape(
            "`source.loop.v1` introduces the attribute's condition with `where`: \
             `[| #unroll, where #bound < N |]`",
        ));
    }
    if !symbol(window, 5, Symbol::Hash) || word(window, 6) != Some(BOUND_CLAUSE) {
        return Err(shape(
            "the only quantity `source.loop.v1`'s attribute constrains is the loop's bound, \
             written `#bound`: `[| #unroll, where #bound < N |]`",
        ));
    }
    if !symbol(window, 7, Symbol::Less) {
        return Err(shape(
            "`source.loop.v1`'s attribute states a strict upper bound, `#bound < N`; `<` is the \
             one comparison it reads, so the licensed bounds are exactly `1 … N-1`",
        ));
    }
    let Some(limit) = integer(window, 8) else {
        return Err(shape(
            "`#bound < N` names N as an integer literal written at the site; a licence the \
             checker has to compute is not one a reader can check against the loop",
        ));
    };
    if !symbol(window, 9, Symbol::QuantClose) || window.len() != 10 {
        return Err(shape(
            "`source.loop.v1` reads exactly `[| #unroll, where #bound < N |]`; a clause list with \
             anything further in it is refused rather than partly read",
        ));
    }
    // The declared numeral's *value* is the bound band's business and is
    // reported at `Q098` rather than here. Two values are decidable from the
    // clause list **alone**, before any loop is read: one above the ceiling, and
    // one (`#bound < 0`, `#bound < 1`) that licenses no bound at all, since the
    // construct counts at least one iteration. Neither names a bound, because
    // the reader has not read one.
    if limit > i64::from(MAX_DECLARED_LIMIT) {
        return Err(refusal_diagnostic(
            &LoopRefusal::Bound(BoundRefusal::LicenseAboveCeiling {
                limit: u32::try_from(limit).unwrap_or(u32::MAX),
            }),
            span,
        ));
    }
    if limit < 2 {
        return Err(refusal_diagnostic(
            &LoopRefusal::Bound(BoundRefusal::LicenseLicensesNothing {
                limit: u32::try_from(limit.max(0)).unwrap_or(0),
            }),
            span,
        ));
    }
    Ok(UnrollLicense {
        limit: u32::try_from(limit).expect("the limit is inside 2..=MAX_DECLARED_LIMIT"),
    })
}

/// What the lowering's constant tier knows of a bound name (card
/// `W48-QR-SCALAR-TYPES-R5`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundValue {
    /// An `i32` of the tier as it was before the card — a literal `let` or an
    /// `extent` binding. Its loop is the loop it always was.
    I32(i32),
    /// A value the tier derived — a conversion or a value statement over the
    /// tier's own values, folded by the shared decision — of any §G02t type.
    Derived { ty: ScalarType, value: i128 },
    /// A derived value of `isize`/`usize` the tier could not fold because no
    /// recorded target gives the type a width.
    Widthless { ty: ScalarType },
}

/// A read `for NAME in 0..BOUND` header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoopHeader {
    pub variable: String,
    pub count: u32,
    /// The bound's §G02t type, and so the index's, with the bound's name when
    /// it is not an `i32` (card `W48-QR-SCALAR-TYPES-R5`).
    pub index: LoopIndex,
}

fn diagnostic(code: &'static str, span: Span, message: impl Into<String>) -> SourceDiagnostic {
    SourceDiagnostic {
        code,
        pass: SemanticPass::Loop,
        span,
        message: message.into(),
    }
}

/// Present a [`LoopRefusal`] as this pass's diagnostic, at its own code.
#[must_use]
pub fn refusal_diagnostic(refusal: &LoopRefusal, span: Span) -> SourceDiagnostic {
    diagnostic(refusal.code(), span, refusal.to_string())
}

/// The header tokens of a `for` statement: everything before its body's `{`.
#[must_use]
pub fn header_window(tokens: &[Token], span: Span) -> Vec<&Token> {
    let mut window: Vec<&Token> = tokens
        .iter()
        .filter(|token| span.start <= token.span.start && token.span.end <= span.end)
        .collect();
    if let Some(brace) = window
        .iter()
        .position(|token| token.kind == TokenKind::Symbol(Symbol::LeftBrace))
    {
        window.truncate(brace);
    }
    window
}

fn word<'a>(window: &'a [&Token], cursor: usize) -> Option<&'a str> {
    match window.get(cursor).map(|token| &token.kind) {
        Some(TokenKind::Word(word)) => Some(word.as_str()),
        _ => None,
    }
}

fn symbol(window: &[&Token], cursor: usize, expected: Symbol) -> bool {
    matches!(
        window.get(cursor).map(|token| &token.kind),
        Some(TokenKind::Symbol(symbol)) if *symbol == expected
    )
}

fn integer(window: &[&Token], cursor: usize) -> Option<i64> {
    match window.get(cursor).map(|token| &token.kind) {
        Some(TokenKind::Integer(text)) => text.parse::<i64>().ok(),
        _ => None,
    }
}

/// Read one `for NAME in 0..BOUND` header.
///
/// `constants` is the admitting route's check-time-constant environment: the
/// names it has bound to integers at this point in the entry. A bound that is a
/// name resolves through it, which is the same decidable tier
/// `source.branch.v1` established for a condition and `source.buffer.v1` for an
/// index — and it is what `STD-BUFFER-COPY` needs, because a copy's length is
/// usually a name.
/// `budget` carries the `[| #unroll, where #bound < N |]` clause list immediately
/// preceding this statement, if one was written and read. Card `LOOP-UNROLL-BUDGET`'s
/// three-layer budget is applied after the bound is known and **before** the
/// body is looked at, so an over-budget loop costs the checking route nothing.
///
/// Card `W48-QR-SCALAR-TYPES-R5`: `known` resolves a name to its §G02t type
/// and number in that environment, and the header's `index` is the bound's
/// type — a literal bound, as before, an `i32`. A bound of another type is
/// decided by [`qcore_core::source_loop::typed_bound`] under `target`; an
/// `i32` bound takes exactly the path it always took.
pub fn read_loop_header(
    window: &[&Token],
    span: Span,
    known: impl Fn(&str) -> Option<BoundValue>,
    target: Option<&RecordedTarget>,
    budget: LoopBudget,
) -> Result<LoopHeader, SourceDiagnostic> {
    if word(window, 0) != Some(FOR_KEYWORD) {
        return Err(diagnostic(
            "Q097",
            span,
            "`source.loop.v1` reads `for NAME in 0..BOUND { … }`",
        ));
    }
    let Some(variable) = word(window, 1) else {
        return Err(diagnostic(
            "Q097",
            span,
            "`for` binds one index name; `source.loop.v1` reads `for NAME in 0..BOUND { … }`",
        ));
    };
    if word(window, 2) != Some(IN_KEYWORD) {
        return Err(diagnostic(
            "Q097",
            span,
            "`source.loop.v1` reads `for NAME in 0..BOUND { … }`; the range is introduced by `in`",
        ));
    }
    if integer(window, 3) != Some(0) {
        return Err(diagnostic(
            "Q097",
            span,
            "`source.loop.v1` counts from 0, written out: `for NAME in 0..BOUND { … }`. A range \
             with another low end is refused rather than silently re-based",
        ));
    }
    if !symbol(window, 4, Symbol::Dot) || !symbol(window, 5, Symbol::Dot) {
        return Err(diagnostic(
            "Q097",
            span,
            "`source.loop.v1` spells its range `0..BOUND`",
        ));
    }
    // `BOUND` is a literal or a name this entry bound to a check-time constant.
    let bound = match window.get(6).map(|token| &token.kind) {
        Some(TokenKind::Integer(text)) => text.parse::<i64>().map_err(|_| {
            refusal_diagnostic(
                &LoopRefusal::Bound(BoundRefusal::NotConstant {
                    spelling: text.clone(),
                }),
                span,
            )
        })?,
        Some(TokenKind::Word(name)) => match known(name) {
            Some(BoundValue::I32(value)) => i64::from(value),
            // Card `W48-QR-SCALAR-TYPES-R5`: a derived value of `isize` or
            // `usize` with no recorded target has no number; the bound is
            // refused for the width, never guessed.
            Some(BoundValue::Widthless { ty }) => {
                return Err(refusal_diagnostic(
                    &LoopRefusal::Bound(BoundRefusal::NeedsRecordedTarget { ty }),
                    span,
                ));
            }
            // Card `W48-QR-SCALAR-TYPES-R5`: a bound the tier derived — of any
            // type, `i32` included — is decided by the typed decision, and the
            // loop names it.
            Some(BoundValue::Derived { ty, value }) => {
                let count = typed_bound(name, ty, value, target)
                    .map_err(|refusal| refusal_diagnostic(&LoopRefusal::Bound(refusal), span))?;
                if let Some(refusal) = budget_refusal(count, budget) {
                    return Err(refusal_diagnostic(&LoopRefusal::Bound(refusal), span));
                }
                return Ok(LoopHeader {
                    variable: variable.to_owned(),
                    count,
                    index: LoopIndex::Typed {
                        ty,
                        bound: name.clone(),
                    },
                });
            }
            None => {
                return Err(refusal_diagnostic(
                    &LoopRefusal::Bound(BoundRefusal::NotConstant {
                        spelling: name.clone(),
                    }),
                    span,
                ));
            }
        },
        // A negative literal lexes as `Minus` then `Integer`, so this is where
        // `for i in 0..-1` arrives.
        Some(TokenKind::Symbol(Symbol::Minus)) => match integer(window, 7) {
            Some(value) => -value,
            None => {
                return Err(diagnostic(
                    "Q097",
                    span,
                    "`source.loop.v1` reads `for NAME in 0..BOUND { … }`",
                ));
            }
        },
        _ => {
            return Err(diagnostic(
                "Q097",
                span,
                "`source.loop.v1` reads `for NAME in 0..BOUND { … }`",
            ));
        }
    };

    let refusal = if bound < 0 {
        Some(BoundRefusal::Negative {
            value: i128::from(i32::try_from(bound).unwrap_or(i32::MIN)),
        })
    } else if bound == 0 {
        Some(BoundRefusal::Zero)
    } else if bound > i64::from(MAX_ITERATIONS) {
        Some(BoundRefusal::TooLarge {
            value: i128::from(bound),
        })
    } else {
        None
    };
    if let Some(refusal) = refusal {
        return Err(refusal_diagnostic(&LoopRefusal::Bound(refusal), span));
    }

    let count = u32::try_from(bound).expect("the bound is inside 1..=MAX_ITERATIONS");
    // Card `LOOP-UNROLL-BUDGET`: the three-layer decision, taken by the core
    // over two numbers and reported at this band's own `Q098`.
    if let Some(refusal) = budget_refusal(count, budget) {
        return Err(refusal_diagnostic(&LoopRefusal::Bound(refusal), span));
    }

    Ok(LoopHeader {
        variable: variable.to_owned(),
        count,
        index: LoopIndex::I32,
    })
}

/// Whether this program writes a `for` loop anywhere, and where the first one
/// is.
///
/// Used to pin the measured refusal of every route that does **not** admit the
/// construct, so "correctly refused" is a statement a test can make.
#[must_use]
pub fn loop_span(program: &SourceProgram) -> Option<Span> {
    for declaration in &program.declarations {
        if declaration.value.kind != DeclarationKind::Function {
            continue;
        }
        if let Some(body) = &declaration.value.body
            && let Some(span) = block_loop_span(body)
        {
            return Some(span);
        }
    }
    None
}

/// The span of the first `for` statement in one block, at any depth.
#[must_use]
pub fn block_loop_span(block: &SourceBlock) -> Option<Span> {
    for statement in &block.statements {
        if matches!(statement.value, StatementKind::For { .. }) {
            return Some(statement.span);
        }
        for nested in statement.value.nested_blocks() {
            if let Some(span) = block_loop_span(nested) {
                return Some(span);
            }
        }
    }
    None
}

/// Whether this program writes a `for` loop anywhere.
#[must_use]
pub fn has_source_loop(program: &SourceProgram) -> bool {
    loop_span(program).is_some()
}
