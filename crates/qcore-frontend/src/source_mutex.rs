//! Exact source lowering for the bounded lexical-guard mutex profile
//! `source.mutex.guard.v1`, over `CheckedMutexInvariantV1`.
//!
//! The admitted surface is deliberately small and fully enumerated below. A
//! program is either entirely inside it and lowers to a complete
//! [`MutexInvariantProgram`] that the core checker validates independently, or
//! it is refused with a precise diagnostic. There is no partially lowered
//! middle state and no optimistic fallback: constructs this profile models as
//! refusals become real `UnsupportedConstruct`/`ExtendProtection` events, so
//! the core checker issues the refusal, and constructs the profile cannot even
//! represent are refused here at `Q600`.
//!
//! ```text
//! module      ::= (error-declaration | entry)*
//! entry       ::= "fn" NAME "(" ")" "->" "unit" "{" scope "}"
//! scope       ::= "thread" "scope" NAME "{" outer* "}"
//! outer       ::= "let" NAME "=" "protected" INT ";"          -- one protected resource
//!               | "let" NAME "=" "mutex" "guards" NAME ";" -- fixed footprint, declared here
//!               | "read" NAME ";" | "write" NAME ";"       -- unguarded access
//!               | "with" "lock" NAME "as" NAME "{" inner* "}"
//! inner       ::= "open" NAME ";" | "close" NAME ";"
//!               | ("read" | "write") NAME "with" NAME ";"
//!               | "let" NAME "=" "borrow" NAME "with" NAME ";"
//!               | "drop" NAME ";"
//!               | "raise" NAME "::" NAME ";"               -- typed-error exit, last only
//! ```
//!
//! Guard names bind only inside their `with lock` body (the W6 frontend-gate
//! rule), so a guard-derived name cannot be mentioned after the block at all —
//! that is a `Q013` name-resolution refusal, not a concurrency one. A borrow
//! that is still live when the block ends is `Q615`.
//!
//! What this lowering does **not** establish: source-to-trace refinement, the
//! real C layout of the declared intervals (`DeclaredFootprintShape`), or the
//! invariant predicate itself (`InvariantPredicateDeclared`). Both premises are
//! named on every certificate this route produces.
use std::collections::BTreeMap;

use crate::frontend::*;
use crate::lexer::{Span, Symbol, Token, TokenKind};
use qcore_core::mutex_invariant::{
    AccessFootprint, AccessKind, FULL_ACCESS, InvariantWitness, MutexId, MutexInvariantDiagnostic,
    MutexInvariantEvent, MutexInvariantProgram, ShareGrant, SharedResourceId,
    UnsupportedSynchronization,
};

/// The single task this profile models. Task creation is not in the admitted
/// grammar; a module that names one is refused as an unsupported composition.
const ROOT_TASK: u32 = 0;

/// One protected resource declared by `let NAME = protected N;`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MutexSourceResource {
    pub name: String,
    pub resource: SharedResourceId,
    pub bytes: u32,
}

/// One mutex generation declared by `let NAME = mutex guards R;`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MutexSourceGeneration {
    pub name: String,
    pub mutex: MutexId,
    pub protects: SharedResourceId,
}

/// A complete lowering of one entry function. The core checker still has to
/// validate `program`; nothing here is evidence on its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoweredMutexModule {
    pub entry: String,
    pub program: MutexInvariantProgram,
    /// Source span of the event at the same index. Every event has one.
    pub event_spans: Vec<Span>,
    /// Fallback span for refusals that name no event (end-of-trace sweep).
    pub scope_span: Span,
    pub resources: Vec<MutexSourceResource>,
    pub generations: Vec<MutexSourceGeneration>,
    /// `exit` id of an `ErrorExit` event to the raised source error name.
    pub error_exits: BTreeMap<u32, String>,
}

impl LoweredMutexModule {
    /// Present one core diagnostic at its own source span, under the code the
    /// core assigns it. The code is the core's; only the location is added.
    pub fn locate(&self, diagnostic: &MutexInvariantDiagnostic) -> SourceDiagnostic {
        let span = event_index(diagnostic)
            .and_then(|index| self.event_spans.get(index).copied())
            .unwrap_or(self.scope_span);
        SourceDiagnostic {
            code: diagnostic.code(),
            pass: SemanticPass::Concurrency,
            span,
            // `Display` already prefixes the code; the presentation boundary
            // adds it too, so carry the structured refusal only.
            message: format!("{diagnostic:?}"),
        }
    }
}

