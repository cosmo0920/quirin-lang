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
//!               | "let" NAME "=" "fact" NAME ";"           -- a guard-scoped refinement fact
//!               | "read" NAME ";" | "write" NAME ";"       -- unguarded access
//!               | "wait" NAME ";"                          -- announced acquisition
//!               | "use" NAME ";"
//!               | "thread" "scope" NAME "{" child* "}"     -- one child task, joined here
//!               | "with" "lock" NAME "as" NAME "{" inner* "}"
//! inner       ::= "open" NAME ";" | "close" NAME ";" | "close" NAME "unchanged" ";"
//!               | ("read" | "write") NAME "with" NAME ";"
//!               | "let" NAME "=" "borrow" NAME "with" NAME ";"
//!               | "drop" NAME ";"
//!               | "assert" NAME "with" NAME ";" | "use" NAME ";"
//!               | "wait" NAME ";"
//!               | "thread" "scope" NAME "{" child* "}"
//!               | "thread" "scope" NAME "under" NAME "{" child* "}" -- guard-local
//!               | "raise" NAME "::" NAME ";"               -- typed-error exit, last only
//! child       ::= any `outer` or `inner` form except a declaration and except
//!                 `drop`; a child task inherits the enclosing critical
//!                 section lexically but holds none of its authority
//! ```
//!
//! Guard names bind only inside their `with lock` body (the W6 frontend-gate
//! rule), so a guard-derived name cannot be mentioned after the block at all —
//! that is a `Q013` name-resolution refusal, not a concurrency one. A borrow
//! that is still live when the block ends is `Q615`.
//!
//! Three completions land in card T03-F, each bounded and each stated as a rule
//! rather than a convenience:
//!
//! * `close GUARD unchanged;` selects the **checked** `NoWriteOccurred`
//!   witness. It is admitted only when every statement since this guard's
//!   `open` was a read or a fact use — neither can write — which is the shape
//!   this lowering can decide; a section
//!   in that shape adds no `InvariantPredicateDeclared` premise. Any other
//!   shape is refused here (`Q600`) — never silently downgraded to the declared
//!   witness, and never assumed.
//! * `let f = fact r;` declares a refinement fact, `assert f with g;`
//!   establishes it under one guard and `use f;` relies on it. Each assertion
//!   mints a fresh model identity, so relying on a fact after its guard was
//!   released is the core's `StaleProtectedFact` (`Q616`), and relying on one
//!   never asserted is `UnknownFact`.
//! * a nested `thread scope c { … }` is one child task, created where the block
//!   opens and joined where it closes. The child receives no share and no
//!   custody: every access it makes is either mutex-mediated or refused. A
//!   child block written *inside* a critical section is refused by the core's
//!   own `JoinWhileGuardHeld` rule, and a child that names the enclosing
//!   guard is refused with `WrongGuardAffinity` (`Q612`).
//!
//! Card T03-GUARD-JOIN adds one opt-in form on top of that last rule.
//! `thread scope c under g { … }`, written inside `g`'s own section, lowers its
//! join to the core's `GuardLocalJoin` instead of `Join`. `under` names the
//! enclosing section explicitly — outside one, or naming a different guard, it
//! is refused here — so the admission is requested at the source rather than
//! inferred from where the block happens to sit. Whether the join is *admitted*
//! is entirely the core's decision: it checks that the child was created inside
//! this same section, that the section is the only one held with its invariant
//! closed and no live borrow, and that the whole joined subtree named no mutex,
//! guard or guard-scoped fact anywhere. A plain `thread scope c { … }` inside a
//! section is unchanged and still refuses at `Q600`, so nothing already written
//! moves.
//!
//! Because a child in this profile can hold no share and no custody, the work a
//! guard-local child can do is whatever this profile does not model: it may not
//! touch a declared resource at all. The admission is about the *join*, not
//! about giving the child authority.
//!
//! What this lowering does **not** establish: source-to-trace refinement, the
//! real C layout of the declared intervals (`DeclaredFootprintShape`), or the
//! invariant predicate itself (`InvariantPredicateDeclared`, on every section
//! that closes by declaration). The task-profile `spawn`/`join` expression
//! forms, captures, owned values and completion capsules remain refused.
use std::collections::BTreeMap;

