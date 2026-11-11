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

//! Exact source reading for `source.region.v1` — the checked error region and
//! its quantitative cleanup-pack opening (card `QUANT-REGION-SOURCE`).
//!
//! ```text
//! stmt   ::= "error" "region" NAME "{" body "}" catch+
//! catch  ::= "catch" "(" ERROR ")" [ "quant" "open" NAME "as" BINDING ] "{" handler "}"
//! ```
//!
//! # What the kernel operation means, and why the surface is spelled this way
//!
//! This module spells [`qcore_core::error_region`]'s existing operations and
//! adds no semantics. In that model an **error region** is a lexical frame that
//! records the ledger it was entered with (its *baseline*) and accumulates one
//! cleanup action per resource acquired inside it, plus a **strict credit
//! remainder** — the region's quantity. The frame closes lexically, and the two
//! closes differ:
//!
//! * on the **normal** edge the frame's remaining actions run in reverse
//!   acquisition order and the remainder must be **zero**
//!   ([`qcore_core::error_region::ErrorRegionError::UnusedStrictCredit`]);
//! * on an **error** edge the frame *seals* into a
//!   [`qcore_core::error_region::CleanupPack`] carrying the region, the error,
//!   the baseline, the pending actions and the remainder.
//!
//! `quant open REGION as B` is valid **only on a catch clause**, and it does
//! not discard the pack: it binds the sealed pack as a **linear capability**
//! named `B`, which `discharge B;` must consume **exactly once** before the
//! handler leaves by any edge. That linearity is the whole content of the word
//! *quant* here — the capability is a quantity of exactly one — and it is why
//! `RegionOp::QuantOpen` exists as an operation at all: an opening written
//! anywhere but a catch clause is the model's `Q302`.
//!
//! # This module reads a header and nothing else
//!
//! Which statements a region body admits, what they lower to and what the
//! model then decides are the admitting route's and
//! [`qcore_core::error_region`]'s respectively. Nothing here re-states a rule
//! either of them owns.
//!
//! # Zero reserved words and zero new codes
//!
//! Every word of the form — `error`, `region`, `catch`, `quant`, `open`, `as`,
//! `discharge`, and the region body's `credit` and `effort` — is **already** in
//! `frontend::is_reserved_source_word`, and the shape already parses
//! (`frontend::error_region_statement`). Card `QUANT-REGION-SOURCE` therefore
//! adds **no** reserved word, and its constraint-14 census is exact rather than
//! statistical: before this card `error region R { … }` resolved `R` as a
//! reference and refused at `Q013`, so **no** program in this repository could
//! contain the statement at all.
//!
//! Codes are likewise unmoved under the `Q-NAMESPACE-DECISION` ice box. Every
//! refusal in this module is a condition of `Q302`, the `ErrorRegion` pass's
//! own generic presentation code, and the model's own refusals keep the exact
//! `Q302`–`Q309` codes that pass already allocates for those conditions — the
//! same allocation `qcore_core::region_task_credit::RegionTaskDiagnostic::code`
//! reads.

use qcore_core::error_region::ErrorRegionError;

use crate::frontend::{SemanticPass, SourceBlock, SourceDiagnostic, SourceProgram};
use crate::lexer::{Span, Symbol, Token, TokenKind};

/// The statement head's two words.
pub const REGION_KEYWORDS: [&str; 2] = ["error", "region"];
/// The handler clause's head.
pub const CATCH_KEYWORD: &str = "catch";
/// The pack-opening clause's two words.
pub const QUANT_OPEN_KEYWORDS: [&str; 2] = ["quant", "open"];
/// The binder of the opened capability.
pub const AS_KEYWORD: &str = "as";
/// The region body's credit declaration. Already reserved by
/// `T08-EPOCH-SRC`; reused rather than respelled, and read **only** inside a
/// region body by this card's own reader.
pub const CREDIT_KEYWORD: &str = "credit";
/// The region body's credit expenditure. Already reserved by the same card and
/// read only here.
pub const EFFORT_KEYWORD: &str = "effort";

/// One `catch (E) [quant open R as B]` clause header, as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegionCatchSyntax {
    /// The error type this clause catches.
    pub error: String,
    /// The region named by `quant open …`, when the clause opens a pack.
    pub opened_region: Option<String>,
    /// The capability name `… as B` binds, when the clause opens a pack.
    pub binding: Option<String>,
    /// The header's own extent, for attribution.
    pub span: Span,
}

/// Read every catch-clause header of one `error region` statement.
///
/// The headers are located **between the blocks** the parser already found:
/// clause `i`'s header is exactly the tokens after `blocks[i]` and before
/// `blocks[i + 1]`. Reading them positionally rather than by scanning for the
/// word `catch` is what makes a nested region inside the body impossible to
/// confuse with a clause of this one, without the reader tracking brace depth.
///
/// `bindings` is the parser's own parallel list (`StatementKind::ErrorRegion`'s
/// `catch_bindings`); it is passed in rather than re-read so the name this
/// module reports is the same one `elaborate_block` put into the handler's
/// scope.
pub fn read_catch_headers(
    program: &SourceProgram,
    blocks: &[SourceBlock],
    bindings: &[Option<String>],
    span: Span,
) -> Result<Vec<RegionCatchSyntax>, SourceDiagnostic> {
    let Some((body, handlers)) = blocks.split_first() else {
        return Err(refusal(
            span,
            "`source.region.v1` reads `error region NAME { … }` followed by at least one \
             `catch (ERROR) { … }` clause",
        ));
    };
    if handlers.len() != bindings.len() {
        return Err(refusal(
            span,
            "internal: the parsed catch clauses and their recorded bindings disagree",
        ));
    }
    let mut previous = body.span;
    let mut clauses = Vec::with_capacity(handlers.len());
    for (handler, binding) in handlers.iter().zip(bindings) {
        let window = between(program.lexical_tokens(), previous, handler.span);
        clauses.push(read_catch_header(&window, binding.as_deref(), span)?);
        previous = handler.span;
    }
    Ok(clauses)
}