fn event_index(diagnostic: &MutexInvariantDiagnostic) -> Option<usize> {
    use MutexInvariantDiagnostic as D;
    match diagnostic {
        D::UnprotectedFootprint { event, .. }
        | D::UnguardedProtectedAccess { event, .. }
        | D::WrongGuardAffinity { event, .. }
        | D::InvariantOpenAtUnlock { event, .. }
        | D::InvariantOpenAtErrorExit { event, .. }
        | D::GuardBorrowEscape { event, .. }
        | D::StaleProtectedFact { event, .. }
        | D::FootprintAlreadyProtected { event, .. }
        | D::DynamicProtectedShape { event, .. }
        | D::NonquiescentMutexDestruction { event, .. }
        | D::ForgedSynchronizationWitness { event, .. }
        | D::UnsupportedSynchronizationConstruct { event, .. }
        | D::DeadMutexGeneration { event, .. }
        | D::DeadGuard { event, .. }
        | D::DoubleOpenInvariant { event, .. }
        | D::WriteWithClosedInvariant { event, .. }
        | D::InvalidInvariantWitness { event, .. }
        | D::UnknownTask { event, .. }
        | D::TerminalTask { event, .. }
        | D::DuplicateTask { event, .. }
        | D::UnknownJoin { event, .. }
        | D::NonquiescentJoin { event, .. }
        | D::JoinWhileGuardHeld { event, .. }
        | D::DeadResource { event, .. }
        | D::UnauthorizedAccess { event, .. }
        | D::WrongCustodian { event, .. }
        | D::NonquiescentDestruction { event, .. }
        | D::DuplicateLockCapability { event, .. }
        | D::UnknownMutex { event, .. }
        | D::IdentityNotFresh { event, .. }
        | D::UnknownBorrow { event, .. }
        | D::UnknownFact { event, .. } => Some(*event),
        D::MutexAuthorityNotConserved { event, .. } | D::InvalidFootprint { event } => *event,
        D::DataRace { first, .. } => Some(*first),
        D::OutstandingTask { .. }
        | D::OutstandingGuard { .. }
        | D::OutstandingMutex { .. }
        | D::OutstandingWaiter { .. } => None,
    }
}

/// Select this route. Detection is syntactic and route-selecting only: it
/// grants no support, and a detected module that leaves the admitted grammar is
/// refused rather than handed to another route.
pub fn has_source_mutex(source: &SourceProgram) -> bool {
    source
        .lexical_tokens()
        .windows(2)
        .any(|pair| word(pair.first(), "mutex") && word(pair.get(1), "guards"))
}

fn failure(span: Span, message: impl Into<String>) -> SourceDiagnostic {
    SourceDiagnostic {
        code: "Q600",
        pass: SemanticPass::Concurrency,
        span,
        message: message.into(),
    }
}

fn word(token: Option<&Token>, expected: &str) -> bool {
    matches!(token.map(|t| &t.kind), Some(TokenKind::Word(w)) if w == expected)
}

fn symbol(token: Option<&Token>, expected: Symbol) -> bool {
    matches!(token.map(|t| &t.kind), Some(TokenKind::Symbol(s)) if *s == expected)
}

fn name(token: Option<&Token>) -> Option<&str> {
    match &token?.kind {
        TokenKind::Word(w) => Some(w),
        _ => None,
    }
}

fn integer(token: Option<&Token>) -> Option<u32> {
    match &token?.kind {
        TokenKind::Integer(text) => text.parse().ok(),
        _ => None,
    }
}

/// Statement tokens, excluding any nested block. A structured statement's
/// header ends at its first `{`.
fn header_tokens(source: &SourceProgram, span: Span) -> Vec<Token> {
    let mut tokens = source
        .lexical_tokens()
        .iter()
        .filter(|token| token.span.start >= span.start && token.span.end <= span.end)
        .cloned()
        .collect::<Vec<_>>();
    if let Some(brace) = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Symbol(Symbol::LeftBrace))
    {
        tokens.truncate(brace);
    }
    tokens
}

#[derive(Default)]
struct Lowering {
    events: Vec<MutexInvariantEvent>,
    spans: Vec<Span>,
    resources: Vec<MutexSourceResource>,
    generations: Vec<MutexSourceGeneration>,
    resource_of: BTreeMap<String, SharedResourceId>,
    bytes_of: BTreeMap<SharedResourceId, u32>,
    mutex_of: BTreeMap<String, MutexId>,
    error_exits: BTreeMap<u32, String>,
    next_guard: u32,
    next_borrow: u32,
    next_exit: u32,
}