use crate::frontend::*;
use crate::lexer::{Span, Symbol, Token, TokenKind};
use qcore_core::prelude::concurrency::mutex_invariant::{
    AccessFootprint, AccessKind, FULL_ACCESS, InvariantWitness, MutexId, MutexInvariantDiagnostic,
    MutexInvariantEvent, MutexInvariantProgram, ShareGrant, SharedResourceId,
    UnsupportedSynchronization,
};

/// The task the entry function itself runs as. Child tasks are the nested
/// `thread scope` blocks; the `spawn`/`join` expression forms of the scoped
/// task profile are refused, because their captures, owned values and
/// completion capsules have no representation in this model.
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

/// The critical section a statement is lexically inside, if any. A child task
/// written inside a section inherits this frame: that is exactly how a child
/// comes to *name* a guard it does not hold, and the core then refuses the act
/// with `WrongGuardAffinity` rather than the name being unresolvable.
#[derive(Clone, Copy)]
struct Section<'a> {
    name: &'a str,
    guard: u32,
    mutex: MutexId,
    /// The task that executed the `Lock` minting `guard`.
    owner: u32,
}

/// Which task's statements are being lowered, and under which section.
#[derive(Clone, Copy)]
struct Frame<'a> {
    task: u32,
    section: Option<Section<'a>>,
}

impl Frame<'_> {
    const fn root() -> Self {
        Self {
            task: ROOT_TASK,
            section: None,
        }
    }

    const fn is_root_task(&self) -> bool {
        self.task == ROOT_TASK
    }
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
    /// Declared refinement facts: source name to the protected resource the
    /// fact is about.
    fact_resource: BTreeMap<String, SharedResourceId>,
    /// The model fact identity a source fact name currently denotes. The
    /// declaration seeds it with an identity no `AssertFact` ever mints, so a
    /// use before any assertion is refused by the core as an unknown fact
    /// rather than by the grammar.
    fact_identity: BTreeMap<String, u32>,
    /// Borrow bindings of one critical section, keyed by that section's guard.
    borrows: BTreeMap<u32, BTreeMap<String, u32>>,
    /// Guards whose invariant is currently open, and whether every statement
    /// since the `open` has been a read. This is the decidable shape for the
    /// checked `NoWriteOccurred` witness; anything else leaves it `false`.
    open_regions: BTreeMap<u32, bool>,
    error_exits: BTreeMap<u32, String>,
    next_guard: u32,
    next_borrow: u32,
    next_fact: u32,
    next_task: u32,
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

    /// Every statement that is not a read makes the enclosing open regions
    /// undecidable for the checked witness. Conservative on purpose: a child
    /// task's statements, a borrow, an assertion and an acquisition all count.
    fn invalidate_open_regions(&mut self) {
        for decidable in self.open_regions.values_mut() {
            *decidable = false;
        }
    }

    fn mint_fact(&mut self) -> u32 {
        self.next_fact += 1;
        self.next_fact
    }
}