fn read_catch_header(
    window: &[&Token],
    binding: Option<&str>,
    span: Span,
) -> Result<RegionCatchSyntax, SourceDiagnostic> {
    let header_span = window.first().map_or(span, |first| Span {
        start: first.span.start,
        end: window.last().map_or(first.span.end, |last| last.span.end),
    });
    if word(window, 0) != Some(CATCH_KEYWORD) {
        return Err(refusal(
            header_span,
            "`source.region.v1` reads a handler as `catch (ERROR) { … }`",
        ));
    }
    if !symbol(window, 1, Symbol::LeftParen) || !symbol(window, 3, Symbol::RightParen) {
        return Err(refusal(
            header_span,
            "`source.region.v1` reads a catch pattern as exactly one declared error type name, \
             written `catch (ERROR)`; a binding pattern is a separate construct and is refused \
             rather than ignored (follow-on `REGION-CATCH-PAYLOAD-BINDING`)",
        ));
    }
    let Some(error) = word(window, 2) else {
        return Err(refusal(
            header_span,
            "`source.region.v1` reads a catch pattern as exactly one declared error type name, \
             written `catch (ERROR)`",
        ));
    };
    let error = error.to_owned();
    if window.len() == 4 {
        if binding.is_some() {
            return Err(refusal(
                header_span,
                "internal: a clause with no `quant open` recorded a binding",
            ));
        }
        return Ok(RegionCatchSyntax {
            error,
            opened_region: None,
            binding: None,
            span: header_span,
        });
    }
    if word(window, 4) != Some(QUANT_OPEN_KEYWORDS[0])
        || word(window, 5) != Some(QUANT_OPEN_KEYWORDS[1])
    {
        return Err(refusal(
            header_span,
            "the only clause `source.region.v1` reads between a catch pattern and its block is \
             `quant open REGION as BINDING`",
        ));
    }
    let (Some(opened), Some(AS_KEYWORD), Some(bound)) = (word(window, 6), word(window, 7), binding)
    else {
        return Err(refusal(
            header_span,
            "`quant open` binds the sealed cleanup pack of one named region as a linear \
             capability: `quant open REGION as BINDING`",
        ));
    };
    if window.len() != 9 {
        return Err(refusal(
            header_span,
            "`source.region.v1` reads exactly `catch (ERROR) quant open REGION as BINDING`; a \
             header with anything further in it is refused rather than partly read",
        ));
    }
    Ok(RegionCatchSyntax {
        error,
        opened_region: Some(opened.to_owned()),
        binding: Some(bound.to_owned()),
        span: header_span,
    })
}

/// The `Q3xx` code the `ErrorRegion` pass already allocates for one model
/// refusal.
///
/// This is the allocation
/// `qcore_core::region_task_credit::RegionTaskDiagnostic::code` reads for the
/// refusals it carries over from this pass, stated once here so the two routes
/// cannot drift. Nothing new is allocated: `Q300` and `Q302`–`Q309` already
/// name exactly these conditions, and every refusal without a finer code of its
/// own presents at `Q302`, the pass's generic presentation code.
#[must_use]
pub fn error_region_code(error: &ErrorRegionError) -> &'static str {
    match error {
        ErrorRegionError::ErrorNotInEffectRow(_) => "Q300",
        ErrorRegionError::WrongRegionOpened { .. } => "Q303",
        ErrorRegionError::CleanupPackDuplicated(_) => "Q304",
        ErrorRegionError::CleanupPackNotDischarged => "Q305",
        ErrorRegionError::CatchWithoutRequiredQuantOpen { .. } => "Q306",
        ErrorRegionError::RethrowWithUndischargedCleanup => "Q307",
        ErrorRegionError::FallibleCleanup => "Q308",
        ErrorRegionError::ResourceInPayloadAndCleanupPack(_) => "Q309",
        _ => "Q302",
    }
}

/// The model's own message, with the code prefix it writes into its `Display`
/// removed exactly when the presentation already carries that code.
///
/// The model's wording is kept verbatim otherwise — "inherit, don't
/// reimplement" applies to a refusal's words as much as to its rule.
#[must_use]
pub fn error_region_message(error: &ErrorRegionError) -> String {
    let code = error_region_code(error);
    let message = error.to_string();
    message
        .strip_prefix(code)
        .and_then(|rest| rest.strip_prefix(": "))
        .map_or(message.clone(), ToOwned::to_owned)
}

/// A refusal of this reader. Always `Q302`: every condition here is a shape
/// condition of the `ErrorRegion` pass's own generic code.
fn refusal(span: Span, message: impl Into<String>) -> SourceDiagnostic {
    SourceDiagnostic {
        code: "Q302",
        pass: SemanticPass::ErrorRegion,
        span,
        message: message.into(),
    }
}

fn between(tokens: &[Token], before: Span, after: Span) -> Vec<&Token> {
    tokens
        .iter()
        .filter(|token| before.end <= token.span.start && token.span.end <= after.start)
        .collect()
}

fn word<'a>(window: &[&'a Token], index: usize) -> Option<&'a str> {
    match &window.get(index)?.kind {
        TokenKind::Word(word) => Some(word),
        _ => None,
    }
}

fn symbol(window: &[&Token], index: usize, expected: Symbol) -> bool {
    matches!(
        window.get(index).map(|token| &token.kind),
        Some(TokenKind::Symbol(symbol)) if *symbol == expected
    )
}

#[cfg(test)]
#[path = "../tests/unit/source_region.rs"]
mod tests;