impl Lowering {
    fn push(&mut self, span: Span, event: MutexInvariantEvent) {
        self.events.push(event);
        self.spans.push(span);
    }

    fn footprint(&self, resource: SharedResourceId) -> AccessFootprint {
        AccessFootprint::new(resource, 0, self.bytes_of[&resource])
    }
}

/// Lower one entry function to a checked-profile program.
///
/// The returned program still requires
/// `qcore_core::mutex_invariant::check_mutex_invariant_program`; lowering
/// establishes nothing on its own.
pub fn lower_source_mutex(
    source: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
) -> Result<LoweredMutexModule, Vec<SourceDiagnostic>> {
    lower(source, typed, entry).map_err(|diagnostic| vec![diagnostic])
}

fn lower(
    source: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
) -> Result<LoweredMutexModule, SourceDiagnostic> {
    let module_span = typed
        .declarations
        .first()
        .map_or(Span { start: 0, end: 0 }, |declaration| declaration.span);
    if !source.task_expressions().is_empty() {
        return Err(failure(
            source
                .task_expressions()
                .first()
                .map_or(module_span, |expression| expression.span),
            "task creation composed with a checked mutex invariant is outside \
             source.mutex.guard.v1; the profile refuses completion-capsule composition",
        ));
    }
    let mut entry_declaration = None;
    for declaration in &source.declarations {
        match declaration.value.kind {
            DeclarationKind::ErrorType => {}
            DeclarationKind::Function if declaration.value.name == entry => {
                entry_declaration = Some(declaration);
            }
            _ => {
                return Err(failure(
                    declaration.span,
                    format!(
                        "source.mutex.guard.v1 admits one entry function and payload-free error \
                         declarations only; `{}` is outside the profile",
                        declaration.value.name
                    ),
                ));
            }
        }
    }
    let Some(declaration) = entry_declaration else {
        return Err(failure(
            module_span,
            format!("entry function `{entry}` has no source body in this module"),
        ));
    };
    if declaration.value.function_contract.is_some() {
        return Err(failure(
            declaration.span,
            "source contracts require an implemented consumer; the checked mutex profile \
             does not enforce them",
        ));
    }
    if !declaration.value.parameters.is_empty()
        || declaration
            .value
            .result_type
            .as_ref()
            .and_then(SourceResultType::executable_type)
            != Some(ExecutableResultType::Unit)
    {
        return Err(failure(
            declaration.span,
            "the checked mutex entry must be a parameter-free `-> unit` function",
        ));
    }
    let body = declaration
        .value
        .body
        .as_ref()
        .ok_or_else(|| failure(declaration.span, "entry function requires a source body"))?;
    let [scope_statement] = body.statements.as_slice() else {
        return Err(failure(
            body.span,
            "the checked mutex entry body must be exactly one `thread scope` statement",
        ));
    };
    let StatementKind::ThreadScope {
        body: scope_body, ..
    } = &scope_statement.value
    else {
        return Err(failure(
            scope_statement.span,
            "the checked mutex entry body must be exactly one `thread scope` statement",
        ));
    };

    let mut state = Lowering::default();
    for statement in &scope_body.statements {
        lower_outer(source, statement, &mut state)?;
    }
    // Scope exit destroys every generation and then every resource, in reverse
    // declaration order. Quiescence is checked by the core, not asserted here.
    let exit_span = Span {
        start: scope_body.span.end.saturating_sub(1),
        end: scope_body.span.end,
    };
    for generation in state.generations.clone().into_iter().rev() {
        state.push(
            exit_span,
            MutexInvariantEvent::DestroyMutex {
                task: ROOT_TASK,
                mutex: generation.mutex,
            },
        );
    }
    for resource in state.resources.clone().into_iter().rev() {
        state.push(
            exit_span,
            MutexInvariantEvent::Destroy {
                task: ROOT_TASK,
                resource: resource.resource,
            },
        );
    }

    let program = MutexInvariantProgram {
        initial_shares: state
            .resources
            .iter()
            .map(|resource| ShareGrant {
                task: ROOT_TASK,
                footprint: AccessFootprint::new(resource.resource, 0, resource.bytes),
                share: FULL_ACCESS,
            })
            .collect(),
        initial_custody: state
            .resources
            .iter()
            .map(|resource| (resource.resource, ROOT_TASK))
            .collect(),
        events: state.events,
    };
    Ok(LoweredMutexModule {
        entry: entry.to_owned(),
        program,
        event_spans: state.spans,
        scope_span: scope_statement.span,
        resources: state.resources,
        generations: state.generations,
        error_exits: state.error_exits,
    })
}