/// Lower one entry function to a checked-profile program.
///
/// The returned program still requires
/// `qcore_core::prelude::concurrency::mutex_invariant::check_mutex_invariant_program`; lowering
/// establishes nothing on its own.
pub fn lower_source_mutex(
    source: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
) -> Result<LoweredMutexModule, Vec<SourceDiagnostic>> {
    crate::module_imports::check_source_imports(source)?;
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
            "the scoped task profile's `spawn`/`join` expressions are outside \
             source.mutex.guard.v1; the profile refuses completion-capsule composition. Its own \
             child-task form is a nested `thread scope NAME { … }`",
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
        if lower_statement(source, statement, Frame::root(), false, &mut state)?.is_some() {
            return Err(failure(
                statement.span,
                "a typed-error exit belongs inside a critical section",
            ));
        }
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

/// Lower one statement of any admitted body: the thread scope's own, a
/// critical section's, or a child task's.
///
/// Returns the `exit` identity when the statement is the enclosing critical
/// section's typed-error exit. `last` says whether this statement ends its
/// block, which only a typed-error exit uses.
fn lower_statement(
    source: &SourceProgram,
    statement: &Located<StatementKind>,
    frame: Frame<'_>,
    last: bool,
    state: &mut Lowering,
) -> Result<Option<u32>, SourceDiagnostic> {
    let span = statement.span;
    let tokens = header_tokens(source, span);
    match &statement.value {
        StatementKind::Let {
            binding: Some(binding),
        } if word(tokens.get(3), "borrow") => {
            state.invalidate_open_regions();
            lower_borrow(&tokens, span, binding, frame, state).map(|()| None)
        }
        StatementKind::Let { binding: Some(_) } => {
            state.invalidate_open_regions();
            if !frame.is_root_task() || frame.section.is_some() {
                return Err(failure(
                    span,
                    "source.mutex.guard.v1 declares protected resources, mutex generations and \
                     refinement facts at thread-scope level in the creating task only",
                ));
            }
            lower_declaration(&tokens, span, state).map(|()| None)
        }
        StatementKind::Drop { place: Some(place) } => {
            state.invalidate_open_regions();
            lower_end_borrow(span, place, frame, state).map(|()| None)
        }
        StatementKind::Raise => {
            state.invalidate_open_regions();
            lower_raise(&tokens, span, frame, last, state).map(Some)
        }
        StatementKind::WithLock { guard, body } => {
            state.invalidate_open_regions();
            if frame.section.is_some() {
                // Represented refusal: the construct becomes a real event the
                // core rejects, rather than being inexpressible here.
                state.push(
                    span,
                    MutexInvariantEvent::UnsupportedConstruct {
                        task: frame.task,
                        construct: UnsupportedSynchronization::NestedProtectingMutex,
                    },
                );
                return Ok(None);
            }
            lower_critical_section(source, span, &tokens, guard, body, frame, state).map(|()| None)
        }
        StatementKind::ThreadScope { name: child, body } => {
            state.invalidate_open_regions();
            lower_child_task(source, span, &tokens, child, body, frame, state).map(|()| None)
        }
        StatementKind::Expression if word(tokens.first(), "open") => {
            state.invalidate_open_regions();
            let section = require_section(span, frame, "open")?;
            expect_guarded_form(&tokens, span, 3, 1, section, "`open GUARD;`")?;
            state.push(
                span,
                MutexInvariantEvent::OpenInvariant {
                    task: frame.task,
                    guard: section.guard,
                },
            );
            // Only the holder's own region is tracked: a child's `open` is
            // refused by the core, and no region may be opened for it.
            if section.owner == frame.task {
                state.open_regions.insert(section.guard, true);
            }
            Ok(None)
        }
        StatementKind::Expression if word(tokens.first(), "close") => {
            let section = require_section(span, frame, "close")?;
            let checked = if tokens.len() == 4 && word(tokens.get(2), "unchanged") {
                if !symbol(tokens.get(3), Symbol::Semicolon) || !names_guard(&tokens, 1, section) {
                    return Err(failure(
                        span,
                        "restoring an unwritten invariant is exactly `close GUARD unchanged;` for \
                         this section's own guard",
                    ));
                }
                true
            } else {
                expect_guarded_form(&tokens, span, 3, 1, section, "`close GUARD;`")?;
                false
            };
            let decidable = state.open_regions.get(&section.guard).copied();
            state.invalidate_open_regions();
            state.open_regions.remove(&section.guard);
            let witness = if checked {
                if decidable != Some(true) {
                    return Err(failure(
                        span,
                        "`close GUARD unchanged;` requires an open region this profile can decide: \
                         every statement since `open GUARD;` must be a read or a fact use. Use \
                         `close GUARD;` to \
                         restore the invariant by declaration instead",
                    ));
                }
                // Checked, not declared: the core verifies that no write
                // occurred while the invariant was open, so this section adds
                // no `InvariantPredicateDeclared` premise.
                InvariantWitness::NoWriteOccurred
            } else {
                // The predicate is declared, never proved: this is exactly
                // where `InvariantPredicateDeclared` enters the certificate.
                // The generation and its invariant share one identity in this
                // lowering, so a `close` names its own mutex.
                InvariantWitness::Declared {
                    invariant: section.mutex,
                }
            };
            state.push(
                span,
                MutexInvariantEvent::CloseInvariant {
                    task: frame.task,
                    guard: section.guard,
                    witness,
                },
            );
            Ok(None)
        }
        StatementKind::Expression if word(tokens.first(), "assert") => {
            state.invalidate_open_regions();
            lower_assert_fact(&tokens, span, frame, state).map(|()| None)
        }
        StatementKind::Expression if word(tokens.first(), "use") => {
            lower_use_fact(&tokens, span, frame, state).map(|()| None)
        }
        StatementKind::Expression if word(tokens.first(), "wait") => {
            state.invalidate_open_regions();
            lower_wait(&tokens, span, frame, state).map(|()| None)
        }
        StatementKind::Expression => lower_access(&tokens, span, frame, state).map(|()| None),
        StatementKind::Defer => Err(failure(
            span,
            "deferred cleanup inside a critical section is outside source.mutex.guard.v1; the \
             invariant must be restored by an explicit `close` before the section exits",
        )),
        _ => Err(failure(
            span,
            "statement is outside source.mutex.guard.v1's grammar",
        )),
    }
}

fn names_guard(tokens: &[Token], index: usize, section: Section<'_>) -> bool {
    name(tokens.get(index)) == Some(section.name)
}

fn require_section<'a>(
    span: Span,
    frame: Frame<'a>,
    what: &str,
) -> Result<Section<'a>, SourceDiagnostic> {
    frame.section.ok_or_else(|| {
        failure(
            span,
            format!("`{what}` names a guard and is admitted only inside a critical section"),
        )
    })
}

