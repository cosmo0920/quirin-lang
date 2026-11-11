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

//! Source lowering for `source.std-io-extern.v1` — one module that both calls a
//! declared extern function and prints (card `STD-IO-COMPOSE`).
//!
//! ```text
//! module ::= "import" "std.extern" ";" ("import" STD_IO_MODULE ";")+
//!            (error-decl | extern-decl | entry)*
//! stmt   ::= <every `source.extern-call.v1` statement>      -- §G02d
//!          | <every `source.std-io.v1` statement>           -- §G02g
//! ```
//!
//! # This lowering owns no statement
//!
//! Every statement of a composed entry is lowered by **one of the two existing
//! lowerings, unchanged**. This module decides only which, and it decides it by
//! the statement head alone: `print`, `log` and `flush` are the standard-IO
//! model's, `return;` is *both* models' single return, and everything else is
//! the extern model's. Each lowering is then run with the other's statements in
//! its skip list, so neither ever sees a statement it does not model and
//! neither refusal surface moves.
//!
//! A module using one feature set is not composed and never reaches here: route
//! selection consults [`has_source_composed`] before either single route, and
//! that predicate is the conjunction of the two routes' own selectors. So a
//! printing module keeps `source.std-io.v1` byte for byte, an extern-calling
//! module keeps `source.extern-call.v1` byte for byte, and this route exists
//! only where both were true and the more specific model used to win.
//!
//! # What the composition adds, and it is only this
//!
//! One decision, made here rather than in either model: **which value a
//! rendering reads**. The extern model owns the value namespace, so the operand
//! names of `print`/`log` are resolved against *its* bindings — a bound scalar,
//! never a resource identity, which has no operand form at all — and the number
//! is the one the source determined, either a literal or a declared
//! `succeeds INTEGER` outcome. A name bound to a call result the module models
//! no number for is refused at `Q043` before the standard-IO lowering runs, so
//! it never becomes an operand whose line no model computed.
//!
//! # Concurrency is still excluded
//!
//! A module that prints and spawns, or extern-calls and spawns, still keeps its
//! concurrent route's refusal: every concurrent selector runs before this one,
//! on the route-selected-last discipline card `STD-MODULES` established. This
//! card composes two *sequential* models and no other pair.

use std::collections::BTreeMap;

use crate::frontend::{
    DeclarationKind, SemanticPass, SourceDiagnostic, SourceProgram, TypedSourceProgram,
};
use crate::lexer::{Span, Token, TokenKind};
use crate::source_extern::{self, LoweredExternModule};
use crate::source_std_io::{
    self, FLUSH_KEYWORD, LOG_KEYWORD, LOG_LEVEL_KEYWORD, LoweredStdIoModule, PRINT_KEYWORD,
};
use qcore_core::composed_std_io::{ComposedProgram, ComposedStep};
use qcore_core::extern_call::ExternCallee;
use qcore_core::scalar_types::{RecordedTarget, ScalarType};

/// A complete lowering of one composed entry. None of the three fields is
/// evidence on its own: the core composed checker still has to validate
/// `program`, which means both projections' checkers still have to validate
/// their own halves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoweredComposedModule {
    pub entry: String,
    pub program: ComposedProgram,
    /// The extern projection's lowering, including the [`FunctionSemantics`]
    /// bundle the established sequential passes validate exactly as they do on
    /// the extern route.
    ///
    /// [`FunctionSemantics`]: crate::whole_program::FunctionSemantics
    pub externs: LoweredExternModule,
    /// The standard-IO projection's lowering.
    pub io: LoweredStdIoModule,
    /// Source span of the composed step at the same index.
    pub step_spans: Vec<Span>,
    /// Fallback span for a refusal that names no step.
    pub scope_span: Span,
}

/// Whether this module is a `source.std-io-extern.v1` module: it calls a
/// declared extern function **and** uses one of the standard-IO statements.
///
/// The conjunction of the two routes' own selectors, and nothing else, so a
/// module can never be composed without both routes agreeing it is theirs.
#[must_use]
pub fn has_source_composed(program: &SourceProgram) -> bool {
    source_extern::has_source_extern(program)
        && (source_std_io::has_source_std_io(program)
            || source_std_io::has_source_log_level(program))
}

fn diagnostic(span: Span, message: impl Into<String>) -> SourceDiagnostic {
    SourceDiagnostic {
        code: "Q043",
        pass: SemanticPass::ComposedStdIo,
        span,
        message: message.into(),
    }
}