fn lower_outer(
    source: &SourceProgram,
    statement: &Located<StatementKind>,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    let span = statement.span;
    let tokens = header_tokens(source, span);
    match &statement.value {
        StatementKind::Let { binding: Some(_) } => lower_declaration(&tokens, span, state),
        StatementKind::Expression => {
            let (kind, resource) = unguarded_access(&tokens, span)?;
            let resource = *state.resource_of.get(resource).ok_or_else(|| {
                failure(
                    span,
                    format!("`{resource}` is not a declared shared resource"),
                )
            })?;
            let footprint = state.footprint(resource);
            state.push(
                span,
                MutexInvariantEvent::Access {
                    task: ROOT_TASK,
                    footprint,
                    kind,
                    guard: None,
                },
            );
            Ok(())
        }
        StatementKind::WithLock { guard, body } => {
            lower_critical_section(source, span, &tokens, guard, body, state)
        }
        _ => Err(failure(
            span,
            "statement is outside source.mutex.guard.v1's thread-scope grammar",
        )),
    }
}

fn unguarded_access(tokens: &[Token], span: Span) -> Result<(AccessKind, &str), SourceDiagnostic> {
    let kind = if word(tokens.first(), "read") {
        AccessKind::Read
    } else if word(tokens.first(), "write") {
        AccessKind::Write
    } else {
        return Err(failure(
            span,
            "statement is outside source.mutex.guard.v1's thread-scope grammar",
        ));
    };
    if tokens.len() != 3 || !symbol(tokens.get(2), Symbol::Semicolon) {
        return Err(failure(
            span,
            "an unguarded access is exactly `read NAME;` or `write NAME;`",
        ));
    }
    let resource = name(tokens.get(1))
        .ok_or_else(|| failure(span, "an unguarded access names one shared resource"))?;
    Ok((kind, resource))
}

fn lower_declaration(
    tokens: &[Token],
    span: Span,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    let binding = name(tokens.get(1))
        .ok_or_else(|| failure(span, "a declaration binds one name"))?
        .to_owned();
    if !word(tokens.first(), "let") || !symbol(tokens.get(2), Symbol::Equal) {
        return Err(failure(
            span,
            "statement is outside source.mutex.guard.v1's thread-scope grammar",
        ));
    }
    if word(tokens.get(3), "protected") {
        if tokens.len() != 6 || !symbol(tokens.get(5), Symbol::Semicolon) {
            return Err(failure(
                span,
                "a shared resource declaration is exactly `let NAME = protected BYTES;`",
            ));
        }
        let bytes = integer(tokens.get(4))
            .filter(|bytes| *bytes > 0)
            .ok_or_else(|| {
                failure(
                    span,
                    "a shared resource needs a positive decimal byte length",
                )
            })?;
        let resource = state.resources.len() as SharedResourceId + 1;
        state.bytes_of.insert(resource, bytes);
        if state
            .resource_of
            .insert(binding.clone(), resource)
            .is_some()
        {
            return Err(failure(span, format!("`{binding}` is already declared")));
        }
        state.resources.push(MutexSourceResource {
            name: binding,
            resource,
            bytes,
        });
        return Ok(());
    }
    if word(tokens.get(3), "mutex") {
        if tokens.len() != 7
            || !word(tokens.get(4), "guards")
            || !symbol(tokens.get(6), Symbol::Semicolon)
        {
            return Err(failure(
                span,
                "a mutex declaration is exactly `let NAME = mutex guards RESOURCE;`; the \
                 protected footprint is fixed at this declaration",
            ));
        }
        let protected = name(tokens.get(5))
            .ok_or_else(|| failure(span, "a mutex declaration names one shared resource"))?;
        let resource = *state.resource_of.get(protected).ok_or_else(|| {
            failure(
                span,
                format!("`{protected}` is not a declared shared resource"),
            )
        })?;
        let mutex = state.generations.len() as MutexId + 1;
        if state.mutex_of.insert(binding.clone(), mutex).is_some() {
            return Err(failure(span, format!("`{binding}` is already declared")));
        }
        let footprint = state.footprint(resource);
        state.push(
            span,
            MutexInvariantEvent::CreateMutex {
                task: ROOT_TASK,
                mutex,
                protects: vec![footprint],
                invariant: mutex,
            },
        );
        state.generations.push(MutexSourceGeneration {
            name: binding,
            mutex,
            protects: resource,
        });
        return Ok(());
    }
    Err(failure(
        span,
        "source.mutex.guard.v1 declares only `protected` resources and `mutex guards` generations \
         at thread-scope level",
    ))
}