fn expect_guarded_form(
    tokens: &[Token],
    span: Span,
    length: usize,
    guard_index: usize,
    section: Section<'_>,
    shape: &str,
) -> Result<(), SourceDiagnostic> {
    if tokens.len() != length
        || !names_guard(tokens, guard_index, section)
        || !symbol(tokens.get(length - 1), Symbol::Semicolon)
    {
        return Err(failure(
            span,
            format!("this statement is exactly {shape} for this section's own guard"),
        ));
    }
    Ok(())
}

/// `read NAME;` / `write NAME;` and `read NAME with GUARD;` /
/// `write NAME with GUARD;`.
fn lower_access(
    tokens: &[Token],
    span: Span,
    frame: Frame<'_>,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    let kind = if word(tokens.first(), "read") {
        AccessKind::Read
    } else if word(tokens.first(), "write") {
        AccessKind::Write
    } else {
        return Err(failure(
            span,
            "statement is outside source.mutex.guard.v1's grammar",
        ));
    };
    if kind == AccessKind::Write {
        state.invalidate_open_regions();
    }
    let guarded = tokens.len() == 5;
    if guarded {
        let section = require_section(span, frame, "a guarded access")?;
        if !word(tokens.get(2), "with")
            || !names_guard(tokens, 3, section)
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
                task: frame.task,
                footprint,
                kind,
                guard: Some(section.guard),
            },
        );
        return Ok(());
    }
    if tokens.len() != 3 || !symbol(tokens.get(2), Symbol::Semicolon) {
        return Err(failure(
            span,
            "an unguarded access is exactly `read NAME;` or `write NAME;`",
        ));
    }
    let resource = resolve_resource(name(tokens.get(1)), span, state)?;
    let footprint = state.footprint(resource);
    state.push(
        span,
        MutexInvariantEvent::Access {
            task: frame.task,
            footprint,
            kind,
            guard: None,
        },
    );
    Ok(())
}

/// `let NAME = borrow RESOURCE with GUARD;`
fn lower_borrow(
    tokens: &[Token],
    span: Span,
    binding: &str,
    frame: Frame<'_>,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    let section = require_section(span, frame, "a guard-derived borrow")?;
    if tokens.len() != 8
        || !symbol(tokens.get(2), Symbol::Equal)
        || !word(tokens.get(5), "with")
        || !names_guard(tokens, 6, section)
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
    state
        .borrows
        .entry(section.guard)
        .or_default()
        .insert(binding.to_owned(), borrow);
    state.push(
        span,
        MutexInvariantEvent::BorrowUnderGuard {
            task: frame.task,
            guard: section.guard,
            footprint,
            borrow,
        },
    );
    Ok(())
}

/// `drop NAME;` for a borrow of the acting task's own critical section.
fn lower_end_borrow(
    span: Span,
    place: &str,
    frame: Frame<'_>,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    let section = require_section(span, frame, "drop")?;
    if section.owner != frame.task {
        // Deliberately a grammar refusal, not a represented one: the model's
        // `EndBorrow` checks only that the acting task is live, so lowering a
        // child's `drop` of its parent's borrow would be *accepted* there.
        return Err(failure(
            span,
            "a guard-derived borrow is released by the task that took it; a child task may not \
             end its parent's borrow",
        ));
    }
    let borrow = state
        .borrows
        .get_mut(&section.guard)
        .and_then(|borrows| borrows.remove(place))
        .ok_or_else(|| {
            failure(
                span,
                format!("`{place}` is not a guard-derived borrow of this critical section"),
            )
        })?;
    state.push(
        span,
        MutexInvariantEvent::EndBorrow {
            task: frame.task,
            borrow,
        },
    );
    Ok(())
}

