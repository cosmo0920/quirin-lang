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

//! Card `QR-LEXICAL-SELECTORS-DECLARATION-NAMES`: the one reading every
//! token-stream route selector shares.
//!
//! Most route selectors read function-body **statements** (`std_io_uses`,
//! `buffer_uses`, `socket_uses`, …), so a declaration can never reach them.
//! Five read the module's token stream instead — the `Q015` import gate's
//! token loop (`module_imports::concurrency_span`),
//! [`crate::source_tasks::has_source_tasks`],
//! [`crate::source_epoch::has_source_epoch`],
//! [`crate::source_mutex::has_source_mutex`] and the two lexical tests of
//! [`crate::source_scalar_calls::has_scalar_calls`] — and before this card a
//! declaration spelling one of their words moved the module: a contract module
//! declaring `extern fn qcore_buffer_new(…)` was refused `Q015` at the name,
//! `type epoch { … }` selected the epoch route, and an `extern fn
//! qcore_acquire` deselected the scalar route (the census,
//! `compiler/tests/qr_lexical_selector_census.rs`: 321 of 2,610 synthetic
//! declarations moved a selection).
//!
//! [`SelectorScope`] is the reading they now share: a declaration's own
//! **name** (an `extern fn`, a source function, an error, an alias, …), a
//! parameter's name, a contract clause and a library binding are never read.
//! Everything else stays read as before — bodies, signatures' types and
//! effect and `throws` rows (154 tracked files spell `effects { …, spawn,
//! join }`), an alias's type — and so does a **record**'s name and fields, by
//! constraint 14's recorded `AGGREGATE-SELECTOR-WORDS` decision. Comments are
//! not tokens and a string literal is not a word, so neither could select
//! before and neither can now.
//!
//! A fail-closed arm sits beside the reading, called from the driver's route
//! selection: [`ambiguous_selector_use`] (a selecting use of a word the module
//! also declares). The contract guard (a `module contract;` module that a
//! source-only route would select is refused) reads the driver's one route
//! table since card `ROUTE-SELECTION-ORDER-RESTATED`
//! (`qcore-verification`'s `driver::source_routes`), not a restated list
//! here. The Lean model is `QCore.RouteSelection` (`AuditRouteSelection`).
use crate::frontend::{DeclarationKind, SemanticPass, SourceDiagnostic, SourceProgram};
use crate::lexer::{Span, Symbol, Token, TokenKind};