fn lower_critical_section(
    source: &SourceProgram,
    span: Span,
    header: &[Token],
    guard_name: &str,
    body: &SourceBlock,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    if header.len() != 5
        || !word(header.first(), "with")
        || !word(header.get(1), "lock")
        || !word(header.get(3), "as")
    {
        return Err(failure(
            span,
            "a critical section is exactly `with lock MUTEX as GUARD { … }`",
        ));
    }
    let mutex_name =
        name(header.get(2)).ok_or_else(|| failure(span, "a critical section names one mutex"))?;
    let mutex = *state
        .mutex_of
        .get(mutex_name)
        .ok_or_else(|| failure(span, format!("`{mutex_name}` is not a declared mutex")))?;
    state.next_guard += 1;
    let guard = state.next_guard;
    let header_span = Span {
        start: span.start,
        end: body.span.start,
    };
    state.push(
        header_span,
        MutexInvariantEvent::Lock {
            task: ROOT_TASK,
            mutex,
            guard,
        },
    );

    let mut borrows = BTreeMap::<String, u32>::new();
    let mut error_exit = None;
    for (index, statement) in body.statements.iter().enumerate() {
        if error_exit.is_some() {
            return Err(failure(
                statement.span,
                "a typed-error exit ends its critical section; no statement may follow it",
            ));
        }
        let last = index + 1 == body.statements.len();
        error_exit = lower_inner(
            source,
            statement,
            guard_name,
            guard,
            mutex,
            last,
            &mut borrows,
            state,
        )?;
    }
    match error_exit {
        Some(exit) => state.push(
            body.statements
                .last()
                .map_or(body.span, |statement| statement.span),
            MutexInvariantEvent::ErrorExit {
                task: ROOT_TASK,
                guard,
                exit,
            },
        ),
        None => state.push(
            Span {
                start: body.span.end.saturating_sub(1),
                end: body.span.end,
            },
            MutexInvariantEvent::Unlock {
                task: ROOT_TASK,
                guard,
            },
        ),
    }
    Ok(())
}