/// `raise ERROR::VARIANT;` — the typed-error exit of a critical section.
fn lower_raise(
    tokens: &[Token],
    span: Span,
    frame: Frame<'_>,
    last: bool,
    state: &mut Lowering,
) -> Result<u32, SourceDiagnostic> {
    let section = require_section(span, frame, "raise")?;
    if section.owner != frame.task {
        return Err(failure(
            span,
            "a typed-error exit leaves the critical section its own task opened",
        ));
    }
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
    Ok(state.next_exit)
}

/// `assert FACT with GUARD;` — establish a guard-scoped refinement fact.
fn lower_assert_fact(
    tokens: &[Token],
    span: Span,
    frame: Frame<'_>,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    let section = require_section(span, frame, "assert")?;
    if tokens.len() != 5
        || !word(tokens.get(2), "with")
        || !names_guard(tokens, 3, section)
        || !symbol(tokens.get(4), Symbol::Semicolon)
    {
        return Err(failure(
            span,
            "establishing a guard-scoped fact is exactly `assert FACT with GUARD;` for this \
             section's own guard",
        ));
    }
    let fact_name = name(tokens.get(1))
        .ok_or_else(|| failure(span, "a fact assertion names one declared fact"))?
        .to_owned();
    let resource = *state.fact_resource.get(&fact_name).ok_or_else(|| {
        failure(
            span,
            format!("`{fact_name}` is not a declared refinement fact"),
        )
    })?;
    let footprint = state.footprint(resource);
    // Each assertion mints a fresh identity: re-establishing a fact under a
    // later guard is a new fact, never a revival of the dropped one.
    let fact = state.mint_fact();
    state.fact_identity.insert(fact_name, fact);
    state.push(
        span,
        MutexInvariantEvent::AssertFact {
            task: frame.task,
            guard: section.guard,
            footprint,
            fact,
        },
    );
    Ok(())
}

/// `use FACT;` — rely on a guard-scoped refinement fact.
fn lower_use_fact(
    tokens: &[Token],
    span: Span,
    frame: Frame<'_>,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    if tokens.len() != 3 || !symbol(tokens.get(2), Symbol::Semicolon) {
        return Err(failure(span, "relying on a fact is exactly `use FACT;`"));
    }
    let fact_name =
        name(tokens.get(1)).ok_or_else(|| failure(span, "a fact use names one declared fact"))?;
    let fact = *state.fact_identity.get(fact_name).ok_or_else(|| {
        failure(
            span,
            format!("`{fact_name}` is not a declared refinement fact"),
        )
    })?;
    state.push(
        span,
        MutexInvariantEvent::UseFact {
            task: frame.task,
            fact,
        },
    );
    Ok(())
}

/// `wait MUTEX;` — announce an acquisition this task has not performed.
fn lower_wait(
    tokens: &[Token],
    span: Span,
    frame: Frame<'_>,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    if tokens.len() != 3 || !symbol(tokens.get(2), Symbol::Semicolon) {
        return Err(failure(
            span,
            "announcing an acquisition is exactly `wait MUTEX;`",
        ));
    }
    let mutex_name =
        name(tokens.get(1)).ok_or_else(|| failure(span, "an announcement names one mutex"))?;
    let mutex = *state
        .mutex_of
        .get(mutex_name)
        .ok_or_else(|| failure(span, format!("`{mutex_name}` is not a declared mutex")))?;
    state.push(
        span,
        MutexInvariantEvent::RegisterWaiter {
            task: frame.task,
            mutex,
        },
    );
    Ok(())
}