fn within(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

/// Where a token-stream selector may read: every retained token except a
/// declaration's own **name**, a parameter's name, a contract clause and a
/// library binding.
///
/// A **record** is the one declaration whose name (and field names) stay
/// read: constraint 14's `AGGREGATE-SELECTOR-WORDS` decision (card
/// `AGGREGATES-V1`, pinned by `compiler/tests/source_record.rs`) keeps a record
/// that spells a selector word refused at its declaration (`Q015`), because
/// its construction cannot be written (the parser claims `spawn:` in a body at
/// `Q000`); excluding it would buy a type that declares and cannot be
/// constructed. Type positions (an alias's type, a parameter or result type,
/// an effect or `throws` row) are uses, not names, and stay read
/// (`type Handle = task<i32>;` still needs `std.concurrency`, pinned by
/// `crates/qcore-frontend/tests/module_imports.rs`).
#[derive(Clone, Debug, Default)]
pub struct SelectorScope {
    bodies: Vec<Span>,
    /// Spans never read: non-record declaration names, parameter names,
    /// contract clauses.
    excluded: Vec<Span>,
    /// Every declaration's own name token, records included: a declaration
    /// is never a use of its own name ([`ambiguous_selector_use`]).
    names: Vec<Span>,
}

impl SelectorScope {
    #[must_use]
    pub fn of(program: &SourceProgram) -> Self {
        let tokens = program.lexical_tokens();
        let mut scope = Self::default();
        for declaration in &program.declarations {
            if declaration.value.kind == DeclarationKind::Function
                && let Some(body) = &declaration.value.body
            {
                scope.bodies.push(body.span);
            }
            // The name is the first retained word inside the declaration
            // that spells the declared name (`fn NAME`, `extern fn NAME`,
            // `error NAME`, `type NAME`, …): a declaration carries its name
            // as text, not as a span, and every head word before it is a
            // keyword. A synthesized name (the concurrency profile's) matches
            // no token and excludes nothing.
            if let Some(name) = tokens.iter().find(|token| {
                within(declaration.span, token.span)
                    && token.span.start < token.span.end
                    && matches!(&token.kind, TokenKind::Word(word) if *word == declaration.value.name)
            }) {
                scope.names.push(name.span);
                if declaration.value.kind != DeclarationKind::Record {
                    scope.excluded.push(name.span);
                }
            }
            for parameter in &declaration.value.parameters {
                scope.excluded.push(parameter.name.span);
            }
            if let Some(contract) = &declaration.value.function_contract {
                scope.excluded.push(contract.span);
            }
        }
        scope
    }

    /// Whether a token-stream selector reads the token at `span`.
    #[must_use]
    pub fn reads(&self, program: &SourceProgram, span: Span) -> bool {
        !program.in_library_binding(span)
            && !self.excluded.iter().any(|excluded| within(*excluded, span))
    }

    /// Whether `span` is a declaration's own name token (records included).
    #[must_use]
    pub fn is_declaration_name(&self, span: Span) -> bool {
        self.names.iter().any(|name| within(*name, span))
    }

    /// The source function bodies' spans, braces included (the print-call
    /// selector's reading, card `QR-PRINT-ROUTE-MATCHES-DECLARATION`).
    #[must_use]
    pub fn bodies(&self) -> &[Span] {
        &self.bodies
    }
}

fn word(token: Option<&Token>) -> Option<&str> {
    match token.map(|token| &token.kind) {
        Some(TokenKind::Word(word)) => Some(word.as_str()),
        _ => None,
    }
}

fn symbol(token: Option<&Token>, expected: Symbol) -> bool {
    token.map(|token| &token.kind) == Some(&TokenKind::Symbol(expected))
}

/// The word a token-stream selector test matches at `at`, if any — the union
/// of the five selectors' tests — named by the word a declaration would have
/// to spell to be confused with it (the head word, or `ArithmeticError` for
/// the scalar route's `throws < ArithmeticError`).
#[must_use]
pub fn selector_word_at(tokens: &[Token], at: usize) -> Option<&str> {
    let next = tokens.get(at + 1);
    match word(tokens.get(at))? {
        word @ ("qcore_buffer_new"
        | "qcore_buffer_read"
        | "qcore_buffer_write"
        | "qcore_error_payload"
        | "spawn"
        | "join"
        | "qcore_acquire"
        | "qcore_check") => Some(word),
        "task" if symbol(next, Symbol::Less) => Some("task"),
        "mutex"
            if symbol(next, Symbol::Less)
                || word(next).is_some_and(|next| next == "guards" || next == "rank") =>
        {
            Some("mutex")
        }
        "epoch" if symbol(next, Symbol::LeftBrace) => Some("epoch"),
        "throws"
            if symbol(next, Symbol::Less)
                && word(tokens.get(at + 2)) == Some("ArithmeticError") =>
        {
            Some("ArithmeticError")
        }
        _ => None,
    }
}

/// A selecting use of a selector word in a module that also declares that
/// word: the use names both the builtin form and the declaration, so the
/// module is refused at selection (`Q020`) rather than routed — the print
/// card's "call beside a declared `print`" rule, for every token-stream
/// selector.
#[must_use]
pub fn ambiguous_selector_use(program: &SourceProgram) -> Option<SourceDiagnostic> {
    let scope = SelectorScope::of(program);
    let tokens = program.lexical_tokens();
    (0..tokens.len()).find_map(|at| {
        let name = selector_word_at(tokens, at)?;
        if !scope.reads(program, tokens[at].span)
            || scope.is_declaration_name(tokens[at].span)
            || !program
                .declarations
                .iter()
                .any(|declaration| declaration.value.name == name)
        {
            return None;
        }
        Some(SourceDiagnostic {
            code: "Q020",
            pass: SemanticPass::OrdinaryTypes,
            span: tokens[at].span,
            message: format!(
                "route selection: this use of the selector word `{name}` selects a route, and the module also declares `{name}`, so the use names both; the module is refused rather than routed"
            ),
        })
    })
}