/// Which model owns one statement, by its head token alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Owner {
    Extern,
    Io,
    /// `return;` — the single return both models require, one source statement
    /// and one composed step.
    Both,
}

fn statement_head(tokens: &[Token], span: Span) -> Option<&str> {
    tokens
        .iter()
        .find(|token| span.start <= token.span.start && token.span.end <= span.end)
        .and_then(|token| match &token.kind {
            TokenKind::Word(word) => Some(word.as_str()),
            _ => None,
        })
}

const fn owner_of(head: &str) -> Owner {
    match head.as_bytes() {
        b"print" | b"log" | b"log_level" | b"flush" => Owner::Io,
        b"return" => Owner::Both,
        _ => Owner::Extern,
    }
}

/// Lower one composed entry.
pub fn lower_source_composed(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
    callees: &BTreeMap<String, ExternCallee>,
    libraries: &[qcore_core::native_library::SourceLibraryBinding],
) -> Result<LoweredComposedModule, Vec<SourceDiagnostic>> {
    lower_source_composed_for_target(parsed, typed, entry, callees, libraries, None)
}

/// [`lower_source_composed`] for the builder's recorded target (card
/// `W48-QR-SCALAR-TYPES-R3`).
pub fn lower_source_composed_for_target(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
    callees: &BTreeMap<String, ExternCallee>,
    libraries: &[qcore_core::native_library::SourceLibraryBinding],
    target: Option<&RecordedTarget>,
) -> Result<LoweredComposedModule, Vec<SourceDiagnostic>> {
    debug_assert_eq!(
        (PRINT_KEYWORD, LOG_KEYWORD, LOG_LEVEL_KEYWORD, FLUSH_KEYWORD),
        ("print", "log", "log_level", "flush"),
        "the statement heads this router splits on are the standard-IO route's own"
    );
    let tokens = parsed.lexical_tokens();
    let Some(declaration) = parsed.declarations.iter().find(|declaration| {
        declaration.value.kind == DeclarationKind::Function && declaration.value.name == entry
    }) else {
        return Err(vec![diagnostic(
            Span { start: 0, end: 0 },
            format!("`{entry}` is not a function declaration in this module"),
        )]);
    };
    let scope_span = declaration.span;
    let Some(body) = &declaration.value.body else {
        return Err(vec![diagnostic(scope_span, "this entry has no body")]);
    };

    // Partition the entry's statements by head. Nothing is dropped: a statement
    // is owned by exactly one model, or by both when it is the shared return.
    let mut skip_for_extern = Vec::new();
    let mut skip_for_io = Vec::new();
    let mut owners: Vec<(Span, Owner)> = Vec::new();
    for statement in &body.statements {
        let head = statement_head(tokens, statement.span).unwrap_or_default();
        let owner = owner_of(head);
        match owner {
            Owner::Extern => skip_for_io.push(statement.span),
            Owner::Io => skip_for_extern.push(statement.span),
            // The shared `return;` is skipped by neither: each model requires
            // its own, and one statement supplies both.
            Owner::Both => {}
        }
        owners.push((statement.span, owner));
    }

    // The extern model runs first, because it owns the value namespace every
    // rendering reads. Its skip list is exactly the standard-IO statements.
    let externs = source_extern::lower_source_extern_skipping(
        parsed,
        typed,
        entry,
        callees,
        libraries,
        &skip_for_extern,
        target,
    )?;

    // Resolve every rendering operand against that namespace before the
    // standard-IO lowering runs, so a composed refusal says why the composition
    // refuses rather than why this route's own literal binding is missing.
    let mut diagnostics = Vec::new();
    let mut scalars: BTreeMap<String, i32> = BTreeMap::new();
    let mut typed_scalars: BTreeMap<String, (ScalarType, i128)> = BTreeMap::new();
    let mut operand_values = BTreeMap::new();
    for (span, name) in source_std_io::rendered_scalar_operands(parsed, entry) {
        let Some((value, number)) = externs.scalar_values.get(&name).copied() else {
            if externs.resource_bindings.contains_key(&name) {
                // Left to the standard-IO route's own `Q028`, which is the
                // refusal card STD-MODULES pinned and whose message is exactly
                // right here: this profile has no operand form that could
                // denote a resource.
                continue;
            }
            // Not bound at all: name resolution's `Q013` already refused it,
            // and the standard-IO route's `Q028` refuses it again.
            continue;
        };
        // Card `W48-QR-SCALAR-TYPES-R3`: a value of every §G02t type renders,
        // at its own type; before the card only an `i32` did. The type is the
        // extern checker's, so a value statement's result renders at the type
        // the checker gave it.
        let Some(ty) = externs.value_types.get(&value).copied() else {
            continue;
        };
        if ty.is_target_width() && target.is_none() {
            diagnostics.push(diagnostic(
                span,
                format!(
                    "`{name}` is a `{ty}`, whose width only a recorded target supplies, and none \
                     was supplied; the rendering is refused rather than guessed"
                ),
            ));
            continue;
        }
        let Some(number) = number else {
            diagnostics.push(diagnostic(
                span,
                format!(
                    "`{name}` is bound to a call result this module models no number for, so \
                     rendering it would publish a line no model computed; write \
                     `let {name} = CALLEE(ARGS) succeeds INTEGER;` to declare the result, or \
                     render a value this entry determines"
                ),
            ));
            continue;
        };
        if ty == ScalarType::I32 {
            let Ok(number) = i32::try_from(number) else {
                continue;
            };
            scalars.insert(name.clone(), number);
        } else {
            typed_scalars.insert(name.clone(), (ty, number));
        }
        operand_values.insert(name, value);
    }

    // The custody half (card `STD-FMT-BORROW`). A `borrow NAME` operand
    // resolves against the extern model's **resource** namespace — the other
    // map, never this one — and supplies the model identity the rendering
    // publishes. Whether that identity is *live* at the statement is not
    // decided here: this lowering has no ledger, and the join reads the extern
    // checker's own published one rather than deriving a second.
    let mut identities: BTreeMap<String, i32> = BTreeMap::new();
    let mut operand_identities = BTreeMap::new();
    for (span, name) in source_std_io::rendered_borrowed_operands(parsed, entry) {
        let Some(resource) = externs.resource_bindings.get(&name).copied() else {
            diagnostics.push(diagnostic(
                span,
                format!(
                    "`borrow {name}` renders the model identity of a live resource, and this \
                     module binds no resource of that name; a borrowed rendering reads the \
                     extern model's custody namespace and never its value namespace"
                ),
            ));
            continue;
        };
        let Ok(identity) = i32::try_from(resource) else {
            diagnostics.push(diagnostic(
                span,
                format!(
                    "the model identity of `{name}` is outside the range \
                     `source.std-io.v1` renders; its two renderings are total on `int32_t` and \
                     on nothing else"
                ),
            ));
            continue;
        };
        identities.insert(name.clone(), identity);
        operand_identities.insert(name, resource);
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let io = source_std_io::lower_source_std_io_skipping(
        parsed,
        typed,
        entry,
        &skip_for_io,
        &scalars,
        &identities,
        &source_std_io::ComposedValues {
            typed: typed_scalars,
            target: target.cloned(),
            admit_arithmetic_error: externs.program.raises,
            admit_float_effects: externs.program.float_effects
                != qcore_core::scalar_types::FloatEffects::default(),
        },
    )?;

    // The interleaving. Both lowerings publish one span per event, so the
    // composed order is read off the source statement order rather than
    // reconstructed: a statement is the extern event with its span, the
    // standard-IO event with its span, or the shared return.
    let mut order = Vec::new();
    let mut step_spans = Vec::new();
    for (span, owner) in owners {
        let step = match owner {
            Owner::Both => Some(ComposedStep::Return),
            Owner::Extern => externs
                .event_spans
                .iter()
                .position(|candidate| *candidate == span)
                .map(ComposedStep::Extern),
            Owner::Io => io
                .event_spans
                .iter()
                .position(|candidate| *candidate == span)
                .map(ComposedStep::Io),
        };
        let Some(step) = step else {
            return Err(vec![diagnostic(
                span,
                "this statement produced no checked event in either composed model, so the \
                 composition would have checked a program the emitted C does not perform",
            )]);
        };
        order.push(step);
        step_spans.push(span);
    }

    let value_abi = externs.value_abi.clone();
    let program = ComposedProgram {
        entry: entry.to_owned(),
        externs: externs.program.clone(),
        io: io.program.clone(),
        order,
        operand_values,
        value_abi,
        operand_identities,
    };
    Ok(LoweredComposedModule {
        entry: entry.to_owned(),
        program,
        externs,
        io,
        step_spans,
        scope_span,
    })
}