/// A nested `thread scope NAME { … }`: one child task, created where the block
/// begins and joined where it ends.
///
/// This is the whole of the admitted task composition. The child receives no
/// share and no custody, so every access it makes to a declared resource is
/// either mutex-mediated or refused for lack of authority; and because the
/// join is emitted at the block's end, a block written inside a critical
/// section is refused by the core's own `JoinWhileGuardHeld` rule rather than
/// by this lowering.
fn lower_child_task(
    source: &SourceProgram,
    span: Span,
    header: &[Token],
    child_name: &str,
    body: &SourceBlock,
    frame: Frame<'_>,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    if !word(header.first(), "thread") || !word(header.get(1), "scope") {
        return Err(failure(
            span,
            "a child task is exactly `thread scope NAME { … }`, or \
             `thread scope NAME under GUARD { … }` for a guard-local child",
        ));
    }
    // Card T03-GUARD-JOIN: `under GUARD` requests the guard-local join. It names
    // the enclosing section explicitly, so the admission is opted into at the
    // source rather than inferred from position; a plain `thread scope NAME`
    // inside a section keeps the core's `JoinWhileGuardHeld` refusal.
    let guard_local = match header.len() {
        3 => None,
        5 if word(header.get(3), "under") => {
            let Some(section) = frame.section else {
                return Err(failure(
                    span,
                    "`thread scope NAME under GUARD { … }` names the critical section the child \
                     is local to, so it belongs inside a `with lock` body",
                ));
            };
            if !names_guard(header, 4, section) {
                return Err(failure(
                    span,
                    "a guard-local child names its own enclosing section's guard: \
                     `thread scope NAME under GUARD { … }`",
                ));
            }
            Some(section)
        }
        _ => {
            return Err(failure(
                span,
                "a child task is exactly `thread scope NAME { … }`, or \
                 `thread scope NAME under GUARD { … }` for a guard-local child",
            ));
        }
    };
    let _ = child_name;
    state.next_task += 1;
    let child = state.next_task;
    let header_span = Span {
        start: span.start,
        end: body.span.start,
    };
    state.push(
        header_span,
        MutexInvariantEvent::Spawn {
            parent: frame.task,
            child,
        },
    );
    let child_frame = Frame {
        task: child,
        section: frame.section,
    };
    for statement in &body.statements {
        if lower_statement(source, statement, child_frame, false, state)?.is_some() {
            return Err(failure(
                statement.span,
                "a typed-error exit belongs inside a critical section",
            ));
        }
    }
    state.push(
        Span {
            start: body.span.end.saturating_sub(1),
            end: body.span.end,
        },
        match guard_local {
            Some(section) => MutexInvariantEvent::GuardLocalJoin {
                parent: frame.task,
                child,
                guard: section.guard,
            },
            None => MutexInvariantEvent::Join {
                parent: frame.task,
                child,
            },
        },
    );
    Ok(())
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
    if word(tokens.get(3), "fact") {
        if tokens.len() != 6 || !symbol(tokens.get(5), Symbol::Semicolon) {
            return Err(failure(
                span,
                "a refinement fact declaration is exactly `let NAME = fact RESOURCE;`",
            ));
        }
        let about = name(tokens.get(4))
            .ok_or_else(|| failure(span, "a fact declaration names one shared resource"))?;
        let resource = *state
            .resource_of
            .get(about)
            .ok_or_else(|| failure(span, format!("`{about}` is not a declared shared resource")))?;
        if state.fact_resource.contains_key(&binding) {
            return Err(failure(span, format!("`{binding}` is already declared")));
        }
        // A declared-but-unasserted fact denotes an identity no `AssertFact`
        // ever mints, so `use` before `assert` is refused by the core as an
        // unknown fact rather than by this grammar.
        let unasserted = state.mint_fact();
        state.fact_resource.insert(binding.clone(), resource);
        state.fact_identity.insert(binding, unasserted);
        return Ok(());
    }
    Err(failure(
        span,
        "source.mutex.guard.v1 declares only `protected` resources, `mutex guards` generations and \
         `fact` refinements at thread-scope level",
    ))
}

fn lower_critical_section(
    source: &SourceProgram,
    span: Span,
    header: &[Token],
    guard_name: &str,
    body: &SourceBlock,
    frame: Frame<'_>,
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
            task: frame.task,
            mutex,
            guard,
        },
    );

    let inner = Frame {
        task: frame.task,
        section: Some(Section {
            name: guard_name,
            guard,
            mutex,
            owner: frame.task,
        }),
    };
    let mut error_exit = None;
    for (index, statement) in body.statements.iter().enumerate() {
        if error_exit.is_some() {
            return Err(failure(
                statement.span,
                "a typed-error exit ends its critical section; no statement may follow it",
            ));
        }
        let last = index + 1 == body.statements.len();
        error_exit = lower_statement(source, statement, inner, last, state)?;
    }
    state.open_regions.remove(&guard);
    state.borrows.remove(&guard);
    match error_exit {
        Some(exit) => state.push(
            body.statements
                .last()
                .map_or(body.span, |statement| statement.span),
            MutexInvariantEvent::ErrorExit {
                task: frame.task,
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
                task: frame.task,
                guard,
            },
        ),
    }
    Ok(())
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