/// Returns the `exit` id when this statement is the critical section's typed
/// error exit.
#[allow(clippy::too_many_arguments)]
fn lower_inner(
    source: &SourceProgram,
    statement: &Located<StatementKind>,
    guard_name: &str,
    guard: u32,
    mutex: MutexId,
    last: bool,
    borrows: &mut BTreeMap<String, u32>,
    state: &mut Lowering,
) -> Result<Option<u32>, SourceDiagnostic> {
    let span = statement.span;
    let tokens = header_tokens(source, span);
    let names_guard = |index: usize| name(tokens.get(index)) == Some(guard_name);
    match &statement.value {
        StatementKind::Expression if word(tokens.first(), "open") => {
            if tokens.len() != 3 || !names_guard(1) || !symbol(tokens.get(2), Symbol::Semicolon) {
                return Err(failure(
                    span,
                    "declaring the invariant broken is exactly `open GUARD;` for this section's \
                     own guard",
                ));
            }
            state.push(
                span,
                MutexInvariantEvent::OpenInvariant {
                    task: ROOT_TASK,
                    guard,
                },
            );
            Ok(None)
        }
        StatementKind::Expression if word(tokens.first(), "close") => {
            if tokens.len() != 3 || !names_guard(1) || !symbol(tokens.get(2), Symbol::Semicolon) {
                return Err(failure(
                    span,
                    "restoring the invariant is exactly `close GUARD;` for this section's own \
                     guard",
                ));
            }
            state.push(
                span,
                MutexInvariantEvent::CloseInvariant {
                    task: ROOT_TASK,
                    guard,
                    // The predicate is declared, never proved: this is exactly
                    // where `InvariantPredicateDeclared` enters the certificate.
                    // The generation and its invariant share one identity in
                    // this lowering, so a `close` names its own mutex.
                    witness: InvariantWitness::Declared { invariant: mutex },
                },
            );
            Ok(None)
        }
        StatementKind::Expression => {
            let kind = if word(tokens.first(), "read") {
                AccessKind::Read
            } else if word(tokens.first(), "write") {
                AccessKind::Write
            } else {
                return Err(failure(
                    span,
                    "statement is outside source.mutex.guard.v1's critical-section grammar",
                ));
            };
            if tokens.len() != 5
                || !word(tokens.get(2), "with")
                || !names_guard(3)
                || !symbol(tokens.get(4), Symbol::Semicolon)
            {
                return Err(failure(
                    span,
                    "a guarded access is exactly `read NAME with GUARD;` or \
                     `write NAME with GUARD;`",
                ));
            }
            let resource = resolve_resource(name(tokens.get(1)), span, state)?;
            let footprint = state.footprint(resource);
            state.push(
                span,
                MutexInvariantEvent::Access {
                    task: ROOT_TASK,
                    footprint,
                    kind,
                    guard: Some(guard),
                },
            );
            Ok(None)
        }
        StatementKind::Let {
            binding: Some(binding),
        } => {
            if tokens.len() != 8
                || !symbol(tokens.get(2), Symbol::Equal)
                || !word(tokens.get(3), "borrow")
                || !word(tokens.get(5), "with")
                || !names_guard(6)
                || !symbol(tokens.get(7), Symbol::Semicolon)
            {
                return Err(failure(
                    span,
                    "a critical section declares only `let NAME = borrow RESOURCE with GUARD;`",
                ));
            }
            let resource = resolve_resource(name(tokens.get(4)), span, state)?;
            let footprint = state.footprint(resource);
            state.next_borrow += 1;
            let borrow = state.next_borrow;
            borrows.insert(binding.clone(), borrow);
            state.push(
                span,
                MutexInvariantEvent::BorrowUnderGuard {
                    task: ROOT_TASK,
                    guard,
                    footprint,
                    borrow,
                },
            );
            Ok(None)
        }
        StatementKind::Drop { place: Some(place) } => {
            let borrow = borrows.remove(place).ok_or_else(|| {
                failure(
                    span,
                    format!("`{place}` is not a guard-derived borrow of this critical section"),
                )
            })?;
            state.push(
                span,
                MutexInvariantEvent::EndBorrow {
                    task: ROOT_TASK,
                    borrow,
                },
            );
            Ok(None)
        }
        StatementKind::Raise => {
            if !last {
                return Err(failure(
                    span,
                    "a typed-error exit must be the last statement of its critical section",
                ));
            }
            if tokens.len() != 5
                || !symbol(tokens.get(2), Symbol::ColonColon)
                || !symbol(tokens.get(4), Symbol::Semicolon)
            {
                return Err(failure(
                    span,
                    "a typed-error exit is exactly `raise ERROR::VARIANT;`",
                ));
            }
            let error = format!(
                "{}::{}",
                name(tokens.get(1)).unwrap_or_default(),
                name(tokens.get(3)).unwrap_or_default()
            );
            state.next_exit += 1;
            state.error_exits.insert(state.next_exit, error);
            Ok(Some(state.next_exit))
        }
        // Represented refusals: the construct exists, the model rejects it.
        StatementKind::WithLock { .. } => {
            state.push(
                span,
                MutexInvariantEvent::UnsupportedConstruct {
                    task: ROOT_TASK,
                    construct: UnsupportedSynchronization::NestedProtectingMutex,
                },
            );
            Ok(None)
        }
        StatementKind::Defer => Err(failure(
            span,
            "deferred cleanup inside a critical section is outside source.mutex.guard.v1; the \
             invariant must be restored by an explicit `close` before the section exits",
        )),
        _ => Err(failure(
            span,
            "statement is outside source.mutex.guard.v1's critical-section grammar",
        )),
    }
}

fn resolve_resource(
    place: Option<&str>,
    span: Span,
    state: &Lowering,
) -> Result<SharedResourceId, SourceDiagnostic> {
    let place = place.ok_or_else(|| failure(span, "expected one shared resource name"))?;
    state
        .resource_of
        .get(place)
        .copied()
        .ok_or_else(|| failure(span, format!("`{place}` is not a declared shared resource")))
}
